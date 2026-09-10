use super::helpers::build_shader_index;
use super::*;
use std::time::{Duration, Instant};

// Owner: Engine Render Shader Registry - Types and Resources
pub const DEFAULT_SHADER_ASSET_ROOT: &str = "assets/shaders";
pub const DEFAULT_SHADER_RELOAD_POLL_INTERVAL_MS: u64 = 500;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShaderReloadPollStatus {
    Disabled,
    Throttled,
    Polled,
}

impl ShaderReloadPollStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Disabled => "disabled",
            Self::Throttled => "throttled",
            Self::Polled => "polled",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShaderReloadPollReport {
    pub status: ShaderReloadPollStatus,
    pub elapsed_ms: f32,
    pub interval_ms: f32,
    pub force_reload: bool,
}

impl Default for ShaderReloadPollReport {
    fn default() -> Self {
        Self {
            status: ShaderReloadPollStatus::Disabled,
            elapsed_ms: 0.0,
            interval_ms: DEFAULT_SHADER_RELOAD_POLL_INTERVAL_MS as f32,
            force_reload: false,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct ShaderHandle(pub(super) usize);

impl ShaderHandle {
    pub fn index(self) -> usize {
        self.0
    }
}

#[derive(Debug, Clone, Default)]
pub struct ShaderAssetComponent {
    pub id: String,
    pub path: String,
    pub source: Option<String>,
    pub modified: Option<SystemTime>,
    pub revision: u64,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone)]
pub(super) struct ShaderRegistryConfigResource {
    pub(super) roots: Vec<String>,
    pub(super) watch_enabled: bool,
    pub(super) force_reload: bool,
    pub(super) poll_interval: Duration,
    pub(super) last_poll_at: Option<Instant>,
    pub(super) last_poll_report: ShaderReloadPollReport,
    pub(super) revision: u64,
}

impl Default for ShaderRegistryConfigResource {
    fn default() -> Self {
        Self {
            roots: vec![DEFAULT_SHADER_ASSET_ROOT.to_string()],
            watch_enabled: true,
            force_reload: true,
            poll_interval: Duration::from_millis(DEFAULT_SHADER_RELOAD_POLL_INTERVAL_MS),
            last_poll_at: None,
            last_poll_report: ShaderReloadPollReport::default(),
            revision: 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShaderRegistryEventKind {
    Discovered,
    Registered,
    PathUpdated,
    DuplicateId,
    Reloaded,
    SkippedEmpty,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShaderRegistryEvent {
    pub kind: ShaderRegistryEventKind,
    pub id: String,
    pub path: String,
    pub revision: u64,
    pub error: Option<String>,
    pub details: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ShaderStatus {
    pub handle: ShaderHandle,
    pub id: String,
    pub path: String,
    pub revision: u64,
    pub loaded: bool,
    pub modified: Option<SystemTime>,
    pub last_error: Option<String>,
}

#[derive(runen_ecs::Component, runen_ecs::Resource)]
pub struct ShaderRegistryResource {
    pub(super) assets: Vec<ShaderAssetComponent>,
    pub(super) by_id: HashMap<String, usize>,
    pub(super) config: ShaderRegistryConfigResource,
    pub(super) events: Vec<ShaderRegistryEvent>,
}

impl std::fmt::Debug for ShaderRegistryResource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ShaderRegistryResource")
            .field("shader_count", &self.shader_count())
            .field("roots", &self.roots())
            .field("watch_enabled", &self.watch_enabled())
            .field("revision", &self.revision())
            .finish()
    }
}

impl Clone for ShaderRegistryResource {
    fn clone(&self) -> Self {
        let assets = self.assets.clone();
        Self {
            by_id: build_shader_index(&assets),
            assets,
            config: self.config.clone(),
            events: Vec::new(),
        }
    }
}

impl Default for ShaderRegistryResource {
    fn default() -> Self {
        Self::new()
    }
}
