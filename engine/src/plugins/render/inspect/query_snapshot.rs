use product::{ProductIdentity, QuerySnapshotPublicationReport, QuerySnapshotPublicationStatus};

use crate::runtime::QuerySnapshotRuntimeResource;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuerySnapshotInspectionEntry {
    pub product_id: ProductIdentity,
    pub source_generation: u64,
    pub response_generation: u64,
    pub status: QuerySnapshotPublicationStatus,
    pub diagnostic_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuerySnapshotInspection {
    pub current_snapshot_count: usize,
    pub journal_entry_count: usize,
    pub last_report: QuerySnapshotPublicationReport,
    pub journal: Vec<QuerySnapshotInspectionEntry>,
}

pub fn inspect_query_snapshots(resource: &QuerySnapshotRuntimeResource) -> QuerySnapshotInspection {
    QuerySnapshotInspection {
        current_snapshot_count: resource.current_snapshots().len(),
        journal_entry_count: resource.journal().len(),
        last_report: resource.last_report().clone(),
        journal: resource
            .journal()
            .iter()
            .map(|entry| QuerySnapshotInspectionEntry {
                product_id: entry.product_id,
                source_generation: entry.source_generation,
                response_generation: entry.response_generation,
                status: entry.status,
                diagnostic_count: entry.diagnostics.len(),
            })
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::PublicationBoundary;
    use product::{
        ProductAuthorityClass, ProductConsumerClass, ProductDescriptorCore, ProductFamily,
        ProductIdentity, ProductKind, ProductLineage, ProductQueryPolicy, ProductResidency,
        ProductScaleBand, ProductScope, QuerySnapshotProductDescriptor,
    };

    fn boundary() -> PublicationBoundary {
        PublicationBoundary::new(1, "RenderPrepare", 0)
    }

    fn snapshot() -> QuerySnapshotProductDescriptor {
        let mut descriptor = ProductDescriptorCore::new(
            ProductIdentity::new(31),
            ProductFamily::StrictQuery,
            ProductKind::new("viewport_observation"),
            ProductScope::View {
                view_id: "main".to_string(),
            },
            ProductScaleBand::Preview,
            ProductLineage::new("render.inspect.test", 5),
        );
        descriptor.consumer_class = ProductConsumerClass::Renderer;
        descriptor.residency = ProductResidency::Resident;
        descriptor.authority_class = ProductAuthorityClass::DeterministicDerived;
        QuerySnapshotProductDescriptor::new(descriptor, 5, 5, ProductQueryPolicy::StrictCurrentOnly)
    }

    #[test]
    fn render_query_snapshot_inspection_exposes_decisions_without_backend_handles() {
        let mut resource = QuerySnapshotRuntimeResource::default();
        resource.stage(snapshot());
        resource.publish_staged(&boundary());

        let inspection = inspect_query_snapshots(&resource);

        assert_eq!(inspection.current_snapshot_count, 1);
        assert_eq!(inspection.journal_entry_count, 1);
        assert_eq!(inspection.last_report.published_count, 1);
        assert_eq!(
            inspection.journal[0].status,
            QuerySnapshotPublicationStatus::Published
        );
    }
}
