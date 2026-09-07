//! Fixed legacy-renderer capability lowering. These facts preserve the current
//! Runenwerk contract; they do not query a backend. G5 deletes this adapter when
//! generic execution consumes admitted RunenGPU facts directly.

use crate::plugins::render::graph::{
    CompiledBindingEntry, CompiledDrawSource, CompiledPassExecutionPlan, CompiledRenderFlowPlan,
    RenderExecutionGraphDiagnostic, RenderExecutionGraphDiagnosticKind,
};
use crate::plugins::render::{
    RenderResourceDeclaration, RenderTextureFormatPolicy, RenderTextureTargetFormat,
};
use runen_gpu::{
    GpuCapabilities, GpuCapabilityFeature, GpuCapabilityRequirement, GpuLimits,
    GpuPreparedWorkGraph, GpuTextureFormat, GpuTextureFormatCapabilities,
};

/// Returns normalized facts for the fixed capability contract the current
/// renderer already assumes. This is not backend-admission evidence.
pub fn current_runtime_gpu_capabilities() -> GpuCapabilities {
    let features = [
        GpuCapabilityFeature::Compute,
        GpuCapabilityFeature::RenderPipeline,
        GpuCapabilityFeature::Copy,
        GpuCapabilityFeature::IndirectExecution,
        GpuCapabilityFeature::StorageTexture,
        GpuCapabilityFeature::DepthAttachment,
        GpuCapabilityFeature::Presentation,
    ]
    .into_iter();
    let limits = GpuLimits::new(
        64 * 1024,
        128 * 1024 * 1024,
        1,
        8,
        16,
        8192,
        4,
        24,
        8,
        4,
        65_535,
    )
    .expect("fixed renderer capability limits are internally consistent");
    GpuCapabilities::from_normalized_facts(features, limits, current_format_facts())
}

fn current_format_facts() -> impl Iterator<Item = (GpuTextureFormat, GpuTextureFormatCapabilities)>
{
    [
        (
            GpuTextureFormat::R8Unorm,
            GpuTextureFormatCapabilities {
                sampled: true,
                filterable: true,
                storage_read: false,
                storage_write: false,
                color_attachment: true,
                depth_stencil: false,
                copy_source: true,
                copy_destination: true,
                block_dimensions: Some((1, 1)),
                block_copy_size: Some(1),
            },
        ),
        (
            GpuTextureFormat::Rgba8Unorm,
            GpuTextureFormatCapabilities {
                sampled: true,
                filterable: true,
                storage_read: false,
                storage_write: true,
                color_attachment: true,
                depth_stencil: false,
                copy_source: true,
                copy_destination: true,
                block_dimensions: Some((1, 1)),
                block_copy_size: Some(4),
            },
        ),
        (
            GpuTextureFormat::Rgba8UnormSrgb,
            GpuTextureFormatCapabilities {
                sampled: true,
                filterable: true,
                storage_read: false,
                storage_write: false,
                color_attachment: true,
                depth_stencil: false,
                copy_source: true,
                copy_destination: true,
                block_dimensions: Some((1, 1)),
                block_copy_size: Some(4),
            },
        ),
        (
            GpuTextureFormat::Bgra8Unorm,
            GpuTextureFormatCapabilities {
                sampled: true,
                filterable: true,
                storage_read: false,
                storage_write: false,
                color_attachment: true,
                depth_stencil: false,
                copy_source: true,
                copy_destination: true,
                block_dimensions: Some((1, 1)),
                block_copy_size: Some(4),
            },
        ),
        (
            GpuTextureFormat::Bgra8UnormSrgb,
            GpuTextureFormatCapabilities {
                sampled: true,
                filterable: true,
                storage_read: false,
                storage_write: false,
                color_attachment: true,
                depth_stencil: false,
                copy_source: true,
                copy_destination: true,
                block_dimensions: Some((1, 1)),
                block_copy_size: Some(4),
            },
        ),
        (
            GpuTextureFormat::R32Uint,
            GpuTextureFormatCapabilities {
                sampled: true,
                filterable: false,
                storage_read: false,
                storage_write: true,
                color_attachment: true,
                depth_stencil: false,
                copy_source: true,
                copy_destination: true,
                block_dimensions: Some((1, 1)),
                block_copy_size: Some(4),
            },
        ),
        (
            GpuTextureFormat::Depth32Float,
            GpuTextureFormatCapabilities {
                sampled: true,
                filterable: false,
                storage_read: false,
                storage_write: false,
                color_attachment: false,
                depth_stencil: true,
                copy_source: true,
                copy_destination: true,
                block_dimensions: Some((1, 1)),
                block_copy_size: Some(4),
            },
        ),
    ]
    .into_iter()
}

pub fn validate_compiled_flow_capabilities(
    flow: &CompiledRenderFlowPlan,
    capabilities: &GpuCapabilities,
) -> Vec<RenderExecutionGraphDiagnostic> {
    let mut diagnostics = Vec::<RenderExecutionGraphDiagnostic>::new();

    for pass in &flow.execution.passes {
        validate_execution_pass_capabilities(flow, pass, capabilities, &mut diagnostics);
    }

    for descriptor in &flow.resources.resources {
        validate_resource_descriptor_capabilities(flow, descriptor, capabilities, &mut diagnostics);
    }

    diagnostics
}

pub fn validate_prepared_gpu_work_capabilities(
    flow: &CompiledRenderFlowPlan,
    graph: &GpuPreparedWorkGraph,
    capabilities: &GpuCapabilities,
) -> Vec<RenderExecutionGraphDiagnostic> {
    graph
        .requirements()
        .iter()
        .filter_map(|requirement| match requirement {
            GpuCapabilityRequirement::Required(feature) if !capabilities.supports(feature) => Some(
                RenderExecutionGraphDiagnostic::error(
                    RenderExecutionGraphDiagnosticKind::BackendCapabilityMismatch,
                    format!(
                        "prepared GPU work for flow '{}' requires unavailable capability {feature:?}",
                        flow.flow_label
                    ),
                )
                .with_flow(flow.flow_id, flow.flow_label.clone())
                .with_capability(format!("feature::{feature:?}")),
            ),
            GpuCapabilityRequirement::Required(_)
            | GpuCapabilityRequirement::Preferred { .. }
            | GpuCapabilityRequirement::Disabled(_) => None,
        })
        .collect()
}

fn validate_execution_pass_capabilities(
    flow: &CompiledRenderFlowPlan,
    pass: &CompiledPassExecutionPlan,
    capabilities: &GpuCapabilities,
    diagnostics: &mut Vec<RenderExecutionGraphDiagnostic>,
) {
    match pass {
        CompiledPassExecutionPlan::Compute(value) => {
            validate_required_feature(
                flow,
                value.pass_id,
                GpuCapabilityFeature::Compute,
                capabilities,
                diagnostics,
            );
            validate_binding_features(
                flow,
                value.pass_id,
                &value.bindings.bind_group.entries,
                capabilities,
                diagnostics,
            );
            validate_bind_group_limit(
                flow,
                value.pass_id,
                value.bindings.bind_group.entries.len(),
                capabilities,
                diagnostics,
            );
        }
        CompiledPassExecutionPlan::Fullscreen(value)
        | CompiledPassExecutionPlan::Graphics(value) => {
            validate_required_feature(
                flow,
                value.pass_id,
                GpuCapabilityFeature::RenderPipeline,
                capabilities,
                diagnostics,
            );
            validate_binding_features(
                flow,
                value.pass_id,
                &value.bindings.bind_group.entries,
                capabilities,
                diagnostics,
            );
            if value.targets.depth_output.is_some() {
                validate_required_feature(
                    flow,
                    value.pass_id,
                    GpuCapabilityFeature::DepthAttachment,
                    capabilities,
                    diagnostics,
                );
            }
            if value
                .draw
                .as_ref()
                .is_some_and(|draw| matches!(draw.source, CompiledDrawSource::Indirect { .. }))
            {
                validate_required_feature(
                    flow,
                    value.pass_id,
                    GpuCapabilityFeature::IndirectExecution,
                    capabilities,
                    diagnostics,
                );
            }
            validate_bind_group_limit(
                flow,
                value.pass_id,
                value.bindings.bind_group.entries.len(),
                capabilities,
                diagnostics,
            );
            let limits = capabilities.limits();
            if count_exceeds_limit(
                value.targets.color_outputs.len(),
                limits.max_color_attachments(),
            ) {
                diagnostics.push(
                    RenderExecutionGraphDiagnostic::error(
                        RenderExecutionGraphDiagnosticKind::BackendCapabilityMismatch,
                        format!(
                            "pass '{}' declares {} color attachments but normalized GPU capabilities support {}",
                            value.pass_id,
                            value.targets.color_outputs.len(),
                            limits.max_color_attachments()
                        ),
                    )
                    .with_flow(flow.flow_id, flow.flow_label.clone())
                    .with_pass(value.pass_id, value.pass_id.to_string())
                    .with_capability("max_color_attachments"),
                );
            }
            if count_exceeds_limit(
                value.draw_buffers.vertex_buffers.len(),
                limits.max_vertex_buffers(),
            ) {
                diagnostics.push(
                    RenderExecutionGraphDiagnostic::error(
                        RenderExecutionGraphDiagnosticKind::BackendCapabilityMismatch,
                        format!(
                            "pass '{}' declares {} vertex buffers but normalized GPU capabilities support {}",
                            value.pass_id,
                            value.draw_buffers.vertex_buffers.len(),
                            limits.max_vertex_buffers()
                        ),
                    )
                    .with_flow(flow.flow_id, flow.flow_label.clone())
                    .with_pass(value.pass_id, value.pass_id.to_string())
                    .with_capability("max_vertex_buffers"),
                );
            }
        }
        CompiledPassExecutionPlan::Copy(value) => validate_required_feature(
            flow,
            value.pass_id,
            GpuCapabilityFeature::Copy,
            capabilities,
            diagnostics,
        ),
        CompiledPassExecutionPlan::Present(value) => validate_required_feature(
            flow,
            value.pass_id,
            GpuCapabilityFeature::Presentation,
            capabilities,
            diagnostics,
        ),
        CompiledPassExecutionPlan::BuiltinUiComposite(value) => validate_required_feature(
            flow,
            value.pass_id,
            GpuCapabilityFeature::RenderPipeline,
            capabilities,
            diagnostics,
        ),
    }
}

fn validate_binding_features(
    flow: &CompiledRenderFlowPlan,
    pass_id: crate::plugins::render::RenderPassId,
    entries: &[CompiledBindingEntry],
    capabilities: &GpuCapabilities,
    diagnostics: &mut Vec<RenderExecutionGraphDiagnostic>,
) {
    if entries
        .iter()
        .any(|entry| matches!(entry, CompiledBindingEntry::StorageTexture { .. }))
    {
        validate_required_feature(
            flow,
            pass_id,
            GpuCapabilityFeature::StorageTexture,
            capabilities,
            diagnostics,
        );
    }
}

fn validate_required_feature(
    flow: &CompiledRenderFlowPlan,
    pass_id: crate::plugins::render::RenderPassId,
    feature: GpuCapabilityFeature,
    capabilities: &GpuCapabilities,
    diagnostics: &mut Vec<RenderExecutionGraphDiagnostic>,
) {
    if capabilities.supports(feature) {
        return;
    }
    diagnostics.push(
        RenderExecutionGraphDiagnostic::error(
            RenderExecutionGraphDiagnosticKind::BackendCapabilityMismatch,
            format!(
                "pass '{}' requires unavailable normalized GPU capability {feature:?}",
                pass_id
            ),
        )
        .with_flow(flow.flow_id, flow.flow_label.clone())
        .with_pass(pass_id, pass_id.to_string())
        .with_capability(format!("feature::{feature:?}")),
    );
}

fn validate_bind_group_limit(
    flow: &CompiledRenderFlowPlan,
    pass_id: crate::plugins::render::RenderPassId,
    entries: usize,
    capabilities: &GpuCapabilities,
    diagnostics: &mut Vec<RenderExecutionGraphDiagnostic>,
) {
    let supported = capabilities.limits().max_bindings_per_group();
    if !count_exceeds_limit(entries, supported) {
        return;
    }
    diagnostics.push(
        RenderExecutionGraphDiagnostic::error(
            RenderExecutionGraphDiagnosticKind::BackendCapabilityMismatch,
            format!(
                "pass '{}' declares {} bind group entries but normalized GPU capabilities support {}",
                pass_id, entries, supported
            ),
        )
        .with_flow(flow.flow_id, flow.flow_label.clone())
        .with_pass(pass_id, pass_id.to_string())
        .with_capability("max_bindings_per_group"),
    );
}

fn count_exceeds_limit(count: usize, limit: u32) -> bool {
    match u64::try_from(count) {
        Ok(count) => count > u64::from(limit),
        Err(_) => true,
    }
}

fn validate_resource_descriptor_capabilities(
    flow: &CompiledRenderFlowPlan,
    descriptor: &RenderResourceDeclaration,
    capabilities: &GpuCapabilities,
    diagnostics: &mut Vec<RenderExecutionGraphDiagnostic>,
) {
    let limits = capabilities.limits();
    match descriptor {
        RenderResourceDeclaration::Uniform(value)
            if value.size_bytes() > limits.max_uniform_buffer_binding_size() =>
        {
            diagnostics.push(resource_capability_diagnostic(
                flow,
                *descriptor.id(),
                "max_uniform_buffer_binding_size",
                format!(
                    "uniform buffer '{}' requires {} bytes but normalized GPU capabilities support {}",
                    value.id(),
                    value.size_bytes(),
                    limits.max_uniform_buffer_binding_size()
                ),
            ));
        }
        RenderResourceDeclaration::Storage(value)
            if value.size_bytes() > limits.max_storage_buffer_binding_size() =>
        {
            diagnostics.push(resource_capability_diagnostic(
                flow,
                *descriptor.id(),
                "max_storage_buffer_binding_size",
                format!(
                    "storage buffer '{}' requires {} bytes but normalized GPU capabilities support {}",
                    value.id(),
                    value.size_bytes(),
                    limits.max_storage_buffer_binding_size()
                ),
            ));
        }
        RenderResourceDeclaration::StorageImage(value) => {
            validate_texture_format_capability(
                flow,
                *descriptor.id(),
                value.texture.format,
                capabilities,
                TextureCapability::Storage,
                diagnostics,
            );
        }
        RenderResourceDeclaration::ColorAttachment(value) => validate_texture_format_capability(
            flow,
            *descriptor.id(),
            value.texture.format,
            capabilities,
            TextureCapability::ColorAttachment,
            diagnostics,
        ),
        RenderResourceDeclaration::DepthAttachment(value) => {
            validate_texture_format_capability(
                flow,
                *descriptor.id(),
                value.texture.format,
                capabilities,
                TextureCapability::DepthAttachment,
                diagnostics,
            );
        }
        RenderResourceDeclaration::Sampled(value) => validate_texture_format_capability(
            flow,
            *descriptor.id(),
            value.texture.format,
            capabilities,
            TextureCapability::Sampled,
            diagnostics,
        ),
        RenderResourceDeclaration::History(value) => validate_texture_format_capability(
            flow,
            *descriptor.id(),
            value.texture.format,
            capabilities,
            TextureCapability::Sampled,
            diagnostics,
        ),
        RenderResourceDeclaration::Uniform(_)
        | RenderResourceDeclaration::Storage(_)
        | RenderResourceDeclaration::TargetAlias(_)
        | RenderResourceDeclaration::ImportedTexture(_)
        | RenderResourceDeclaration::ImportedBuffer(_) => {}
    }
}

#[derive(Debug, Clone, Copy)]
enum TextureCapability {
    Sampled,
    Storage,
    ColorAttachment,
    DepthAttachment,
}

fn validate_texture_format_capability(
    flow: &CompiledRenderFlowPlan,
    resource_id: runen_gpu::GpuWorkResourceId,
    format: RenderTextureFormatPolicy,
    capabilities: &GpuCapabilities,
    required: TextureCapability,
    diagnostics: &mut Vec<RenderExecutionGraphDiagnostic>,
) {
    let RenderTextureFormatPolicy::Exact(format) = format else {
        return;
    };
    let normalized = normalized_render_format(format);
    let supported = capabilities
        .format(normalized)
        .is_some_and(|facts| match required {
            TextureCapability::Sampled => facts.sampled,
            TextureCapability::Storage => facts.storage_write,
            TextureCapability::ColorAttachment => facts.color_attachment,
            TextureCapability::DepthAttachment => facts.depth_stencil,
        });
    if supported {
        return;
    }
    diagnostics.push(resource_capability_diagnostic(
        flow,
        resource_id,
        format!("format::{normalized:?}::{required:?}"),
        format!(
            "resource '{}' requires {:?} support for normalized format {:?}",
            resource_id, required, normalized
        ),
    ));
}

pub const fn normalized_render_format(format: RenderTextureTargetFormat) -> GpuTextureFormat {
    match format {
        RenderTextureTargetFormat::Rgba8Unorm => GpuTextureFormat::Rgba8Unorm,
        RenderTextureTargetFormat::Rgba8UnormSrgb => GpuTextureFormat::Rgba8UnormSrgb,
        RenderTextureTargetFormat::R32Uint => GpuTextureFormat::R32Uint,
        RenderTextureTargetFormat::Depth32Float => GpuTextureFormat::Depth32Float,
    }
}

fn resource_capability_diagnostic(
    flow: &CompiledRenderFlowPlan,
    resource_id: runen_gpu::GpuWorkResourceId,
    capability: impl Into<String>,
    message: impl Into<String>,
) -> RenderExecutionGraphDiagnostic {
    RenderExecutionGraphDiagnostic::error(
        RenderExecutionGraphDiagnosticKind::BackendCapabilityMismatch,
        message,
    )
    .with_flow(flow.flow_id, flow.flow_label.clone())
    .with_resource(resource_id, flow.resource_label(resource_id))
    .with_capability(capability)
}

pub fn bind_group_entry_count(pass: &CompiledPassExecutionPlan) -> usize {
    match pass {
        CompiledPassExecutionPlan::Compute(value) => value.bindings.bind_group.entries.len(),
        CompiledPassExecutionPlan::Fullscreen(value)
        | CompiledPassExecutionPlan::Graphics(value) => value.bindings.bind_group.entries.len(),
        CompiledPassExecutionPlan::Copy(_)
        | CompiledPassExecutionPlan::Present(_)
        | CompiledPassExecutionPlan::BuiltinUiComposite(_) => 0,
    }
}

pub fn binding_entry_resource_label(entry: &CompiledBindingEntry) -> &'static str {
    match entry {
        CompiledBindingEntry::SampledTexture { .. } => "sampled_texture",
        CompiledBindingEntry::Sampler { .. } => "sampler",
        CompiledBindingEntry::StorageTexture { .. } => "storage_texture",
        CompiledBindingEntry::UniformBuffer { .. } => "uniform_buffer",
        CompiledBindingEntry::StorageBuffer { .. } => "storage_buffer",
    }
}
