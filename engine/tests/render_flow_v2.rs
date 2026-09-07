use engine::plugins::render::{
    CompiledPassExecutionPlan, DrawIndirectArgs, GpuStorage, GpuUniform, PreparedFlowInputs,
    PreparedFlowInvocation, PreparedFrameContext, PreparedFrameContributions, PreparedRenderFrame,
    PreparedShaderSnapshot, PreparedSurfaceInfo, PreparedViewFrame,
    RenderExecutionGraphDiagnosticKind, RenderFlow, RenderFlowValidationIssue,
    RenderFrameDataRegistry, RenderPassId, RenderPassShapeIntent, RenderResourceDeclaration,
    RenderTextureFormatPolicy, RenderTextureSizePolicy, RenderTextureTargetFormat,
    RenderVertexBufferLayout, RenderVertexFormat, ShaderRegistryResource, compile_flow_plan,
    compile_flow_plan_checked, current_runtime_gpu_capabilities,
    preflight_prepared_render_frame_runtime_guards,
};
use runen_gpu::{GpuBindingKey, GpuCapabilities, GpuCapabilityFeature};
use std::any::TypeId;
use std::collections::BTreeMap;
use ui_render_data::ViewportSurfaceBindingRegistry;

#[derive(Debug, Clone, Copy, GpuStorage)]
struct Cell {
    alive: u32,
}

#[derive(Debug, Clone, Copy, GpuStorage)]
struct Vertex {
    position: [f32; 3],
}

#[derive(Debug, Clone, Copy, GpuUniform)]
struct ComputeParams {
    tick: u32,
    grid: [u32; 2],
    step: u32,
}

#[derive(Debug, Clone, Copy, GpuUniform)]
struct ComposeParams {
    grid: [u32; 2],
    surface: [f32; 2],
}

#[derive(Debug, Clone, ecs::Resource)]
struct FlowState {
    tick: u32,
    grid: [u32; 2],
}

impl Default for FlowState {
    fn default() -> Self {
        Self {
            tick: 0,
            grid: [16, 9],
        }
    }
}

impl FlowState {
    fn compute_params(&self) -> ComputeParams {
        ComputeParams {
            tick: self.tick,
            grid: self.grid,
            step: 1,
        }
    }

    fn compose_params(&self, surface: (u32, u32)) -> ComposeParams {
        ComposeParams {
            grid: self.grid,
            surface: [surface.0 as f32, surface.1 as f32],
        }
    }

    fn dispatch(&self) -> [u32; 3] {
        [self.grid[0].div_ceil(8), self.grid[1].div_ceil(8), 1]
    }
}

fn binding_key(binding: u64) -> GpuBindingKey {
    GpuBindingKey::try_new(0, binding).expect("test shader binding should fit GpuBindingKey")
}

fn build_flow() -> RenderFlow {
    // The game-of-life shaders declare params at binding 0 and their
    // ping-pong cell buffers at bindings 1 and 2.
    let compute_params_binding = binding_key(0);
    let cells_a_binding = binding_key(1);
    let cells_b_binding = binding_key(2);
    let compose_params_binding = binding_key(0);

    RenderFlow::new("v2.flow")
        .with_state::<FlowState>()
        .with_surface_color()
        .expect("render flow authoring should succeed")
        .with_builtin_ui()
        .double_buffer_storage_array::<Cell>("cells", 16 * 9)
        .expect("render flow authoring should succeed")
        .compute_pass("simulate")
        .shader_asset("assets/shaders/game_of_life_compute.wgsl")
        .uniform_from_state(compute_params_binding, FlowState::compute_params)
        .expect("render flow authoring should succeed")
        .bind_ping_pong_storage(cells_a_binding, cells_b_binding, "cells")
        .dispatch_from_state(FlowState::dispatch)
        .finish()
        .fullscreen_pass("compose")
        .shader_asset("assets/shaders/game_of_life_compose.wgsl")
        .uniform_from_state_with_surface(compose_params_binding, FlowState::compose_params)
        .expect("render flow authoring should succeed")
        .bind_ping_pong_storage(cells_a_binding, cells_b_binding, "cells")
        .write_surface_color()
        .expect("render flow authoring should succeed")
        .finish()
        .builtin_ui_composite_pass("ui")
        .expect("render flow authoring should succeed")
        .finish()
        .validate()
        .expect("flow should validate")
}

fn pass_id_by_label(flow: &RenderFlow, label: &str) -> RenderPassId {
    flow.graph()
        .passes
        .passes
        .iter()
        .find(|pass| pass.label == label)
        .map(|pass| pass.id)
        .expect("pass label should exist")
}

fn prepared_frame_for_flow(flow_id: engine::plugins::render::RenderFlowId) -> PreparedRenderFrame {
    PreparedRenderFrame {
        context: PreparedFrameContext {
            frame_index: 1,
            flow_registry_revision: 1,
            shader_registry_revision: 1,
            prepare_epoch: 1,
        },
        surface: PreparedSurfaceInfo::primary((800, 600)),
        views: vec![PreparedViewFrame::main((800, 600))],
        flows: BTreeMap::new(),
        flow_invocations: vec![PreparedFlowInvocation::main(
            flow_id,
            PreparedFlowInputs::default(),
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

fn instanced_fullscreen_style_flow(instance_count: u32) -> RenderFlow {
    let (flow, cells) = RenderFlow::new("v2.pass-shape.fullscreen-instanced")
        .with_surface_color()
        .expect("render flow authoring should succeed")
        .storage_array::<Cell>("cells", 64)
        .expect("render flow authoring should succeed");
    flow.graphics_pass("compose")
        .shader_asset("assets/shaders/game_of_life_compose.wgsl")
        .bind_storage(binding_key(1), cells)
        .write_surface_color()
        .expect("render flow authoring should succeed")
        .draw(3, instance_count)
        .finish()
        .validate()
        .expect("legacy-valid graphics shape should reach compiler guard")
}

#[test]
fn v2_flow_keeps_planning_contract_inspectable() {
    let flow = build_flow();
    let lexical_order = flow
        .lexical_pass_order()
        .expect("flow should expose render-owned lexical planning order");
    let pass_labels_by_id = flow
        .graph()
        .passes
        .passes
        .iter()
        .map(|pass| (pass.id, pass.label.as_str()))
        .collect::<BTreeMap<_, _>>();
    let ordered_labels = lexical_order
        .iter()
        .map(|id| {
            pass_labels_by_id
                .get(id)
                .copied()
                .expect("pass should exist")
        })
        .collect::<Vec<_>>();
    assert_eq!(ordered_labels, vec!["simulate", "compose", "ui"]);

    let compiled =
        compile_flow_plan(&flow).expect("flow should compile into render execution recipes");
    let simulate = compiled
        .execution
        .passes
        .iter()
        .find_map(|pass| match pass {
            CompiledPassExecutionPlan::Compute(pass)
                if pass.pass_id == pass_id_by_label(&flow, "simulate") =>
            {
                Some(pass)
            }
            _ => None,
        })
        .expect("simulate compute recipe should compile");
    assert_eq!(
        simulate.bindings.storage_order.len(),
        2,
        "compile-time inspection should preserve both ping-pong storage bindings without claiming prepared G3 execution order"
    );
}

#[test]
fn render_flow_compiler_reports_typed_static_resource_diagnostics() {
    let (flow, _cells) = RenderFlow::new("v2.invalid.compiler")
        .with_surface_color()
        .expect("render flow authoring should succeed")
        .storage_array::<Cell>("cells", 4)
        .expect("render flow authoring should succeed");
    let flow = flow
        .fullscreen_pass("compose")
        .sample_texture(binding_key(0), binding_key(1), "cells")
        .write_surface_color()
        .expect("render flow authoring should succeed")
        .finish();

    let err = compile_flow_plan_checked(&flow, &current_runtime_gpu_capabilities())
        .expect_err("compiler should expose typed diagnostics for invalid static resources");

    assert!(err.diagnostics.iter().any(|diagnostic| {
        diagnostic.kind == RenderExecutionGraphDiagnosticKind::InvalidResource
            && diagnostic.message.contains("samples resource")
    }));
}

#[test]
fn render_flow_compiler_reports_backend_capability_mismatches() {
    let flow = RenderFlow::new("v2.compiler.capability")
        .with_surface_color()
        .expect("render flow authoring should succeed")
        .compute_pass("simulate")
        .dispatch([1, 1, 1])
        .finish()
        .validate()
        .expect("flow should validate before capability check");

    let err =
        compile_flow_plan_checked(&flow, &capabilities_without(GpuCapabilityFeature::Compute))
            .expect_err("unsupported compute capability should be typed");

    assert!(err.diagnostics.iter().any(|diagnostic| {
        diagnostic.kind == RenderExecutionGraphDiagnosticKind::BackendCapabilityMismatch
            && diagnostic.capability.as_deref() == Some("feature::Compute")
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
fn render_flow_compiler_requires_indirect_draw_capability() {
    let (flow, args) = RenderFlow::new("indirect.capability")
        .with_color_target("color")
        .expect("render flow authoring should succeed")
        .storage_array::<DrawIndirectArgs>("args", 1)
        .expect("render flow authoring should succeed");
    let flow = flow
        .graphics_pass("draw")
        .write_color_target("color")
        .draw_indirect(args, 3, 1)
        .expect("declared draw-argument layout should match")
        .finish()
        .validate()
        .expect("flow shape should be valid");

    let error = compile_flow_plan_checked(
        &flow,
        &capabilities_without(GpuCapabilityFeature::IndirectExecution),
    )
    .expect_err("indirect draw must require normalized capability support");
    assert!(error.diagnostics.iter().any(|diagnostic| {
        diagnostic.kind == RenderExecutionGraphDiagnosticKind::BackendCapabilityMismatch
            && diagnostic.capability.as_deref() == Some("feature::IndirectExecution")
    }));
}

#[test]
fn render_flow_compiler_rejects_instanced_fullscreen_style_graphics_by_default() {
    let flow = instanced_fullscreen_style_flow(512);

    let err = compile_flow_plan_checked(&flow, &current_runtime_gpu_capabilities())
        .expect_err("instanced fullscreen-style work should require explicit intent");

    assert!(err.diagnostics.iter().any(|diagnostic| {
        diagnostic.kind == RenderExecutionGraphDiagnosticKind::FullscreenInstancedWork
            && diagnostic.capability.as_deref() == Some("pass_shape::fullscreen_instanced_work")
            && diagnostic.pass_label.as_deref() == Some("compose")
    }));
}

#[test]
fn render_flow_compiler_accepts_bounded_instanced_fullscreen_intent() {
    let (flow, cells) = RenderFlow::new("v2.pass-shape.explicit")
        .with_surface_color()
        .expect("render flow authoring should succeed")
        .storage_array::<Cell>("cells", 64)
        .expect("render flow authoring should succeed");
    let flow = flow
        .graphics_pass("compose")
        .shader_asset("assets/shaders/game_of_life_compose.wgsl")
        .bind_storage(binding_key(1), cells)
        .write_surface_color()
        .expect("render flow authoring should succeed")
        .draw(3, 512)
        .allow_instanced_fullscreen(1024, "bounded diagnostic stress pass")
        .finish()
        .validate()
        .expect("explicit advanced intent should preserve legacy-valid graph shape");

    let compiled = compile_flow_plan_checked(&flow, &current_runtime_gpu_capabilities())
        .expect("bounded explicit intent should pass compiler guard");
    let pass = compiled
        .render_passes
        .iter()
        .find(|pass| pass.pass_label() == "compose")
        .expect("compose pass should compile");
    let RenderPassShapeIntent::AdvancedInstancedFullscreen {
        max_instances,
        reason,
    } = &pass.node().shape_intent
    else {
        panic!("compose should record explicit advanced pass-shape intent");
    };
    assert_eq!(*max_instances, 1024);
    assert_eq!(reason, "bounded diagnostic stress pass");
}

#[test]
fn render_flow_compiler_rejects_instanced_fullscreen_intent_over_limit() {
    let (flow, cells) = RenderFlow::new("v2.pass-shape.explicit-over-limit")
        .with_surface_color()
        .expect("render flow authoring should succeed")
        .storage_array::<Cell>("cells", 64)
        .expect("render flow authoring should succeed");
    let flow = flow
        .graphics_pass("compose")
        .shader_asset("assets/shaders/game_of_life_compose.wgsl")
        .bind_storage(binding_key(1), cells)
        .write_surface_color()
        .expect("render flow authoring should succeed")
        .draw(3, 2048)
        .allow_instanced_fullscreen(1024, "bounded diagnostic stress pass")
        .finish()
        .validate()
        .expect("limit enforcement belongs to compiler guard");

    let err = compile_flow_plan_checked(&flow, &current_runtime_gpu_capabilities())
        .expect_err("advanced intent must enforce its own bound");

    assert!(err.diagnostics.iter().any(|diagnostic| {
        diagnostic.kind == RenderExecutionGraphDiagnosticKind::FullscreenInstancedWork
            && diagnostic.capability.as_deref()
                == Some("pass_shape::advanced_instanced_fullscreen_limit")
    }));
}

#[test]
fn render_flow_compiler_preserves_instanced_graphics_with_local_geometry() {
    let (flow, vertices) = RenderFlow::new("v2.pass-shape.local-geometry")
        .with_surface_color()
        .expect("render flow authoring should succeed")
        .storage_array::<Vertex>("vertices", 3)
        .expect("render flow authoring should succeed");
    let (flow, instances) = flow
        .storage_array::<Cell>("instances", 512)
        .expect("render flow authoring should succeed");
    let flow = flow
        .graphics_pass("sprites")
        .shader_asset("assets/shaders/game_of_life_compose.wgsl")
        .vertex_buffer(
            vertices,
            RenderVertexBufferLayout::vertex(0, 16).attribute(0, 0, RenderVertexFormat::Float32x3),
        )
        .instance_buffer(
            instances,
            RenderVertexBufferLayout::instance(1, 16).attribute(1, 0, RenderVertexFormat::Uint32),
        )
        .write_surface_color()
        .expect("render flow authoring should succeed")
        .draw(3, 512)
        .finish()
        .validate()
        .expect("local geometry path should remain valid");

    compile_flow_plan_checked(&flow, &current_runtime_gpu_capabilities())
        .expect("local vertex/instance geometry should not need fullscreen opt-in");
}

#[test]
fn render_flow_runtime_guard_rejects_cached_instanced_fullscreen_hazard() {
    let flow = instanced_fullscreen_style_flow(512);
    let compiled = compile_flow_plan(&flow).expect("unchecked compile keeps legacy shape visible");
    let frame = prepared_frame_for_flow(compiled.flow_id);

    let err = preflight_prepared_render_frame_runtime_guards(&frame, &[compiled])
        .expect_err("runtime guard should reject pass-shape hazards before cache hit");

    assert!(err.diagnostics.iter().any(|diagnostic| {
        diagnostic.kind == RenderExecutionGraphDiagnosticKind::FullscreenInstancedWork
            && diagnostic.pass_label.as_deref() == Some("compose")
    }));
}

#[test]
fn v2_uniform_projection_uses_state_bindings() {
    let flow = build_flow();
    let state = FlowState::default();
    let frame_data = RenderFrameDataRegistry::new().with(&state);
    let projections = flow
        .project_uniforms(&frame_data, (1280, 720))
        .expect("projection should succeed");
    assert!(
        projections
            .pass(pass_id_by_label(&flow, "simulate"))
            .is_some()
    );
    assert!(
        projections
            .pass(pass_id_by_label(&flow, "compose"))
            .is_some()
    );
}

#[test]
fn v2_named_uniform_buffers_support_prepared_invocation_overrides() {
    let (flow, handle) = RenderFlow::new("v2.named-uniform")
        .with_state::<FlowState>()
        .uniform_buffer::<ComposeParams>("compose.per_invocation")
        .expect("render flow authoring should succeed");

    assert_eq!(
        flow.resource_id("compose.per_invocation"),
        Some(handle.diagnostic_identity())
    );
    assert_eq!(
        flow.graph()
            .resources
            .uniform_buffer_ids_by_params_type(TypeId::of::<ComposeParams>()),
        vec![handle.diagnostic_identity()],
        "named uniform buffers should be real flow resources, not ad hoc request bytes"
    );
}

#[test]
fn v2_exact_color_target_is_surface_sized_with_exact_format() {
    let flow = RenderFlow::new("v2.exact-color")
        .with_color_target_exact("proof.bytes", RenderTextureTargetFormat::Rgba8Unorm)
        .expect("render flow authoring should succeed");
    let id = flow
        .resource_id("proof.bytes")
        .expect("exact color target should register a resource");
    let resource = flow
        .graph()
        .resources
        .resources
        .iter()
        .find(|resource| *resource.id() == id)
        .expect("registered resource should have a descriptor");

    let RenderResourceDeclaration::ColorAttachment(target) = resource else {
        panic!("exact color target should remain a color target");
    };
    assert_eq!(target.texture.size, RenderTextureSizePolicy::Surface);
    assert_eq!(
        target.texture.format,
        RenderTextureFormatPolicy::Exact(RenderTextureTargetFormat::Rgba8Unorm)
    );
}

#[test]
fn v2_exact_color_target_rejects_depth_format() {
    let err = RenderFlow::new("v2.exact-color-depth")
        .with_color_target_exact("proof.bytes", RenderTextureTargetFormat::Depth32Float)
        .expect("render flow authoring should succeed")
        .validation_report()
        .expect_err("exact color targets cannot resolve to depth formats");

    assert!(err.issues.iter().any(|issue| matches!(
        issue,
        RenderFlowValidationIssue::InvalidTextureFormatClass {
            resource_kind: "color_target",
            format: RenderTextureTargetFormat::Depth32Float,
            ..
        }
    )));
}

#[test]
fn v2_uniform_projection_infers_types_from_method_items() {
    let compute_params_binding = binding_key(0);
    let cells_a_binding = binding_key(1);
    let cells_b_binding = binding_key(2);
    let compose_params_binding = binding_key(0);

    let flow = RenderFlow::new("v2.inference")
        .with_state::<FlowState>()
        .with_surface_color()
        .expect("render flow authoring should succeed")
        .double_buffer_storage_array::<Cell>("cells", 16 * 9)
        .expect("render flow authoring should succeed")
        .compute_pass("simulate")
        .uniform_from_state(compute_params_binding, FlowState::compute_params)
        .expect("render flow authoring should succeed")
        .bind_ping_pong_storage(cells_a_binding, cells_b_binding, "cells")
        .dispatch_from_state(FlowState::dispatch)
        .finish()
        .fullscreen_pass("compose")
        .uniform_from_state_with_surface(compose_params_binding, FlowState::compose_params)
        .expect("render flow authoring should succeed")
        .bind_ping_pong_storage(cells_a_binding, cells_b_binding, "cells")
        .write_surface_color()
        .expect("render flow authoring should succeed")
        .finish()
        .validate()
        .expect("flow should validate");

    let state = FlowState::default();
    let frame_data = RenderFrameDataRegistry::new().with(&state);
    let projections = flow
        .project_uniforms(&frame_data, (1920, 1080))
        .expect("projection should succeed");
    assert!(
        projections
            .pass(pass_id_by_label(&flow, "simulate"))
            .is_some()
    );
    assert!(
        projections
            .pass(pass_id_by_label(&flow, "compose"))
            .is_some()
    );
}

#[test]
fn shader_registry_supports_asset_and_explicit_registration() {
    let mut registry = ShaderRegistryResource::default();
    let a = registry.register_shader("assets/shaders/game_of_life_compute.wgsl");
    let b = registry
        .register_shader_with_id("custom.compose", "assets/shaders/game_of_life_compose.wgsl");
    assert!(registry.shader_count() >= 2);
    assert_eq!(
        registry.handle("custom.compose"),
        Some(b),
        "explicit id should resolve to registered shader handle"
    );
    assert_ne!(a, b);
}
