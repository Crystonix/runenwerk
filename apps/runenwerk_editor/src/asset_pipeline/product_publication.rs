use asset::{AssetArtifactDescriptor, AssetArtifactId, AssetDiagnosticCode, AssetDiagnosticRecord};
use engine::runtime::{ProductPublicationRuntimeResource, PublicationBoundary};
use product::{
    FieldProductDiagnostic, FieldProductDiagnosticCode, ProductIdentity, ProductPublicationOutcome,
    ProductPublicationReport, ProductPublicationStatus, ratify_product_publication,
};
use world_sdf::{FieldProductCandidate, FieldProductId};

use crate::editor_app::RunenwerkEditorApp;

use super::FieldProductJobOutcome;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditorFieldProductPublication {
    pub publication: ProductPublicationOutcome,
    pub candidate: FieldProductCandidate,
    pub artifact: AssetArtifactDescriptor,
}

impl EditorFieldProductPublication {
    pub fn from_job_outcome(outcome: FieldProductJobOutcome) -> Self {
        Self {
            publication: outcome.publication,
            candidate: outcome.candidate,
            artifact: outcome.artifact,
        }
    }

    pub fn product_id(&self) -> ProductIdentity {
        ProductIdentity::new(self.candidate.descriptor.product_id.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditorFieldProductPublicationJournalEntry {
    pub product_id: FieldProductId,
    pub artifact_id: AssetArtifactId,
    pub status: ProductPublicationStatus,
}

pub fn publish_pending_field_product_publications(
    app: &mut RunenwerkEditorApp,
    publications: &mut ProductPublicationRuntimeResource,
    boundary: &PublicationBoundary,
) -> ProductPublicationReport {
    let pending = app.take_pending_field_product_publications();
    if pending.is_empty() {
        return ProductPublicationReport::default();
    }

    for pending_publication in &pending {
        publications.stage(pending_publication.publication.clone());
    }

    let journal_start = publications.journal().len();
    let report = publications.publish_staged(boundary);
    let published_entries = &publications.journal()[journal_start..];

    for diagnostic in &report.diagnostics {
        app.asset_catalog_runtime_mut()
            .record_diagnostic(asset_diagnostic_from_product_diagnostic(diagnostic));
    }

    for pending_publication in pending {
        if !published_entries.iter().any(|entry| {
            entry.product_job_id == pending_publication.publication.product_job.job_id
                && entry.stage_sequence == pending_publication.publication.stage_sequence
                && entry
                    .output_products
                    .contains(&pending_publication.product_id())
        }) {
            let ratification = ratify_product_publication(&pending_publication.publication);
            if ratification.has_blocking_issues() {
                for issue in ratification.iter() {
                    app.asset_catalog_runtime_mut().record_diagnostic(
                        AssetDiagnosticRecord::error(
                            AssetDiagnosticCode::RatificationRejected,
                            format!("field-product publication rejected: {}", issue.message()),
                        ),
                    );
                }
            }
            continue;
        }

        app.asset_catalog_runtime_mut()
            .catalog_mut()
            .insert_artifact(pending_publication.artifact.clone());
        app.asset_catalog_runtime_mut()
            .set_selected_field_product(Some(pending_publication.candidate.descriptor.clone()));
        app.record_field_product_publication(EditorFieldProductPublicationJournalEntry {
            product_id: pending_publication.candidate.descriptor.product_id,
            artifact_id: pending_publication.artifact.artifact_id,
            status: pending_publication.publication.status,
        });
        app.append_console_line(format!(
            "[product] published field product {} via publication boundary {}",
            pending_publication.candidate.descriptor.product_id.0, boundary.index
        ));
    }

    report
}

pub fn publication_diagnostic(message: impl Into<String>) -> FieldProductDiagnostic {
    FieldProductDiagnostic::blocking(FieldProductDiagnosticCode::FormationFailure, message)
}

fn asset_diagnostic_from_product_diagnostic(
    diagnostic: &FieldProductDiagnostic,
) -> AssetDiagnosticRecord {
    AssetDiagnosticRecord::error(
        AssetDiagnosticCode::RatificationRejected,
        diagnostic.message.clone(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use asset::{
        AssetKind, AssetRecord, FieldProductResolution, ImportSettings, SourceHash, asset_id,
        asset_source_id, import_job_id,
    };

    use crate::asset_pipeline::run_field_product_job;

    fn boundary() -> PublicationBoundary {
        PublicationBoundary::new(7, "Update", 0)
    }

    #[test]
    fn field_product_publication_updates_catalog_only_at_boundary() {
        let root = unique_temp_dir("runenwerk_editor_field_publication");
        let source = asset::AssetSourceDescriptor::new(
            asset_source_id(2),
            asset_id(1),
            AssetKind::SdfGraph,
            "assets/fields/brush.ron",
        )
        .with_hash(SourceHash::new("sha256", "abc"));
        let plan = asset::ImportPlan::deterministic(
            import_job_id(3),
            &source,
            ImportSettings::SdfGraph {
                resolution: FieldProductResolution::new(64, 64, 1),
            },
            AssetKind::FormedFieldProduct,
        );
        let outcome = run_field_product_job(&source, &plan, &root, &root.join(".cache"))
            .expect("field product job should produce a publication candidate");

        let mut app = RunenwerkEditorApp::new();
        app.asset_catalog_runtime_mut()
            .catalog_mut()
            .insert_asset_record(AssetRecord::new(
                source.asset_id,
                "brush",
                "Brush",
                AssetKind::SdfGraph,
            ));
        let artifact_id = outcome.artifact.artifact_id;
        app.queue_field_product_publication(EditorFieldProductPublication::from_job_outcome(
            outcome,
        ));
        let mut publications = ProductPublicationRuntimeResource::default();

        assert!(
            app.asset_catalog_runtime()
                .catalog()
                .artifact(artifact_id)
                .is_none()
        );
        assert_eq!(app.pending_field_product_publication_count(), 1);

        let report =
            publish_pending_field_product_publications(&mut app, &mut publications, &boundary());

        assert_eq!(report.published_count, 1);
        assert_eq!(app.pending_field_product_publication_count(), 0);
        assert!(
            app.asset_catalog_runtime()
                .catalog()
                .artifact(artifact_id)
                .is_some()
        );
        assert!(
            app.asset_catalog_runtime()
                .selected_field_product()
                .is_some()
        );
        assert_eq!(app.field_product_publication_journal().len(), 1);
        assert_eq!(publications.journal().len(), 1);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn rejected_field_product_publication_records_diagnostics_without_artifact() {
        let mut app = RunenwerkEditorApp::new();
        let mut job = product::ProductJobDescriptor::new(
            product::ProductJobId::new(10),
            product::ProductKind::new("invalid"),
            "test.producer",
            ProductIdentity::new(10),
            product::ProductScope::non_spatial("test"),
            product::ProductScaleBand::Preview,
        );
        job.output_products.clear();
        let publication = ProductPublicationOutcome::ready(job, [], 1);
        let chunk = runen_spatial::ChunkId::new(
            runen_spatial::WorldId::new(1),
            runen_spatial::ChunkCoord3 { x: 0, y: 0, z: 0 },
        );
        let candidate = FieldProductCandidate::new(world_sdf::FieldProductDescriptor::new(
            FieldProductId(10),
            world_sdf::FieldProductKind::ScalarDistance,
            world_sdf::FieldProductScope::from_chunks([chunk]),
            world_sdf::FieldProductLineage::new(1, "test.producer"),
        ));
        let artifact = AssetArtifactDescriptor::new(
            asset::asset_artifact_id(10),
            asset_id(10),
            AssetKind::FormedFieldProduct,
            asset::ArtifactPayloadKind::FormedFieldProduct {
                product_id: "10".to_string(),
            },
            asset::ArtifactCacheKey::new("invalid"),
        );

        app.queue_field_product_publication(EditorFieldProductPublication {
            publication,
            candidate,
            artifact,
        });
        let mut publications = ProductPublicationRuntimeResource::default();

        let report =
            publish_pending_field_product_publications(&mut app, &mut publications, &boundary());

        assert_eq!(report.rejected_count, 1);
        assert_eq!(app.field_product_publication_journal().len(), 0);
        assert!(!app.asset_catalog_runtime().diagnostics().is_empty());
    }

    #[test]
    fn field_product_publication_duplicate_product_id_matches_exact_published_job_entry() {
        let mut app = RunenwerkEditorApp::new();
        let chunk = runen_spatial::ChunkId::new(
            runen_spatial::WorldId::new(1),
            runen_spatial::ChunkCoord3 { x: 0, y: 0, z: 0 },
        );
        let candidate = FieldProductCandidate::new(world_sdf::FieldProductDescriptor::new(
            FieldProductId(10),
            world_sdf::FieldProductKind::ScalarDistance,
            world_sdf::FieldProductScope::from_chunks([chunk]),
            world_sdf::FieldProductLineage::new(1, "test.producer"),
        ));
        let artifact = AssetArtifactDescriptor::new(
            asset::asset_artifact_id(10),
            asset_id(10),
            AssetKind::FormedFieldProduct,
            asset::ArtifactPayloadKind::FormedFieldProduct {
                product_id: "10".to_string(),
            },
            asset::ArtifactCacheKey::new("valid"),
        );
        let valid_job = product::ProductJobDescriptor::new(
            product::ProductJobId::new(10),
            product::ProductKind::new("valid"),
            "test.producer",
            ProductIdentity::new(10),
            product::ProductScope::non_spatial("test"),
            product::ProductScaleBand::Preview,
        );
        let valid_publication =
            ProductPublicationOutcome::ready(valid_job, [candidate.descriptor.product_core()], 10);
        app.queue_field_product_publication(EditorFieldProductPublication {
            publication: valid_publication,
            candidate: candidate.clone(),
            artifact,
        });

        let mut invalid_job = product::ProductJobDescriptor::new(
            product::ProductJobId::new(11),
            product::ProductKind::new("invalid"),
            "test.producer",
            ProductIdentity::new(10),
            product::ProductScope::non_spatial("test"),
            product::ProductScaleBand::Preview,
        );
        invalid_job.output_products.clear();
        app.queue_field_product_publication(EditorFieldProductPublication {
            publication: ProductPublicationOutcome::ready(invalid_job, [], 11),
            candidate,
            artifact: AssetArtifactDescriptor::new(
                asset::asset_artifact_id(11),
                asset_id(10),
                AssetKind::FormedFieldProduct,
                asset::ArtifactPayloadKind::FormedFieldProduct {
                    product_id: "10".to_string(),
                },
                asset::ArtifactCacheKey::new("invalid"),
            ),
        });
        let mut publications = ProductPublicationRuntimeResource::default();

        let report =
            publish_pending_field_product_publications(&mut app, &mut publications, &boundary());

        assert_eq!(report.published_count, 1);
        assert_eq!(report.rejected_count, 1);
        assert_eq!(app.field_product_publication_journal().len(), 1);
        assert_eq!(
            app.field_product_publication_journal()[0].artifact_id,
            asset::asset_artifact_id(10)
        );
        assert!(
            app.asset_catalog_runtime()
                .catalog()
                .artifact(asset::asset_artifact_id(11))
                .is_none()
        );
    }

    fn unique_temp_dir(label: &str) -> std::path::PathBuf {
        let mut root = std::env::temp_dir();
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time should be after unix epoch")
            .as_nanos();
        root.push(format!("{label}_{nanos}"));
        std::fs::create_dir_all(&root).expect("temp dir should be creatable");
        root
    }
}
