use engine::plugins::render::inspect::{
    inspect_fragment_pass_provenance, inspect_render_fragment_merge_report,
};
use engine::plugins::render::{
    RenderFlow, RenderFragmentDescriptor, RenderFragmentDiagnosticKind,
    RenderFragmentPackageDescriptor, RenderFragmentPackageId, RenderFragmentPackageStatus,
    RenderFragmentPassDescriptor, RenderFragmentResourceDescriptor, RenderTextureTargetFormat,
    current_runtime_gpu_capabilities, merge_fragment_package_into_flow, validate_fragment_package,
};
use runen_gpu::{GpuBindingKey, GpuCapabilities, GpuCapabilityFeature};

fn binding_key(binding: u64) -> GpuBindingKey {
    GpuBindingKey::try_new(0, binding).expect("fragment test binding should fit GpuBindingKey")
}

fn compositor_package(revision: u64) -> RenderFragmentPackageDescriptor {
    RenderFragmentPackageDescriptor::new(
        "pkg.compositor",
        "compositor",
        "render/fragments/compositor.ron",
        revision,
    )
    .with_fragment(
        RenderFragmentDescriptor::new("compose", "compositor")
            .with_resource(RenderFragmentResourceDescriptor::color_target_exact(
                "scene",
                RenderTextureTargetFormat::Rgba8Unorm,
            ))
            .with_pass(
                RenderFragmentPassDescriptor::fullscreen("compose")
                    .shader_asset("assets/shaders/fullscreen_composite.wgsl")
                    .write_local_color_target("scene")
                    .clear_color([0.0, 0.0, 0.0, 1.0]),
            ),
    )
}

fn invalid_package(revision: u64) -> RenderFragmentPackageDescriptor {
    RenderFragmentPackageDescriptor::new(
        "pkg.compositor",
        "compositor",
        "render/fragments/compositor.ron",
        revision,
    )
    .with_fragment(
        RenderFragmentDescriptor::new("compose", "compositor").with_pass(
            RenderFragmentPassDescriptor::fullscreen("compose")
                .shader_asset("assets/shaders/fullscreen_composite.wgsl")
                .write_local_color_target("missing"),
        ),
    )
}

fn compute_fragment_package_with_sampled_texture() -> RenderFragmentPackageDescriptor {
    RenderFragmentPackageDescriptor::new(
        "pkg.compute.sample",
        "compute_sample",
        "render/fragments/compute-sample.ron",
        1,
    )
    .with_fragment(
        RenderFragmentDescriptor::new("simulate", "compute_sample")
            .with_resource(RenderFragmentResourceDescriptor::sampled_texture("input"))
            .with_resource(RenderFragmentResourceDescriptor::storage_texture("state"))
            .with_pass(
                RenderFragmentPassDescriptor::compute("simulate")
                    .shader_asset("assets/shaders/simulate.wgsl")
                    .sample_local_texture(binding_key(0), binding_key(1), "input")
                    .write_local_texture(binding_key(2), "state")
                    .dispatch([1, 1, 1]),
            ),
    )
}

fn valid_compute_fragment_package() -> RenderFragmentPackageDescriptor {
    RenderFragmentPackageDescriptor::new(
        "pkg.compute.write",
        "compute_write",
        "render/fragments/compute-write.ron",
        1,
    )
    .with_fragment(
        RenderFragmentDescriptor::new("simulate", "compute_write")
            .with_resource(RenderFragmentResourceDescriptor::storage_texture("state"))
            .with_pass(
                RenderFragmentPassDescriptor::compute("simulate")
                    .shader_asset("assets/shaders/simulate.wgsl")
                    .write_local_texture(binding_key(0), "state")
                    .dispatch([1, 1, 1]),
            ),
    )
}

fn fullscreen_fragment_package_with_sampled_texture() -> RenderFragmentPackageDescriptor {
    RenderFragmentPackageDescriptor::new(
        "pkg.fullscreen.sample",
        "fullscreen_sample",
        "render/fragments/fullscreen-sample.ron",
        1,
    )
    .with_fragment(
        RenderFragmentDescriptor::new("compose", "fullscreen_sample")
            .with_resource(RenderFragmentResourceDescriptor::sampled_texture("input"))
            .with_resource(RenderFragmentResourceDescriptor::color_target_exact(
                "scene",
                RenderTextureTargetFormat::Rgba8Unorm,
            ))
            .with_pass(
                RenderFragmentPassDescriptor::fullscreen("compose")
                    .shader_asset("assets/shaders/fullscreen_composite.wgsl")
                    .sample_local_texture(binding_key(0), binding_key(1), "input")
                    .write_local_color_target("scene"),
            ),
    )
}

fn graphics_fragment_package_with_sampled_texture() -> RenderFragmentPackageDescriptor {
    RenderFragmentPackageDescriptor::new(
        "pkg.graphics.sample",
        "graphics_sample",
        "render/fragments/graphics-sample.ron",
        1,
    )
    .with_fragment(
        RenderFragmentDescriptor::new("draw", "graphics_sample")
            .with_resource(RenderFragmentResourceDescriptor::sampled_texture("input"))
            .with_resource(RenderFragmentResourceDescriptor::color_target_exact(
                "scene",
                RenderTextureTargetFormat::Rgba8Unorm,
            ))
            .with_pass(
                RenderFragmentPassDescriptor::graphics("draw")
                    .shader_asset("assets/shaders/mesh.wgsl")
                    .sample_local_texture(binding_key(0), binding_key(1), "input")
                    .write_local_color_target("scene")
                    .draw(3, 1),
            ),
    )
}

#[test]
fn fragment_validation_rejects_missing_local_resources_before_merge() {
    let report = validate_fragment_package(&invalid_package(1));

    assert!(report.has_errors());
    assert!(report.diagnostics.iter().any(|diagnostic| {
        diagnostic.kind == RenderFragmentDiagnosticKind::MissingResourceReference
            && diagnostic.label.as_deref() == Some("compose")
    }));
}

#[test]
fn fragment_validation_rejects_compute_sampled_textures() {
    let report = validate_fragment_package(&compute_fragment_package_with_sampled_texture());

    assert!(report.has_errors());
    assert!(report.diagnostics.iter().any(|diagnostic| {
        diagnostic.kind == RenderFragmentDiagnosticKind::InvalidPassShape
            && diagnostic.label.as_deref() == Some("simulate")
            && diagnostic.message.contains("sampled textures")
    }));
}

#[test]
fn fragment_validation_accepts_compute_dispatch_and_write_textures() {
    let report = validate_fragment_package(&valid_compute_fragment_package());

    assert!(
        !report.has_errors(),
        "compute fragments with dispatch and write textures should remain valid: {:?}",
        report.diagnostics
    );
}

#[test]
fn fragment_validation_keeps_sampled_textures_valid_for_fullscreen_and_graphics_passes() {
    let fullscreen_report =
        validate_fragment_package(&fullscreen_fragment_package_with_sampled_texture());
    let graphics_report =
        validate_fragment_package(&graphics_fragment_package_with_sampled_texture());

    assert!(
        !fullscreen_report.has_errors(),
        "fullscreen sampled textures should remain valid: {:?}",
        fullscreen_report.diagnostics
    );
    assert!(
        !graphics_report.has_errors(),
        "graphics sampled textures should remain valid: {:?}",
        graphics_report.diagnostics
    );
}

#[test]
fn fragment_merge_qualifies_labels_and_compiles_normal_render_flow() {
    let package = compositor_package(7);
    let merged = merge_fragment_package_into_flow(
        RenderFlow::new("fragment.flow"),
        &package,
        &current_runtime_gpu_capabilities(),
    )
    .expect("valid fragment package should merge and compile");

    assert!(
        merged.flow.resource_id("compositor::scene").is_some(),
        "fragment resource labels must be namespace qualified"
    );
    assert!(
        merged.flow.pass_id("compositor::compose").is_some(),
        "fragment pass labels must be namespace qualified"
    );
    assert_eq!(merged.flow.lexical_pass_order().unwrap().len(), 1);
    let flow_id = merged.flow.id().to_string();
    assert_eq!(
        merged.report.generated_flow_id.as_deref(),
        Some(flow_id.as_str())
    );
    assert!(merged.report.provenance.iter().any(|record| {
        record.source_label == "compose" && record.generated_label == "compositor::compose"
    }));
}

#[test]
fn fragment_merge_reports_backend_capability_failures_as_typed_diagnostics() {
    let package = RenderFragmentPackageDescriptor::new(
        "pkg.compute",
        "compute_frag",
        "render/fragments/compute.ron",
        1,
    )
    .with_fragment(
        RenderFragmentDescriptor::new("simulate", "compute_frag")
            .with_resource(RenderFragmentResourceDescriptor::storage_texture("state"))
            .with_pass(
                RenderFragmentPassDescriptor::compute("simulate")
                    .shader_asset("assets/shaders/simulate.wgsl")
                    .write_local_texture(binding_key(0), "state")
                    .dispatch([1, 1, 1]),
            ),
    );

    let error = merge_fragment_package_into_flow(
        RenderFlow::new("fragment.compute"),
        &package,
        &capabilities_without(GpuCapabilityFeature::Compute),
    )
    .expect_err("unsupported compute backend should reject fragment merge");

    assert!(error.diagnostics.iter().any(|diagnostic| {
        diagnostic.kind == RenderFragmentDiagnosticKind::BackendCapabilityMismatch
    }));
}

fn capabilities_without(feature: GpuCapabilityFeature) -> GpuCapabilities {
    let capabilities = current_runtime_gpu_capabilities();
    GpuCapabilities::from_normalized_facts(
        capabilities
            .features()
            .filter(|candidate| *candidate != feature),
        capabilities.limits(),
        capabilities.formats(),
    )
}

#[test]
fn fragment_registry_preserves_last_good_revision_after_failed_reload() {
    let mut registry = engine::plugins::render::RenderFragmentRegistryResource::default();

    let first = registry.apply_package(compositor_package(1));
    assert_eq!(first.status, RenderFragmentPackageStatus::Active);
    assert_eq!(registry.active_packages().len(), 1);

    let failed = registry.apply_package(invalid_package(2));
    assert_eq!(failed.status, RenderFragmentPackageStatus::Failed);
    assert_eq!(failed.active_revision.map(|revision| revision.0), Some(1));
    assert_eq!(
        failed.last_good_revision.map(|revision| revision.0),
        Some(1)
    );
    assert_eq!(failed.failed_revision.map(|revision| revision.0), Some(2));
    assert_eq!(
        registry.active_packages().len(),
        1,
        "failed reload must not remove the last-good active package"
    );

    let merged = registry
        .merge_active_packages(
            RenderFlow::new("fragment.registry"),
            &current_runtime_gpu_capabilities(),
        )
        .expect("last-good fragment package should still merge");
    assert!(merged.flow.pass_id("compositor::compose").is_some());
}

#[test]
fn fragment_inspection_exposes_merge_and_pass_provenance() {
    let package = compositor_package(11);
    let merged = merge_fragment_package_into_flow(
        RenderFlow::new("fragment.inspect"),
        &package,
        &current_runtime_gpu_capabilities(),
    )
    .expect("valid fragment package should merge and compile");

    let inspection = inspect_render_fragment_merge_report(&merged.report);
    assert_eq!(inspection.package_id.as_deref(), Some("pkg.compositor"));
    assert_eq!(inspection.source_revision, Some(11));
    assert!(inspection.provenance_count >= 2);
    assert!(
        inspection.lines.iter().any(|line| {
            line.contains("compositor::compose") && line.contains("pkg.compositor")
        })
    );

    let pass_provenance = inspect_fragment_pass_provenance(&merged.report);
    assert_eq!(pass_provenance.len(), 1);
    assert_eq!(pass_provenance[0].fragment_id, "compose");
    assert_eq!(pass_provenance[0].generated_label, "compositor::compose");
}

#[test]
fn fragment_hot_reload_request_uses_registry_apply_path() {
    let mut registry = engine::plugins::render::RenderFragmentRegistryResource::default();
    let outcome = engine::plugins::render::apply_render_fragment_hot_reload(
        &mut registry,
        engine::plugins::render::RenderFragmentHotReloadRequest::new(compositor_package(3)),
    );

    assert_eq!(outcome.status, RenderFragmentPackageStatus::Active);
    assert_eq!(
        registry
            .record(&RenderFragmentPackageId::new("pkg.compositor"))
            .and_then(|record| record.active_revision)
            .map(|revision| revision.0),
        Some(3)
    );
}
