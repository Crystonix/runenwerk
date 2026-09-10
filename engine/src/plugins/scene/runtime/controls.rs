use super::normalize_scene_label_alias;
use crate::WindowState;
use crate::plugins::{SceneManager, SceneResource};
use crate::prelude::domain::{SceneCommand, SceneId, SceneLayer};
use anyhow::{Result, anyhow};

// Owner: Engine Scene Plugin - Runtime Controls
fn with_scene_manager_mut<T>(
    world: &mut runen_ecs::World,
    f: impl FnOnce(&mut SceneManager) -> Result<T>,
) -> Result<T> {
    if !world.has_resource::<SceneResource>() {
        return Err(anyhow!("ScenePlugin is not installed"));
    }
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
        .ok_or_else(|| anyhow!("scene manager failed to initialize"))?;
    f(manager)
}

pub fn switch_scene_by_id(world: &mut runen_ecs::World, scene_id: &str) -> Result<bool> {
    let normalized = normalize_scene_label_alias(scene_id);
    let Some(scene) = SceneId::from_label(&normalized) else {
        return Ok(false);
    };
    with_scene_manager_mut(world, |manager| {
        match scene.layer() {
            SceneLayer::World => {
                manager.queue(SceneCommand::ReplaceWorldByLabel(normalized));
                manager.queue(SceneCommand::PauseWorld(false));
            }
            SceneLayer::OverlayUi => {
                manager.queue(SceneCommand::ReplaceOverlayByLabel(normalized));
                manager.queue(SceneCommand::PauseWorld(true));
            }
        }
        Ok(true)
    })
}

pub fn set_world_by_id(world: &mut runen_ecs::World, scene_id: &str) -> Result<bool> {
    let normalized = normalize_scene_label_alias(scene_id);
    let Some(scene) = SceneId::from_label(&normalized) else {
        return Ok(false);
    };
    if scene.layer() != SceneLayer::World {
        return Ok(false);
    }
    with_scene_manager_mut(world, |manager| {
        manager.queue(SceneCommand::ReplaceWorldByLabel(normalized));
        manager.queue(SceneCommand::PauseWorld(false));
        Ok(true)
    })
}

pub fn push_overlay_by_id(world: &mut runen_ecs::World, scene_id: &str) -> Result<bool> {
    let normalized = normalize_scene_label_alias(scene_id);
    let Some(scene) = SceneId::from_label(&normalized) else {
        return Ok(false);
    };
    if scene.layer() != SceneLayer::OverlayUi {
        return Ok(false);
    }
    with_scene_manager_mut(world, |manager| {
        manager.queue(SceneCommand::PushOverlayByLabel(normalized));
        manager.queue(SceneCommand::PauseWorld(true));
        Ok(true)
    })
}

pub fn pop_overlay(world: &mut runen_ecs::World) -> Result<()> {
    with_scene_manager_mut(world, |manager| {
        manager.queue(SceneCommand::PopOverlay);
        manager.queue(SceneCommand::PauseWorld(false));
        Ok(())
    })
}

pub fn set_world_paused(world: &mut runen_ecs::World, paused: bool) -> Result<()> {
    with_scene_manager_mut(world, |manager| {
        manager.queue(SceneCommand::PauseWorld(paused));
        Ok(())
    })
}

pub fn toggle_world_pause(world: &mut runen_ecs::World) -> Result<()> {
    with_scene_manager_mut(world, |manager| {
        manager.queue(SceneCommand::PauseWorld(!manager.world.paused));
        Ok(())
    })
}
