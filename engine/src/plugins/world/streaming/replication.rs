use super::super::adapters::resources::{
    OperationLogResource, RegionInvalidationJournalResource, ReplicationStateResource,
    SdfChunkStoreResource,
};
use super::super::chunks::lifecycle::WorldChunkRuntimeMapResource;
use super::super::plugin::WorldAuthorityState;
use crate::runtime::WorldMut;
use world_ops::{
    ChunkContentDelta, ChunkHeaderDelta, ChunkResidencyHint, OpWindowDelta, OperationId,
    RegionInvalidationDelta,
};

#[derive(Debug, Copy, Clone, Default, ecs::Component, ecs::Resource)]
pub struct WorldReplicationExtractionCursor {
    pub last_tick: u64,
}

fn world_replication_inputs_changed(world: &ecs::World, since_tick: u64) -> bool {
    world.resource_changed_since::<OperationLogResource>(since_tick)
        || world.resource_changed_since::<SdfChunkStoreResource>(since_tick)
        || world.resource_changed_since::<WorldChunkRuntimeMapResource>(since_tick)
        || world.resource_changed_since::<RegionInvalidationJournalResource>(since_tick)
        || world.resource_changed_since::<WorldAuthorityState>(since_tick)
}

pub fn rebuild_world_replication_state_system(mut world: WorldMut) {
    let current_tick = world.current_change_tick();
    let previous_tick = world
        .resource::<WorldReplicationExtractionCursor>()
        .map(|cursor| cursor.last_tick)
        .unwrap_or_default();

    if !world_replication_inputs_changed(&world, previous_tick) {
        return;
    }

    let world_revision = world
        .resource::<WorldAuthorityState>()
        .map(|authority| authority.world_revision)
        .unwrap_or_default();

    let next_op_id = world
        .resource::<OperationLogResource>()
        .map(|op_log| OperationId(op_log.next_op_id))
        .unwrap_or_default();

    let chunk_runtime_records = world
        .resource::<WorldChunkRuntimeMapResource>()
        .map(|chunk_runtime| {
            chunk_runtime
                .by_chunk_id
                .values()
                .map(|record| {
                    (
                        record.chunk_id,
                        record.chunk_revision,
                        record.chunk_generation,
                        record.gameplay_locked,
                    )
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let chunk_payloads = world
        .resource::<SdfChunkStoreResource>()
        .map(|sdf_store| sdf_store.chunks.values().cloned().collect::<Vec<_>>())
        .unwrap_or_default();

    let op_records = world
        .resource::<OperationLogResource>()
        .map(|op_log| op_log.operations.clone())
        .unwrap_or_default();

    let region_records = world
        .resource::<RegionInvalidationJournalResource>()
        .map(|journal| journal.recent_records.clone())
        .unwrap_or_default();

    let chunk_checksum_by_id = chunk_payloads
        .iter()
        .map(|payload| (payload.chunk_id, payload.checksum))
        .collect::<std::collections::BTreeMap<_, _>>();

    let pending_header_deltas = chunk_runtime_records
        .iter()
        .map(
            |(chunk_id, chunk_revision, chunk_generation, gameplay_locked)| {
                (
                    *chunk_id,
                    ChunkHeaderDelta {
                        chunk_id: *chunk_id,
                        chunk_revision: *chunk_revision,
                        chunk_generation: *chunk_generation,
                        checksum: chunk_checksum_by_id.get(chunk_id).copied().unwrap_or(0),
                        flags: if *gameplay_locked { 1 } else { 0 },
                    },
                )
            },
        )
        .collect::<std::collections::BTreeMap<_, _>>();

    let pending_residency_hints = chunk_runtime_records
        .iter()
        .map(|(chunk_id, _, _, gameplay_locked)| {
            (
                *chunk_id,
                ChunkResidencyHint {
                    chunk_id: *chunk_id,
                    relevant_to_viewer: true,
                    gameplay_locked: *gameplay_locked,
                },
            )
        })
        .collect::<std::collections::BTreeMap<_, _>>();

    let pending_content_deltas = chunk_payloads
        .iter()
        .map(|payload| {
            (
                payload.chunk_id,
                ChunkContentDelta {
                    chunk_id: payload.chunk_id,
                    chunk_revision: payload.chunk_revision,
                    page_deltas: Vec::new(),
                    full_payload: postcard::to_allocvec(payload).ok(),
                },
            )
        })
        .collect::<std::collections::BTreeMap<_, _>>();

    let pending_op_windows =
        if let (Some(first), Some(last)) = (op_records.first(), op_records.last()) {
            vec![OpWindowDelta {
                start_exclusive: OperationId(first.op_id.0.saturating_sub(1)),
                end_inclusive: last.op_id,
                operations: op_records,
            }]
        } else {
            Vec::new()
        };

    let pending_region_invalidations = region_records
        .iter()
        .map(|record| RegionInvalidationDelta {
            sequence: record.sequence,
            source: record.source,
            world_revision: record.world_revision,
            op_id: record.op_id,
            chunk_ids: record.chunk_ids.iter().copied().collect::<Vec<_>>(),
            region_ids: record.region_ids.iter().copied().collect::<Vec<_>>(),
        })
        .collect::<Vec<_>>();

    if let Ok(replication) = world.resource_mut::<ReplicationStateResource>() {
        replication.world_revision = world_revision;
        replication.next_op_id = next_op_id;
        replication.pending_header_deltas = pending_header_deltas;
        replication.pending_residency_hints = pending_residency_hints;
        replication.pending_content_deltas = pending_content_deltas;
        replication.pending_op_windows = pending_op_windows;
        replication.pending_region_invalidations = pending_region_invalidations;
    }

    if let Ok(cursor) = world.resource_mut::<WorldReplicationExtractionCursor>() {
        cursor.last_tick = current_tick;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Copy, Clone, ecs::Component)]
    struct UnrelatedComponent;

    #[derive(Debug, Copy, Clone, Default, ecs::Resource)]
    struct UnrelatedResource;

    fn world_with_replication_inputs() -> ecs::World {
        let mut world = ecs::World::new();
        world.insert_resource(OperationLogResource::default());
        world.insert_resource(SdfChunkStoreResource::default());
        world.insert_resource(WorldChunkRuntimeMapResource::default());
        world.insert_resource(RegionInvalidationJournalResource::default());
        world.insert_resource(WorldAuthorityState::default());
        world
    }

    #[test]
    fn dirty_detection_ignores_unrelated_component_and_resource_changes() {
        let mut world = world_with_replication_inputs();
        let baseline = world.current_change_tick();

        world
            .spawn(UnrelatedComponent)
            .expect("unrelated component should spawn");
        world.insert_resource(UnrelatedResource);

        assert!(!world_replication_inputs_changed(&world, baseline));
    }

    #[test]
    fn dirty_detection_tracks_each_replication_input() {
        let mut world = world_with_replication_inputs();

        let baseline = world.current_change_tick();
        let _ = world
            .resource_mut::<OperationLogResource>()
            .expect("operation log should exist");
        assert!(world_replication_inputs_changed(&world, baseline));

        let baseline = world.current_change_tick();
        let _ = world
            .resource_mut::<SdfChunkStoreResource>()
            .expect("SDF chunk store should exist");
        assert!(world_replication_inputs_changed(&world, baseline));

        let baseline = world.current_change_tick();
        let _ = world
            .resource_mut::<WorldChunkRuntimeMapResource>()
            .expect("world chunk runtime map should exist");
        assert!(world_replication_inputs_changed(&world, baseline));

        let baseline = world.current_change_tick();
        let _ = world
            .resource_mut::<RegionInvalidationJournalResource>()
            .expect("region invalidation journal should exist");
        assert!(world_replication_inputs_changed(&world, baseline));
    }

    #[test]
    fn dirty_detection_tracks_world_authority_independently() {
        let mut world = world_with_replication_inputs();
        let baseline = world.current_change_tick();

        world
            .resource_mut::<WorldAuthorityState>()
            .expect("world authority state should exist")
            .world_revision = world_ops::WorldRevision(7);

        assert!(world_replication_inputs_changed(&world, baseline));
    }
}
