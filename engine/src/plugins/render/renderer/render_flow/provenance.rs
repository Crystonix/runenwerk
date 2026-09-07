use super::*;
use crate::plugins::render::{RenderFlowId, RenderPassId};

#[derive(Debug, Clone)]
pub struct EncodedPassEvidence {
    pub shader_id: String,
    pub shader_revision: u64,
    pub fallback_used: bool,
    pub pipeline_key: Option<FlowPassPipelineKey>,
}

#[derive(Debug, Default)]
pub struct PassResourceTruth {
    pub render_targets: Vec<String>,
    pub sampled_textures: Vec<String>,
    pub storage_textures: Vec<String>,
    pub depth_targets: Vec<String>,
    pub capture_points_available: Vec<RenderCapturePointIdentity>,
}

pub fn collect_pass_resource_truth(
    flow_id: RenderFlowId,
    pass: &CompiledPassExecutionPlan,
    runtime_resources: &FlowRuntimeResources,
) -> PassResourceTruth {
    let pass_id = execution_pass_id(pass);
    let pass_label = pass_id.to_string();
    let mut render_targets = Vec::<String>::new();
    let mut sampled_textures = Vec::<String>::new();
    let mut storage_textures = Vec::<String>::new();
    let mut depth_targets = Vec::<String>::new();
    let mut render_target_seen = BTreeSet::<String>::new();
    let mut sampled_seen = BTreeSet::<String>::new();
    let mut storage_seen = BTreeSet::<String>::new();
    let mut depth_seen = BTreeSet::<String>::new();

    match pass {
        CompiledPassExecutionPlan::Compute(plan) => {
            for entry in &plan.bindings.bind_group.entries {
                match entry {
                    CompiledBindingEntry::SampledTexture { resource, .. } => {
                        push_resolved_resource_id(
                            pass_id,
                            resource,
                            "sampled_texture",
                            runtime_resources,
                            &mut sampled_seen,
                            &mut sampled_textures,
                        );
                    }
                    CompiledBindingEntry::StorageTexture { resource, .. } => {
                        push_resolved_resource_id(
                            pass_id,
                            resource,
                            "storage_texture",
                            runtime_resources,
                            &mut storage_seen,
                            &mut storage_textures,
                        );
                    }
                    _ => {}
                }
            }
        }
        CompiledPassExecutionPlan::Fullscreen(plan) | CompiledPassExecutionPlan::Graphics(plan) => {
            for target in &plan.targets.color_outputs {
                push_resolved_resource_id(
                    pass_id,
                    target,
                    "color_target",
                    runtime_resources,
                    &mut render_target_seen,
                    &mut render_targets,
                );
            }
            if let Some(depth_output) = plan.targets.depth_output.as_ref() {
                push_resolved_resource_id(
                    pass_id,
                    depth_output,
                    "depth_target",
                    runtime_resources,
                    &mut depth_seen,
                    &mut depth_targets,
                );
            }
            for entry in &plan.bindings.bind_group.entries {
                match entry {
                    CompiledBindingEntry::SampledTexture { resource, .. } => {
                        push_resolved_resource_id(
                            pass_id,
                            resource,
                            "sampled_texture",
                            runtime_resources,
                            &mut sampled_seen,
                            &mut sampled_textures,
                        );
                    }
                    CompiledBindingEntry::StorageTexture { resource, .. } => {
                        push_resolved_resource_id(
                            pass_id,
                            resource,
                            "storage_texture",
                            runtime_resources,
                            &mut storage_seen,
                            &mut storage_textures,
                        );
                    }
                    _ => {}
                }
            }
        }
        CompiledPassExecutionPlan::Copy(plan) => {
            if let Some(source) = plan.source.as_ref() {
                push_resolved_resource_id(
                    pass_id,
                    source,
                    "copy_source",
                    runtime_resources,
                    &mut render_target_seen,
                    &mut render_targets,
                );
            }
            if let Some(destination) = plan.destination.as_ref() {
                push_resolved_resource_id(
                    pass_id,
                    destination,
                    "copy_destination",
                    runtime_resources,
                    &mut render_target_seen,
                    &mut render_targets,
                );
            }
        }
        CompiledPassExecutionPlan::Present(plan) => {
            if let Some(source) = plan.source.as_ref() {
                push_resolved_resource_id(
                    pass_id,
                    source,
                    "present_source",
                    runtime_resources,
                    &mut render_target_seen,
                    &mut render_targets,
                );
            }
        }
        CompiledPassExecutionPlan::BuiltinUiComposite(plan) => {
            push_resolved_resource_id(
                pass_id,
                &plan.color_output,
                "ui_composite_color_output",
                runtime_resources,
                &mut render_target_seen,
                &mut render_targets,
            );
        }
    }

    let mut capture_points_available = Vec::<RenderCapturePointIdentity>::new();
    let mut capture_point_seen = BTreeSet::<RenderCapturePointIdentity>::new();
    for resource_id in render_targets
        .iter()
        .chain(sampled_textures.iter())
        .chain(storage_textures.iter())
        .chain(depth_targets.iter())
    {
        let texture_class = runtime_resources
            .capture_texture_class(resource_id.as_str(), CaptureTextureClass::ColorTarget);
        for stage in [CaptureStage::Before, CaptureStage::After] {
            let identity = RenderCapturePointIdentity {
                flow_id: flow_id.to_string(),
                pass_id: pass_label.clone(),
                stage,
                resource_id: resource_id.clone(),
                texture_class,
            };
            if capture_point_seen.insert(identity.clone()) {
                capture_points_available.push(identity);
            }
        }
    }

    PassResourceTruth {
        render_targets,
        sampled_textures,
        storage_textures,
        depth_targets,
        capture_points_available,
    }
}

pub fn collect_pass_material_binding_evidence(
    packet: &RendererPreparedPacket,
    pass: &CompiledPassExecutionPlan,
) -> RenderPassMaterialBindingEvidence {
    let feature_id = execution_pass_feature_id(pass);
    let shader_reference = execution_pass_shader_reference(pass);
    let consumes_material_resources =
        pass_consumes_material_resources(feature_id, shader_reference);
    let Some(material) = packet.prepared_material.as_ref() else {
        return RenderPassMaterialBindingEvidence {
            consumes_material_resources,
            ..RenderPassMaterialBindingEvidence::default()
        };
    };
    let model_mesh_material_selections_available_to_pass = if consumes_material_resources {
        material
            .model_mesh_material_selections
            .iter()
            .map(inspect_model_mesh_material_selection)
            .collect()
    } else {
        Vec::new()
    };
    let scene_bundle = material.scene_bundle.as_ref();

    RenderPassMaterialBindingEvidence {
        consumes_material_resources,
        prepared_material_available: true,
        material_table_identity: scene_bundle.map(|bundle| bundle.material_table_identity.clone()),
        scene_shader_identity: scene_bundle.map(|bundle| bundle.shader_identity.clone()),
        scene_shader_path: scene_bundle.map(|bundle| bundle.shader_path.clone()),
        material_instance_count: material.instances.len(),
        material_binding_slot_count: material.binding_table.slots.len(),
        prepared_model_mesh_material_selection_count: material.model_mesh_material_selections.len(),
        model_mesh_material_selections_available_to_pass,
    }
}

fn inspect_model_mesh_material_selection(
    selection: &crate::plugins::render::PreparedModelMeshMaterialSelection,
) -> RenderPassModelMeshMaterialSelectionEvidence {
    RenderPassModelMeshMaterialSelectionEvidence {
        source_asset_id: selection.surface.source.asset_id,
        source_id: selection.surface.source.source_id,
        source_revision_id: selection.surface.source.source_revision_id,
        source_revision: selection.surface.source.source_revision.clone(),
        region_key: selection.surface.region_key.clone(),
        requested_material_slot_id: selection.requested_material_slot_id,
        resolved_material_slot_id: selection.resolved_material_slot_id,
        material_table_index: selection.material_table_index,
        used_default_fallback: selection.used_default_fallback,
    }
}

pub fn push_resolved_resource_id(
    pass_id: RenderPassId,
    target: &CompiledResourceRef,
    role: &str,
    runtime_resources: &FlowRuntimeResources,
    seen: &mut BTreeSet<String>,
    output: &mut Vec<String>,
) {
    if let Ok(resource_id) = runtime_resources.resolve_resource_key(pass_id, target, role) {
        let resource_id = resource_id.to_string();
        if seen.insert(resource_id.clone()) {
            output.push(resource_id);
        }
    }
}

pub fn execution_pass_kind_name(pass: &CompiledPassExecutionPlan) -> &'static str {
    match pass {
        CompiledPassExecutionPlan::Compute(_) => "compute",
        CompiledPassExecutionPlan::Fullscreen(_) => "fullscreen",
        CompiledPassExecutionPlan::Graphics(_) => "graphics",
        CompiledPassExecutionPlan::Copy(_) => "copy",
        CompiledPassExecutionPlan::Present(_) => "present",
        CompiledPassExecutionPlan::BuiltinUiComposite(_) => "builtin_ui_composite",
    }
}

pub fn execution_flow_pass_kind(pass: &CompiledPassExecutionPlan) -> FlowPassKind {
    match pass {
        CompiledPassExecutionPlan::Compute(_) => FlowPassKind::Compute,
        CompiledPassExecutionPlan::Fullscreen(_) => FlowPassKind::Fullscreen,
        CompiledPassExecutionPlan::Graphics(_) => FlowPassKind::Graphics,
        CompiledPassExecutionPlan::Copy(_) => FlowPassKind::Copy,
        CompiledPassExecutionPlan::Present(_) => FlowPassKind::Present,
        CompiledPassExecutionPlan::BuiltinUiComposite(_) => FlowPassKind::BuiltinUiComposite,
    }
}

pub fn execution_pass_id(pass: &CompiledPassExecutionPlan) -> RenderPassId {
    match pass {
        CompiledPassExecutionPlan::Compute(value) => value.pass_id,
        CompiledPassExecutionPlan::Fullscreen(value) => value.pass_id,
        CompiledPassExecutionPlan::Graphics(value) => value.pass_id,
        CompiledPassExecutionPlan::Copy(value) => value.pass_id,
        CompiledPassExecutionPlan::Present(value) => value.pass_id,
        CompiledPassExecutionPlan::BuiltinUiComposite(value) => value.pass_id,
    }
}

pub fn execution_pass_feature_id(pass: &CompiledPassExecutionPlan) -> Option<RenderFeatureId> {
    match pass {
        CompiledPassExecutionPlan::Compute(value) => value.feature_id,
        CompiledPassExecutionPlan::Fullscreen(value)
        | CompiledPassExecutionPlan::Graphics(value) => value.feature_id,
        CompiledPassExecutionPlan::Copy(value) => value.feature_id,
        CompiledPassExecutionPlan::Present(value) => value.feature_id,
        CompiledPassExecutionPlan::BuiltinUiComposite(value) => Some(value.feature_id),
    }
}

pub fn execution_pass_shader_reference(
    pass: &CompiledPassExecutionPlan,
) -> Option<&RenderShaderReference> {
    match pass {
        CompiledPassExecutionPlan::Compute(value) => value.shader.as_ref(),
        CompiledPassExecutionPlan::Fullscreen(value)
        | CompiledPassExecutionPlan::Graphics(value) => value.shader.as_ref(),
        CompiledPassExecutionPlan::Copy(_)
        | CompiledPassExecutionPlan::Present(_)
        | CompiledPassExecutionPlan::BuiltinUiComposite(_) => None,
    }
}

pub fn execution_pass_authoring_index(pass: &CompiledPassExecutionPlan) -> usize {
    match pass {
        CompiledPassExecutionPlan::Compute(value) => value.authoring_index,
        CompiledPassExecutionPlan::Fullscreen(value) => value.authoring_index,
        CompiledPassExecutionPlan::Graphics(value) => value.authoring_index,
        CompiledPassExecutionPlan::Copy(value) => value.authoring_index,
        CompiledPassExecutionPlan::Present(value) => value.authoring_index,
        CompiledPassExecutionPlan::BuiltinUiComposite(value) => value.authoring_index,
    }
}

#[derive(Debug, Clone)]
pub struct ResolvedShaderMaterial<'a> {
    pub source: &'a str,
    pub shader_id: String,
    pub pipeline_identity: String,
    pub revision: u64,
    pub fallback_used: bool,
}

pub fn resolve_shader_material<'a>(
    reference: Option<&RenderShaderReference>,
    shader_registry: &'a ShaderRegistryResource,
    fallback_source: &'a str,
    fallback_identity: &'static str,
) -> ResolvedShaderMaterial<'a> {
    match reference {
        Some(RenderShaderReference::AssetPath(path)) => {
            let revision = shader_registry.revision_for(path);
            ResolvedShaderMaterial {
                source: shader_registry.source_or(path, fallback_source),
                shader_id: path.clone(),
                pipeline_identity: format!("asset:{path}"),
                revision,
                fallback_used: revision == 0,
            }
        }
        Some(RenderShaderReference::MaterialSceneBundle { fallback_asset }) => {
            let revision = shader_registry.revision_for(fallback_asset);
            ResolvedShaderMaterial {
                source: shader_registry.source_or(fallback_asset, fallback_source),
                shader_id: fallback_asset.clone(),
                pipeline_identity: format!("asset:{fallback_asset}"),
                revision,
                fallback_used: revision == 0,
            }
        }
        Some(RenderShaderReference::RegistryHandle(handle)) => {
            let revision = shader_registry.revision_for_handle(*handle);
            ResolvedShaderMaterial {
                source: shader_registry.source_or_handle(*handle, fallback_source),
                shader_id: format!("handle:{}", handle.index()),
                pipeline_identity: format!("handle:{}", handle.index()),
                revision,
                fallback_used: revision == 0,
            }
        }
        None => ResolvedShaderMaterial {
            source: fallback_source,
            shader_id: fallback_identity.to_string(),
            pipeline_identity: fallback_identity.to_string(),
            revision: 0,
            fallback_used: false,
        },
    }
}

pub fn resolve_shader_material_for_packet<'a>(
    reference: Option<&RenderShaderReference>,
    packet: &RendererPreparedPacket,
    shader_registry: &'a ShaderRegistryResource,
    fallback_source: &'a str,
    fallback_identity: &'static str,
) -> ResolvedShaderMaterial<'a> {
    match reference {
        Some(RenderShaderReference::MaterialSceneBundle { fallback_asset }) => {
            if let Some(scene_bundle) = packet
                .prepared_material
                .as_ref()
                .and_then(|material| material.scene_bundle.as_ref())
            {
                let revision = shader_registry.revision_for(scene_bundle.shader_path.as_str());
                return ResolvedShaderMaterial {
                    source: shader_registry
                        .source_or(scene_bundle.shader_path.as_str(), fallback_source),
                    shader_id: scene_bundle.shader_path.clone(),
                    pipeline_identity: format!(
                        "material-scene-bundle:{}:{}:{}",
                        scene_bundle.shader_identity,
                        scene_bundle.material_table_identity,
                        scene_bundle.shader_cache_key
                    ),
                    revision,
                    fallback_used: revision == 0,
                };
            }
            let fallback = resolve_shader_material(
                Some(&RenderShaderReference::AssetPath(fallback_asset.clone())),
                shader_registry,
                fallback_source,
                fallback_identity,
            );
            ResolvedShaderMaterial {
                fallback_used: true,
                ..fallback
            }
        }
        other => {
            resolve_shader_material(other, shader_registry, fallback_source, fallback_identity)
        }
    }
}

pub fn hash_view_signature(view_id: &str, surface_size: (u32, u32)) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    view_id.hash(&mut hasher);
    surface_size.hash(&mut hasher);
    hasher.finish()
}

pub fn material_specialization_fragment_hash(
    packet: &RendererPreparedPacket,
    pass_feature_id: Option<RenderFeatureId>,
) -> u64 {
    if pass_feature_id != Some(crate::plugins::render::features::MATERIAL_RENDER_FEATURE_ID) {
        return 0;
    }

    pass_feature_id
        .and_then(|feature_id| packet.feature_runtime_signatures.get(&feature_id).copied())
        .unwrap_or_default()
}

pub fn feature_runtime_version(
    packet: &RendererPreparedPacket,
    pass_feature_id: Option<RenderFeatureId>,
) -> u64 {
    let Some(feature_id) = pass_feature_id else {
        return 0;
    };

    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    feature_id.hash(&mut hasher);
    if let Some(gate) = packet.feature_gates.get(&feature_id) {
        gate.status.hash(&mut hasher);
        gate.fallback_policy.hash(&mut hasher);
    }
    packet
        .feature_runtime_signatures
        .get(&feature_id)
        .copied()
        .unwrap_or_default()
        .hash(&mut hasher);
    hasher.finish()
}

pub fn pass_consumes_material_resources(
    feature_id: Option<crate::plugins::render::RenderFeatureId>,
    shader: Option<&RenderShaderReference>,
) -> bool {
    feature_id == Some(crate::plugins::render::features::MATERIAL_RENDER_FEATURE_ID)
        || matches!(
            shader,
            Some(RenderShaderReference::MaterialSceneBundle { .. })
        )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugins::render::features::{
        MATERIAL_RENDER_FEATURE_ID, UI_RENDER_FEATURE_ID, WORLD_DRAW_RENDER_FEATURE_ID,
    };
    use crate::plugins::render::{
        CompiledDrawBufferPlan, CompiledPassBindings, CompiledPresentExecutionPlan,
        CompiledRasterExecutionPlan, CompiledTargetPlan, CompiledViewMask,
        PreparedMaterialBindingSlot, PreparedMaterialBindingTable,
        PreparedMaterialFeatureContribution, PreparedMaterialInstanceInput,
        PreparedMaterialParameterPayloadV1, PreparedModelMeshMaterialRegionIdentity,
        PreparedModelMeshMaterialSelection, PreparedModelMeshMaterialSourceIdentity,
        PreparedSceneMaterialBundle, RenderShaderReference,
    };

    fn packet_with_feature_gate(
        feature_id: RenderFeatureId,
        gate: FeatureExecutionGate,
    ) -> RendererPreparedPacket {
        let mut feature_gates = BTreeMap::new();
        feature_gates.insert(feature_id, gate);
        let mut feature_runtime_signatures = BTreeMap::new();
        feature_runtime_signatures.insert(feature_id, 1);
        RendererPreparedPacket {
            surface_format: GpuTextureFormat::Rgba8Unorm,
            surface_size: (1, 1),
            view_id: "main".to_string(),
            feature_gates,
            feature_runtime_signatures,
            prepared_material: None,
            prepared_material_gpu_resources: None,
            prepared_ui: UiPreparedDraws::default(),
            ui_dynamic_bind_groups: UiDynamicBindGroups::default(),
            pending_operations: RendererPendingOperations::default(),
            viewport_surface_bindings: Default::default(),
            prepare_timings: RendererFrameTimings::default(),
        }
    }

    #[test]
    fn ui_feature_gate_skips_when_missing_and_policy_is_skip() {
        let renderer = Renderer::new();
        let packet = packet_with_feature_gate(
            UI_RENDER_FEATURE_ID,
            FeatureExecutionGate {
                status: FeatureContributionStatus::Missing,
                fallback_policy: FeatureFallbackPolicy::SkipFeaturePasses,
            },
        );
        let action = renderer
            .resolve_feature_pass_action(
                UI_RENDER_FEATURE_ID,
                RenderPassId::try_from_raw(1).unwrap(),
                &packet,
            )
            .expect("skip policy should not error");
        assert_eq!(action, FeaturePassAction::Skip);
    }

    #[test]
    fn ui_feature_gate_fails_when_missing_and_policy_is_fail_frame() {
        let renderer = Renderer::new();
        let packet = packet_with_feature_gate(
            UI_RENDER_FEATURE_ID,
            FeatureExecutionGate {
                status: FeatureContributionStatus::Missing,
                fallback_policy: FeatureFallbackPolicy::FailFrame,
            },
        );
        assert!(
            renderer
                .resolve_feature_pass_action(
                    UI_RENDER_FEATURE_ID,
                    RenderPassId::try_from_raw(1).unwrap(),
                    &packet
                )
                .is_err(),
            "missing + fail-frame should produce an execution error"
        );
    }

    #[test]
    fn generic_feature_gate_applies_to_non_ui_passes() {
        let renderer = Renderer::new();
        let packet = packet_with_feature_gate(
            WORLD_DRAW_RENDER_FEATURE_ID,
            FeatureExecutionGate {
                status: FeatureContributionStatus::Missing,
                fallback_policy: FeatureFallbackPolicy::SkipFeaturePasses,
            },
        );

        let action = renderer
            .resolve_feature_pass_action(
                WORLD_DRAW_RENDER_FEATURE_ID,
                RenderPassId::try_from_raw(2).unwrap(),
                &packet,
            )
            .expect("skip policy should not error");
        assert_eq!(action, FeaturePassAction::Skip);
    }

    #[test]
    fn feature_runtime_version_changes_when_runtime_signature_changes() {
        let mut packet = packet_with_feature_gate(
            WORLD_DRAW_RENDER_FEATURE_ID,
            FeatureExecutionGate {
                status: FeatureContributionStatus::Ready,
                fallback_policy: FeatureFallbackPolicy::SkipFeaturePasses,
            },
        );
        let base = feature_runtime_version(&packet, Some(WORLD_DRAW_RENDER_FEATURE_ID));
        packet
            .feature_runtime_signatures
            .insert(WORLD_DRAW_RENDER_FEATURE_ID, 99);
        let changed = feature_runtime_version(&packet, Some(WORLD_DRAW_RENDER_FEATURE_ID));
        assert_ne!(base, changed);
    }

    #[test]
    fn material_specialization_hash_uses_material_feature_signature() {
        let mut packet = packet_with_feature_gate(
            MATERIAL_RENDER_FEATURE_ID,
            FeatureExecutionGate {
                status: FeatureContributionStatus::Ready,
                fallback_policy: FeatureFallbackPolicy::SkipFeaturePasses,
            },
        );
        packet
            .feature_runtime_signatures
            .insert(MATERIAL_RENDER_FEATURE_ID, 1234);
        assert_eq!(
            material_specialization_fragment_hash(&packet, Some(MATERIAL_RENDER_FEATURE_ID)),
            1234
        );
        assert_eq!(
            material_specialization_fragment_hash(&packet, Some(WORLD_DRAW_RENDER_FEATURE_ID)),
            0
        );
    }

    #[test]
    fn material_pass_provenance_exposes_model_mesh_selection_table() {
        let selection = source_backed_model_mesh_selection();
        let mut packet = packet_with_feature_gate(
            MATERIAL_RENDER_FEATURE_ID,
            FeatureExecutionGate {
                status: FeatureContributionStatus::Ready,
                fallback_policy: FeatureFallbackPolicy::FailFrame,
            },
        );
        packet.prepared_material = Some(PreparedMaterialFeatureContribution {
            instances: vec![PreparedMaterialInstanceInput {
                material_instance_id: "material.product.7".to_string(),
                specialization_key_fragment: "shader.fragment.7".to_string(),
                parameter_payload: PreparedMaterialParameterPayloadV1::default(),
                texture_bindings: Vec::new(),
            }],
            binding_table: PreparedMaterialBindingTable::fixed_capacity([
                PreparedMaterialBindingSlot::new(
                    3,
                    "material.product.7",
                    "artifact.material.7",
                    "artifact.shader.7",
                    "material-cache.7",
                    "shader-cache.7",
                ),
            ])
            .expect("material binding table should form"),
            scene_bundle: Some(PreparedSceneMaterialBundle::new(
                "artifact.shader.7",
                "shader-cache.7",
                "generated/wr029-scene.wgsl",
                "scene-shader-identity.7",
                "material-table-identity.7",
            )),
            model_mesh_material_selections: vec![selection],
        });

        let pass = material_scene_fullscreen_pass();
        let evidence = collect_pass_material_binding_evidence(&packet, &pass);

        assert!(evidence.consumes_material_resources);
        assert!(evidence.prepared_material_available);
        assert_eq!(
            evidence.material_table_identity.as_deref(),
            Some("material-table-identity.7")
        );
        assert_eq!(
            evidence.scene_shader_identity.as_deref(),
            Some("scene-shader-identity.7")
        );
        assert_eq!(evidence.material_instance_count, 1);
        assert_eq!(evidence.material_binding_slot_count, 1);
        assert_eq!(evidence.prepared_model_mesh_material_selection_count, 1);

        let selected = evidence
            .model_mesh_material_selections_available_to_pass
            .first()
            .expect("material pass should expose the source-backed model/mesh selection");
        assert_eq!(selected.source_asset_id, 42);
        assert_eq!(selected.source_id, 84);
        assert_eq!(selected.source_revision_id, Some(2));
        assert_eq!(selected.source_revision.as_deref(), Some("sha256:abc"));
        assert_eq!(selected.region_key, "source_material_slot:0");
        assert_eq!(selected.requested_material_slot_id, 7);
        assert_eq!(selected.resolved_material_slot_id, 7);
        assert_eq!(selected.material_table_index, 3);
        assert!(!selected.used_default_fallback);
    }

    #[test]
    fn non_material_pass_provenance_keeps_model_mesh_selection_out_of_pass_scope() {
        let mut packet = packet_with_feature_gate(
            WORLD_DRAW_RENDER_FEATURE_ID,
            FeatureExecutionGate {
                status: FeatureContributionStatus::Ready,
                fallback_policy: FeatureFallbackPolicy::SkipFeaturePasses,
            },
        );
        packet.prepared_material = Some(PreparedMaterialFeatureContribution {
            model_mesh_material_selections: vec![source_backed_model_mesh_selection()],
            ..PreparedMaterialFeatureContribution::default()
        });
        let pass = CompiledPassExecutionPlan::Present(CompiledPresentExecutionPlan {
            pass_id: RenderPassId::try_from_raw(9).unwrap(),
            authoring_index: 0,
            feature_id: Some(WORLD_DRAW_RENDER_FEATURE_ID),
            view_mask: CompiledViewMask::AllViews,
            source: None,
        });

        let evidence = collect_pass_material_binding_evidence(&packet, &pass);

        assert!(!evidence.consumes_material_resources);
        assert!(evidence.prepared_material_available);
        assert_eq!(evidence.prepared_model_mesh_material_selection_count, 1);
        assert!(
            evidence
                .model_mesh_material_selections_available_to_pass
                .is_empty(),
            "selection details should be scoped only to passes that consume material resources"
        );
    }

    #[test]
    fn material_scene_bundle_missing_from_packet_is_a_forbidden_fallback() {
        let packet = packet_with_feature_gate(
            MATERIAL_RENDER_FEATURE_ID,
            FeatureExecutionGate {
                status: FeatureContributionStatus::Ready,
                fallback_policy: FeatureFallbackPolicy::FailFrame,
            },
        );
        let shader_registry = ShaderRegistryResource::new();
        let shader = resolve_shader_material_for_packet(
            Some(&RenderShaderReference::MaterialSceneBundle {
                fallback_asset: "assets/shaders/editor_viewport_scene_product.wgsl".to_string(),
            }),
            &packet,
            &shader_registry,
            "fallback shader",
            "builtin:graphics",
        );

        assert!(
            shader.fallback_used,
            "material scene passes must not silently use the fallback scene shader when no generated scene bundle is prepared"
        );
    }

    #[test]
    fn pass_view_mask_filters_non_matching_views() {
        let renderer = Renderer::new();
        let mut explicit = BTreeSet::new();
        explicit.insert("main".to_string());
        let pass = CompiledPassExecutionPlan::Present(CompiledPresentExecutionPlan {
            pass_id: RenderPassId::try_from_raw(1).unwrap(),
            authoring_index: 0,
            feature_id: None,
            view_mask: CompiledViewMask::Explicit(explicit),
            source: None,
        });

        assert!(renderer.pass_targets_active_view(
            &pass,
            "main",
            crate::plugins::render::PreparedViewKind::MainSurface
        ));
        assert!(!renderer.pass_targets_active_view(
            &pass,
            "minimap",
            crate::plugins::render::PreparedViewKind::OffscreenProduct
        ));
    }

    fn source_backed_model_mesh_selection() -> PreparedModelMeshMaterialSelection {
        let source = PreparedModelMeshMaterialSourceIdentity::new(42, 84)
            .expect("source-backed model/mesh identity should be valid")
            .with_source_revision_id(2)
            .with_source_revision("sha256:abc");
        let surface =
            PreparedModelMeshMaterialRegionIdentity::new(source, "source_material_slot:0")
                .expect("source-backed region should form");
        PreparedModelMeshMaterialSelection::new(surface, 7, 7, 3, false)
            .expect("source-backed selection should form")
    }

    fn material_scene_fullscreen_pass() -> CompiledPassExecutionPlan {
        CompiledPassExecutionPlan::Fullscreen(CompiledRasterExecutionPlan {
            pass_id: RenderPassId::try_from_raw(7).unwrap(),
            authoring_index: 0,
            feature_id: Some(MATERIAL_RENDER_FEATURE_ID),
            shader: Some(RenderShaderReference::MaterialSceneBundle {
                fallback_asset: "fallback-scene.wgsl".to_string(),
            }),
            view_mask: CompiledViewMask::AllViews,
            bindings: CompiledPassBindings::default(),
            targets: CompiledTargetPlan::default(),
            draw_buffers: CompiledDrawBufferPlan::default(),
            raster_state: crate::plugins::render::CompiledRasterState {
                state: crate::plugins::render::RenderRasterState::default(),
            },
            clear_color: None,
            draw: None,
        })
    }
}
