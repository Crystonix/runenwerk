use engine::SimulationTick;
use engine::net::prelude::{Ack, ClientMessage, NetPlugin, NetRole, SnapshotCursor};
use engine::plugins::net::{
    NetStreamingStateResource, RunenNetSessionCore, RunenNetSessionProjection,
    enqueue_server_inbox_from, sync_runennet_session_projection,
};
use engine::plugins::world::adapters::resources::{
    PartitionConfigResource, ReplicationStateResource, WorldQuantizationScaleResource,
};
use engine::plugins::world::edits::ingress::{WorldEditIngressMeta, submit_world_operation};
use engine::plugins::world::plugin::{WorldAuthorityState, WorldPlugin};
use engine::prelude::App;
use engine_net::replication::{InputDriver, ReplicationDriver, SnapshotApplyDriver};
use runen_net::identity::{ConnectionHandle, ParticipantId, SessionId};
use runen_net::protocol::{
    CompatibilityOffer, NegotiatedContract, NegotiationManager, NegotiationManagerLimits,
    NegotiationRequirements, OfferLimits, ProtocolContract, ProtocolId, ProtocolRevision,
};
use runen_net::session::{RetentionPolicy, Session, SessionLimits};
use runen_spatial::{ChunkCoord3, ChunkId, GridPartitionConfig, WorldId};
use serde::{Deserialize, Serialize};
use std::io;
use std::num::NonZeroUsize;
use world_ops::{
    BrushShape, DirtyChunkMap, Operation, OperationId, OperationLog, OperationRecord,
    RegionInvalidationDelta, ReplayWindow, SyncCursor, WorldQuantizationScale, WorldRevision,
    mark_dirty_chunks_from_operation_log, operations_for_replay_window, quantize_aabb,
    quantize_position,
};

fn test_quantization_scale() -> WorldQuantizationScale {
    WorldQuantizationScale::try_new(1024).expect("test quantization scale is valid")
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
    let capacity = NonZeroUsize::new(4).expect("test session capacity must be non-zero");
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

fn build_test_log() -> OperationLog {
    let mut log = OperationLog::default();
    let scale = test_quantization_scale();
    let operations = [
        Operation::CsgSubtract {
            brush: BrushShape::Sphere {
                center_q: quantize_position([2.0, 0.0, -1.0], scale),
                radius_q: 1536,
            },
        },
        Operation::Smooth {
            bounds_q: quantize_aabb([-6.0, -2.0, -6.0], [6.0, 2.0, 6.0], scale),
            kernel_radius_q: 512,
            strength_q: 192,
        },
        Operation::MaterialFieldEdit {
            bounds_q: quantize_aabb([-4.0, -1.0, -4.0], [4.0, 1.0, 4.0], scale),
            channel_mask: 0b0011,
            payload: vec![1, 2, 3, 4],
        },
    ];

    for operation in operations {
        let bounds_q = match &operation {
            Operation::CsgSubtract { .. } => {
                quantize_aabb([-3.0, -3.0, -3.0], [3.0, 3.0, 3.0], scale)
            }
            Operation::Smooth { bounds_q, .. } => *bounds_q,
            Operation::MaterialFieldEdit { bounds_q, .. } => *bounds_q,
            _ => quantize_aabb([-1.0, -1.0, -1.0], [1.0, 1.0, 1.0], scale),
        };
        let _ = log.append(OperationRecord {
            op_id: OperationId(0),
            base_world_revision: WorldRevision(1),
            planet_id: WorldId::new(0),
            operation,
            affected_bounds_q: bounds_q,
            deterministic_seed: 1337,
        });
    }
    log
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct ReplicationProbeSnapshot {
    world_revision: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
struct ReplicationProbeDelta {
    changed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
struct ReplicationProbeInput;

struct ReplicationProbeDriver;

impl ReplicationDriver for ReplicationProbeDriver {
    type Snapshot = ReplicationProbeSnapshot;
    type Delta = ReplicationProbeDelta;
    type Input = ReplicationProbeInput;
    type Error = io::Error;

    fn capture_snapshot(world: &ecs::World) -> Result<Option<Self::Snapshot>, Self::Error> {
        let world_revision = world
            .resource::<WorldAuthorityState>()
            .map(|state| state.world_revision.0)
            .unwrap_or(0);
        Ok(Some(ReplicationProbeSnapshot { world_revision }))
    }

    fn build_delta(previous: &Self::Snapshot, current: &Self::Snapshot) -> Self::Delta {
        ReplicationProbeDelta {
            changed: previous != current,
        }
    }

    fn apply_delta_to_snapshot(base: &Self::Snapshot, delta: &Self::Delta) -> Self::Snapshot {
        if delta.changed {
            Self::Snapshot {
                world_revision: base.world_revision.saturating_add(1),
            }
        } else {
            base.clone()
        }
    }

    fn map_codec_error(error: postcard::Error) -> Self::Error {
        io::Error::new(io::ErrorKind::InvalidData, error.to_string())
    }
}

impl SnapshotApplyDriver for ReplicationProbeDriver {
    fn apply_snapshot(
        _world: &mut ecs::World,
        _tick: SimulationTick,
        _snapshot: Self::Snapshot,
    ) -> Result<bool, Self::Error> {
        Ok(true)
    }

    fn apply_delta(
        _world: &mut ecs::World,
        _tick: SimulationTick,
        _delta: Self::Delta,
    ) -> Result<bool, Self::Error> {
        Ok(true)
    }
}

impl InputDriver for ReplicationProbeDriver {
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

#[test]
fn op_log_replay_and_invalidation_are_deterministic() {
    let log_a = build_test_log();
    let log_b = build_test_log();

    let replay_window = ReplayWindow {
        applied_op_exclusive: OperationId(0),
        target_op_inclusive: OperationId(3),
    };

    let replay_a = operations_for_replay_window(&log_a, replay_window);
    let replay_b = operations_for_replay_window(&log_b, replay_window);
    assert_eq!(replay_a, replay_b, "replay output must be deterministic");

    let partition = GridPartitionConfig::default();
    let scale = test_quantization_scale();
    let mut dirty_a = DirtyChunkMap::default();
    let mut dirty_b = DirtyChunkMap::default();
    mark_dirty_chunks_from_operation_log(&mut dirty_a, &partition, &log_a, scale)
        .expect("test operation bounds should map to chunks");
    mark_dirty_chunks_from_operation_log(&mut dirty_b, &partition, &log_b, scale)
        .expect("test operation bounds should map to chunks");
    assert_eq!(
        dirty_a.by_chunk, dirty_b.by_chunk,
        "dirty invalidation set must be deterministic for identical op logs"
    );
}

#[test]
fn world_replication_state_is_built_from_world_runtime() {
    let mut app = App::headless();
    app.add_plugin(WorldPlugin);

    let fixed_point_scale = **app
        .world()
        .resource::<WorldQuantizationScaleResource>()
        .expect("world quantization scale should exist");
    let op_id = submit_world_operation(
        app.world_mut(),
        Operation::Stamp {
            stamp_id: "tests.world.replication-runtime".to_string(),
            anchor_q: quantize_position([1.0, 1.0, 1.0], fixed_point_scale),
            payload: vec![9, 8, 7, 6],
        },
        quantize_aabb([0.0, 0.0, 0.0], [1.0, 1.0, 1.0], fixed_point_scale),
        WorldEditIngressMeta {
            planet_id: WorldId::new(0),
            deterministic_seed: 99,
        },
    );
    assert!(op_id.is_some(), "world ingress should append operation");

    let app = app
        .run_for_ticks(1)
        .expect("world runtime should process one fixed tick");

    let replication = app
        .world()
        .resource::<ReplicationStateResource>()
        .expect("world replication state should exist");
    let authority = app
        .world()
        .resource::<WorldAuthorityState>()
        .expect("world authority state should exist");

    assert_eq!(
        replication.world_revision, authority.world_revision,
        "replication state revision should track authoritative world revision"
    );
    assert_eq!(
        replication.next_op_id.0, 2,
        "replication next op id should advance with ingress operation log"
    );
    assert_eq!(
        replication.pending_op_windows.len(),
        1,
        "replication state should publish op-window deltas from world runtime"
    );
    assert_eq!(
        replication.pending_op_windows[0].operations.len(),
        1,
        "replication op-window should include submitted operation"
    );
    assert!(
        !replication.pending_header_deltas.is_empty(),
        "replication state should publish chunk header deltas from runtime chunks"
    );
    assert!(
        !replication.pending_content_deltas.is_empty(),
        "replication state should publish chunk content deltas from authoritative store"
    );
    assert!(
        replication
            .pending_content_deltas
            .values()
            .all(|value| value
                .full_payload
                .as_ref()
                .is_some_and(|payload| !payload.is_empty())),
        "content deltas should carry serialized authoritative chunk payload snapshots"
    );
    let partition = app
        .world()
        .resource::<PartitionConfigResource>()
        .expect("world partition config should exist");
    let expected_chunk = ChunkId::new(WorldId::new(0), ChunkCoord3 { x: 0, y: 0, z: 0 });
    let expected_region = partition.region_id_from_chunk_id(expected_chunk);
    assert!(
        replication
            .pending_region_invalidations
            .iter()
            .any(|record| record.chunk_ids.contains(&expected_chunk)
                && record.region_ids.contains(&expected_region)),
        "replication state should surface region+chunk invalidation records from world-owned journal"
    );
    assert!(
        replication
            .pending_region_invalidations
            .windows(2)
            .all(|pair| pair[0].sequence < pair[1].sequence),
        "region invalidation projection should preserve journal sequence ordering"
    );
}

#[test]
fn world_region_invalidation_projection_is_deterministic() {
    fn run_projection() -> Vec<RegionInvalidationDelta> {
        let mut app = App::headless();
        app.add_plugin(WorldPlugin);
        let fixed_point_scale = **app
            .world()
            .resource::<WorldQuantizationScaleResource>()
            .expect("world quantization scale should exist");
        let operations = [
            (
                quantize_aabb([0.0, 0.0, 0.0], [40.0, 1.0, 1.0], fixed_point_scale),
                31_u64,
            ),
            (
                quantize_aabb([-24.0, -2.0, -24.0], [-1.0, 2.0, -1.0], fixed_point_scale),
                32_u64,
            ),
        ];
        for (bounds_q, seed) in operations {
            let op_id = submit_world_operation(
                app.world_mut(),
                Operation::Stamp {
                    stamp_id: "tests.world.region-journal".to_string(),
                    anchor_q: quantize_position([0.0, 0.0, 0.0], fixed_point_scale),
                    payload: vec![5, 4, 3, 2],
                },
                bounds_q,
                WorldEditIngressMeta {
                    planet_id: WorldId::new(0),
                    deterministic_seed: seed,
                },
            );
            assert!(op_id.is_some(), "ingress op should append to journal");
        }

        let app = app
            .run_for_ticks(1)
            .expect("world tick should publish replication projection");
        app.world()
            .resource::<ReplicationStateResource>()
            .expect("replication resource should exist")
            .pending_region_invalidations
            .clone()
    }

    let projection_a = run_projection();
    let projection_b = run_projection();
    assert_eq!(
        projection_a, projection_b,
        "region invalidation projection must be deterministic across equivalent runs"
    );
}

#[test]
fn world_streaming_interest_tracks_connection_cursor_and_cleanup() {
    let mut app = App::headless();
    app.add_plugin(WorldPlugin);
    app.add_plugin(NetPlugin::<ReplicationProbeDriver>::new(NetRole::Server));

    let connection = ConnectionHandle::new(55);
    let participant = ParticipantId::new(55);
    let mut core = test_runennet_session_core();
    let mut projection = RunenNetSessionProjection::default();
    establish_runennet_connection(&mut core, &mut projection, participant, connection);
    app.world_mut().insert_resource(projection.clone());
    sync_runennet_session_projection(app.world_mut());

    let fixed_point_scale = **app
        .world()
        .resource::<WorldQuantizationScaleResource>()
        .expect("world quantization scale should exist");
    let _ = submit_world_operation(
        app.world_mut(),
        Operation::Stamp {
            stamp_id: "tests.world.streaming-interest".to_string(),
            anchor_q: quantize_position([2.0, 0.0, -2.0], fixed_point_scale),
            payload: vec![4, 3, 2, 1],
        },
        quantize_aabb([-1.0, -1.0, -1.0], [3.0, 1.0, 3.0], fixed_point_scale),
        WorldEditIngressMeta {
            planet_id: WorldId::new(0),
            deterministic_seed: 17,
        },
    );

    let mut app = app
        .run_for_ticks(1)
        .expect("fixed tick should produce one replication step");

    {
        let interest = app
            .world()
            .resource::<NetStreamingStateResource>()
            .expect("world streaming interest should exist");
        let per_connection = interest
            .per_connection
            .get(&connection)
            .expect("active connection should have streaming interest");
        assert!(
            !per_connection.relevant_chunks.is_empty(),
            "admitted connection should track authoritative runtime chunks without an ECS ownership target"
        );
        assert_eq!(
            per_connection.last_sent_cursor,
            SyncCursor(1),
            "streaming cursor should advance after first replicated snapshot"
        );
        assert_eq!(
            per_connection.last_ack_cursor,
            SyncCursor(0),
            "ack cursor should remain at zero before client acknowledgment"
        );
        assert!(
            !per_connection.needs_full_resync,
            "full snapshot send should clear full-resync requirement"
        );
        assert!(
            per_connection.prepared_region_sequence > 0,
            "streaming interest should stage region journal sequence coverage before snapshot send"
        );
    }

    enqueue_server_inbox_from(
        app.world_mut(),
        Some(connection),
        ClientMessage::Ack(Ack {
            cursor: SnapshotCursor(1),
            last_received_tick: SimulationTick(9),
        }),
    )
    .expect("server inbox enqueue should succeed");
    let next_tick = app
        .world()
        .resource::<SimulationTick>()
        .expect("simulation tick should exist")
        .0
        .saturating_add(1);
    app = app
        .run_for_ticks(next_tick)
        .expect("ack tick should update world streaming cursor state");

    let second_chunk = ChunkId::new(WorldId::new(0), ChunkCoord3 { x: 2, y: 0, z: 0 });
    {
        let interest = app
            .world()
            .resource::<NetStreamingStateResource>()
            .expect("world streaming interest should exist");
        let per_connection = interest
            .per_connection
            .get(&connection)
            .expect("active connection should have streaming interest");
        assert_eq!(
            per_connection.last_ack_cursor,
            SyncCursor(1),
            "server ack processing should advance per-connection ack cursor"
        );
    }

    let _ = submit_world_operation(
        app.world_mut(),
        Operation::Stamp {
            stamp_id: "tests.world.streaming-interest.region-delta".to_string(),
            anchor_q: quantize_position([80.0, 0.0, 0.0], fixed_point_scale),
            payload: vec![8, 8, 8, 8],
        },
        quantize_aabb([80.0, 0.0, 0.0], [80.0, 0.0, 0.0], fixed_point_scale),
        WorldEditIngressMeta {
            planet_id: WorldId::new(0),
            deterministic_seed: 18,
        },
    );
    let next_tick = app
        .world()
        .resource::<SimulationTick>()
        .expect("simulation tick should exist")
        .0
        .saturating_add(1);
    app = app
        .run_for_ticks(next_tick)
        .expect("region delta tick should update per-connection relevant chunks");

    {
        let interest = app
            .world()
            .resource::<NetStreamingStateResource>()
            .expect("world streaming interest should exist");
        let per_connection = interest
            .per_connection
            .get(&connection)
            .expect("active connection should have streaming interest");
        assert_eq!(
            per_connection.last_ack_cursor,
            SyncCursor(1),
            "server ack processing should advance per-connection ack cursor"
        );
        assert!(
            per_connection.relevant_chunks.contains(&second_chunk),
            "post-ack region invalidation policy should surface only newly invalidated chunks"
        );
    }

    core.connection_lost(
        &mut projection,
        participant,
        connection,
        RetentionPolicy::Terminate,
    )
    .expect("RunenNet terminal connection loss should succeed");
    app.world_mut().insert_resource(projection);
    sync_runennet_session_projection(app.world_mut());

    let cleanup_tick = app
        .world()
        .resource::<SimulationTick>()
        .expect("simulation tick should exist")
        .0
        .saturating_add(1);
    app = app
        .run_for_ticks(cleanup_tick)
        .expect("post-loss tick should clean streaming projection state");

    let interest = app
        .world()
        .resource::<NetStreamingStateResource>()
        .expect("world streaming interest should exist");
    assert!(
        !interest.per_connection.contains_key(&connection),
        "streaming interest must drop entries after RunenNet terminal connection loss"
    );
}
