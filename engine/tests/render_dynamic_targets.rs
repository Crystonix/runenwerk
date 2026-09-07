use engine::plugins::render::{
    PreparedFlowInputs, PreparedFlowInvocation, PreparedFlowInvocationId,
    PreparedFlowInvocationRequest, PreparedFrameContext, PreparedFrameContributions,
    PreparedRenderFrame, PreparedRenderFrameRequestError, PreparedRenderFrameRequestKind,
    PreparedRenderFrameRequestResource, PreparedShaderSnapshot, PreparedSurfaceInfo,
    PreparedTargetBinding, PreparedViewFrame, PreparedViewKind, RenderDynamicTextureRetention,
    RenderDynamicTextureTargetDescriptor, RenderDynamicTextureTargetDescriptorError,
    RenderDynamicTextureTargetKey, RenderDynamicTextureTargetRequestRegistryResource,
    RenderDynamicTextureUploadDescriptor, RenderExecutionGraphDiagnosticKind, RenderFlow,
    RenderFlowId, RenderFrameProducerId, RenderProductSurfaceDiagnosticKind,
    RenderProductSurfaceDiagnosticSeverity, RenderProductSurfaceManifest,
    RenderProductSurfaceRequest, RenderProductSurfaceRequestBatch, RenderProductSurfaceRequestKind,
    RenderProductSurfaceStatusKind, RenderTargetAliasKey, RenderTextureSampleMode,
    RenderTextureTargetFormat, RenderTextureTargetUsage, RenderTextureUploadAlphaMode,
    compile_flow_plan, current_runtime_gpu_capabilities, prepared_render_frame_preflight_cache_key,
    validate_prepared_render_frame,
};
use runen_gpu::{GpuBindingKey, GpuWorkResourceId};
use std::collections::BTreeMap;
use ui_render_data::{ProductSurfaceTextureBindingSource, ViewportSurfaceBindingRegistry};

fn dynamic_descriptor(
    target_id: &str,
    width: u32,
    height: u32,
    format: RenderTextureTargetFormat,
    usage: RenderTextureTargetUsage,
    sample_mode: RenderTextureSampleMode,
) -> RenderDynamicTextureTargetDescriptor {
    RenderDynamicTextureTargetDescriptor::new(
        RenderDynamicTextureTargetKey::new("test.dynamic", target_id),
        width,
        height,
        format,
        usage,
        sample_mode,
        RenderDynamicTextureRetention::RetainWhileRequested,
    )
}

fn upload_descriptor(
    key: RenderDynamicTextureTargetKey,
    width: u32,
    height: u32,
) -> RenderDynamicTextureUploadDescriptor {
    RenderDynamicTextureUploadDescriptor::rgba8(
        key,
        0,
        0,
        width,
        height,
        RenderTextureUploadAlphaMode::Straight,
        1,
        vec![255; width as usize * height as usize * 4],
    )
}

fn producer(raw: u64) -> RenderFrameProducerId {
    RenderFrameProducerId::try_from_raw(raw).unwrap()
}

fn alias_key(value: &str) -> RenderTargetAliasKey {
    RenderTargetAliasKey::new(value).expect("test alias key should be valid")
}

fn binding_key(binding: u64) -> GpuBindingKey {
    GpuBindingKey::try_new(0, binding)
        .expect("dynamic-target test binding should fit GpuBindingKey")
}

fn test_resource_ids(count: usize) -> Vec<GpuWorkResourceId> {
    let labels = (0..count)
        .map(|index| format!("test.dynamic.resource.{index}"))
        .collect::<Vec<_>>();
    let flow = labels
        .iter()
        .fold(RenderFlow::new("test.dynamic.resources"), |flow, label| {
            flow.with_color_target(label.clone())
                .expect("render flow authoring should succeed")
        });
    labels
        .iter()
        .map(|label| flow.resource_id(label).expect("test resource should exist"))
        .collect()
}

fn prepared_frame_for_invocations(
    views: Vec<PreparedViewFrame>,
    invocations: Vec<PreparedFlowInvocation>,
    dynamic_texture_targets: Vec<RenderDynamicTextureTargetDescriptor>,
) -> PreparedRenderFrame {
    PreparedRenderFrame {
        context: PreparedFrameContext {
            frame_index: 1,
            flow_registry_revision: 1,
            shader_registry_revision: 1,
            prepare_epoch: 1,
        },
        surface: PreparedSurfaceInfo::primary((800, 600)),
        views,
        flows: BTreeMap::new(),
        flow_invocations: invocations,
        dynamic_texture_targets,
        dynamic_texture_uploads: Vec::new(),
        product_selections: Vec::new(),
        viewport_surface_bindings: ViewportSurfaceBindingRegistry::default(),
        contributions: PreparedFrameContributions::default(),
        shader: PreparedShaderSnapshot {
            registry_revision: 1,
        },
    }
}

fn invocation(
    invocation_id: &str,
    flow_id: RenderFlowId,
    view_id: &str,
    bindings: BTreeMap<RenderTargetAliasKey, PreparedTargetBinding>,
) -> PreparedFlowInvocation {
    PreparedFlowInvocation {
        invocation_id: PreparedFlowInvocationId::new(invocation_id),
        flow_id,
        view_id: view_id.to_string(),
        inputs: PreparedFlowInputs::default(),
        target_alias_bindings: bindings,
        history_signature: None,
    }
}

#[test]
fn render_dynamic_targets_descriptor_constructors_build_valid_common_shapes() {
    let retention = RenderDynamicTextureRetention::RetainWhileRequested;
    let color = RenderDynamicTextureTargetDescriptor::color_sampled(
        RenderDynamicTextureTargetKey::new("test.dynamic", "color"),
        64,
        64,
        RenderTextureTargetFormat::Rgba8Unorm,
        RenderTextureSampleMode::FilterableFloat,
        retention,
    );
    let attachment_only = RenderDynamicTextureTargetDescriptor::color_attachment_only(
        RenderDynamicTextureTargetKey::new("test.dynamic", "attachment"),
        64,
        64,
        RenderTextureTargetFormat::R32Uint,
        retention,
    );
    let storage = RenderDynamicTextureTargetDescriptor::storage_sampled(
        RenderDynamicTextureTargetKey::new("test.dynamic", "storage"),
        64,
        64,
        RenderTextureTargetFormat::Rgba8Unorm,
        RenderTextureSampleMode::FilterableFloat,
        retention,
    );
    let depth = RenderDynamicTextureTargetDescriptor::depth_sampled(
        RenderDynamicTextureTargetKey::new("test.dynamic", "depth"),
        64,
        64,
        retention,
    );

    for descriptor in [&color, &attachment_only, &storage, &depth] {
        descriptor
            .validate()
            .expect("constructor-built descriptor should validate");
        assert_eq!(descriptor.retention, retention);
    }
    assert!(color.usage.color_attachment);
    assert!(color.usage.sampled);
    assert!(attachment_only.usage.color_attachment);
    assert!(!attachment_only.usage.sampled);
    assert!(storage.usage.storage);
    assert_eq!(depth.format, RenderTextureTargetFormat::Depth32Float);
    assert!(depth.usage.depth_attachment);
}

#[test]
fn prepared_target_alias_bindings_validate_and_normalize_keys() {
    let flow_id = RenderFlowId::try_from_raw(7).unwrap();
    let request = PreparedFlowInvocationRequest::new("alias.keys", flow_id, "view")
        .bind_surface_color_alias("  scene_color  ")
        .expect("non-empty alias key should be valid");

    assert_eq!(
        request.target_alias_bindings.get(&alias_key("scene_color")),
        Some(&PreparedTargetBinding::SurfaceColor)
    );
    assert!(
        PreparedFlowInvocationRequest::new("alias.empty", flow_id, "view")
            .bind_surface_color_alias("   ")
            .is_err()
    );
}

#[test]
fn render_dynamic_targets_product_surface_manifest_builds_upload_backed_batches() {
    let key = RenderDynamicTextureTargetKey::new("test.product", "preview");
    let descriptor = RenderDynamicTextureTargetDescriptor::color_sampled(
        key.clone(),
        32,
        16,
        RenderTextureTargetFormat::Rgba8Unorm,
        RenderTextureSampleMode::FilterableFloat,
        RenderDynamicTextureRetention::RetainWhileRequested,
    );
    let upload = upload_descriptor(key.clone(), 32, 16);
    let manifest = RenderProductSurfaceManifest::new(producer(77), "test.product.preview")
        .with_dynamic_target(descriptor.clone())
        .with_dynamic_upload(upload.clone())
        .with_upload_backed_product_surface_binding(
            "preview.surface",
            ProductSurfaceTextureBindingSource::dynamic_texture(
                key.namespace.clone(),
                key.target_id.clone(),
            ),
        );

    assert!(
        manifest.diagnostics().is_empty(),
        "valid upload-backed product surface should have no manifest diagnostics"
    );
    assert!(manifest.product_bindings()[0].upload_required);

    let (targets, uploads, views, invocations) = manifest.into_render_parts();
    assert_eq!(targets, vec![descriptor]);
    assert_eq!(uploads, vec![upload]);
    assert!(views.is_empty());
    assert!(invocations.is_empty());
}

#[test]
fn render_dynamic_targets_product_surface_manifest_reports_typed_surface_diagnostics() {
    let key = RenderDynamicTextureTargetKey::new("test.product", "shared");
    let descriptor = RenderDynamicTextureTargetDescriptor::color_sampled(
        key.clone(),
        32,
        16,
        RenderTextureTargetFormat::Rgba8Unorm,
        RenderTextureSampleMode::FilterableFloat,
        RenderDynamicTextureRetention::RetainWhileRequested,
    );
    let manifest = RenderProductSurfaceManifest::new(producer(78), "test.product.preview")
        .with_dynamic_target(descriptor.clone())
        .with_dynamic_target(descriptor)
        .with_upload_backed_product_surface_binding(
            "preview.surface",
            ProductSurfaceTextureBindingSource::dynamic_texture(
                key.namespace.clone(),
                key.target_id.clone(),
            ),
        )
        .with_dynamic_upload(upload_descriptor(
            RenderDynamicTextureTargetKey::new("test.product", "missing-target"),
            32,
            16,
        ))
        .with_status(
            "preview.surface",
            RenderProductSurfaceStatusKind::Unavailable,
            "producer reported unavailable preview surface",
        );

    let diagnostics = manifest.diagnostics();
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.diagnostic_kind == RenderProductSurfaceDiagnosticKind::DuplicateSurfaceKey
            && diagnostic.request_kind == RenderProductSurfaceRequestKind::DynamicTarget
            && diagnostic.dynamic_target_key.as_ref() == Some(&key)
    }));
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.diagnostic_kind == RenderProductSurfaceDiagnosticKind::MissingUpload
            && diagnostic.request_kind == RenderProductSurfaceRequestKind::DynamicUpload
            && diagnostic.surface_key.as_deref() == Some("preview.surface")
    }));
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.diagnostic_kind == RenderProductSurfaceDiagnosticKind::MissingDynamicTarget
            && diagnostic.request_kind == RenderProductSurfaceRequestKind::DynamicUpload
            && diagnostic
                .dynamic_target_key
                .as_ref()
                .is_some_and(|key| key.target_id == "missing-target")
    }));
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.diagnostic_kind == RenderProductSurfaceDiagnosticKind::ProducerStatus
            && diagnostic.status == Some(RenderProductSurfaceStatusKind::Unavailable)
            && diagnostic.severity == RenderProductSurfaceDiagnosticSeverity::Error
    }));
}

#[test]
fn render_dynamic_targets_product_surface_manifest_reports_ui_sampleability_and_history_conflicts()
{
    let key = RenderDynamicTextureTargetKey::new("test.product", "history");
    let descriptor = RenderDynamicTextureTargetDescriptor::color_attachment_only(
        key.clone(),
        32,
        16,
        RenderTextureTargetFormat::Rgba8Unorm,
        RenderDynamicTextureRetention::RetainWhileRequested,
    );
    let flow_id = RenderFlowId::try_from_raw(7).unwrap();
    let first = PreparedFlowInvocationRequest::new("history.a", flow_id, "view.a")
        .bind_dynamic_texture_alias("surface", key.clone())
        .expect("test alias key should be valid")
        .with_history_signature("camera:a");
    let second = PreparedFlowInvocationRequest::new("history.b", flow_id, "view.b")
        .bind_dynamic_texture_alias("surface", key.clone())
        .expect("test alias key should be valid")
        .with_history_signature("camera:b");
    let manifest = RenderProductSurfaceManifest::new(producer(79), "test.product.history")
        .with_dynamic_target(descriptor)
        .with_product_surface_binding(
            "history.surface",
            ProductSurfaceTextureBindingSource::dynamic_texture(
                key.namespace.clone(),
                key.target_id.clone(),
            ),
        )
        .with_flow_invocation(first)
        .with_flow_invocation(second);

    let diagnostics = manifest.diagnostics();
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.diagnostic_kind == RenderProductSurfaceDiagnosticKind::NonSampleableUiBinding
            && diagnostic.request_kind == RenderProductSurfaceRequestKind::ProductSurfaceBinding
            && diagnostic.dynamic_target_key.as_ref() == Some(&key)
    }));
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.diagnostic_kind
            == RenderProductSurfaceDiagnosticKind::ConflictingHistorySignature
            && diagnostic.request_kind == RenderProductSurfaceRequestKind::HistorySignature
            && diagnostic.invocation_id.as_ref().map(|id| id.0.as_str()) == Some("history.b")
    }));
}

#[test]
fn render_dynamic_targets_descriptor_validation_rejects_invalid_shapes() {
    let zero = dynamic_descriptor(
        "zero",
        0,
        64,
        RenderTextureTargetFormat::Rgba8Unorm,
        RenderTextureTargetUsage::color_sampled(),
        RenderTextureSampleMode::FilterableFloat,
    );
    assert!(matches!(
        zero.validate(),
        Err(RenderDynamicTextureTargetDescriptorError::ZeroDimensions {
            width: 0,
            height: 64
        })
    ));

    let sampled_without_usage = dynamic_descriptor(
        "sampled-without-usage",
        64,
        64,
        RenderTextureTargetFormat::Rgba8Unorm,
        RenderTextureTargetUsage::color_attachment_only(),
        RenderTextureSampleMode::FilterableFloat,
    );
    assert!(matches!(
        sampled_without_usage.validate(),
        Err(RenderDynamicTextureTargetDescriptorError::SampleModeRequiresSampledUsage)
    ));

    let uint_with_float_sampling = dynamic_descriptor(
        "uint-float-sampling",
        64,
        64,
        RenderTextureTargetFormat::R32Uint,
        RenderTextureTargetUsage::color_sampled(),
        RenderTextureSampleMode::FilterableFloat,
    );
    assert!(matches!(
        uint_with_float_sampling.validate(),
        Err(RenderDynamicTextureTargetDescriptorError::InvalidSampleModeForFormat)
    ));

    let depth_with_color_usage = dynamic_descriptor(
        "depth-color",
        64,
        64,
        RenderTextureTargetFormat::Depth32Float,
        RenderTextureTargetUsage::color_sampled(),
        RenderTextureSampleMode::Depth,
    );
    assert!(matches!(
        depth_with_color_usage.validate(),
        Err(RenderDynamicTextureTargetDescriptorError::InvalidDepthUsage)
    ));
}

#[test]
fn render_dynamic_targets_preflight_reports_missing_target_alias_binding() {
    let flow = RenderFlow::new("preflight.alias.missing")
        .with_color_target_alias("scene_color")
        .expect("render flow authoring should succeed")
        .fullscreen_pass("compose")
        .offscreen_products_only()
        .write_target_alias("scene_color")
        .finish()
        .validate()
        .expect("flow should validate");
    let compiled = compile_flow_plan(&flow).expect("flow should compile");
    let frame = prepared_frame_for_invocations(
        vec![PreparedViewFrame::offscreen_product(
            "viewport.1",
            (320, 180),
        )],
        vec![invocation(
            "viewport.1.scene",
            compiled.flow_id,
            "viewport.1",
            BTreeMap::new(),
        )],
        Vec::new(),
    );

    let report =
        validate_prepared_render_frame(&frame, &[compiled], &current_runtime_gpu_capabilities());

    assert!(report.diagnostics.iter().any(|diagnostic| {
        diagnostic.kind == RenderExecutionGraphDiagnosticKind::TargetAliasMissingBinding
            && diagnostic
                .alias_binding_key
                .as_ref()
                .is_some_and(|key| key.as_str() == "scene_color")
            && diagnostic.view_id.as_deref() == Some("viewport.1")
    }));
}

#[test]
fn render_dynamic_targets_preflight_rejects_non_sampleable_dynamic_target_when_sampled() {
    let scene_texture_binding = binding_key(0);
    let scene_sampler_binding = binding_key(1);
    let flow = RenderFlow::new("preflight.dynamic.sampled")
        .with_surface_color()
        .expect("render flow authoring should succeed")
        .with_color_target_alias("scene_color")
        .expect("render flow authoring should succeed")
        .fullscreen_pass("draw_scene")
        .offscreen_products_only()
        .write_target_alias("scene_color")
        .finish()
        .fullscreen_pass("sample_scene")
        .offscreen_products_only()
        .sample_texture(scene_texture_binding, scene_sampler_binding, "scene_color")
        .write_surface_color()
        .expect("render flow authoring should succeed")
        .finish()
        .validate()
        .expect("flow should validate");
    let compiled = compile_flow_plan(&flow).expect("flow should compile");
    let key = RenderDynamicTextureTargetKey::new("preflight", "scene");
    let descriptor = RenderDynamicTextureTargetDescriptor::color_attachment_only(
        key.clone(),
        320,
        180,
        RenderTextureTargetFormat::Rgba8Unorm,
        RenderDynamicTextureRetention::RetainWhileRequested,
    );
    let mut bindings = BTreeMap::new();
    bindings.insert(
        alias_key("scene_color"),
        PreparedTargetBinding::DynamicTexture(key.clone()),
    );
    let frame = prepared_frame_for_invocations(
        vec![PreparedViewFrame::offscreen_product(
            "viewport.1",
            (320, 180),
        )],
        vec![invocation(
            "viewport.1.scene",
            compiled.flow_id,
            "viewport.1",
            bindings,
        )],
        vec![descriptor],
    );

    let report =
        validate_prepared_render_frame(&frame, &[compiled], &current_runtime_gpu_capabilities());

    assert!(report.diagnostics.iter().any(|diagnostic| {
        diagnostic.kind == RenderExecutionGraphDiagnosticKind::DynamicTargetUsageMismatch
            && diagnostic.dynamic_target_key.as_ref() == Some(&key)
            && diagnostic
                .alias_binding_key
                .as_ref()
                .is_some_and(|key| key.as_str() == "scene_color")
    }));
}

#[test]
fn render_dynamic_targets_preflight_reports_typed_history_signature_conflicts() {
    let flow = RenderFlow::new("preflight.history")
        .with_color_target_alias("scene_color")
        .expect("render flow authoring should succeed")
        .fullscreen_pass("draw_scene")
        .offscreen_products_only()
        .write_target_alias("scene_color")
        .finish()
        .validate()
        .expect("flow should validate");
    let compiled = compile_flow_plan(&flow).expect("flow should compile");
    let key = RenderDynamicTextureTargetKey::new("preflight", "shared-history");
    let descriptor = RenderDynamicTextureTargetDescriptor::color_sampled(
        key.clone(),
        320,
        180,
        RenderTextureTargetFormat::Rgba8Unorm,
        RenderTextureSampleMode::FilterableFloat,
        RenderDynamicTextureRetention::RetainWhileRequested,
    );
    let binding = PreparedTargetBinding::DynamicTexture(key.clone());
    let mut first = invocation(
        "viewport.1.a",
        compiled.flow_id,
        "viewport.1",
        [(alias_key("scene_color"), binding.clone())]
            .into_iter()
            .collect(),
    );
    first.history_signature = Some("camera:a".to_string());
    let mut second = invocation(
        "viewport.1.b",
        compiled.flow_id,
        "viewport.1",
        [(alias_key("scene_color"), binding)].into_iter().collect(),
    );
    second.history_signature = Some("camera:b".to_string());
    let frame = prepared_frame_for_invocations(
        vec![PreparedViewFrame::offscreen_product(
            "viewport.1",
            (320, 180),
        )],
        vec![first, second],
        vec![descriptor],
    );

    let report =
        validate_prepared_render_frame(&frame, &[compiled], &current_runtime_gpu_capabilities());

    assert!(report.diagnostics.iter().any(|diagnostic| {
        diagnostic.kind == RenderExecutionGraphDiagnosticKind::HistorySignatureConflict
            && diagnostic.dynamic_target_key.as_ref() == Some(&key)
    }));
}

#[test]
fn render_dynamic_targets_preflight_cache_key_ignores_frame_epoch_and_raw_uniform_values() {
    let flow = RenderFlow::new("preflight.cache.stable")
        .with_color_target_alias("scene_color")
        .expect("render flow authoring should succeed")
        .fullscreen_pass("draw_scene")
        .offscreen_products_only()
        .write_target_alias("scene_color")
        .finish()
        .validate()
        .expect("flow should validate");
    let compiled = compile_flow_plan(&flow).expect("flow should compile");
    let key = RenderDynamicTextureTargetKey::new("preflight", "cache-stable");
    let descriptor = RenderDynamicTextureTargetDescriptor::color_sampled(
        key.clone(),
        320,
        180,
        RenderTextureTargetFormat::Rgba8Unorm,
        RenderTextureSampleMode::FilterableFloat,
        RenderDynamicTextureRetention::RetainWhileRequested,
    );
    let mut bindings = BTreeMap::new();
    bindings.insert(
        alias_key("scene_color"),
        PreparedTargetBinding::DynamicTexture(key),
    );
    let mut invocation = invocation("viewport.cache", compiled.flow_id, "viewport.1", bindings);
    let uniform_id = test_resource_ids(1)[0];
    invocation
        .inputs
        .projected_uniform_bytes
        .insert(uniform_id, vec![1, 2, 3, 4]);
    let mut frame = prepared_frame_for_invocations(
        vec![PreparedViewFrame::offscreen_product(
            "viewport.1",
            (320, 180),
        )],
        vec![invocation],
        vec![descriptor],
    );
    let baseline = prepared_render_frame_preflight_cache_key(
        &frame,
        std::slice::from_ref(&compiled),
        &current_runtime_gpu_capabilities(),
    );

    frame.context.frame_index += 1;
    frame.context.prepare_epoch += 1;
    frame.flow_invocations[0]
        .inputs
        .projected_uniform_bytes
        .get_mut(&uniform_id)
        .expect("uniform should exist")
        .copy_from_slice(&[9, 8, 7, 6]);
    let changed_values = prepared_render_frame_preflight_cache_key(
        &frame,
        std::slice::from_ref(&compiled),
        &current_runtime_gpu_capabilities(),
    );

    assert_eq!(baseline, changed_values);
}

#[test]
fn render_dynamic_targets_preflight_cache_key_invalidates_structural_render_inputs() {
    let flow = RenderFlow::new("preflight.cache.invalidates")
        .with_color_target_alias("scene_color")
        .expect("render flow authoring should succeed")
        .fullscreen_pass("draw_scene")
        .offscreen_products_only()
        .write_target_alias("scene_color")
        .finish()
        .validate()
        .expect("flow should validate");
    let compiled = compile_flow_plan(&flow).expect("flow should compile");
    let key = RenderDynamicTextureTargetKey::new("preflight", "cache-invalidates");
    let descriptor = RenderDynamicTextureTargetDescriptor::color_sampled(
        key.clone(),
        320,
        180,
        RenderTextureTargetFormat::Rgba8Unorm,
        RenderTextureSampleMode::FilterableFloat,
        RenderDynamicTextureRetention::RetainWhileRequested,
    );
    let mut bindings = BTreeMap::new();
    bindings.insert(
        alias_key("scene_color"),
        PreparedTargetBinding::DynamicTexture(key.clone()),
    );
    let mut invocation = invocation(
        "viewport.cache.invalidate",
        compiled.flow_id,
        "viewport.1",
        bindings,
    );
    invocation.history_signature = Some("history:a".to_string());
    let uniform_id = test_resource_ids(1)[0];
    invocation
        .inputs
        .projected_uniform_bytes
        .insert(uniform_id, vec![1, 2, 3, 4]);
    let frame = prepared_frame_for_invocations(
        vec![PreparedViewFrame::offscreen_product(
            "viewport.1",
            (320, 180),
        )],
        vec![invocation],
        vec![descriptor],
    );
    let baseline = prepared_render_frame_preflight_cache_key(
        &frame,
        std::slice::from_ref(&compiled),
        &current_runtime_gpu_capabilities(),
    );

    let mut alias_changed = frame.clone();
    alias_changed.flow_invocations[0]
        .target_alias_bindings
        .insert(
            alias_key("scene_color"),
            PreparedTargetBinding::DynamicTexture(RenderDynamicTextureTargetKey::new(
                "preflight",
                "cache-other",
            )),
        );
    let mut descriptor_changed = frame.clone();
    descriptor_changed.dynamic_texture_targets[0].width = 640;
    let mut history_changed = frame.clone();
    history_changed.flow_invocations[0].history_signature = Some("history:b".to_string());
    let mut uniform_shape_changed = frame.clone();
    uniform_shape_changed.flow_invocations[0]
        .inputs
        .projected_uniform_bytes
        .get_mut(&uniform_id)
        .expect("uniform should exist")
        .push(5);

    for changed in [
        alias_changed,
        descriptor_changed,
        history_changed,
        uniform_shape_changed,
    ] {
        let changed_key = prepared_render_frame_preflight_cache_key(
            &changed,
            std::slice::from_ref(&compiled),
            &current_runtime_gpu_capabilities(),
        );
        assert_ne!(baseline, changed_key);
    }
}

#[test]
fn render_dynamic_targets_request_registry_snapshots_valid_requests_by_key() {
    let mut registry = RenderDynamicTextureTargetRequestRegistryResource::default();
    registry
        .replace_contribution(
            producer(1),
            [
                dynamic_descriptor(
                    "b",
                    64,
                    64,
                    RenderTextureTargetFormat::Rgba8Unorm,
                    RenderTextureTargetUsage::color_sampled(),
                    RenderTextureSampleMode::FilterableFloat,
                ),
                dynamic_descriptor(
                    "a",
                    128,
                    64,
                    RenderTextureTargetFormat::Rgba8UnormSrgb,
                    RenderTextureTargetUsage::color_sampled(),
                    RenderTextureSampleMode::FilterableFloat,
                ),
            ],
        )
        .unwrap();

    let labels = registry
        .snapshot()
        .iter()
        .map(|descriptor| descriptor.key.label())
        .collect::<Vec<_>>();
    assert_eq!(labels, vec!["test.dynamic:a", "test.dynamic:b"]);

    let invalid = RenderDynamicTextureTargetDescriptor::new(
        RenderDynamicTextureTargetKey::new("", "bad"),
        64,
        64,
        RenderTextureTargetFormat::Rgba8Unorm,
        RenderTextureTargetUsage::color_sampled(),
        RenderTextureSampleMode::FilterableFloat,
        RenderDynamicTextureRetention::RetainWhileRequested,
    );
    assert!(
        registry
            .replace_contribution(producer(2), [invalid])
            .is_err()
    );
    assert_eq!(registry.diagnostics().len(), 1);
    assert_eq!(registry.snapshot().len(), 2);
}

#[test]
fn render_dynamic_targets_request_registry_rejects_cross_producer_key_collisions() {
    let mut registry = RenderDynamicTextureTargetRequestRegistryResource::default();
    registry
        .replace_contribution(
            producer(1),
            [dynamic_descriptor(
                "shared",
                64,
                64,
                RenderTextureTargetFormat::Rgba8Unorm,
                RenderTextureTargetUsage::color_sampled(),
                RenderTextureSampleMode::FilterableFloat,
            )],
        )
        .unwrap();

    let err = registry
        .replace_contribution(
            producer(2),
            [dynamic_descriptor(
                "shared",
                64,
                64,
                RenderTextureTargetFormat::Rgba8Unorm,
                RenderTextureTargetUsage::color_sampled(),
                RenderTextureSampleMode::FilterableFloat,
            )],
        )
        .expect_err("dynamic target keys must have one owning producer");

    assert!(
        err.to_string().contains("already owned"),
        "unexpected error: {err}"
    );
    assert_eq!(registry.snapshot().len(), 1);
}

#[test]
fn render_dynamic_targets_prepared_frame_requests_carry_offscreen_view_and_target_alias_data() {
    let flow_id = RenderFlowId::try_from_raw(7).unwrap();
    let resource_ids = test_resource_ids(2);
    let uniform_id = resource_ids[0];
    let flow_owned_id = resource_ids[1];
    let dynamic_key = RenderDynamicTextureTargetKey::new("test.dynamic", "viewport.7.scene");
    let descriptor = RenderDynamicTextureTargetDescriptor::color_sampled(
        dynamic_key.clone(),
        320,
        180,
        RenderTextureTargetFormat::Rgba8Unorm,
        RenderTextureSampleMode::FilterableFloat,
        RenderDynamicTextureRetention::RetainForFrames(2),
    );
    let product_surface_request = RenderProductSurfaceRequest::new(
        PreparedViewFrame::offscreen_product("viewport.7", (320, 180))
            .with_history_signature("viewport.7:view"),
        PreparedFlowInvocationRequest::new("viewport.7.editor", flow_id, "viewport.7")
            .bind_dynamic_texture_alias("scene_color", dynamic_key.clone())
            .expect("test alias key should be valid")
            .bind_surface_color_alias("surface_color")
            .expect("test alias key should be valid")
            .bind_flow_owned_alias("owned_color", flow_owned_id)
            .expect("test alias key should be valid")
            .with_uniform_override(uniform_id, vec![1, 2, 3, 4])
            .with_history_signature("viewport.7:v1"),
    )
    .with_dynamic_target(descriptor.clone());
    let batch = RenderProductSurfaceRequestBatch::from_request(product_surface_request);

    let mut requests = PreparedRenderFrameRequestResource::default();
    requests
        .replace_contribution(
            producer(1),
            batch.views().iter().cloned(),
            batch.flow_invocations().iter().cloned(),
        )
        .unwrap();

    assert_eq!(batch.dynamic_targets(), &[descriptor]);
    let views = requests.requested_views();
    assert_eq!(views.len(), 1);
    assert_eq!(views[0].kind, PreparedViewKind::OffscreenProduct);
    assert_eq!(views[0].target_size_px, (320, 180));
    assert_eq!(
        views[0].history_signature.as_deref(),
        Some("viewport.7:view")
    );

    let invocations = requests.requested_flow_invocations();
    assert_eq!(invocations.len(), 1);
    assert_eq!(invocations[0].flow_id, flow_id);
    assert_eq!(invocations[0].view_id, "viewport.7");
    assert_eq!(
        invocations[0]
            .target_alias_bindings
            .get(&alias_key("scene_color")),
        Some(&PreparedTargetBinding::DynamicTexture(dynamic_key))
    );
    assert_eq!(
        invocations[0].history_signature.as_deref(),
        Some("viewport.7:v1")
    );
    assert_eq!(
        invocations[0]
            .target_alias_bindings
            .get(&alias_key("surface_color")),
        Some(&PreparedTargetBinding::SurfaceColor)
    );
    assert_eq!(
        invocations[0]
            .target_alias_bindings
            .get(&alias_key("owned_color")),
        Some(&PreparedTargetBinding::FlowOwned(flow_owned_id))
    );
    assert_eq!(
        invocations[0].uniform_overrides.get(&uniform_id),
        Some(&vec![1, 2, 3, 4])
    );
}

#[test]
fn render_dynamic_targets_prepared_frame_requests_report_typed_duplicate_diagnostics() {
    let flow_id = RenderFlowId::try_from_raw(7).unwrap();
    let request = |invocation_id: &str, view_id: &str| {
        PreparedFlowInvocationRequest::new(invocation_id, flow_id, view_id)
    };
    let mut requests = PreparedRenderFrameRequestResource::default();

    let err = requests
        .replace_contribution(
            producer(1),
            [
                PreparedViewFrame::offscreen_product("same.view", (320, 180)),
                PreparedViewFrame::offscreen_product("same.view", (320, 180)),
            ],
            [],
        )
        .expect_err("duplicate views inside one producer should be typed");
    assert!(matches!(
        err,
        PreparedRenderFrameRequestError::DuplicateViewWithinProducer { .. }
    ));
    assert_eq!(requests.diagnostics().len(), 1);
    assert_eq!(
        requests.diagnostics()[0].request_kind,
        PreparedRenderFrameRequestKind::View
    );
    assert_eq!(requests.diagnostics()[0].producer_id, producer(1));
    assert_eq!(requests.diagnostics()[0].existing_producer_id, None);
    assert_eq!(
        requests.diagnostics()[0].view_id.as_deref(),
        Some("same.view")
    );

    requests.clear();
    let err = requests
        .replace_contribution(
            producer(1),
            [PreparedViewFrame::offscreen_product(
                "viewport.1",
                (320, 180),
            )],
            [
                request("shared.invocation", "viewport.1"),
                request("shared.invocation", "viewport.1"),
            ],
        )
        .expect_err("duplicate invocations inside one producer should be typed");
    assert!(matches!(
        err,
        PreparedRenderFrameRequestError::DuplicateInvocationWithinProducer { .. }
    ));
    assert_eq!(
        requests.diagnostics()[0].request_kind,
        PreparedRenderFrameRequestKind::Invocation
    );
    assert_eq!(
        requests.diagnostics()[0]
            .invocation_id
            .as_ref()
            .map(|id| id.0.as_str()),
        Some("shared.invocation")
    );

    requests.clear();
    requests
        .replace_contribution(
            producer(1),
            [PreparedViewFrame::offscreen_product(
                "viewport.1",
                (320, 180),
            )],
            [request("producer.1.invocation", "viewport.1")],
        )
        .unwrap();

    let err = requests
        .replace_contribution(
            producer(2),
            [PreparedViewFrame::offscreen_product(
                "viewport.1",
                (320, 180),
            )],
            [request("producer.2.invocation", "viewport.1")],
        )
        .expect_err("view ids must be globally unique inside a frame");
    assert!(matches!(
        err,
        PreparedRenderFrameRequestError::DuplicateViewAcrossProducers { .. }
    ));
    assert_eq!(
        requests.diagnostics()[0].existing_producer_id,
        Some(producer(1))
    );
    assert_eq!(requests.requested_views().len(), 1);

    requests.clear();
    requests
        .replace_contribution(
            producer(1),
            [PreparedViewFrame::offscreen_product(
                "viewport.1",
                (320, 180),
            )],
            [request("shared.invocation", "viewport.1")],
        )
        .unwrap();
    let err = requests
        .replace_contribution(
            producer(2),
            [PreparedViewFrame::offscreen_product(
                "viewport.2",
                (320, 180),
            )],
            [request("shared.invocation", "viewport.2")],
        )
        .expect_err("invocation ids must be globally unique inside a frame");

    assert!(matches!(
        err,
        PreparedRenderFrameRequestError::DuplicateInvocationAcrossProducers { .. }
    ));
    assert_eq!(requests.diagnostics()[0].producer_id, producer(2));
    assert_eq!(
        requests.diagnostics()[0].existing_producer_id,
        Some(producer(1))
    );
    assert_eq!(
        requests.diagnostics()[0].request_kind,
        PreparedRenderFrameRequestKind::Invocation
    );
    assert_eq!(
        requests.diagnostics()[0]
            .invocation_id
            .as_ref()
            .map(|id| id.0.as_str()),
        Some("shared.invocation")
    );
}
