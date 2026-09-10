use super::projector::{project_report_summary, should_emit_projected_summary};
use crate::plugins::diagnostics::core::plan::ResolvedDiagnosticsPlan;
use crate::plugins::diagnostics::core::store::DiagnosticsReportStoreResource;
use crate::plugins::scene::SceneResource;
use crate::runtime::WorldMut;

#[derive(Debug, Clone, Default, runen_ecs::Component, runen_ecs::Resource)]
pub struct DiagnosticsConsoleAdapterStateResource {
    pub last_report_id: Option<String>,
    pub last_info_frame_index: Option<u64>,
    pub last_fingerprint: Option<u64>,
}

#[derive(Debug, Clone, Default, runen_ecs::Component, runen_ecs::Resource)]
pub struct DiagnosticsConsoleFeedResource {
    pub lines: Vec<String>,
}

impl DiagnosticsConsoleFeedResource {
    pub fn append_lines<I>(&mut self, lines: I, max_lines: usize)
    where
        I: IntoIterator<Item = String>,
    {
        self.lines.extend(lines);
        clamp_lines(&mut self.lines, max_lines);
    }
}

pub fn emit_console_feed_system(mut world: WorldMut) {
    let plan = match world.resource::<ResolvedDiagnosticsPlan>() {
        Ok(value) => value.clone(),
        Err(_) => return,
    };
    if !plan.enabled || !plan.console_enabled {
        return;
    }

    let projected = {
        let Ok(store) = world.resource::<DiagnosticsReportStoreResource>() else {
            return;
        };
        let Some(report) = store.latest_report() else {
            return;
        };
        project_report_summary(report)
    };

    let should_emit = {
        let Ok(state) = world.resource_mut::<DiagnosticsConsoleAdapterStateResource>() else {
            return;
        };

        if state
            .last_report_id
            .as_deref()
            .is_some_and(|value| value == projected.report_id)
        {
            return;
        }

        let should_emit = should_emit_projected_summary(
            &projected,
            state.last_info_frame_index,
            state.last_fingerprint,
            plan.live_info_interval_frames,
            plan.live_dedupe_enabled,
        );

        state.last_report_id = Some(projected.report_id.clone());
        state.last_fingerprint = Some(projected.fingerprint);
        if should_emit && projected.error_count == 0 && projected.warning_count == 0 {
            state.last_info_frame_index = Some(projected.frame_index);
        }

        should_emit
    };

    if !should_emit {
        return;
    }

    let mut lines = Vec::<String>::new();
    lines.push(projected.primary_line.clone());
    if projected.error_count > 0 || projected.warning_count > 0 {
        lines.extend(projected.detail_lines.clone());
    }

    if let Ok(feed) = world.resource_mut::<DiagnosticsConsoleFeedResource>() {
        feed.append_lines(lines.clone(), plan.console_max_lines);
    }

    if let Ok(scene_resource) = world.resource_mut::<SceneResource>()
        && let Some(manager) = scene_resource.manager.as_mut()
    {
        manager.channels.overlay_console_lines.extend(lines);
    }
}

fn clamp_lines(lines: &mut Vec<String>, max_lines: usize) {
    if max_lines == 0 {
        lines.clear();
        return;
    }
    let overflow = lines.len().saturating_sub(max_lines);
    if overflow > 0 {
        lines.drain(..overflow);
    }
}
