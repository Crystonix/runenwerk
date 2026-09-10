use super::super::adapters::resources::{
    BuildGraphResource, BuildQueueResource, OperationLogResource, PartitionConfigResource,
    WorldQuantizationScaleResource,
};
use super::super::chunks::lifecycle::{ChunkLifecycleState, WorldChunkRuntimeMapResource};
use super::super::debug::metrics::WorldDebugMetricsResource;
use super::integration::{WorldCompletedBuildOutput, WorldCompletedBuildQueueResource};
use crate::runtime::{Res, ResMut};
use runen_spatial::{ChunkId, GridPartitionConfig, SpatialMathError};
use std::collections::BTreeMap;
use std::hash::{Hash, Hasher};
use world_ops::{
    BuildGeneration, BuildGraphNode, BuildGraphPhase, BuildQueueClass, BuildQueueItem,
    ChunkGeneration, ChunkRevision, CsgBooleanMode, DirtyReasonSet, Operation, OperationId,
    OperationLog, OperationRecord, WorldQuantizationScale, touched_chunks_from_quantized_bounds,
};
use world_sdf::{
    RegionSdfSummary, SdfBrickMetadata, SdfBrickRecord, SdfBrickSamples, SdfChunkPayload,
    SdfPageCoord3, SdfPageRecord,
};

#[derive(Debug, Copy, Clone, PartialEq, Eq, runen_ecs::Resource)]
pub enum WorldBuildStaleness {
    Current,
    Superseded,
    InvalidBase,
}

#[derive(Debug, Clone, runen_ecs::Resource)]
pub struct WorldBuildJob {
    pub chunk_id: ChunkId,
    pub target_chunk_revision: ChunkRevision,
    pub target_build_generation: BuildGeneration,
}

#[derive(Debug, Clone, runen_ecs::Component, runen_ecs::Resource)]
pub struct WorldBuildJobRuntimeResource {
    pub max_jobs_per_update: usize,
    pub enqueued_jobs: u64,
    pub completed_jobs: u64,
    pub dropped_jobs: u64,
    pub spatial_mapping_failures: u64,
}

impl Default for WorldBuildJobRuntimeResource {
    fn default() -> Self {
        Self {
            max_jobs_per_update: 4,
            enqueued_jobs: 0,
            completed_jobs: 0,
            dropped_jobs: 0,
            spatial_mapping_failures: 0,
        }
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "ECS resources are injected independently; quantization policy must remain separate from partition topology"
)]
pub fn dispatch_world_build_jobs_system(
    mut chunks: ResMut<WorldChunkRuntimeMapResource>,
    mut queue: ResMut<BuildQueueResource>,
    mut graph: ResMut<BuildGraphResource>,
    mut runtime: ResMut<WorldBuildJobRuntimeResource>,
    mut completed: ResMut<WorldCompletedBuildQueueResource>,
    partition: Res<PartitionConfigResource>,
    quantization_scale: Res<WorldQuantizationScaleResource>,
    op_log: Res<OperationLogResource>,
) {
    queue_dirty_chunks(&chunks, &mut queue, &mut runtime);

    for _ in 0..runtime.max_jobs_per_update {
        let Some(item) = queue.pop_next() else {
            break;
        };

        let (target_chunk_revision, target_build_generation) = {
            let record = chunks.ensure_chunk(item.chunk_id);
            if !matches!(
                record.lifecycle,
                ChunkLifecycleState::Dirty | ChunkLifecycleState::Rebuilding
            ) {
                runtime.dropped_jobs = runtime.dropped_jobs.saturating_add(1);
                continue;
            }
            (
                ChunkRevision(record.chunk_revision.0.saturating_add(1)),
                BuildGeneration(record.build_generation.0.saturating_add(1)),
            )
        };

        let payload = match build_chunk_payload_from_op_window(
            item.chunk_id,
            target_chunk_revision,
            target_build_generation,
            &partition,
            &op_log,
            **quantization_scale,
        ) {
            Ok(payload) => payload,
            Err(_) => {
                runtime.dropped_jobs = runtime.dropped_jobs.saturating_add(1);
                runtime.spatial_mapping_failures =
                    runtime.spatial_mapping_failures.saturating_add(1);
                continue;
            }
        };

        let record = chunks.ensure_chunk(item.chunk_id);
        // Consume dirty state only after checked spatial mapping succeeds. A mapping failure leaves
        // the chunk Dirty and retryable instead of silently treating the operation as unrelated.
        record.dirty_reasons = DirtyReasonSet::default();
        record.pending_build_generation = Some(target_build_generation);
        record.lifecycle = ChunkLifecycleState::Rebuilding;

        push_phase_nodes(
            &mut graph,
            item.chunk_id,
            target_chunk_revision,
            target_build_generation,
        );

        let region_summary = summarize_region_from_payload(&payload);
        completed.outputs.push_back(WorldCompletedBuildOutput {
            chunk_id: item.chunk_id,
            target_chunk_revision,
            target_build_generation,
            staleness: WorldBuildStaleness::Current,
            chunk_payload: payload,
            region_summary,
        });
        runtime.completed_jobs = runtime.completed_jobs.saturating_add(1);
    }
}

pub fn sync_world_build_debug_metrics_system(
    queue: Res<BuildQueueResource>,
    runtime: Res<WorldBuildJobRuntimeResource>,
    mut debug: ResMut<WorldDebugMetricsResource>,
) {
    let (interactive_depth, background_depth) = queue.queue_depths();
    debug.interactive_queue_depth = interactive_depth;
    debug.background_queue_depth = background_depth;
    debug.enqueued_build_jobs = runtime.enqueued_jobs;
    debug.completed_build_jobs = runtime.completed_jobs;
    debug.build_spatial_mapping_failures = runtime.spatial_mapping_failures;
}

fn queue_dirty_chunks(
    chunks: &WorldChunkRuntimeMapResource,
    queue: &mut BuildQueueResource,
    runtime: &mut WorldBuildJobRuntimeResource,
) {
    for record in chunks.by_chunk_id.values() {
        if !matches!(record.lifecycle, ChunkLifecycleState::Dirty) {
            continue;
        }
        if record.pending_build_generation.is_some() || queue.contains_chunk(record.chunk_id) {
            continue;
        }
        let queue_class = if record.gameplay_locked {
            BuildQueueClass::Interactive
        } else {
            BuildQueueClass::Background
        };
        let priority_score =
            build_priority_score(record.chunk_id, queue_class, record.gameplay_locked);
        queue.enqueue(BuildQueueItem {
            chunk_id: record.chunk_id,
            queue_class,
            priority_score,
            starvation_age: 0,
        });
        runtime.enqueued_jobs = runtime.enqueued_jobs.saturating_add(1);
    }
}

fn build_priority_score(chunk_id: ChunkId, class: BuildQueueClass, gameplay_locked: bool) -> i64 {
    let base = match class {
        BuildQueueClass::Interactive => 1_000_i64,
        BuildQueueClass::Background => 100_i64,
    };
    let lock_bonus = if gameplay_locked { 500 } else { 0 };
    let coord_hash = chunk_id.coord.x.wrapping_mul(73856093)
        ^ chunk_id.coord.y.wrapping_mul(19349663)
        ^ chunk_id.coord.z.wrapping_mul(83492791);
    base.saturating_add(lock_bonus)
        .saturating_add(coord_hash.abs() % 97)
}

fn push_phase_nodes(
    graph: &mut BuildGraphResource,
    chunk_id: ChunkId,
    target_chunk_revision: ChunkRevision,
    target_build_generation: BuildGeneration,
) {
    let phases = [
        BuildGraphPhase::DirtyPlan,
        BuildGraphPhase::OpWindowResolve,
        BuildGraphPhase::SdfFieldBuild,
        BuildGraphPhase::SummaryBuild,
        BuildGraphPhase::DerivedRenderBuild,
        BuildGraphPhase::Publish,
    ];
    for phase in phases {
        graph.nodes.push(BuildGraphNode {
            chunk_id,
            phase,
            target_chunk_revision,
            input_generation_stamp: target_build_generation,
        });
    }
}

fn build_chunk_payload_from_op_window(
    chunk_id: ChunkId,
    target_chunk_revision: ChunkRevision,
    target_build_generation: BuildGeneration,
    partition: &GridPartitionConfig,
    op_log: &OperationLog,
    fixed_point_scale: WorldQuantizationScale,
) -> Result<SdfChunkPayload, SpatialMathError> {
    let affecting_ops = operations_affecting_chunk(op_log, partition, chunk_id, fixed_point_scale)?;
    let (solid_payload, last_op_id, material_channel_mask) =
        chunk_solid_state_from_operations(&affecting_ops);
    let mut page_table = BTreeMap::<SdfPageCoord3, SdfPageRecord>::new();
    if solid_payload {
        let brick = SdfBrickRecord {
            metadata: SdfBrickMetadata {
                min_distance: -16,
                max_distance: 16,
                occupancy_mask: 0xFF,
                material_channel_mask,
                last_touched_op_id: last_op_id,
                surface_band_present: true,
                compression_scheme: 1,
            },
            samples: SdfBrickSamples {
                distances: vec![-8; 8],
            },
        };
        let mut page = SdfPageRecord {
            page_generation: target_build_generation.0,
            bricks: BTreeMap::new(),
        };
        page.bricks.insert([0, 0, 0], brick);
        page_table.insert(SdfPageCoord3 { x: 0, y: 0, z: 0 }, page);
    }

    let mut checksum_hasher = std::collections::hash_map::DefaultHasher::new();
    chunk_id.hash(&mut checksum_hasher);
    target_chunk_revision.0.hash(&mut checksum_hasher);
    target_build_generation.0.hash(&mut checksum_hasher);
    solid_payload.hash(&mut checksum_hasher);
    for record in &affecting_ops {
        operation_signature(record).hash(&mut checksum_hasher);
    }

    Ok(SdfChunkPayload {
        chunk_id,
        chunk_revision: target_chunk_revision,
        chunk_generation: ChunkGeneration(target_build_generation.0),
        page_table,
        hierarchy_revision: last_op_id.0.max(target_build_generation.0),
        checksum: checksum_hasher.finish(),
    })
}

fn operations_affecting_chunk<'a>(
    op_log: &'a OperationLog,
    partition: &GridPartitionConfig,
    chunk_id: ChunkId,
    fixed_point_scale: WorldQuantizationScale,
) -> Result<Vec<&'a OperationRecord>, SpatialMathError> {
    let mut affecting = Vec::new();
    for record in &op_log.operations {
        if record.planet_id != chunk_id.world_id {
            continue;
        }
        let touched_chunks = touched_chunks_from_quantized_bounds(
            partition,
            record.affected_bounds_q,
            chunk_id.world_id,
            fixed_point_scale,
        )?;
        if touched_chunks.contains(&chunk_id) {
            affecting.push(record);
        }
    }
    Ok(affecting)
}

fn chunk_solid_state_from_operations(records: &[&OperationRecord]) -> (bool, OperationId, u16) {
    let mut solid_payload = false;
    let mut material_channel_mask = 0_u16;
    let mut last_op_id = OperationId::default();

    for record in records {
        last_op_id = record.op_id;
        match &record.operation {
            Operation::CsgAdd {
                material_channel, ..
            } => {
                solid_payload = true;
                material_channel_mask |= material_channel_bit(*material_channel);
            }
            Operation::CsgBrush(brush) => match brush.mode {
                CsgBooleanMode::Add | CsgBooleanMode::SmoothAdd { .. } => {
                    solid_payload = true;
                    if let Some(material_channel) = brush.material_channel {
                        material_channel_mask |= material_channel_bit(material_channel);
                    }
                }
                CsgBooleanMode::Subtract | CsgBooleanMode::SmoothSubtract { .. } => {
                    solid_payload = false;
                }
                CsgBooleanMode::Intersect | CsgBooleanMode::SmoothIntersect { .. } => {
                    if solid_payload && let Some(material_channel) = brush.material_channel {
                        material_channel_mask |= material_channel_bit(material_channel);
                    }
                }
            },
            Operation::Stamp { .. } => {
                solid_payload = true;
                material_channel_mask |= 1;
            }
            Operation::StructurePlace { .. } => {
                solid_payload = true;
                material_channel_mask |= 1;
            }
            Operation::CsgSubtract { .. } | Operation::StructureRemove { .. } => {
                solid_payload = false;
            }
            Operation::MaterialFieldEdit { channel_mask, .. } => {
                material_channel_mask |= *channel_mask;
            }
            Operation::Smooth { .. } | Operation::DensityFieldDeform { .. } => {}
        }
    }

    if solid_payload && material_channel_mask == 0 {
        material_channel_mask = 1;
    }

    (solid_payload, last_op_id, material_channel_mask)
}

fn material_channel_bit(material_channel: u16) -> u16 {
    1_u16
        .checked_shl((material_channel as u32).min(15))
        .unwrap_or(1)
}

fn operation_signature(record: &OperationRecord) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    record.op_id.0.hash(&mut hasher);
    record.base_world_revision.0.hash(&mut hasher);
    record.planet_id.hash(&mut hasher);
    record.deterministic_seed.hash(&mut hasher);
    format!("{:?}", record.operation).hash(&mut hasher);
    hasher.finish()
}

fn summarize_region_from_payload(payload: &SdfChunkPayload) -> RegionSdfSummary {
    let mut summary = RegionSdfSummary::default();
    let mut saw_any = false;
    for page in payload.page_table.values() {
        for brick in page.bricks.values() {
            if !saw_any {
                summary.min_distance = brick.metadata.min_distance;
                summary.max_distance = brick.metadata.max_distance;
                saw_any = true;
            } else {
                summary.min_distance = summary.min_distance.min(brick.metadata.min_distance);
                summary.max_distance = summary.max_distance.max(brick.metadata.max_distance);
            }
            if brick.metadata.occupancy_mask != 0 {
                summary.occupied_chunk_count = 1;
            }
            if brick.metadata.surface_band_present {
                summary.surface_chunk_count = 1;
            }
        }
    }
    summary
}
