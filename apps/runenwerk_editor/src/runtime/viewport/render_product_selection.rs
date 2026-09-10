//! File: apps/runenwerk_editor/src/runtime/viewport/render_product_selection.rs
//! Purpose: App-owned viewport render product selection producer.

use std::collections::BTreeSet;

use editor_viewport::{
    ExpressionFormat, ExpressionProductId, ViewportId, ViewportPresentationState,
};
use engine::plugins::render::{PreparedRenderProductSelectionResource, RenderTextureTargetFormat};
use engine::runtime::{QuerySnapshotRuntimeResource, Res, ResMut};
use product::{
    FieldProductDiagnostic, FieldProductDiagnosticCode, ProductConsumerClass,
    ProductConsumptionRequest, ProductIdentity, ProductQueryPolicy, ProductResidency,
    RenderProductSelection, RenderResidencyRequest, RenderSelectedProduct, RenderTargetDescriptor,
    evaluate_product_consumption,
};

use crate::editor_app::RunenwerkEditorApp;
use crate::runtime::resources::EditorHostResource;
use crate::runtime::viewport::{
    EDITOR_VIEWPORT_RENDER_PRODUCT_PRODUCER_ID, OVERLAY_PRODUCT_ID, PICKING_IDS_PRODUCT_ID,
    ViewportPresentationStateResource, ViewportProductTargetRecord,
    ViewportProductTargetRegistryResource, ViewportProductTargetStatus, ViewportRenderJobResource,
    prepared_view_id,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditorViewportRenderSelectionJournalEntry {
    pub viewport_id: ViewportId,
    pub view_id: String,
    pub selected_product_count: usize,
    pub rejected_product_count: usize,
    pub diagnostic_count: usize,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct EditorViewportRenderSelectionSummary {
    pub selection_count: usize,
    pub selected_product_count: usize,
    pub rejected_product_count: usize,
    pub diagnostic_count: usize,
}

#[allow(clippy::too_many_arguments)]
pub fn prepare_viewport_render_product_selections_system(
    mut host: ResMut<EditorHostResource>,
    snapshots: Res<QuerySnapshotRuntimeResource>,
    presentations: Res<ViewportPresentationStateResource>,
    product_targets: Res<ViewportProductTargetRegistryResource>,
    render_jobs: Res<ViewportRenderJobResource>,
    mut prepared_selections: ResMut<PreparedRenderProductSelectionResource>,
) {
    prepare_viewport_render_product_selections(
        &mut host.app,
        &snapshots,
        &presentations,
        &product_targets,
        &render_jobs,
        &mut prepared_selections,
    );
}

pub fn prepare_viewport_render_product_selections(
    app: &mut RunenwerkEditorApp,
    snapshots: &QuerySnapshotRuntimeResource,
    presentations: &ViewportPresentationStateResource,
    product_targets: &ViewportProductTargetRegistryResource,
    render_jobs: &ViewportRenderJobResource,
    prepared_selections: &mut PreparedRenderProductSelectionResource,
) -> EditorViewportRenderSelectionSummary {
    let mut selections = Vec::new();
    let mut summary = EditorViewportRenderSelectionSummary::default();

    for job in render_jobs.jobs() {
        let view_id = prepared_view_id(job.viewport_id);
        let Some(presentation) = presentations.state_for(job.viewport_id) else {
            let mut selection = RenderProductSelection::new(view_id.clone());
            selection
                .diagnostics
                .push(missing_viewport_presentation_diagnostic(job.viewport_id));
            summary.rejected_product_count = summary.rejected_product_count.saturating_add(1);
            summary.diagnostic_count = summary
                .diagnostic_count
                .saturating_add(selection.diagnostics.len());
            record_selection_entry(app, job.viewport_id, &selection, 1);
            selections.push(selection);
            continue;
        };

        let mut selection = RenderProductSelection::new(view_id.clone());
        add_required_viewport_targets(
            job.viewport_id,
            presentation,
            product_targets,
            &mut selection,
        );

        let mut rejected_products = 0usize;
        if !add_selected_product(
            presentation.selected_primary_product_id,
            true,
            snapshots,
            &mut selection,
        ) {
            rejected_products = rejected_products.saturating_add(1);
        }

        let mut overlays = presentation
            .selected_overlay_product_ids
            .iter()
            .copied()
            .collect::<BTreeSet<_>>();
        overlays.remove(&presentation.selected_primary_product_id);
        for overlay_product_id in overlays {
            if !add_selected_product(overlay_product_id, false, snapshots, &mut selection) {
                rejected_products = rejected_products.saturating_add(1);
            }
        }

        summary.selected_product_count = summary
            .selected_product_count
            .saturating_add(selection.selected_products.len());
        summary.rejected_product_count = summary
            .rejected_product_count
            .saturating_add(rejected_products);
        summary.diagnostic_count = summary
            .diagnostic_count
            .saturating_add(selection.diagnostics.len());
        record_selection_entry(app, job.viewport_id, &selection, rejected_products);
        selections.push(selection);
    }

    summary.selection_count = selections.len();
    if let Err(err) = prepared_selections
        .replace_contribution(EDITOR_VIEWPORT_RENDER_PRODUCT_PRODUCER_ID, selections)
    {
        summary.diagnostic_count = summary.diagnostic_count.saturating_add(1);
        app.append_console_warning(format!("[render_selection] {err}"));
    }

    if app.update_viewport_render_selection_summary(summary) {
        app.append_console_line(format!(
            "[render_selection] selections={} selected={} rejected={} diagnostics={}",
            summary.selection_count,
            summary.selected_product_count,
            summary.rejected_product_count,
            summary.diagnostic_count
        ));
    }

    summary
}

fn add_required_viewport_targets(
    viewport_id: ViewportId,
    presentation: &ViewportPresentationState,
    product_targets: &ViewportProductTargetRegistryResource,
    selection: &mut RenderProductSelection,
) {
    let required = [
        (
            editor_viewport::ViewportSurfacePresentationSlot::Primary,
            presentation.selected_primary_product_id,
        ),
        (
            editor_viewport::ViewportSurfacePresentationSlot::Picking,
            PICKING_IDS_PRODUCT_ID,
        ),
        (
            editor_viewport::ViewportSurfacePresentationSlot::Overlay,
            OVERLAY_PRODUCT_ID,
        ),
    ];

    for (slot, product_id) in required {
        let Some(record) = product_targets.record_for_product(viewport_id, slot, product_id) else {
            selection
                .diagnostics
                .push(missing_target_diagnostic(product_id));
            continue;
        };
        if let Some(target) = render_target_descriptor_for_record(record) {
            selection.required_targets.push(target);
        } else {
            selection
                .diagnostics
                .push(unavailable_target_diagnostic(product_id));
        }
    }
}

fn add_selected_product(
    product_id: ExpressionProductId,
    primary: bool,
    snapshots: &QuerySnapshotRuntimeResource,
    selection: &mut RenderProductSelection,
) -> bool {
    let identity = ProductIdentity::new(product_id.0);
    let Some(snapshot) = snapshots.current_snapshot(identity) else {
        selection
            .diagnostics
            .push(missing_snapshot_diagnostic(identity));
        return false;
    };

    let request = ProductConsumptionRequest::new(
        ProductConsumerClass::Renderer,
        ProductQueryPolicy::StrictCurrentOnly,
    );
    let decision = evaluate_product_consumption(&snapshot.descriptor, &request);
    if !decision.is_accepted() {
        selection.extend_diagnostics(decision.diagnostics);
        return false;
    }

    selection
        .selected_products
        .push(RenderSelectedProduct::from_query_snapshot(snapshot));
    selection
        .residency_requests
        .push(RenderResidencyRequest::new(
            identity,
            ProductResidency::Resident,
            if primary { 100 } else { 50 },
            primary,
        ));
    true
}

fn render_target_descriptor_for_record(
    record: &ViewportProductTargetRecord,
) -> Option<RenderTargetDescriptor> {
    if record.status != ViewportProductTargetStatus::Requested {
        return None;
    }
    let format = record
        .target_format
        .map(render_target_format_label)
        .unwrap_or_else(|| expression_format_label(&record.format));
    if format.trim().is_empty() {
        return None;
    }
    Some(RenderTargetDescriptor::new(
        record.dynamic_key().label(),
        record.width,
        record.height,
        format,
    ))
}

fn render_target_format_label(format: RenderTextureTargetFormat) -> String {
    match format {
        RenderTextureTargetFormat::Rgba8Unorm => "rgba8_unorm".to_string(),
        RenderTextureTargetFormat::Rgba8UnormSrgb => "rgba8_unorm_srgb".to_string(),
        RenderTextureTargetFormat::R32Uint => "r32_uint".to_string(),
        RenderTextureTargetFormat::Depth32Float => "depth32_float".to_string(),
    }
}

fn expression_format_label(format: &ExpressionFormat) -> String {
    match format {
        ExpressionFormat::Rgba8Unorm => "rgba8_unorm".to_string(),
        ExpressionFormat::R32Uint => "r32_uint".to_string(),
        ExpressionFormat::Depth32Float => "depth32_float".to_string(),
        ExpressionFormat::Other(value) => value.clone(),
    }
}

fn record_selection_entry(
    app: &mut RunenwerkEditorApp,
    viewport_id: ViewportId,
    selection: &RenderProductSelection,
    rejected_product_count: usize,
) {
    app.record_viewport_render_selection(EditorViewportRenderSelectionJournalEntry {
        viewport_id,
        view_id: selection.view_id.clone(),
        selected_product_count: selection.selected_products.len(),
        rejected_product_count,
        diagnostic_count: selection.diagnostics.len(),
    });
}

fn missing_viewport_presentation_diagnostic(viewport_id: ViewportId) -> FieldProductDiagnostic {
    FieldProductDiagnostic::blocking(
        FieldProductDiagnosticCode::MissingProduct,
        format!(
            "viewport {} render selection has no presentation state",
            viewport_id.0
        ),
    )
}

fn missing_snapshot_diagnostic(product_id: ProductIdentity) -> FieldProductDiagnostic {
    FieldProductDiagnostic::blocking(
        FieldProductDiagnosticCode::MissingProduct,
        "viewport render selection requires a published query snapshot",
    )
    .for_product(product_id)
}

fn missing_target_diagnostic(product_id: ExpressionProductId) -> FieldProductDiagnostic {
    FieldProductDiagnostic::blocking(
        FieldProductDiagnosticCode::MissingProduct,
        "viewport render selection requires a product target record",
    )
    .for_product(ProductIdentity::new(product_id.0))
}

fn unavailable_target_diagnostic(product_id: ExpressionProductId) -> FieldProductDiagnostic {
    FieldProductDiagnostic::blocking(
        FieldProductDiagnosticCode::NonResident,
        "viewport render selection target is unavailable",
    )
    .for_product(ProductIdentity::new(product_id.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    use editor_core::RealityVersion;
    use editor_viewport::{
        ExpressionDimensions, ViewportPresentationState, ViewportSurfacePresentationSlot,
    };
    use engine::plugins::render::{
        PreparedFlowInvocationRequest, PreparedViewFrame, RenderFlowId, RenderProductSurfaceRequest,
    };
    use engine::runtime::PublicationBoundary;
    use product::{
        ProductAuthorityClass, ProductDescriptorCore, ProductFamily, ProductKind, ProductLineage,
        ProductScaleBand, ProductScope, QuerySnapshotProductDescriptor,
    };
    use ui_math::UiRect;

    use crate::runtime::viewport::{
        SCENE_COLOR_PRODUCT_ID, ViewportRenderJob, initial_product_descriptors,
        material_preview_descriptor,
    };

    fn publication_boundary() -> PublicationBoundary {
        PublicationBoundary::new(1, "RenderPrepare", 0)
    }

    fn snapshot(product_id: ExpressionProductId) -> QuerySnapshotProductDescriptor {
        let mut descriptor = ProductDescriptorCore::new(
            ProductIdentity::new(product_id.0),
            ProductFamily::Expression,
            ProductKind::new("viewport_product"),
            ProductScope::View {
                view_id: "1".to_string(),
            },
            ProductScaleBand::Preview,
            ProductLineage::new("test.viewport", 7),
        );
        descriptor.consumer_class = ProductConsumerClass::Renderer;
        descriptor.residency = ProductResidency::Resident;
        descriptor.authority_class = ProductAuthorityClass::DeterministicDerived;
        QuerySnapshotProductDescriptor::new(descriptor, 7, 7, ProductQueryPolicy::StrictCurrentOnly)
    }

    fn target_registry(viewport_id: ViewportId) -> ViewportProductTargetRegistryResource {
        let descriptors =
            initial_product_descriptors(ExpressionDimensions::new(320, 200), RealityVersion(1));
        ViewportProductTargetRegistryResource::from_descriptors_for_viewport(
            viewport_id,
            &descriptors,
        )
    }

    fn target_registry_with_material_preview(
        viewport_id: ViewportId,
        product_id: ExpressionProductId,
    ) -> ViewportProductTargetRegistryResource {
        let mut descriptors =
            initial_product_descriptors(ExpressionDimensions::new(320, 200), RealityVersion(1));
        descriptors.push(material_preview_descriptor(
            product_id,
            ExpressionDimensions::new(320, 200),
            RealityVersion(1),
            "material.first_slice.render_material".to_string(),
        ));
        ViewportProductTargetRegistryResource::from_descriptors_for_viewport(
            viewport_id,
            &descriptors,
        )
    }

    fn render_job(
        viewport_id: ViewportId,
        targets: &ViewportProductTargetRegistryResource,
    ) -> ViewportRenderJob {
        let scene_color_target = targets
            .record_for_product(
                viewport_id,
                ViewportSurfacePresentationSlot::Primary,
                SCENE_COLOR_PRODUCT_ID,
            )
            .expect("scene color target should exist")
            .dynamic_key();
        let picking_ids_target = targets
            .record_for_product(
                viewport_id,
                ViewportSurfacePresentationSlot::Picking,
                PICKING_IDS_PRODUCT_ID,
            )
            .expect("picking target should exist")
            .dynamic_key();
        let overlay_target = targets
            .record_for_product(
                viewport_id,
                ViewportSurfacePresentationSlot::Overlay,
                OVERLAY_PRODUCT_ID,
            )
            .expect("overlay target should exist")
            .dynamic_key();
        let view_id = prepared_view_id(viewport_id);

        ViewportRenderJob {
            viewport_id,
            bounds: UiRect::new(0.0, 0.0, 320.0, 200.0),
            dimensions: ExpressionDimensions::new(320, 200),
            scene_color_target,
            picking_ids_target,
            overlay_target,
            product_surface_request: RenderProductSurfaceRequest::new(
                PreparedViewFrame::offscreen_product(view_id.clone(), (320, 200)),
                PreparedFlowInvocationRequest::new(
                    "viewport.test",
                    RenderFlowId::try_from_raw(1).unwrap(),
                    view_id,
                ),
            ),
        }
    }

    #[test]
    fn render_product_selection_accepts_primary_and_overlay_snapshots() {
        let viewport_id = ViewportId(1);
        let targets = target_registry(viewport_id);
        let mut jobs = ViewportRenderJobResource::default();
        jobs.replace_jobs([render_job(viewport_id, &targets)]);

        let mut presentations = ViewportPresentationStateResource::default();
        let mut presentation = ViewportPresentationState::new(viewport_id, SCENE_COLOR_PRODUCT_ID);
        presentation.set_overlay_products(vec![OVERLAY_PRODUCT_ID]);
        presentations.upsert_state(presentation);

        let mut snapshots = QuerySnapshotRuntimeResource::default();
        snapshots.stage(snapshot(SCENE_COLOR_PRODUCT_ID));
        snapshots.stage(snapshot(OVERLAY_PRODUCT_ID));
        snapshots.publish_staged(&publication_boundary());

        let mut app = RunenwerkEditorApp::new();
        let mut prepared = PreparedRenderProductSelectionResource::default();

        let summary = prepare_viewport_render_product_selections(
            &mut app,
            &snapshots,
            &presentations,
            &targets,
            &jobs,
            &mut prepared,
        );

        let selection = &prepared.snapshot()[0];
        assert_eq!(summary.selection_count, 1);
        assert_eq!(summary.selected_product_count, 2);
        assert_eq!(summary.rejected_product_count, 0);
        assert_eq!(selection.required_targets.len(), 3);
        assert_eq!(selection.selected_products.len(), 2);
        assert_eq!(selection.residency_requests.len(), 2);
        assert!(selection.residency_requests[0].hard_pin);
        assert_eq!(app.viewport_render_selection_journal().len(), 1);
    }

    #[test]
    fn render_product_selection_rejects_missing_snapshots_with_diagnostics() {
        let viewport_id = ViewportId(1);
        let targets = target_registry(viewport_id);
        let mut jobs = ViewportRenderJobResource::default();
        jobs.replace_jobs([render_job(viewport_id, &targets)]);

        let mut presentations = ViewportPresentationStateResource::default();
        presentations.upsert_state(ViewportPresentationState::new(
            viewport_id,
            SCENE_COLOR_PRODUCT_ID,
        ));

        let snapshots = QuerySnapshotRuntimeResource::default();
        let mut app = RunenwerkEditorApp::new();
        let mut prepared = PreparedRenderProductSelectionResource::default();

        let summary = prepare_viewport_render_product_selections(
            &mut app,
            &snapshots,
            &presentations,
            &targets,
            &jobs,
            &mut prepared,
        );

        let selection = &prepared.snapshot()[0];
        assert_eq!(summary.selected_product_count, 0);
        assert_eq!(summary.rejected_product_count, 1);
        assert_eq!(selection.selected_products.len(), 0);
        assert!(
            selection
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == FieldProductDiagnosticCode::MissingProduct)
        );
    }

    #[test]
    fn render_product_selection_uses_selected_material_preview_target_as_primary() {
        let viewport_id = ViewportId(1);
        let material_product_id = ExpressionProductId(420);
        let targets = target_registry_with_material_preview(viewport_id, material_product_id);
        let mut jobs = ViewportRenderJobResource::default();
        jobs.replace_jobs([render_job(viewport_id, &targets)]);

        let mut presentations = ViewportPresentationStateResource::default();
        presentations.upsert_state(ViewportPresentationState::new(
            viewport_id,
            material_product_id,
        ));

        let mut snapshots = QuerySnapshotRuntimeResource::default();
        snapshots.stage(snapshot(material_product_id));
        snapshots.publish_staged(&publication_boundary());

        let mut app = RunenwerkEditorApp::new();
        let mut prepared = PreparedRenderProductSelectionResource::default();

        let summary = prepare_viewport_render_product_selections(
            &mut app,
            &snapshots,
            &presentations,
            &targets,
            &jobs,
            &mut prepared,
        );

        let selection = &prepared.snapshot()[0];
        let material_target = targets
            .record_for_product(
                viewport_id,
                ViewportSurfacePresentationSlot::Primary,
                material_product_id,
            )
            .expect("material target should exist")
            .dynamic_key()
            .label();
        let scene_target = targets
            .record_for_product(
                viewport_id,
                ViewportSurfacePresentationSlot::Primary,
                SCENE_COLOR_PRODUCT_ID,
            )
            .expect("scene target should exist")
            .dynamic_key()
            .label();

        assert_eq!(summary.rejected_product_count, 0);
        assert_eq!(selection.required_targets.len(), 3);
        assert!(
            selection
                .required_targets
                .iter()
                .any(|target| target.target_id == material_target)
        );
        assert!(
            !selection
                .required_targets
                .iter()
                .any(|target| target.target_id == scene_target)
        );
        assert_eq!(
            selection.selected_products[0].product_id,
            ProductIdentity::new(material_product_id.0)
        );
    }
}
