//! Draw-owned GPU ink proof wiring through public render-flow contracts.

use std::collections::BTreeMap;

use anyhow::Result;

use drawing::DrawingInkTileProduct;
use engine::plugins::render::inspect::{
    CaptureStage, CaptureTextureClass, RenderCaptureSelector, RenderDebugConfigResource,
    RenderDebugFrameReport, RenderDebugFrameReportState, RenderTextureDiffMetrics,
    RenderTextureDiffRequest, RenderTextureDiffResult, RenderTextureDiffStatus,
};
use engine::plugins::render::{
    PreparedFlowInvocationId, PreparedFlowInvocationRequest, PreparedRenderFrameRequestResource,
    PreparedTargetBinding, PreparedViewFrame, RenderDynamicTextureRetention,
    RenderDynamicTextureTargetDescriptor, RenderDynamicTextureTargetKey, RenderFlow, RenderFlowId,
    RenderFrameProducerId, RenderPassId, RenderTargetAliasKey, RenderTargetAliasKind,
    RenderTextureSampleMode, RenderTextureTargetFormat, RenderTextureTargetUsage,
};
use engine::runtime::{Res, ResMut};
use runen_gpu::GpuBindingKey;

use crate::app::{
    DRAWING_INK_TEXTURE_NAMESPACE, DrawingInkGpuValidationMetrics, DrawingInkSurfaceKind,
    RunenwerkDrawApp, drawing_ink_texture_target_id,
};

const DRAWING_GPU_INK_FLOW_LABEL: &str = "runenwerk.draw.ink.gpu.proof";
const DRAWING_GPU_INK_CPU_INPUT_ALIAS: &str = "runenwerk.draw.ink.cpu.input";
const DRAWING_GPU_INK_CPU_REFERENCE_TARGET: &str = "runenwerk.draw.ink.cpu.reference";
const DRAWING_GPU_INK_SCRATCH_TARGET: &str = "runenwerk.draw.ink.gpu.scratch";
const DRAWING_GPU_INK_OUTPUT_ALIAS: &str = "runenwerk.draw.ink.gpu.output";
const DRAWING_GPU_INK_CPU_REFERENCE_PASS: &str = "runenwerk.draw.ink.cpu.reference.copy";
const DRAWING_GPU_INK_COMPUTE_PASS: &str = "runenwerk.draw.ink.gpu.compute";
const DRAWING_GPU_INK_OUTPUT_COPY_PASS: &str = "runenwerk.draw.ink.gpu.output.copy";
const DRAWING_GPU_INK_DIFF_PREFIX: &str = "runenwerk.draw.ink.gpu.diff.";
const DRAWING_GPU_MAX_CHANNEL_DELTA: u8 = 2;
const DRAWING_GPU_MAX_CHANGED_PIXELS_PER_MILLION: u32 = 10_000;

#[derive(Debug, Clone, Copy, runen_ecs::Resource)]
pub struct DrawingInkGpuFlowResource {
    pub flow_id: RenderFlowId,
    pub cpu_reference_pass_id: RenderPassId,
    pub gpu_output_pass_id: RenderPassId,
}

#[derive(Debug, Default, runen_ecs::Resource)]
pub struct DrawingInkGpuValidationReportCursorResource {
    last_frame_index: Option<u64>,
}

#[derive(Debug, Clone)]
struct DrawingInkGpuValidationCandidate {
    surface_kind: DrawingInkSurfaceKind,
    product: DrawingInkTileProduct,
}

pub fn register_drawing_ink_gpu_flow(app: &mut engine::App) -> Result<()> {
    let (flow, resource) = build_drawing_ink_gpu_flow()?;
    app.add_render_flow(flow);
    app.insert_resource(resource);
    Ok(())
}

fn build_drawing_ink_gpu_flow() -> Result<(RenderFlow, DrawingInkGpuFlowResource)> {
    let scratch_storage_binding = GpuBindingKey::try_new(0, 0)?;
    let flow = RenderFlow::new(DRAWING_GPU_INK_FLOW_LABEL)
        .with_target_alias(
            DRAWING_GPU_INK_CPU_INPUT_ALIAS,
            RenderTargetAliasKind::Texture,
        )?
        .with_color_target_exact(
            DRAWING_GPU_INK_CPU_REFERENCE_TARGET,
            RenderTextureTargetFormat::Rgba8Unorm,
        )?
        .with_storage_texture(DRAWING_GPU_INK_SCRATCH_TARGET)?
        .with_target_alias(DRAWING_GPU_INK_OUTPUT_ALIAS, RenderTargetAliasKind::Texture)?
        .copy_pass(DRAWING_GPU_INK_CPU_REFERENCE_PASS)
        .offscreen_products_only()
        .source(DRAWING_GPU_INK_CPU_INPUT_ALIAS)
        .destination(DRAWING_GPU_INK_CPU_REFERENCE_TARGET)
        .finish()
        .compute_pass(DRAWING_GPU_INK_COMPUTE_PASS)
        .offscreen_products_only()
        .write_texture(scratch_storage_binding, DRAWING_GPU_INK_SCRATCH_TARGET)
        .dispatch([1, 1, 1])
        .finish()
        .copy_pass(DRAWING_GPU_INK_OUTPUT_COPY_PASS)
        .offscreen_products_only()
        .source(DRAWING_GPU_INK_SCRATCH_TARGET)
        .destination(DRAWING_GPU_INK_OUTPUT_ALIAS)
        .finish()
        .validate()
        .expect("drawing GPU ink proof flow should validate");
    let resource = DrawingInkGpuFlowResource {
        flow_id: flow.id(),
        cpu_reference_pass_id: flow
            .pass_id(DRAWING_GPU_INK_CPU_REFERENCE_PASS)
            .expect("CPU reference pass id should be registered"),
        gpu_output_pass_id: flow
            .pass_id(DRAWING_GPU_INK_OUTPUT_COPY_PASS)
            .expect("GPU output copy pass id should be registered"),
    };
    Ok((flow, resource))
}

pub fn process_drawing_ink_gpu_validation_report_system(
    mut host: ResMut<crate::runtime::resources::DrawingHostResource>,
    report_state: Res<RenderDebugFrameReportState>,
    mut cursor: ResMut<DrawingInkGpuValidationReportCursorResource>,
) {
    let Some(report) = report_state.latest.as_ref() else {
        return;
    };
    if cursor.last_frame_index == Some(report.frame_index) {
        return;
    }
    cursor.last_frame_index = Some(report.frame_index);
    if apply_drawing_ink_gpu_validation_report(&mut host.app, report) {
        host.app.rebuild_visible_frame();
    }
}

pub fn prepare_drawing_ink_gpu_frame(
    app: &mut RunenwerkDrawApp,
    producer_id: RenderFrameProducerId,
    flow: &DrawingInkGpuFlowResource,
    frame_requests: &mut PreparedRenderFrameRequestResource,
    debug_config: &mut RenderDebugConfigResource,
    committed_products: &[DrawingInkTileProduct],
    preview_products: &[DrawingInkTileProduct],
) -> Vec<RenderDynamicTextureTargetDescriptor> {
    let candidate = next_gpu_validation_candidate(app, committed_products, preview_products);
    if let Some(candidate) = candidate.as_ref() {
        app.ink_runtime_mut()
            .record_gpu_validation_pending(candidate.surface_kind, &candidate.product);
    }

    let descriptors = gpu_target_descriptors(app, committed_products, preview_products);
    replace_gpu_validation_debug_config(debug_config, flow, candidate.as_ref());
    replace_gpu_validation_invocation(frame_requests, producer_id, flow, candidate.as_ref());
    descriptors
}

pub fn apply_drawing_ink_gpu_validation_report(
    app: &mut RunenwerkDrawApp,
    report: &RenderDebugFrameReport,
) -> bool {
    let candidates = current_gpu_validation_products(app);
    let mut changed = false;
    for result in &report.texture_diff_results {
        if !result.diff_id.starts_with(DRAWING_GPU_INK_DIFF_PREFIX) {
            continue;
        }
        let Some(candidate) = candidates.get(result.diff_id.as_str()) else {
            continue;
        };
        match result.status {
            RenderTextureDiffStatus::Skipped => {}
            RenderTextureDiffStatus::Compared => {
                let Some(metrics) = result.metrics else {
                    continue;
                };
                if drawing_gpu_metrics_pass(metrics) {
                    app.ink_runtime_mut().record_gpu_validation_pass(
                        candidate.surface_kind,
                        &candidate.product,
                        gpu_metrics(metrics),
                    );
                    changed = true;
                } else {
                    app.ink_runtime_mut().record_gpu_validation_failure(
                        candidate.surface_kind,
                        &candidate.product,
                        gpu_failure_reason(result, metrics),
                    );
                    changed = true;
                }
            }
            RenderTextureDiffStatus::Failed => {
                let reason = result
                    .message
                    .as_ref()
                    .map(|message| message.detail.clone())
                    .unwrap_or_else(|| "gpu validation texture diff failed".to_string());
                app.ink_runtime_mut().record_gpu_validation_failure(
                    candidate.surface_kind,
                    &candidate.product,
                    reason,
                );
                changed = true;
            }
        }
    }
    changed
}

fn next_gpu_validation_candidate(
    app: &RunenwerkDrawApp,
    committed_products: &[DrawingInkTileProduct],
    preview_products: &[DrawingInkTileProduct],
) -> Option<DrawingInkGpuValidationCandidate> {
    committed_products
        .iter()
        .find(|product| {
            app.ink_runtime()
                .should_request_gpu_validation(DrawingInkSurfaceKind::Committed, product)
        })
        .map(|product| DrawingInkGpuValidationCandidate {
            surface_kind: DrawingInkSurfaceKind::Committed,
            product: product.clone(),
        })
        .or_else(|| {
            preview_products
                .iter()
                .find(|product| {
                    app.ink_runtime()
                        .should_request_gpu_validation(DrawingInkSurfaceKind::Preview, product)
                })
                .map(|product| DrawingInkGpuValidationCandidate {
                    surface_kind: DrawingInkSurfaceKind::Preview,
                    product: product.clone(),
                })
        })
}

fn gpu_target_descriptors(
    app: &RunenwerkDrawApp,
    committed_products: &[DrawingInkTileProduct],
    preview_products: &[DrawingInkTileProduct],
) -> Vec<RenderDynamicTextureTargetDescriptor> {
    committed_products
        .iter()
        .filter(|product| {
            app.ink_runtime()
                .should_request_gpu_target(DrawingInkSurfaceKind::Committed, product)
        })
        .map(|product| gpu_target_descriptor(DrawingInkSurfaceKind::Committed, product))
        .chain(
            preview_products
                .iter()
                .filter(|product| {
                    app.ink_runtime()
                        .should_request_gpu_target(DrawingInkSurfaceKind::Preview, product)
                })
                .map(|product| gpu_target_descriptor(DrawingInkSurfaceKind::Preview, product)),
        )
        .collect()
}

fn gpu_target_descriptor(
    surface_kind: DrawingInkSurfaceKind,
    product: &DrawingInkTileProduct,
) -> RenderDynamicTextureTargetDescriptor {
    RenderDynamicTextureTargetDescriptor::new(
        gpu_target_key(surface_kind, product),
        product.payload.width.max(1),
        product.payload.height.max(1),
        RenderTextureTargetFormat::Rgba8Unorm,
        gpu_texture_target_usage(),
        RenderTextureSampleMode::FilterableFloat,
        RenderDynamicTextureRetention::RetainWhileRequested,
    )
}

fn replace_gpu_validation_invocation(
    frame_requests: &mut PreparedRenderFrameRequestResource,
    producer_id: RenderFrameProducerId,
    flow: &DrawingInkGpuFlowResource,
    candidate: Option<&DrawingInkGpuValidationCandidate>,
) {
    let Some(candidate) = candidate else {
        frame_requests.remove_contribution(producer_id);
        return;
    };

    let view_id = gpu_validation_view_id(candidate);
    let mut aliases = BTreeMap::new();
    aliases.insert(
        RenderTargetAliasKey::new(DRAWING_GPU_INK_CPU_INPUT_ALIAS)
            .expect("drawing CPU input alias key is a valid constant"),
        PreparedTargetBinding::DynamicTexture(cpu_target_key(
            candidate.surface_kind,
            &candidate.product,
        )),
    );
    aliases.insert(
        RenderTargetAliasKey::new(DRAWING_GPU_INK_OUTPUT_ALIAS)
            .expect("drawing output alias key is a valid constant"),
        PreparedTargetBinding::DynamicTexture(gpu_target_key(
            candidate.surface_kind,
            &candidate.product,
        )),
    );
    let request = PreparedFlowInvocationRequest {
        invocation_id: PreparedFlowInvocationId::new(gpu_validation_invocation_id(candidate)),
        flow_id: flow.flow_id,
        view_id: view_id.clone(),
        target_alias_bindings: aliases,
        uniform_overrides: BTreeMap::new(),
        history_signature: None,
    };
    if let Err(err) = frame_requests.replace_contribution(
        producer_id,
        [PreparedViewFrame::offscreen_product(
            view_id,
            (
                candidate.product.payload.width.max(1),
                candidate.product.payload.height.max(1),
            ),
        )],
        [request],
    ) {
        tracing::warn!(target = "runenwerk_draw.ink", error = %err, "drawing GPU ink validation request rejected");
    }
}

fn replace_gpu_validation_debug_config(
    debug_config: &mut RenderDebugConfigResource,
    flow: &DrawingInkGpuFlowResource,
    candidate: Option<&DrawingInkGpuValidationCandidate>,
) {
    let flow_id = flow.flow_id.to_string();
    debug_config
        .capture_selectors
        .retain(|selector| selector.flow_id.as_deref() != Some(flow_id.as_str()));
    debug_config
        .texture_diffs
        .retain(|diff| !diff.id.starts_with(DRAWING_GPU_INK_DIFF_PREFIX));

    let Some(candidate) = candidate else {
        return;
    };

    let cpu_selector = RenderCaptureSelector {
        flow_id: Some(flow_id.clone()),
        pass_id: Some(flow.cpu_reference_pass_id.to_string()),
        stage: CaptureStage::After,
        resource_id: DRAWING_GPU_INK_CPU_REFERENCE_TARGET.to_string(),
        texture_class: CaptureTextureClass::ColorTarget,
    };
    let gpu_selector = RenderCaptureSelector {
        flow_id: Some(flow_id),
        pass_id: Some(flow.gpu_output_pass_id.to_string()),
        stage: CaptureStage::After,
        resource_id: DRAWING_GPU_INK_OUTPUT_ALIAS.to_string(),
        texture_class: CaptureTextureClass::ImportedTexture,
    };
    debug_config.capture_selectors.push(cpu_selector.clone());
    debug_config.capture_selectors.push(gpu_selector.clone());
    debug_config.texture_diffs.push(
        RenderTextureDiffRequest::new(
            gpu_validation_diff_id(candidate),
            gpu_selector,
            cpu_selector,
        )
        .with_thresholds(
            DRAWING_GPU_MAX_CHANNEL_DELTA,
            DRAWING_GPU_MAX_CHANGED_PIXELS_PER_MILLION,
        ),
    );
}

fn current_gpu_validation_products(
    app: &RunenwerkDrawApp,
) -> BTreeMap<String, DrawingInkGpuValidationCandidate> {
    app.ink_runtime()
        .visible_products()
        .map(|product| DrawingInkGpuValidationCandidate {
            surface_kind: DrawingInkSurfaceKind::Committed,
            product: product.clone(),
        })
        .chain(app.ink_runtime().preview_products().iter().map(|product| {
            DrawingInkGpuValidationCandidate {
                surface_kind: DrawingInkSurfaceKind::Preview,
                product: product.clone(),
            }
        }))
        .map(|candidate| (gpu_validation_diff_id(&candidate), candidate))
        .collect()
}

fn drawing_gpu_metrics_pass(metrics: RenderTextureDiffMetrics) -> bool {
    metrics.max_delta <= DRAWING_GPU_MAX_CHANNEL_DELTA
        && changed_pixels_per_million(metrics) <= DRAWING_GPU_MAX_CHANGED_PIXELS_PER_MILLION as u64
}

fn gpu_metrics(metrics: RenderTextureDiffMetrics) -> DrawingInkGpuValidationMetrics {
    DrawingInkGpuValidationMetrics {
        max_channel_delta: metrics.max_delta,
        changed_pixel_count: metrics.changed_pixel_count,
        total_pixel_count: metrics.total_pixel_count,
        changed_pixel_ratio: metrics.changed_pixel_ratio,
    }
}

fn gpu_failure_reason(
    result: &RenderTextureDiffResult,
    metrics: RenderTextureDiffMetrics,
) -> String {
    result
        .message
        .as_ref()
        .map(|message| message.detail.clone())
        .unwrap_or_else(|| {
            format!(
                "gpu validation exceeded thresholds: max_delta={} changed_ppm={}",
                metrics.max_delta,
                changed_pixels_per_million(metrics)
            )
        })
}

fn changed_pixels_per_million(metrics: RenderTextureDiffMetrics) -> u64 {
    if metrics.total_pixel_count == 0 {
        return 0;
    }
    metrics.changed_pixel_count.saturating_mul(1_000_000) / metrics.total_pixel_count
}

fn gpu_validation_view_id(candidate: &DrawingInkGpuValidationCandidate) -> String {
    format!("{}.view", gpu_validation_token(candidate))
}

fn gpu_validation_invocation_id(candidate: &DrawingInkGpuValidationCandidate) -> String {
    format!("{}.invoke", gpu_validation_token(candidate))
}

fn gpu_validation_diff_id(candidate: &DrawingInkGpuValidationCandidate) -> String {
    format!("{}.diff", gpu_validation_token(candidate))
}

fn gpu_validation_token(candidate: &DrawingInkGpuValidationCandidate) -> String {
    let tile = candidate.product.metadata.tile_id;
    format!(
        "{}{}{}.L{}.{}.{}.{}",
        DRAWING_GPU_INK_DIFF_PREFIX,
        source_surface_token(candidate.surface_kind),
        candidate.product.metadata.quality_class.cache_token(),
        tile.level.raw(),
        tile.x,
        tile.y,
        candidate.product.descriptor_generation
    )
}

fn source_surface_token(surface_kind: DrawingInkSurfaceKind) -> &'static str {
    match surface_kind {
        DrawingInkSurfaceKind::Committed | DrawingInkSurfaceKind::GpuCommitted => "committed.",
        DrawingInkSurfaceKind::Preview | DrawingInkSurfaceKind::GpuPreview => "preview.",
    }
}

fn cpu_target_key(
    surface_kind: DrawingInkSurfaceKind,
    product: &DrawingInkTileProduct,
) -> RenderDynamicTextureTargetKey {
    RenderDynamicTextureTargetKey::new(
        DRAWING_INK_TEXTURE_NAMESPACE,
        drawing_ink_texture_target_id(
            surface_kind,
            product.metadata.quality_class,
            product.metadata.tile_id,
        ),
    )
}

fn gpu_target_key(
    surface_kind: DrawingInkSurfaceKind,
    product: &DrawingInkTileProduct,
) -> RenderDynamicTextureTargetKey {
    RenderDynamicTextureTargetKey::new(
        DRAWING_INK_TEXTURE_NAMESPACE,
        drawing_ink_texture_target_id(
            surface_kind.gpu_variant(),
            product.metadata.quality_class,
            product.metadata.tile_id,
        ),
    )
}

fn gpu_texture_target_usage() -> RenderTextureTargetUsage {
    RenderTextureTargetUsage {
        color_attachment: false,
        depth_attachment: false,
        sampled: true,
        storage: false,
        copy_src: true,
        copy_dst: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use engine::plugins::render::{
        RenderResourceDeclaration, RenderTextureFormatPolicy, RenderTextureSizePolicy,
    };

    #[test]
    fn cpu_reference_target_is_exact_rgba8_unorm_proof_data() {
        let (flow, _resource) =
            build_drawing_ink_gpu_flow().expect("drawing ink GPU flow authoring should succeed");
        let id = flow
            .resource_id(DRAWING_GPU_INK_CPU_REFERENCE_TARGET)
            .expect("CPU reference target should be registered");
        let descriptor = flow
            .graph()
            .resources
            .resources
            .iter()
            .find(|descriptor| *descriptor.id() == id)
            .expect("CPU reference target should have a descriptor");

        let RenderResourceDeclaration::ColorAttachment(target) = descriptor else {
            panic!("CPU reference proof data should be a flow-owned color target");
        };
        assert_eq!(target.texture.size, RenderTextureSizePolicy::Surface);
        assert_eq!(
            target.texture.format,
            RenderTextureFormatPolicy::Exact(RenderTextureTargetFormat::Rgba8Unorm)
        );
    }
}
