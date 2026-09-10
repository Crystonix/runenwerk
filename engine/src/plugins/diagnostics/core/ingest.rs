use super::config::DiagnosticsConfigResource;
use super::model::{DiagnosticsAttachment, DiagnosticsEntry, DiagnosticsFrameReport};
use super::plan::ResolvedDiagnosticsPlan;
use super::store::DiagnosticsReportStoreResource;
use super::validate::validate_submission_contract;
use crate::runtime::WorldMut;
use std::collections::BTreeMap;

#[derive(Debug, Clone)]
pub struct DiagnosticsEntrySubmission {
    pub frame_index: u64,
    pub simulation_tick: u64,
    pub entry: DiagnosticsEntry,
    pub attachments: Vec<DiagnosticsAttachment>,
}

#[derive(Debug, Clone, Default, runen_ecs::Component, runen_ecs::Resource)]
pub struct DiagnosticsPendingReportsResource {
    pub by_frame_index: BTreeMap<u64, DiagnosticsFrameReport>,
}

pub fn submit_diagnostics_entry(
    world: &mut runen_ecs::World,
    submission: DiagnosticsEntrySubmission,
) -> anyhow::Result<()> {
    let plan = resolved_plan_snapshot(world);
    if !plan.enabled {
        return Ok(());
    }
    if !plan.producer_enabled(submission.entry.producer_id.as_str()) {
        return Ok(());
    }

    validate_submission_contract(&submission.entry, &submission.attachments)?;

    let Ok(pending) = world.resource_mut::<DiagnosticsPendingReportsResource>() else {
        return Ok(());
    };

    let report = pending
        .by_frame_index
        .entry(submission.frame_index)
        .or_insert_with(|| {
            DiagnosticsFrameReport::new_pending(submission.frame_index, submission.simulation_tick)
        });
    if report.simulation_tick == 0 {
        report.simulation_tick = submission.simulation_tick;
    }
    report.append_entry(submission.entry, submission.attachments);
    Ok(())
}

pub fn finalize_diagnostics_reports_system(mut world: WorldMut) {
    let plan = resolved_plan_snapshot(&world);
    if !plan.enabled {
        if let Ok(pending) = world.resource_mut::<DiagnosticsPendingReportsResource>() {
            pending.by_frame_index.clear();
        }
        return;
    }

    let mut pending = match world.remove_resource::<DiagnosticsPendingReportsResource>() {
        Some(value) => value,
        None => return,
    };
    if pending.by_frame_index.is_empty() {
        world.insert_resource(pending);
        return;
    }

    let mut store = world
        .remove_resource::<DiagnosticsReportStoreResource>()
        .unwrap_or_default();

    for (_, mut report) in std::mem::take(&mut pending.by_frame_index) {
        if report.entries.is_empty() {
            continue;
        }
        report.recompute_summary();
        let sequence = store.allocate_report_sequence();
        report.assign_identity(sequence);
        store.push_report(report, plan.retention_max_reports);
    }

    world.insert_resource(pending);
    world.insert_resource(store);
}

fn resolved_plan_snapshot(world: &runen_ecs::World) -> ResolvedDiagnosticsPlan {
    if let Ok(plan) = world.resource::<ResolvedDiagnosticsPlan>() {
        return plan.clone();
    }

    world
        .resource::<DiagnosticsConfigResource>()
        .map(ResolvedDiagnosticsPlan::from_config)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugins::diagnostics::core::model::{DiagnosticsSeverity, DiagnosticsStatus};

    #[test]
    fn ingest_merges_entries_into_same_frame_report() {
        let mut world = runen_ecs::World::new();
        world.insert_resource(DiagnosticsConfigResource::default());
        world.insert_resource(ResolvedDiagnosticsPlan::default());
        world.insert_resource(DiagnosticsPendingReportsResource::default());
        world.insert_resource(DiagnosticsReportStoreResource::default());

        let base_entry = DiagnosticsEntry {
            entry_id: "entry.1".to_string(),
            producer_id: "render.inspect".to_string(),
            domain_id: "render".to_string(),
            schema_id: "schema".to_string(),
            schema_version: 1,
            payload_json: serde_json::json!({"frame": 1}),
            severity: DiagnosticsSeverity::Info,
            status: DiagnosticsStatus::Ok,
            attachment_ids: Vec::new(),
        };

        submit_diagnostics_entry(
            &mut world,
            DiagnosticsEntrySubmission {
                frame_index: 7,
                simulation_tick: 33,
                entry: base_entry.clone(),
                attachments: Vec::new(),
            },
        )
        .expect("first submission should succeed");

        let mut second_entry = base_entry;
        second_entry.entry_id = "entry.2".to_string();
        submit_diagnostics_entry(
            &mut world,
            DiagnosticsEntrySubmission {
                frame_index: 7,
                simulation_tick: 33,
                entry: second_entry,
                attachments: Vec::new(),
            },
        )
        .expect("second submission should succeed");

        let pending = world
            .resource::<DiagnosticsPendingReportsResource>()
            .expect("pending resource should exist");
        let report = pending
            .by_frame_index
            .get(&7)
            .expect("frame report should be present");
        assert_eq!(report.entries.len(), 2);
    }
}
