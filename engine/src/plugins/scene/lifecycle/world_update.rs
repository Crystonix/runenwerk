use super::super::runtime::publish_scene_state;
use crate::plugins::SceneResource;
use crate::runtime::{FixedTimeConfig, Res, ResMut};
use crate::{GameplayRuntimeConfig, SceneRuntimeState, UiOverlayState};
use anyhow::Result;

pub(crate) fn world_scene_update_system(
    fixed_time: Res<FixedTimeConfig>,
    mut scene_resource: ResMut<SceneResource>,
    mut scene_state: ResMut<SceneRuntimeState>,
    mut gameplay: ResMut<GameplayRuntimeConfig>,
    mut overlay: ResMut<UiOverlayState>,
) -> Result<()> {
    let Some(manager) = scene_resource.manager.as_mut() else {
        return Ok(());
    };
    if !manager.world.visible || manager.world.paused {
        publish_scene_state(manager, &mut scene_state, &mut gameplay, &mut overlay);
        return Ok(());
    }

    manager.world_runtime.ctx.delta_seconds =
        fixed_time.step_seconds.clamp(1.0 / 240.0, 1.0 / 30.0);
    manager.world_runtime.ctx.fixed_step_seconds = manager.world_runtime.ctx.delta_seconds;
    manager.world_runtime.run()?;
    let outbound = std::mem::take(&mut manager.world_runtime.ctx.outbound_notifications);
    manager.channels.world_to_overlay.extend(outbound);
    publish_scene_state(manager, &mut scene_state, &mut gameplay, &mut overlay);
    Ok(())
}
