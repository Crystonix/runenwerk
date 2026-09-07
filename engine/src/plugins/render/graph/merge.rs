use crate::plugins::render::composition::{
    RenderFragmentDescriptor, RenderFragmentDiagnostic, RenderFragmentDiagnosticKind,
    RenderFragmentLabelRef, RenderFragmentMergeReport, RenderFragmentPackageDescriptor,
    RenderFragmentPassDescriptor, RenderFragmentPassKind, RenderFragmentProvenanceElementKind,
    RenderFragmentProvenanceRecord, RenderFragmentResourceKind, RenderFragmentShaderBinding,
    validate_fragment_package,
};
use crate::plugins::render::{
    RenderFlow, RenderFlowAuthoringError, RenderPassViewScope, RenderTargetAliasKind,
    compile_flow_plan_checked,
};
use runen_gpu::GpuCapabilities;

#[derive(Debug)]
pub struct RenderFragmentMergeResult {
    pub flow: RenderFlow,
    pub report: RenderFragmentMergeReport,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{message}")]
pub struct RenderFragmentMergeError {
    pub diagnostics: Vec<RenderFragmentDiagnostic>,
    pub message: String,
    #[source]
    authoring_error: Option<RenderFlowAuthoringError>,
}

impl RenderFragmentMergeError {
    pub fn new(diagnostics: Vec<RenderFragmentDiagnostic>) -> Self {
        let message = if diagnostics.is_empty() {
            "render fragment merge failed".to_string()
        } else {
            let details = diagnostics
                .iter()
                .filter(|diagnostic| diagnostic.is_error())
                .map(|diagnostic| diagnostic.message.as_str())
                .collect::<Vec<_>>()
                .join("; ");
            format!("render fragment merge failed: {details}")
        };
        Self {
            diagnostics,
            message,
            authoring_error: None,
        }
    }

    fn from_authoring(
        error: RenderFlowAuthoringError,
        package: &RenderFragmentPackageDescriptor,
        fragment: &RenderFragmentDescriptor,
    ) -> Self {
        let diagnostic = RenderFragmentDiagnostic::error(
            RenderFragmentDiagnosticKind::CompileValidationFailed,
            error.to_string(),
        )
        .with_package(package)
        .with_fragment(fragment);
        let message = format!("render fragment merge failed: {error}");
        Self {
            diagnostics: vec![diagnostic],
            message,
            authoring_error: Some(error),
        }
    }
}

pub fn merge_fragment_package_into_flow(
    mut flow: RenderFlow,
    package: &RenderFragmentPackageDescriptor,
    capabilities: &GpuCapabilities,
) -> Result<RenderFragmentMergeResult, RenderFragmentMergeError> {
    let validation = validate_fragment_package(package);
    if validation.has_errors() {
        return Err(RenderFragmentMergeError::new(validation.diagnostics));
    }

    let mut report = RenderFragmentMergeReport {
        package_id: Some(package.package_id.clone()),
        source_path: Some(package.source_path.clone()),
        source_revision: Some(crate::plugins::render::composition::RenderFragmentRevision(
            package.source_revision,
        )),
        generated_flow_id: None,
        provenance: Vec::new(),
        diagnostics: Vec::new(),
    };

    let mut diagnostics = Vec::<RenderFragmentDiagnostic>::new();
    for fragment in &package.fragments {
        diagnostics.extend(validate_merge_references(&flow, package, fragment));
        if diagnostics.iter().any(RenderFragmentDiagnostic::is_error) {
            return Err(RenderFragmentMergeError::new(diagnostics));
        }
        flow = merge_fragment_into_flow(flow, package, fragment, &mut report)
            .map_err(|error| RenderFragmentMergeError::from_authoring(error, package, fragment))?;
    }

    match compile_flow_plan_checked(&flow, capabilities) {
        Ok(_) => {
            report.generated_flow_id = Some(flow.id().to_string());
            Ok(RenderFragmentMergeResult { flow, report })
        }
        Err(err) => {
            let diagnostics = err
                .diagnostics
                .into_iter()
                .map(|diagnostic| {
                    let kind = match diagnostic.kind {
                        crate::plugins::render::RenderExecutionGraphDiagnosticKind::BackendCapabilityMismatch => {
                            RenderFragmentDiagnosticKind::BackendCapabilityMismatch
                        }
                        _ => RenderFragmentDiagnosticKind::CompileValidationFailed,
                    };
                    RenderFragmentDiagnostic::error(kind, diagnostic.message).with_package(package)
                })
                .collect::<Vec<_>>();
            Err(RenderFragmentMergeError::new(diagnostics))
        }
    }
}

fn validate_merge_references(
    flow: &RenderFlow,
    package: &RenderFragmentPackageDescriptor,
    fragment: &RenderFragmentDescriptor,
) -> Vec<RenderFragmentDiagnostic> {
    let mut diagnostics = Vec::<RenderFragmentDiagnostic>::new();
    for pass in &fragment.passes {
        for reference in pass
            .sample_textures
            .iter()
            .chain(pass.write_textures.iter())
            .chain(pass.color_outputs.iter())
            .chain(pass.depth_target.iter())
            .chain(pass.copy_source.iter())
            .chain(pass.copy_destination.iter())
            .chain(pass.present_source.iter())
        {
            if matches!(reference, RenderFragmentLabelRef::Absolute(label) if flow.resolve_resource_id(label).is_none())
            {
                diagnostics.push(
                    RenderFragmentDiagnostic::error(
                        RenderFragmentDiagnosticKind::MissingResourceReference,
                        format!(
                            "pass '{}' references missing absolute resource '{}'",
                            pass.label,
                            reference.raw_label()
                        ),
                    )
                    .with_package(package)
                    .with_fragment(fragment)
                    .with_label(pass.label.clone()),
                );
            }
        }

        for ordered_before in &pass.non_data_order_after {
            if matches!(ordered_before, RenderFragmentLabelRef::Absolute(label) if flow.resolve_pass_id(label).is_none())
            {
                diagnostics.push(
                    RenderFragmentDiagnostic::error(
                        RenderFragmentDiagnosticKind::MissingPassReference,
                        format!(
                            "pass '{}' orders after missing absolute pass '{}'",
                            pass.label,
                            ordered_before.raw_label()
                        ),
                    )
                    .with_package(package)
                    .with_fragment(fragment)
                    .with_label(pass.label.clone()),
                );
            }
        }
    }
    diagnostics
}

fn merge_fragment_into_flow(
    mut flow: RenderFlow,
    package: &RenderFragmentPackageDescriptor,
    fragment: &RenderFragmentDescriptor,
    report: &mut RenderFragmentMergeReport,
) -> Result<RenderFlow, RenderFlowAuthoringError> {
    for resource in &fragment.resources {
        let generated_label = resource.generated_label(&fragment.namespace);
        flow = match resource.kind {
            RenderFragmentResourceKind::SurfaceColor => flow.with_surface_color(),
            RenderFragmentResourceKind::SurfaceDepth => flow.with_surface_depth(),
            RenderFragmentResourceKind::ColorTarget => {
                flow.with_color_target(generated_label.clone())
            }
            RenderFragmentResourceKind::ColorTargetExact(format) => {
                flow.with_color_target_exact(generated_label.clone(), format)
            }
            RenderFragmentResourceKind::DepthTarget => {
                flow.with_depth_target(generated_label.clone())
            }
            RenderFragmentResourceKind::SampledTexture => {
                flow.with_sampled_texture(generated_label.clone())
            }
            RenderFragmentResourceKind::StorageTexture => {
                flow.with_storage_texture(generated_label.clone())
            }
            RenderFragmentResourceKind::HistoryTexture => {
                flow.with_history_texture(generated_label.clone())
            }
            RenderFragmentResourceKind::TargetAlias(kind) => {
                flow.with_target_alias(generated_label.clone(), kind)
            }
        }?;
        report.provenance.push(provenance_record(
            package,
            fragment,
            RenderFragmentProvenanceElementKind::Resource,
            resource.label.clone(),
            generated_label,
        ));
    }

    for pass in &fragment.passes {
        let generated_label = pass.generated_label(&fragment.namespace);
        flow = merge_pass_into_flow(flow, pass, &fragment.namespace)?;
        report.provenance.push(provenance_record(
            package,
            fragment,
            RenderFragmentProvenanceElementKind::Pass,
            pass.label.clone(),
            generated_label,
        ));
        for ordered_before in &pass.non_data_order_after {
            report.provenance.push(provenance_record(
                package,
                fragment,
                RenderFragmentProvenanceElementKind::NonDataOrder,
                ordered_before.raw_label().to_string(),
                ordered_before.resolve(&fragment.namespace),
            ));
        }
    }
    Ok(flow)
}

fn merge_pass_into_flow(
    flow: RenderFlow,
    pass: &RenderFragmentPassDescriptor,
    namespace: &crate::plugins::render::composition::RenderFragmentNamespace,
) -> Result<RenderFlow, RenderFlowAuthoringError> {
    let label = pass.generated_label(namespace);
    match pass.kind {
        RenderFragmentPassKind::Compute => {
            let mut builder = flow.compute_pass(label);
            builder = apply_compute_view_scope(builder, pass.view_scope);
            if let Some(
                crate::plugins::render::composition::RenderFragmentShaderReference::AssetPath(path),
            ) = &pass.shader
            {
                builder = builder.shader_asset(path.clone());
            }
            for binding in &pass.shader_bindings {
                if let RenderFragmentShaderBinding::StorageTexture { binding, resource } = binding {
                    builder = builder.write_texture(*binding, resource.resolve(namespace));
                }
            }
            if let Some(dispatch) = pass.compute_dispatch {
                builder = builder.dispatch(dispatch);
            }
            for ordered_before in &pass.non_data_order_after {
                builder = builder.order_after(ordered_before.resolve(namespace));
            }
            Ok(builder.finish())
        }
        RenderFragmentPassKind::Fullscreen => {
            let mut builder = flow.fullscreen_pass(label);
            builder = apply_fullscreen_view_scope(builder, pass.view_scope);
            if let Some(shader) = &pass.shader {
                match shader {
                    crate::plugins::render::composition::RenderFragmentShaderReference::AssetPath(path) => {
                        builder = builder.shader_asset(path.clone());
                    }
                    crate::plugins::render::composition::RenderFragmentShaderReference::MaterialSceneBundle { fallback_asset } => {
                        builder = builder.material_scene_shader_asset(fallback_asset.clone());
                    }
                }
            }
            for binding in &pass.shader_bindings {
                match binding {
                    RenderFragmentShaderBinding::SampledTexture {
                        texture_binding,
                        sampler_binding,
                        resource,
                    } => {
                        builder = builder.sample_texture(
                            *texture_binding,
                            *sampler_binding,
                            resource.resolve(namespace),
                        );
                    }
                    RenderFragmentShaderBinding::StorageTexture { binding, resource } => {
                        builder = builder.write_texture(*binding, resource.resolve(namespace));
                    }
                }
            }
            for color in &pass.color_outputs {
                builder = builder.write_color_target(color.resolve(namespace));
            }
            if pass.write_surface_color {
                builder = builder.write_surface_color()?;
            }
            if let Some(clear_color) = pass.clear_color {
                builder = builder.clear_color(clear_color);
            }
            for ordered_before in &pass.non_data_order_after {
                builder = builder.order_after(ordered_before.resolve(namespace));
            }
            Ok(builder.finish())
        }
        RenderFragmentPassKind::Graphics => {
            let mut builder = flow.graphics_pass(label);
            builder = apply_graphics_view_scope(builder, pass.view_scope);
            if let Some(shader) = &pass.shader {
                match shader {
                    crate::plugins::render::composition::RenderFragmentShaderReference::AssetPath(path) => {
                        builder = builder.shader_asset(path.clone());
                    }
                    crate::plugins::render::composition::RenderFragmentShaderReference::MaterialSceneBundle { fallback_asset } => {
                        builder = builder.material_scene_shader_asset(fallback_asset.clone());
                    }
                }
            }
            for binding in &pass.shader_bindings {
                match binding {
                    RenderFragmentShaderBinding::SampledTexture {
                        texture_binding,
                        sampler_binding,
                        resource,
                    } => {
                        builder = builder.sample_texture(
                            *texture_binding,
                            *sampler_binding,
                            resource.resolve(namespace),
                        );
                    }
                    RenderFragmentShaderBinding::StorageTexture { binding, resource } => {
                        builder = builder.write_texture(*binding, resource.resolve(namespace));
                    }
                }
            }
            for color in &pass.color_outputs {
                builder = builder.write_color_target(color.resolve(namespace));
            }
            if pass.write_surface_color {
                builder = builder.write_surface_color()?;
            }
            if let Some(depth_target) = &pass.depth_target {
                builder = builder.depth_target(depth_target.resolve(namespace));
            }
            if let Some(clear_color) = pass.clear_color {
                builder = builder.clear_color(clear_color);
            }
            if let Some(draw) = pass.draw {
                builder = builder.draw_with_offsets(
                    draw.vertex_count,
                    draw.instance_count,
                    draw.first_vertex,
                    draw.first_instance,
                );
            }
            for ordered_before in &pass.non_data_order_after {
                builder = builder.order_after(ordered_before.resolve(namespace));
            }
            Ok(builder.finish())
        }
        RenderFragmentPassKind::Copy => {
            let mut builder = flow.copy_pass(label);
            builder = apply_copy_view_scope(builder, pass.view_scope);
            if let Some(source) = &pass.copy_source {
                builder = builder.source(source.resolve(namespace));
            }
            if let Some(destination) = &pass.copy_destination {
                builder = builder.destination(destination.resolve(namespace));
            }
            for ordered_before in &pass.non_data_order_after {
                builder = builder.order_after(ordered_before.resolve(namespace));
            }
            Ok(builder.finish())
        }
        RenderFragmentPassKind::Present => {
            let mut builder = flow.present_pass(label)?;
            if pass.view_scope == RenderPassViewScope::MainSurfaceOnly {
                builder = builder.main_surface_only();
            }
            if pass.write_surface_color {
                builder = builder.surface_color()?;
            }
            if let Some(source) = &pass.present_source {
                builder = builder.source(source.resolve(namespace));
            }
            for ordered_before in &pass.non_data_order_after {
                builder = builder.order_after(ordered_before.resolve(namespace));
            }
            Ok(builder.finish())
        }
        RenderFragmentPassKind::BuiltinUiComposite => {
            let mut builder = flow.builtin_ui_composite_pass(label)?;
            if pass.view_scope == RenderPassViewScope::MainSurfaceOnly {
                builder = builder.main_surface_only();
            }
            for ordered_before in &pass.non_data_order_after {
                builder = builder.order_after(ordered_before.resolve(namespace));
            }
            Ok(builder.finish())
        }
    }
}

fn apply_compute_view_scope(
    builder: crate::plugins::render::api::ComputePassBuilder,
    view_scope: RenderPassViewScope,
) -> crate::plugins::render::api::ComputePassBuilder {
    match view_scope {
        RenderPassViewScope::AllViews => builder,
        RenderPassViewScope::MainSurfaceOnly => builder.main_surface_only(),
        RenderPassViewScope::OffscreenProductsOnly => builder.offscreen_products_only(),
    }
}

fn apply_fullscreen_view_scope(
    builder: crate::plugins::render::api::FullscreenPassBuilder,
    view_scope: RenderPassViewScope,
) -> crate::plugins::render::api::FullscreenPassBuilder {
    match view_scope {
        RenderPassViewScope::AllViews => builder,
        RenderPassViewScope::MainSurfaceOnly => builder.main_surface_only(),
        RenderPassViewScope::OffscreenProductsOnly => builder.offscreen_products_only(),
    }
}

fn apply_graphics_view_scope(
    builder: crate::plugins::render::api::GraphicsPassBuilder,
    view_scope: RenderPassViewScope,
) -> crate::plugins::render::api::GraphicsPassBuilder {
    match view_scope {
        RenderPassViewScope::AllViews => builder,
        RenderPassViewScope::MainSurfaceOnly => builder.main_surface_only(),
        RenderPassViewScope::OffscreenProductsOnly => builder.offscreen_products_only(),
    }
}

fn apply_copy_view_scope(
    builder: crate::plugins::render::api::CopyPassBuilder,
    view_scope: RenderPassViewScope,
) -> crate::plugins::render::api::CopyPassBuilder {
    match view_scope {
        RenderPassViewScope::AllViews => builder,
        RenderPassViewScope::MainSurfaceOnly => builder.main_surface_only(),
        RenderPassViewScope::OffscreenProductsOnly => builder.offscreen_products_only(),
    }
}

fn provenance_record(
    package: &RenderFragmentPackageDescriptor,
    fragment: &RenderFragmentDescriptor,
    element_kind: RenderFragmentProvenanceElementKind,
    source_label: String,
    generated_label: String,
) -> RenderFragmentProvenanceRecord {
    RenderFragmentProvenanceRecord {
        package_id: package.package_id.clone(),
        fragment_id: fragment.id.clone(),
        namespace: fragment.namespace.clone(),
        source_path: package.source_path.clone(),
        source_revision: crate::plugins::render::composition::RenderFragmentRevision(
            package.source_revision,
        ),
        element_kind,
        source_label,
        generated_label,
    }
}

#[allow(dead_code)]
fn _target_alias_kind_name(kind: RenderTargetAliasKind) -> &'static str {
    match kind {
        RenderTargetAliasKind::Color => "color",
        RenderTargetAliasKind::Depth => "depth",
        RenderTargetAliasKind::Texture => "texture",
    }
}
