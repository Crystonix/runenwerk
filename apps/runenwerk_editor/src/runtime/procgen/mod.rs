//! File: apps/runenwerk_editor/src/runtime/procgen/mod.rs
//! Purpose: Editor-owned Phase 6C procgen CPU preview proof wiring.

use std::collections::BTreeSet;

use anyhow::Result;
use editor_viewport::ExpressionProductId;
use engine::runtime::{
    ProductPublicationRuntimeResource, PublicationBoundary, QuerySnapshotRuntimeResource, Res,
    ResMut,
};
use graph::{
    CyclePolicy, EdgeDefinition, EdgeId, GraphDefinition, GraphId, NodeDefinition, NodeId,
    PortDefinition, PortDirection, PortId, PortTypeId,
};
use procgen::{
    ProcgenBakeDiagnostic, ProcgenBakeOutcome, ProcgenDocument, ProcgenDocumentId,
    ProcgenFieldPreviewDiagnostic, ProcgenFieldPreviewFormation, ProcgenFieldPreviewPolicy,
    ProcgenGeneratorId, ProcgenInputProduct, ProcgenLoweringPolicy, ProcgenNodeCatalog,
    ProcgenNodeKind, ProcgenNodeParameters, ProcgenOutputKind, ProcgenOutputProduct,
    ProcgenRatificationReport, ProcgenReservation, ProcgenReservationId, ProcgenScope,
    ProcgenWorldOpsLoweringResult, ProcgenWriteTarget, bake_procgen_document,
    build_procgen_formed_preview_product_contracts,
    build_procgen_formed_preview_publication_outcome,
    catalog::{
        FIELD_PRODUCT_OUTPUT_NODE, HEIGHT_NOISE_NODE, MATERIAL_RULE_NODE, WORLD_OPS_OUTPUT_NODE,
    },
    determinism_key_for_document, form_procgen_field_preview_products, lower_procgen_to_world_ops,
    ratify_procgen_document,
};
use product::{
    ProductConsumerClass, ProductDescriptorCore, ProductFamily, ProductFreshness, ProductIdentity,
    ProductPublicationOutcome, ProductPublicationReport, ProductQueryPolicy, ProductResidency,
    QuerySnapshotProductDescriptor, QuerySnapshotPublicationReport, QuerySnapshotPublicationStatus,
};
use runen_ecs::World;
use runen_spatial::{ChunkCoord3, ChunkId, RegionCoord3, RegionId, WorldId};
use world_ops::{QuantizedAabb, QuantizedVec3, WorldRevision};
use world_sdf::{FieldPreviewPayload, FieldPreviewProduct, FieldProductId};

use crate::editor_app::RunenwerkEditorApp;
use crate::runtime::resources::EditorHostResource;
use crate::runtime::viewport::ViewportPresentationStateResource;

pub const PROCGEN_WORLD_OPS_PRODUCT_ID: ExpressionProductId = ExpressionProductId(8_001);
pub const PROCGEN_FIELD_CANDIDATE_PRODUCT_ID: ExpressionProductId = ExpressionProductId(8_002);

const PROCGEN_PUBLICATION_STAGE_SEQUENCE: u64 = 6_200;
const PROCGEN_BAKE_PUBLICATION_STAGE_SEQUENCE: u64 = 6_260;
const PROCGEN_JOURNAL_LIMIT: usize = 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorProcgenJournalStage {
    Ratification,
    Lowering,
    FieldPreviewFormation,
    Bake,
    Rollback,
    ProductPublication,
    QuerySnapshotPublication,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditorProcgenJournalEntry {
    pub stage: EditorProcgenJournalStage,
    pub accepted: bool,
    pub summary: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditorProcgenBakeRecord {
    pub determinism_key: String,
    pub source_revision: String,
    pub authored_overlay_generation: u64,
    pub operation_records: Vec<world_ops::OperationRecord>,
    pub changed_regions: Vec<procgen::ProcgenChangedRegion>,
    pub explanations: Vec<procgen::ProcgenExplanationEntry>,
    pub field_preview_products: Vec<FieldPreviewProduct>,
    pub product_descriptors: Vec<ProductDescriptorCore>,
    pub diagnostics: Vec<ProcgenBakeDiagnostic>,
}

impl EditorProcgenBakeRecord {
    fn from_outcome(outcome: &ProcgenBakeOutcome) -> Option<Self> {
        let rollback = outcome.rollback_point.as_ref()?;
        Some(Self {
            determinism_key: rollback.determinism_key.clone(),
            source_revision: rollback.source_revision.clone(),
            authored_overlay_generation: rollback.authored_overlay_generation,
            operation_records: rollback.operation_records.clone(),
            changed_regions: rollback.changed_regions.clone(),
            explanations: rollback.explanations.clone(),
            field_preview_products: outcome.field_preview_products.clone(),
            product_descriptors: rollback.product_descriptors.clone(),
            diagnostics: outcome.diagnostics.clone(),
        })
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EditorProcgenBakeReport {
    pub accepted: bool,
    pub published_count: usize,
    pub rejected_count: usize,
    pub operation_count: usize,
    pub product_count: usize,
    pub descriptor_count: usize,
    pub diagnostics_count: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EditorProcgenRollbackReport {
    pub accepted: bool,
    pub restored_product_count: usize,
    pub restored_descriptor_count: usize,
    pub diagnostics_count: usize,
}

#[derive(Debug, Clone)]
pub struct ProcgenRuntimeState {
    document: ProcgenDocument,
    catalog: ProcgenNodeCatalog,
    published_descriptors: Vec<ProductDescriptorCore>,
    formed_preview_products: Vec<FieldPreviewProduct>,
    selected_preview_product_id: Option<FieldProductId>,
    known_overlay_product_ids: Vec<ExpressionProductId>,
    last_field_preview_key: Option<String>,
    last_field_preview_diagnostics: Vec<ProcgenFieldPreviewDiagnostic>,
    last_publication_key: Option<String>,
    last_query_snapshot_key: Option<String>,
    last_console_summary: Option<String>,
    last_bake: Option<EditorProcgenBakeRecord>,
    journal: Vec<EditorProcgenJournalEntry>,
}

impl Default for ProcgenRuntimeState {
    fn default() -> Self {
        Self::new()
    }
}

impl ProcgenRuntimeState {
    pub fn new() -> Self {
        Self {
            document: default_procgen_document(),
            catalog: ProcgenNodeCatalog::first_slice(),
            published_descriptors: Vec::new(),
            formed_preview_products: Vec::new(),
            selected_preview_product_id: None,
            known_overlay_product_ids: procgen_overlay_product_ids().to_vec(),
            last_field_preview_key: None,
            last_field_preview_diagnostics: Vec::new(),
            last_publication_key: None,
            last_query_snapshot_key: None,
            last_console_summary: None,
            last_bake: None,
            journal: Vec::new(),
        }
    }

    pub fn document(&self) -> &ProcgenDocument {
        &self.document
    }

    pub fn document_mut(&mut self) -> &mut ProcgenDocument {
        &mut self.document
    }

    pub fn catalog(&self) -> &ProcgenNodeCatalog {
        &self.catalog
    }

    pub fn journal(&self) -> &[EditorProcgenJournalEntry] {
        &self.journal
    }

    pub fn last_bake(&self) -> Option<&EditorProcgenBakeRecord> {
        self.last_bake.as_ref()
    }

    pub fn published_descriptors(&self) -> &[ProductDescriptorCore] {
        &self.published_descriptors
    }

    pub fn formed_preview_products(&self) -> &[FieldPreviewProduct] {
        &self.formed_preview_products
    }

    pub fn selected_formed_preview_product(&self) -> Option<&FieldPreviewProduct> {
        self.selected_preview_product_id.and_then(|product_id| {
            self.formed_preview_products
                .iter()
                .find(|product| product.descriptor.product_id == product_id)
        })
    }

    pub fn ratification_report(&self) -> ProcgenRatificationReport {
        ratify_procgen_document(&self.document, &self.catalog)
    }

    pub fn lowering_result(&self) -> ProcgenWorldOpsLoweringResult {
        lower_procgen_to_world_ops(&self.document, &self.catalog)
    }

    pub fn active_overlay_product_ids(&self) -> Vec<ExpressionProductId> {
        self.formed_preview_products
            .iter()
            .map(|product| ExpressionProductId(product.descriptor.product_id.0))
            .collect()
    }

    pub fn owned_overlay_product_ids(&self) -> Vec<ExpressionProductId> {
        let mut ids = self.known_overlay_product_ids.clone();
        ids.sort();
        ids.dedup();
        ids
    }

    pub fn graph_canvas_lines(&self) -> Vec<String> {
        let report = self.ratification_report();
        let node_names = self
            .document
            .graph
            .nodes
            .iter()
            .map(|node| node.name.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        let output_ids = self
            .document
            .output_products
            .iter()
            .map(|output| format!("{}:{:?}", output.product_id.raw(), output.kind))
            .collect::<Vec<_>>()
            .join(", ");
        let mut lines = vec![
            "procgen graph canvas: domain-backed Phase 6D bake-capable CPU preview".to_string(),
            format!(
                "document: {} #{}",
                self.document.label,
                self.document.document_id.raw()
            ),
            format!(
                "schema={} generator={}@{} source={}",
                self.document.schema_version,
                self.document.generator_id.raw(),
                self.document.generator_version,
                self.document.source_revision
            ),
            format!("world seed: {}", self.document.world_seed),
            format!("bounded scope: {}", scope_summary(&self.document.scope)),
            format!("graph nodes: {node_names}"),
            format!("output products: {output_ids}"),
            format!(
                "ratification: {} issues={}",
                if report.has_blocking_issues() {
                    "rejected"
                } else {
                    "accepted"
                },
                report.len()
            ),
        ];
        lines.extend(
            report
                .iter()
                .take(4)
                .map(|issue| format!("diagnostic {:?}: {}", issue.code(), issue.message())),
        );
        lines
    }

    pub fn preview_lines(&self) -> Vec<String> {
        let lowering = self.lowering_result();
        let mut lines = vec![
            "procgen preview: concrete terrain/material CPU preview".to_string(),
            format!(
                "determinism key: {}",
                determinism_key_for_document(&self.document).as_str()
            ),
        ];
        match &lowering.realization {
            Some(realization) => {
                lines.push(format!(
                    "operation-window overlay: operations={} changed_regions={} reservations={}",
                    realization.operation_records.len(),
                    realization.changed_regions.len(),
                    self.document.reservations.len()
                ));
                lines.extend(realization.changed_regions.iter().take(4).map(|region| {
                    format!(
                        "changed region {} {} product={}",
                        region.target_id,
                        bounds_summary(region.bounds_q),
                        region
                            .product_id
                            .map(|id| id.raw().to_string())
                            .unwrap_or_else(|| "none".to_string())
                    )
                }));
                lines.extend(
                    self.document
                        .reservations
                        .iter()
                        .take(4)
                        .map(|reservation| {
                            format!(
                                "reservation {} {} {:?}",
                                reservation.target_id,
                                bounds_summary(reservation.bounds_q),
                                reservation.kind
                            )
                        }),
                );
            }
            None => {
                lines.push("operation-window overlay: rejected".to_string());
                lines.extend(
                    lowering
                        .report
                        .iter()
                        .take(5)
                        .map(|issue| format!("diagnostic {:?}: {}", issue.code(), issue.message())),
                );
            }
        }
        lines.extend(self.field_preview_lines());
        lines.extend(self.substrate_status_lines());
        lines
    }

    pub fn field_preview_lines(&self) -> Vec<String> {
        let mut lines = vec![format!(
            "procgen field preview products: {}",
            self.formed_preview_products.len()
        )];
        if let Some(key) = &self.last_field_preview_key {
            lines.push(format!("procgen field preview key: {key}"));
        } else {
            lines.push("procgen field preview key: waiting".to_string());
        }
        if let Some(product) = self.selected_formed_preview_product() {
            lines.push(format!(
                "selected procgen preview product: {}",
                product.descriptor.product_id.0
            ));
            lines.push(format!(
                "procgen preview kind: {:?}",
                product.descriptor.kind
            ));
            lines.push(format!(
                "procgen preview grid: {:?}",
                product.payload.grid().dimensions
            ));
            lines.push(format!(
                "procgen preview sample count: {}",
                product.payload.sample_count()
            ));
            lines.extend(preview_payload_summary_lines(&product.payload));
        } else {
            lines.push("selected procgen preview product: none".to_string());
        }
        lines.extend(
            self.last_field_preview_diagnostics
                .iter()
                .take(5)
                .map(|diagnostic| {
                    format!(
                        "procgen field preview diagnostic {:?}: {}",
                        diagnostic.code, diagnostic.message
                    )
                }),
        );
        if let Some(bake) = &self.last_bake {
            lines.push(format!(
                "last procgen bake: key={} operations={} products={}",
                bake.determinism_key,
                bake.operation_records.len(),
                bake.field_preview_products.len()
            ));
        } else {
            lines.push("last procgen bake: none".to_string());
        }
        lines
    }

    pub fn viewport_overlay_status_lines(&self) -> Vec<String> {
        let lowering = self.lowering_result();
        let Some(realization) = lowering.realization else {
            if lowering.report.has_blocking_issues() {
                return vec![format!(
                    "Procgen rejected: {} diagnostic(s)",
                    lowering.report.len()
                )];
            }
            return Vec::new();
        };
        let mut lines = vec![format!(
            "Procgen overlay: {} region(s), {} reservation(s)",
            realization.changed_regions.len(),
            self.document.reservations.len()
        )];
        lines.extend(
            realization
                .changed_regions
                .iter()
                .take(2)
                .map(|region| format!("{} {}", region.target_id, bounds_summary(region.bounds_q))),
        );
        lines
    }

    fn substrate_status_lines(&self) -> Vec<String> {
        let publication = if self.last_publication_key.is_some() {
            "published"
        } else {
            "waiting"
        };
        let query = if self.last_query_snapshot_key.is_some() {
            "snapshotted"
        } else {
            "waiting"
        };
        let bake = self
            .last_bake
            .as_ref()
            .map(|record| record.determinism_key.as_str())
            .unwrap_or("waiting");
        vec![
            format!("product publication: {publication}"),
            format!("query snapshot: {query}"),
            format!("offline bake: {bake}"),
            format!("journal entries: {}", self.journal.len()),
        ]
    }

    fn record_journal(
        &mut self,
        stage: EditorProcgenJournalStage,
        accepted: bool,
        summary: impl Into<String>,
    ) {
        self.journal.push(EditorProcgenJournalEntry {
            stage,
            accepted,
            summary: summary.into(),
        });
        if self.journal.len() > PROCGEN_JOURNAL_LIMIT {
            let drain = self.journal.len() - PROCGEN_JOURNAL_LIMIT;
            self.journal.drain(0..drain);
        }
    }

    fn update_console_summary(&mut self, summary: impl Into<String>) -> Option<String> {
        let summary = summary.into();
        if self.last_console_summary.as_ref() == Some(&summary) {
            return None;
        }
        self.last_console_summary = Some(summary.clone());
        Some(summary)
    }

    fn record_published_descriptors(&mut self, descriptors: Vec<ProductDescriptorCore>) {
        self.published_descriptors = descriptors;
    }

    fn record_formed_preview_products(&mut self, formation: ProcgenFieldPreviewFormation) {
        self.last_field_preview_key = formation.determinism_key;
        self.last_field_preview_diagnostics = formation.diagnostics;
        self.formed_preview_products = formation.products;
        self.select_first_formed_preview_product();
        self.track_active_overlay_products();
    }

    fn record_baked_products(
        &mut self,
        key: String,
        products: Vec<FieldPreviewProduct>,
        diagnostics: Vec<ProcgenFieldPreviewDiagnostic>,
    ) {
        self.last_field_preview_key = Some(key);
        self.last_field_preview_diagnostics = diagnostics;
        self.formed_preview_products = products;
        self.select_first_formed_preview_product();
        self.track_active_overlay_products();
    }

    fn select_first_formed_preview_product(&mut self) {
        self.selected_preview_product_id = self
            .formed_preview_products
            .iter()
            .find(|product| matches!(product.payload, FieldPreviewPayload::ScalarDistance { .. }))
            .or_else(|| self.formed_preview_products.first())
            .map(|product| product.descriptor.product_id);
    }

    fn track_active_overlay_products(&mut self) {
        self.known_overlay_product_ids
            .extend(self.active_overlay_product_ids());
        self.known_overlay_product_ids.sort();
        self.known_overlay_product_ids.dedup();
    }

    fn clear_formed_preview_products(&mut self, diagnostics: Vec<ProcgenFieldPreviewDiagnostic>) {
        self.formed_preview_products.clear();
        self.selected_preview_product_id = None;
        self.last_field_preview_key = None;
        self.last_field_preview_diagnostics = diagnostics;
    }

    fn record_accepted_bake(&mut self, outcome: &ProcgenBakeOutcome, publication_key: String) {
        let Some(record) = EditorProcgenBakeRecord::from_outcome(outcome) else {
            return;
        };
        let key = record.determinism_key.clone();
        self.record_published_descriptors(record.product_descriptors.clone());
        self.record_baked_products(key, record.field_preview_products.clone(), Vec::new());
        self.last_publication_key = Some(publication_key);
        self.last_query_snapshot_key = None;
        self.last_bake = Some(record);
    }

    fn restore_last_bake(&mut self) -> EditorProcgenRollbackReport {
        let Some(record) = self.last_bake.clone() else {
            self.record_journal(
                EditorProcgenJournalStage::Rollback,
                false,
                "no accepted bake is available",
            );
            return EditorProcgenRollbackReport::default();
        };

        self.record_published_descriptors(record.product_descriptors.clone());
        self.record_baked_products(
            record.determinism_key.clone(),
            record.field_preview_products.clone(),
            Vec::new(),
        );
        self.last_publication_key = Some(format!("bake:{}", record.determinism_key));
        self.last_query_snapshot_key = None;
        self.record_journal(
            EditorProcgenJournalStage::Rollback,
            true,
            format!(
                "restored_products={} descriptors={}",
                record.field_preview_products.len(),
                record.product_descriptors.len()
            ),
        );
        EditorProcgenRollbackReport {
            accepted: true,
            restored_product_count: record.field_preview_products.len(),
            restored_descriptor_count: record.product_descriptors.len(),
            diagnostics_count: record.diagnostics.len(),
        }
    }

    fn last_bake_is_current(&self) -> bool {
        let Some(record) = &self.last_bake else {
            return false;
        };
        self.last_publication_key.as_deref()
            == Some(format!("bake:{}", record.determinism_key).as_str())
            && self.published_descriptors == record.product_descriptors
            && self.formed_preview_products == record.field_preview_products
    }

    fn preserve_last_bake_after_rejection(&mut self) -> Option<EditorProcgenRollbackReport> {
        let record = self.last_bake.as_ref()?;
        if self.last_bake_is_current() {
            return Some(EditorProcgenRollbackReport {
                accepted: true,
                restored_product_count: record.field_preview_products.len(),
                restored_descriptor_count: record.product_descriptors.len(),
                diagnostics_count: record.diagnostics.len(),
            });
        }
        Some(self.restore_last_bake())
    }
}

pub fn procgen_overlay_product_ids() -> &'static [ExpressionProductId; 2] {
    &[
        PROCGEN_WORLD_OPS_PRODUCT_ID,
        PROCGEN_FIELD_CANDIDATE_PRODUCT_ID,
    ]
}

pub fn publish_procgen_products_at_boundary(
    boundary: &PublicationBoundary,
    world: &mut World,
) -> Result<()> {
    let Some(mut host) = world.remove_resource::<EditorHostResource>() else {
        return Ok(());
    };
    let Some(mut publications) = world.remove_resource::<ProductPublicationRuntimeResource>()
    else {
        world.insert_resource(host);
        return Ok(());
    };

    publish_procgen_products(&mut host.app, &mut publications, boundary);

    world.insert_resource(publications);
    world.insert_resource(host);
    Ok(())
}

pub fn publish_procgen_query_snapshots_at_boundary(
    boundary: &PublicationBoundary,
    world: &mut World,
) -> Result<()> {
    let Some(mut host) = world.remove_resource::<EditorHostResource>() else {
        return Ok(());
    };
    let Some(mut snapshots) = world.remove_resource::<QuerySnapshotRuntimeResource>() else {
        world.insert_resource(host);
        return Ok(());
    };

    publish_procgen_query_snapshots(&mut host.app, &mut snapshots, boundary);

    world.insert_resource(snapshots);
    world.insert_resource(host);
    Ok(())
}

pub fn publish_procgen_products(
    app: &mut RunenwerkEditorApp,
    publications: &mut ProductPublicationRuntimeResource,
    boundary: &PublicationBoundary,
) -> ProductPublicationReport {
    let key = determinism_key_for_document(app.procgen_runtime().document())
        .as_str()
        .to_string();
    let bake_key = format!("bake:{key}");
    if app.procgen_runtime().last_publication_key.as_ref() == Some(&key)
        || app.procgen_runtime().last_publication_key.as_ref() == Some(&bake_key)
    {
        return ProductPublicationReport::default();
    }

    let report = app.procgen_runtime().ratification_report();
    if report.has_blocking_issues() {
        let issue_count = report.len();
        if let Some(preserved) = app
            .procgen_runtime_mut()
            .preserve_last_bake_after_rejection()
        {
            app.procgen_runtime_mut().record_journal(
                EditorProcgenJournalStage::Ratification,
                false,
                format!(
                    "rejected issues={} preserved_last_bake_products={}",
                    issue_count, preserved.restored_product_count
                ),
            );
            if let Some(summary) = app.procgen_runtime_mut().update_console_summary(format!(
                "[procgen] rejected issues={issue_count}; preserved last accepted bake products={}",
                preserved.restored_product_count
            )) {
                app.append_console_warning(summary);
            }
            return ProductPublicationReport::default();
        }
        app.procgen_runtime_mut().published_descriptors.clear();
        app.procgen_runtime_mut().last_publication_key = None;
        app.procgen_runtime_mut().last_query_snapshot_key = None;
        app.procgen_runtime_mut()
            .clear_formed_preview_products(Vec::new());
        app.procgen_runtime_mut().record_journal(
            EditorProcgenJournalStage::Ratification,
            false,
            format!("rejected issues={issue_count}"),
        );
        if let Some(summary) = app
            .procgen_runtime_mut()
            .update_console_summary(format!("[procgen] rejected issues={issue_count}"))
        {
            app.append_console_warning(summary);
        }
        return ProductPublicationReport::default();
    }

    let formation = form_procgen_field_preview_products(
        app.procgen_runtime().document(),
        app.procgen_runtime().catalog(),
        ProcgenFieldPreviewPolicy::default(),
    );
    if !formation.is_accepted() || formation.products.is_empty() {
        let diagnostics = formation.diagnostics.clone();
        app.procgen_runtime_mut().published_descriptors.clear();
        app.procgen_runtime_mut().last_publication_key = None;
        app.procgen_runtime_mut().last_query_snapshot_key = None;
        app.procgen_runtime_mut()
            .clear_formed_preview_products(diagnostics.clone());
        app.procgen_runtime_mut().record_journal(
            EditorProcgenJournalStage::FieldPreviewFormation,
            false,
            format!(
                "products={} diagnostics={}",
                formation.products.len(),
                diagnostics.len()
            ),
        );
        if let Some(summary) = app.procgen_runtime_mut().update_console_summary(format!(
            "[procgen] field preview rejected diagnostics={}",
            diagnostics.len()
        )) {
            app.append_console_warning(summary);
        }
        return ProductPublicationReport::default();
    }
    let Some(contracts) = build_procgen_formed_preview_product_contracts(
        app.procgen_runtime().document(),
        app.procgen_runtime().catalog(),
        &formation.products,
    ) else {
        return ProductPublicationReport::default();
    };
    let Some(outcome) = build_procgen_formed_preview_publication_outcome(
        app.procgen_runtime().document(),
        app.procgen_runtime().catalog(),
        &formation.products,
        PROCGEN_PUBLICATION_STAGE_SEQUENCE,
    ) else {
        return ProductPublicationReport::default();
    };

    let journal_start = publications.journal().len();
    publications.stage(outcome);
    let report = publications.publish_staged(boundary);
    let published_entries = &publications.journal()[journal_start..];

    if report.published_count > 0 {
        app.procgen_runtime_mut()
            .record_published_descriptors(contracts.output_descriptors);
        app.procgen_runtime_mut()
            .record_formed_preview_products(formation);
        app.procgen_runtime_mut().last_publication_key = Some(key);
    }
    let formed_product_count = app.procgen_runtime().formed_preview_products().len();
    let formed_diagnostic_count = app.procgen_runtime().last_field_preview_diagnostics.len();
    app.procgen_runtime_mut().record_journal(
        EditorProcgenJournalStage::FieldPreviewFormation,
        report.published_count > 0,
        format!(
            "products={} diagnostics={}",
            formed_product_count, formed_diagnostic_count
        ),
    );
    app.procgen_runtime_mut().record_journal(
        EditorProcgenJournalStage::ProductPublication,
        report.rejected_count == 0,
        format!(
            "published={} rejected={} failed_preserved={}",
            report.published_count, report.rejected_count, report.failed_preserved_count
        ),
    );

    if let Some(summary) = app.procgen_runtime_mut().update_console_summary(format!(
        "[procgen] publication boundary {}: published={} rejected={} outputs={}",
        boundary.index,
        report.published_count,
        report.rejected_count,
        published_entries
            .iter()
            .flat_map(|entry| entry.output_products.iter())
            .map(|id| id.raw().to_string())
            .collect::<Vec<_>>()
            .join(",")
    )) {
        app.append_console_line(summary);
    }
    for diagnostic in report.diagnostics.iter().take(5) {
        app.append_console_warning(format!(
            "[procgen] publication {:?}: {}",
            diagnostic.code, diagnostic.message
        ));
    }

    report
}

pub fn bake_procgen_products(
    app: &mut RunenwerkEditorApp,
    publications: &mut ProductPublicationRuntimeResource,
    boundary: &PublicationBoundary,
) -> EditorProcgenBakeReport {
    let outcome = bake_procgen_document(
        app.procgen_runtime().document(),
        app.procgen_runtime().catalog(),
        ProcgenFieldPreviewPolicy::default(),
    );
    let mut bake_report = EditorProcgenBakeReport {
        accepted: outcome.is_accepted(),
        published_count: 0,
        rejected_count: 0,
        operation_count: outcome.operation_records.len(),
        product_count: outcome.field_preview_products.len(),
        descriptor_count: outcome.output_descriptors.len(),
        diagnostics_count: outcome.diagnostics.len(),
    };

    let Some(product_job) = outcome.product_job.clone() else {
        app.procgen_runtime_mut().record_journal(
            EditorProcgenJournalStage::Bake,
            false,
            format!("rejected diagnostics={}", outcome.diagnostics.len()),
        );
        if let Some(summary) = app.procgen_runtime_mut().update_console_summary(format!(
            "[procgen] bake rejected diagnostics={}",
            outcome.diagnostics.len()
        )) {
            app.append_console_warning(summary);
        }
        for diagnostic in outcome.diagnostics.iter().take(5) {
            app.append_console_warning(format!(
                "[procgen] bake {:?}: {}",
                diagnostic.code, diagnostic.message
            ));
        }
        bake_report.accepted = false;
        return bake_report;
    };

    let outcome_for_publication = ProductPublicationOutcome::ready(
        product_job,
        outcome.output_descriptors.clone(),
        PROCGEN_BAKE_PUBLICATION_STAGE_SEQUENCE,
    );
    publications.stage(outcome_for_publication);
    let publication_report = publications.publish_staged(boundary);
    bake_report.published_count = publication_report.published_count;
    bake_report.rejected_count = publication_report.rejected_count;
    bake_report.accepted =
        publication_report.published_count > 0 && publication_report.rejected_count == 0;

    if bake_report.accepted {
        let publication_key = outcome
            .determinism_key
            .as_ref()
            .map(|key| format!("bake:{key}"))
            .unwrap_or_else(|| format!("bake:boundary:{}", boundary.index));
        app.procgen_runtime_mut()
            .record_accepted_bake(&outcome, publication_key);
    }

    app.procgen_runtime_mut().record_journal(
        EditorProcgenJournalStage::Bake,
        bake_report.accepted,
        format!(
            "published={} rejected={} operations={} products={} diagnostics={}",
            bake_report.published_count,
            bake_report.rejected_count,
            bake_report.operation_count,
            bake_report.product_count,
            bake_report.diagnostics_count
        ),
    );

    if let Some(summary) = app.procgen_runtime_mut().update_console_summary(format!(
        "[procgen] bake publication boundary {}: published={} rejected={} operations={} products={}",
        boundary.index,
        bake_report.published_count,
        bake_report.rejected_count,
        bake_report.operation_count,
        bake_report.product_count
    )) {
        if bake_report.accepted {
            app.append_console_line(summary);
        } else {
            app.append_console_warning(summary);
        }
    }
    for diagnostic in publication_report.diagnostics.iter().take(5) {
        app.append_console_warning(format!(
            "[procgen] bake publication {:?}: {}",
            diagnostic.code, diagnostic.message
        ));
    }

    bake_report
}

pub fn rollback_procgen_bake(app: &mut RunenwerkEditorApp) -> EditorProcgenRollbackReport {
    let report = app.procgen_runtime_mut().restore_last_bake();
    if let Some(summary) = app.procgen_runtime_mut().update_console_summary(format!(
        "[procgen] rollback bake: accepted={} products={} descriptors={}",
        report.accepted, report.restored_product_count, report.restored_descriptor_count
    )) {
        if report.accepted {
            app.append_console_line(summary);
        } else {
            app.append_console_warning(summary);
        }
    }
    report
}

pub fn publish_procgen_query_snapshots(
    app: &mut RunenwerkEditorApp,
    snapshots: &mut QuerySnapshotRuntimeResource,
    boundary: &PublicationBoundary,
) -> QuerySnapshotPublicationReport {
    let snapshot_key = descriptor_generation_key(app.procgen_runtime().published_descriptors());
    let Some(snapshot_key) = snapshot_key else {
        return QuerySnapshotPublicationReport::default();
    };
    if app.procgen_runtime().last_query_snapshot_key.as_ref() == Some(&snapshot_key) {
        return QuerySnapshotPublicationReport::default();
    }

    let staged = app
        .procgen_runtime()
        .published_descriptors()
        .iter()
        .cloned()
        .map(procgen_query_snapshot_for_descriptor)
        .collect::<Vec<_>>();
    if staged.is_empty() {
        return QuerySnapshotPublicationReport::default();
    }

    snapshots.stage_all(staged);
    let report = snapshots.publish_staged(boundary);
    let published_entries = snapshots.last_published_entries().to_vec();

    if report.published_count > 0 {
        app.procgen_runtime_mut().last_query_snapshot_key = Some(snapshot_key);
    }
    app.procgen_runtime_mut().record_journal(
        EditorProcgenJournalStage::QuerySnapshotPublication,
        report.rejected_count == 0,
        format!(
            "published={} rejected={} preserved={} invalidated={}",
            report.published_count,
            report.rejected_count,
            report.preserved_count,
            report.invalidated_count
        ),
    );

    if let Some(summary) = app.procgen_runtime_mut().update_console_summary(format!(
        "[procgen] query publication boundary {}: published={} rejected={} preserved={} invalidated={}",
        boundary.index,
        report.published_count,
        report.rejected_count,
        report.preserved_count,
        report.invalidated_count
    )) {
        app.append_console_line(summary);
    }
    for diagnostic in report.diagnostics.iter().take(5) {
        app.append_console_warning(format!(
            "[procgen] query {:?}: {}",
            diagnostic.code, diagnostic.message
        ));
    }

    for entry in &published_entries {
        if entry.status != QuerySnapshotPublicationStatus::Published {
            app.procgen_runtime_mut().record_journal(
                EditorProcgenJournalStage::QuerySnapshotPublication,
                false,
                format!(
                    "product={} status={:?} diagnostics={}",
                    entry.product_id.raw(),
                    entry.status,
                    entry.diagnostics.len()
                ),
            );
        }
    }

    report
}

pub fn sync_procgen_viewport_overlay_system(
    host: Res<EditorHostResource>,
    mut presentations: ResMut<ViewportPresentationStateResource>,
) {
    sync_procgen_viewport_overlays(&host.app, &mut presentations);
}

pub fn sync_procgen_viewport_overlays(
    app: &RunenwerkEditorApp,
    presentations: &mut ViewportPresentationStateResource,
) {
    let procgen_ids = app
        .procgen_runtime()
        .owned_overlay_product_ids()
        .into_iter()
        .collect::<BTreeSet<_>>();
    let active_ids = app.procgen_runtime().active_overlay_product_ids();
    let viewport_ids = presentations.viewport_ids().collect::<Vec<_>>();
    for viewport_id in viewport_ids {
        let Some(state) = presentations.state_for_mut(viewport_id) else {
            continue;
        };
        let mut overlays = state
            .selected_overlay_product_ids
            .iter()
            .copied()
            .filter(|product_id| !procgen_ids.contains(product_id))
            .collect::<Vec<_>>();
        overlays.extend(active_ids.iter().copied());
        overlays.sort();
        overlays.dedup();
        state.set_overlay_products(overlays);
    }
}

fn preview_payload_summary_lines(payload: &FieldPreviewPayload) -> Vec<String> {
    match payload {
        FieldPreviewPayload::ScalarDistance { samples, .. } => {
            let min = samples.iter().copied().min().unwrap_or_default();
            let max = samples.iter().copied().max().unwrap_or_default();
            vec![format!("distance range: {min}..{max}")]
        }
        FieldPreviewPayload::MaterialChannel { samples, .. } => {
            let mut masks = samples.iter().copied().collect::<BTreeSet<_>>();
            if masks.len() > 6 {
                masks = masks.into_iter().take(6).collect();
            }
            vec![format!(
                "material masks: {}",
                masks
                    .into_iter()
                    .map(|mask| mask.to_string())
                    .collect::<Vec<_>>()
                    .join(",")
            )]
        }
        FieldPreviewPayload::VectorGradient { samples, .. } => {
            vec![format!("gradient samples: {}", samples.len())]
        }
        FieldPreviewPayload::OccupancySupport { samples, .. } => {
            let occupied = samples.iter().filter(|sample| **sample != 0).count();
            vec![format!("occupied samples: {occupied}/{}", samples.len())]
        }
    }
}

fn procgen_query_snapshot_for_descriptor(
    mut descriptor: ProductDescriptorCore,
) -> QuerySnapshotProductDescriptor {
    descriptor.consumer_class = ProductConsumerClass::Renderer;
    descriptor.query_policy = ProductQueryPolicy::StrictCurrentOnly;
    descriptor.freshness = ProductFreshness::Current;
    if descriptor.family == ProductFamily::SurfaceSdf {
        descriptor.residency = ProductResidency::Resident;
    }
    let generation = descriptor.lineage.generation;
    QuerySnapshotProductDescriptor::new(
        descriptor,
        generation,
        generation,
        ProductQueryPolicy::StrictCurrentOnly,
    )
}

fn descriptor_generation_key(descriptors: &[ProductDescriptorCore]) -> Option<String> {
    if descriptors.is_empty() {
        return None;
    }
    let mut parts = descriptors
        .iter()
        .map(|descriptor| {
            format!(
                "{}:{}",
                descriptor.identity.raw(),
                descriptor.lineage.generation
            )
        })
        .collect::<Vec<_>>();
    parts.sort();
    Some(parts.join("|"))
}

fn default_procgen_document() -> ProcgenDocument {
    let world_id = WorldId::new(1);
    let density_target = ProcgenWriteTarget::density("density-main", default_bounds());
    let material_target =
        ProcgenWriteTarget::material_channel("material-main", default_bounds(), 2);
    let mut document = ProcgenDocument::new(
        ProcgenDocumentId::new(101),
        "first terrain material",
        default_procgen_graph(),
        ProcgenScope::new(
            world_id,
            [ChunkId::new(world_id, ChunkCoord3 { x: 0, y: 0, z: 0 })],
            [RegionId::new(world_id, RegionCoord3 { x: 0, y: 0, z: 0 })],
        ),
    )
    .with_schema_version("procgen.schema.v1")
    .with_generator(ProcgenGeneratorId::new(33), "terrain-material.v1")
    .with_world_seed("world-seed:alpha")
    .with_source_revision("source-rev-1")
    .with_authored_overlay_generation(9)
    .with_lowering_policy(ProcgenLoweringPolicy::new(
        "lowering.v1",
        16,
        WorldRevision(4),
    ))
    .with_input_product(ProcgenInputProduct::new(ProductIdentity::new(77), 12))
    .with_node_parameter(
        ProcgenNodeParameters::new(NodeId::new(1), ProcgenNodeKind::HeightNoise, "height")
            .with_seed_salt("height-a")
            .with_weight(5),
    )
    .with_node_parameter(
        ProcgenNodeParameters::new(NodeId::new(2), ProcgenNodeKind::MaterialRule, "material")
            .with_seed_salt("material-a")
            .with_material_channel(2),
    )
    .with_node_parameter(ProcgenNodeParameters::new(
        NodeId::new(3),
        ProcgenNodeKind::WorldOpsOutput,
        "world ops",
    ))
    .with_node_parameter(ProcgenNodeParameters::new(
        NodeId::new(4),
        ProcgenNodeKind::FieldProductOutput,
        "field product",
    ))
    .with_write_target(density_target.clone())
    .with_write_target(material_target.clone())
    .with_output_product(ProcgenOutputProduct::new(
        ProductIdentity::new(PROCGEN_WORLD_OPS_PRODUCT_ID.0),
        ProcgenOutputKind::WorldOpsWindow,
        "operation window",
    ))
    .with_output_product(ProcgenOutputProduct::new(
        ProductIdentity::new(PROCGEN_FIELD_CANDIDATE_PRODUCT_ID.0),
        ProcgenOutputKind::FieldProductCandidate,
        "field product candidate",
    ))
    .with_reservation(ProcgenReservation::from_target(
        ProcgenReservationId::new(7_001),
        &density_target,
    ))
    .with_reservation(ProcgenReservation::from_target(
        ProcgenReservationId::new(7_002),
        &material_target,
    ));
    document.refresh_cache_lineage();
    document
}

fn default_procgen_graph() -> GraphDefinition {
    let scalar = PortTypeId::new(1);
    GraphDefinition::new(
        GraphId::new(7),
        "terrain_material",
        CyclePolicy::RejectDirectedCycles,
        [
            NodeDefinition::new(
                NodeId::new(1),
                HEIGHT_NOISE_NODE,
                [PortDefinition::new(
                    PortId::new(1),
                    "height",
                    PortDirection::Output,
                    scalar,
                )],
            ),
            NodeDefinition::new(
                NodeId::new(2),
                MATERIAL_RULE_NODE,
                [
                    PortDefinition::new(PortId::new(2), "height", PortDirection::Input, scalar),
                    PortDefinition::new(PortId::new(3), "material", PortDirection::Output, scalar),
                ],
            ),
            NodeDefinition::new(
                NodeId::new(3),
                WORLD_OPS_OUTPUT_NODE,
                [PortDefinition::new(
                    PortId::new(4),
                    "material",
                    PortDirection::Input,
                    scalar,
                )],
            ),
            NodeDefinition::new(
                NodeId::new(4),
                FIELD_PRODUCT_OUTPUT_NODE,
                [PortDefinition::new(
                    PortId::new(5),
                    "material",
                    PortDirection::Input,
                    scalar,
                )],
            ),
        ],
        [
            EdgeDefinition::new(EdgeId::new(1), PortId::new(1), PortId::new(2)),
            EdgeDefinition::new(EdgeId::new(2), PortId::new(3), PortId::new(4)),
            EdgeDefinition::new(EdgeId::new(3), PortId::new(3), PortId::new(5)),
        ],
    )
}

fn default_bounds() -> QuantizedAabb {
    QuantizedAabb {
        min: QuantizedVec3 { x: 0, y: 0, z: 0 },
        max: QuantizedVec3 {
            x: 16,
            y: 16,
            z: 16,
        },
    }
}

fn scope_summary(scope: &ProcgenScope) -> String {
    format!(
        "world={} chunks={} regions={}",
        scope.world_id.get(),
        scope.chunk_ids.len(),
        scope.region_ids.len()
    )
}

fn bounds_summary(bounds: QuantizedAabb) -> String {
    format!(
        "[{}, {}, {}]-[{}, {}, {}]",
        bounds.min.x, bounds.min.y, bounds.min.z, bounds.max.x, bounds.max.y, bounds.max.z
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use editor_viewport::ViewportPresentationState;
    use engine::runtime::QuerySnapshotRuntimeResource;
    use product::{ProductScaleBand, evaluate_product_consumption};

    fn boundary() -> PublicationBoundary {
        PublicationBoundary::new(17, "Update", 0)
    }

    #[test]
    fn default_procgen_state_ratifies_lowers_and_exposes_overlay_ids() {
        let state = ProcgenRuntimeState::new();

        assert!(state.ratification_report().is_accepted());
        assert!(state.lowering_result().realization.is_some());
        assert!(state.active_overlay_product_ids().is_empty());
        assert!(
            state
                .preview_lines()
                .iter()
                .any(|line| line.contains("changed_regions=2"))
        );
    }

    #[test]
    fn procgen_products_publish_at_product_publication_boundary() {
        let mut app = RunenwerkEditorApp::new();
        let mut publications = ProductPublicationRuntimeResource::default();

        let report = publish_procgen_products(&mut app, &mut publications, &boundary());

        assert_eq!(report.published_count, 1);
        assert_eq!(app.procgen_runtime().published_descriptors().len(), 3);
        assert_eq!(app.procgen_runtime().formed_preview_products().len(), 2);
        assert!(publications.staged().is_empty());
    }

    #[test]
    fn procgen_bake_publishes_offline_products_at_product_publication_boundary() {
        let mut app = RunenwerkEditorApp::new();
        let mut publications = ProductPublicationRuntimeResource::default();

        let report = bake_procgen_products(&mut app, &mut publications, &boundary());

        assert!(report.accepted);
        assert_eq!(report.published_count, 1);
        assert_eq!(report.operation_count, 2);
        assert_eq!(report.product_count, 2);
        assert_eq!(app.procgen_runtime().published_descriptors().len(), 3);
        assert!(
            app.procgen_runtime()
                .published_descriptors()
                .iter()
                .any(|descriptor| descriptor.scale_band == ProductScaleBand::Offline)
        );
        assert!(app.procgen_runtime().last_bake().is_some());
        assert!(app.procgen_runtime().last_query_snapshot_key.is_none());
    }

    #[test]
    fn procgen_rollback_restores_last_good_bake_after_invalid_document() {
        let mut app = RunenwerkEditorApp::new();
        let mut publications = ProductPublicationRuntimeResource::default();

        let bake_report = bake_procgen_products(&mut app, &mut publications, &boundary());
        assert!(bake_report.accepted);
        let baked_descriptor_count = app.procgen_runtime().published_descriptors().len();
        let baked_product_count = app.procgen_runtime().formed_preview_products().len();

        app.procgen_runtime_mut().document_mut().scope = ProcgenScope::new(WorldId::new(1), [], []);
        publish_procgen_products(&mut app, &mut publications, &boundary());
        assert_eq!(
            app.procgen_runtime().published_descriptors().len(),
            baked_descriptor_count
        );
        assert_eq!(
            app.procgen_runtime().formed_preview_products().len(),
            baked_product_count
        );

        app.procgen_runtime_mut().published_descriptors.clear();
        app.procgen_runtime_mut()
            .clear_formed_preview_products(Vec::new());
        app.procgen_runtime_mut().last_publication_key = None;

        let rollback_report = rollback_procgen_bake(&mut app);

        assert!(rollback_report.accepted);
        assert_eq!(
            rollback_report.restored_descriptor_count,
            baked_descriptor_count
        );
        assert_eq!(rollback_report.restored_product_count, baked_product_count);
        assert_eq!(
            app.procgen_runtime().published_descriptors().len(),
            baked_descriptor_count
        );
        assert_eq!(
            app.procgen_runtime().formed_preview_products().len(),
            baked_product_count
        );
    }

    #[test]
    fn procgen_query_snapshots_publish_after_product_publication_and_pass_strict_renderer_policy() {
        let mut app = RunenwerkEditorApp::new();
        let mut publications = ProductPublicationRuntimeResource::default();
        let mut snapshots = QuerySnapshotRuntimeResource::default();

        publish_procgen_products(&mut app, &mut publications, &boundary());
        let report = publish_procgen_query_snapshots(&mut app, &mut snapshots, &boundary());

        assert_eq!(report.published_count, 3);
        for product_id in app.procgen_runtime().active_overlay_product_ids() {
            let snapshot = snapshots
                .current_snapshot(ProductIdentity::new(product_id.0))
                .expect("procgen query snapshot should publish");
            let decision = evaluate_product_consumption(
                &snapshot.descriptor,
                &product::ProductConsumptionRequest::new(
                    ProductConsumerClass::Renderer,
                    ProductQueryPolicy::StrictCurrentOnly,
                ),
            );
            assert!(decision.is_accepted());
        }
    }

    #[test]
    fn invalid_procgen_document_does_not_publish_or_select_overlay_products() {
        let mut app = RunenwerkEditorApp::new();
        let mut publications = ProductPublicationRuntimeResource::default();
        publish_procgen_products(&mut app, &mut publications, &boundary());
        assert_eq!(app.procgen_runtime().published_descriptors().len(), 3);
        assert_eq!(app.procgen_runtime().formed_preview_products().len(), 2);

        app.procgen_runtime_mut().document_mut().scope = ProcgenScope::new(WorldId::new(1), [], []);

        let report = publish_procgen_products(&mut app, &mut publications, &boundary());

        assert_eq!(report.published_count, 0);
        assert!(app.procgen_runtime().published_descriptors().is_empty());
        assert!(app.procgen_runtime().formed_preview_products().is_empty());
        assert!(
            app.procgen_runtime()
                .active_overlay_product_ids()
                .is_empty()
        );
    }

    #[test]
    fn procgen_viewport_overlay_sync_adds_and_removes_app_owned_overlay_products() {
        let mut app = RunenwerkEditorApp::new();
        let mut publications = ProductPublicationRuntimeResource::default();
        let mut presentations = ViewportPresentationStateResource::default();
        presentations.upsert_state(ViewportPresentationState::new(
            editor_viewport::ViewportId(1),
            crate::runtime::viewport::SCENE_COLOR_PRODUCT_ID,
        ));

        publish_procgen_products(&mut app, &mut publications, &boundary());
        let active_ids = app.procgen_runtime().active_overlay_product_ids();
        sync_procgen_viewport_overlays(&app, &mut presentations);
        let state = presentations
            .state_for(editor_viewport::ViewportId(1))
            .unwrap();
        assert_eq!(state.selected_overlay_product_ids, active_ids);

        app.procgen_runtime_mut().document_mut().scope = ProcgenScope::new(WorldId::new(1), [], []);
        publish_procgen_products(&mut app, &mut publications, &boundary());
        sync_procgen_viewport_overlays(&app, &mut presentations);
        let state = presentations
            .state_for(editor_viewport::ViewportId(1))
            .unwrap();
        assert!(state.selected_overlay_product_ids.is_empty());
    }

    #[test]
    fn procgen_query_snapshots_wait_for_product_publication() {
        let mut app = RunenwerkEditorApp::new();
        let mut snapshots = QuerySnapshotRuntimeResource::default();

        let report = publish_procgen_query_snapshots(&mut app, &mut snapshots, &boundary());

        assert_eq!(report.published_count, 0);
        assert!(snapshots.current_snapshots().is_empty());
    }
}
