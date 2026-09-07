use super::render_flow::RendererProgramSourceAuthority;
use super::{DEFAULT_COMPUTE_SHADER, DEFAULT_FULLSCREEN_SHADER, DEFAULT_GRAPHICS_SHADER};
use crate::plugins::render::RenderFlowId;
use crate::plugins::render::pipelines::FlowPassPipelineKey;
use runen_gpu::{
    GpuAdmittedProgramSource, GpuContext, GpuProgramSourceError, GpuProgramSourceKey,
    GpuProgramSourceProvenance, GpuRealizedSampler, GpuSamplerDescriptor, GpuSamplerHandle,
    GpuWorkResourceIdAllocator,
};
use std::collections::HashMap;

const RENDERER_PROGRAM_SOURCE_MAX_RECORDS: usize = 1024;
const RENDERER_PROGRAM_SOURCE_MAX_RETAINED_BYTES: usize = 64 * 1024 * 1024;

#[derive(Debug, Clone, Copy, Default)]
pub struct RendererPipelineCacheStats {
    pub hits: u64,
    pub misses: u64,
    pub failures: u64,
    pub program_source_owner: u64,
    pub program_source_records: usize,
    pub program_source_bytes: usize,
    pub program_source_max_records: usize,
    pub program_source_max_bytes: usize,
}

#[derive(Debug)]
pub struct FlowPipelineArtifactCache {
    samplers: HashMap<FlowPassPipelineKey, RendererRealizedSampler>,
    pub stats: RendererPipelineCacheStats,
    program_sources: RendererProgramSourceAuthority,
    resource_ids: GpuWorkResourceIdAllocator,
}

#[derive(Debug, Clone)]
pub(super) struct RendererRealizedSampler {
    handle: GpuSamplerHandle,
    _realized: GpuRealizedSampler,
}

impl RendererRealizedSampler {
    pub(super) fn handle(&self) -> &GpuSamplerHandle {
        &self.handle
    }
}

impl Default for FlowPipelineArtifactCache {
    fn default() -> Self {
        let program_sources = RendererProgramSourceAuthority::new(
            RENDERER_PROGRAM_SOURCE_MAX_RECORDS,
            RENDERER_PROGRAM_SOURCE_MAX_RETAINED_BYTES,
        )
        .expect("renderer program-source authority policy is nonzero and process-local");
        let mut cache = Self {
            samplers: HashMap::new(),
            stats: RendererPipelineCacheStats::default(),
            program_sources,
            resource_ids: GpuWorkResourceIdAllocator::new(),
        };
        admit_builtin_program_source(&mut cache, "builtin:compute", DEFAULT_COMPUTE_SHADER);
        admit_builtin_program_source(&mut cache, "builtin:fullscreen", DEFAULT_FULLSCREEN_SHADER);
        admit_builtin_program_source(&mut cache, "builtin:graphics", DEFAULT_GRAPHICS_SHADER);
        cache
    }
}

impl FlowPipelineArtifactCache {
    pub fn stats(&self) -> RendererPipelineCacheStats {
        let source_stats = self.program_sources.stats();
        RendererPipelineCacheStats {
            program_source_owner: self.program_sources.owner().diagnostic_raw(),
            program_source_records: source_stats.retained_records(),
            program_source_bytes: source_stats.retained_source_bytes(),
            program_source_max_records: source_stats.max_records(),
            program_source_max_bytes: source_stats.max_retained_source_bytes(),
            ..self.stats
        }
    }

    pub(crate) fn admit_program_source(
        &mut self,
        key: GpuProgramSourceKey,
        renderer_revision: u64,
        canonical_wgsl: impl Into<String>,
        provenance: GpuProgramSourceProvenance,
    ) -> Result<GpuAdmittedProgramSource, GpuProgramSourceError> {
        self.program_sources
            .admit_wgsl(key, renderer_revision, canonical_wgsl, provenance)
    }

    pub fn get_or_realize_sampler(
        &mut self,
        context: &GpuContext,
        key: FlowPassPipelineKey,
        descriptor: GpuSamplerDescriptor,
    ) -> anyhow::Result<RendererRealizedSampler> {
        if let Some(value) = self.samplers.get(&key) {
            if value._realized.descriptor() != &descriptor {
                anyhow::bail!(
                    "renderer sampler cache key reused with changed normalized descriptor"
                );
            }
            self.stats.hits = self.stats.hits.saturating_add(1);
            return Ok(value.clone());
        }
        self.stats.misses = self.stats.misses.saturating_add(1);
        let handle = self.resource_ids.allocate_sampler_handle(descriptor)?;
        let realized = context.realize_sampler(&handle)?;
        self.samplers.insert(
            key.clone(),
            RendererRealizedSampler {
                handle,
                _realized: realized,
            },
        );
        Ok(self
            .samplers
            .get(&key)
            .expect("inserted renderer sampler must be immediately available")
            .clone())
    }

    pub fn retain_flows(&mut self, active_flow_ids: &[RenderFlowId]) {
        self.samplers
            .retain(|key, _| active_flow_ids.contains(&key.flow_id));
        self.program_sources.collect_unretained();
    }
}

fn admit_builtin_program_source(cache: &mut FlowPipelineArtifactCache, key: &str, source: &str) {
    cache
        .admit_program_source(
            GpuProgramSourceKey::new(key).expect("builtin source key is static and valid"),
            0,
            source,
            GpuProgramSourceProvenance::new("renderer-builtin-fallback", Some(key.to_owned()))
                .expect("builtin source provenance is static and valid"),
        )
        .expect("builtin canonical WGSL source must admit before renderer realization");
}

#[cfg(test)]
mod tests {
    use super::*;
    use runen_gpu::{GpuBindingLayoutRefinement, GpuEntryPointName, GpuProgramDescriptor};

    fn compute_program(source: GpuAdmittedProgramSource) -> GpuProgramDescriptor {
        let entry_point = GpuEntryPointName::new("cs_main")
            .expect("test compute entry-point name should be valid");
        GpuProgramDescriptor::new(
            source,
            [entry_point],
            std::iter::empty::<GpuBindingLayoutRefinement>(),
        )
        .expect("test program descriptor should retain its admitted source")
    }

    #[test]
    fn renderer_cache_admits_builtin_program_sources_as_reclaimable_lookup_records() {
        let mut cache = FlowPipelineArtifactCache::default();
        let stats = cache.stats();

        assert_ne!(stats.program_source_owner, 0);
        assert_eq!(stats.program_source_records, 3);
        assert_eq!(
            stats.program_source_bytes,
            DEFAULT_COMPUTE_SHADER.len()
                + DEFAULT_FULLSCREEN_SHADER.len()
                + DEFAULT_GRAPHICS_SHADER.len()
        );
        assert_eq!(
            stats.program_source_max_records,
            RENDERER_PROGRAM_SOURCE_MAX_RECORDS
        );
        assert_eq!(
            stats.program_source_max_bytes,
            RENDERER_PROGRAM_SOURCE_MAX_RETAINED_BYTES
        );

        cache.retain_flows(&[]);
        assert_eq!(cache.stats().program_source_records, 0);
    }

    #[test]
    fn cache_source_admission_is_idempotent_and_conflict_checked() {
        let mut cache = FlowPipelineArtifactCache::default();
        let key = || {
            GpuProgramSourceKey::new("asset:test-resolved-program")
                .expect("test source key should be valid")
        };
        let provenance = || {
            GpuProgramSourceProvenance::new(
                "renderer-resolved-program-test",
                Some("asset-backed source".to_owned()),
            )
            .expect("test provenance should be valid")
        };
        let source = "@compute @workgroup_size(1) fn cs_main() {}";
        let first = cache
            .admit_program_source(key(), 4, source, provenance())
            .expect("resolved source should admit");
        let repeated = cache
            .admit_program_source(key(), 4, source, provenance())
            .expect("identical source should remain idempotent");

        assert_eq!(
            first.identity().owner().diagnostic_raw(),
            cache.stats().program_source_owner
        );
        assert_eq!(first.identity().revision().get(), 5);
        assert!(first.is_same_record(&repeated));
        assert_eq!(cache.stats().program_source_records, 4);

        let error = cache
            .admit_program_source(
                key(),
                4,
                "@compute @workgroup_size(8) fn cs_main() {}",
                provenance(),
            )
            .expect_err("different source text must allocate a new renderer revision");
        assert_eq!(
            error.cause(),
            runen_gpu::GpuProgramSourceCause::SourceRevisionConflict
        );
        assert_eq!(cache.stats().program_source_records, 4);
    }

    #[test]
    fn independent_renderer_caches_do_not_share_source_owner_identity() {
        let first = FlowPipelineArtifactCache::default().stats();
        let second = FlowPipelineArtifactCache::default().stats();
        assert_ne!(first.program_source_owner, second.program_source_owner);
    }

    #[test]
    fn flow_retirement_respects_then_reclaims_descriptor_held_source_lifetime() {
        let mut cache = FlowPipelineArtifactCache::default();
        let source = cache
            .admit_program_source(
                GpuProgramSourceKey::new("asset:descriptor-held-across-flow-retirement")
                    .expect("test source key should be valid"),
                1,
                "@compute @workgroup_size(1) fn cs_main() {}",
                GpuProgramSourceProvenance::new(
                    "renderer-resolved-program-test",
                    Some("flow retirement".to_owned()),
                )
                .expect("test provenance should be valid"),
            )
            .expect("resolved source should admit");
        let program = compute_program(source);
        cache.retain_flows(&[]);

        assert_eq!(cache.stats().program_source_records, 1);
        drop(program);
        cache.retain_flows(&[]);
        assert_eq!(cache.stats().program_source_records, 0);
    }
}
