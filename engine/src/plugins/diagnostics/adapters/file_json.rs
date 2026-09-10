use crate::plugins::diagnostics::core::plan::ResolvedDiagnosticsPlan;
use crate::plugins::diagnostics::core::store::DiagnosticsReportStoreResource;
use crate::plugins::diagnostics::core::validate::validate_before_persist;
use crate::runtime::{Res, ResMut};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, runen_ecs::Component, runen_ecs::Resource)]
pub struct DiagnosticsFileAdapterStateResource {
    pub run_id: String,
    pub persisted_report_ids: BTreeSet<String>,
}

impl Default for DiagnosticsFileAdapterStateResource {
    fn default() -> Self {
        Self {
            run_id: default_run_id(),
            persisted_report_ids: BTreeSet::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct DiagnosticsRunManifest {
    run_id: String,
    report_files: Vec<String>,
}

pub fn persist_reports_and_manifest_system(
    plan: Res<ResolvedDiagnosticsPlan>,
    store: Res<DiagnosticsReportStoreResource>,
    mut state: ResMut<DiagnosticsFileAdapterStateResource>,
) {
    if !plan.enabled || !plan.file_json_enabled {
        return;
    }

    let run_root = plan.output_root.join(format!("run_{}", state.run_id));
    let reports_root = run_root.join("reports");
    if let Err(error) = fs::create_dir_all(&reports_root) {
        tracing::error!(
            path = %reports_root.display(),
            error = %error,
            "failed creating diagnostics reports output directory"
        );
        return;
    }

    let mut persisted_files = Vec::<String>::new();

    for report in &store.reports {
        if state
            .persisted_report_ids
            .contains(report.report_id.as_str())
        {
            continue;
        }

        if let Err(error) = validate_before_persist(report) {
            tracing::error!(
                report_id = %report.report_id,
                error = %error,
                "diagnostics report failed persistence validation"
            );
            continue;
        }

        let file_name = format!(
            "frame_{:010}_seq_{:010}.json",
            report.frame_index, report.report_sequence
        );
        let file_path = reports_root.join(file_name.clone());

        let payload = match serde_json::to_vec_pretty(report) {
            Ok(value) => value,
            Err(error) => {
                tracing::error!(
                    report_id = %report.report_id,
                    error = %error,
                    "failed serializing diagnostics report"
                );
                continue;
            }
        };

        if let Err(error) = fs::write(&file_path, payload) {
            tracing::error!(
                report_id = %report.report_id,
                path = %file_path.display(),
                error = %error,
                "failed writing diagnostics report file"
            );
            continue;
        }

        state.persisted_report_ids.insert(report.report_id.clone());
        persisted_files.push(format!("reports/{}", file_name));
    }

    if persisted_files.is_empty() {
        return;
    }

    let manifest_path = run_root.join("run_manifest.json");
    if let Err(error) = upsert_run_manifest(&manifest_path, state.run_id.as_str(), &persisted_files)
    {
        tracing::error!(
            path = %manifest_path.display(),
            error = %error,
            "failed writing diagnostics run manifest"
        );
        return;
    }

    tracing::info!(
        run_id = %state.run_id,
        output_root = %run_root.display(),
        persisted_report_count = persisted_files.len(),
        "diagnostics json persistence updated"
    );
}

fn upsert_run_manifest(
    manifest_path: &Path,
    run_id: &str,
    persisted_files: &[String],
) -> anyhow::Result<()> {
    let mut manifest = load_manifest(manifest_path).unwrap_or_else(|| DiagnosticsRunManifest {
        run_id: run_id.to_string(),
        report_files: Vec::new(),
    });

    for file in persisted_files {
        if !manifest.report_files.iter().any(|value| value == file) {
            manifest.report_files.push(file.clone());
        }
    }
    manifest.report_files.sort();

    let payload = serde_json::to_vec_pretty(&manifest)?;
    fs::write(manifest_path, payload)?;
    Ok(())
}

fn load_manifest(manifest_path: &Path) -> Option<DiagnosticsRunManifest> {
    if !manifest_path.exists() {
        return None;
    }
    let bytes = fs::read(manifest_path).ok()?;
    serde_json::from_slice::<DiagnosticsRunManifest>(&bytes).ok()
}

fn default_run_id() -> String {
    let unix_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_millis())
        .unwrap_or(0);
    format!("{}-{}", unix_ms, std::process::id())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_manifest_upsert_is_stable() {
        let temp_root = std::env::temp_dir().join(format!(
            "runenwerk_diag_manifest_test_{}_{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock should be after epoch")
                .as_nanos()
        ));
        fs::create_dir_all(&temp_root).expect("temp dir should be created");
        let manifest_path = temp_root.join("run_manifest.json");

        upsert_run_manifest(
            manifest_path.as_path(),
            "run-1",
            &["reports/a.json".to_string(), "reports/b.json".to_string()],
        )
        .expect("first upsert should succeed");
        upsert_run_manifest(
            manifest_path.as_path(),
            "run-1",
            &["reports/a.json".to_string()],
        )
        .expect("second upsert should succeed");

        let manifest = load_manifest(manifest_path.as_path()).expect("manifest should exist");
        assert_eq!(manifest.run_id, "run-1");
        assert_eq!(manifest.report_files.len(), 2);

        let _ = fs::remove_dir_all(temp_root);
    }
}
