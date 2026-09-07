use crate::plugins::render::RenderImportedTextureSemantic;
use crate::plugins::render::inspect::{CaptureStage, RenderCapturedTexture};
use crate::plugins::render::{RenderFlow, RenderResourceDeclaration, RenderTargetAliasKind};
use runen_gpu::GpuResourceLifetime;

#[derive(Debug, Clone, Default, ecs::Component, ecs::Resource)]
pub struct RenderTextureInspectorState {
    pub selected_texture: Option<String>,
    pub hovered_texture: Option<String>,
    pub captured: Vec<CapturedTextureViewEntry>,
}

impl RenderTextureInspectorState {
    pub fn observe_captures(&mut self, captures: &[RenderCapturedTexture]) {
        self.captured.clear();
        self.captured.extend(captures.iter().map(|capture| {
            let identity = &capture.identity;
            CapturedTextureViewEntry {
                frame_index: identity.frame_index,
                flow_id: identity.flow_id().to_string(),
                pass_id: identity.pass_id().to_string(),
                pass_label: identity.pass_label.clone(),
                stage: identity.stage(),
                resource_id: identity.resource_id().to_string(),
                texture_class: identity.texture_class().as_str().to_string(),
                width: capture.width,
                height: capture.height,
                format: capture.format.clone(),
                has_pixels: capture.bytes_rgba8.is_some(),
                terminal_code: capture.terminal.code.as_str().to_string(),
                terminal_reason: capture
                    .terminal
                    .reason
                    .as_ref()
                    .map(|value| value.detail.clone()),
            }
        }));
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapturedTextureViewEntry {
    pub frame_index: u64,
    pub flow_id: String,
    pub pass_id: String,
    pub pass_label: String,
    pub stage: CaptureStage,
    pub resource_id: String,
    pub texture_class: String,
    pub width: u32,
    pub height: u32,
    pub format: String,
    pub has_pixels: bool,
    pub terminal_code: String,
    pub terminal_reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextureResourceView {
    pub id: String,
    pub category: String,
    pub lifetime: GpuResourceLifetime,
    pub target_alias_binding_key: Option<String>,
    pub target_alias_kind: Option<String>,
}

pub fn inspect_texture_resources(flow: &RenderFlow) -> Vec<TextureResourceView> {
    flow.graph()
        .resources
        .resources
        .iter()
        .filter_map(|resource| {
            let category = match resource {
                RenderResourceDeclaration::Sampled(_) => Some("sampled_texture"),
                RenderResourceDeclaration::StorageImage(_) => Some("storage_texture"),
                RenderResourceDeclaration::ColorAttachment(_) => Some("color_target"),
                RenderResourceDeclaration::DepthAttachment(_) => Some("depth_target"),
                RenderResourceDeclaration::History(_) => Some("history_texture"),
                RenderResourceDeclaration::TargetAlias(value) => {
                    Some(target_alias_texture_category(value.kind()))
                }
                RenderResourceDeclaration::ImportedTexture(value) => Some(match value.semantic {
                    RenderImportedTextureSemantic::SurfaceColor => {
                        "imported_texture(surface_color)"
                    }
                    RenderImportedTextureSemantic::SurfaceDepth => {
                        "imported_texture(surface_depth)"
                    }
                    RenderImportedTextureSemantic::HistoryTexture => {
                        "imported_texture(history_texture)"
                    }
                    RenderImportedTextureSemantic::External => "imported_texture(external)",
                }),
                _ => None,
            }?;

            Some(TextureResourceView {
                id: resource.id().to_string(),
                category: category.to_string(),
                lifetime: resource.lifetime(),
                target_alias_binding_key: target_alias_binding_key(resource),
                target_alias_kind: target_alias_kind(resource),
            })
        })
        .collect()
}

fn target_alias_texture_category(kind: RenderTargetAliasKind) -> &'static str {
    match kind {
        RenderTargetAliasKind::Color => "target_alias(color)",
        RenderTargetAliasKind::Depth => "target_alias(depth)",
        RenderTargetAliasKind::Texture => "target_alias(texture)",
    }
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
        RenderResourceDeclaration::TargetAlias(value) => Some(
            match value.kind() {
                RenderTargetAliasKind::Color => "color",
                RenderTargetAliasKind::Depth => "depth",
                RenderTargetAliasKind::Texture => "texture",
            }
            .to_string(),
        ),
        _ => None,
    }
}
