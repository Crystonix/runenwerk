use super::{
    RenderDebugFrameReport, RenderPixelProbeResult, RenderPixelProbeStatus,
    RenderSelectorResolution, RenderTextureDiffResult, RenderTextureDiffStatus,
};
use crate::plugins::diagnostics::core::ingest::{
    DiagnosticsEntrySubmission, submit_diagnostics_entry,
};
use crate::plugins::diagnostics::core::model::{
    DiagnosticsAttachment, DiagnosticsEntry, DiagnosticsSeverity, DiagnosticsStatus,
};
use serde::Serialize;

const RENDER_PRODUCER_ID: &str = "render.inspect";
const RENDER_DOMAIN_ID: &str = "render";
const RENDER_SCHEMA_ID: &str = "runenwerk.render.frame_report";
const RENDER_SCHEMA_VERSION: u32 = 2;

#[derive(Debug, Serialize)]
pub struct RenderDiagnosticsEntryDto {
    pub frame_index: u64,
    pub provenance_count: usize,
    pub capture_selector_count: usize,
    pub capture_result_count: usize,
    pub completed_capture_count: usize,
    pub failed_capture_count: usize,
    pub pixel_probe_count: usize,
    pub pixel_probe_failed_count: usize,
    pub texture_diff_count: usize,
    pub texture_diff_failed_count: usize,
    pub warning_count: usize,
    pub error_count: usize,
    pub artifact_manifest_path: Option<String>,
    pub selectors: Vec<RenderCaptureSelectorDto>,
    pub capture_results: Vec<RenderCaptureResultDto>,
    pub pixel_probes: Vec<RenderPixelProbeDto>,
    pub texture_diffs: Vec<RenderTextureDiffDto>,
}

#[derive(Debug, Serialize)]
pub struct RenderCaptureSelectorDto {
    pub selector_index: usize,
    pub flow_id: Option<String>,
    pub pass_id: Option<String>,
    pub stage: String,
    pub resource_id: String,
    pub texture_class: String,
    pub resolution: String,
    pub reason_code: Option<String>,
    pub reason_detail: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct RenderCaptureResultDto {
    pub selector_index: usize,
    pub frame_index: Option<u64>,
    pub flow_id: String,
    pub pass_id: String,
    pub stage: String,
    pub resource_id: String,
    pub texture_class: String,
    pub terminal_code: String,
    pub terminal_reason_code: Option<String>,
    pub terminal_reason_detail: Option<String>,
    pub artifact_path: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct RenderPixelProbeDto {
    pub probe_id: String,
    pub status: String,
    pub message_code: Option<String>,
    pub message_detail: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct RenderTextureDiffDto {
    pub diff_id: String,
    pub status: String,
    pub mismatch_sample_count: usize,
    pub total_pixel_count: Option<u64>,
    pub changed_pixel_count: Option<u64>,
    pub changed_pixel_ratio: Option<f32>,
    pub max_delta: Option<u8>,
    pub mean_delta: Option<f32>,
    pub diff_image_path: Option<String>,
    pub message_code: Option<String>,
    pub message_detail: Option<String>,
}

#[derive(Debug, Serialize)]
struct RenderManifestAttachmentMetadata {
    frame_index: u64,
}

#[derive(Debug, Serialize)]
struct RenderCaptureImageAttachmentMetadata {
    frame_index: u64,
    selector_index: usize,
    flow_id: String,
    pass_id: String,
    stage: String,
    resource_id: String,
}

#[derive(Debug, Serialize)]
struct RenderTextureDiffAttachmentMetadata {
    frame_index: u64,
    diff_id: String,
}

pub fn submit_render_frame_report_to_diagnostics(
    world: &mut runen_ecs::World,
    report: &RenderDebugFrameReport,
    simulation_tick: u64,
) -> anyhow::Result<()> {
    let submission = map_render_report_to_submission(report, simulation_tick)?;
    submit_diagnostics_entry(world, submission)
}

pub fn map_render_report_to_entry_dto(
    report: &RenderDebugFrameReport,
) -> RenderDiagnosticsEntryDto {
    let completed_capture_count = report
        .capture_results
        .iter()
        .filter(|value| value.terminal.code.as_str() == "completed")
        .count();

    let failed_capture_count = report
        .capture_results
        .iter()
        .filter(|value| value.terminal.code.is_failure())
        .count();

    let pixel_probe_failed_count = report
        .pixel_probe_results
        .iter()
        .filter(|value| matches!(value.status, RenderPixelProbeStatus::Failed))
        .count();

    let texture_diff_failed_count = report
        .texture_diff_results
        .iter()
        .filter(|value| matches!(value.status, RenderTextureDiffStatus::Failed))
        .count();

    RenderDiagnosticsEntryDto {
        frame_index: report.frame_index,
        provenance_count: report.provenance.len(),
        capture_selector_count: report.capture_plan.selectors.len(),
        capture_result_count: report.capture_results.len(),
        completed_capture_count,
        failed_capture_count,
        pixel_probe_count: report.pixel_probe_results.len(),
        pixel_probe_failed_count,
        texture_diff_count: report.texture_diff_results.len(),
        texture_diff_failed_count,
        warning_count: report.warnings.len(),
        error_count: report.errors.len(),
        artifact_manifest_path: report
            .artifact_manifest_path
            .as_ref()
            .map(|value| value.display().to_string()),
        selectors: report
            .capture_plan
            .selectors
            .iter()
            .map(|value| {
                let (resolution, reason_code, reason_detail) =
                    capture_selector_resolution_parts(&value.resolution);
                RenderCaptureSelectorDto {
                    selector_index: value.selector_index,
                    flow_id: value.selector.flow_id.clone(),
                    pass_id: value.selector.pass_id.clone(),
                    stage: value.selector.stage.as_str().to_string(),
                    resource_id: value.selector.resource_id.clone(),
                    texture_class: value.selector.texture_class.as_str().to_string(),
                    resolution,
                    reason_code,
                    reason_detail,
                }
            })
            .collect(),
        capture_results: report
            .capture_results
            .iter()
            .map(|value| RenderCaptureResultDto {
                selector_index: value.selector_index,
                frame_index: value
                    .frame_identity
                    .as_ref()
                    .map(|identity| identity.frame_index),
                flow_id: value.capture_point.flow_id.clone(),
                pass_id: value.capture_point.pass_id.clone(),
                stage: value.capture_point.stage.as_str().to_string(),
                resource_id: value.capture_point.resource_id.clone(),
                texture_class: value.capture_point.texture_class.as_str().to_string(),
                terminal_code: value.terminal.code.as_str().to_string(),
                terminal_reason_code: value
                    .terminal
                    .reason
                    .as_ref()
                    .map(|reason| reason.code.clone()),
                terminal_reason_detail: value
                    .terminal
                    .reason
                    .as_ref()
                    .map(|reason| reason.detail.clone()),
                artifact_path: value
                    .artifact_path
                    .as_ref()
                    .map(|path| path.display().to_string()),
            })
            .collect(),
        pixel_probes: report
            .pixel_probe_results
            .iter()
            .map(pixel_probe_to_dto)
            .collect(),
        texture_diffs: report
            .texture_diff_results
            .iter()
            .map(texture_diff_to_dto)
            .collect(),
    }
}

fn map_render_report_to_submission(
    report: &RenderDebugFrameReport,
    simulation_tick: u64,
) -> anyhow::Result<DiagnosticsEntrySubmission> {
    let payload_dto = map_render_report_to_entry_dto(report);
    let payload_json = serde_json::to_value(payload_dto)?;

    let mut attachments = Vec::<DiagnosticsAttachment>::new();

    if let Some(manifest_path) = report.artifact_manifest_path.as_ref() {
        attachments.push(DiagnosticsAttachment {
            attachment_id: format!("render.manifest.{}", report.frame_index),
            kind: "render.capture_manifest".to_string(),
            label: "Render Capture Manifest".to_string(),
            producer_id: RENDER_PRODUCER_ID.to_string(),
            domain_id: RENDER_DOMAIN_ID.to_string(),
            path_or_handle: manifest_path.display().to_string(),
            metadata: Some(serde_json::to_value(RenderManifestAttachmentMetadata {
                frame_index: report.frame_index,
            })?),
        });
    }

    for (index, result) in report.capture_results.iter().enumerate() {
        let Some(path) = result.artifact_path.as_ref() else {
            continue;
        };
        let capture_frame_index = result
            .frame_identity
            .as_ref()
            .map(|identity| identity.frame_index)
            .unwrap_or(report.frame_index);
        attachments.push(DiagnosticsAttachment {
            attachment_id: format!("render.capture.image.{capture_frame_index}.{index}"),
            kind: "render.capture_image".to_string(),
            label: format!(
                "Capture {}:{}:{}:{}",
                result.capture_point.flow_id,
                result.capture_point.pass_id,
                result.capture_point.stage.as_str(),
                result.capture_point.resource_id
            ),
            producer_id: RENDER_PRODUCER_ID.to_string(),
            domain_id: RENDER_DOMAIN_ID.to_string(),
            path_or_handle: path.display().to_string(),
            metadata: Some(serde_json::to_value(
                RenderCaptureImageAttachmentMetadata {
                    frame_index: capture_frame_index,
                    selector_index: result.selector_index,
                    flow_id: result.capture_point.flow_id.clone(),
                    pass_id: result.capture_point.pass_id.clone(),
                    stage: result.capture_point.stage.as_str().to_string(),
                    resource_id: result.capture_point.resource_id.clone(),
                },
            )?),
        });
    }

    for (index, diff) in report.texture_diff_results.iter().enumerate() {
        let Some(path) = diff.diff_image_path.as_ref() else {
            continue;
        };
        attachments.push(DiagnosticsAttachment {
            attachment_id: format!(
                "render.texture.diff.image.{}.{}.{}",
                report.frame_index,
                index,
                sanitize_token(diff.diff_id.as_str())
            ),
            kind: "render.texture_diff_image".to_string(),
            label: format!("Texture Diff {}", diff.diff_id),
            producer_id: RENDER_PRODUCER_ID.to_string(),
            domain_id: RENDER_DOMAIN_ID.to_string(),
            path_or_handle: path.display().to_string(),
            metadata: Some(serde_json::to_value(RenderTextureDiffAttachmentMetadata {
                frame_index: report.frame_index,
                diff_id: diff.diff_id.clone(),
            })?),
        });
    }

    let attachment_ids = attachments
        .iter()
        .map(|value| value.attachment_id.clone())
        .collect::<Vec<_>>();

    let severity = if !report.errors.is_empty() {
        DiagnosticsSeverity::Error
    } else if !report.warnings.is_empty() {
        DiagnosticsSeverity::Warning
    } else {
        DiagnosticsSeverity::Info
    };

    let status = if !report.errors.is_empty() {
        DiagnosticsStatus::Failed
    } else if !report.warnings.is_empty() {
        DiagnosticsStatus::Degraded
    } else {
        DiagnosticsStatus::Ok
    };

    Ok(DiagnosticsEntrySubmission {
        frame_index: report.frame_index,
        simulation_tick,
        entry: DiagnosticsEntry {
            entry_id: format!("render.inspect.frame.{}", report.frame_index),
            producer_id: RENDER_PRODUCER_ID.to_string(),
            domain_id: RENDER_DOMAIN_ID.to_string(),
            schema_id: RENDER_SCHEMA_ID.to_string(),
            schema_version: RENDER_SCHEMA_VERSION,
            payload_json,
            severity,
            status,
            attachment_ids,
        },
        attachments,
    })
}

fn capture_selector_resolution_parts(
    resolution: &RenderSelectorResolution,
) -> (String, Option<String>, Option<String>) {
    match resolution {
        RenderSelectorResolution::Pending { .. } => ("pending".to_string(), None, None),
        RenderSelectorResolution::Matched { .. } => ("matched".to_string(), None, None),
        RenderSelectorResolution::Unmatched { reason } => (
            "unmatched".to_string(),
            Some(reason.code.clone()),
            Some(reason.detail.clone()),
        ),
        RenderSelectorResolution::Disabled { reason } => (
            "disabled".to_string(),
            Some(reason.code.clone()),
            Some(reason.detail.clone()),
        ),
        RenderSelectorResolution::Unsupported { reason } => (
            "unsupported".to_string(),
            Some(reason.code.clone()),
            Some(reason.detail.clone()),
        ),
        RenderSelectorResolution::Skipped { reason } => (
            "skipped".to_string(),
            Some(reason.code.clone()),
            Some(reason.detail.clone()),
        ),
    }
}

fn pixel_probe_to_dto(value: &RenderPixelProbeResult) -> RenderPixelProbeDto {
    RenderPixelProbeDto {
        probe_id: value.probe_id.clone(),
        status: match value.status {
            RenderPixelProbeStatus::Sampled => "sampled".to_string(),
            RenderPixelProbeStatus::Passed => "passed".to_string(),
            RenderPixelProbeStatus::Failed => "failed".to_string(),
            RenderPixelProbeStatus::Skipped => "skipped".to_string(),
        },
        message_code: value.message.as_ref().map(|reason| reason.code.clone()),
        message_detail: value.message.as_ref().map(|reason| reason.detail.clone()),
    }
}

fn texture_diff_to_dto(value: &RenderTextureDiffResult) -> RenderTextureDiffDto {
    RenderTextureDiffDto {
        diff_id: value.diff_id.clone(),
        status: match value.status {
            RenderTextureDiffStatus::Compared => "compared".to_string(),
            RenderTextureDiffStatus::Skipped => "skipped".to_string(),
            RenderTextureDiffStatus::Failed => "failed".to_string(),
        },
        mismatch_sample_count: value.mismatch_samples.len(),
        total_pixel_count: value
            .metrics
            .as_ref()
            .map(|metrics| metrics.total_pixel_count),
        changed_pixel_count: value
            .metrics
            .as_ref()
            .map(|metrics| metrics.changed_pixel_count),
        changed_pixel_ratio: value
            .metrics
            .as_ref()
            .map(|metrics| metrics.changed_pixel_ratio),
        max_delta: value.metrics.as_ref().map(|metrics| metrics.max_delta),
        mean_delta: value.metrics.as_ref().map(|metrics| metrics.mean_delta),
        diff_image_path: value
            .diff_image_path
            .as_ref()
            .map(|path| path.display().to_string()),
        message_code: value.message.as_ref().map(|reason| reason.code.clone()),
        message_detail: value.message.as_ref().map(|reason| reason.detail.clone()),
    }
}

fn sanitize_token(value: &str) -> String {
    value
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' })
        .collect::<String>()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugins::diagnostics::{
        DiagnosticsConfigResource, DiagnosticsPendingReportsResource, ResolvedDiagnosticsPlan,
    };
    use crate::runtime::SimulationTick;

    #[test]
    fn render_mapping_keeps_schema_and_attachment_contract_fields() {
        let report = RenderDebugFrameReport::default();
        let submission = map_render_report_to_submission(&report, 0)
            .expect("render report should map to diagnostics submission");

        assert_eq!(submission.entry.producer_id, "render.inspect");
        assert_eq!(submission.entry.domain_id, "render");
        assert_eq!(submission.entry.schema_id, "runenwerk.render.frame_report");
        assert_eq!(submission.entry.schema_version, 2);
        assert_eq!(submission.simulation_tick, 0);
    }

    #[test]
    fn delayed_render_submission_uses_original_tick_not_current_world_tick() {
        let mut world = runen_ecs::World::new();
        world.insert_resource(DiagnosticsConfigResource::default());
        world.insert_resource(ResolvedDiagnosticsPlan::default());
        world.insert_resource(DiagnosticsPendingReportsResource::default());
        world.insert_resource(SimulationTick(99));

        let report = RenderDebugFrameReport {
            frame_index: 7,
            ..RenderDebugFrameReport::default()
        };
        submit_render_frame_report_to_diagnostics(&mut world, &report, 42)
            .expect("delayed render report should submit with its original tick");

        let pending = world
            .resource::<DiagnosticsPendingReportsResource>()
            .expect("pending diagnostics reports should be installed");
        let diagnostics_report = pending
            .by_frame_index
            .get(&7)
            .expect("render diagnostics report should retain semantic frame identity");
        assert_eq!(diagnostics_report.frame_index, 7);
        assert_eq!(diagnostics_report.simulation_tick, 42);
    }
}
