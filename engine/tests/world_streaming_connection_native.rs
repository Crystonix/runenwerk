use engine::SimulationTick;
use engine::net::prelude::{NetPlugin, NetRole};
use engine::plugins::net::{
    NetStreamingStateResource, RunenNetSessionCore, RunenNetSessionProjection,
    sync_runennet_session_projection,
};
use engine::plugins::world::adapters::resources::WorldQuantizationScaleResource;
use engine::plugins::world::edits::ingress::{WorldEditIngressMeta, submit_world_operation};
use engine::plugins::world::plugin::WorldPlugin;
use engine::prelude::App;
use engine_net::replication::{InputDriver, ReplicationDriver, SnapshotApplyDriver};
use runen_net::identity::{ConnectionHandle, ParticipantId, SessionId};
use runen_net::protocol::{
    CompatibilityOffer, NegotiatedContract, NegotiationManager, NegotiationManagerLimits,
    NegotiationRequirements, OfferLimits, ProtocolContract, ProtocolId, ProtocolRevision,
};
use runen_net::session::{Session, SessionLimits};
use runen_spatial::WorldId;
use std::io;
use std::num::NonZeroUsize;
use world_ops::{Operation, quantize_aabb, quantize_position};

struct ProbeDriver;

impl ReplicationDriver for ProbeDriver {
    type Snapshot = ();
    type Delta = ();
    type Input = ();
    type Error = io::Error;

    fn capture_snapshot(_world: &ecs::World) -> Result<Option<Self::Snapshot>, Self::Error> {
        Ok(Some(()))
    }

    fn build_delta(_previous: &Self::Snapshot, _current: &Self::Snapshot) -> Self::Delta {}

    fn apply_delta_to_snapshot(_base: &Self::Snapshot, _delta: &Self::Delta) -> Self::Snapshot {}

    fn map_codec_error(error: postcard::Error) -> Self::Error {
        io::Error::new(io::ErrorKind::InvalidData, error.to_string())
    }
}

impl SnapshotApplyDriver for ProbeDriver {
    fn apply_snapshot(
        _world: &mut ecs::World,
        _tick: SimulationTick,
        _snapshot: Self::Snapshot,
    ) -> Result<bool, Self::Error> {
        Ok(false)
    }

    fn apply_delta(
        _world: &mut ecs::World,
        _tick: SimulationTick,
        _delta: Self::Delta,
    ) -> Result<bool, Self::Error> {
        Ok(false)
    }
}

impl InputDriver for ProbeDriver {
    fn receive_remote_input(
        _world: &mut ecs::World,
        _connection: ConnectionHandle,
        _tick: SimulationTick,
        _input: Vec<Self::Input>,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    fn take_local_input(_world: &mut ecs::World) -> Result<Vec<Self::Input>, Self::Error> {
        Ok(Vec::new())
    }

    fn apply_input(_world: &mut ecs::World, _input: &[Self::Input]) -> Result<(), Self::Error> {
        Ok(())
    }
}

fn protocol_contract() -> ProtocolContract {
    ProtocolContract::new(ProtocolId::new(1), ProtocolRevision::new(1))
}

fn compatibility_offer() -> CompatibilityOffer {
    CompatibilityOffer::new(vec![protocol_contract()], vec![], vec![], None)
}

fn admit_connection(
    projection: &mut RunenNetSessionProjection,
    participant: ParticipantId,
    connection: ConnectionHandle,
) -> RunenNetSessionCore {
    let mut negotiation =
        NegotiationManager::new(OfferLimits::default(), NegotiationManagerLimits::default())
            .expect("test negotiation limits must be valid");
    negotiation
        .start(connection, compatibility_offer(), compatibility_offer())
        .expect("compatible negotiation must start");
    negotiation
        .propose(
            connection,
            NegotiatedContract::new(protocol_contract()),
            &NegotiationRequirements::default(),
        )
        .expect("compatible contract must be proposed");
    negotiation
        .validate_authority(connection)
        .expect("authority validation must succeed");
    negotiation
        .validate_peer(connection)
        .expect("peer validation must establish compatibility");

    let capacity = NonZeroUsize::new(4).expect("session capacity must be non-zero");
    let limits = SessionLimits::new(capacity, capacity).expect("session limits must be valid");
    let session = Session::new(SessionId::new(1), limits);
    let mut core = RunenNetSessionCore::new(negotiation, session);
    core.admit_established(projection, participant, connection)
        .expect("established connection must be admitted");
    core
}

#[test]
fn admitted_runennet_connection_streams_without_ecs_ownership_target() {
    let mut app = App::headless();
    app.add_plugin(WorldPlugin);
    app.add_plugin(NetPlugin::<ProbeDriver>::new(NetRole::Server));

    let connection = ConnectionHandle::new(55);
    let participant = ParticipantId::new(55);
    let mut projection = RunenNetSessionProjection::default();
    let _core = admit_connection(&mut projection, participant, connection);
    app.world_mut().insert_resource(projection);
    sync_runennet_session_projection(app.world_mut());

    let fixed_point_scale = **app
        .world()
        .resource::<WorldQuantizationScaleResource>()
        .expect("world quantization scale should exist");
    submit_world_operation(
        app.world_mut(),
        Operation::Stamp {
            stamp_id: "tests.world.connection-native-streaming".to_string(),
            anchor_q: quantize_position([2.0, 0.0, -2.0], fixed_point_scale),
            payload: vec![4, 3, 2, 1],
        },
        quantize_aabb([-1.0, -1.0, -1.0], [3.0, 1.0, 3.0], fixed_point_scale),
        WorldEditIngressMeta {
            planet_id: WorldId::new(0),
            deterministic_seed: 17,
        },
    )
    .expect("world operation should be accepted");

    let app = app
        .run_for_ticks(1)
        .expect("fixed tick should prepare per-connection streaming state");

    let streaming = app
        .world()
        .resource::<NetStreamingStateResource>()
        .expect("streaming state should exist");
    let connection_state = streaming
        .per_connection
        .get(&connection)
        .expect("admitted connection should have streaming state");

    assert!(
        !connection_state.relevant_chunks.is_empty(),
        "RunenNet admission alone should make authoritative runtime chunks streamable"
    );
}
