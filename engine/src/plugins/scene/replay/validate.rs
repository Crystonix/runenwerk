use super::super::runtime::{apply_overlay_messages, republish_scene_resources};
use super::super::snapshot::restore_scene_simulation_snapshot;
use super::super::{SceneManager, SceneResource};
use super::codec::{SceneReplayArchive, SceneReplayInputFrameV2};
use super::playback::replay_scene_frame;
use crate::runtime::{SimulationTick, WindowState};
use anyhow::{Result, anyhow};
use engine_replay::{ReplayJournalFrame, ReplayValidationReport};

pub(crate) fn validate_scene_replay(
    world: &mut runen_ecs::World,
    archive: &SceneReplayArchive,
    target_tick: SimulationTick,
) -> Result<ReplayValidationReport> {
    let checkpoint = archive
        .checkpoints
        .iter()
        .filter(|checkpoint| checkpoint.meta.tick.0 <= target_tick.0)
        .max_by_key(|checkpoint| checkpoint.meta.tick.0)
        .cloned()
        .ok_or_else(|| anyhow!("no replay checkpoint available for tick {}", target_tick.0))?;
    let frames: Vec<ReplayJournalFrame<SceneReplayInputFrameV2>> = archive
        .journal
        .iter()
        .filter(|frame| frame.tick.0 > checkpoint.meta.tick.0 && frame.tick.0 <= target_tick.0)
        .cloned()
        .collect();

    let report = {
        let window = world.resource::<WindowState>().ok().cloned();
        let scene_resource = world
            .resource_mut::<SceneResource>()
            .map_err(|_| anyhow!("ScenePlugin resource is not available"))?;
        if scene_resource.manager.is_none() {
            let window = window.ok_or_else(|| anyhow!("WindowState is not available"))?;
            scene_resource.manager = Some(SceneManager::new(&window)?);
        }
        let manager = scene_resource
            .manager
            .as_mut()
            .ok_or_else(|| anyhow!("scene manager is not initialized"))?;
        restore_scene_simulation_snapshot(manager, &checkpoint.snapshot)?;
        let mut report = ReplayValidationReport::default();
        for frame in &frames {
            let command = frame
                .commands
                .first()
                .ok_or_else(|| anyhow!("replay journal frame {} has no commands", frame.tick.0))?;
            let actual = replay_scene_frame(manager, command)?;
            if let Some(expected) = frame.post_hash
                && expected != actual
            {
                report
                    .mismatches
                    .push(engine_replay::ReplayMismatch::TickHashMismatch {
                        tick: frame.tick,
                        expected,
                        actual,
                    });
            }
        }
        apply_overlay_messages(manager);
        report
    };

    if let Ok(tick) = world.resource_mut::<SimulationTick>() {
        *tick = target_tick;
    }
    republish_scene_resources(world)?;
    Ok(report)
}
