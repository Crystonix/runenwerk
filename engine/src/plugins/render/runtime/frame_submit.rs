use crate::plugins::SceneResource;
use crate::plugins::inspect::{
    RenderCapturedTextureState, RenderDebugConfigResource, RenderDebugControlResource,
    RenderDebugFrameReportState, RenderDebugTimingsState, RenderFrameDiagnosticsMode,
    RenderFrameDiagnosticsPolicyResource, RenderPassProvenanceState,
    RenderRuntimeResourceInspectorState, RenderTextureInspectorState,
    submit_render_frame_report_to_diagnostics,
};
use crate::plugins::pipelines::{PipelineCacheResource, PipelineCacheStats};
use crate::plugins::render::backend::RenderSurfaceAcquireError;
use crate::plugins::render::backend::{RenderSurfaceDiagnostic, RenderSurfaceRegistryResource};
use crate::plugins::render::runtime::{
    CompletedRenderFrameDiagnostics, RenderFrameDiagnosticsSnapshot,
    RenderFrameDiagnosticsTransactionState,
};
use crate::plugins::render::*;
use crate::plugins::time::domain::Time;
use crate::runtime::FramePacingRuntimeStateResource;
use crate::runtime::{SimulationTick, WorldMut};
use crate::state::{DebugMetricsState, StartupState};
use anyhow::anyhow;

const FRAME_TIMING_LOG_THRESHOLD_MS: f32 = 20.0;
const MESH_HOT_PATH_LOG_THRESHOLD_MS: f32 = 8.0;

fn render_timing_logging_enabled() -> bool {
    std::env::var("GROTTO_RENDER_TIMING_LOG")
        .map(|value| {
            matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
        .unwrap_or(false)
}

pub(crate) fn frame_render_submit_system(mut world: WorldMut) -> anyhow::Result<()> {
    if world.resource::<SceneResource>()?.manager.is_none() {
        return Ok(());
    }

    let _submit_span = tracing::info_span!("systems.frame_render_submit").entered();
    let startup_ready_before = world.resource::<StartupState>()?.is_ready();
    let delta_seconds = world.resource::<Time>()?.delta_seconds;
    let timing_log_enabled = render_timing_logging_enabled();

    let (prepared_frame, additional_prepared_frames) = {
        let Some(mut prepared_resource) = world.remove_resource::<PreparedRenderFrameResource>()
        else {
            return Ok(());
        };
        let mut prepared_frames = prepared_resource.take_all().into_iter();
        let Some(prepared_frame) = prepared_frames.next() else {
            world.insert_resource(prepared_resource);
            return Ok(());
        };
        let additional_prepared_frames = prepared_frames.collect::<Vec<_>>();
        world.insert_resource(prepared_resource);
        (prepared_frame, additional_prepared_frames)
    };

    validate_prepared_frame_surface_scope(&mut world, &prepared_frame)?;

    let Some(mut shader_registry) = world.remove_resource::<ShaderRegistryResource>() else {
        return Ok(());
    };

    let Some(mut gfx) = world.remove_resource::<Gfx>() else {
        world.insert_resource(shader_registry);
        return Ok(());
    };

    let (target_w, target_h) = prepared_frame
        .views
        .iter()
        .find(|view| matches!(view.kind, PreparedViewKind::MainSurface))
        .ok_or_else(|| anyhow!("prepared render frame is missing a main surface view"))?
        .target_size_px;

    let render_surface_id = prepared_frame.surface.render_surface_id;
    let surface_size = gfx.surface_size(render_surface_id);
    if surface_size != Some((target_w, target_h)) {
        gfx.resize(render_surface_id, target_w, target_h);
    }

    let ui_font_atlas = world
        .resource::<UiFontAtlasResource>()
        .ok()
        .cloned()
        .unwrap_or_default();
    let debug_control = world
        .resource::<RenderDebugControlResource>()
        .ok()
        .cloned()
        .unwrap_or_default();
    let debug_config = world
        .resource::<RenderDebugConfigResource>()
        .ok()
        .cloned()
        .unwrap_or_default();
    let preflight_config = world
        .resource::<RenderPreflightValidationConfigResource>()
        .ok()
        .copied()
        .unwrap_or_default();
    let diagnostics_policy = world
        .resource::<RenderFrameDiagnosticsPolicyResource>()
        .ok()
        .copied()
        .unwrap_or_default();
    let diagnostics_simulation_tick = world
        .resource::<SimulationTick>()
        .map(|tick| tick.0)
        .unwrap_or_default();

    let render_result = {
        let flow_registry = match world.resource::<RenderFlowRegistryResource>() {
            Ok(registry) => registry,
            Err(_) => {
                world.insert_resource(shader_registry);
                world.insert_resource(gfx);
                return Ok(());
            }
        };
        if flow_registry.revision() != prepared_frame.context.flow_registry_revision {
            world.insert_resource(shader_registry);
            world.insert_resource(gfx);
            return Ok(());
        }
        let compiled_flows = flow_registry.compiled_flows();

        let ui_rect_shader: Option<ShaderHandle> = prepared_frame
            .ui()
            .and_then(|ui| ui.first_rect_shader_asset_id())
            .and_then(|id| shader_registry.handle(id));

        gfx.render(
            &prepared_frame,
            &mut shader_registry,
            compiled_flows,
            ui_rect_shader,
            &ui_font_atlas,
            &prepared_frame.viewport_surface_bindings,
            preflight_config,
            &debug_control,
            &debug_config,
        )
    };

    let result = match render_result {
        Ok(timings) => {
            world.resource_mut::<DebugMetricsState>()?.last_timings = Some(timings);

            if let Ok(render_debug_timings) = world.resource_mut::<RenderDebugTimingsState>() {
                render_debug_timings.observe_frame_timings(timings);
                render_debug_timings.observe_pass_timings(gfx.renderer.last_pass_timings());
                render_debug_timings
                    .observe_gpu_pass_timing_evidence(gfx.renderer.last_gpu_pass_timing_evidence());
            }

            let cache_stats = gfx.renderer.flow_pipeline_cache_stats();
            if let Ok(cache_resource) = world.resource_mut::<PipelineCacheResource>() {
                cache_resource.observe_stats(PipelineCacheStats {
                    hits: cache_stats.hits,
                    misses: cache_stats.misses,
                    failures: cache_stats.failures,
                });
            }

            if let Ok(runtime_resources) =
                world.resource_mut::<RenderRuntimeResourceInspectorState>()
            {
                runtime_resources.observe_runtime_resources(gfx.renderer.last_runtime_resources());
                runtime_resources
                    .observe_pipeline_cache_stats(cache_stats.hits, cache_stats.misses);
            }

            if let Ok(pass_provenance) = world.resource_mut::<RenderPassProvenanceState>() {
                pass_provenance.observe_frame(
                    prepared_frame.context.frame_index,
                    gfx.renderer.last_pass_provenance(),
                );
            }

            let published_capture_results = gfx.renderer.take_published_capture_selector_results();
            let published_captures = gfx.renderer.take_published_captured_textures();
            if let Ok(captured_textures) = world.resource_mut::<RenderCapturedTextureState>() {
                captured_textures
                    .observe_frame(prepared_frame.context.frame_index, &published_captures);
            }

            if let Ok(texture_inspector) = world.resource_mut::<RenderTextureInspectorState>() {
                texture_inspector.observe_captures(&published_captures);
            }

            let total_ms = timings.acquire_ms
                + timings.renderer.prepare_ui_ms
                + timings.renderer.prepare_mesh_ms
                + timings.renderer.world_prepare_ms
                + timings.renderer.preflight_ms
                + timings.renderer.flow_encode_ms
                + timings.renderer.encode_submit_ms
                + timings.present_ms;

            let workload_ms = timings.renderer.prepare_ui_ms
                + timings.renderer.prepare_mesh_ms
                + timings.renderer.world_prepare_ms
                + timings.renderer.preflight_ms
                + timings.renderer.flow_encode_ms
                + timings.renderer.encode_submit_ms;

            let full_diagnostics = should_build_full_render_diagnostics(
                diagnostics_policy,
                &debug_control,
                &debug_config,
                workload_ms,
            );
            if diagnostics_policy.force_next_full_report
                && let Ok(policy) = world.resource_mut::<RenderFrameDiagnosticsPolicyResource>()
            {
                policy.force_next_full_report = false;
            }

            let diagnostics_start = std::time::Instant::now();
            let semantic_frame_index = prepared_frame.context.frame_index;
            let (current_captures, delayed_captures): (Vec<_>, Vec<_>) = published_captures
                .into_iter()
                .partition(|capture| capture.identity.frame_index == semantic_frame_index);
            let (current_capture_results, delayed_capture_results): (Vec<_>, Vec<_>) =
                published_capture_results.into_iter().partition(|result| {
                    result
                        .frame_identity
                        .as_ref()
                        .is_none_or(|identity| identity.frame_index == semantic_frame_index)
                });

            let mut diagnostics_transactions = world
                .remove_resource::<RenderFrameDiagnosticsTransactionState>()
                .unwrap_or_default();
            let mut completed_reports: Vec<CompletedRenderFrameDiagnostics> =
                diagnostics_transactions
                    .observe_terminal_captures(delayed_captures, delayed_capture_results);
            if full_diagnostics {
                completed_reports.extend(
                    diagnostics_transactions.begin_frame(
                        RenderFrameDiagnosticsSnapshot {
                            frame_index: semantic_frame_index,
                            simulation_tick: diagnostics_simulation_tick,
                            provenance: gfx.renderer.last_pass_provenance().to_vec(),
                            capture_plan: gfx.renderer.last_capture_plan().clone(),
                            pixel_probes: debug_config.pixel_probes.clone(),
                            texture_diffs: debug_config.texture_diffs.clone(),
                            artifact_output_dir: debug_control
                                .artifact_export_enabled
                                .then(|| debug_control.artifact_output_dir.clone()),
                        },
                        current_captures,
                        current_capture_results,
                    ),
                );
            }
            world.insert_resource(diagnostics_transactions);

            for completed in completed_reports {
                let simulation_tick = completed.simulation_tick;
                let mut frame_report = completed.report;
                frame_report
                    .errors
                    .extend(frame_report.validate_invariants());
                if let Err(err) = submit_render_frame_report_to_diagnostics(
                    &mut world,
                    &frame_report,
                    simulation_tick,
                ) {
                    tracing::warn!(
                        frame = frame_report.frame_index,
                        error = %err,
                        "failed submitting render diagnostics report to canonical diagnostics core"
                    );
                }
                if let Ok(report_state) = world.resource_mut::<RenderDebugFrameReportState>() {
                    report_state.observe_frame(frame_report);
                }
            }
            let diagnostics_mode = if full_diagnostics {
                "full"
            } else {
                "lightweight"
            };
            let diagnostics_report_ms = diagnostics_start.elapsed().as_secs_f32() * 1000.0;

            let pacing_state = world
                .resource::<FramePacingRuntimeStateResource>()
                .ok()
                .cloned();
            if let Ok(render_debug_timings) = world.resource_mut::<RenderDebugTimingsState>() {
                render_debug_timings
                    .observe_preflight_cache_state(gfx.renderer.last_preflight_cache_state());
                render_debug_timings
                    .observe_diagnostics_report(diagnostics_mode, diagnostics_report_ms);
                if let Some(pacing_state) = pacing_state.as_ref() {
                    render_debug_timings.observe_frame_pacing(pacing_state);
                }
            }

            let mesh_hot = timings.renderer.mesh_hot_path;
            let warm_frame = mesh_hot.is_warm_frame();
            let (warmup_completed, elapsed_loading_seconds, stable_frames, required_stable_frames) = {
                let startup = world.resource_mut::<StartupState>()?;
                let warmup_completed =
                    startup.observe_render_warm_frame(warm_frame, delta_seconds.max(0.0));
                (
                    warmup_completed,
                    startup.elapsed_loading_seconds,
                    startup.stable_frames,
                    startup.required_stable_frames,
                )
            };

            if warmup_completed {
                tracing::info!(
                    elapsed_loading_seconds,
                    stable_frames,
                    required_stable_frames,
                    warm_frame,
                    "startup warmup complete; scene flow can transition out of loading screen"
                );
            }

            if startup_ready_before
                && timing_log_enabled
                && workload_ms > FRAME_TIMING_LOG_THRESHOLD_MS
            {
                tracing::info!(
                    workload_ms = workload_ms,
                    total_ms = total_ms,
                    acquire_ms = timings.acquire_ms,
                    prepare_ui_ms = timings.renderer.prepare_ui_ms,
                    prepare_mesh_ms = timings.renderer.prepare_mesh_ms,
                    world_prepare_ms = timings.renderer.world_prepare_ms,
                    preflight_ms = timings.renderer.preflight_ms,
                    flow_encode_ms = timings.renderer.flow_encode_ms,
                    encode_submit_ms = timings.renderer.encode_submit_ms,
                    present_ms = timings.present_ms,
                    mesh_model_collect_ms = mesh_hot.model_collect_ms,
                    mesh_chunk_collect_ms = mesh_hot.chunk_collect_ms,
                    mesh_merge_filter_ms = mesh_hot.merge_filter_ms,
                    mesh_camera_update_ms = mesh_hot.camera_update_ms,
                    mesh_static_upload_ms = mesh_hot.static_upload_ms,
                    mesh_agent_upload_ms = mesh_hot.agent_upload_ms,
                    mesh_model_meshes = mesh_hot.model_meshes,
                    mesh_chunk_meshes = mesh_hot.chunk_meshes,
                    mesh_merged_meshes = mesh_hot.merged_meshes,
                    mesh_skipped_meshes = mesh_hot.skipped_meshes,
                    mesh_draw_items = mesh_hot.draw_items,
                    mesh_textured_meshes = mesh_hot.textured_meshes,
                    mesh_vertex_count = mesh_hot.vertex_count,
                    mesh_index_count = mesh_hot.index_count,
                    mesh_texture_upload_bytes = mesh_hot.texture_upload_bytes,
                    mesh_vertex_upload_bytes = mesh_hot.vertex_upload_bytes,
                    mesh_index_upload_bytes = mesh_hot.index_upload_bytes,
                    mesh_instance_upload_bytes = mesh_hot.instance_upload_bytes,
                    mesh_uniform_upload_bytes = mesh_hot.uniform_upload_bytes,
                    mesh_agent_instances = mesh_hot.agent_instances,
                    mesh_static_cache_hits = mesh_hot.static_cache_hits,
                    mesh_static_cache_misses = mesh_hot.static_cache_misses,
                    "frame render timing breakdown"
                );
            }

            if startup_ready_before
                && timing_log_enabled
                && timings.renderer.prepare_mesh_ms > MESH_HOT_PATH_LOG_THRESHOLD_MS
            {
                tracing::info!(
                    prepare_mesh_ms = timings.renderer.prepare_mesh_ms,
                    model_collect_ms = mesh_hot.model_collect_ms,
                    chunk_collect_ms = mesh_hot.chunk_collect_ms,
                    merge_filter_ms = mesh_hot.merge_filter_ms,
                    static_upload_ms = mesh_hot.static_upload_ms,
                    agent_upload_ms = mesh_hot.agent_upload_ms,
                    model_meshes = mesh_hot.model_meshes,
                    chunk_meshes = mesh_hot.chunk_meshes,
                    merged_meshes = mesh_hot.merged_meshes,
                    skipped_meshes = mesh_hot.skipped_meshes,
                    draw_items = mesh_hot.draw_items,
                    textured_meshes = mesh_hot.textured_meshes,
                    vertex_count = mesh_hot.vertex_count,
                    index_count = mesh_hot.index_count,
                    texture_upload_bytes = mesh_hot.texture_upload_bytes,
                    vertex_upload_bytes = mesh_hot.vertex_upload_bytes,
                    index_upload_bytes = mesh_hot.index_upload_bytes,
                    instance_upload_bytes = mesh_hot.instance_upload_bytes,
                    uniform_upload_bytes = mesh_hot.uniform_upload_bytes,
                    agent_instances = mesh_hot.agent_instances,
                    static_cache_hits = mesh_hot.static_cache_hits,
                    static_cache_misses = mesh_hot.static_cache_misses,
                    "mesh prepare hot path breakdown"
                );
            }

            gfx.renderer.clear_published_gpu_observations();
            Ok(())
        }
        Err(err) => {
            if let Some(surface_error) = err.downcast_ref::<RenderSurfaceAcquireError>() {
                match surface_error {
                    RenderSurfaceAcquireError::Lost | RenderSurfaceAcquireError::Outdated => {
                        gfx.resize(render_surface_id, target_w, target_h);
                        Ok(())
                    }
                    RenderSurfaceAcquireError::Timeout | RenderSurfaceAcquireError::Validation => {
                        Ok(())
                    }
                }
            } else {
                Err(anyhow!("render backend execution failed: {err:#}"))
            }
        }
    };

    let result = result.and_then(|_| {
        render_additional_surfaces(
            &mut world,
            &additional_prepared_frames,
            &mut gfx,
            &mut shader_registry,
            &ui_font_atlas,
            preflight_config,
            &debug_control,
            &debug_config,
        )
    });

    world.insert_resource(shader_registry);
    world.insert_resource(gfx);
    result
}

#[allow(clippy::too_many_arguments)]
fn render_additional_surfaces(
    world: &mut WorldMut,
    prepared_frames: &[PreparedRenderFrame],
    gfx: &mut Gfx,
    shader_registry: &mut ShaderRegistryResource,
    ui_font_atlas: &UiFontAtlasResource,
    preflight_config: RenderPreflightValidationConfigResource,
    debug_control: &RenderDebugControlResource,
    debug_config: &RenderDebugConfigResource,
) -> anyhow::Result<()> {
    for prepared_frame in prepared_frames {
        validate_prepared_frame_surface_scope(world, prepared_frame)?;
        let render_surface_id = prepared_frame.surface.render_surface_id;
        if !gfx.has_surface(render_surface_id) {
            if let Ok(registry) = world.resource_mut::<RenderSurfaceRegistryResource>() {
                registry.record_diagnostic(RenderSurfaceDiagnostic {
                    render_surface_id: Some(render_surface_id),
                    native_window_id: prepared_frame.surface.native_window_id,
                    message: format!(
                        "prepared frame {} skipped because render surface {} is not attached",
                        prepared_frame.context.frame_index,
                        render_surface_id.raw()
                    ),
                });
            }
            continue;
        }

        let (target_w, target_h) = prepared_frame
            .main_view()
            .ok_or_else(|| anyhow!("prepared render frame is missing a main surface view"))?
            .target_size_px;
        let surface_size = gfx.surface_size(render_surface_id);
        if surface_size != Some((target_w, target_h)) {
            gfx.resize(render_surface_id, target_w, target_h);
        }

        let flow_registry = world
            .resource::<RenderFlowRegistryResource>()
            .map_err(|_| anyhow!("render flow registry is unavailable"))?;
        if flow_registry.revision() != prepared_frame.context.flow_registry_revision {
            continue;
        }
        let ui_rect_shader = prepared_frame
            .ui()
            .and_then(|ui| ui.first_rect_shader_asset_id())
            .and_then(|id| shader_registry.handle(id));
        let render_result = gfx.render(
            prepared_frame,
            shader_registry,
            flow_registry.compiled_flows(),
            ui_rect_shader,
            ui_font_atlas,
            &prepared_frame.viewport_surface_bindings,
            preflight_config,
            debug_control,
            debug_config,
        );
        if let Err(err) = render_result {
            if let Some(surface_error) = err.downcast_ref::<RenderSurfaceAcquireError>() {
                match surface_error {
                    RenderSurfaceAcquireError::Lost | RenderSurfaceAcquireError::Outdated => {
                        gfx.resize(render_surface_id, target_w, target_h);
                    }
                    RenderSurfaceAcquireError::Timeout | RenderSurfaceAcquireError::Validation => {}
                }
            } else {
                return Err(anyhow!(
                    "render backend execution failed for surface {}: {err:#}",
                    render_surface_id.raw()
                ));
            }
        }
    }
    Ok(())
}

fn should_build_full_render_diagnostics(
    policy: RenderFrameDiagnosticsPolicyResource,
    debug_control: &RenderDebugControlResource,
    debug_config: &RenderDebugConfigResource,
    workload_ms: f32,
) -> bool {
    if policy.mode == RenderFrameDiagnosticsMode::FullEveryFrame || policy.force_next_full_report {
        return true;
    }
    if debug_control.provenance_enabled
        || debug_control.capture_enabled
        || debug_control.readback_enabled
        || debug_control.artifact_export_enabled
    {
        return true;
    }
    if !debug_config.capture_selectors.is_empty()
        || !debug_config.pixel_probes.is_empty()
        || !debug_config.texture_diffs.is_empty()
    {
        return true;
    }
    workload_ms > policy.slow_frame_threshold_ms
}

fn validate_prepared_frame_surface_scope(
    world: &mut WorldMut,
    prepared_frame: &PreparedRenderFrame,
) -> anyhow::Result<()> {
    let Ok(registry) = world.resource_mut::<RenderSurfaceRegistryResource>() else {
        return Ok(());
    };
    let Some(record) = registry.record(prepared_frame.surface.render_surface_id) else {
        let message = format!(
            "prepared frame {} targets unknown render surface {}",
            prepared_frame.context.frame_index,
            prepared_frame.surface.render_surface_id.raw()
        );
        registry.record_diagnostic(RenderSurfaceDiagnostic {
            render_surface_id: Some(prepared_frame.surface.render_surface_id),
            native_window_id: prepared_frame.surface.native_window_id,
            message: message.clone(),
        });
        anyhow::bail!(message);
    };
    let registered_native_window_id = record.native_window_id;
    if prepared_frame.surface.native_window_id != Some(registered_native_window_id) {
        let message = format!(
            "prepared frame {} targets render surface {} for native window {:?}, but the registry owns native window {:?}",
            prepared_frame.context.frame_index,
            prepared_frame.surface.render_surface_id.raw(),
            prepared_frame.surface.native_window_id,
            registered_native_window_id
        );
        registry.record_diagnostic(RenderSurfaceDiagnostic {
            render_surface_id: Some(prepared_frame.surface.render_surface_id),
            native_window_id: prepared_frame.surface.native_window_id,
            message: message.clone(),
        });
        anyhow::bail!(message);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugins::render::inspect::{RenderCaptureSelector, RenderPixelProbeRequest};

    #[test]
    fn render_diagnostics_tier_skips_full_report_in_healthy_steady_state() {
        let policy = RenderFrameDiagnosticsPolicyResource::default();
        let control = RenderDebugControlResource {
            provenance_enabled: false,
            capture_enabled: false,
            readback_enabled: false,
            artifact_export_enabled: false,
            ..RenderDebugControlResource::default()
        };
        let config = RenderDebugConfigResource::default();

        assert!(!should_build_full_render_diagnostics(
            policy, &control, &config, 2.0
        ));
    }

    #[test]
    fn render_diagnostics_tier_forces_full_report_for_capture_work() {
        let policy = RenderFrameDiagnosticsPolicyResource::default();
        let control = RenderDebugControlResource {
            capture_enabled: true,
            ..RenderDebugControlResource::default()
        };

        assert!(should_build_full_render_diagnostics(
            policy,
            &control,
            &RenderDebugConfigResource::default(),
            2.0
        ));
    }

    #[test]
    fn render_diagnostics_tier_forces_full_report_for_debug_requests() {
        let policy = RenderFrameDiagnosticsPolicyResource::default();
        let control = RenderDebugControlResource {
            provenance_enabled: false,
            capture_enabled: false,
            readback_enabled: false,
            artifact_export_enabled: false,
            ..RenderDebugControlResource::default()
        };
        let mut config = RenderDebugConfigResource::default();
        config.pixel_probes.push(RenderPixelProbeRequest::center(
            "probe",
            RenderCaptureSelector::named_pass_surface_color("flow", "pass"),
        ));

        assert!(should_build_full_render_diagnostics(
            policy, &control, &config, 2.0
        ));
    }

    #[test]
    fn render_diagnostics_tier_forces_full_report_for_slow_frames() {
        let policy = RenderFrameDiagnosticsPolicyResource {
            slow_frame_threshold_ms: 10.0,
            ..RenderFrameDiagnosticsPolicyResource::default()
        };
        let control = RenderDebugControlResource {
            provenance_enabled: false,
            capture_enabled: false,
            readback_enabled: false,
            artifact_export_enabled: false,
            ..RenderDebugControlResource::default()
        };

        assert!(should_build_full_render_diagnostics(
            policy,
            &control,
            &RenderDebugConfigResource::default(),
            10.1
        ));
    }
}
