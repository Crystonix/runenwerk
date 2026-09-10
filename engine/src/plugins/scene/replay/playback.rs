use super::super::SceneManager;
use super::super::domain::{SceneChannels, build_world_scene_runtime};
use super::super::runtime::rebuild_overlay_stack;
use super::super::snapshot::capture_scene_simulation_snapshot;
use super::codec::{SceneReplayInputFrameV2, SceneSimulationCodec};
use anyhow::Result;
use engine_sim::{SimulationCodec, SimulationHash};

pub(crate) fn replay_scene_frame(
    manager: &mut SceneManager,
    frame: &SceneReplayInputFrameV2,
) -> Result<SimulationHash> {
    if manager.world != frame.world {
        manager.world = frame.world;
        manager.world_runtime = build_world_scene_runtime(frame.world.active)?;
        manager.channels = SceneChannels::default();
        manager.pending.clear();
    }
    if manager.overlays != frame.overlays {
        rebuild_overlay_stack(manager, &frame.overlays)?;
    }

    manager.world = frame.world;
    let ctx = &mut manager.world_runtime.ctx;
    ctx.world_scene_label = frame.world_scene_label.clone();
    ctx.overlay_scene_label = frame.overlay_scene_label.clone();
    ctx.gameplay_config = frame.gameplay_config.clone();
    ctx.gameplay_config_revision = frame.gameplay_config_revision;
    ctx.overlay_consumed = frame.overlay_consumed;
    ctx.player_move_x = frame.player_move_x;
    ctx.player_move_y = frame.player_move_y;
    ctx.camera_yaw = frame.camera_yaw;
    ctx.camera_pitch = frame.camera_pitch;
    ctx.camera_distance = frame.camera_distance;
    ctx.delta_seconds = frame.delta_seconds;
    ctx.fixed_step_seconds = frame.fixed_step_seconds;
    ctx.outbound_notifications.clear();

    if manager.world.visible && !manager.world.paused {
        manager.world_runtime.run()?;
        let outbound = std::mem::take(&mut manager.world_runtime.ctx.outbound_notifications);
        manager.channels.world_to_overlay.extend(outbound);
    }

    let snapshot = capture_scene_simulation_snapshot(manager)?;
    SceneSimulationCodec::hash(&snapshot)
}
