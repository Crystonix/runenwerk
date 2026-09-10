use super::super::adapters::resources::{
    PartitionConfigResource, RegionInvalidationJournalResource, SdfChunkStoreResource,
};
use super::super::chunks::lifecycle::{ChunkLifecycleState, WorldChunkRuntimeMapResource};
use super::super::chunks::render_cache_bridge::WorldRenderCacheInvalidationQueueResource;
use super::super::debug::metrics::WorldDebugMetricsResource;
use super::super::plugin::{WorldAuthorityState, WorldRuntimeState};
use super::jobs::WorldBuildStaleness;
use crate::runtime::{Res, ResMut};
use runen_spatial::ChunkId;
use std::collections::VecDeque;
use world_ops::{BuildGeneration, ChunkGeneration, ChunkRevision};
use world_sdf::{RegionSdfSummary, SdfChunkPayload};

#[derive(Debug, Clone, runen_ecs::Resource)]
pub struct WorldCompletedBuildOutput {
    pub chunk_id: ChunkId,
    pub target_chunk_revision: ChunkRevision,
    pub target_build_generation: BuildGeneration,
    pub staleness: WorldBuildStaleness,
    pub chunk_payload: SdfChunkPayload,
    pub region_summary: RegionSdfSummary,
}

#[derive(Debug, Clone, Default, runen_ecs::Component, runen_ecs::Resource)]
pub struct WorldCompletedBuildQueueResource {
    pub outputs: VecDeque<WorldCompletedBuildOutput>,
}

#[derive(Debug, Clone)]
pub struct WorldSdfRuntimePayloadPackage {
    pub payloads: Vec<SdfChunkPayload>,
    pub region_summary: RegionSdfSummary,
}

impl WorldSdfRuntimePayloadPackage {
    pub fn new(payloads: Vec<SdfChunkPayload>, region_summary: RegionSdfSummary) -> Self {
        Self {
            payloads,
            region_summary,
        }
    }
}

pub fn enqueue_ratified_world_sdf_payload_package(
    completed: &mut WorldCompletedBuildQueueResource,
    chunks: &mut WorldChunkRuntimeMapResource,
    package: WorldSdfRuntimePayloadPackage,
) -> usize {
    let mut enqueued = 0usize;
    for payload in package.payloads {
        let chunk_id = payload.chunk_id;
        let build_generation = BuildGeneration(payload.chunk_generation.0);
        let record = chunks.ensure_chunk(chunk_id);
        record.pending_build_generation = Some(build_generation);
        completed.outputs.push_back(WorldCompletedBuildOutput {
            chunk_id,
            target_chunk_revision: payload.chunk_revision,
            target_build_generation: build_generation,
            staleness: WorldBuildStaleness::Current,
            chunk_payload: payload,
            region_summary: package.region_summary,
        });
        enqueued = enqueued.saturating_add(1);
    }
    enqueued
}

#[allow(clippy::too_many_arguments)]
pub fn integrate_completed_build_outputs_system(
    mut completed: ResMut<WorldCompletedBuildQueueResource>,
    mut chunks: ResMut<WorldChunkRuntimeMapResource>,
    mut sdf_store: ResMut<SdfChunkStoreResource>,
    partition: Res<PartitionConfigResource>,
    mut runtime: ResMut<WorldRuntimeState>,
    mut authority: ResMut<WorldAuthorityState>,
    mut render_cache_invalidation: ResMut<WorldRenderCacheInvalidationQueueResource>,
    mut region_invalidation_journal: ResMut<RegionInvalidationJournalResource>,
) {
    let mut integrated = 0_u64;
    let mut dropped = 0_u64;

    while let Some(output) = completed.outputs.pop_front() {
        if !matches!(output.staleness, WorldBuildStaleness::Current) {
            dropped = dropped.saturating_add(1);
            continue;
        }

        let record = chunks.ensure_chunk(output.chunk_id);
        if record.pending_build_generation != Some(output.target_build_generation) {
            dropped = dropped.saturating_add(1);
            continue;
        }
        if !payload_matches_target_contract(&output) {
            dropped = dropped.saturating_add(1);
            continue;
        }

        record.chunk_revision = output.target_chunk_revision;
        record.chunk_generation = output.chunk_payload.chunk_generation;
        record.build_generation = output.target_build_generation;
        record.pending_build_generation = None;
        if record.dirty_reasons.is_empty() {
            if !matches!(record.lifecycle, ChunkLifecycleState::Resident) {
                record.lifecycle = ChunkLifecycleState::Ready;
            }
        } else {
            record.lifecycle = ChunkLifecycleState::Dirty;
        }

        sdf_store
            .chunks
            .insert(output.chunk_id, output.chunk_payload);
        let region_id = partition.region_id_from_chunk_id(output.chunk_id);
        sdf_store
            .region_summaries
            .insert(region_id, output.region_summary);
        render_cache_invalidation.enqueue_integrated_chunk(&partition, output.chunk_id);
        integrated = integrated.saturating_add(1);
        authority.world_revision.0 = authority.world_revision.0.saturating_add(1);
        region_invalidation_journal.append_integration_record(
            &partition,
            output.chunk_id,
            authority.world_revision,
        );
    }

    runtime.integrated_build_outputs = runtime.integrated_build_outputs.saturating_add(integrated);
    runtime.dropped_stale_build_outputs =
        runtime.dropped_stale_build_outputs.saturating_add(dropped);
}

pub fn sync_world_runtime_debug_metrics_system(
    runtime: Res<WorldRuntimeState>,
    authority: Res<WorldAuthorityState>,
    mut debug: ResMut<WorldDebugMetricsResource>,
) {
    debug.integrated_build_outputs = runtime.integrated_build_outputs;
    debug.dropped_stale_build_outputs = runtime.dropped_stale_build_outputs;
    debug.last_world_revision = authority.world_revision.0;
}

fn payload_matches_target_contract(output: &WorldCompletedBuildOutput) -> bool {
    if output.chunk_payload.chunk_id != output.chunk_id {
        return false;
    }
    if output.chunk_payload.chunk_revision != output.target_chunk_revision {
        return false;
    }
    output.chunk_payload.chunk_generation == ChunkGeneration(output.target_build_generation.0)
}
