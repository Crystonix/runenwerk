use engine::plugins::render::api::RenderPassId;
use engine::plugins::render::{
    CompiledPassExecutionPlan, CompiledResourceRef, GpuParams, GpuStorage, GpuUniform, RenderFlow,
    RenderGpuParamsLayout, RenderGpuResourceAdapterError, RenderGpuResourceLowering,
    RenderImportedBufferSemantic, RenderImportedTextureSemantic, RenderResourceDeclaration,
    RenderTargetAliasKind, compile_flow_plan, detect_duplicate_resource_ids,
};
use runen_gpu::{
    GpuBufferInitialization, GpuResourceDescriptor, GpuResourceLifetime, GpuTextureFormat,
    GpuWorkResourceId, GpuWorkResourceIdAllocator,
};

#[derive(Debug, Clone, Copy, GpuUniform)]
struct ResourceTestParams {
    value: u32,
}

#[derive(Debug, Clone, Copy, GpuUniform)]
struct AlternateResourceTestParams {
    value: u32,
}

#[derive(Debug, Clone, Copy, GpuStorage)]
struct StorageResourceTestParams {
    value: u32,
}

fn test_resource_ids(count: usize) -> Vec<GpuWorkResourceId> {
    let labels = (0..count)
        .map(|index| format!("test.resource.{index}"))
        .collect::<Vec<_>>();
    let flow = labels
        .iter()
        .fold(RenderFlow::new("test.resource.ids"), |flow, label| {
            flow.with_color_target(label.clone())
                .expect("render flow authoring should succeed")
        });
    labels
        .iter()
        .map(|label| flow.resource_id(label).expect("test resource should exist"))
        .collect()
}

#[test]
fn descriptor_construction_tracks_resource_kind_and_type_metadata() {
    let mut allocator = GpuWorkResourceIdAllocator::new();
    let descriptor = RenderResourceDeclaration::declare_uniform::<ResourceTestParams>(
        &mut allocator,
        "test uniform",
    )
    .expect("uniform declaration should be valid");
    let id = *descriptor.id();

    match descriptor {
        RenderResourceDeclaration::Uniform(value) => {
            assert_eq!(*value.id(), id);
            assert_eq!(
                value.params_type_id(),
                std::any::TypeId::of::<ResourceTestParams>()
            );
            assert!(value.params_type_name().contains("ResourceTestParams"));
            assert!(value.size_bytes() > 0);
            let raw = ResourceTestParams { value: 9 }.to_gpu();
            assert_eq!(raw.bytes.len() as u64, value.size_bytes());
            assert_eq!(u32::from_le_bytes(raw.bytes[0..4].try_into().unwrap()), 9);
        }
        other => panic!("unexpected descriptor variant: {other:?}"),
    }
}

#[test]
fn transient_storage_declarations_do_not_claim_descriptor_initialization() {
    let mut allocator = GpuWorkResourceIdAllocator::new();
    let declaration = RenderResourceDeclaration::declare_storage_array_with_lifetime::<
        StorageResourceTestParams,
    >(
        &mut allocator,
        "transient storage",
        4,
        GpuResourceLifetime::Transient,
    )
    .expect("transient storage declaration should be valid");

    let RenderResourceDeclaration::Storage(storage) = declaration else {
        panic!("expected storage declaration");
    };
    assert_eq!(
        storage.handle().descriptor().common().lifetime(),
        GpuResourceLifetime::Transient
    );
    assert_eq!(
        storage.handle().descriptor().initialization(),
        &GpuBufferInitialization::Uninitialized
    );
}

#[test]
fn typed_ids_preserve_pass_raw_value_and_sort_resources_by_owner_local_identity() {
    let pass = RenderPassId::try_from_raw(7).unwrap();
    let raw: u64 = pass.into();
    assert_eq!(raw, 7);

    let ids = test_resource_ids(2);
    let a = ids[0];
    let b = ids[1];
    assert!(a < b);
    let (owner, local) = a.diagnostic_parts();
    assert_ne!(owner, 0);
    assert_eq!(local, 1);
    assert_eq!(a.to_string(), format!("{owner}:1"));
}

#[test]
fn duplicate_resource_detection_finds_collisions() {
    let ids = test_resource_ids(2);
    let duplicate = ids[0];
    let descriptors = vec![
        RenderResourceDeclaration::declare_sampled_texture(duplicate, "sampled"),
        RenderResourceDeclaration::declare_color_attachment(ids[1], "color"),
        RenderResourceDeclaration::declare_imported_external_texture(duplicate, "external"),
    ];

    let duplicates = detect_duplicate_resource_ids(&descriptors);
    assert_eq!(duplicates, vec![duplicate]);
}

#[test]
fn owned_buffer_lowering_returns_normalized_buffer() {
    let mut allocator = GpuWorkResourceIdAllocator::new();
    let declaration = RenderResourceDeclaration::declare_uniform::<ResourceTestParams>(
        &mut allocator,
        "owned buffer",
    )
    .unwrap();

    let lowering = declaration
        .lower_gpu_resource((64, 64), GpuTextureFormat::Rgba8Unorm)
        .unwrap();
    let RenderGpuResourceLowering::Normalized(normalized) = lowering else {
        panic!("owned buffer should have normalized descriptor facts");
    };

    assert!(matches!(
        normalized.as_ref(),
        GpuResourceDescriptor::Buffer(buffer) if buffer.size_bytes() > 0
    ));
}

#[test]
fn owned_texture_lowering_returns_normalized_texture() {
    let id = test_resource_ids(1)[0];
    let declaration = RenderResourceDeclaration::declare_color_attachment(id, "owned texture");

    let lowering = declaration
        .lower_gpu_resource((320, 180), GpuTextureFormat::Bgra8UnormSrgb)
        .unwrap();
    let RenderGpuResourceLowering::Normalized(normalized) = lowering else {
        panic!("owned texture should have normalized descriptor facts");
    };

    assert!(matches!(
        normalized.as_ref(),
        GpuResourceDescriptor::Texture(texture)
            if texture.extent().width() == 320
                && texture.extent().height() == 180
                && texture.format() == GpuTextureFormat::Bgra8UnormSrgb
    ));
}

#[test]
fn imported_texture_lowering_preserves_unresolved_render_intent() {
    let id = test_resource_ids(1)[0];
    let declaration =
        RenderResourceDeclaration::declare_imported_external_texture(id, "external texture");

    let lowering = declaration
        .lower_gpu_resource((320, 180), GpuTextureFormat::Rgba8Unorm)
        .unwrap();

    assert!(matches!(
        lowering,
        RenderGpuResourceLowering::ImportedTexture(intent)
            if intent.id == id
                && intent.label == "external texture"
                && intent.semantic == RenderImportedTextureSemantic::External
    ));
}

#[test]
fn imported_buffer_lowering_preserves_unresolved_render_intent() {
    let id = test_resource_ids(1)[0];
    let declaration =
        RenderResourceDeclaration::declare_imported_history_buffer(id, "history buffer");

    let lowering = declaration
        .lower_gpu_resource((320, 180), GpuTextureFormat::Rgba8Unorm)
        .unwrap();

    assert!(matches!(
        lowering,
        RenderGpuResourceLowering::ImportedBuffer(intent)
            if intent.id == id
                && intent.label == "history buffer"
                && intent.semantic == RenderImportedBufferSemantic::HistoryBuffer
    ));
}

#[test]
fn target_alias_lowering_remains_a_render_relationship() {
    let id = test_resource_ids(1)[0];
    let declaration = RenderResourceDeclaration::declare_target_alias(
        id,
        "surface alias",
        RenderTargetAliasKind::Color,
    )
    .expect("target alias declaration should be valid");

    let lowering = declaration
        .lower_gpu_resource((320, 180), GpuTextureFormat::Rgba8Unorm)
        .unwrap();

    assert!(matches!(
        lowering,
        RenderGpuResourceLowering::TargetAlias(alias)
            if alias.id() == id
                && alias.binding_key().as_str() == "surface alias"
                && alias.kind() == RenderTargetAliasKind::Color
    ));
}

#[test]
fn unresolved_import_lowering_rejects_an_empty_diagnostic_label() {
    let id = test_resource_ids(1)[0];
    let declaration = RenderResourceDeclaration::declare_imported_external_buffer(id, "   ");

    assert!(
        declaration
            .lower_gpu_resource((1, 1), GpuTextureFormat::Rgba8Unorm)
            .is_err()
    );
}

#[test]
fn diagnostic_texture_labels_do_not_change_texture_intent_semantics() {
    let ids = test_resource_ids(3);
    let first = RenderResourceDeclaration::declare_color_attachment(ids[0], "first texture label");
    let second =
        RenderResourceDeclaration::declare_color_attachment(ids[0], "second texture label");
    let (
        RenderResourceDeclaration::ColorAttachment(first),
        RenderResourceDeclaration::ColorAttachment(second),
    ) = (first, second)
    else {
        panic!("expected color attachment declarations");
    };

    assert_eq!(first, second);
    assert_ne!(first.label(), second.label());
}

#[test]
fn imported_display_labels_do_not_change_import_intent_semantics() {
    let id = test_resource_ids(1)[0];
    let first =
        RenderResourceDeclaration::declare_imported_external_texture(id, "first import label");
    let second =
        RenderResourceDeclaration::declare_imported_external_texture(id, "second import label");
    let (
        RenderResourceDeclaration::ImportedTexture(first),
        RenderResourceDeclaration::ImportedTexture(second),
    ) = (first, second)
    else {
        panic!("expected imported texture declarations");
    };

    assert_eq!(first, second);
    assert_ne!(first.label, second.label);
}

#[test]
fn target_alias_binding_keys_participate_in_semantic_equality() {
    let id = test_resource_ids(1)[0];
    let first = RenderResourceDeclaration::declare_target_alias(
        id,
        "first.alias",
        RenderTargetAliasKind::Color,
    )
    .unwrap();
    let second = RenderResourceDeclaration::declare_target_alias(
        id,
        "second.alias",
        RenderTargetAliasKind::Color,
    )
    .unwrap();
    let (
        RenderResourceDeclaration::TargetAlias(first),
        RenderResourceDeclaration::TargetAlias(second),
    ) = (first, second)
    else {
        panic!("expected target alias declarations");
    };

    assert_ne!(first, second);
}

#[test]
fn target_alias_binding_keys_reject_empty_values_and_normalize_whitespace() {
    let id = test_resource_ids(1)[0];
    let error =
        RenderResourceDeclaration::declare_target_alias(id, "   ", RenderTargetAliasKind::Color)
            .expect_err("empty alias binding keys must be rejected");
    assert!(matches!(
        error,
        RenderGpuResourceAdapterError::InvalidTargetAliasBindingKey {
            value,
            correction: "provide at least one non-whitespace character",
        } if value == "   "
    ));

    let declaration = RenderResourceDeclaration::declare_target_alias(
        id,
        "  viewport.scene_color  ",
        RenderTargetAliasKind::Color,
    )
    .expect("non-empty alias binding key should be valid");
    let RenderResourceDeclaration::TargetAlias(alias) = declaration else {
        panic!("expected target alias declaration");
    };
    assert_eq!(alias.binding_key().as_str(), "viewport.scene_color");
}

#[test]
fn compiled_target_alias_refs_preserve_normalized_binding_keys() {
    let flow = RenderFlow::new("compiled.alias.key")
        .with_color_target_alias("  scene_color  ")
        .expect("render flow authoring should succeed")
        .fullscreen_pass("compose")
        .write_target_alias("scene_color")
        .finish()
        .validate()
        .expect("flow should validate");
    let compiled = compile_flow_plan(&flow).expect("flow should compile");
    let Some(CompiledPassExecutionPlan::Fullscreen(pass)) = compiled.execution.passes.first()
    else {
        panic!("expected compiled fullscreen pass");
    };
    let Some(CompiledResourceRef::TargetAlias(alias)) = pass.targets.color_outputs.first() else {
        panic!("expected compiled target alias output");
    };

    assert_eq!(alias.binding_key.as_str(), "scene_color");
}

#[test]
fn render_parameter_layouts_expose_allocation_and_declared_type_checks_separately() {
    let first = RenderGpuParamsLayout::uniform::<ResourceTestParams>("first").unwrap();
    let same_type = RenderGpuParamsLayout::uniform::<ResourceTestParams>("same type").unwrap();
    let other_type =
        RenderGpuParamsLayout::uniform::<AlternateResourceTestParams>("other type").unwrap();

    assert!(first.is_allocation_compatible_with(&other_type));
    assert!(!first.declares_same_params_type_as(&other_type));
    assert!(first.declares_same_params_type_as(&same_type));
}
