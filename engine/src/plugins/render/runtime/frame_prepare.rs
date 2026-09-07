use crate::plugins::render::backend::{RenderSurfaceLifecycleState, RenderSurfaceRegistryResource};
use crate::plugins::render::inspect::RenderDebugTimingsState;
use crate::plugins::render::*;
use crate::plugins::scene::SceneResource;
use crate::runtime::{
    CatchupBudget, FixedTimeConfig, FixedTimeState, NativeWindowLifecycleState, WorldMut,
};
use runen_gpu::GpuWorkResourceId;
use std::any::{Any, TypeId};
use std::collections::{BTreeMap, BTreeSet};
use std::time::Instant;

type ExtractedRenderStateMap<'a> = BTreeMap<TypeId, &'a dyn Any>;

pub(crate) fn frame_render_prepare_system(mut world: WorldMut) -> anyhow::Result<()> {
    let (target_size, active_world_label, active_overlay_label) = {
        let scene_resource = world.resource::<SceneResource>()?;
        let Some(manager) = scene_resource.manager.as_ref() else {
            clear_prepared_frame(&mut world);
            return Ok(());
        };
        let (window_w, window_h) = manager.overlay_runtime.ui.screen_size;
        (
            (
                window_w.max(1.0).round() as u32,
                window_h.max(1.0).round() as u32,
            ),
            manager.world.active.label().to_string(),
            manager.active_overlay().label().to_string(),
        )
    };

    let Some(mut shader_registry) = world.remove_resource::<ShaderRegistryResource>() else {
        clear_prepared_frame(&mut world);
        return Ok(());
    };

    let shader_poll_start = Instant::now();
    let _ = shader_registry.poll_updates();
    let shader_poll_ms = shader_poll_start.elapsed().as_secs_f32() * 1000.0;
    let shader_poll_report = shader_registry.last_reload_poll_report();
    if let Ok(render_debug_timings) = world.resource_mut::<RenderDebugTimingsState>() {
        render_debug_timings.observe_shader_reload_poll(shader_poll_report, shader_poll_ms);
    }
    let shader_reload_messages = shader_registry.drain_message_lines();
    if !shader_reload_messages.is_empty() {
        let scene_resource = world.resource_mut::<SceneResource>()?;
        if let Some(manager) = scene_resource.manager.as_mut() {
            for msg in shader_reload_messages {
                manager
                    .overlay_runtime
                    .ui
                    .log_lines
                    .push(format!("[world] {msg}"));
            }
            crate::plugins::render::runtime::debug_eval::clamp_lines(
                &mut manager.overlay_runtime.ui.log_lines,
                manager.overlay_runtime.ui.max_lines,
            );
            manager.overlay_runtime.ui.log_scroll_lines_from_bottom = 0;
        }
    }

    let surface_infos = prepared_surface_infos(&mut world, target_size);

    let (flow_registry_revision, compiled_flows, execution_feature_ids, surface_packets) = {
        let flow_registry = match world.resource::<RenderFlowRegistryResource>() {
            Ok(registry) => registry,
            Err(_) => {
                world.insert_resource(shader_registry);
                clear_prepared_frame(&mut world);
                return Ok(());
            }
        };
        let compiled_flows = flow_registry.compiled_flows();
        let execution_feature_ids = collect_execution_feature_ids(compiled_flows);
        let extracted = collect_flow_declared_state_resources(&world, compiled_flows);
        let frame_requests = world
            .resource::<PreparedRenderFrameRequestResource>()
            .ok()
            .cloned()
            .unwrap_or_default();
        let mut surface_packets = Vec::with_capacity(surface_infos.len());
        for surface in surface_infos {
            let target_size = surface.target_size_px();
            let flows = build_prepared_flow_inputs(compiled_flows, &extracted, target_size)?;
            let views = build_prepared_views(target_size, &frame_requests)?;
            let flow_invocations = build_prepared_flow_invocations(
                compiled_flows,
                &extracted,
                &flows,
                &views,
                &frame_requests,
            )?;
            surface_packets.push((surface, flows, views, flow_invocations));
        }
        (
            flow_registry.revision(),
            compiled_flows.len(),
            execution_feature_ids,
            surface_packets,
        )
    };

    let (frame_index, prepare_epoch) = {
        if let Ok(prepared_resource) = world.resource_mut::<PreparedRenderFrameResource>() {
            (
                prepared_resource.allocate_frame_index(),
                prepared_resource.allocate_prepare_epoch(),
            )
        } else {
            (0, 0)
        }
    };

    let contributions = build_frame_feature_contributions(
        &world,
        active_world_label,
        active_overlay_label,
        &execution_feature_ids,
    );
    let dynamic_texture_targets = world
        .resource::<RenderDynamicTextureTargetRequestRegistryResource>()
        .ok()
        .map(|resource| resource.snapshot())
        .unwrap_or_default();
    let dynamic_texture_uploads = world
        .resource::<RenderDynamicTextureUploadRegistryResource>()
        .ok()
        .map(|resource| resource.snapshot())
        .unwrap_or_default();
    let product_selections = world
        .resource::<PreparedRenderProductSelectionResource>()
        .ok()
        .map(|resource| resource.snapshot())
        .unwrap_or_default();
    let viewport_surface_bindings = world
        .resource::<ViewportSurfaceBindingRegistryResource>()
        .ok()
        .map(|resource| resource.registry().clone())
        .unwrap_or_default();

    let prepared_frames = surface_packets
        .into_iter()
        .map(|(surface, flows, views, flow_invocations)| {
            let mut surface_contributions = contributions.clone();
            apply_surface_ui_contribution(
                &world,
                surface.render_surface_id,
                &mut surface_contributions,
            );
            PreparedRenderFrame {
                context: PreparedFrameContext {
                    frame_index,
                    flow_registry_revision,
                    shader_registry_revision: shader_registry.revision(),
                    prepare_epoch,
                },
                surface,
                views,
                flows,
                flow_invocations,
                dynamic_texture_targets: dynamic_texture_targets.clone(),
                dynamic_texture_uploads: dynamic_texture_uploads.clone(),
                product_selections: product_selections.clone(),
                viewport_surface_bindings: viewport_surface_bindings.clone(),
                contributions: surface_contributions,
                shader: PreparedShaderSnapshot {
                    registry_revision: shader_registry.revision(),
                },
            }
        })
        .collect::<Vec<_>>();

    if let Ok(prepared_resource) = world.resource_mut::<PreparedRenderFrameResource>() {
        prepared_resource.publish_set(prepared_frames);
    } else {
        let mut prepared_resource = PreparedRenderFrameResource::default();
        prepared_resource.publish_set(prepared_frames);
        world.insert_resource(prepared_resource);
    }

    world.insert_resource(shader_registry);

    if compiled_flows == 0 {
        clear_prepared_frame(&mut world);
    }

    Ok(())
}

fn prepared_surface_infos(
    world: &mut WorldMut,
    primary_target_size: (u32, u32),
) -> Vec<PreparedSurfaceInfo> {
    let primary_native_window_id = world
        .resource::<crate::runtime::WindowStateRegistryResource>()
        .ok()
        .and_then(|registry| registry.primary_window_id())
        .unwrap_or_else(crate::runtime::NativeWindowId::primary);
    if let Ok(registry) = world.resource_mut::<RenderSurfaceRegistryResource>() {
        registry.ensure_surface_for_native_window(primary_native_window_id, primary_target_size);
    }

    let created_windows = world
        .resource::<crate::runtime::WindowStateRegistryResource>()
        .ok()
        .map(|registry| {
            registry
                .records()
                .filter(|record| record.lifecycle_state == NativeWindowLifecycleState::Created)
                .map(|record| record.native_window_id)
                .collect::<BTreeSet<_>>()
        })
        .unwrap_or_else(|| BTreeSet::from([primary_native_window_id]));

    world
        .resource::<RenderSurfaceRegistryResource>()
        .ok()
        .map(|registry| {
            registry
                .records()
                .filter(|record| {
                    record.lifecycle_state == RenderSurfaceLifecycleState::Registered
                        && created_windows.contains(&record.native_window_id)
                })
                .map(|record| {
                    PreparedSurfaceInfo::for_surface(
                        record.render_surface_id,
                        record.native_window_id,
                        record.target_size_px,
                    )
                })
                .collect::<Vec<_>>()
        })
        .filter(|surfaces| !surfaces.is_empty())
        .unwrap_or_else(|| vec![PreparedSurfaceInfo::primary(primary_target_size)])
}

fn build_prepared_views(
    surface_size: (u32, u32),
    requests: &PreparedRenderFrameRequestResource,
) -> anyhow::Result<Vec<PreparedViewFrame>> {
    let mut views = BTreeMap::<String, PreparedViewFrame>::new();
    let main = PreparedViewFrame::main(surface_size);
    views.insert(main.view_id.clone(), main);
    for view in requests.requested_views() {
        if views.insert(view.view_id.clone(), view.clone()).is_some() {
            anyhow::bail!(
                "prepared render frame request publishes duplicate view '{}'",
                view.view_id
            );
        }
    }
    Ok(views.into_values().collect())
}

fn build_prepared_flow_invocations(
    compiled_flows: &[CompiledRenderFlowPlan],
    extracted_state: &ExtractedRenderStateMap<'_>,
    main_inputs_by_flow: &BTreeMap<RenderFlowId, PreparedFlowInputs>,
    views: &[PreparedViewFrame],
    requests: &PreparedRenderFrameRequestResource,
) -> anyhow::Result<Vec<PreparedFlowInvocation>> {
    let mut invocations = Vec::<PreparedFlowInvocation>::new();
    let views_by_id = views
        .iter()
        .map(|view| (view.view_id.as_str(), view))
        .collect::<BTreeMap<_, _>>();
    let flows_by_id = compiled_flows
        .iter()
        .map(|flow| (flow.flow_id, flow))
        .collect::<BTreeMap<_, _>>();

    let requested_flow_invocations = requests.requested_flow_invocations();
    let mut invocation_ids = BTreeSet::<&PreparedFlowInvocationId>::new();
    for request in &requested_flow_invocations {
        if !invocation_ids.insert(&request.invocation_id) {
            anyhow::bail!(
                "prepared flow invocation '{}' is requested more than once",
                request.invocation_id.0
            );
        }
        if !flows_by_id.contains_key(&request.flow_id) {
            anyhow::bail!(
                "prepared flow invocation '{}' references unknown flow '{:?}'",
                request.invocation_id.0,
                request.flow_id
            );
        }
        if !views_by_id.contains_key(request.view_id.as_str()) {
            anyhow::bail!(
                "prepared flow invocation '{}' references unknown view '{}'",
                request.invocation_id.0,
                request.view_id
            );
        }
    }

    for flow in compiled_flows {
        for request in requested_flow_invocations
            .iter()
            .copied()
            .filter(|request| request.flow_id == flow.flow_id)
        {
            let view = views_by_id
                .get(request.view_id.as_str())
                .expect("requested invocation view should be prevalidated");
            let inputs_by_flow = build_prepared_flow_inputs(
                std::slice::from_ref(flow),
                extracted_state,
                view.target_size_px,
            )?;
            let mut inputs = inputs_by_flow
                .get(&request.flow_id)
                .cloned()
                .ok_or_else(|| {
                    anyhow::anyhow!("missing prepared inputs for flow '{:?}'", request.flow_id)
                })?;
            apply_invocation_uniform_overrides(flow, request, &mut inputs)?;
            invocations.push(PreparedFlowInvocation {
                invocation_id: request.invocation_id.clone(),
                flow_id: request.flow_id,
                view_id: request.view_id.clone(),
                inputs,
                target_alias_bindings: request.target_alias_bindings.clone(),
                history_signature: request.history_signature.clone(),
            });
        }

        let inputs = main_inputs_by_flow
            .get(&flow.flow_id)
            .cloned()
            .ok_or_else(|| {
                anyhow::anyhow!("missing main prepared inputs for flow '{:?}'", flow.flow_id)
            })?;
        invocations.push(PreparedFlowInvocation::main(flow.flow_id, inputs));
    }

    Ok(invocations)
}

fn apply_invocation_uniform_overrides(
    flow: &CompiledRenderFlowPlan,
    request: &PreparedFlowInvocationRequest,
    inputs: &mut PreparedFlowInputs,
) -> anyhow::Result<()> {
    for (uniform_id, bytes) in &request.uniform_overrides {
        if !flow.resources.has_uniform_buffer(uniform_id) {
            anyhow::bail!(
                "prepared flow invocation '{}' overrides unknown uniform buffer '{:?}' in flow '{:?}'",
                request.invocation_id.0,
                uniform_id,
                flow.flow_id
            );
        }
        if bytes.is_empty() {
            anyhow::bail!(
                "prepared flow invocation '{}' overrides uniform buffer '{:?}' with empty bytes",
                request.invocation_id.0,
                uniform_id
            );
        }
        inputs
            .projected_uniform_bytes
            .insert(*uniform_id, bytes.clone());
    }

    Ok(())
}

pub(crate) fn clear_prepared_frame(world: &mut WorldMut) {
    if let Ok(prepared_resource) = world.resource_mut::<PreparedRenderFrameResource>() {
        prepared_resource.clear();
    }
}

pub(crate) fn build_frame_feature_contributions(
    world: &ecs::World,
    world_scene_label: String,
    overlay_scene_label: String,
    execution_feature_ids: &[RenderFeatureId],
) -> PreparedFrameContributions {
    let mut contributions = PreparedFrameContributions::default();
    let scene_route = PreparedSceneRouteContribution {
        world_scene_label,
        overlay_scene_label,
    };

    collect_registered_feature_contributions(world, &scene_route, &mut contributions);

    if contributions.feature(&UI_RENDER_FEATURE_ID).is_none()
        && let Ok(resource) = world.resource::<PreparedUiFrameResource>()
    {
        let ui_policy = feature_policy(world, UI_RENDER_FEATURE_ID, resource.fallback_policy);
        contributions.insert_ui(resource.payload.clone(), resource.status, ui_policy);
    }

    let world_draw_from_registry = contributions
        .feature(&WORLD_DRAW_RENDER_FEATURE_ID)
        .is_some();
    if !world_draw_from_registry {
        if let Ok(resource) = world.resource::<PreparedWorldFeatureResource>() {
            let world_policy = feature_policy(
                world,
                WORLD_DRAW_RENDER_FEATURE_ID,
                resource.fallback_policy,
            );
            contributions.insert_world(resource.payload.clone(), resource.status, world_policy);
        }

        if let Ok(resource) = world.resource::<PreparedDrawFeatureResource>() {
            let world_feature_id = WORLD_DRAW_RENDER_FEATURE_ID;
            let should_publish_draw =
                !matches!(resource.status, FeatureContributionStatus::Missing)
                    || contributions.feature(&world_feature_id).is_none();
            if should_publish_draw {
                let draw_policy = feature_policy(world, world_feature_id, resource.fallback_policy);
                contributions.insert_draw(resource.payload.clone(), resource.status, draw_policy);
            }
        }
    }

    if contributions
        .feature(&CAVE_INTERIOR_RENDER_FEATURE_ID)
        .is_none()
        && let Ok(resource) = world.resource::<PreparedCaveFeatureResource>()
    {
        let cave_policy = feature_policy(
            world,
            CAVE_INTERIOR_RENDER_FEATURE_ID,
            resource.fallback_policy,
        );
        contributions.insert_caves(resource.payload.clone(), resource.status, cave_policy);
    }

    if contributions.feature(&DETAIL_RENDER_FEATURE_ID).is_none()
        && let Ok(resource) = world.resource::<PreparedDetailFeatureResource>()
    {
        let detail_policy =
            feature_policy(world, DETAIL_RENDER_FEATURE_ID, resource.fallback_policy);
        contributions.insert_detail(resource.payload.clone(), resource.status, detail_policy);
    }

    if contributions
        .feature(&PROCEDURAL_WORLD_RENDER_FEATURE_ID)
        .is_none()
        && let Ok(resource) = world.resource::<PreparedProceduralWorldFeatureResource>()
    {
        let procedural_policy = feature_policy(
            world,
            PROCEDURAL_WORLD_RENDER_FEATURE_ID,
            resource.fallback_policy,
        );
        contributions.insert_procedural_world(
            resource.payload.clone(),
            resource.status,
            procedural_policy,
        );
    }

    if contributions.feature(&MATERIAL_RENDER_FEATURE_ID).is_none()
        && let Ok(resource) = world.resource::<PreparedMaterialFeatureResource>()
    {
        let material_policy =
            feature_policy(world, MATERIAL_RENDER_FEATURE_ID, resource.fallback_policy);
        contributions.insert_material(resource.payload.clone(), resource.status, material_policy);
    }

    if contributions
        .feature(&DEFORMATION_RENDER_FEATURE_ID)
        .is_none()
        && let Ok(resource) = world.resource::<PreparedDeformationFeatureResource>()
    {
        let deformation_policy = feature_policy(
            world,
            DEFORMATION_RENDER_FEATURE_ID,
            resource.fallback_policy,
        );
        contributions.insert_deformation(
            resource.payload.clone(),
            resource.status,
            deformation_policy,
        );
    }

    if contributions
        .feature(&WIND_FIELDS_RENDER_FEATURE_ID)
        .is_none()
        && let Ok(resource) = world.resource::<PreparedWindFieldFeatureResource>()
    {
        let wind_policy = feature_policy(
            world,
            WIND_FIELDS_RENDER_FEATURE_ID,
            resource.fallback_policy,
        );
        contributions.insert_wind_fields(resource.payload.clone(), resource.status, wind_policy);
    }

    for feature_id in execution_feature_ids {
        if contributions.feature(feature_id).is_some() {
            continue;
        }
        let fallback_policy =
            feature_policy(world, *feature_id, FeatureFallbackPolicy::SkipFeaturePasses);
        contributions.insert_missing(*feature_id, fallback_policy);
    }

    if let Ok(feature_registry) = world.resource::<RenderFeatureRegistryResource>() {
        for feature_id in feature_registry.resolved_order() {
            if contributions.feature(feature_id).is_some() {
                continue;
            }
            let fallback_policy = feature_registry
                .descriptor(feature_id)
                .map(|descriptor| descriptor.fallback_policy)
                .unwrap_or(FeatureFallbackPolicy::SkipFeaturePasses);
            contributions.insert_missing(*feature_id, fallback_policy);
        }
    }

    contributions
}

fn apply_surface_ui_contribution(
    world: &ecs::World,
    render_surface_id: crate::plugins::render::backend::RenderSurfaceId,
    contributions: &mut PreparedFrameContributions,
) {
    let Ok(resource) = world.resource::<PreparedUiFrameResource>() else {
        return;
    };
    let ui_policy = feature_policy(world, UI_RENDER_FEATURE_ID, resource.fallback_policy);
    contributions.insert_ui(
        resource.payload_for_surface(render_surface_id).clone(),
        resource.status_for_surface(render_surface_id),
        ui_policy,
    );
}

fn collect_registered_feature_contributions(
    world: &ecs::World,
    scene_route: &PreparedSceneRouteContribution,
    contributions: &mut PreparedFrameContributions,
) {
    let collector_registry = world
        .resource::<RenderFeatureContributionCollectorRegistryResource>()
        .ok()
        .cloned()
        .unwrap_or_default();
    for diagnostic in collector_registry.diagnostics() {
        contributions.push_diagnostic(diagnostic.clone());
    }
    let feature_registry = world.resource::<RenderFeatureRegistryResource>().ok();

    for collector in collector_registry.collectors() {
        let descriptor = &collector.descriptor;
        if let Some(feature_registry) = feature_registry
            && feature_registry
                .descriptor(&descriptor.feature_id)
                .is_none()
        {
            contributions.push_diagnostic(
                PreparedFeatureContributionDiagnostic::error(
                    descriptor.feature_id,
                    format!(
                        "collector '{}' references unknown render feature {:?}",
                        descriptor.collector_id, descriptor.feature_id
                    ),
                )
                .with_collector_id(descriptor.collector_id.clone())
                .with_payload_kind(descriptor.payload_kind.clone()),
            );
            continue;
        }
        if let Err(diagnostic) = validate_collector_resources(world, descriptor) {
            contributions.push_diagnostic(diagnostic);
            continue;
        }
        if contributions.feature(&descriptor.feature_id).is_some() {
            contributions.push_diagnostic(
                PreparedFeatureContributionDiagnostic::error(
                    descriptor.feature_id,
                    format!(
                        "collector '{}' conflicts with existing contribution for feature {:?}",
                        descriptor.collector_id, descriptor.feature_id
                    ),
                )
                .with_collector_id(descriptor.collector_id.clone())
                .with_payload_kind(descriptor.payload_kind.clone()),
            );
            continue;
        }

        let fallback_policy =
            feature_policy(world, descriptor.feature_id, descriptor.fallback_policy);
        let context = RenderFeatureContributionContext::new(
            world,
            descriptor,
            fallback_policy,
            Some(scene_route),
        );
        match (collector.collect)(&context) {
            Ok(contribution) => {
                if let Err(diagnostic) = validate_collected_contribution(descriptor, &contribution)
                {
                    contributions.push_diagnostic(diagnostic);
                    continue;
                }
                contributions.insert(descriptor.feature_id, contribution);
            }
            Err(diagnostic) => contributions.push_diagnostic(diagnostic),
        }
    }
}

fn feature_policy(
    world: &ecs::World,
    feature_id: RenderFeatureId,
    fallback: FeatureFallbackPolicy,
) -> FeatureFallbackPolicy {
    world
        .resource::<RenderFeatureRegistryResource>()
        .ok()
        .and_then(|registry| registry.descriptor(&feature_id))
        .map(|descriptor| descriptor.fallback_policy)
        .unwrap_or(fallback)
}

fn collect_execution_feature_ids(
    compiled_flows: &[CompiledRenderFlowPlan],
) -> Vec<RenderFeatureId> {
    let mut ids = BTreeSet::<RenderFeatureId>::new();

    for flow in compiled_flows {
        for pass in &flow.execution.passes {
            let feature_id = match pass {
                CompiledPassExecutionPlan::Compute(value) => value.feature_id,
                CompiledPassExecutionPlan::Fullscreen(value) => value.feature_id,
                CompiledPassExecutionPlan::Graphics(value) => value.feature_id,
                CompiledPassExecutionPlan::Copy(value) => value.feature_id,
                CompiledPassExecutionPlan::Present(value) => value.feature_id,
                CompiledPassExecutionPlan::BuiltinUiComposite(value) => Some(value.feature_id),
            };

            if let Some(feature_id) = feature_id {
                ids.insert(feature_id);
            }
        }
    }

    ids.into_iter().collect()
}

fn collect_flow_declared_state_resources<'a>(
    world: &'a ecs::World,
    compiled_flows: &[CompiledRenderFlowPlan],
) -> ExtractedRenderStateMap<'a> {
    let mut values = ExtractedRenderStateMap::new();
    let mut type_ids = BTreeSet::<TypeId>::new();

    for flow in compiled_flows {
        for declaration in &flow.resources.state_resources {
            type_ids.insert(declaration.type_id);
        }
    }

    for type_id in type_ids {
        if let Some(resource) = world.resource_by_type_id(type_id) {
            values.insert(type_id, resource);
        }
    }

    values
}

fn build_prepared_flow_inputs(
    compiled_flows: &[CompiledRenderFlowPlan],
    extracted_state: &ExtractedRenderStateMap<'_>,
    surface_size: (u32, u32),
) -> anyhow::Result<BTreeMap<RenderFlowId, PreparedFlowInputs>> {
    let mut outputs = BTreeMap::<RenderFlowId, PreparedFlowInputs>::new();

    for flow in compiled_flows {
        let mut projected_uniform_bytes = BTreeMap::<GpuWorkResourceId, Vec<u8>>::new();

        for pass in &flow.render_passes {
            for binding in &pass.node().uniform_bindings {
                if !flow.resources.has_state_resource(binding.state_type_id()) {
                    anyhow::bail!(
                        "uniform projection for flow '{:?}' pass '{:?}' requires undeclared state '{}'",
                        flow.flow_id,
                        pass.pass_id(),
                        binding.state_type_name()
                    );
                }

                if !flow.resources.has_uniform_buffer(binding.uniform_id()) {
                    anyhow::bail!(
                        "uniform projection for flow '{:?}' pass '{:?}' references unknown uniform buffer '{:?}'",
                        flow.flow_id,
                        pass.pass_id(),
                        binding.uniform_id()
                    );
                }

                let state = extracted_state
                    .get(&binding.state_type_id())
                    .copied()
                    .ok_or_else(|| {
                        anyhow::anyhow!(
                            "uniform projection for flow '{:?}' pass '{:?}' requires missing ecs state '{}'",
                            flow.flow_id,
                            pass.pass_id(),
                            binding.state_type_name()
                        )
                    })?;

                let bytes = binding.project_bytes(state, surface_size).ok_or_else(|| {
                    anyhow::anyhow!(
                        "uniform projection for flow '{:?}' pass '{:?}' failed for state '{}'",
                        flow.flow_id,
                        pass.pass_id(),
                        binding.state_type_name()
                    )
                })?;

                let key = *binding.uniform_id();
                if let Some(existing) = projected_uniform_bytes.get(&key) {
                    if existing != &bytes {
                        anyhow::bail!(
                            "uniform projection conflict for buffer '{:?}' in flow '{:?}'",
                            key,
                            flow.flow_id
                        );
                    }
                    continue;
                }
                projected_uniform_bytes.insert(key, bytes);
            }
        }
        project_fixed_step_region_uniforms(flow, extracted_state, &mut projected_uniform_bytes)?;

        let mut projected_dispatch_workgroups = BTreeMap::<RenderPassId, [u32; 3]>::new();
        for pass in &flow.render_passes {
            if !matches!(pass.node().kind, RenderPassKind::Compute) {
                continue;
            }
            let dispatch = project_dispatch_for_pass(pass.node(), extracted_state)?;
            projected_dispatch_workgroups.insert(pass.pass_id(), dispatch);
        }

        let required_state_types = flow
            .resources
            .state_resources
            .iter()
            .map(|value| PreparedStateTypeInfo {
                type_name: value.type_name,
            })
            .collect::<Vec<_>>();

        outputs.insert(
            flow.flow_id,
            PreparedFlowInputs {
                projected_uniform_bytes,
                projected_dispatch_workgroups,
                required_state_types,
            },
        );
    }

    Ok(outputs)
}

fn project_fixed_step_region_uniforms(
    flow: &CompiledRenderFlowPlan,
    extracted_state: &ExtractedRenderStateMap<'_>,
    projected_uniform_bytes: &mut BTreeMap<GpuWorkResourceId, Vec<u8>>,
) -> anyhow::Result<()> {
    if flow.execution.fixed_step_regions.is_empty() {
        return Ok(());
    }

    let fixed_config = extracted_state
        .get(&TypeId::of::<FixedTimeConfig>())
        .and_then(|value| value.downcast_ref::<FixedTimeConfig>())
        .copied()
        .ok_or_else(|| {
            anyhow::anyhow!(
                "flow '{:?}' fixed-step region requires missing ecs state '{}'",
                flow.flow_id,
                std::any::type_name::<FixedTimeConfig>()
            )
        })?;
    let fixed_state = extracted_state
        .get(&TypeId::of::<FixedTimeState>())
        .and_then(|value| value.downcast_ref::<FixedTimeState>())
        .copied()
        .ok_or_else(|| {
            anyhow::anyhow!(
                "flow '{:?}' fixed-step region requires missing ecs state '{}'",
                flow.flow_id,
                std::any::type_name::<FixedTimeState>()
            )
        })?;
    let catchup_budget = extracted_state
        .get(&TypeId::of::<CatchupBudget>())
        .and_then(|value| value.downcast_ref::<CatchupBudget>())
        .copied()
        .ok_or_else(|| {
            anyhow::anyhow!(
                "flow '{:?}' fixed-step region requires missing ecs state '{}'",
                flow.flow_id,
                std::any::type_name::<CatchupBudget>()
            )
        })?;

    for region in &flow.execution.fixed_step_regions {
        let submitted_substeps =
            fixed_state.bounded_render_substeps(catchup_budget, region.max_substeps);
        let uniform = RenderFixedStepIterationUniform::new(
            0,
            submitted_substeps,
            region.max_substeps,
            fixed_state.saturated_frames,
            fixed_config.step_seconds,
            fixed_state.accumulator_seconds,
        );
        projected_uniform_bytes.insert(region.iteration_uniform, uniform.to_uniform_bytes());
    }

    Ok(())
}

fn project_dispatch_for_pass(
    node: &crate::plugins::render::RenderPassNode,
    extracted_state: &ExtractedRenderStateMap<'_>,
) -> anyhow::Result<[u32; 3]> {
    let dispatch = match &node.compute_dispatch {
        Some(ComputeDispatchDescriptor::Fixed(value)) => *value,
        Some(ComputeDispatchDescriptor::State(binding)) => {
            let state = extracted_state
                .get(&binding.state_type_id())
                .copied()
                .ok_or_else(|| {
                    anyhow::anyhow!(
                        "compute pass '{:?}' dispatch_state requires missing ecs resource '{}'",
                        node.id,
                        binding.state_type_name()
                    )
                })?;
            binding.project_dispatch(state).ok_or_else(|| {
                anyhow::anyhow!(
                    "compute pass '{:?}' failed to project dispatch_state for '{}'",
                    node.id,
                    binding.state_type_name()
                )
            })?
        }
        None => {
            anyhow::bail!(
                "compute pass '{:?}' must declare explicit dispatch_workgroups(...) or dispatch_state(...)",
                node.id
            );
        }
    };

    Ok(dispatch)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, ecs::Component, ecs::Resource)]
    struct TestContributionResource {
        value: String,
    }

    fn test_feature_id() -> RenderFeatureId {
        RenderFeatureId::try_from_raw(10_042).expect("test feature id should be non-zero")
    }

    fn registered_test_collector() -> RenderFeatureContributionCollector {
        RenderFeatureContributionCollector::new(
            RenderFeatureContributionCollectorDescriptor::new(
                test_feature_id(),
                "test.collector",
                "test.payload",
            )
            .require_resource::<TestContributionResource>(),
            collect_test_registered_contribution,
        )
    }

    fn collect_test_registered_contribution(
        context: &RenderFeatureContributionContext<'_>,
    ) -> Result<PreparedFeatureContribution, PreparedFeatureContributionDiagnostic> {
        let Some(resource) = context.resource::<TestContributionResource>() else {
            return Err(PreparedFeatureContributionDiagnostic::error(
                context.descriptor().feature_id,
                "test collector requires TestContributionResource",
            )
            .with_collector_id(context.descriptor().collector_id.clone())
            .with_payload_kind(context.descriptor().payload_kind.clone()));
        };
        Ok(PreparedFeatureContribution {
            status: FeatureContributionStatus::Ready,
            fallback_policy: context.fallback_policy(),
            payload: PreparedFeaturePayload::Registered(PreparedRegisteredFeaturePayload::new(
                StaticRegisteredFeaturePayload::new("test.payload", "test contribution")
                    .with_field("value", resource.value.clone()),
            )),
        })
    }

    fn world_with_test_feature() -> ecs::World {
        let mut world = ecs::World::default();
        let mut feature_registry = RenderFeatureRegistryResource::default();
        feature_registry.upsert_descriptor(RenderFeatureDescriptor::new(
            test_feature_id(),
            "test.registered",
        ));
        world.insert_resource(feature_registry);
        world
    }

    #[test]
    fn requested_offscreen_invocations_prepare_before_main_invocation() {
        let flow = RenderFlow::new("prepare.order")
            .with_surface_color()
            .expect("render flow authoring should succeed")
            .fullscreen_pass("main")
            .main_surface_only()
            .write_surface_color()
            .expect("render flow authoring should succeed")
            .finish()
            .validate()
            .expect("test flow should validate");
        let compiled = compile_flow_plan(&flow).expect("test flow should compile");
        let compiled_flows = vec![compiled.clone()];
        let extracted = ExtractedRenderStateMap::new();
        let main_inputs =
            build_prepared_flow_inputs(&compiled_flows, &extracted, (800, 600)).unwrap();
        let mut requests = PreparedRenderFrameRequestResource::default();
        requests
            .replace_contribution(
                RenderFrameProducerId::try_from_raw(1).unwrap(),
                [PreparedViewFrame::offscreen_product(
                    "viewport.1",
                    (320, 200),
                )],
                [PreparedFlowInvocationRequest {
                    invocation_id: PreparedFlowInvocationId::new("viewport.1.scene"),
                    flow_id: compiled.flow_id,
                    view_id: "viewport.1".to_string(),
                    target_alias_bindings: BTreeMap::new(),
                    uniform_overrides: BTreeMap::new(),
                    history_signature: None,
                }],
            )
            .unwrap();
        let views = build_prepared_views((800, 600), &requests).unwrap();

        let invocations = build_prepared_flow_invocations(
            &compiled_flows,
            &extracted,
            &main_inputs,
            &views,
            &requests,
        )
        .expect("invocations should prepare");

        assert_eq!(invocations.len(), 2);
        assert_eq!(invocations[0].view_id, "viewport.1");
        assert_eq!(invocations[1].view_id, "main");
    }

    #[test]
    fn render_feature_contributions_default_scene_route_uses_registered_collector() {
        let world = ecs::World::default();

        let contributions = build_frame_feature_contributions(
            &world,
            "world.scene".to_string(),
            "overlay.scene".to_string(),
            &[],
        );

        assert_eq!(
            contributions.scene_route_labels(),
            Some(("world.scene", "overlay.scene"))
        );
        assert!(contributions.diagnostics().is_empty());
    }

    #[test]
    fn render_feature_contributions_registered_payload_does_not_need_central_variant() {
        let mut world = world_with_test_feature();
        world.insert_resource(TestContributionResource {
            value: "from-resource".to_string(),
        });
        let mut collector_registry = RenderFeatureContributionCollectorRegistryResource::default();
        collector_registry
            .try_register_collector(registered_test_collector())
            .expect("test collector should register");
        world.insert_resource(collector_registry);

        let contributions = build_frame_feature_contributions(
            &world,
            "world.scene".to_string(),
            "overlay.scene".to_string(),
            &[test_feature_id()],
        );

        let contribution = contributions
            .feature(&test_feature_id())
            .expect("registered feature should contribute");
        let PreparedFeaturePayload::Registered(payload) = &contribution.payload else {
            panic!("test feature should use registered payload bridge");
        };
        let inspection = payload.inspect();
        assert_eq!(inspection.payload_kind, "test.payload");
        assert_eq!(inspection.summary, "test contribution");
        assert_eq!(
            inspection.fields,
            vec![("value".to_string(), "from-resource".to_string())]
        );
        assert!(contributions.diagnostics().is_empty());
    }

    #[test]
    fn render_feature_contributions_missing_declared_resource_is_typed_diagnostic() {
        let mut world = world_with_test_feature();
        let mut collector_registry = RenderFeatureContributionCollectorRegistryResource::default();
        collector_registry
            .try_register_collector(registered_test_collector())
            .expect("test collector should register");
        world.insert_resource(collector_registry);

        let contributions = build_frame_feature_contributions(
            &world,
            "world.scene".to_string(),
            "overlay.scene".to_string(),
            &[test_feature_id()],
        );

        let contribution = contributions
            .feature(&test_feature_id())
            .expect("execution feature should still receive a missing contribution");
        assert_eq!(contribution.status, FeatureContributionStatus::Missing);
        assert!(contributions.diagnostics().iter().any(|diagnostic| {
            diagnostic
                .message
                .contains("requires missing prepared resource")
        }));
    }
}
