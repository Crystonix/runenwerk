use std::path::PathBuf;

#[derive(Debug, Clone, runen_ecs::Component, runen_ecs::Resource, Default)]
pub struct DiagnosticsAdapterConfig {
    pub stdout_enabled: bool,
    pub file_json_enabled: bool,
    pub console_enabled: bool,
}

#[derive(Debug, Clone, runen_ecs::Component, runen_ecs::Resource)]
pub struct DiagnosticsProducerConfig {
    pub render_enabled: bool,
    pub scheduler_enabled: bool,
    pub world_enabled: bool,
    pub net_enabled: bool,
    pub replay_enabled: bool,
    pub ecs_enabled: bool,
}

impl Default for DiagnosticsProducerConfig {
    fn default() -> Self {
        Self {
            render_enabled: true,
            scheduler_enabled: true,
            world_enabled: false,
            net_enabled: false,
            replay_enabled: false,
            ecs_enabled: false,
        }
    }
}

#[derive(Debug, Clone, runen_ecs::Component, runen_ecs::Resource)]
pub struct DiagnosticsConfigResource {
    pub enabled: bool,
    pub retention_max_reports: usize,
    pub output_root: PathBuf,
    pub adapters: DiagnosticsAdapterConfig,
    pub producers: DiagnosticsProducerConfig,
    pub live_info_interval_frames: u64,
    pub live_dedupe_enabled: bool,
    pub console_max_lines: usize,
}

impl Default for DiagnosticsConfigResource {
    fn default() -> Self {
        Self {
            enabled: true,
            retention_max_reports: 120,
            output_root: std::env::var("RUNENWERK_DIAGNOSTICS_OUTPUT_DIR")
                .map(PathBuf::from)
                .unwrap_or_else(|_| PathBuf::from("target/engine-diagnostics")),
            adapters: DiagnosticsAdapterConfig::default(),
            producers: DiagnosticsProducerConfig::default(),
            live_info_interval_frames: 120,
            live_dedupe_enabled: true,
            console_max_lines: 256,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diagnostics_defaults_match_locked_plan() {
        let config = DiagnosticsConfigResource::default();
        assert!(config.enabled);
        assert_eq!(config.retention_max_reports, 120);
        assert_eq!(
            config.output_root,
            PathBuf::from("target/engine-diagnostics")
        );
        assert!(!config.adapters.stdout_enabled);
        assert!(!config.adapters.file_json_enabled);
        assert!(!config.adapters.console_enabled);
        assert!(config.producers.render_enabled);
        assert!(config.producers.scheduler_enabled);
    }
}
