use super::super::runtime::{
    SceneTemplateFlowResource, apply_overlay_messages, publish_scene_state,
    rebuild_overlay_ui_frame,
};
use crate::plugins::render::{
    RenderFrameProducerId, SurfaceFrameRoute, SurfaceFrameSubmission, SurfaceFrameSubmissionOrder,
    SurfaceFrameSubmissionRegistryResource,
};
use crate::plugins::scene::ui::UiRenderShaderConfig;
use crate::plugins::{SceneManager, SceneResource};
use crate::runtime::ResMut;
use crate::{GameplayRuntimeConfig, SceneRuntimeState, UiOverlayState};
use anyhow::Result;

const SCENE_OVERLAY_FRAME_PRODUCER_ID: RenderFrameProducerId = render_frame_producer_id(1);

const fn render_frame_producer_id(raw: u64) -> RenderFrameProducerId {
    match RenderFrameProducerId::try_from_raw(raw) {
        Ok(id) => id,
        Err(_) => panic!("render frame producer id constants must be non-zero"),
    }
}

pub(crate) fn scene_overlay_update_system(
    scene_templates: ResMut<SceneTemplateFlowResource>,
    mut scene_resource: ResMut<SceneResource>,
    mut scene_state: ResMut<SceneRuntimeState>,
    mut gameplay: ResMut<GameplayRuntimeConfig>,
    mut overlay: ResMut<UiOverlayState>,
    mut submissions: ResMut<SurfaceFrameSubmissionRegistryResource>,
) -> Result<()> {
    let Some(manager) = scene_resource.manager.as_mut() else {
        submissions.remove(&SCENE_OVERLAY_FRAME_PRODUCER_ID);
        return Ok(());
    };

    apply_overlay_messages(manager);
    rebuild_overlay_ui_frame(manager, &scene_templates)?;
    publish_scene_state(manager, &mut scene_state, &mut gameplay, &mut overlay);
    publish_scene_overlay_frame(manager, &mut submissions);
    Ok(())
}

fn publish_scene_overlay_frame(
    manager: &SceneManager,
    submissions: &mut SurfaceFrameSubmissionRegistryResource,
) {
    let frame = manager.overlay_runtime.ui.frame.clone();
    if frame.is_empty() {
        submissions.remove(&SCENE_OVERLAY_FRAME_PRODUCER_ID);
        return;
    }

    let rect_shader_asset_id = manager
        .overlay_runtime
        .world
        .resource::<UiRenderShaderConfig>()
        .ok()
        .map(|config| config.rect_shader_asset_id.trim().to_string())
        .filter(|id| !id.is_empty());

    submissions.replace(
        SurfaceFrameSubmission::new(SCENE_OVERLAY_FRAME_PRODUCER_ID)
            .with_route(SurfaceFrameRoute::Screen)
            .with_order(SurfaceFrameSubmissionOrder::new(0, 0))
            .with_frame(frame)
            .with_rect_shader_asset_id(rect_shader_asset_id),
    );
}
