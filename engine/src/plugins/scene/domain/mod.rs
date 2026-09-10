use crate::plugins::scene::ui::{
    ConsoleUiRuntimeState, initialize_console_ui, load_console_template,
};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::time::SystemTime;

pub mod config;
pub mod lifecycle;
pub mod registry;

pub use config::{
    GAMEPLAY_CONFIG_PATH, GameplayConfig, gameplay_config_modified, load_gameplay_config,
    load_gameplay_config_with_modified, load_gameplay_config_with_modified_and_error,
};
pub use lifecycle::{SceneLifecycleEvent, SceneLifecyclePhase};
pub use registry::{SceneDescriptor, SceneRegistry};

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SceneLayer {
    World,
    OverlayUi,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SceneId {
    GameplayStub,
    HubStub,
    ConsoleUi,
    HudUi,
    InventoryUi,
}

impl SceneId {
    pub fn layer(self) -> SceneLayer {
        match self {
            Self::GameplayStub | Self::HubStub => SceneLayer::World,
            Self::ConsoleUi | Self::HudUi | Self::InventoryUi => SceneLayer::OverlayUi,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::GameplayStub => "gameplay_stub",
            Self::HubStub => "hub_stub",
            Self::ConsoleUi => "console_ui",
            Self::HudUi => "hud_ui",
            Self::InventoryUi => "inventory_ui",
        }
    }

    pub fn from_label(label: &str) -> Option<Self> {
        match label.trim().to_ascii_lowercase().as_str() {
            "gameplay_stub" => Some(Self::GameplayStub),
            "hub_stub" => Some(Self::HubStub),
            "console_ui" => Some(Self::ConsoleUi),
            "hud_ui" => Some(Self::HudUi),
            "inventory_ui" => Some(Self::InventoryUi),
            _ => None,
        }
    }

    pub fn next_overlay(self) -> Self {
        match self {
            Self::ConsoleUi => Self::HudUi,
            Self::HudUi => Self::InventoryUi,
            Self::InventoryUi => Self::ConsoleUi,
            Self::GameplayStub | Self::HubStub => Self::ConsoleUi,
        }
    }

    pub fn previous_overlay(self) -> Self {
        match self {
            Self::ConsoleUi => Self::InventoryUi,
            Self::HudUi => Self::ConsoleUi,
            Self::InventoryUi => Self::HudUi,
            Self::GameplayStub | Self::HubStub => Self::ConsoleUi,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SceneSlot {
    pub active: SceneId,
    pub paused: bool,
    pub visible: bool,
}

impl SceneSlot {
    pub fn new(active: SceneId) -> Self {
        Self {
            active,
            paused: false,
            visible: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SceneCommand {
    ReplaceWorld(SceneId),
    ReplaceWorldByLabel(String),
    ReplaceOverlay(SceneId),
    ReplaceOverlayByLabel(String),
    PushOverlay(SceneId),
    PushOverlayByLabel(String),
    PopOverlay,
    PauseWorld(bool),
}

#[derive(Debug, Default, Clone)]
pub struct SceneTransitionResult {
    pub world_changed: bool,
    pub overlay_changed: bool,
    pub world_pause_changed: bool,
}

#[derive(
    Debug,
    Copy,
    Clone,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    runen_ecs::Component,
    runen_ecs::Resource,
)]
pub struct WorldFrameCounter {
    pub value: u64,
}

#[derive(
    Debug, Copy, Clone, PartialEq, Serialize, Deserialize, runen_ecs::Component, runen_ecs::Resource,
)]
pub struct WorldDebugPosition {
    pub x: f32,
    pub y: f32,
}

#[derive(
    Debug, Copy, Clone, PartialEq, Serialize, Deserialize, runen_ecs::Component, runen_ecs::Resource,
)]
pub struct WorldDebugVelocity {
    pub x: f32,
    pub y: f32,
}

pub struct WorldSceneContext {
    pub world: runen_ecs::World,
    pub world_scene_label: String,
    pub gameplay_config: GameplayConfig,
    pub delta_seconds: f32,
    pub fixed_step_seconds: f32,
    pub fixed_step_accumulator: f32,
    pub gameplay_config_modified: Option<SystemTime>,
    pub gameplay_config_revision: u64,
    pub overlay_consumed: bool,
    pub overlay_scene_label: String,
    pub player_move_x: f32,
    pub player_move_y: f32,
    pub camera_yaw: f32,
    pub camera_pitch: f32,
    pub camera_distance: f32,
    pub tick_entity: runen_ecs::Entity,
    pub debug_entity: runen_ecs::Entity,
    pub frame_count: u64,
    pub enemy_kills: u32,
    pub outbound_notifications: Vec<WorldToOverlayMessage>,
}

pub struct WorldSceneRuntime {
    pub ctx: WorldSceneContext,
}

impl WorldSceneRuntime {
    /// Executes the fixed Engine-owned scene update contract in explicit order.
    pub fn run(&mut self) -> Result<()> {
        world_scene_input_gate_system(&mut self.ctx)?;
        world_scene_tick_system(&mut self.ctx)?;
        world_scene_debug_motion_system(&mut self.ctx)?;
        Ok(())
    }
}

pub struct OverlaySceneRuntime {
    pub world: runen_ecs::World,
    pub ui: ConsoleUiRuntimeState,
}

#[derive(Debug, Default, Clone)]
pub struct SceneChannels {
    pub overlay_submit: Vec<OverlaySubmitMessage>,
    pub overlay_command_inputs: Vec<OverlayCommandInput>,
    pub world_to_overlay: Vec<WorldToOverlayMessage>,
    pub overlay_console_lines: Vec<String>,
    pub lifecycle_events: Vec<SceneLifecycleEvent>,
}

#[derive(Debug, Clone)]
pub enum OverlaySubmitMessage {
    Line(String),
}

#[derive(Debug, Clone)]
pub enum OverlayCommandInput {
    Line(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WorldToOverlayMessage {
    Tick {
        tick: u64,
        overlay: String,
    },
    Combat {
        source: String,
        target: String,
        damage: u32,
        critical: bool,
    },
    Loot {
        item: String,
        amount: u32,
        rarity: String,
    },
    Quest {
        quest: String,
        state: QuestState,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QuestState {
    Started,
    Progress { current: u32, goal: u32 },
    Completed,
}

pub fn build_overlay_runtime(
    scene: SceneId,
    screen_size: (f32, f32),
    scale: f32,
    registry: &SceneRegistry,
) -> Result<OverlaySceneRuntime> {
    let mut world = runen_ecs::World::new();
    let mut ui = initialize_console_ui(&mut world)?;
    ui.screen_size = screen_size;
    ui.scale = scale;
    if let Some(path) = registry.ui_template_path(scene) {
        let template = std::path::Path::new(path);
        if template.exists() {
            load_console_template(&mut world, &mut ui, template)?;
        }
    }
    Ok(OverlaySceneRuntime { world, ui })
}

fn world_scene_input_gate_system(ctx: &mut WorldSceneContext) -> Result<()> {
    if ctx.overlay_consumed {
        return Ok(());
    }
    Ok(())
}

fn world_scene_tick_system(ctx: &mut WorldSceneContext) -> Result<()> {
    if ctx.overlay_consumed {
        return Ok(());
    }
    if let Ok(mut entity) = ctx.world.entity_mut(ctx.tick_entity)
        && let Some(mut counter) = entity.get_mut::<WorldFrameCounter>()
    {
        counter.value = counter.value.saturating_add(1);
        ctx.frame_count = counter.value;
        if counter.value % 60 == 0 {
            ctx.outbound_notifications
                .push(WorldToOverlayMessage::Tick {
                    tick: counter.value,
                    overlay: ctx.overlay_scene_label.clone(),
                });
        }
        if ctx.world_scene_label == SceneId::HubStub.label() && counter.value % 180 == 0 {
            ctx.outbound_notifications
                .push(WorldToOverlayMessage::Loot {
                    item: "Rested".to_string(),
                    amount: 1,
                    rarity: "hub".to_string(),
                });
        }
        if ctx.world_scene_label == SceneId::HubStub.label() && counter.value % 300 == 0 {
            ctx.outbound_notifications
                .push(WorldToOverlayMessage::Quest {
                    quest: "Prepare For Expedition".to_string(),
                    state: QuestState::Started,
                });
        }
    }
    Ok(())
}

fn world_scene_debug_motion_system(ctx: &mut WorldSceneContext) -> Result<()> {
    if ctx.overlay_consumed {
        return Ok(());
    }
    let mut velocity = ctx
        .world
        .get::<WorldDebugVelocity>(ctx.debug_entity)
        .copied()
        .unwrap_or(WorldDebugVelocity { x: 0.0, y: 0.0 });
    let sim_step = ctx.delta_seconds.clamp(0.0, 0.25);
    if let Ok(mut entity) = ctx.world.entity_mut(ctx.debug_entity)
        && let Some(mut position) = entity.get_mut::<WorldDebugPosition>()
    {
        position.x += velocity.x * sim_step;
        position.y += velocity.y * sim_step;
        let min_x = ctx.gameplay_config.bounds.min_x;
        let max_x = ctx.gameplay_config.bounds.max_x;
        let min_y = ctx.gameplay_config.bounds.min_y;
        let max_y = ctx.gameplay_config.bounds.max_y;
        if position.x < min_x || position.x > max_x {
            velocity.x = -velocity.x;
            position.x = position.x.clamp(min_x, max_x);
        }
        if position.y < min_y || position.y > max_y {
            velocity.y = -velocity.y;
            position.y = position.y.clamp(min_y, max_y);
        }
    }
    if let Ok(mut entity) = ctx.world.entity_mut(ctx.debug_entity)
        && let Some(mut velocity_mut) = entity.get_mut::<WorldDebugVelocity>()
    {
        *velocity_mut = velocity;
    }
    Ok(())
}

pub fn build_world_scene_runtime(scene: SceneId) -> Result<WorldSceneRuntime> {
    let (gameplay_config, gameplay_config_modified) = load_gameplay_config_with_modified();
    let mut world = runen_ecs::World::new();
    let tick_entity = world.spawn(WorldFrameCounter { value: 0 })?;
    let debug_entity = world.spawn((
        WorldDebugPosition { x: 0.0, y: 0.0 },
        WorldDebugVelocity { x: 1.25, y: 0.75 },
    ))?;
    let ctx = WorldSceneContext {
        world,
        world_scene_label: scene.label().to_string(),
        camera_yaw: gameplay_config.camera.initial_yaw,
        camera_pitch: gameplay_config.camera.initial_pitch,
        camera_distance: gameplay_config.camera.initial_distance,
        gameplay_config,
        delta_seconds: 1.0 / 60.0,
        fixed_step_seconds: 1.0 / 60.0,
        fixed_step_accumulator: 0.0,
        gameplay_config_modified,
        gameplay_config_revision: 0,
        overlay_consumed: false,
        overlay_scene_label: SceneId::ConsoleUi.label().to_string(),
        player_move_x: 0.0,
        player_move_y: 0.0,
        tick_entity,
        debug_entity,
        frame_count: 0,
        enemy_kills: 0,
        outbound_notifications: Vec::new(),
    };

    Ok(WorldSceneRuntime { ctx })
}

#[cfg(test)]
mod tests {
    use super::{
        SceneId, WorldDebugPosition, WorldFrameCounter, WorldToOverlayMessage,
        build_overlay_runtime, build_world_scene_runtime, load_gameplay_config,
    };

    #[test]
    fn gameplay_config_loads_with_positive_core_values() {
        let cfg = load_gameplay_config();
        assert!(cfg.player.health > 0);
        assert!(cfg.enemy.health > 0);
        assert!(cfg.enemies_per_chunk > 0);
        assert!(cfg.chunk_size > 0.0);
        assert!(cfg.bounds.max_x > cfg.bounds.min_x);
        assert!(cfg.bounds.max_y > cfg.bounds.min_y);
    }

    #[test]
    fn overlay_cycle_includes_inventory_scene() {
        assert_eq!(SceneId::ConsoleUi.next_overlay(), SceneId::HudUi);
        assert_eq!(SceneId::HudUi.next_overlay(), SceneId::InventoryUi);
        assert_eq!(SceneId::InventoryUi.next_overlay(), SceneId::ConsoleUi);
        assert_eq!(SceneId::ConsoleUi.previous_overlay(), SceneId::InventoryUi);
    }

    #[test]
    fn world_scene_runtime_blocks_when_overlay_consumed() {
        let mut runtime =
            build_world_scene_runtime(SceneId::GameplayStub).expect("runtime should build");
        runtime.ctx.overlay_consumed = true;
        let initial_pos = runtime
            .ctx
            .world
            .get::<WorldDebugPosition>(runtime.ctx.debug_entity)
            .expect("position component should exist")
            .to_owned();
        runtime.run().expect("world scene runtime should run");
        assert_eq!(runtime.ctx.frame_count, 0);
        let blocked_pos = runtime
            .ctx
            .world
            .get::<WorldDebugPosition>(runtime.ctx.debug_entity)
            .expect("position component should exist")
            .to_owned();
        assert_eq!(blocked_pos.x, initial_pos.x);
        assert_eq!(blocked_pos.y, initial_pos.y);

        runtime.ctx.overlay_consumed = false;
        runtime.run().expect("world scene runtime should run");
        assert_eq!(runtime.ctx.frame_count, 1);
        let value = runtime
            .ctx
            .world
            .get::<WorldFrameCounter>(runtime.ctx.tick_entity)
            .expect("counter component should exist")
            .value;
        assert_eq!(value, 1);
        let moved_pos = runtime
            .ctx
            .world
            .get::<WorldDebugPosition>(runtime.ctx.debug_entity)
            .expect("position component should exist")
            .to_owned();
        assert_ne!(moved_pos.x, blocked_pos.x);
    }

    #[test]
    fn world_scene_runtime_emits_notifications_on_tick_boundaries() {
        let mut runtime =
            build_world_scene_runtime(SceneId::GameplayStub).expect("runtime should build");
        runtime.ctx.overlay_consumed = false;
        for _ in 0..60 {
            runtime.run().expect("world scene runtime should run");
        }
        assert_eq!(runtime.ctx.frame_count, 60);
        assert!(
            runtime
                .ctx
                .outbound_notifications
                .iter()
                .any(|msg| matches!(msg, WorldToOverlayMessage::Tick { tick: 60, .. }))
        );
    }

    #[test]
    fn overlay_runtime_initializes_console_ui() {
        let registry = super::SceneRegistry::load();
        let runtime = build_overlay_runtime(SceneId::ConsoleUi, (1280.0, 720.0), 1.0, &registry)
            .expect("overlay runtime should build");
        assert_eq!(runtime.ui.screen_size, (1280.0, 720.0));
        assert_eq!(runtime.ui.scale, 1.0);
        assert!(
            runtime
                .world
                .get::<crate::plugins::scene::ui::UiNode>(runtime.ui.root)
                .is_some()
        );
    }
}
