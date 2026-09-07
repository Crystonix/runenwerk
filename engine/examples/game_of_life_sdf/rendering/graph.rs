use crate::rendering::{DEFAULT_GRID_CELL_COUNT, GameOfLifeCell, GameOfLifeRenderState};
use engine::plugins::render::RenderFlow;
use runen_gpu::GpuBindingKey;

pub(crate) fn build_render_flow() -> RenderFlow {
    // `game_of_life_{compute,compose}.wgsl` use params at binding 0 and the
    // ping-pong storage pair at bindings 1 and 2.
    let compute_params_binding = binding_key(0);
    let cells_a_binding = binding_key(1);
    let cells_b_binding = binding_key(2);
    let compose_params_binding = binding_key(0);

    RenderFlow::new("game_of_life_sdf")
        .with_state::<GameOfLifeRenderState>()
        .with_surface_color()
        .expect("render flow authoring should succeed")
        .with_builtin_ui()
        .double_buffer_storage_array::<GameOfLifeCell>("cells", DEFAULT_GRID_CELL_COUNT)
        .expect("render flow authoring should succeed")
        .compute_pass("simulate")
        .shader_asset("assets/shaders/game_of_life_compute.wgsl")
        .uniform_from_state(
            compute_params_binding,
            GameOfLifeRenderState::compute_params,
        )
        .expect("render flow authoring should succeed")
        .bind_ping_pong_storage(cells_a_binding, cells_b_binding, "cells")
        .dispatch_from_state(GameOfLifeRenderState::dispatch_workgroups)
        .finish()
        .fullscreen_pass("compose")
        .shader_asset("assets/shaders/game_of_life_compose.wgsl")
        .uniform_from_state_with_surface(
            compose_params_binding,
            GameOfLifeRenderState::compose_params,
        )
        .expect("render flow authoring should succeed")
        .bind_ping_pong_storage(cells_a_binding, cells_b_binding, "cells")
        .write_surface_color()
        .expect("render flow authoring should succeed")
        .finish()
        .builtin_ui_composite_pass("ui")
        .expect("render flow authoring should succeed")
        .finish()
        .validate()
        .expect("game_of_life_sdf flow should validate")
}

fn binding_key(binding: u64) -> GpuBindingKey {
    GpuBindingKey::try_new(0, binding)
        .expect("game-of-life shader binding should fit GpuBindingKey")
}

#[cfg(test)]
mod tests {
    #[allow(deprecated)]
    use super::*;
    #[allow(deprecated)]
    use engine::plugins::render::{RenderFrameDataRegistry, RenderPassId, RenderPassKind};

    fn pass_kind(flow: &RenderFlow, pass_id: &str) -> RenderPassKind {
        flow.graph()
            .passes
            .passes
            .iter()
            .find(|pass| pass.label == pass_id)
            .map(|pass| pass.kind)
            .expect("requested pass should exist")
    }

    fn pass_id(flow: &RenderFlow, pass_label: &str) -> RenderPassId {
        flow.graph()
            .passes
            .passes
            .iter()
            .find(|pass| pass.label == pass_label)
            .map(|pass| pass.id)
            .expect("requested pass should exist")
    }

    #[test]
    fn flow_declares_expected_contract() {
        let flow = build_render_flow();
        let graph = flow.graph();
        let pass_ids = graph
            .passes
            .passes
            .iter()
            .map(|pass| pass.label.clone())
            .collect::<Vec<_>>();
        assert_eq!(pass_ids, vec!["simulate", "compose", "ui"]);

        assert_eq!(pass_kind(&flow, "simulate"), RenderPassKind::Compute);
        assert_eq!(pass_kind(&flow, "compose"), RenderPassKind::Fullscreen);
    }

    #[test]
    fn state_projects_compute_and_compose_uniforms() {
        let flow = build_render_flow();
        let state = GameOfLifeRenderState::default();
        // Projection-helper compatibility surface; active runtime submission uses PreparedRenderFrame.
        #[allow(deprecated)]
        let frame_data = RenderFrameDataRegistry::new().with(&state);

        let uniforms = flow
            .project_uniforms(&frame_data, (1280, 720))
            .expect("uniform projection should succeed");

        assert!(uniforms.pass(pass_id(&flow, "simulate")).is_some());
        assert!(uniforms.pass(pass_id(&flow, "compose")).is_some());
    }
}
