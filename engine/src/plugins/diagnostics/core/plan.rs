use super::config::DiagnosticsConfigResource;
use crate::runtime::{Res, ResMut};
use std::path::PathBuf;

#[derive(Debug, Clone, runen_ecs::Component, runen_ecs::Resource)]
pub struct ResolvedDiagnosticsPlan {
    pub enabled: bool,
    pub retention_max_reports: usize,
    pub output_root: PathBuf,
    pub stdout_enabled: bool,
    pub file_json_enabled: bool,
    pub console_enabled: bool,
    pub render_enabled: bool,
    pub scheduler_enabled: bool,
    pub world_enabled: bool,
    pub net_enabled: bool,
    pub replay_enabled: bool,
    pub ecs_enabled: bool,
    pub live_info_interval_frames: u64,
    pub live_dedupe_enabled: bool,
    pub console_max_lines: usize,
}

impl Default for ResolvedDiagnosticsPlan {
    fn default() -> Self {
        Self::from_config(&DiagnosticsConfigResource::default())
    }
}

impl ResolvedDiagnosticsPlan {
    pub fn from_config(config: &DiagnosticsConfigResource) -> Self {
        Self {
            enabled: config.enabled,
            retention_max_reports: config.retention_max_reports.max(1),
            output_root: config.output_root.clone(),
            stdout_enabled: config.adapters.stdout_enabled,
            file_json_enabled: config.adapters.file_json_enabled,
            console_enabled: config.adapters.console_enabled,
            render_enabled: config.producers.render_enabled,
            scheduler_enabled: config.producers.scheduler_enabled,
            world_enabled: config.producers.world_enabled,
            net_enabled: config.producers.net_enabled,
            replay_enabled: config.producers.replay_enabled,
            ecs_enabled: config.producers.ecs_enabled,
            live_info_interval_frames: config.live_info_interval_frames.max(1),
            live_dedupe_enabled: config.live_dedupe_enabled,
            console_max_lines: config.console_max_lines.max(1),
        }
    }

    pub fn producer_enabled(&self, producer_id: &str) -> bool {
        match producer_id {
            "render.inspect" => self.render_enabled,
            "scheduler.runtime" => self.scheduler_enabled,
            "world.runtime" => self.world_enabled,
            "net.runtime" => self.net_enabled,
            "replay.runtime" => self.replay_enabled,
            "ecs.runtime" => self.ecs_enabled,
            _ => true,
        }
    }
}

pub fn resolve_diagnostics_plan_system(
    config: Res<DiagnosticsConfigResource>,
    mut plan: ResMut<ResolvedDiagnosticsPlan>,
) {
    *plan = ResolvedDiagnosticsPlan::from_config(&config);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn producer_gate_is_render_and_scheduler_first() {
        let mut config = DiagnosticsConfigResource::default();
        config.producers.world_enabled = false;
        config.producers.net_enabled = false;
        let plan = ResolvedDiagnosticsPlan::from_config(&config);

        assert!(plan.producer_enabled("render.inspect"));
        assert!(plan.producer_enabled("scheduler.runtime"));
        assert!(!plan.producer_enabled("world.runtime"));
        assert!(!plan.producer_enabled("net.runtime"));
    }
}
