use crate::plugins::render::RenderGpuCacheHandle;
use runen_spatial::ChunkId;
use std::collections::{BTreeMap, BTreeSet};
use world_ops::{ChunkGeneration, ChunkRevision};

#[derive(Debug, Clone, PartialEq, Eq, runen_ecs::Resource)]
pub struct WorldGpuResidencyEntry {
    pub chunk_id: ChunkId,
    pub chunk_revision: ChunkRevision,
    pub cache_generation: ChunkGeneration,
    pub cache_handle: RenderGpuCacheHandle,
    pub pinned: bool,
    pub priority: i32,
}

#[derive(Debug, Clone, Default, runen_ecs::Component, runen_ecs::Resource)]
pub struct WorldRuntimeCacheResource {
    pub by_chunk: BTreeMap<ChunkId, WorldGpuResidencyEntry>,
    pub stale_chunks: BTreeSet<ChunkId>,
    pub brick_pool_budget_bytes: u64,
    pub clipmap_budget_bytes: u64,
    pub detail_budget_bytes: u64,
    pub history_budget_bytes: u64,
}

impl WorldRuntimeCacheResource {
    pub fn mark_stale(&mut self, chunk_id: ChunkId) {
        self.stale_chunks.insert(chunk_id);
        self.by_chunk.remove(&chunk_id);
    }

    pub fn upsert_entry(&mut self, entry: WorldGpuResidencyEntry) {
        self.stale_chunks.remove(&entry.chunk_id);
        self.by_chunk.insert(entry.chunk_id, entry);
    }
}
