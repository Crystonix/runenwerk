use crate::plugins::render::features::world::runtime_cache::WorldRuntimeCacheResource;
use crate::runtime::WorldMut;
use runen_spatial::{ChunkCoord3, ChunkId, GridPartitionConfig, RegionId, WorldId};
use std::collections::{BTreeSet, VecDeque};

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum WorldRenderInvalidationSource {
    EditIngress,
    BuildIntegration,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct WorldRenderChunkBounds {
    pub planet_id: WorldId,
    pub min: ChunkCoord3,
    pub max: ChunkCoord3,
}

impl WorldRenderChunkBounds {
    fn from_chunk_id(chunk_id: ChunkId) -> Self {
        Self {
            planet_id: chunk_id.world_id,
            min: chunk_id.coord,
            max: chunk_id.coord,
        }
    }

    fn from_chunk_ids(chunk_ids: &BTreeSet<ChunkId>) -> Option<Self> {
        let mut chunks = chunk_ids.iter().copied();
        let first = chunks.next()?;
        let mut min = first.coord;
        let mut max = first.coord;

        for chunk in chunks {
            min.x = min.x.min(chunk.coord.x);
            min.y = min.y.min(chunk.coord.y);
            min.z = min.z.min(chunk.coord.z);
            max.x = max.x.max(chunk.coord.x);
            max.y = max.y.max(chunk.coord.y);
            max.z = max.z.max(chunk.coord.z);
        }

        Some(Self {
            planet_id: first.world_id,
            min,
            max,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldRenderCacheInvalidationRecord {
    pub source: WorldRenderInvalidationSource,
    pub chunk_bounds: WorldRenderChunkBounds,
    pub chunk_ids: BTreeSet<ChunkId>,
    pub region_ids: BTreeSet<RegionId>,
}

#[derive(Debug, Clone, Default, runen_ecs::Component, runen_ecs::Resource)]
pub struct WorldRenderCacheInvalidationQueueResource {
    pub pending_records: VecDeque<WorldRenderCacheInvalidationRecord>,
}

impl WorldRenderCacheInvalidationQueueResource {
    pub fn enqueue_ingress_bounds(
        &mut self,
        partition: &GridPartitionConfig,
        touched_chunks: BTreeSet<ChunkId>,
    ) {
        let Some(chunk_bounds) = WorldRenderChunkBounds::from_chunk_ids(&touched_chunks) else {
            return;
        };
        let region_ids = touched_chunks
            .iter()
            .copied()
            .map(|chunk_id| partition.region_id_from_chunk_id(chunk_id))
            .collect();
        self.pending_records
            .push_back(WorldRenderCacheInvalidationRecord {
                source: WorldRenderInvalidationSource::EditIngress,
                chunk_bounds,
                chunk_ids: touched_chunks,
                region_ids,
            });
    }

    pub fn enqueue_integrated_chunk(&mut self, partition: &GridPartitionConfig, chunk_id: ChunkId) {
        let mut chunk_ids = BTreeSet::new();
        chunk_ids.insert(chunk_id);
        let mut region_ids = BTreeSet::new();
        region_ids.insert(partition.region_id_from_chunk_id(chunk_id));
        self.pending_records
            .push_back(WorldRenderCacheInvalidationRecord {
                source: WorldRenderInvalidationSource::BuildIntegration,
                chunk_bounds: WorldRenderChunkBounds::from_chunk_id(chunk_id),
                chunk_ids,
                region_ids,
            });
    }
}

pub fn flush_world_render_cache_invalidations_system(mut world: WorldMut) {
    let pending_records = match world.resource::<WorldRenderCacheInvalidationQueueResource>() {
        Ok(queue) => {
            if queue.pending_records.is_empty() {
                return;
            }
            queue.pending_records.iter().cloned().collect::<Vec<_>>()
        }
        Err(_) => return,
    };

    let Ok(runtime_cache) = world.resource_mut::<WorldRuntimeCacheResource>() else {
        // Keep pending invalidations queued until the render cache resource is available.
        return;
    };
    let mut deduped_chunks = BTreeSet::new();
    for record in pending_records {
        deduped_chunks.extend(record.chunk_ids);
    }
    for chunk_id in deduped_chunks {
        runtime_cache.mark_stale(chunk_id);
    }

    if let Ok(queue) = world.resource_mut::<WorldRenderCacheInvalidationQueueResource>() {
        queue.pending_records.clear();
    }
}
