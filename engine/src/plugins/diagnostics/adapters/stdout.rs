use super::projector::{project_report_summary, should_emit_projected_summary};
use crate::plugins::diagnostics::core::plan::ResolvedDiagnosticsPlan;
use crate::plugins::diagnostics::core::store::DiagnosticsReportStoreResource;
use crate::runtime::{Res, ResMut};

#[derive(Debug, Clone, Default, runen_ecs::Component, runen_ecs::Resource)]
pub struct DiagnosticsStdoutAdapterStateResource {
    pub last_report_id: Option<String>,
    pub last_info_frame_index: Option<u64>,
    pub last_fingerprint: Option<u64>,
}

pub fn emit_stdout_adapter_system(
    plan: Res<ResolvedDiagnosticsPlan>,
    store: Res<DiagnosticsReportStoreResource>,
    mut state: ResMut<DiagnosticsStdoutAdapterStateResource>,
) {
    if !plan.enabled || !plan.stdout_enabled {
        return;
    }

    let Some(report) = store.latest_report() else {
        return;
    };

    if state
        .last_report_id
        .as_deref()
        .is_some_and(|value| value == report.report_id)
    {
        return;
    }

    let projected = project_report_summary(report);
    let should_emit = should_emit_projected_summary(
        &projected,
        state.last_info_frame_index,
        state.last_fingerprint,
        plan.live_info_interval_frames,
        plan.live_dedupe_enabled,
    );

    state.last_report_id = Some(projected.report_id.clone());
    state.last_fingerprint = Some(projected.fingerprint);

    if !should_emit {
        return;
    }

    if projected.error_count > 0 {
        tracing::error!(
            frame = projected.frame_index,
            tick = projected.simulation_tick,
            entries = projected.total_entries,
            warnings = projected.warning_count,
            errors = projected.error_count,
            "{}",
            projected.primary_line
        );
        for line in &projected.detail_lines {
            tracing::error!(frame = projected.frame_index, "{}", line);
        }
        return;
    }

    if projected.warning_count > 0 {
        tracing::warn!(
            frame = projected.frame_index,
            tick = projected.simulation_tick,
            entries = projected.total_entries,
            warnings = projected.warning_count,
            errors = projected.error_count,
            "{}",
            projected.primary_line
        );
        for line in &projected.detail_lines {
            tracing::warn!(frame = projected.frame_index, "{}", line);
        }
        return;
    }

    state.last_info_frame_index = Some(projected.frame_index);
    tracing::info!(
        frame = projected.frame_index,
        tick = projected.simulation_tick,
        entries = projected.total_entries,
        warnings = projected.warning_count,
        errors = projected.error_count,
        "{}",
        projected.primary_line
    );
}
