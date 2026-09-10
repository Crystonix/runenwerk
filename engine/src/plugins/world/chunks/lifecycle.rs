use super::DirtyChunkMapResource;
use crate::runtime::{ResMut, WorldMut};
use runen_spatial::ChunkId;
use std::collections::BTreeMap;
use world_ops::{BuildGeneration, ChunkGeneration, ChunkRevision, DirtyReasonSet};

#[derive(Debug, Copy, Clone, PartialEq, Eq, runen_ecs::Component, runen_ecs::Resource)]
pub enum ChunkLifecycleState {
    Unloaded,
    Loading,
    Ready,
    Dirty,
    Rebuilding,
    Resident,
}

#[derive(Debug, Clone, runen_ecs::Component, runen_ecs::Resource)]
pub struct WorldChunkRuntimeRecord {
    pub chunk_id: ChunkId,
    pub lifecycle: ChunkLifecycleState,
    pub chunk_revision: ChunkRevision,
    pub chunk_generation: ChunkGeneration,
    pub build_generation: BuildGeneration,
    pub dirty_reasons: DirtyReasonSet,
    pub pending_build_generation: Option<BuildGeneration>,
    pub gameplay_locked: bool,
}

impl WorldChunkRuntimeRecord {
    pub fn new(chunk_id: ChunkId) -> Self {
        Self {
            chunk_id,
            lifecycle: ChunkLifecycleState::Unloaded,
            chunk_revision: ChunkRevision::default(),
            chunk_generation: ChunkGeneration::default(),
            build_generation: BuildGeneration::default(),
            dirty_reasons: DirtyReasonSet::default(),
            pending_build_generation: None,
            gameplay_locked: false,
        }
    }
}

#[derive(Debug, Clone, Default, runen_ecs::Component, runen_ecs::Resource)]
pub struct WorldChunkRuntimeMapResource {
    pub by_chunk_id: BTreeMap<ChunkId, WorldChunkRuntimeRecord>,
}

impl WorldChunkRuntimeMapResource {
    pub fn ensure_chunk(&mut self, chunk_id: ChunkId) -> &mut WorldChunkRuntimeRecord {
        self.by_chunk_id
            .entry(chunk_id)
            .or_insert_with(|| WorldChunkRuntimeRecord::new(chunk_id))
    }
}

pub fn advance_chunk_lifecycle_system(
    mut chunks: ResMut<WorldChunkRuntimeMapResource>,
    mut dirty: ResMut<DirtyChunkMapResource>,
) {
    let dirty_ids = dirty.by_chunk.keys().copied().collect::<Vec<_>>();
    for chunk_id in dirty_ids {
        chunks.ensure_chunk(chunk_id);
    }

    let ids = chunks.by_chunk_id.keys().copied().collect::<Vec<_>>();
    for chunk_id in ids {
        let Some(record) = chunks.by_chunk_id.get_mut(&chunk_id) else {
            continue;
        };
        if let Some(reasons) = dirty.take_reasons(&chunk_id) {
            record.dirty_reasons.merge_from(reasons);
            if record.pending_build_generation.is_none()
                && !matches!(record.lifecycle, ChunkLifecycleState::Rebuilding)
            {
                record.lifecycle = ChunkLifecycleState::Dirty;
            }
        }
        if matches!(record.lifecycle, ChunkLifecycleState::Loading) {
            record.lifecycle = ChunkLifecycleState::Ready;
        }
        if matches!(record.lifecycle, ChunkLifecycleState::Resident)
            && !record.gameplay_locked
            && record.dirty_reasons.is_empty()
        {
            record.lifecycle = ChunkLifecycleState::Ready;
        }
    }
}

pub fn ensure_chunk_runtime_record(world: &mut WorldMut, chunk_id: ChunkId) {
    if let Ok(chunks) = world.resource_mut::<WorldChunkRuntimeMapResource>() {
        chunks.ensure_chunk(chunk_id);
    }
}
