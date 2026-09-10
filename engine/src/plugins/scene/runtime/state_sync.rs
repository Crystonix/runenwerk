use super::helpers::snapshot_public_scene_state;
use crate::plugins::shared::{ReloadStatusPayload, should_reload};
use crate::plugins::{InputState, SceneManager, SceneResource};
use crate::prelude::domain::{
    GAMEPLAY_CONFIG_PATH, gameplay_config_modified, load_gameplay_config_with_modified_and_error,
};
use crate::{GameplayRuntimeConfig, SceneRuntimeState, UiOverlayState, WindowState};
use anyhow::Result;

// Owner: Engine Scene Plugin - Runtime State Sync
pub(crate) fn sync_overlay_viewport(manager: &mut SceneManager, window: &WindowState) {
    manager.set_overlay_viewport(
        (window.size_px.0 as f32, window.size_px.1 as f32),
        window.scale_factor as f32,
    );
}

pub(crate) fn sync_world_scene_context_from_input(
    manager: &mut SceneManager,
    input: &InputState,
    frame_delta_seconds: f32,
    fixed_step_seconds: f32,
) {
    let active_overlay_label = manager.active_overlay().label().to_string();
    let overlay_visible = manager.overlay_visible();
    let world_paused = manager.world.paused;
    let runtime = &mut manager.world_runtime;
    runtime.ctx.overlay_consumed = input.overlay_consumed;
    runtime.ctx.overlay_scene_label = active_overlay_label;
    runtime.ctx.player_move_x = (if input.world_move_right { 1.0 } else { 0.0 })
        - (if input.world_move_left { 1.0 } else { 0.0 });
    runtime.ctx.player_move_y = (if input.world_move_up { 1.0 } else { 0.0 })
        - (if input.world_move_down { 1.0 } else { 0.0 });
    runtime.ctx.fixed_step_seconds = fixed_step_seconds;

    if !overlay_visible && !world_paused {
        let camera_cfg = &runtime.ctx.gameplay_config.camera;
        let rotate_sensitivity = camera_cfg.rotate_sensitivity.max(0.0);
        let yaw_sign = if camera_cfg.invert_x { 1.0 } else { -1.0 };
        let pitch_sign = if camera_cfg.invert_y { -1.0 } else { 1.0 };
        runtime.ctx.camera_yaw += input.mouse_delta.0 * rotate_sensitivity * yaw_sign;
        runtime.ctx.camera_pitch += input.mouse_delta.1 * rotate_sensitivity * pitch_sign;
    }
    if !overlay_visible && input.scroll_delta.abs() > f32::EPSILON {
        let camera_cfg = &runtime.ctx.gameplay_config.camera;
        let zoom_sensitivity = camera_cfg.zoom_sensitivity.max(0.0);
        let zoom_sign = if camera_cfg.invert_zoom { 1.0 } else { -1.0 };
        runtime.ctx.camera_distance += input.scroll_delta * zoom_sensitivity * zoom_sign;
    }
    let camera_cfg = &runtime.ctx.gameplay_config.camera;
    let pitch_min = camera_cfg.pitch_min.min(camera_cfg.pitch_max);
    let pitch_max = camera_cfg.pitch_min.max(camera_cfg.pitch_max);
    let distance_min = camera_cfg
        .distance_min
        .min(camera_cfg.distance_max)
        .max(0.1);
    let distance_max = camera_cfg
        .distance_min
        .max(camera_cfg.distance_max)
        .max(distance_min);
    runtime.ctx.camera_pitch = runtime.ctx.camera_pitch.clamp(pitch_min, pitch_max);
    runtime.ctx.camera_distance = runtime
        .ctx
        .camera_distance
        .clamp(distance_min, distance_max);

    let latest_modified = gameplay_config_modified();
    if should_reload(
        true,
        false,
        runtime.ctx.gameplay_config_modified,
        latest_modified,
    ) {
        let (config, modified, error) = load_gameplay_config_with_modified_and_error();
        runtime.ctx.gameplay_config = config;
        runtime.ctx.gameplay_config_modified = modified;
        runtime.ctx.gameplay_config_revision =
            runtime.ctx.gameplay_config_revision.saturating_add(1);
        let payload = ReloadStatusPayload::new(
            "gameplay_config",
            manager.world.active.label(),
            if error.is_some() {
                "fallback"
            } else {
                "reloaded"
            },
            GAMEPLAY_CONFIG_PATH,
            runtime.ctx.gameplay_config_revision,
            true,
            modified,
            error,
            None,
        );
        manager
            .channels
            .overlay_console_lines
            .push(format!("[world] {}", payload.line()));
    }

    runtime.ctx.delta_seconds = frame_delta_seconds.max(0.0);
}

pub(crate) fn publish_scene_state(
    manager: &SceneManager,
    scene_state: &mut SceneRuntimeState,
    gameplay: &mut GameplayRuntimeConfig,
    overlay: &mut UiOverlayState,
) {
    let (scene_state_value, gameplay_value, overlay_value) = snapshot_public_scene_state(manager);
    *scene_state = scene_state_value;
    *gameplay = gameplay_value;
    *overlay = overlay_value;
}

pub(crate) fn republish_scene_resources(world: &mut runen_ecs::World) -> Result<()> {
    let Some((scene_state_value, gameplay_value, overlay_value)) = world
        .resource::<SceneResource>()
        .ok()
        .and_then(|scene_resource| scene_resource.manager.as_ref())
        .map(snapshot_public_scene_state)
    else {
        return Ok(());
    };

    if let Ok(scene_state) = world.resource_mut::<SceneRuntimeState>() {
        *scene_state = scene_state_value;
    }
    if let Ok(gameplay) = world.resource_mut::<GameplayRuntimeConfig>() {
        *gameplay = gameplay_value;
    }
    if let Ok(overlay) = world.resource_mut::<UiOverlayState>() {
        *overlay = overlay_value;
    }
    Ok(())
}
