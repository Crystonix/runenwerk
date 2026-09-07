use crate::rendering::ProceduralSkyTerrainState;
use engine::plugins::render::RenderFlow;
use runen_gpu::GpuBindingKey;

pub(crate) fn build_render_flow() -> RenderFlow {
    // `procedural_sky_sdf_terrain_compose.wgsl` declares its only resource at
    // group 0, binding 0.
    let compose_params_binding = binding_key(0);

    RenderFlow::new("procedural_sky_sdf_terrain")
        .with_state::<ProceduralSkyTerrainState>()
        .with_surface_color()
        .expect("render flow authoring should succeed")
        .fullscreen_pass("terrain.compose")
        .shader_asset("assets/shaders/procedural_sky_sdf_terrain_compose.wgsl")
        .uniform_from_state_with_surface(
            compose_params_binding,
            ProceduralSkyTerrainState::compose_params,
        )
        .expect("render flow authoring should succeed")
        .write_surface_color()
        .expect("render flow authoring should succeed")
        .finish()
        .validate()
        .expect("procedural_sky_sdf_terrain should validate")
}

fn binding_key(binding: u64) -> GpuBindingKey {
    GpuBindingKey::try_new(0, binding)
        .expect("procedural-terrain shader binding should fit GpuBindingKey")
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
    fn flow_declares_single_fullscreen_pass() {
        let flow = build_render_flow();
        let graph = flow.graph();
        let pass_ids = graph
            .passes
            .passes
            .iter()
            .map(|pass| pass.label.clone())
            .collect::<Vec<_>>();
        assert_eq!(pass_ids, vec!["terrain.compose"]);
        assert_eq!(
            pass_kind(&flow, "terrain.compose"),
            RenderPassKind::Fullscreen
        );
    }

    #[test]
    fn state_projects_compose_uniforms() {
        let flow = build_render_flow();
        let state = ProceduralSkyTerrainState::default();
        #[allow(deprecated)]
        let frame_data = RenderFrameDataRegistry::new().with(&state);

        let uniforms = flow
            .project_uniforms(&frame_data, (1600, 900))
            .expect("uniform projection should succeed");

        assert!(uniforms.pass(pass_id(&flow, "terrain.compose")).is_some());
    }
}
