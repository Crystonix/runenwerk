//! Drawing app-owned ink tile publication and query snapshot handlers.

use anyhow::Result;
use drawing::{
    DrawingInkTileFormation, DrawingTileFormationPolicy, build_drawing_ink_tile_product_contracts,
    build_drawing_ink_tile_publication_outcome, drawing_committed_ink_tile_source_cache_key,
    drawing_ink_tile_product_cache_identity, drawing_ink_tile_query_snapshot_for_descriptor,
    form_drawing_ink_tiles_for_ids,
};
use engine::runtime::{
    ProductPublicationRuntimeResource, PublicationBoundary, QuerySnapshotRuntimeResource,
    RuntimeJobExecutorResource, RuntimeJobStatus, RuntimeProductCacheResource,
};
use product::{
    ProductCacheDecisionKind, ProductDescriptorCore, ProductPublicationReport,
    QuerySnapshotPublicationReport, QuerySnapshotPublicationStatus,
};
use runen_ecs::World;

use crate::app::{
    DrawingInkJournalStage, DrawingPreviewTileJobSnapshot, DrawingPreviewTileJobTracker,
    RunenwerkDrawApp,
};
use crate::runtime::ink_jobs::{
    DrawingCommittedInkTileJob, DrawingCommittedInkTileJobOutput, DrawingPreviewInkTileJob,
    DrawingPreviewInkTileJobOutput, drawing_committed_ink_job_key,
};
use crate::runtime::resources::DrawingHostResource;

const DRAWING_INK_PUBLICATION_STAGE_SEQUENCE: u64 = 5_100;

struct DrawingInkCacheLookupRequest {
    document: drawing::DrawingDocument,
    dirty_tiles: Vec<drawing::CanvasTileId>,
    policy: DrawingTileFormationPolicy,
    formation_key: String,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct DrawingPreviewInkJobProcessReport {
    pub submitted_count: usize,
    pub applied_count: usize,
    pub failed_count: usize,
    pub stale_count: usize,
}

pub fn publish_drawing_ink_products_at_boundary(
    boundary: &PublicationBoundary,
    world: &mut World,
) -> Result<()> {
    let Some(mut host) = world.remove_resource::<DrawingHostResource>() else {
        return Ok(());
    };
    let Some(mut publications) = world.remove_resource::<ProductPublicationRuntimeResource>()
    else {
        world.insert_resource(host);
        return Ok(());
    };

    if let Some(mut executor) = world.remove_resource::<RuntimeJobExecutorResource>() {
        if let Some(mut cache) = world.remove_resource::<RuntimeProductCacheResource>() {
            publish_drawing_ink_products_with_executor_and_cache(
                &mut host.app,
                &mut publications,
                &mut executor,
                &mut cache,
                boundary,
            );
            world.insert_resource(cache);
        } else {
            publish_drawing_ink_products_with_executor(
                &mut host.app,
                &mut publications,
                &mut executor,
                boundary,
            );
        }
        world.insert_resource(executor);
    } else {
        publish_drawing_ink_products(&mut host.app, &mut publications, boundary);
    }

    world.insert_resource(publications);
    world.insert_resource(host);
    Ok(())
}

pub fn publish_drawing_ink_query_snapshots_at_boundary(
    boundary: &PublicationBoundary,
    world: &mut World,
) -> Result<()> {
    let Some(mut host) = world.remove_resource::<DrawingHostResource>() else {
        return Ok(());
    };
    let Some(mut snapshots) = world.remove_resource::<QuerySnapshotRuntimeResource>() else {
        world.insert_resource(host);
        return Ok(());
    };

    publish_drawing_ink_query_snapshots(&mut host.app, &mut snapshots, boundary);

    world.insert_resource(snapshots);
    world.insert_resource(host);
    Ok(())
}

pub fn publish_drawing_ink_products(
    app: &mut RunenwerkDrawApp,
    publications: &mut ProductPublicationRuntimeResource,
    boundary: &PublicationBoundary,
) -> ProductPublicationReport {
    let Some(document) = app.document().cloned() else {
        return ProductPublicationReport::default();
    };
    let policy = committed_ink_tile_policy();
    let dirty_tiles = app
        .ink_runtime()
        .next_dirty_tile_batch(policy.max_affected_tiles);
    if dirty_tiles.is_empty() {
        return ProductPublicationReport::default();
    }
    let formation = form_drawing_ink_tiles_for_ids(&document, dirty_tiles.iter().copied(), policy);
    publish_formed_drawing_ink_products(
        app,
        publications,
        None,
        boundary,
        document,
        dirty_tiles,
        formation,
    )
}

pub fn process_drawing_preview_ink_jobs(
    app: &mut RunenwerkDrawApp,
    executor: &mut RuntimeJobExecutorResource,
) -> DrawingPreviewInkJobProcessReport {
    let mut report = DrawingPreviewInkJobProcessReport::default();
    drain_completed_preview_ink_jobs(app, executor, &mut report);
    submit_next_preview_ink_job(app, executor, &mut report);
    drain_completed_preview_ink_jobs(app, executor, &mut report);
    report
}

fn submit_next_preview_ink_job(
    app: &mut RunenwerkDrawApp,
    executor: &mut RuntimeJobExecutorResource,
    report: &mut DrawingPreviewInkJobProcessReport,
) {
    let Some(snapshot) = app.next_preview_tile_job_snapshot() else {
        return;
    };
    let job = preview_job_from_snapshot(snapshot);
    let tracker = DrawingPreviewTileJobTracker {
        stroke_id: job.preview_stroke.stroke_id,
        document_revision: job.document.revision,
        preview_generation: job.preview_generation,
        preview_sample_count: job.preview_sample_count,
        dirty_start_sample_index: job.dirty_start_sample_index,
        formation_key: job.formation_key.clone(),
    };
    match executor.submit(job) {
        Ok(_) => {
            app.record_pending_preview_tile_job(tracker);
            report.submitted_count = report.submitted_count.saturating_add(1);
        }
        Err(err) => {
            app.ink_runtime_mut().record_journal(
                DrawingInkJournalStage::Formation,
                false,
                format!(
                    "runtime preview ink job submission failed diagnostics={}",
                    err.diagnostics.len()
                ),
            );
            report.failed_count = report.failed_count.saturating_add(1);
        }
    }
}

fn preview_job_from_snapshot(snapshot: DrawingPreviewTileJobSnapshot) -> DrawingPreviewInkTileJob {
    DrawingPreviewInkTileJob::new(
        snapshot.document,
        snapshot.preview_stroke,
        snapshot.dirty_preview_stroke,
        snapshot.dirty_start_sample_index,
        snapshot.preview_generation,
        snapshot.policy,
    )
}

fn drain_completed_preview_ink_jobs(
    app: &mut RunenwerkDrawApp,
    executor: &mut RuntimeJobExecutorResource,
    report: &mut DrawingPreviewInkJobProcessReport,
) {
    let completions = executor.drain_completed::<DrawingPreviewInkTileJobOutput>();
    for completion in completions {
        match completion.status {
            RuntimeJobStatus::Completed => {
                let Some(output) = completion.output else {
                    continue;
                };
                app.clear_pending_preview_tile_job(&output.formation_key);
                if !app.preview_tile_job_can_apply(
                    output.stroke_id,
                    output.document_revision,
                    output.preview_sample_count,
                ) {
                    app.ink_runtime_mut().record_journal(
                        DrawingInkJournalStage::Formation,
                        false,
                        "stale preview ink job ignored",
                    );
                    report.stale_count = report.stale_count.saturating_add(1);
                    continue;
                }
                if !output.formation.is_accepted() {
                    app.ink_runtime_mut()
                        .record_preview_failure(output.formation.diagnostics.clone());
                    app.ink_runtime_mut().record_journal(
                        DrawingInkJournalStage::Formation,
                        false,
                        format!(
                            "preview products=0 diagnostics={}",
                            output.formation.diagnostics.len()
                        ),
                    );
                    app.rebuild_visible_frame();
                    report.failed_count = report.failed_count.saturating_add(1);
                    continue;
                }

                let product_count = output.formation.products.len();
                let cleared_count = output.formation.cleared_tiles.len();
                let diagnostic_count = output.formation.diagnostics.len();
                app.ink_runtime_mut().replace_preview_products_for_tiles(
                    output.dirty_tile_ids,
                    output.formation.products,
                    output.formation.cleared_tiles,
                    output.formation.diagnostics,
                );
                if product_count > 0 || cleared_count > 0 {
                    app.record_applied_preview_tile_job(output.preview_sample_count);
                }
                app.ink_runtime_mut().record_journal(
                    DrawingInkJournalStage::Formation,
                    true,
                    format!(
                        "preview products={} diagnostics={}",
                        product_count, diagnostic_count
                    ),
                );
                app.rebuild_visible_frame();
                report.applied_count = report.applied_count.saturating_add(1);
            }
            RuntimeJobStatus::Failed => {
                app.clear_pending_preview_tile_job_generation(completion.handle.generation.raw());
                app.ink_runtime_mut().record_journal(
                    DrawingInkJournalStage::Formation,
                    false,
                    format!(
                        "runtime preview ink job failed diagnostics={}",
                        completion.diagnostics.len()
                    ),
                );
                report.failed_count = report.failed_count.saturating_add(1);
            }
            RuntimeJobStatus::Stale => {
                app.clear_pending_preview_tile_job_generation(completion.handle.generation.raw());
                app.ink_runtime_mut().record_journal(
                    DrawingInkJournalStage::Formation,
                    false,
                    "stale preview ink job ignored",
                );
                report.stale_count = report.stale_count.saturating_add(1);
            }
        }
    }
}

pub fn publish_drawing_ink_products_with_executor(
    app: &mut RunenwerkDrawApp,
    publications: &mut ProductPublicationRuntimeResource,
    executor: &mut RuntimeJobExecutorResource,
    boundary: &PublicationBoundary,
) -> ProductPublicationReport {
    publish_drawing_ink_products_with_optional_cache(app, publications, executor, None, boundary)
}

pub fn publish_drawing_ink_products_with_executor_and_cache(
    app: &mut RunenwerkDrawApp,
    publications: &mut ProductPublicationRuntimeResource,
    executor: &mut RuntimeJobExecutorResource,
    cache: &mut RuntimeProductCacheResource,
    boundary: &PublicationBoundary,
) -> ProductPublicationReport {
    publish_drawing_ink_products_with_optional_cache(
        app,
        publications,
        executor,
        Some(cache),
        boundary,
    )
}

fn publish_drawing_ink_products_with_optional_cache(
    app: &mut RunenwerkDrawApp,
    publications: &mut ProductPublicationRuntimeResource,
    executor: &mut RuntimeJobExecutorResource,
    mut cache: Option<&mut RuntimeProductCacheResource>,
    boundary: &PublicationBoundary,
) -> ProductPublicationReport {
    let completed_report = publish_completed_drawing_ink_jobs(
        app,
        publications,
        executor,
        cache.as_deref_mut(),
        boundary,
    );
    if product_publication_report_has_activity(&completed_report) {
        return completed_report;
    }

    let Some(document) = app.document().cloned() else {
        return ProductPublicationReport::default();
    };
    let policy = committed_ink_tile_policy();
    let dirty_tiles = app
        .ink_runtime()
        .next_dirty_tile_batch(policy.max_affected_tiles);
    if dirty_tiles.is_empty() {
        return ProductPublicationReport::default();
    }

    let formation_key = drawing_committed_ink_job_key(&document, &dirty_tiles, policy);
    if app.ink_runtime().pending_formation_key() == Some(formation_key.as_str()) {
        return ProductPublicationReport::default();
    }
    if let Some(cache) = cache.as_deref_mut()
        && let Some(report) = try_publish_cached_drawing_ink_products(
            app,
            publications,
            cache,
            boundary,
            DrawingInkCacheLookupRequest {
                document: document.clone(),
                dirty_tiles: dirty_tiles.clone(),
                policy,
                formation_key: formation_key.clone(),
            },
        )
    {
        return report;
    }
    if dirty_tiles.is_empty()
        && app.ink_runtime().last_publication_key() == Some(formation_key.as_str())
    {
        return ProductPublicationReport::default();
    }

    match executor.submit(DrawingCommittedInkTileJob::new(
        document,
        dirty_tiles,
        policy,
    )) {
        Ok(_) => {
            app.ink_runtime_mut()
                .record_pending_formation_job(formation_key);
        }
        Err(err) => {
            app.ink_runtime_mut().record_journal(
                DrawingInkJournalStage::Formation,
                false,
                format!(
                    "runtime job submission failed diagnostics={}",
                    err.diagnostics.len()
                ),
            );
            return ProductPublicationReport::default();
        }
    }

    publish_completed_drawing_ink_jobs(app, publications, executor, cache, boundary)
}

fn try_publish_cached_drawing_ink_products(
    app: &mut RunenwerkDrawApp,
    publications: &mut ProductPublicationRuntimeResource,
    cache: &mut RuntimeProductCacheResource,
    boundary: &PublicationBoundary,
    request: DrawingInkCacheLookupRequest,
) -> Option<ProductPublicationReport> {
    let DrawingInkCacheLookupRequest {
        document,
        dirty_tiles,
        policy,
        formation_key,
    } = request;
    let mut products = Vec::with_capacity(dirty_tiles.len());
    for tile_id in &dirty_tiles {
        let source_key = drawing_committed_ink_tile_source_cache_key(&document, policy, *tile_id)?;
        let (_product_key, product) = app
            .ink_runtime_mut()
            .cached_product_for_source_key(&source_key)?;
        let identity = drawing_ink_tile_product_cache_identity(&product);
        let decision = cache.lookup(&identity);
        match decision.kind {
            ProductCacheDecisionKind::Hit => products.push(product),
            ProductCacheDecisionKind::Stale | ProductCacheDecisionKind::Rejected => {
                cache.record_preserved_last_good(&identity, decision.diagnostics.clone());
                app.ink_runtime_mut().record_journal(
                    DrawingInkJournalStage::Formation,
                    false,
                    format!(
                        "committed ink cache {:?}; preserving last-good",
                        decision.kind
                    ),
                );
                return None;
            }
            ProductCacheDecisionKind::Miss
            | ProductCacheDecisionKind::WriteFailed
            | ProductCacheDecisionKind::PreservedLastGood => {
                app.ink_runtime_mut().record_journal(
                    DrawingInkJournalStage::Formation,
                    false,
                    format!("committed ink cache {:?}; submitting job", decision.kind),
                );
                return None;
            }
        }
    }

    if products.is_empty() {
        return None;
    }

    app.ink_runtime_mut().record_journal(
        DrawingInkJournalStage::Formation,
        true,
        format!("committed ink cache hit products={}", products.len()),
    );
    let formation = DrawingInkTileFormation {
        products,
        cleared_tiles: Vec::new(),
        diagnostics: Vec::new(),
        determinism_key: formation_key,
    };
    Some(publish_formed_drawing_ink_products(
        app,
        publications,
        Some(cache),
        boundary,
        document,
        dirty_tiles,
        formation,
    ))
}

fn publish_completed_drawing_ink_jobs(
    app: &mut RunenwerkDrawApp,
    publications: &mut ProductPublicationRuntimeResource,
    executor: &mut RuntimeJobExecutorResource,
    mut cache: Option<&mut RuntimeProductCacheResource>,
    boundary: &PublicationBoundary,
) -> ProductPublicationReport {
    let completions = executor.drain_completed::<DrawingCommittedInkTileJobOutput>();
    let mut aggregate = ProductPublicationReport::default();
    for completion in completions {
        match completion.status {
            RuntimeJobStatus::Completed => {
                let Some(output) = completion.output else {
                    continue;
                };
                app.ink_runtime_mut()
                    .clear_pending_formation_job(&output.formation_key);
                if app
                    .document()
                    .is_some_and(|document| document.revision != output.document_revision)
                {
                    app.ink_runtime_mut().record_journal(
                        DrawingInkJournalStage::Formation,
                        false,
                        "stale committed ink job ignored after document revision changed",
                    );
                    continue;
                }
                let report = publish_formed_drawing_ink_products(
                    app,
                    publications,
                    cache.as_deref_mut(),
                    boundary,
                    output.document,
                    output.dirty_tiles,
                    output.formation,
                );
                merge_product_publication_report(&mut aggregate, report);
            }
            RuntimeJobStatus::Failed => {
                app.ink_runtime_mut().record_journal(
                    DrawingInkJournalStage::Formation,
                    false,
                    format!(
                        "runtime committed ink job failed diagnostics={}",
                        completion.diagnostics.len()
                    ),
                );
            }
            RuntimeJobStatus::Stale => {
                app.ink_runtime_mut().record_journal(
                    DrawingInkJournalStage::Formation,
                    false,
                    "stale committed ink job ignored",
                );
            }
        }
    }
    aggregate
}

fn publish_formed_drawing_ink_products(
    app: &mut RunenwerkDrawApp,
    publications: &mut ProductPublicationRuntimeResource,
    cache: Option<&mut RuntimeProductCacheResource>,
    boundary: &PublicationBoundary,
    document: drawing::DrawingDocument,
    dirty_tiles: Vec<drawing::CanvasTileId>,
    formation: drawing::DrawingInkTileFormation,
) -> ProductPublicationReport {
    let formation_key = formation.determinism_key.clone();

    if dirty_tiles.is_empty()
        && app.ink_runtime().last_publication_key() == Some(formation_key.as_str())
    {
        return ProductPublicationReport::default();
    }

    if !formation.is_accepted() {
        let diagnostics = formation.diagnostics.clone();
        let clear_preview_products = should_clear_failed_preview(app);
        app.ink_runtime_mut().record_failed_generation(
            formation_key.clone(),
            diagnostics.clone(),
            clear_preview_products,
        );
        app.ink_runtime_mut().record_journal(
            DrawingInkJournalStage::Formation,
            false,
            format!(
                "products={} diagnostics={}",
                formation.products.len(),
                diagnostics.len()
            ),
        );
        return ProductPublicationReport::default();
    }

    if formation.products.is_empty() {
        let cleared_count = formation.cleared_tiles.len();
        app.ink_runtime_mut().record_accepted_clear_generation(
            formation_key.clone(),
            dirty_tiles,
            formation.cleared_tiles,
            formation.diagnostics.clone(),
        );
        app.rebuild_visible_frame();
        app.ink_runtime_mut().record_journal(
            DrawingInkJournalStage::Formation,
            true,
            format!(
                "products=0 cleared={} diagnostics={}",
                cleared_count,
                formation.diagnostics.len()
            ),
        );
        return ProductPublicationReport::default();
    }

    let Some(contracts) = build_drawing_ink_tile_product_contracts(&document, &formation.products)
    else {
        let clear_preview_products = should_clear_failed_preview(app);
        app.ink_runtime_mut().record_failed_generation(
            formation_key.clone(),
            formation.diagnostics.clone(),
            clear_preview_products,
        );
        return ProductPublicationReport::default();
    };
    let Some(outcome) = build_drawing_ink_tile_publication_outcome(
        &document,
        &formation.products,
        DRAWING_INK_PUBLICATION_STAGE_SEQUENCE,
    ) else {
        let clear_preview_products = should_clear_failed_preview(app);
        app.ink_runtime_mut().record_failed_generation(
            formation_key.clone(),
            formation.diagnostics.clone(),
            clear_preview_products,
        );
        return ProductPublicationReport::default();
    };

    publications.stage(outcome);
    let report = publications.publish_staged(boundary);

    if report.published_count > 0 && report.rejected_count == 0 {
        let cache_products = formation.products.clone();
        let cache_descriptors = contracts.output_descriptors.clone();
        app.ink_runtime_mut().record_candidate_products(
            formation_key.clone(),
            dirty_tiles,
            formation.products,
            formation.cleared_tiles,
            formation.diagnostics,
        );
        app.ink_runtime_mut()
            .record_published_descriptors(formation_key, contracts.output_descriptors);
        if let Some(cache) = cache {
            app.ink_runtime_mut()
                .record_cached_products(cache_products.iter());
            cache.record_accepted_descriptors(cache_descriptors);
        }
    } else if report.rejected_count > 0 {
        let clear_preview_products = should_clear_failed_preview(app);
        app.ink_runtime_mut().record_failed_generation(
            formation_key.clone(),
            formation.diagnostics.clone(),
            clear_preview_products,
        );
    }
    app.ink_runtime_mut().record_journal(
        DrawingInkJournalStage::ProductPublication,
        report.rejected_count == 0,
        format!(
            "published={} rejected={} failed_preserved={}",
            report.published_count, report.rejected_count, report.failed_preserved_count
        ),
    );

    report
}

fn product_publication_report_has_activity(report: &ProductPublicationReport) -> bool {
    report.published_count > 0
        || report.failed_preserved_count > 0
        || report.rejected_count > 0
        || !report.diagnostics.is_empty()
}

fn merge_product_publication_report(
    aggregate: &mut ProductPublicationReport,
    report: ProductPublicationReport,
) {
    aggregate.published_count = aggregate
        .published_count
        .saturating_add(report.published_count);
    aggregate.failed_preserved_count = aggregate
        .failed_preserved_count
        .saturating_add(report.failed_preserved_count);
    aggregate.rejected_count = aggregate
        .rejected_count
        .saturating_add(report.rejected_count);
    aggregate.diagnostics.extend(report.diagnostics);
}

fn should_clear_failed_preview(app: &RunenwerkDrawApp) -> bool {
    app.preview_stroke().is_none_or(|preview| !preview.active)
}

fn committed_ink_tile_policy() -> DrawingTileFormationPolicy {
    DrawingTileFormationPolicy::final_quality()
}

pub fn publish_drawing_ink_query_snapshots(
    app: &mut RunenwerkDrawApp,
    snapshots: &mut QuerySnapshotRuntimeResource,
    boundary: &PublicationBoundary,
) -> QuerySnapshotPublicationReport {
    let Some(snapshot_key) = descriptor_generation_key(app.ink_runtime().published_descriptors())
    else {
        return QuerySnapshotPublicationReport::default();
    };
    if app.ink_runtime().last_query_snapshot_key() == Some(snapshot_key.as_str())
        && app.ink_runtime().formed_products().is_empty()
    {
        return QuerySnapshotPublicationReport::default();
    }

    let staged = app
        .ink_runtime()
        .published_descriptors()
        .iter()
        .cloned()
        .map(drawing_ink_tile_query_snapshot_for_descriptor)
        .collect::<Vec<_>>();
    if staged.is_empty() {
        return QuerySnapshotPublicationReport::default();
    }

    snapshots.stage_all(staged);
    let report = snapshots.publish_staged(boundary);
    let accepted = snapshots
        .last_published_entries()
        .iter()
        .filter(|entry| entry.status == QuerySnapshotPublicationStatus::Published)
        .map(|entry| entry.product_id)
        .collect::<Vec<_>>();

    if report.published_count > 0 && report.rejected_count == 0 {
        let preserve_active_preview = app.preview_stroke().is_some_and(|preview| preview.active);
        let accepted = app.ink_runtime_mut().record_accepted_snapshots(
            snapshot_key,
            accepted,
            !preserve_active_preview,
        );
        if accepted && preserve_active_preview {
            app.clear_committed_stroke_overlays_if_clean();
            app.rebuild_visible_frame();
        } else if accepted {
            app.clear_preview_after_committed_acceptance();
        }
    }
    app.ink_runtime_mut().record_journal(
        DrawingInkJournalStage::QuerySnapshotPublication,
        report.rejected_count == 0,
        format!(
            "published={} rejected={} preserved={} invalidated={}",
            report.published_count,
            report.rejected_count,
            report.preserved_count,
            report.invalidated_count
        ),
    );

    report
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

#[cfg(test)]
mod tests {
    use drawing::CanvasCoordinate;
    use engine::runtime::{RuntimeJobExecutorConfig, RuntimeJobExecutorResource};
    use ui_input::{Modifiers, PointerButton, PointerEvent, PointerEventKind, UiInputEvent};
    use ui_math::{UiPoint, UiVector};
    use ui_render_data::UiPrimitive;

    use super::*;
    use crate::app::RunenwerkDrawApp;

    #[test]
    fn compatible_lagging_preview_job_advances_product_coverage() {
        let mut app = RunenwerkDrawApp::new();
        let mut executor =
            RuntimeJobExecutorResource::with_config(RuntimeJobExecutorConfig::serial());
        let start = screen_point_for_canvas(&app, 64.0, 64.0);
        app.dispatch_input(&pointer_event(PointerEventKind::Down, start, 1));
        for step in 1..=2 {
            let position = screen_point_for_canvas(&app, 64.0 + step as f64 * 220.0, 64.0);
            app.dispatch_input(&pointer_event(PointerEventKind::Move, position, 0));
        }

        let mut report = DrawingPreviewInkJobProcessReport::default();
        submit_next_preview_ink_job(&mut app, &mut executor, &mut report);
        assert_eq!(report.submitted_count, 1);
        let submitted_sample_count = app
            .pending_preview_tile_job()
            .expect("preview job should be pending")
            .preview_sample_count;

        for step in 3..=6 {
            let position = screen_point_for_canvas(&app, 64.0 + step as f64 * 220.0, 64.0);
            app.dispatch_input(&pointer_event(PointerEventKind::Move, position, 0));
        }
        assert!(
            app.preview_generation()
                > app
                    .pending_preview_tile_job()
                    .expect("preview job should still be pending")
                    .preview_generation
        );

        drain_completed_preview_ink_jobs(&mut app, &mut executor, &mut report);

        assert_eq!(
            report.stale_count, 0,
            "older compatible preview output should not be discarded just because input advanced"
        );
        assert_eq!(report.applied_count, 1);
        assert_eq!(app.preview_product_sample_count(), submitted_sample_count);
        assert!(!app.ink_runtime().preview_products().is_empty());
        assert!(
            app.preview_dirty_start_sample_index().is_some(),
            "tail input gathered while the job was pending must remain scheduled for catch-up"
        );
        assert!(
            stroke_primitive_count(app.last_frame()) > 0,
            "unformed tail samples should remain visible over the applied preview products"
        );
        let preview_sample_count = app
            .preview_stroke()
            .expect("preview stroke should remain active")
            .samples
            .len();
        let tail_point_count = stroke_primitive_point_count(app.last_frame());
        assert!(
            tail_point_count > 0 && tail_point_count < preview_sample_count,
            "immediate projection should cover only the unformed tail, not the whole long stroke"
        );
    }

    fn pointer_event(kind: PointerEventKind, position: UiPoint, click_count: u8) -> UiInputEvent {
        UiInputEvent::Pointer(PointerEvent::new(
            kind,
            position,
            UiVector::ZERO,
            Some(PointerButton::Primary),
            Modifiers::default(),
            click_count,
        ))
    }

    fn screen_point_for_canvas(app: &RunenwerkDrawApp, x: f64, y: f64) -> UiPoint {
        app.composition_projection()
            .canvas_view
            .canvas_to_screen(CanvasCoordinate::new(x, y))
            .expect("test canvas point should be visible")
    }

    fn stroke_primitive_count(frame: &ui_render_data::UiFrame) -> usize {
        frame
            .surfaces
            .iter()
            .flat_map(|surface| surface.layers.iter())
            .flat_map(|layer| layer.primitives.iter())
            .filter(|primitive| matches!(primitive, UiPrimitive::Stroke(_)))
            .count()
    }

    fn stroke_primitive_point_count(frame: &ui_render_data::UiFrame) -> usize {
        frame
            .surfaces
            .iter()
            .flat_map(|surface| surface.layers.iter())
            .flat_map(|layer| layer.primitives.iter())
            .filter_map(|primitive| match primitive {
                UiPrimitive::Stroke(stroke) => Some(stroke.points.len()),
                _ => None,
            })
            .sum()
    }
}
