use criterion::{Criterion, black_box, criterion_group, criterion_main};
use engine::plugins::render::features::world::sdf_raymarch::{
    RenderSdfDistanceMipLevel, RenderSdfRaymarchAccelerationReport, RenderSdfRaymarchCandidate,
    RenderSdfRaymarchCandidateList,
};
use engine::plugins::render::graph::{CompiledPassExecutionPlan, CompiledRenderFlowPlan};
use engine::plugins::render::inspect::{
    PassTimingSample, RenderDebugTimingsState, RenderGpuResidencyBudgetInspection,
    RenderGpuResidencyInspection, RenderGpuTimingCapability, RenderGpuTimingDiagnostic,
    RenderMeshMaterialHandoffCounts, RenderMeshMaterialHandoffInspection,
    RenderMeshMaterialProductionEvidenceRequest, RenderMeshMaterialProductionHardwareProfile,
    RenderMeshMaterialRuntimeVisualEvidence, RenderPassTimingEvidence,
    RenderPipelineFallbackCounts, RenderPipelineFallbackInspection,
    RenderRayReconstructionInputEvidence, RenderRayReconstructionInputKind,
    RenderScaleProductionEvidenceRequest, RenderScaleProductionHardwareProfile,
    RenderScaleVisibilityCandidate, RenderScaleVisibilityCapabilities,
    RenderScaleVisibilityCapabilityStatus, RenderSdfProductionEvidenceRequest,
    RenderSdfProductionHardwareProfile, RenderSdfResidencyBudgetInspection,
    RenderSdfResidencyInspection, RenderSdfRuntimeVisualEvidence, RenderTemporalHistoryEvidence,
    RenderTemporalInputEvidence, RenderTemporalInputKind, RenderTemporalInspection,
    RenderTemporalInspectionRequest, RenderTemporalJitterEvidence,
    RenderTemporalProductionEvidenceRequest, RenderTemporalProductionHardwareProfile,
    RenderTemporalReconstructionMode, RenderTemporalResolutionEvidence,
    RenderTemporalRuntimeVisualEvidence, RenderTemporalUpscalingAdapterEvidence,
    RenderTemporalUpscalingAdapterKind, RenderTemporalUpscalingCapabilityState,
    RenderTemporalUpscalingInspection, RenderTemporalUpscalingInspectionRequest,
    inspect_render_mesh_material_production_evidence, inspect_render_scale_production_evidence,
    inspect_render_scale_visibility, inspect_render_sdf_production_evidence,
    inspect_render_temporal_inputs, inspect_render_temporal_production_evidence,
    inspect_render_temporal_upscaling,
};
use engine::plugins::render::{
    BoundedUniformGrid2dBuildPlan, BoundedUniformGrid2dConfig, BoundedUniformGrid2dStage,
    DrawIndirectArgs, GpuPrimitiveExecutionPlan, GpuPrimitiveStep, GpuStorage, GpuUniform,
    IndirectDrawArgsGenerationDescriptor, PrefixScanMode, PreparedFlowInputs,
    PreparedFlowInvocation, PreparedFrameContext, PreparedFrameContributions, PreparedRenderFrame,
    PreparedShaderSnapshot, PreparedSurfaceInfo, PreparedViewFrame, ProceduralBufferBinding,
    ProceduralPassDescriptor, ProceduralTargetDescriptor, RenderExecutionGraphPreparedReport,
    RenderFlow, RenderPreparedFramePreflightCacheKey, RenderVertexBufferLayout, RenderVertexFormat,
    U32Counter, U32PrefixScanDescriptor, U32ScanElement, U32ScatterDescriptor, compile_flow_plan,
    current_runtime_gpu_capabilities, preflight_prepared_render_frame,
    preflight_prepared_render_frame_runtime_guards, prepared_render_frame_preflight_cache_key,
};
use engine::prelude::Resource;
use runen_gpu::GpuBindingKey;
use ui_render_data::ViewportSurfaceBindingRegistry;

#[derive(Debug, Clone, Copy, GpuStorage)]
struct BoidInstance {
    position: [f32; 4],
}

#[derive(Debug, Clone, Copy, GpuStorage)]
struct ProceduralBoidInstance {
    position: [f32; 2],
    velocity: [f32; 2],
    visual_heading: [f32; 2],
}

#[derive(Debug, Clone, Copy, GpuUniform)]
struct ComputeParams {
    tick: u32,
    step: u32,
}

#[derive(Debug, Clone, Copy, GpuUniform)]
struct ComposeParams {
    surface_size: [u32; 2],
    intensity: f32,
}

#[derive(Debug, Clone, Copy, GpuUniform)]
struct ProceduralBoidsDrawParams {
    surface: [f32; 4],
    sprite: [f32; 4],
}

const BENCH_BOID_COUNT: u32 = 4096;
const BENCH_GRID_CELLS_X: u32 = 64;
const BENCH_GRID_CELLS_Y: u32 = 64;
const BENCH_GRID_CELL_COUNT: u32 = BENCH_GRID_CELLS_X * BENCH_GRID_CELLS_Y;

fn binding_key(binding: u64) -> GpuBindingKey {
    GpuBindingKey::try_new(0, binding).expect("benchmark binding should fit GpuBindingKey")
}

#[derive(Debug, Clone, Resource)]
struct BenchState {
    tick: u32,
    dispatch: [u32; 3],
}

impl Default for BenchState {
    fn default() -> Self {
        Self {
            tick: 1,
            dispatch: [16, 16, 1],
        }
    }
}

impl BenchState {
    fn compute_params(&self) -> ComputeParams {
        ComputeParams {
            tick: self.tick,
            step: 1,
        }
    }

    fn compose_params(&self, surface: (u32, u32)) -> ComposeParams {
        ComposeParams {
            surface_size: [surface.0, surface.1],
            intensity: 1.0,
        }
    }

    fn procedural_boids_draw_params(&self, surface: (u32, u32)) -> ProceduralBoidsDrawParams {
        let width = surface.0.max(1) as f32;
        let height = surface.1.max(1) as f32;
        ProceduralBoidsDrawParams {
            surface: [width, height, 1.0 / width, 1.0 / height],
            sprite: [10.5, 0.72, 1.35, 0.0],
        }
    }

    fn dispatch_workgroups(&self) -> [u32; 3] {
        self.dispatch
    }

    fn dispatch_boids_workgroups(&self) -> [u32; 3] {
        [BENCH_BOID_COUNT.div_ceil(64), 1, 1]
    }

    fn dispatch_grid_workgroups(&self) -> [u32; 3] {
        [BENCH_GRID_CELL_COUNT.div_ceil(64), 1, 1]
    }

    fn dispatch_scan_workgroups(&self) -> [u32; 3] {
        [1, 1, 1]
    }
}

fn run_validation_and_planning(flow: &RenderFlow) {
    let report = flow.validation_report().expect("flow should validate");
    let compiled = compile_flow_plan(flow).expect("flow should compile through render planning");
    black_box(report.lexical_pass_ids.len());
    black_box(compiled.render_passes.len());
    black_box(compiled.execution.passes.len());
    black_box(compiled.resources.resources.len());
}

fn prepared_inputs_for_flow(compiled: &CompiledRenderFlowPlan) -> PreparedFlowInputs {
    let mut inputs = PreparedFlowInputs::default();
    for pass in &compiled.execution.passes {
        match pass {
            CompiledPassExecutionPlan::Compute(value) => {
                for uniform_id in &value.bindings.uniform_order {
                    inputs
                        .projected_uniform_bytes
                        .insert(*uniform_id, vec![0; 16]);
                }
                if value.dispatch.is_some() {
                    inputs
                        .projected_dispatch_workgroups
                        .insert(value.pass_id, [16, 16, 1]);
                }
            }
            CompiledPassExecutionPlan::Fullscreen(value)
            | CompiledPassExecutionPlan::Graphics(value) => {
                for uniform_id in &value.bindings.uniform_order {
                    inputs
                        .projected_uniform_bytes
                        .insert(*uniform_id, vec![0; 16]);
                }
            }
            CompiledPassExecutionPlan::Copy(_)
            | CompiledPassExecutionPlan::Present(_)
            | CompiledPassExecutionPlan::BuiltinUiComposite(_) => {}
        }
    }
    inputs
}

fn prepared_frame_for_flow(compiled: &CompiledRenderFlowPlan) -> PreparedRenderFrame {
    PreparedRenderFrame {
        context: PreparedFrameContext {
            frame_index: 1,
            flow_registry_revision: 1,
            shader_registry_revision: 1,
            prepare_epoch: 1,
        },
        surface: PreparedSurfaceInfo::primary((1280, 720)),
        views: vec![PreparedViewFrame::main((1280, 720))],
        flows: Default::default(),
        flow_invocations: vec![PreparedFlowInvocation::main(
            compiled.flow_id,
            prepared_inputs_for_flow(compiled),
        )],
        dynamic_texture_targets: Vec::new(),
        dynamic_texture_uploads: Vec::new(),
        product_selections: Vec::new(),
        viewport_surface_bindings: ViewportSurfaceBindingRegistry::default(),
        contributions: PreparedFrameContributions::default(),
        shader: PreparedShaderSnapshot {
            registry_revision: 1,
        },
    }
}

fn run_cold_preflight(compiled: &CompiledRenderFlowPlan, frame: &PreparedRenderFrame) {
    let profile = current_runtime_gpu_capabilities();
    let key =
        prepared_render_frame_preflight_cache_key(frame, std::slice::from_ref(compiled), &profile);
    preflight_prepared_render_frame_runtime_guards(frame, std::slice::from_ref(compiled))
        .expect("runtime guards should pass");
    let report = preflight_prepared_render_frame(frame, std::slice::from_ref(compiled), &profile)
        .expect("prepared frame should preflight");
    black_box(key.prepared_structure_hash);
    black_box(report.error_count());
}

fn run_warm_cached_preflight(
    compiled: &CompiledRenderFlowPlan,
    frame: &PreparedRenderFrame,
    cached_key: &RenderPreparedFramePreflightCacheKey,
    cached_report: &RenderExecutionGraphPreparedReport,
) {
    let profile = current_runtime_gpu_capabilities();
    preflight_prepared_render_frame_runtime_guards(frame, std::slice::from_ref(compiled))
        .expect("runtime guards should pass");
    let key =
        prepared_render_frame_preflight_cache_key(frame, std::slice::from_ref(compiled), &profile);
    if &key == cached_key {
        black_box(cached_report.error_count());
    } else {
        run_cold_preflight(compiled, frame);
    }
}

fn run_prefix_scan_primitive_plan(element_count: u32) {
    let (flow, input) = RenderFlow::new("bench.population.scan")
        .storage_array::<U32ScanElement>("bench.population.scan.input", u64::from(element_count))
        .expect("render flow authoring should succeed");
    let (flow, output) = flow
        .storage_array::<U32ScanElement>("bench.population.scan.output", u64::from(element_count))
        .expect("render flow authoring should succeed");
    let scan = U32PrefixScanDescriptor::new(
        "bench.population.scan",
        input,
        output,
        element_count,
        PrefixScanMode::Exclusive,
    )
    .expect("valid prefix scan descriptor");
    let plan = GpuPrimitiveExecutionPlan::new(
        "bench.population.scan.plan",
        [GpuPrimitiveStep::from(scan)],
    )
    .expect("valid scan primitive plan");

    black_box(plan.step_count());
    let flow = flow
        .gpu_primitive_plan(&plan)
        .expect("valid primitive dispatch should append to flow")
        .validate()
        .expect("primitive dispatch flow should validate");
    let compiled = compile_flow_plan(&flow).expect("primitive dispatch flow should compile");
    black_box(compiled.execution.passes.len());
}

fn run_scan_compaction_indirect_args_plan(element_count: u32) {
    let (flow, source_indices) = RenderFlow::new("bench.population.primitives")
        .storage_array::<U32ScanElement>(
            "bench.population.primitives.source_indices",
            u64::from(element_count),
        )
        .expect("render flow authoring should succeed");
    let (flow, prefix_offsets) = flow
        .storage_array::<U32ScanElement>(
            "bench.population.primitives.prefix_offsets",
            u64::from(element_count),
        )
        .expect("render flow authoring should succeed");
    let (flow, output_indices) = flow
        .storage_array::<U32ScanElement>(
            "bench.population.primitives.output_indices",
            u64::from(element_count),
        )
        .expect("render flow authoring should succeed");
    let (flow, draw_args) = flow
        .storage_array::<DrawIndirectArgs>("bench.population.primitives.draw_args", 1)
        .expect("render flow authoring should succeed");

    let scan = U32PrefixScanDescriptor::new(
        "bench.population.primitives.scan",
        source_indices.clone(),
        prefix_offsets.clone(),
        element_count,
        PrefixScanMode::Exclusive,
    )
    .expect("valid scan descriptor");
    let scatter = U32ScatterDescriptor::new(
        "bench.population.primitives.scatter",
        source_indices,
        prefix_offsets,
        output_indices,
        element_count,
        element_count,
    )
    .expect("valid scatter descriptor");
    let args = IndirectDrawArgsGenerationDescriptor::draw(
        "bench.population.primitives.draw_args",
        draw_args,
        0,
        DrawIndirectArgs::new(6, element_count, 0, 0),
    )
    .expect("valid indirect draw args descriptor");
    let plan = GpuPrimitiveExecutionPlan::new(
        "bench.population.primitives.plan",
        [
            GpuPrimitiveStep::from(scan),
            GpuPrimitiveStep::from(scatter),
            GpuPrimitiveStep::from(args),
        ],
    )
    .expect("valid primitive execution plan");

    black_box(plan.step_count());
    let flow = flow
        .gpu_primitive_plan(&plan)
        .expect("valid primitive dispatch should append to flow")
        .validate()
        .expect("primitive dispatch flow should validate");
    let compiled = compile_flow_plan(&flow).expect("primitive dispatch flow should compile");
    black_box(compiled.execution.passes.len());
}

fn run_bounded_grid_build_plan(agent_count: u32) {
    let config =
        BoundedUniformGrid2dConfig::new(BENCH_GRID_CELLS_X, BENCH_GRID_CELLS_Y, agent_count);
    let cell_count = config
        .checked_cell_count()
        .expect("benchmark grid dimensions should validate");
    let (flow, counts) = RenderFlow::new("bench.population.grid")
        .storage_array::<U32Counter>("bench.population.grid.counts", u64::from(cell_count))
        .expect("render flow authoring should succeed");
    let (flow, offsets) = flow
        .storage_array::<U32ScanElement>("bench.population.grid.offsets", u64::from(cell_count))
        .expect("render flow authoring should succeed");
    let (flow, cursors) = flow
        .storage_array::<U32Counter>("bench.population.grid.cursors", u64::from(cell_count))
        .expect("render flow authoring should succeed");
    let (_flow, sorted) = flow
        .storage_array::<U32ScanElement>("bench.population.grid.sorted", u64::from(agent_count))
        .expect("render flow authoring should succeed");
    let plan = BoundedUniformGrid2dBuildPlan::new(
        "bench.population.grid",
        config,
        counts,
        offsets,
        cursors,
        sorted,
    )
    .expect("valid grid build plan");

    black_box(plan.config.cell_count());
    black_box(plan.resources.sorted_index_capacity);
    black_box(plan.primitive_plan.step_count());
    black_box(plan.stages.len());
}

fn run_boids_production_evidence_report(flow: &RenderFlow) {
    let compiled = compile_flow_plan(flow).expect("procedural boids flow should compile");
    let pass_count = compiled.execution.passes.len();
    let grid_stage_count = compiled
        .render_passes
        .iter()
        .filter(|pass| {
            pass.pass_label()
                .starts_with("bench.procedural_boids.grid.")
        })
        .count();
    let local_instance_draw = compiled.execution.passes.iter().any(|pass| match pass {
        CompiledPassExecutionPlan::Graphics(value) => {
            !value.draw_buffers.instance_buffers.is_empty()
                && value.draw.is_some_and(|draw| {
                    draw.vertex_count == 6 && draw.instance_count == BENCH_BOID_COUNT
                })
        }
        _ => false,
    });
    let max_aspect_error_px = [(1600, 900), (900, 1600), (1024, 1024)]
        .into_iter()
        .map(boids_resize_aspect_error_px)
        .fold(0.0_f32, f32::max);
    let evidence = format!(
        "boids_population_bench_evidence flow={} passes={} grid_stages={} local_instance_draw={} no_silent_grid_overflow=true max_aspect_error_px={:.5} benchmark_command=cargo bench -p engine --bench render_flow_planning",
        compiled.flow_label, pass_count, grid_stage_count, local_instance_draw, max_aspect_error_px,
    );

    black_box(evidence.len());
}

fn boids_resize_aspect_error_px(surface_size: (u32, u32)) -> f32 {
    let width = surface_size.0.max(1) as f32;
    let height = surface_size.1.max(1) as f32;
    let radius_px = 10.5;
    let sprite_width_px = radius_px * 0.72 * 2.0;
    let sprite_height_px = radius_px * 1.35 * 2.0;
    let clip_width = sprite_width_px * 2.0 / width;
    let clip_height = sprite_height_px * 2.0 / height;
    let reconstructed_width_px = clip_width * width * 0.5;
    let reconstructed_height_px = clip_height * height * 0.5;

    (reconstructed_width_px - sprite_width_px)
        .abs()
        .max((reconstructed_height_px - sprite_height_px).abs())
}

fn run_mesh_material_production_evidence() {
    let material_handoff = RenderMeshMaterialHandoffInspection {
        counts: RenderMeshMaterialHandoffCounts {
            material_instance_count: 1,
            texture_binding_count: 1,
            material_binding_slot_count: 1,
            model_mesh_selection_count: 1,
            material_consuming_pass_count: 1,
            pass_exposed_model_mesh_selection_count: 1,
        },
        scene_shader_identity: Some("shader.identity.scene".to_string()),
        scene_shader_path: Some("generated/scene_material.wgsl".to_string()),
        shader_artifact_id: Some("shader.artifact.scene".to_string()),
        shader_cache_key: Some("shader.cache.scene".to_string()),
        material_table_identity: Some("scene.material.table:v1".to_string()),
        resource_layout_identity: Some("resource.layout:v1".to_string()),
        diagnostics: Vec::new(),
    };
    let pipeline_fallback = RenderPipelineFallbackInspection {
        counts: RenderPipelineFallbackCounts {
            pass_count: 1,
            pipeline_backed_pass_count: 1,
            material_pass_count: 1,
            fallback_pass_count: 0,
            material_fallback_pass_count: 0,
            shader_failure_event_count: 1,
            prior_valid_shader_failure_count: 1,
            pipeline_cache_hit_count: 2,
            pipeline_cache_miss_count: 1,
            pipeline_cache_failure_count: 0,
        },
        shader_reload_status: None,
        passes: Vec::new(),
        shader_failures: Vec::new(),
        diagnostics: Vec::new(),
    };
    let mut timings = RenderDebugTimingsState::default();
    timings.observe_pass_timings(&[
        PassTimingSample {
            flow_id: "bench.mesh.material.production".to_string(),
            pass_id: "mesh.material.prepare".to_string(),
            pass_kind: "compute".to_string(),
            millis: 0.12,
            dispatch_workgroups: Some([1, 1, 1]),
        },
        PassTimingSample {
            flow_id: "bench.mesh.material.production".to_string(),
            pass_id: "mesh.material.draw".to_string(),
            pass_kind: "graphics".to_string(),
            millis: 0.31,
            dispatch_workgroups: None,
        },
    ]);
    timings.observe_gpu_timing_diagnostic(RenderGpuTimingDiagnostic::unsupported(
        "criterion mesh/material production evidence uses portable unsupported timestamp diagnostics",
    ));
    let report =
        inspect_render_mesh_material_production_evidence(RenderMeshMaterialProductionEvidenceRequest {
            hardware_profile: RenderMeshMaterialProductionHardwareProfile {
                profile_key: "bench-mesh-material-profile".to_string(),
                adapter_name: Some("criterion".to_string()),
                backend: Some("wgpu".to_string()),
                timestamp_query: RenderGpuTimingCapability::Unsupported,
            },
            material_handoff,
            pipeline_fallback,
            timings,
            visual_evidence: vec![RenderMeshMaterialRuntimeVisualEvidence {
                view_label: "bench.mesh.material.summary".to_string(),
                artifact_path:
                    "engine/benchmark-artifacts/render-mesh-material-production-evidence/summary.txt"
                        .to_string(),
                material_table_identity: "scene.material.table:v1".to_string(),
                scene_shader_identity: "shader.identity.scene".to_string(),
                material_instance_count: 1,
                rendered_pixel_count: 4096,
                consumed_material_handoff: true,
                consumed_pipeline_fallback: true,
            }],
            benchmark_commands: vec![
                "cargo bench -p engine --bench render_flow_planning".to_string(),
            ],
            artifact_paths: vec![
                "engine/benchmark-artifacts/render-mesh-material-production-evidence/summary.txt"
                    .to_string(),
                "docs-site/src/content/docs/reports/benchmarks/render/mesh-material-production-evidence.md"
                    .to_string(),
            ],
        });
    black_box(report.is_runtime_ready());
    black_box(report.counts.material_instance_count);
    black_box(report.timings.gpu_timing_diagnostic_count);
}

fn run_scale_production_evidence(candidate_count: usize) {
    let residency = RenderGpuResidencyInspection {
        addressable_count: 1_000_000,
        selected_count: candidate_count,
        requested_count: candidate_count,
        accepted_count: candidate_count,
        resident_count: candidate_count,
        allocated_count: candidate_count,
        preserved_count: 0,
        invalidated_count: 0,
        evicted_count: 0,
        rejected_count: 0,
        resident_bytes: candidate_count as u64 * 4096,
        upload_bytes: candidate_count.min(4096) as u64 * 1024,
        budget: RenderGpuResidencyBudgetInspection {
            max_resident_entries: 1_000_000,
            max_resident_bytes: 4_u64 * 1024 * 1024 * 1024,
            max_upload_bytes_per_frame: 128_u64 * 1024 * 1024,
            resident_entry_status: "within_budget".to_string(),
            resident_byte_status: "within_budget".to_string(),
            upload_byte_status: "within_budget".to_string(),
            hard_pinned_over_entry_budget: false,
        },
        diagnostic_count: 0,
        entries: Vec::new(),
        journal: Vec::new(),
    };
    let candidates = (0..candidate_count)
        .map(|index| RenderScaleVisibilityCandidate {
            product_id: index as u64,
            cache_id: format!("scale.chunk.{index}"),
            center: [((index % 64) as f32 / 64.0) - 0.5, 0.0, 0.0],
            radius: 0.01,
            screen_size_px: if index % 4 == 0 { 0.25 } else { 64.0 },
            resident_bytes: 4096,
        })
        .collect::<Vec<_>>();
    let visibility = inspect_render_scale_visibility(
        &candidates,
        Default::default(),
        RenderScaleVisibilityCapabilities::supported(),
    );
    let mut timings = RenderDebugTimingsState::default();
    timings.observe_pass_timings(&[PassTimingSample {
        flow_id: "bench.scale".to_string(),
        pass_id: "scale.visibility".to_string(),
        pass_kind: "compute".to_string(),
        millis: 0.5,
        dispatch_workgroups: Some([64, 1, 1]),
    }]);
    timings.observe_gpu_pass_timing_evidence(&[RenderPassTimingEvidence::gpu_sample(
        Some(1),
        Some(1),
        "bench.scale",
        "scale.visibility",
        "compute",
        0.45,
    )]);
    let report = inspect_render_scale_production_evidence(RenderScaleProductionEvidenceRequest {
        hardware_profile: RenderScaleProductionHardwareProfile {
            profile_key: "bench-scale-profile".to_string(),
            adapter_name: Some("criterion".to_string()),
            backend: Some("wgpu".to_string()),
            timestamp_query: RenderGpuTimingCapability::Supported,
            storage_compaction: RenderScaleVisibilityCapabilityStatus::Supported,
            indirect_submission: RenderScaleVisibilityCapabilityStatus::Supported,
            readback: RenderScaleVisibilityCapabilityStatus::Supported,
        },
        residency,
        visibility,
        timings,
        benchmark_commands: vec!["cargo bench -p engine --bench render_flow_planning".to_string()],
        artifact_paths: vec![
            "engine/benchmark-artifacts/render-scale-evidence/criterion.md".to_string(),
        ],
    });
    black_box(report.is_runtime_ready());
    black_box(report.counts.submitted_draw_count);
    black_box(report.timings.gpu_total_pass_millis);
}

fn run_sdf_runtime_evidence(product_count: usize) {
    let residency = RenderSdfResidencyInspection {
        addressable_product_count: product_count,
        selected_product_count: product_count,
        requested_product_count: product_count,
        resident_product_count: product_count,
        resident_page_count: product_count * 2,
        resident_brick_count: product_count * 2,
        clipmap_window_count: product_count,
        invalidated_product_count: 0,
        rejected_product_count: 0,
        resident_bytes: product_count as u64 * 8192,
        upload_bytes: product_count as u64 * 2048,
        budget: RenderSdfResidencyBudgetInspection {
            page_status: "within_budget".to_string(),
            brick_status: "within_budget".to_string(),
            resident_byte_status: "within_budget".to_string(),
            upload_byte_status: "within_budget".to_string(),
            clipmap_page_status: "within_budget".to_string(),
        },
        diagnostic_count: 0,
        diagnostics: Vec::new(),
        entries: Vec::new(),
        clipmap_windows: Vec::new(),
    };
    let candidates = (0..product_count)
        .map(|index| RenderSdfRaymarchCandidate {
            product_id: index as u64,
            cache_generation: index as u64 + 1,
            scale_band: match index % 4 {
                0 => "Near",
                1 => "Mid",
                2 => "Far",
                _ => "Summary",
            }
            .to_string(),
            page_count: 2,
            brick_count: 2,
            resident_bytes: 8192,
        })
        .collect::<Vec<_>>();
    let raymarch = RenderSdfRaymarchAccelerationReport {
        resident_product_count: product_count,
        resident_page_count: product_count * 2,
        resident_brick_count: product_count * 2,
        clipmap_window_count: product_count,
        distance_mips: (0..product_count.min(8))
            .map(|level| RenderSdfDistanceMipLevel {
                level: u8::try_from(level).expect("bench mip level fits in u8"),
                source_page_count: 2,
                source_brick_count: 2,
                conservative_min_distance: 0.0,
                max_safe_step: 1.0 / (level as f32 + 1.0),
                unsafe_overstep_risk: false,
            })
            .collect(),
        candidate_lists: vec![RenderSdfRaymarchCandidateList {
            tile_index: 0,
            depth_slice: 0,
            candidate_count: candidates.len(),
            rejected_candidate_count: 0,
            candidates,
        }],
        total_candidate_count: product_count,
        rejected_candidate_count: 0,
        max_candidates_per_list: product_count.max(1),
        max_steps_per_ray: 160,
        fullscreen_entity_multiplier: 1,
        diagnostics: Vec::new(),
    };
    let mut timings = RenderDebugTimingsState::default();
    timings.observe_pass_timings(&[PassTimingSample {
        flow_id: "bench.sdf.runtime".to_string(),
        pass_id: "sdf.compose".to_string(),
        pass_kind: "fullscreen".to_string(),
        millis: 0.5,
        dispatch_workgroups: None,
    }]);
    timings.observe_gpu_timing_diagnostic(RenderGpuTimingDiagnostic::unsupported(
        "criterion SDF runtime evidence uses portable unsupported timestamp diagnostics",
    ));
    let visual_evidence = ["near", "mid", "far", "summary"]
        .into_iter()
        .map(|coverage_band| RenderSdfRuntimeVisualEvidence {
            view_label: format!("bench.{coverage_band}"),
            coverage_band: coverage_band.to_string(),
            artifact_path: format!(
                "engine/benchmark-artifacts/render-sdf-runtime-evidence/{coverage_band}.txt"
            ),
            step_count: 64,
            missed_surface_risk: false,
            overstep_risk: false,
        })
        .collect::<Vec<_>>();
    let report = inspect_render_sdf_production_evidence(RenderSdfProductionEvidenceRequest {
        hardware_profile: RenderSdfProductionHardwareProfile {
            profile_key: "bench-sdf-runtime-profile".to_string(),
            adapter_name: Some("criterion".to_string()),
            backend: Some("wgpu".to_string()),
            timestamp_query: RenderGpuTimingCapability::Unsupported,
        },
        residency,
        raymarch,
        timings,
        visual_evidence,
        benchmark_commands: vec!["cargo bench -p engine --bench render_flow_planning".to_string()],
        artifact_paths: vec![
            "engine/benchmark-artifacts/render-sdf-runtime-evidence/criterion.md".to_string(),
        ],
    });
    black_box(report.is_runtime_ready());
    black_box(report.counts.total_candidate_count);
    black_box(report.timings.cpu_total_pass_millis);
}

fn run_temporal_production_evidence() {
    let temporal = temporal_inspection();
    let upscaling = temporal_upscaling_inspection(temporal.clone());
    let mut timings = RenderDebugTimingsState::default();
    timings.observe_pass_timings(&[
        PassTimingSample {
            flow_id: "bench.temporal.production".to_string(),
            pass_id: "temporal.reconstruct".to_string(),
            pass_kind: "fullscreen".to_string(),
            millis: 0.24,
            dispatch_workgroups: None,
        },
        PassTimingSample {
            flow_id: "bench.temporal.production".to_string(),
            pass_id: "temporal.resolve".to_string(),
            pass_kind: "fullscreen".to_string(),
            millis: 0.18,
            dispatch_workgroups: None,
        },
    ]);
    timings.observe_gpu_timing_diagnostic(RenderGpuTimingDiagnostic::unsupported(
        "criterion temporal production evidence uses portable unsupported timestamp diagnostics",
    ));
    let report =
        inspect_render_temporal_production_evidence(RenderTemporalProductionEvidenceRequest {
            hardware_profile: RenderTemporalProductionHardwareProfile {
                profile_key: "bench-temporal-production-profile".to_string(),
                adapter_name: Some("criterion".to_string()),
                backend: Some("wgpu".to_string()),
                timestamp_query: RenderGpuTimingCapability::Unsupported,
            },
            temporal,
            upscaling,
            timings,
            visual_evidence: temporal_visual_evidence(),
            benchmark_commands: vec![
                "cargo bench -p engine --bench render_flow_planning".to_string(),
            ],
            artifact_paths: vec![
                "engine/benchmark-artifacts/render-temporal-production-evidence/summary.txt"
                    .to_string(),
                "docs-site/src/content/docs/reports/benchmarks/render/temporal-production-evidence.md"
                    .to_string(),
            ],
        });
    black_box(report.is_runtime_ready());
    black_box(report.counts.rendered_pixel_count);
    black_box(report.timings.gpu_timing_diagnostic_count);
}

fn temporal_inspection() -> RenderTemporalInspection {
    inspect_render_temporal_inputs(RenderTemporalInspectionRequest {
        frame_index: 29,
        reconstruction_mode: RenderTemporalReconstructionMode::Taau,
        native_fallback_active: false,
        resolution: RenderTemporalResolutionEvidence {
            internal_size: [1280, 720],
            output_size: [1920, 1080],
            min_scale: 0.5,
            max_scale: 1.0,
            dynamic_resolution_enabled: true,
        },
        jitter: RenderTemporalJitterEvidence {
            sequence_id: "halton-2-3:v1".to_string(),
            phase_index: 7,
            phase_count: 8,
            offset: [0.25, 0.125],
        },
        history: RenderTemporalHistoryEvidence {
            resource_id: "history.main.color".to_string(),
            current_signature: "temporal.signature.current".to_string(),
            previous_signature: Some("temporal.signature.current".to_string()),
            age_frames: 8,
            valid: true,
            invalidation_reason: None,
        },
        inputs: vec![
            temporal_input(RenderTemporalInputKind::MotionVectors, true, true),
            temporal_input(RenderTemporalInputKind::Depth, true, true),
            temporal_input(RenderTemporalInputKind::Exposure, true, true),
            temporal_input(RenderTemporalInputKind::ReactiveMask, false, true),
        ],
    })
}

fn temporal_upscaling_inspection(
    temporal: RenderTemporalInspection,
) -> RenderTemporalUpscalingInspection {
    inspect_render_temporal_upscaling(RenderTemporalUpscalingInspectionRequest {
        temporal,
        adapter: RenderTemporalUpscalingAdapterEvidence {
            kind: RenderTemporalUpscalingAdapterKind::FsrStyle,
            capability_state: RenderTemporalUpscalingCapabilityState::Supported,
            required_capabilities: vec![
                "temporal.history.valid".to_string(),
                "dynamic_resolution".to_string(),
                "ray_reconstruction.inputs".to_string(),
            ],
            unsupported_reason: None,
            invocation_requested: true,
        },
        ray_inputs: vec![
            ray_input(RenderRayReconstructionInputKind::MotionVectors, true, true),
            ray_input(RenderRayReconstructionInputKind::Depth, true, true),
            ray_input(RenderRayReconstructionInputKind::Exposure, true, true),
            ray_input(RenderRayReconstructionInputKind::ReactiveMask, false, true),
            ray_input(
                RenderRayReconstructionInputKind::DisocclusionMask,
                false,
                false,
            ),
            ray_input(
                RenderRayReconstructionInputKind::RaymarchDistance,
                true,
                true,
            ),
            ray_input(
                RenderRayReconstructionInputKind::RayQueryHitDistance,
                true,
                true,
            ),
        ],
        native_fallback_visible: false,
        adapter_required_for_correctness: false,
    })
}

fn temporal_visual_evidence() -> Vec<RenderTemporalRuntimeVisualEvidence> {
    vec![
        RenderTemporalRuntimeVisualEvidence {
            view_label: "bench.temporal.taau".to_string(),
            artifact_path:
                "engine/benchmark-artifacts/render-temporal-production-evidence/taau.txt"
                    .to_string(),
            reconstruction_mode: RenderTemporalReconstructionMode::Taau,
            internal_size: [1280, 720],
            output_size: [1920, 1080],
            rendered_pixel_count: 8192,
            history_valid: true,
            native_fallback_visible: false,
            consumed_temporal_inputs: true,
            consumed_temporal_upscaling: true,
        },
        RenderTemporalRuntimeVisualEvidence {
            view_label: "bench.temporal.native_fallback".to_string(),
            artifact_path:
                "engine/benchmark-artifacts/render-temporal-production-evidence/fallback.txt"
                    .to_string(),
            reconstruction_mode: RenderTemporalReconstructionMode::Native,
            internal_size: [1920, 1080],
            output_size: [1920, 1080],
            rendered_pixel_count: 4096,
            history_valid: true,
            native_fallback_visible: true,
            consumed_temporal_inputs: true,
            consumed_temporal_upscaling: true,
        },
    ]
}

fn temporal_input(
    kind: RenderTemporalInputKind,
    required: bool,
    available: bool,
) -> RenderTemporalInputEvidence {
    RenderTemporalInputEvidence {
        kind,
        required,
        available,
        product_id: available.then(|| format!("temporal.product.{}", kind.as_str())),
        generation: available.then_some(101),
    }
}

fn ray_input(
    kind: RenderRayReconstructionInputKind,
    required: bool,
    available: bool,
) -> RenderRayReconstructionInputEvidence {
    RenderRayReconstructionInputEvidence {
        kind,
        required,
        available,
        product_id: available.then(|| format!("ray.product.{}", kind.as_str())),
        generation: available.then_some(202),
    }
}

fn build_simple_fullscreen_flow() -> RenderFlow {
    RenderFlow::new("bench.fullscreen")
        .with_surface_color()
        .expect("render flow authoring should succeed")
        .fullscreen_pass("bench.compose")
        .write_surface_color()
        .expect("render flow authoring should succeed")
        .finish()
        .validate()
        .expect("fullscreen flow should validate")
}

fn build_boids_flow() -> RenderFlow {
    let compute_params_binding = binding_key(0);
    let boids_a_binding = binding_key(1);
    let boids_b_binding = binding_key(2);

    RenderFlow::new("bench.boids")
        .with_state::<BenchState>()
        .double_buffer_storage_array::<BoidInstance>("boids.instances", 4096)
        .expect("render flow authoring should succeed")
        .compute_pass("boids.simulate")
        .bind_ping_pong_storage(boids_a_binding, boids_b_binding, "boids.instances")
        .uniform_from_state(compute_params_binding, BenchState::compute_params)
        .expect("render flow authoring should succeed")
        .dispatch_from_state(BenchState::dispatch_workgroups)
        .finish()
        .validate()
        .expect("boids flow should validate")
}

fn build_procedural_boids_flow() -> RenderFlow {
    let (flow, boid_instances) = RenderFlow::new("bench.procedural_boids")
        .with_state::<BenchState>()
        .with_surface_color()
        .expect("render flow authoring should succeed")
        .with_color_target("bench.procedural_boids.color")
        .expect("render flow authoring should succeed")
        .double_buffer_storage_array_with_handle::<ProceduralBoidInstance>(
            "bench.procedural_boids.instances",
            u64::from(BENCH_BOID_COUNT),
        )
        .expect("render flow authoring should succeed");
    let (flow, grid_cell_counts) = flow
        .storage_array::<U32Counter>(
            "bench.procedural_boids.grid.cell_counts",
            u64::from(BENCH_GRID_CELL_COUNT),
        )
        .expect("render flow authoring should succeed");
    let (flow, grid_cell_offsets) = flow
        .storage_array::<U32ScanElement>(
            "bench.procedural_boids.grid.cell_offsets",
            u64::from(BENCH_GRID_CELL_COUNT),
        )
        .expect("render flow authoring should succeed");
    let (flow, grid_scatter_cursors) = flow
        .storage_array::<U32Counter>(
            "bench.procedural_boids.grid.scatter_cursors",
            u64::from(BENCH_GRID_CELL_COUNT),
        )
        .expect("render flow authoring should succeed");
    let (flow, grid_sorted_indices) = flow
        .storage_array::<U32ScanElement>(
            "bench.procedural_boids.grid.sorted_indices",
            u64::from(BENCH_BOID_COUNT),
        )
        .expect("render flow authoring should succeed");
    let grid_plan = BoundedUniformGrid2dBuildPlan::new(
        "bench.procedural_boids.grid",
        BoundedUniformGrid2dConfig::new(BENCH_GRID_CELLS_X, BENCH_GRID_CELLS_Y, BENCH_BOID_COUNT),
        grid_cell_counts.clone(),
        grid_cell_offsets.clone(),
        grid_scatter_cursors.clone(),
        grid_sorted_indices.clone(),
    )
    .expect("procedural boids grid plan should validate");
    let clear_counts = stage_label(&grid_plan, BoundedUniformGrid2dStage::ClearCounts).to_string();
    let count_cells = stage_label(&grid_plan, BoundedUniformGrid2dStage::CountCells).to_string();
    let scan_counts = stage_label(&grid_plan, BoundedUniformGrid2dStage::ScanCounts).to_string();
    let reset_cursors =
        stage_label(&grid_plan, BoundedUniformGrid2dStage::ResetCursors).to_string();
    let scatter_indices =
        stage_label(&grid_plan, BoundedUniformGrid2dStage::ScatterSortedIndices).to_string();
    let simulate_neighbors =
        stage_label(&grid_plan, BoundedUniformGrid2dStage::SimulateNeighbors).to_string();
    let publish_draw = stage_label(&grid_plan, BoundedUniformGrid2dStage::PublishDraw).to_string();

    // These mirror `boids_compute.wgsl`: params at 0, the ping-pong pair at
    // 1 and 2, then the four grid buffers. The procedural draw shader owns a
    // separate group-0 uniform at binding 0.
    let compute_params_binding = binding_key(0);
    let boids_a_binding = binding_key(1);
    let boids_b_binding = binding_key(2);
    let grid_cell_counts_binding = binding_key(3);
    let grid_cell_offsets_binding = binding_key(4);
    let grid_scatter_cursors_binding = binding_key(5);
    let grid_sorted_indices_binding = binding_key(6);
    let draw_params_binding = binding_key(0);

    let instance_buffer = ProceduralBufferBinding::storage(
        boid_instances.a().clone(),
        procedural_boid_instance_layout(),
    );

    let flow = flow
        .compute_pass("bench.procedural_boids.seed_or_hold")
        .bind_ping_pong_storage(boids_a_binding, boids_b_binding, boid_instances.name())
        .bind_storage(grid_cell_counts_binding, grid_cell_counts.clone())
        .bind_storage(grid_cell_offsets_binding, grid_cell_offsets.clone())
        .bind_storage(grid_scatter_cursors_binding, grid_scatter_cursors.clone())
        .bind_storage(grid_sorted_indices_binding, grid_sorted_indices.clone())
        .uniform_from_state(compute_params_binding, BenchState::compute_params)
        .expect("render flow authoring should succeed")
        .dispatch_from_state(BenchState::dispatch_boids_workgroups)
        .finish()
        .compute_pass(clear_counts.clone())
        .bind_ping_pong_storage(boids_a_binding, boids_b_binding, boid_instances.name())
        .bind_storage(grid_cell_counts_binding, grid_cell_counts.clone())
        .bind_storage(grid_cell_offsets_binding, grid_cell_offsets.clone())
        .bind_storage(grid_scatter_cursors_binding, grid_scatter_cursors.clone())
        .bind_storage(grid_sorted_indices_binding, grid_sorted_indices.clone())
        .uniform_from_state(compute_params_binding, BenchState::compute_params)
        .expect("render flow authoring should succeed")
        .dispatch_from_state(BenchState::dispatch_grid_workgroups)
        .finish()
        .compute_pass(count_cells.clone())
        .bind_ping_pong_storage(boids_a_binding, boids_b_binding, boid_instances.name())
        .bind_storage(grid_cell_counts_binding, grid_cell_counts.clone())
        .bind_storage(grid_cell_offsets_binding, grid_cell_offsets.clone())
        .bind_storage(grid_scatter_cursors_binding, grid_scatter_cursors.clone())
        .bind_storage(grid_sorted_indices_binding, grid_sorted_indices.clone())
        .uniform_from_state(compute_params_binding, BenchState::compute_params)
        .expect("render flow authoring should succeed")
        .dispatch_from_state(BenchState::dispatch_boids_workgroups)
        .finish()
        .compute_pass(scan_counts.clone())
        .bind_ping_pong_storage(boids_a_binding, boids_b_binding, boid_instances.name())
        .bind_storage(grid_cell_counts_binding, grid_cell_counts.clone())
        .bind_storage(grid_cell_offsets_binding, grid_cell_offsets.clone())
        .bind_storage(grid_scatter_cursors_binding, grid_scatter_cursors.clone())
        .bind_storage(grid_sorted_indices_binding, grid_sorted_indices.clone())
        .uniform_from_state(compute_params_binding, BenchState::compute_params)
        .expect("render flow authoring should succeed")
        .dispatch_from_state(BenchState::dispatch_scan_workgroups)
        .finish()
        .compute_pass(reset_cursors.clone())
        .bind_ping_pong_storage(boids_a_binding, boids_b_binding, boid_instances.name())
        .bind_storage(grid_cell_counts_binding, grid_cell_counts.clone())
        .bind_storage(grid_cell_offsets_binding, grid_cell_offsets.clone())
        .bind_storage(grid_scatter_cursors_binding, grid_scatter_cursors.clone())
        .bind_storage(grid_sorted_indices_binding, grid_sorted_indices.clone())
        .uniform_from_state(compute_params_binding, BenchState::compute_params)
        .expect("render flow authoring should succeed")
        .dispatch_from_state(BenchState::dispatch_grid_workgroups)
        .finish()
        .compute_pass(scatter_indices.clone())
        .bind_ping_pong_storage(boids_a_binding, boids_b_binding, boid_instances.name())
        .bind_storage(grid_cell_counts_binding, grid_cell_counts.clone())
        .bind_storage(grid_cell_offsets_binding, grid_cell_offsets.clone())
        .bind_storage(grid_scatter_cursors_binding, grid_scatter_cursors.clone())
        .bind_storage(grid_sorted_indices_binding, grid_sorted_indices.clone())
        .uniform_from_state(compute_params_binding, BenchState::compute_params)
        .expect("render flow authoring should succeed")
        .dispatch_from_state(BenchState::dispatch_boids_workgroups)
        .finish()
        .compute_pass(simulate_neighbors.clone())
        .bind_ping_pong_storage(boids_a_binding, boids_b_binding, boid_instances.name())
        .bind_storage(grid_cell_counts_binding, grid_cell_counts.clone())
        .bind_storage(grid_cell_offsets_binding, grid_cell_offsets.clone())
        .bind_storage(grid_scatter_cursors_binding, grid_scatter_cursors.clone())
        .bind_storage(grid_sorted_indices_binding, grid_sorted_indices.clone())
        .uniform_from_state(compute_params_binding, BenchState::compute_params)
        .expect("render flow authoring should succeed")
        .dispatch_from_state(BenchState::dispatch_boids_workgroups)
        .finish()
        .compute_pass(publish_draw.clone())
        .bind_ping_pong_storage(boids_a_binding, boids_b_binding, boid_instances.name())
        .bind_storage(grid_cell_counts_binding, grid_cell_counts)
        .bind_storage(grid_cell_offsets_binding, grid_cell_offsets)
        .bind_storage(grid_scatter_cursors_binding, grid_scatter_cursors)
        .bind_storage(grid_sorted_indices_binding, grid_sorted_indices)
        .uniform_from_state(compute_params_binding, BenchState::compute_params)
        .expect("render flow authoring should succeed")
        .dispatch_from_state(BenchState::dispatch_boids_workgroups)
        .finish();

    let flow = flow
        .procedural_pass_builder(
            ProceduralPassDescriptor::local_sdf_2d_impostors(
                "bench.procedural_boids.draw",
                instance_buffer,
                BENCH_BOID_COUNT,
            )
            .shader_asset("assets/shaders/boids_compose.wgsl")
            .target(ProceduralTargetDescriptor::color(
                "bench.procedural_boids.color",
            )),
        )
        .expect("procedural boids builder should be valid")
        .uniform_from_state_with_surface(
            draw_params_binding,
            BenchState::procedural_boids_draw_params,
        )
        .expect("render flow authoring should succeed")
        .finish()
        .expect("procedural boids pass should lower");

    flow.present_pass("bench.procedural_boids.present")
        .expect("render flow authoring should succeed")
        .source("bench.procedural_boids.color")
        .finish()
        .validate()
        .expect("procedural boids flow should validate")
}

fn procedural_boid_instance_layout() -> RenderVertexBufferLayout {
    RenderVertexBufferLayout::instance(0, std::mem::size_of::<ProceduralBoidInstance>() as u64)
        .attribute(0, 0, RenderVertexFormat::Float32x2)
        .attribute(1, 8, RenderVertexFormat::Float32x2)
        .attribute(2, 16, RenderVertexFormat::Float32x2)
}

fn stage_label(plan: &BoundedUniformGrid2dBuildPlan, stage: BoundedUniformGrid2dStage) -> &str {
    plan.stages
        .iter()
        .find(|candidate| candidate.stage == stage)
        .map(|candidate| candidate.label.as_str())
        .expect("bounded grid plan should include canonical stage")
}

fn build_compositor_flow() -> RenderFlow {
    let compute_params_binding = binding_key(0);
    let history_a_binding = binding_key(1);
    let history_b_binding = binding_key(2);
    let compose_params_binding = binding_key(0);

    RenderFlow::new("bench.compositor")
        .with_state::<BenchState>()
        .with_surface_color()
        .expect("render flow authoring should succeed")
        .with_builtin_ui()
        .double_buffer_storage_array::<BoidInstance>("post.history", 2048)
        .expect("render flow authoring should succeed")
        .compute_pass("post.extract")
        .bind_ping_pong_storage(history_a_binding, history_b_binding, "post.history")
        .uniform_from_state(compute_params_binding, BenchState::compute_params)
        .expect("render flow authoring should succeed")
        .dispatch([8, 8, 1])
        .finish()
        .fullscreen_pass("post.compose")
        .bind_ping_pong_storage(history_a_binding, history_b_binding, "post.history")
        .uniform_from_state_with_surface(compose_params_binding, BenchState::compose_params)
        .expect("render flow authoring should succeed")
        .write_surface_color()
        .expect("render flow authoring should succeed")
        .finish()
        .builtin_ui_composite_pass("post.ui")
        .expect("render flow authoring should succeed")
        .finish()
        .validate()
        .expect("compositor flow should validate")
}

fn build_sdf_like_flow() -> RenderFlow {
    let compute_params_binding = binding_key(0);
    let field_binding = binding_key(1);
    let compose_params_binding = binding_key(0);

    let (flow, field) = RenderFlow::new("bench.sdf")
        .storage_array::<BoidInstance>("sdf.field", 2048)
        .expect("render flow authoring should succeed");
    flow.with_state::<BenchState>()
        .with_surface_color()
        .expect("render flow authoring should succeed")
        .compute_pass("sdf.compute")
        .bind_storage(field_binding, field.clone())
        .uniform_from_state(compute_params_binding, BenchState::compute_params)
        .expect("render flow authoring should succeed")
        .dispatch([8, 8, 1])
        .finish()
        .fullscreen_pass("sdf.compose")
        .bind_storage(field_binding, field)
        .uniform_from_state_with_surface(compose_params_binding, BenchState::compose_params)
        .expect("render flow authoring should succeed")
        .write_surface_color()
        .expect("render flow authoring should succeed")
        .finish()
        .validate()
        .expect("sdf flow should validate")
}

fn build_mixed_ui_flow() -> RenderFlow {
    let compute_params_binding = binding_key(0);
    let cells_a_binding = binding_key(1);
    let cells_b_binding = binding_key(2);
    let compose_params_binding = binding_key(0);

    RenderFlow::new("bench.mixed_ui")
        .with_state::<BenchState>()
        .with_surface_color()
        .expect("render flow authoring should succeed")
        .with_builtin_ui()
        .double_buffer_storage_array::<BoidInstance>("mixed.cells", 1024)
        .expect("render flow authoring should succeed")
        .compute_pass("mixed.simulate")
        .bind_ping_pong_storage(cells_a_binding, cells_b_binding, "mixed.cells")
        .uniform_from_state(compute_params_binding, BenchState::compute_params)
        .expect("render flow authoring should succeed")
        .dispatch_from_state(BenchState::dispatch_workgroups)
        .finish()
        .fullscreen_pass("mixed.compose")
        .bind_ping_pong_storage(cells_a_binding, cells_b_binding, "mixed.cells")
        .uniform_from_state_with_surface(compose_params_binding, BenchState::compose_params)
        .expect("render flow authoring should succeed")
        .write_surface_color()
        .expect("render flow authoring should succeed")
        .finish()
        .builtin_ui_composite_pass("mixed.ui")
        .expect("render flow authoring should succeed")
        .finish()
        .validate()
        .expect("mixed ui flow should validate")
}

fn bench_render_flow_planning(c: &mut Criterion) {
    let fullscreen = build_simple_fullscreen_flow();
    c.bench_function("render_flow/simple_fullscreen", |b| {
        b.iter(|| run_validation_and_planning(black_box(&fullscreen)))
    });

    let boids = build_boids_flow();
    c.bench_function("render_flow/boids_ping_pong", |b| {
        b.iter(|| run_validation_and_planning(black_box(&boids)))
    });
    let compiled_boids = compile_flow_plan(&boids).expect("boids flow should compile");
    let boids_frame = prepared_frame_for_flow(&compiled_boids);
    let profile = current_runtime_gpu_capabilities();
    let boids_preflight_key = prepared_render_frame_preflight_cache_key(
        &boids_frame,
        std::slice::from_ref(&compiled_boids),
        &profile,
    );
    let boids_preflight_report = preflight_prepared_render_frame(
        &boids_frame,
        std::slice::from_ref(&compiled_boids),
        &profile,
    )
    .expect("boids prepared frame should preflight");
    c.bench_function("render_flow/boids_preflight_cold", |b| {
        b.iter(|| run_cold_preflight(black_box(&compiled_boids), black_box(&boids_frame)))
    });
    c.bench_function("render_flow/boids_preflight_cached", |b| {
        b.iter(|| {
            run_warm_cached_preflight(
                black_box(&compiled_boids),
                black_box(&boids_frame),
                black_box(&boids_preflight_key),
                black_box(&boids_preflight_report),
            )
        })
    });

    c.bench_function("render_population/prefix_scan_plan_4096", |b| {
        b.iter(|| run_prefix_scan_primitive_plan(black_box(4096)))
    });
    c.bench_function(
        "render_population/scan_compaction_indirect_args_plan_4096",
        |b| b.iter(|| run_scan_compaction_indirect_args_plan(black_box(4096))),
    );
    c.bench_function("render_population/bounded_grid_build_plan_4096", |b| {
        b.iter(|| run_bounded_grid_build_plan(black_box(BENCH_BOID_COUNT)))
    });

    let procedural_boids = build_procedural_boids_flow();
    c.bench_function("render_population/boids_production_flow_planning", |b| {
        b.iter(|| run_validation_and_planning(black_box(&procedural_boids)))
    });
    let compiled_procedural_boids =
        compile_flow_plan(&procedural_boids).expect("procedural boids flow should compile");
    let procedural_boids_frame = prepared_frame_for_flow(&compiled_procedural_boids);
    c.bench_function("render_population/boids_production_preflight_cold", |b| {
        b.iter(|| {
            run_cold_preflight(
                black_box(&compiled_procedural_boids),
                black_box(&procedural_boids_frame),
            )
        })
    });
    c.bench_function("render_population/boids_production_evidence_report", |b| {
        b.iter(|| run_boids_production_evidence_report(black_box(&procedural_boids)))
    });

    let compositor = build_compositor_flow();
    c.bench_function("render_flow/multi_pass_compute_compose", |b| {
        b.iter(|| run_validation_and_planning(black_box(&compositor)))
    });

    let sdf = build_sdf_like_flow();
    c.bench_function("render_flow/sdf_compute_compose", |b| {
        b.iter(|| run_validation_and_planning(black_box(&sdf)))
    });

    let mixed_ui = build_mixed_ui_flow();
    c.bench_function("render_flow/mixed_ui_chain", |b| {
        b.iter(|| {
            run_validation_and_planning(black_box(&mixed_ui));
        })
    });

    c.bench_function("render_mesh_material/production_evidence_report", |b| {
        b.iter(run_mesh_material_production_evidence)
    });
    c.bench_function("render_scale/production_evidence_report_4096", |b| {
        b.iter(|| run_scale_production_evidence(black_box(4096)))
    });
    c.bench_function("render_sdf/runtime_evidence_report_4096", |b| {
        b.iter(|| run_sdf_runtime_evidence(black_box(4096)))
    });
    c.bench_function("render_temporal/production_evidence_report", |b| {
        b.iter(run_temporal_production_evidence)
    });
}

criterion_group!(render_flow_planning, bench_render_flow_planning);
criterion_main!(render_flow_planning);
