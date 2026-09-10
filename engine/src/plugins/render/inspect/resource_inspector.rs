use crate::plugins::render::{
    RenderFlow, RenderImportedBufferSemantic, RenderImportedTextureSemantic,
    RenderResourceDeclaration, RenderTargetAliasKind,
};
use runen_gpu::GpuResourceLifetime;

#[derive(Debug, Clone, Default, runen_ecs::Component, runen_ecs::Resource)]
pub struct RenderDebugOverlayState {
    pub enabled: bool,
    pub lines: Vec<String>,
}

#[derive(Debug, Clone, Default, runen_ecs::Component, runen_ecs::Resource)]
pub struct RenderRuntimeResourceInspectorState {
    pub entries: Vec<RuntimeResourceInspectionEntry>,
    pub pipeline_cache_hits: u64,
    pub pipeline_cache_misses: u64,
    pub world_resident_chunks: usize,
    pub world_stale_chunks: usize,
    pub world_page_miss_count: u64,
    pub world_interactive_queue_depth: usize,
    pub world_background_queue_depth: usize,
}

impl RenderRuntimeResourceInspectorState {
    pub fn observe_runtime_resources(&mut self, entries: &[RuntimeResourceInspectionEntry]) {
        self.entries.clear();
        self.entries.extend_from_slice(entries);
    }

    pub fn observe_pipeline_cache_stats(&mut self, hits: u64, misses: u64) {
        self.pipeline_cache_hits = hits;
        self.pipeline_cache_misses = misses;
    }

    pub fn observe_world_runtime(
        &mut self,
        resident_chunks: usize,
        stale_chunks: usize,
        page_miss_count: u64,
        interactive_queue_depth: usize,
        background_queue_depth: usize,
    ) {
        self.world_resident_chunks = resident_chunks;
        self.world_stale_chunks = stale_chunks;
        self.world_page_miss_count = page_miss_count;
        self.world_interactive_queue_depth = interactive_queue_depth;
        self.world_background_queue_depth = background_queue_depth;
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceInspectionEntry {
    pub id: String,
    pub kind: String,
    pub lifetime: GpuResourceLifetime,
    pub imported: bool,
    pub target_alias_binding_key: Option<String>,
    pub target_alias_kind: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeResourceReuse {
    Created,
    Reused,
    NotRealized,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeResourceInspectionEntry {
    pub flow_id: String,
    pub id: String,
    pub kind: String,
    pub lifetime: GpuResourceLifetime,
    pub imported: bool,
    pub realized: bool,
    pub reuse: RuntimeResourceReuse,
    pub size_bytes: Option<u64>,
    pub texture_size: Option<(u32, u32)>,
    pub texture_format: Option<String>,
    pub element_count: Option<u64>,
    pub generation: Option<u64>,
}

pub fn resource_kind_name(resource: &RenderResourceDeclaration) -> &'static str {
    match resource {
        RenderResourceDeclaration::Uniform(_) => "uniform_buffer",
        RenderResourceDeclaration::Storage(_) => "storage_buffer",
        RenderResourceDeclaration::Sampled(_) => "sampled_texture",
        RenderResourceDeclaration::StorageImage(_) => "storage_texture",
        RenderResourceDeclaration::ColorAttachment(_) => "color_target",
        RenderResourceDeclaration::DepthAttachment(_) => "depth_target",
        RenderResourceDeclaration::History(_) => "history_texture",
        RenderResourceDeclaration::TargetAlias(value) => {
            target_alias_kind_resource_name(value.kind())
        }
        RenderResourceDeclaration::ImportedTexture(value) => match value.semantic {
            RenderImportedTextureSemantic::SurfaceColor => "imported_texture(surface_color)",
            RenderImportedTextureSemantic::SurfaceDepth => "imported_texture(surface_depth)",
            RenderImportedTextureSemantic::HistoryTexture => "imported_texture(history_texture)",
            RenderImportedTextureSemantic::External => "imported_texture(external)",
        },
        RenderResourceDeclaration::ImportedBuffer(value) => match value.semantic {
            RenderImportedBufferSemantic::HistoryBuffer => "imported_buffer(history_buffer)",
            RenderImportedBufferSemantic::External => "imported_buffer(external)",
        },
    }
}

pub fn target_alias_kind_name(kind: RenderTargetAliasKind) -> &'static str {
    match kind {
        RenderTargetAliasKind::Color => "color",
        RenderTargetAliasKind::Depth => "depth",
        RenderTargetAliasKind::Texture => "texture",
    }
}

fn target_alias_kind_resource_name(kind: RenderTargetAliasKind) -> &'static str {
    match kind {
        RenderTargetAliasKind::Color => "target_alias(color)",
        RenderTargetAliasKind::Depth => "target_alias(depth)",
        RenderTargetAliasKind::Texture => "target_alias(texture)",
    }
}

pub fn inspect_resources(flow: &RenderFlow) -> Vec<ResourceInspectionEntry> {
    flow.graph()
        .resources
        .resources
        .iter()
        .map(|resource| {
            let lifetime = resource.lifetime();
            ResourceInspectionEntry {
                id: resource.id().to_string(),
                kind: resource_kind_name(resource).to_string(),
                lifetime,
                imported: resource.is_imported(),
                target_alias_binding_key: target_alias_binding_key(resource),
                target_alias_kind: target_alias_kind(resource),
            }
        })
        .collect()
}

fn target_alias_binding_key(resource: &RenderResourceDeclaration) -> Option<String> {
    match resource {
        RenderResourceDeclaration::TargetAlias(value) => {
            Some(value.binding_key().as_str().to_string())
        }
        _ => None,
    }
}

fn target_alias_kind(resource: &RenderResourceDeclaration) -> Option<String> {
    match resource {
        RenderResourceDeclaration::TargetAlias(value) => {
            Some(target_alias_kind_name(value.kind()).to_string())
        }
        _ => None,
    }
}
