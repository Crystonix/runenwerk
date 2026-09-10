use crate::plugins::render::renderer::GfxFrameTimings;
use runen_ecs::Component;
use std::collections::HashMap;
use std::path::Path;
use ui_render_data::UiFrame;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SceneRegistration {
    pub id: String,
    pub template_path: String,
}

impl SceneRegistration {
    pub fn new(id: impl Into<String>, template_path: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            template_path: template_path.into(),
        }
    }

    pub fn from_template_path(template_path: impl Into<String>) -> Self {
        let template_path = template_path.into();
        let id = Self::derive_id_from_template_path(&template_path);
        Self { id, template_path }
    }

    pub fn derive_id_from_template_path(template_path: &str) -> String {
        let raw = Path::new(template_path)
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or("scene");
        let mut out = String::new();
        let mut previous_was_sep = false;
        for ch in raw.chars() {
            if ch.is_ascii_alphanumeric() {
                out.push(ch.to_ascii_lowercase());
                previous_was_sep = false;
            } else if !previous_was_sep {
                out.push('_');
                previous_was_sep = true;
            }
        }
        let normalized = out.trim_matches('_');
        if normalized.is_empty() {
            "scene".to_string()
        } else {
            normalized.to_string()
        }
    }
}

impl From<String> for SceneRegistration {
    fn from(template_path: String) -> Self {
        Self::from_template_path(template_path)
    }
}

impl From<&str> for SceneRegistration {
    fn from(template_path: &str) -> Self {
        Self::from_template_path(template_path)
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct SceneHandle(u32);

impl SceneHandle {
    pub fn index(self) -> u32 {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegisteredScene {
    pub handle: SceneHandle,
    pub id: String,
    pub template_path: String,
}

#[derive(Debug, Clone, Default, Component, runen_ecs::Resource)]
pub struct SceneCatalog {
    scenes: Vec<RegisteredScene>,
    by_id: HashMap<String, SceneHandle>,
}

impl SceneCatalog {
    pub fn from_registrations(registrations: &[SceneRegistration]) -> Self {
        let mut catalog = Self::default();
        for registration in registrations {
            catalog.register(registration.id.clone(), registration.template_path.clone());
        }
        catalog
    }

    pub fn register(
        &mut self,
        id: impl Into<String>,
        template_path: impl Into<String>,
    ) -> SceneHandle {
        let id = id.into();
        if let Some(existing) = self.by_id.get(&id).copied() {
            tracing::warn!(scene_id = %id, "duplicate scene registration id ignored");
            return existing;
        }

        let handle = SceneHandle(self.scenes.len() as u32);
        let scene = RegisteredScene {
            handle,
            id: id.clone(),
            template_path: template_path.into(),
        };
        self.by_id.insert(id, handle);
        self.scenes.push(scene);
        handle
    }

    pub fn len(&self) -> usize {
        self.scenes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.scenes.is_empty()
    }

    pub fn handle(&self, id: &str) -> Option<SceneHandle> {
        self.by_id.get(id).copied()
    }

    pub fn get(&self, handle: SceneHandle) -> Option<&RegisteredScene> {
        self.scenes.get(handle.index() as usize)
    }

    pub fn iter(&self) -> impl Iterator<Item = &RegisteredScene> {
        self.scenes.iter()
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum StartupPhase {
    Loading,
    Ready,
}

#[derive(Debug, Copy, Clone, Component, runen_ecs::Resource)]
pub struct StartupState {
    pub phase: StartupPhase,
    pub stable_frames: u32,
    pub required_stable_frames: u32,
    pub elapsed_loading_seconds: f32,
    pub max_loading_seconds: f32,
}

impl Default for StartupState {
    fn default() -> Self {
        Self::loading()
    }
}

impl StartupState {
    pub const DEFAULT_REQUIRED_STABLE_FRAMES: u32 = 8;
    pub const DEFAULT_MAX_LOADING_SECONDS: f32 = 5.0;

    pub fn loading() -> Self {
        Self {
            phase: StartupPhase::Loading,
            stable_frames: 0,
            required_stable_frames: Self::DEFAULT_REQUIRED_STABLE_FRAMES,
            elapsed_loading_seconds: 0.0,
            max_loading_seconds: Self::DEFAULT_MAX_LOADING_SECONDS,
        }
    }

    pub fn ready() -> Self {
        let mut state = Self::loading();
        state.phase = StartupPhase::Ready;
        state.stable_frames = state.required_stable_frames;
        state
    }

    pub fn is_loading(&self) -> bool {
        self.phase == StartupPhase::Loading
    }

    pub fn is_ready(&self) -> bool {
        self.phase == StartupPhase::Ready
    }

    pub fn observe_render_warm_frame(&mut self, warm_frame: bool, delta_seconds: f32) -> bool {
        if self.is_ready() {
            return false;
        }

        self.elapsed_loading_seconds += delta_seconds.max(0.0);
        if warm_frame {
            self.stable_frames = self.stable_frames.saturating_add(1);
        } else {
            self.stable_frames = 0;
        }

        let stable_target = self.required_stable_frames.max(1);
        let timeout_limit = self.max_loading_seconds.max(0.0);
        let timed_out = self.elapsed_loading_seconds + 1.0e-6 >= timeout_limit;
        if self.stable_frames >= stable_target || timed_out {
            self.phase = StartupPhase::Ready;
            return true;
        }

        false
    }
}

#[derive(Debug, Clone, Copy, Component, runen_ecs::Resource)]
pub struct DebugMetricsState {
    pub visible: bool,
    pub fps_ema: f32,
    pub frame_ms_ema: f32,
    pub last_timings: Option<GfxFrameTimings>,
}

impl Default for DebugMetricsState {
    fn default() -> Self {
        Self {
            visible: false,
            fps_ema: 0.0,
            frame_ms_ema: 0.0,
            last_timings: None,
        }
    }
}

impl DebugMetricsState {
    pub fn observe_frame_delta(&mut self, delta_seconds: f32) {
        let safe_dt = delta_seconds.max(1.0 / 1000.0);
        let fps = (1.0 / safe_dt).clamp(0.0, 2000.0);
        let frame_ms = (safe_dt * 1000.0).clamp(0.0, 1000.0);
        let alpha = 0.12;
        if self.fps_ema <= f32::EPSILON {
            self.fps_ema = fps;
        } else {
            self.fps_ema = self.fps_ema + (fps - self.fps_ema) * alpha;
        }
        if self.frame_ms_ema <= f32::EPSILON {
            self.frame_ms_ema = frame_ms;
        } else {
            self.frame_ms_ema = self.frame_ms_ema + (frame_ms - self.frame_ms_ema) * alpha;
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Component, runen_ecs::Resource)]
pub struct GameplayRuntimeConfig {
    pub chunk_size: f32,
    pub chunk_load_radius: u32,
    pub infinite_world: bool,
}

impl Default for GameplayRuntimeConfig {
    fn default() -> Self {
        Self {
            chunk_size: 24.0,
            chunk_load_radius: 2,
            infinite_world: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Component, runen_ecs::Resource)]
pub struct SceneRuntimeState {
    pub world_scene_label: String,
    pub overlay_scene_label: String,
    pub overlay_visible: bool,
    pub world_paused: bool,
    pub enemy_kills: u32,
    pub gameplay: GameplayRuntimeConfig,
}

impl Default for SceneRuntimeState {
    fn default() -> Self {
        Self {
            world_scene_label: "gameplay_stub".to_string(),
            overlay_scene_label: "console_ui".to_string(),
            overlay_visible: false,
            world_paused: false,
            enemy_kills: 0,
            gameplay: GameplayRuntimeConfig::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Component, runen_ecs::Resource)]
pub struct UiOverlayState {
    pub screen_size: (f32, f32),
    pub scale: f32,
    pub debug_frame: UiFrame,
}

impl Default for UiOverlayState {
    fn default() -> Self {
        Self {
            screen_size: (1280.0, 720.0),
            scale: 1.0,
            debug_frame: UiFrame::default(),
        }
    }
}
