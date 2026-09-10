#[derive(Debug, Clone, Copy, Default)]
pub struct PipelineCacheStats {
    pub hits: u64,
    pub misses: u64,
    pub failures: u64,
}

#[derive(Debug, Clone, Default, runen_ecs::Component, runen_ecs::Resource)]
pub struct PipelineCacheResource {
    stats: PipelineCacheStats,
}

impl PipelineCacheResource {
    pub fn stats(&self) -> PipelineCacheStats {
        self.stats
    }

    pub fn observe_stats(&mut self, stats: PipelineCacheStats) {
        self.stats = stats;
    }
}

#[cfg(test)]
mod tests {
    use super::{PipelineCacheResource, PipelineCacheStats};

    #[test]
    fn pipeline_cache_resource_tracks_only_canonical_stats() {
        let mut resource = PipelineCacheResource::default();
        resource.observe_stats(PipelineCacheStats {
            hits: 3,
            misses: 1,
            failures: 0,
        });
        let stats = resource.stats();
        assert_eq!(stats.hits, 3);
        assert_eq!(stats.misses, 1);
        assert_eq!(stats.failures, 0);
    }
}
