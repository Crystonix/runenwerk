use std::collections::BTreeMap;

use crate::runtime::publication::PublicationBoundary;
use anyhow::Result;
use ecs::World;
use product::{
    FieldProductDiagnostic, FieldProductDiagnosticCode, FieldProductDiagnosticSeverity,
    ProductConsumptionRequest, ProductConsumptionStatus, ProductIdentity,
    QuerySnapshotInvalidationPolicy, QuerySnapshotProductDescriptor,
    QuerySnapshotPublicationReport, QuerySnapshotPublicationStatus, evaluate_product_consumption,
    ratify_query_snapshot_product,
};

const QUERY_SNAPSHOT_JOURNAL_LIMIT: usize = 512;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuerySnapshotJournalEntry {
    pub publication_boundary_index: usize,
    pub schedule_label: &'static str,
    pub deferred_apply_index: usize,
    pub product_id: ProductIdentity,
    pub source_generation: u64,
    pub response_generation: u64,
    pub status: QuerySnapshotPublicationStatus,
    pub diagnostics: Vec<FieldProductDiagnostic>,
}

#[derive(Debug, Clone, Default, ecs::Resource)]
pub struct QuerySnapshotRuntimeResource {
    staged: Vec<QuerySnapshotProductDescriptor>,
    current: BTreeMap<ProductIdentity, QuerySnapshotProductDescriptor>,
    journal: Vec<QuerySnapshotJournalEntry>,
    last_published_entries: Vec<QuerySnapshotJournalEntry>,
    last_report: QuerySnapshotPublicationReport,
}

impl QuerySnapshotRuntimeResource {
    pub fn stage(&mut self, snapshot: QuerySnapshotProductDescriptor) {
        self.staged.push(snapshot);
    }

    pub fn stage_all(
        &mut self,
        snapshots: impl IntoIterator<Item = QuerySnapshotProductDescriptor>,
    ) {
        self.staged.extend(snapshots);
    }

    pub fn staged(&self) -> &[QuerySnapshotProductDescriptor] {
        &self.staged
    }

    pub fn current_snapshots(&self) -> &BTreeMap<ProductIdentity, QuerySnapshotProductDescriptor> {
        &self.current
    }

    pub fn current_snapshot(
        &self,
        product_id: ProductIdentity,
    ) -> Option<&QuerySnapshotProductDescriptor> {
        self.current.get(&product_id)
    }

    pub fn journal(&self) -> &[QuerySnapshotJournalEntry] {
        &self.journal
    }

    pub fn last_published_entries(&self) -> &[QuerySnapshotJournalEntry] {
        &self.last_published_entries
    }

    pub fn last_report(&self) -> &QuerySnapshotPublicationReport {
        &self.last_report
    }

    pub fn publish_staged(
        &mut self,
        boundary: &PublicationBoundary,
    ) -> QuerySnapshotPublicationReport {
        self.last_published_entries.clear();

        let mut staged = std::mem::take(&mut self.staged);
        staged.sort_by_key(|snapshot| {
            (
                snapshot.product_id().raw(),
                snapshot.source_generation,
                snapshot.response_generation,
            )
        });

        let mut report = QuerySnapshotPublicationReport::default();
        for snapshot in staged {
            self.publish_one(snapshot, boundary, &mut report);
        }
        self.last_report = report.clone();
        report
    }

    fn publish_one(
        &mut self,
        snapshot: QuerySnapshotProductDescriptor,
        boundary: &PublicationBoundary,
        report: &mut QuerySnapshotPublicationReport,
    ) {
        let diagnostics = snapshot_rejection_diagnostics(&snapshot);
        if !diagnostics.is_empty() {
            if self.current.contains_key(&snapshot.product_id()) {
                report.record_status(QuerySnapshotPublicationStatus::Preserved);
                report.extend_diagnostics(diagnostics.clone());
                self.push_journal(
                    boundary,
                    &snapshot,
                    QuerySnapshotPublicationStatus::Preserved,
                    diagnostics,
                );
            } else {
                report.record_status(QuerySnapshotPublicationStatus::Rejected);
                report.extend_diagnostics(diagnostics.clone());
                self.push_journal(
                    boundary,
                    &snapshot,
                    QuerySnapshotPublicationStatus::Rejected,
                    diagnostics,
                );
            }
            return;
        }

        if let Some(previous) = self.current.get(&snapshot.product_id()).cloned()
            && snapshot.invalidation_policy
                == QuerySnapshotInvalidationPolicy::InvalidateOnSourceGenerationChange
            && previous.source_generation != snapshot.source_generation
        {
            let diagnostic = invalidation_diagnostic(&previous, &snapshot);
            report.record_status(QuerySnapshotPublicationStatus::Invalidated);
            report.extend_diagnostics([diagnostic.clone()]);
            self.push_journal(
                boundary,
                &previous,
                QuerySnapshotPublicationStatus::Invalidated,
                vec![diagnostic],
            );
        }

        report.record_status(QuerySnapshotPublicationStatus::Published);
        self.push_journal(
            boundary,
            &snapshot,
            QuerySnapshotPublicationStatus::Published,
            Vec::new(),
        );
        self.current.insert(snapshot.product_id(), snapshot);
    }

    fn push_journal(
        &mut self,
        boundary: &PublicationBoundary,
        snapshot: &QuerySnapshotProductDescriptor,
        status: QuerySnapshotPublicationStatus,
        diagnostics: Vec<FieldProductDiagnostic>,
    ) {
        let entry = QuerySnapshotJournalEntry {
            publication_boundary_index: boundary.index,
            schedule_label: boundary.schedule_label,
            deferred_apply_index: boundary.deferred_apply_index,
            product_id: snapshot.product_id(),
            source_generation: snapshot.source_generation,
            response_generation: snapshot.response_generation,
            status,
            diagnostics,
        };
        self.journal.push(entry.clone());
        self.last_published_entries.push(entry);
        if self.journal.len() > QUERY_SNAPSHOT_JOURNAL_LIMIT {
            let drain = self.journal.len() - QUERY_SNAPSHOT_JOURNAL_LIMIT;
            self.journal.drain(0..drain);
        }
    }
}

pub fn publish_staged_query_snapshots(
    boundary: &PublicationBoundary,
    world: &mut World,
) -> Result<()> {
    if let Ok(snapshots) = world.resource_mut::<QuerySnapshotRuntimeResource>() {
        snapshots.publish_staged(boundary);
    }
    Ok(())
}

fn snapshot_rejection_diagnostics(
    snapshot: &QuerySnapshotProductDescriptor,
) -> Vec<FieldProductDiagnostic> {
    let mut diagnostics = Vec::new();
    let ratification = ratify_query_snapshot_product(snapshot);
    diagnostics.extend(ratification.iter().map(|issue| {
        FieldProductDiagnostic::blocking(
            FieldProductDiagnosticCode::FormationFailure,
            format!("query snapshot rejected: {}", issue.message()),
        )
        .for_product(snapshot.product_id())
    }));

    let request =
        ProductConsumptionRequest::new(snapshot.consumer_class, snapshot.requested_policy);
    let decision = evaluate_product_consumption(&snapshot.descriptor, &request);
    if decision.status == ProductConsumptionStatus::Rejected {
        diagnostics.extend(decision.diagnostics);
    }

    diagnostics
}

fn invalidation_diagnostic(
    previous: &QuerySnapshotProductDescriptor,
    next: &QuerySnapshotProductDescriptor,
) -> FieldProductDiagnostic {
    let mut diagnostic = FieldProductDiagnostic::new(
        FieldProductDiagnosticCode::GenerationMismatch,
        FieldProductDiagnosticSeverity::Warning,
        format!(
            "query snapshot source generation changed from {} to {}",
            previous.source_generation, next.source_generation
        ),
    )
    .for_product(previous.product_id());
    diagnostic.consumer_class = Some(next.consumer_class);
    diagnostic.generation = Some(next.response_generation);
    diagnostic
}

#[cfg(test)]
mod tests {
    use super::*;
    use product::{
        ProductAuthorityClass, ProductConsumerClass, ProductDescriptorCore, ProductFamily,
        ProductFreshness, ProductKind, ProductLineage, ProductQueryPolicy, ProductResidency,
        ProductScaleBand, ProductScope,
    };

    fn boundary(index: usize) -> PublicationBoundary {
        PublicationBoundary::new(index, "Update", 0)
    }

    fn descriptor(id: u64, generation: u64) -> ProductDescriptorCore {
        let mut descriptor = ProductDescriptorCore::new(
            ProductIdentity::new(id),
            ProductFamily::StrictQuery,
            ProductKind::new("viewport_observation"),
            ProductScope::View {
                view_id: "main".to_string(),
            },
            ProductScaleBand::Preview,
            ProductLineage::new("engine.test", generation),
        );
        descriptor.consumer_class = ProductConsumerClass::Renderer;
        descriptor.residency = ProductResidency::Resident;
        descriptor.authority_class = ProductAuthorityClass::DeterministicDerived;
        descriptor.query_policy = ProductQueryPolicy::StrictCurrentOnly;
        descriptor
    }

    fn snapshot(id: u64, source_generation: u64) -> QuerySnapshotProductDescriptor {
        QuerySnapshotProductDescriptor::new(
            descriptor(id, source_generation),
            source_generation,
            source_generation,
            ProductQueryPolicy::StrictCurrentOnly,
        )
    }

    #[test]
    fn query_snapshot_staged_snapshots_publish_at_engine_publication_boundary() {
        let mut resource = QuerySnapshotRuntimeResource::default();
        resource.stage(snapshot(1, 10));

        assert_eq!(resource.staged().len(), 1);
        assert!(resource.current_snapshots().is_empty());

        let report = resource.publish_staged(&boundary(2));

        assert_eq!(report.published_count, 1);
        assert_eq!(resource.staged().len(), 0);
        assert!(resource.current_snapshot(ProductIdentity::new(1)).is_some());
        assert_eq!(resource.journal()[0].publication_boundary_index, 2);
        assert_eq!(resource.journal()[0].schedule_label, "Update");
        assert_eq!(resource.journal()[0].deferred_apply_index, 0);
        assert_eq!(resource.last_published_entries().len(), 1);
        assert_eq!(resource.last_published_entries()[0].product_id.raw(), 1);
    }

    #[test]
    fn query_snapshot_invalidates_previous_generation_deterministically() {
        let mut resource = QuerySnapshotRuntimeResource::default();
        resource.stage(snapshot(3, 10));
        resource.publish_staged(&boundary(1));

        resource.stage(snapshot(3, 11));
        let report = resource.publish_staged(&boundary(2));

        assert_eq!(report.invalidated_count, 1);
        assert_eq!(report.published_count, 1);
        assert_eq!(
            resource
                .current_snapshot(ProductIdentity::new(3))
                .unwrap()
                .source_generation,
            11
        );
        assert_eq!(
            resource
                .journal()
                .iter()
                .map(|entry| entry.status)
                .collect::<Vec<_>>(),
            vec![
                QuerySnapshotPublicationStatus::Published,
                QuerySnapshotPublicationStatus::Invalidated,
                QuerySnapshotPublicationStatus::Published
            ]
        );
        assert!(!report.diagnostics.is_empty());
    }

    #[test]
    fn query_snapshot_strict_rejected_snapshot_preserves_existing_current_snapshot() {
        let mut resource = QuerySnapshotRuntimeResource::default();
        resource.stage(snapshot(5, 10));
        resource.publish_staged(&boundary(1));

        let mut rejected = snapshot(5, 11);
        rejected.descriptor.freshness = ProductFreshness::Stale;
        rejected.freshness = ProductFreshness::Stale;
        resource.stage(rejected);
        let report = resource.publish_staged(&boundary(2));

        assert_eq!(report.preserved_count, 1);
        assert_eq!(
            resource
                .current_snapshot(ProductIdentity::new(5))
                .unwrap()
                .source_generation,
            10
        );
        assert!(resource.journal().iter().any(|entry| entry.status
            == QuerySnapshotPublicationStatus::Preserved
            && !entry.diagnostics.is_empty()));
    }

    #[test]
    fn query_snapshot_rejected_snapshot_without_current_is_not_published() {
        let mut resource = QuerySnapshotRuntimeResource::default();
        let mut rejected = snapshot(7, 10);
        rejected.descriptor.residency = ProductResidency::NonResident;
        resource.stage(rejected);

        let report = resource.publish_staged(&boundary(1));

        assert_eq!(report.rejected_count, 1);
        assert!(resource.current_snapshot(ProductIdentity::new(7)).is_none());
        assert!(!report.diagnostics.is_empty());
    }

    #[test]
    fn query_snapshot_journal_keeps_recent_entries_but_last_publish_remains_complete() {
        let mut resource = QuerySnapshotRuntimeResource::default();
        for index in 0..(QUERY_SNAPSHOT_JOURNAL_LIMIT as u64 + 4) {
            resource.stage(snapshot(1000 + index, 10));
            resource.publish_staged(&boundary(index as usize));
        }

        assert_eq!(resource.journal().len(), QUERY_SNAPSHOT_JOURNAL_LIMIT);
        assert_eq!(resource.last_published_entries().len(), 1);
        assert_eq!(
            resource.last_published_entries()[0].product_id.raw(),
            1000 + QUERY_SNAPSHOT_JOURNAL_LIMIT as u64 + 3
        );
    }
}
