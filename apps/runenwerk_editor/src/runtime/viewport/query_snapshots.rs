use anyhow::Result;
use ecs::World;
use editor_viewport::{
    ArtifactObservationFrame, ExpressionFreshness, ExpressionProductDescriptor,
    ExpressionProductId, ExpressionSourceRealityClass, ProductAvailabilityState,
};
use engine::runtime::{PublicationBoundary, QuerySnapshotRuntimeResource};
use product::{
    FieldProductDiagnostic, FieldProductDiagnosticCode, ProductAuthorityClass,
    ProductConsumerClass, ProductDescriptorCore, ProductFamily, ProductFreshness, ProductIdentity,
    ProductKind, ProductLineage, ProductQueryPolicy, ProductResidency, ProductScaleBand,
    ProductScope, QuerySnapshotProductDescriptor, QuerySnapshotPublicationReport,
    QuerySnapshotPublicationStatus,
};

use crate::editor_app::RunenwerkEditorApp;
use crate::runtime::resources::EditorHostResource;
use crate::runtime::viewport::ViewportArtifactObservationResource;

use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditorViewportQuerySnapshotJournalEntry {
    pub publication_boundary_index: usize,
    pub product_id: ProductIdentity,
    pub status: QuerySnapshotPublicationStatus,
    pub source_generation: u64,
    pub response_generation: u64,
    pub diagnostic_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditorViewportQuerySnapshotSummary {
    pub published_count: usize,
    pub rejected_count: usize,
    pub preserved_count: usize,
    pub invalidated_count: usize,
    pub diagnostic_signature: Vec<String>,
}

impl EditorViewportQuerySnapshotSummary {
    fn from_report(report: &QuerySnapshotPublicationReport) -> Self {
        let mut diagnostic_signature = report
            .diagnostics
            .iter()
            .map(|diagnostic| {
                format!(
                    "{:?}:{:?}:{}",
                    diagnostic.code,
                    diagnostic.product_id.map(|id| id.raw()),
                    diagnostic.message
                )
            })
            .collect::<Vec<_>>();
        diagnostic_signature.sort();
        diagnostic_signature.dedup();
        Self {
            published_count: report.published_count,
            rejected_count: report.rejected_count,
            preserved_count: report.preserved_count,
            invalidated_count: report.invalidated_count,
            diagnostic_signature,
        }
    }
}

pub fn publish_viewport_query_snapshots_at_boundary(
    boundary: &PublicationBoundary,
    world: &mut World,
) -> Result<()> {
    let Some(observations) = world
        .resource::<ViewportArtifactObservationResource>()
        .ok()
        .cloned()
    else {
        return Ok(());
    };
    let Some(mut host) = world.remove_resource::<EditorHostResource>() else {
        return Ok(());
    };
    let Some(mut snapshots) = world.remove_resource::<QuerySnapshotRuntimeResource>() else {
        world.insert_resource(host);
        return Ok(());
    };

    publish_viewport_query_snapshots(&mut host.app, &observations, &mut snapshots, boundary);

    world.insert_resource(snapshots);
    world.insert_resource(host);
    Ok(())
}

pub fn publish_viewport_query_snapshots(
    app: &mut RunenwerkEditorApp,
    observations: &ViewportArtifactObservationResource,
    snapshots: &mut QuerySnapshotRuntimeResource,
    boundary: &PublicationBoundary,
) -> QuerySnapshotPublicationReport {
    let staged = build_viewport_query_snapshot_descriptors(observations);
    if staged.is_empty() {
        return QuerySnapshotPublicationReport::default();
    }

    snapshots.stage_all(staged);
    let report = snapshots.publish_staged(boundary);
    let published_entries = snapshots.last_published_entries().to_vec();

    for entry in &published_entries {
        app.record_viewport_query_snapshot(EditorViewportQuerySnapshotJournalEntry {
            publication_boundary_index: entry.publication_boundary_index,
            product_id: entry.product_id,
            status: entry.status,
            source_generation: entry.source_generation,
            response_generation: entry.response_generation,
            diagnostic_count: entry.diagnostics.len(),
        });
    }

    let summary = EditorViewportQuerySnapshotSummary::from_report(&report);
    if app.update_viewport_query_snapshot_summary(summary) {
        app.append_console_line(format!(
            "[query_snapshot] publication boundary {}: published={} rejected={} preserved={} invalidated={}",
            boundary.index,
            report.published_count,
            report.rejected_count,
            report.preserved_count,
            report.invalidated_count
        ));
        for diagnostic in report.diagnostics.iter().take(5) {
            app.append_console_warning(format!(
                "[query_snapshot] {:?}: {}",
                diagnostic.code, diagnostic.message
            ));
        }
    }

    report
}

pub fn build_viewport_query_snapshot_descriptors(
    observations: &ViewportArtifactObservationResource,
) -> Vec<QuerySnapshotProductDescriptor> {
    let source_generation = observations.generation();
    observations
        .viewport_ids()
        .filter_map(|viewport_id| observations.frame_for(viewport_id))
        .flat_map(|frame| frame_query_snapshot_descriptors(frame, source_generation))
        .collect()
}

fn frame_query_snapshot_descriptors(
    frame: &ArtifactObservationFrame,
    source_generation: u64,
) -> Vec<QuerySnapshotProductDescriptor> {
    selected_product_ids(frame)
        .into_iter()
        .filter_map(|product_id| {
            let descriptor = frame
                .available_products
                .iter()
                .find(|descriptor| descriptor.id == product_id)?;
            let availability = frame
                .availability_by_product
                .get(&descriptor.id)
                .copied()
                .unwrap_or(ProductAvailabilityState::Unavailable);
            let mut core = product_descriptor_for_viewport_product(
                frame,
                descriptor,
                availability,
                source_generation,
            );
            let mut snapshot = QuerySnapshotProductDescriptor::new(
                core.clone(),
                source_generation,
                source_generation,
                ProductQueryPolicy::StrictCurrentOnly,
            );
            if availability == ProductAvailabilityState::Unavailable {
                let diagnostic = unavailable_product_diagnostic(&core);
                core.diagnostics.push(diagnostic.clone());
                snapshot.descriptor = core;
                snapshot.diagnostics.push(diagnostic);
            }
            Some(snapshot)
        })
        .collect()
}

fn selected_product_ids(frame: &ArtifactObservationFrame) -> Vec<ExpressionProductId> {
    let mut selected = BTreeSet::new();
    if let Some(primary) = frame.selected_primary_product_id {
        selected.insert(primary);
    }
    selected.extend(frame.selected_overlay_product_ids.iter().copied());
    selected.into_iter().collect()
}

fn product_descriptor_for_viewport_product(
    frame: &ArtifactObservationFrame,
    descriptor: &ExpressionProductDescriptor,
    availability: ProductAvailabilityState,
    source_generation: u64,
) -> ProductDescriptorCore {
    let mut product = ProductDescriptorCore::new(
        ProductIdentity::new(descriptor.id.0),
        ProductFamily::Expression,
        ProductKind::new(format!("{:?}", descriptor.kind)),
        ProductScope::View {
            view_id: frame.viewport_id.0.to_string(),
        },
        ProductScaleBand::Preview,
        ProductLineage::new(descriptor.producer_label.clone(), source_generation)
            .with_source_revision(format!("reality:{}", descriptor.source_version.0)),
    );
    product.freshness = product_freshness(descriptor.freshness, availability);
    product.residency = product_residency(availability);
    product.consumer_class = ProductConsumerClass::Renderer;
    product.authority_class = product_authority(descriptor.source_reality_class);
    product.query_policy = ProductQueryPolicy::StrictCurrentOnly;
    product
}

fn product_freshness(
    freshness: ExpressionFreshness,
    availability: ProductAvailabilityState,
) -> ProductFreshness {
    if availability == ProductAvailabilityState::Unavailable {
        return ProductFreshness::Missing;
    }
    match freshness {
        ExpressionFreshness::Current => ProductFreshness::Current,
        ExpressionFreshness::PotentiallyStale => ProductFreshness::PotentiallyStale,
    }
}

fn product_residency(availability: ProductAvailabilityState) -> ProductResidency {
    match availability {
        ProductAvailabilityState::Available => ProductResidency::Resident,
        ProductAvailabilityState::Unavailable => ProductResidency::NonResident,
    }
}

fn product_authority(source_reality_class: ExpressionSourceRealityClass) -> ProductAuthorityClass {
    match source_reality_class {
        ExpressionSourceRealityClass::Diagnostics => ProductAuthorityClass::DiagnosticOnly,
        ExpressionSourceRealityClass::ObservedScene
        | ExpressionSourceRealityClass::DerivedPicking
        | ExpressionSourceRealityClass::DerivedOverlay
        | ExpressionSourceRealityClass::DerivedField
        | ExpressionSourceRealityClass::DerivedAsset
        | ExpressionSourceRealityClass::DerivedVolume
        | ExpressionSourceRealityClass::DerivedMaterial
        | ExpressionSourceRealityClass::DerivedHistory => {
            ProductAuthorityClass::DeterministicDerived
        }
    }
}

fn unavailable_product_diagnostic(descriptor: &ProductDescriptorCore) -> FieldProductDiagnostic {
    let mut diagnostic = FieldProductDiagnostic::blocking(
        FieldProductDiagnosticCode::MissingProduct,
        "viewport product observation is unavailable at query snapshot publication",
    )
    .for_product(descriptor.identity);
    diagnostic.family = Some(descriptor.family);
    diagnostic.scale_band = Some(descriptor.scale_band);
    diagnostic.consumer_class = Some(descriptor.consumer_class);
    diagnostic.generation = Some(descriptor.lineage.generation);
    diagnostic
}

#[cfg(test)]
mod tests {
    use super::*;
    use editor_core::RealityVersion;
    use editor_viewport::{
        ExpressionDimensions, ExpressionFormat, ExpressionPresentationHints, ExpressionProductId,
        ExpressionProductKind, ViewportId,
    };
    use engine::runtime::QuerySnapshotRuntimeResource;

    fn boundary() -> PublicationBoundary {
        PublicationBoundary::new(9, "Update", 0)
    }

    fn descriptor(id: u64, freshness: ExpressionFreshness) -> ExpressionProductDescriptor {
        ExpressionProductDescriptor::new(
            ExpressionProductId(id),
            ExpressionProductKind::SceneColor2D,
            ExpressionDimensions::new(320, 200),
            ExpressionFormat::Rgba8Unorm,
            "editor.viewport.test",
            ExpressionSourceRealityClass::ObservedScene,
            RealityVersion(3),
            freshness,
            ExpressionPresentationHints::default(),
            None,
        )
    }

    fn observations(
        descriptor: ExpressionProductDescriptor,
        availability: ProductAvailabilityState,
    ) -> ViewportArtifactObservationResource {
        let mut frame = ArtifactObservationFrame::new(ViewportId(1), RealityVersion(3));
        frame.available_products.push(descriptor.clone());
        frame.selected_primary_product_id = Some(descriptor.id);
        frame
            .availability_by_product
            .insert(descriptor.id, availability);

        let mut observations = ViewportArtifactObservationResource::default();
        observations.upsert_frame(frame);
        observations
    }

    #[test]
    fn viewport_query_snapshots_stage_and_publish_at_boundary() {
        let observations = observations(
            descriptor(1, ExpressionFreshness::Current),
            ProductAvailabilityState::Available,
        );
        let mut app = RunenwerkEditorApp::new();
        let mut snapshots = QuerySnapshotRuntimeResource::default();

        let report =
            publish_viewport_query_snapshots(&mut app, &observations, &mut snapshots, &boundary());

        assert_eq!(report.published_count, 1);
        assert!(
            snapshots
                .current_snapshot(ProductIdentity::new(1))
                .is_some()
        );
        assert_eq!(app.viewport_query_snapshot_journal().len(), 1);
        assert_eq!(
            app.viewport_query_snapshot_journal()[0].publication_boundary_index,
            9
        );
    }

    #[test]
    fn viewport_query_snapshots_reject_unavailable_products_with_diagnostics() {
        let observations = observations(
            descriptor(2, ExpressionFreshness::Current),
            ProductAvailabilityState::Unavailable,
        );
        let mut app = RunenwerkEditorApp::new();
        let mut snapshots = QuerySnapshotRuntimeResource::default();

        let report =
            publish_viewport_query_snapshots(&mut app, &observations, &mut snapshots, &boundary());

        assert_eq!(report.rejected_count, 1);
        assert!(
            snapshots
                .current_snapshot(ProductIdentity::new(2))
                .is_none()
        );
        assert!(
            app.viewport_query_snapshot_journal()
                .iter()
                .any(|entry| entry.status == QuerySnapshotPublicationStatus::Rejected)
        );
        assert!(app.console_lines().iter().any(|line| {
            line.text.contains("rejected=1")
                || line
                    .text
                    .contains("viewport product observation is unavailable")
        }));
    }

    #[test]
    fn viewport_query_snapshots_preserve_previous_snapshot_when_next_is_stale() {
        let first = observations(
            descriptor(3, ExpressionFreshness::Current),
            ProductAvailabilityState::Available,
        );
        let second = observations(
            descriptor(3, ExpressionFreshness::PotentiallyStale),
            ProductAvailabilityState::Available,
        );
        let mut app = RunenwerkEditorApp::new();
        let mut snapshots = QuerySnapshotRuntimeResource::default();

        publish_viewport_query_snapshots(&mut app, &first, &mut snapshots, &boundary());
        let report =
            publish_viewport_query_snapshots(&mut app, &second, &mut snapshots, &boundary());

        assert_eq!(report.preserved_count, 1);
        assert_eq!(
            snapshots
                .current_snapshot(ProductIdentity::new(3))
                .unwrap()
                .source_generation,
            1
        );
        assert!(
            app.viewport_query_snapshot_journal()
                .iter()
                .any(|entry| entry.status == QuerySnapshotPublicationStatus::Preserved)
        );
    }

    #[test]
    fn viewport_query_snapshots_publish_only_selected_products() {
        let selected = descriptor(4, ExpressionFreshness::Current);
        let selected_overlay = descriptor(5, ExpressionFreshness::Current);
        let unselected_stale = descriptor(6, ExpressionFreshness::PotentiallyStale);
        let mut frame = ArtifactObservationFrame::new(ViewportId(1), RealityVersion(3));
        frame.selected_primary_product_id = Some(selected.id);
        frame.selected_overlay_product_ids = vec![selected_overlay.id];
        frame.available_products = vec![
            selected.clone(),
            selected_overlay.clone(),
            unselected_stale.clone(),
        ];
        frame
            .availability_by_product
            .insert(selected.id, ProductAvailabilityState::Available);
        frame
            .availability_by_product
            .insert(selected_overlay.id, ProductAvailabilityState::Available);
        frame
            .availability_by_product
            .insert(unselected_stale.id, ProductAvailabilityState::Available);
        let mut observations = ViewportArtifactObservationResource::default();
        observations.upsert_frame(frame);

        let mut app = RunenwerkEditorApp::new();
        let mut snapshots = QuerySnapshotRuntimeResource::default();
        let report =
            publish_viewport_query_snapshots(&mut app, &observations, &mut snapshots, &boundary());

        assert_eq!(report.published_count, 2);
        assert_eq!(report.rejected_count, 0);
        assert!(
            snapshots
                .current_snapshot(ProductIdentity::new(selected.id.0))
                .is_some()
        );
        assert!(
            snapshots
                .current_snapshot(ProductIdentity::new(selected_overlay.id.0))
                .is_some()
        );
        assert!(
            snapshots
                .current_snapshot(ProductIdentity::new(unselected_stale.id.0))
                .is_none()
        );
    }

    #[test]
    fn repeated_identical_viewport_query_snapshot_does_not_log_or_invalidate() {
        let observations = observations(
            descriptor(7, ExpressionFreshness::Current),
            ProductAvailabilityState::Available,
        );
        let mut app = RunenwerkEditorApp::new();
        let mut snapshots = QuerySnapshotRuntimeResource::default();
        let boundary = boundary();

        let first =
            publish_viewport_query_snapshots(&mut app, &observations, &mut snapshots, &boundary);
        let console_count = app.console_lines().len();
        let second =
            publish_viewport_query_snapshots(&mut app, &observations, &mut snapshots, &boundary);

        assert_eq!(first.invalidated_count, 0);
        assert_eq!(second.invalidated_count, 0);
        assert!(!second.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == FieldProductDiagnosticCode::GenerationMismatch
        }));
        assert_eq!(app.console_lines().len(), console_count);
    }

    #[test]
    fn derived_material_products_are_deterministic_derived_authority() {
        assert_eq!(
            product_authority(ExpressionSourceRealityClass::DerivedMaterial),
            ProductAuthorityClass::DeterministicDerived
        );
    }
}
