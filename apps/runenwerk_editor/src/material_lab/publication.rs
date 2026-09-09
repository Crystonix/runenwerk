use asset::{AssetArtifactDescriptor, AssetArtifactId, AssetDiagnosticCode, AssetDiagnosticRecord};
use engine::runtime::{ProductPublicationRuntimeResource, PublicationBoundary};
use material_graph::MaterialProductId;
use product::{
    FieldProductDiagnostic, FieldProductDiagnosticCode, ProductIdentity, ProductPublicationOutcome,
    ProductPublicationReport, ProductPublicationStatus, ratify_product_publication,
};

use crate::editor_app::RunenwerkEditorApp;
use crate::material_lab::{
    EditorMaterialPreviewProduct, EditorMaterialPreviewPublicationJournalEntry,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditorMaterialPreviewPublication {
    pub publication: Option<ProductPublicationOutcome>,
    pub preview: Option<EditorMaterialPreviewProduct>,
    pub artifact: AssetArtifactDescriptor,
    pub derived_artifacts: Vec<AssetArtifactDescriptor>,
    pub status: ProductPublicationStatus,
}

impl EditorMaterialPreviewPublication {
    pub fn ready(
        publication: ProductPublicationOutcome,
        preview: EditorMaterialPreviewProduct,
        artifact: AssetArtifactDescriptor,
        derived_artifacts: impl IntoIterator<Item = AssetArtifactDescriptor>,
    ) -> Self {
        Self {
            publication: Some(publication),
            preview: Some(preview),
            artifact,
            derived_artifacts: derived_artifacts.into_iter().collect(),
            status: ProductPublicationStatus::Ready,
        }
    }

    pub fn failed_preserved(artifact: AssetArtifactDescriptor) -> Self {
        Self {
            publication: None,
            preview: None,
            artifact,
            derived_artifacts: Vec::new(),
            status: ProductPublicationStatus::FailedPreserved,
        }
    }

    pub fn artifact_id(&self) -> AssetArtifactId {
        self.artifact.artifact_id
    }

    pub fn material_product_id(&self) -> Option<MaterialProductId> {
        self.preview
            .as_ref()
            .map(EditorMaterialPreviewProduct::product_id)
    }

    fn product_identity(&self) -> Option<ProductIdentity> {
        self.material_product_id()
            .map(|product_id| ProductIdentity::new(product_id.raw()))
    }
}

pub fn publish_pending_material_preview_publications(
    app: &mut RunenwerkEditorApp,
    publications: &mut ProductPublicationRuntimeResource,
    boundary: &PublicationBoundary,
) -> ProductPublicationReport {
    let pending = app.take_pending_material_preview_publications();
    if pending.is_empty() {
        return ProductPublicationReport::default();
    }

    for pending_publication in &pending {
        if let Some(publication) = &pending_publication.publication {
            publications.stage(publication.clone());
        }
    }

    let journal_start = publications.journal().len();
    let report = publications.publish_staged(boundary);
    let published_entries = &publications.journal()[journal_start..];

    for diagnostic in &report.diagnostics {
        app.asset_catalog_runtime_mut()
            .record_diagnostic(asset_diagnostic_from_product_diagnostic(diagnostic));
    }

    for pending_publication in pending {
        if let Some(publication) = &pending_publication.publication {
            let product_identity = pending_publication.product_identity();
            let was_published = published_entries.iter().any(|entry| {
                entry.product_job_id == publication.product_job.job_id
                    && entry.stage_sequence == publication.stage_sequence
                    && product_identity
                        .as_ref()
                        .is_some_and(|product_id| entry.output_products.contains(product_id))
            });
            if !was_published {
                let ratification = ratify_product_publication(publication);
                if ratification.has_blocking_issues() {
                    for issue in ratification.iter() {
                        app.asset_catalog_runtime_mut().record_diagnostic(
                            AssetDiagnosticRecord::error(
                                AssetDiagnosticCode::RatificationRejected,
                                format!(
                                    "material preview publication rejected: {}",
                                    issue.message()
                                ),
                            ),
                        );
                    }
                }
                continue;
            }
        }

        app.asset_catalog_runtime_mut()
            .catalog_mut()
            .insert_artifact(pending_publication.artifact.clone());
        for artifact in &pending_publication.derived_artifacts {
            app.asset_catalog_runtime_mut()
                .catalog_mut()
                .insert_artifact(artifact.clone());
        }
        let status = app
            .asset_catalog_runtime()
            .classify_artifact_reload(&pending_publication.artifact);
        app.asset_catalog_runtime_mut().record_reload_status(status);

        if let Some(preview) = pending_publication.preview.clone() {
            app.material_lab_runtime_mut().set_active_preview(preview);
        }
        app.record_material_preview_publication(EditorMaterialPreviewPublicationJournalEntry {
            artifact_id: pending_publication.artifact_id(),
            product_id: pending_publication.material_product_id(),
            status: pending_publication.status,
        });
        app.append_console_line(format!(
            "[material] preview publication {:?} artifact {} via publication boundary {}",
            pending_publication.status,
            pending_publication.artifact_id().raw(),
            boundary.index
        ));
    }

    report
}

pub fn material_publication_diagnostic(message: impl Into<String>) -> FieldProductDiagnostic {
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
