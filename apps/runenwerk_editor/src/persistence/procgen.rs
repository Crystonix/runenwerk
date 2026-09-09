//! File: apps/runenwerk_editor/src/persistence/procgen.rs
//! Purpose: Editor-owned procgen bake archive persistence.

use std::path::Path;

use anyhow::{Context, Result};
use editor_persistence::{decode_ron, encode_ron_pretty};
use product::ProductDescriptorCore;
use serde::{Deserialize, Serialize};
use world_ops::{OperationRecord, QuantizedAabb};
use world_sdf::FieldPreviewProduct;

use crate::runtime::procgen::{EditorProcgenBakeRecord, ProcgenRuntimeState};

pub const PROCGEN_BAKE_ARCHIVE_VERSION_V1: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProcgenBakeArchiveV1 {
    pub version: u32,
    pub determinism_key: String,
    pub source_revision: String,
    pub authored_overlay_generation: u64,
    pub operation_records: Vec<OperationRecord>,
    pub changed_regions: Vec<PersistedProcgenChangedRegionV1>,
    pub explanations: Vec<PersistedProcgenExplanationEntryV1>,
    pub field_preview_products: Vec<FieldPreviewProduct>,
    pub product_descriptors: Vec<ProductDescriptorCore>,
    pub diagnostics: Vec<PersistedProcgenBakeDiagnosticV1>,
}

impl ProcgenBakeArchiveV1 {
    pub fn from_bake_record(record: &EditorProcgenBakeRecord) -> Self {
        Self {
            version: PROCGEN_BAKE_ARCHIVE_VERSION_V1,
            determinism_key: record.determinism_key.clone(),
            source_revision: record.source_revision.clone(),
            authored_overlay_generation: record.authored_overlay_generation,
            operation_records: record.operation_records.clone(),
            changed_regions: record
                .changed_regions
                .iter()
                .map(|region| PersistedProcgenChangedRegionV1 {
                    target_id: region.target_id.clone(),
                    bounds_q: region.bounds_q,
                    product_id: region.product_id.map(|product_id| product_id.raw()),
                })
                .collect(),
            explanations: record
                .explanations
                .iter()
                .map(|entry| PersistedProcgenExplanationEntryV1 {
                    subject: entry.subject.clone(),
                    message: entry.message.clone(),
                })
                .collect(),
            field_preview_products: record.field_preview_products.clone(),
            product_descriptors: record.product_descriptors.clone(),
            diagnostics: record
                .diagnostics
                .iter()
                .map(|diagnostic| PersistedProcgenBakeDiagnosticV1 {
                    code: format!("{:?}", diagnostic.code),
                    message: diagnostic.message.clone(),
                })
                .collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersistedProcgenChangedRegionV1 {
    pub target_id: String,
    pub bounds_q: QuantizedAabb,
    pub product_id: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersistedProcgenExplanationEntryV1 {
    pub subject: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersistedProcgenBakeDiagnosticV1 {
    pub code: String,
    pub message: String,
}

pub fn procgen_bake_archive_from_runtime(
    state: &ProcgenRuntimeState,
) -> Option<ProcgenBakeArchiveV1> {
    state
        .last_bake()
        .map(ProcgenBakeArchiveV1::from_bake_record)
}

pub fn write_procgen_bake_archive(path: &Path, state: &ProcgenRuntimeState) -> Result<()> {
    let archive = procgen_bake_archive_from_runtime(state)
        .context("procgen runtime does not have an accepted bake to persist")?;
    let ron = encode_ron_pretty(&archive).context("failed to encode procgen bake archive")?;
    std::fs::write(path, ron)
        .with_context(|| format!("failed to write procgen bake archive: {}", path.display()))
}

pub fn read_procgen_bake_archive(path: &Path) -> Result<ProcgenBakeArchiveV1> {
    let source = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read procgen bake archive: {}", path.display()))?;
    let archive: ProcgenBakeArchiveV1 =
        decode_ron(&source).context("failed to decode procgen bake archive")?;
    if archive.version != PROCGEN_BAKE_ARCHIVE_VERSION_V1 {
        anyhow::bail!(
            "unsupported procgen bake archive version: {}",
            archive.version
        );
    }
    Ok(archive)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editor_app::RunenwerkEditorApp;
    use crate::runtime::procgen::bake_procgen_products;
    use engine::runtime::{ProductPublicationRuntimeResource, PublicationBoundary};

    fn boundary() -> PublicationBoundary {
        PublicationBoundary::new(17, "Update", 0)
    }

    fn temp_archive_path(name: &str) -> std::path::PathBuf {
        let mut path = std::env::temp_dir();
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time should be after unix epoch")
            .as_nanos();
        path.push(format!("runenwerk_{name}_{nanos}.procgen_bake.ron"));
        path
    }

    #[test]
    fn procgen_bake_archive_roundtrips_last_accepted_bake() {
        let mut app = RunenwerkEditorApp::new();
        let mut publications = ProductPublicationRuntimeResource::default();
        let bake_report = bake_procgen_products(&mut app, &mut publications, &boundary());
        assert!(bake_report.accepted);

        let path = temp_archive_path("roundtrip");
        write_procgen_bake_archive(&path, app.procgen_runtime())
            .expect("procgen bake archive should write");
        let archive = read_procgen_bake_archive(&path).expect("procgen bake archive should read");

        assert_eq!(archive.version, PROCGEN_BAKE_ARCHIVE_VERSION_V1);
        assert_eq!(archive.operation_records.len(), bake_report.operation_count);
        assert_eq!(
            archive.field_preview_products.len(),
            bake_report.product_count
        );
        assert_eq!(
            archive.product_descriptors.len(),
            bake_report.descriptor_count
        );
        assert!(!archive.changed_regions.is_empty());

        let _ = std::fs::remove_file(path);
    }
}
