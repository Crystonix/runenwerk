use crate::rendering::{Sdf3dRenderState, SdfHistoryProbe};
use engine::plugins::render::RenderFlow;
use runen_gpu::GpuBindingKey;

pub(crate) fn build_render_flow() -> RenderFlow {
    // The flow's established binding contract is the uniform at binding 0 and
    // the history probe pair at bindings 1 and 2. `sdf_render_flow_3d_compose`
    // consumes the compose uniform at binding 0.
    let prepare_params_binding = binding_key(0);
    let history_probe_a_binding = binding_key(1);
    let history_probe_b_binding = binding_key(2);
    let compose_params_binding = binding_key(0);

    RenderFlow::new("sdf_render_flow_3d")
        .with_state::<Sdf3dRenderState>()
        .with_surface_color()
        .expect("render flow authoring should succeed")
        .with_color_target("sdf.color")
        .expect("render flow authoring should succeed")
        .with_history_texture("sdf.history")
        .expect("render flow authoring should succeed")
        .double_buffer_storage_array::<SdfHistoryProbe>("sdf.history.probe", 4)
        .expect("render flow authoring should succeed")
        .compute_pass("sdf.prepare")
        .uniform_from_state(prepare_params_binding, Sdf3dRenderState::prepare_params)
        .expect("render flow authoring should succeed")
        .bind_ping_pong_storage(
            history_probe_a_binding,
            history_probe_b_binding,
            "sdf.history.probe",
        )
        .dispatch([1, 1, 1])
        .finish()
        .fullscreen_pass("sdf.compose")
        .shader_asset("assets/shaders/sdf_render_flow_3d_compose.wgsl")
        .uniform_from_state_with_surface(compose_params_binding, Sdf3dRenderState::compose_params)
        .expect("render flow authoring should succeed")
        .bind_ping_pong_storage(
            history_probe_a_binding,
            history_probe_b_binding,
            "sdf.history.probe",
        )
        .write_color_target("sdf.color")
        .finish()
        .copy_pass("sdf.history")
        .source("sdf.color")
        .destination("sdf.history")
        .finish()
        .present_pass("sdf.present")
        .expect("render flow authoring should succeed")
        .source("sdf.color")
        .order_after("sdf.history")
        .finish()
        .validate()
        .expect("sdf_render_flow_3d should validate")
}

fn binding_key(binding: u64) -> GpuBindingKey {
    GpuBindingKey::try_new(0, binding).expect("SDF shader binding should fit GpuBindingKey")
}

#[cfg(test)]
mod tests {
    #[allow(deprecated)]
    use super::*;
    #[allow(deprecated)]
    use engine::plugins::render::{RenderFrameDataRegistry, RenderPassKind};

    fn pass_kind(flow: &RenderFlow, pass_id: &str) -> RenderPassKind {
        flow.graph()
            .passes
            .passes
            .iter()
            .find(|pass| pass.label == pass_id)
            .map(|pass| pass.kind)
            .expect("requested pass should exist")
    }

    #[test]
    fn flow_declares_compute_fullscreen_history_then_present() {
        let flow = build_render_flow();
        let graph = flow.graph();
        let pass_ids = graph
            .passes
            .passes
            .iter()
            .map(|pass| pass.label.clone())
            .collect::<Vec<_>>();
        assert_eq!(
            pass_ids,
            vec!["sdf.prepare", "sdf.compose", "sdf.history", "sdf.present"]
        );
        assert_eq!(pass_kind(&flow, "sdf.prepare"), RenderPassKind::Compute);
        assert_eq!(pass_kind(&flow, "sdf.compose"), RenderPassKind::Fullscreen);
        assert_eq!(pass_kind(&flow, "sdf.history"), RenderPassKind::Copy);
        assert_eq!(pass_kind(&flow, "sdf.present"), RenderPassKind::Present);
    }

    #[test]
    fn flow_orders_prepare_compose_history_before_terminal_present() {
        let flow = build_render_flow();
        let order = flow
            .lexical_pass_order()
            .expect("sdf_render_flow pass order should validate")
            .into_iter()
            .map(|id| {
                flow.graph()
                    .passes
                    .passes
                    .iter()
                    .find(|pass| pass.id == id)
                    .map(|pass| pass.label.clone())
                    .expect("ordered pass should exist")
            })
            .collect::<Vec<_>>();

        assert_eq!(
            order,
            vec!["sdf.prepare", "sdf.compose", "sdf.history", "sdf.present"]
        );
    }

    #[test]
    fn state_projects_prepare_and_compose_uniforms() {
        let flow = build_render_flow();
        let state = Sdf3dRenderState::default();
        #[allow(deprecated)]
        let frame_data = RenderFrameDataRegistry::new().with(&state);

        let uniforms = flow
            .project_uniforms(&frame_data, (1600, 900))
            .expect("uniform projection should succeed");

        let compose_id = flow
            .graph()
            .passes
            .passes
            .iter()
            .find(|pass| pass.label == "sdf.compose")
            .map(|pass| pass.id)
            .expect("compose pass should exist");
        let prepare_id = flow
            .graph()
            .passes
            .passes
            .iter()
            .find(|pass| pass.label == "sdf.prepare")
            .map(|pass| pass.id)
            .expect("prepare pass should exist");
        assert!(uniforms.pass(prepare_id).is_some());
        assert!(uniforms.pass(compose_id).is_some());
    }
}
