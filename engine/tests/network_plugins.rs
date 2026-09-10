use engine::net::prelude::*;
use engine::plugins::net::{
    ClientSnapshotReplicationState, NetworkClientInbox, NetworkClientOutbox, NetworkDiagnostics,
    NetworkOutboundQueue, NetworkServerInbox, NetworkServerOutbox, NetworkSessionStatus,
    OutboundServerMessage, PredictionDiagnostics, PredictionState as NetPredictionState,
    ReplicationDiagnostics, RunenNetSessionCore, RunenNetSessionProjection,
    ServerSnapshotReplicationState, client_inbox_is_empty, client_outbox_len, enqueue_client_inbox,
    enqueue_client_outbox, enqueue_server_inbox, enqueue_server_inbox_from,
    enqueue_server_outbox_broadcast, record_reconnect_attempt, server_inbox_is_empty,
    server_outbox_len, sync_runennet_session_projection,
};
use engine::plugins::{ScenePlugin, default_plugins};
use engine::prelude::*;
use runen_net::identity::{ConnectionHandle, ParticipantId, SessionId};
use runen_net::protocol::{
    CompatibilityOffer, NegotiatedContract, NegotiationManager, NegotiationManagerLimits,
    NegotiationRequirements, OfferLimits, ProtocolContract, ProtocolId, ProtocolRevision,
};
use runen_net::session::{Session, SessionLimits};
use serde::{Deserialize, Serialize};
use std::io;
use std::num::NonZeroUsize;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
struct MoveCommand {
    x: f32,
    y: f32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct AbilityCommand {
    slot: u8,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
enum ClientCommandEnvelope {
    Move(MoveCommand),
    Ability(AbilityCommand),
}

impl Default for ClientCommandEnvelope {
    fn default() -> Self {
        Self::Move(MoveCommand { x: 0.0, y: 0.0 })
    }
}

#[derive(Debug, Clone, Default, PartialEq, Component, runen_ecs::Resource)]
struct PlayerCommandBuffer {
    commands: Vec<ClientCommandEnvelope>,
}

impl PlayerCommandBuffer {
    fn push(&mut self, command: ClientCommandEnvelope) {
        self.commands.push(command);
    }

    fn drain(&mut self) -> Vec<ClientCommandEnvelope> {
        std::mem::take(&mut self.commands)
    }

    fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }
}

#[derive(Debug, Clone, Default, PartialEq, runen_ecs::Resource)]
struct AppliedInputLog {
    inputs: Vec<ClientCommandEnvelope>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct TestSnapshot {
    context: TestSnapshotContext,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct TestSnapshotContext {
    world_scene_label: String,
}

impl Default for TestSnapshot {
    fn default() -> Self {
        Self {
            context: TestSnapshotContext {
                world_scene_label: "gameplay_stub".to_string(),
            },
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
struct TestDelta {
    changed: bool,
}

struct TestReplicationDriver;

impl ReplicationDriver for TestReplicationDriver {
    type Snapshot = TestSnapshot;
    type Delta = TestDelta;
    type Input = ClientCommandEnvelope;
    type Error = io::Error;

    fn capture_snapshot(_world: &World) -> Result<Option<Self::Snapshot>, Self::Error> {
        Ok(Some(TestSnapshot::default()))
    }

    fn build_delta(previous: &Self::Snapshot, current: &Self::Snapshot) -> Self::Delta {
        TestDelta {
            changed: previous != current,
        }
    }

    fn apply_delta_to_snapshot(base: &Self::Snapshot, delta: &Self::Delta) -> Self::Snapshot {
        if delta.changed {
            Self::Snapshot::default()
        } else {
            base.clone()
        }
    }

    fn map_codec_error(error: postcard::Error) -> Self::Error {
        io::Error::new(io::ErrorKind::InvalidData, error.to_string())
    }
}

impl SnapshotApplyDriver for TestReplicationDriver {
    fn apply_snapshot(
        _world: &mut World,
        _tick: engine_sim::SimulationTick,
        _snapshot: Self::Snapshot,
    ) -> Result<bool, Self::Error> {
        Ok(true)
    }

    fn apply_delta(
        _world: &mut World,
        _tick: engine_sim::SimulationTick,
        _delta: Self::Delta,
    ) -> Result<bool, Self::Error> {
        Ok(true)
    }
}

impl InputDriver for TestReplicationDriver {
    fn receive_remote_input(
        _world: &mut World,
        _connection: ConnectionHandle,
        _tick: engine_sim::SimulationTick,
        _input: Vec<Self::Input>,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    fn take_local_input(world: &mut World) -> Result<Vec<Self::Input>, Self::Error> {
        Ok(world
            .resource_mut::<PlayerCommandBuffer>()
            .map(|commands| commands.drain())
            .unwrap_or_default())
    }

    fn apply_input(world: &mut World, input: &[Self::Input]) -> Result<(), Self::Error> {
        if world.resource::<AppliedInputLog>().is_err() {
            world.insert_resource(AppliedInputLog::default());
        }
        world
            .resource_mut::<AppliedInputLog>()
            .expect("applied-input log should exist after initialization")
            .inputs
            .extend_from_slice(input);
        Ok(())
    }
}

type PredictionState = NetPredictionState<ClientCommandEnvelope>;
type ClientSnapshotState = ClientSnapshotReplicationState<TestSnapshot>;
type ServerSnapshotState = ServerSnapshotReplicationState<TestSnapshot>;

struct NetworkClientPlugin;

impl Plugin for NetworkClientPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PlayerCommandBuffer>();
        app.add_plugin(NetPlugin::<TestReplicationDriver>::new(NetRole::Client));
    }
}

struct NetworkServerPlugin;

impl Plugin for NetworkServerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PlayerCommandBuffer>();
        app.add_plugin(NetPlugin::<TestReplicationDriver>::new(NetRole::Server));
    }
}

struct NetworkHostPlugin;

impl Plugin for NetworkHostPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PlayerCommandBuffer>();
        app.add_plugin(NetPlugin::<TestReplicationDriver>::new(NetRole::Host));
    }
}

fn test_protocol_contract() -> ProtocolContract {
    ProtocolContract::new(ProtocolId::new(1), ProtocolRevision::new(1))
}

fn test_compatibility_offer() -> CompatibilityOffer {
    CompatibilityOffer::new(vec![test_protocol_contract()], vec![], vec![], None)
}

fn test_runennet_session_core() -> RunenNetSessionCore {
    let negotiation =
        NegotiationManager::new(OfferLimits::default(), NegotiationManagerLimits::default())
            .expect("test negotiation limits must be valid");
    let capacity = NonZeroUsize::new(16).expect("test session capacity must be non-zero");
    let limits = SessionLimits::new(capacity, capacity).expect("test session limits must be valid");
    let session = Session::new(SessionId::new(1), limits);
    RunenNetSessionCore::new(negotiation, session)
}

fn establish_runennet_connection(
    core: &mut RunenNetSessionCore,
    projection: &mut RunenNetSessionProjection,
    participant: ParticipantId,
    connection: ConnectionHandle,
) {
    core.negotiation_mut()
        .start(
            connection,
            test_compatibility_offer(),
            test_compatibility_offer(),
        )
        .expect("compatible test negotiation must start");
    core.negotiation_mut()
        .propose(
            connection,
            NegotiatedContract::new(test_protocol_contract()),
            &NegotiationRequirements::default(),
        )
        .expect("compatible test contract must be proposed");
    core.negotiation_mut()
        .validate_authority(connection)
        .expect("authority validation must succeed");
    core.negotiation_mut()
        .validate_peer(connection)
        .expect("peer validation must establish compatibility");
    core.admit_established(projection, participant, connection)
        .expect("established RunenNet connection must be admitted");
}

fn install_runennet_connections(app: &mut App, bindings: &[(ConnectionHandle, ParticipantId)]) {
    let mut core = test_runennet_session_core();
    let mut projection = RunenNetSessionProjection::default();
    for (connection, participant) in bindings.iter().copied() {
        establish_runennet_connection(&mut core, &mut projection, participant, connection);
    }
    app.world_mut().insert_resource(core);
    app.world_mut().insert_resource(projection);
    sync_runennet_session_projection(app.world_mut());
}

include!("network_plugins/basic_flow.rs");

include!("network_plugins/runtime_and_replication.rs");

include!("network_plugins/delta_and_reconnect.rs");
