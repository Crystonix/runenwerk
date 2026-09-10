use super::super::adapters::resources::{
    OperationLogResource, PartitionConfigResource, RegionInvalidationJournalResource,
    WorldQuantizationScaleResource,
};
use super::super::chunks::DirtyChunkMapResource;
use super::super::chunks::render_cache_bridge::WorldRenderCacheInvalidationQueueResource;
use super::super::debug::metrics::WorldDebugMetricsResource;
use super::super::{WorldAuthorityState, WorldRuntimeConfig, WorldRuntimeMode};
use runen_ecs::World;
use runen_spatial::WorldId;
use world_ops::{
    Operation, OperationId, OperationRecord, QuantizedAabb, dirty_reason_for_operation,
    touched_chunks_from_quantized_bounds,
};

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct WorldEditIngressMeta {
    pub planet_id: WorldId,
    pub deterministic_seed: u64,
}

impl Default for WorldEditIngressMeta {
    fn default() -> Self {
        Self {
            planet_id: WorldId::new(0),
            deterministic_seed: 0,
        }
    }
}

pub fn submit_world_operation(
    world: &mut World,
    operation: Operation,
    affected_bounds_q: QuantizedAabb,
    meta: WorldEditIngressMeta,
) -> Option<OperationId> {
    if !world_runtime_is_authoritative(world) {
        return None;
    }

    let base_world_revision = world
        .resource::<WorldAuthorityState>()
        .map(|value| value.world_revision)
        .unwrap_or_default();
    let dirty_reason = dirty_reason_for_operation(&operation);
    let partition = world.resource::<PartitionConfigResource>().ok()?.clone();
    let quantization_scale = **world.resource::<WorldQuantizationScaleResource>().ok()?;
    let touched_chunks = touched_chunks_from_quantized_bounds(
        &partition,
        affected_bounds_q,
        meta.planet_id,
        quantization_scale,
    )
    .ok()?;

    let op_id = {
        let op_log = world.resource_mut::<OperationLogResource>().ok()?;
        op_log.append(OperationRecord {
            op_id: OperationId(0),
            base_world_revision,
            planet_id: meta.planet_id,
            operation,
            affected_bounds_q,
            deterministic_seed: meta.deterministic_seed,
        })
    };

    {
        let dirty = world.resource_mut::<DirtyChunkMapResource>().ok()?;
        for chunk_id in touched_chunks.iter().copied() {
            dirty.mark_dirty(chunk_id, dirty_reason);
        }
    }
    let invalidated_chunk_count = touched_chunks.len() as u64;

    if let Ok(queue) = world.resource_mut::<WorldRenderCacheInvalidationQueueResource>() {
        queue.enqueue_ingress_bounds(&partition, touched_chunks.clone());
    }
    if let Ok(journal) = world.resource_mut::<RegionInvalidationJournalResource>() {
        journal.append_ingress_record(&partition, touched_chunks, base_world_revision, op_id);
    }

    let op_count = world
        .resource::<OperationLogResource>()
        .map(|op_log| op_log.operations.len() as u64)
        .ok();
    if let (Some(op_count), Ok(metrics)) =
        (op_count, world.resource_mut::<WorldDebugMetricsResource>())
    {
        metrics.op_log_count = op_count;
        metrics.ingress_operations = metrics.ingress_operations.saturating_add(1);
        metrics.invalidated_chunks = metrics
            .invalidated_chunks
            .saturating_add(invalidated_chunk_count);
    }

    Some(op_id)
}

fn world_runtime_is_authoritative(world: &World) -> bool {
    world
        .resource::<WorldRuntimeConfig>()
        .map(|config| matches!(config.mode, WorldRuntimeMode::Writable))
        .unwrap_or(true)
}
