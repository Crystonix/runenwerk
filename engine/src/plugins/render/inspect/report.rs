use super::{
    RenderCaptureIdentity, RenderCapturePointIdentity, RenderCaptureSelectorResult,
    RenderCaptureTerminalReason, RenderPixelCoordinate, RenderPixelProbeAssertionMode,
    RenderPixelSampleMode, RenderTextureDiffRequest, ResolvedRenderCapturePlan,
    validate_selector_terminal_invariant,
};
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenderPixelProbeStatus {
    Sampled,
    Passed,
    Failed,
    Skipped,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RenderPixelProbeResult {
    pub probe_id: String,
    pub capture_point_identity: RenderCapturePointIdentity,
    pub frame_identity: Option<RenderCaptureIdentity>,
    pub sample_mode: RenderPixelSampleMode,
    pub resolved_coordinate: Option<RenderPixelCoordinate>,
    pub comparison_mode: RenderPixelProbeAssertionMode,
    pub sampled_rgba8: Option<[u8; 4]>,
    pub compared_rgba8: Option<[u8; 4]>,
    pub status: RenderPixelProbeStatus,
    pub message: Option<RenderCaptureTerminalReason>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenderTextureDiffStatus {
    Compared,
    Skipped,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RenderTextureDiffMetrics {
    pub total_pixel_count: u64,
    pub changed_pixel_count: u64,
    pub changed_pixel_ratio: f32,
    pub max_delta: u8,
    pub mean_delta: f32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderTextureDiffMismatchSample {
    pub coordinate: RenderPixelCoordinate,
    pub left_rgba8: [u8; 4],
    pub right_rgba8: [u8; 4],
    pub max_channel_delta: u8,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RenderTextureDiffResult {
    pub diff_id: String,
    pub request: RenderTextureDiffRequest,
    pub left_capture_point: RenderCapturePointIdentity,
    pub right_capture_point: RenderCapturePointIdentity,
    pub left_frame_identity: Option<RenderCaptureIdentity>,
    pub right_frame_identity: Option<RenderCaptureIdentity>,
    pub status: RenderTextureDiffStatus,
    pub metrics: Option<RenderTextureDiffMetrics>,
    pub mismatch_samples: Vec<RenderTextureDiffMismatchSample>,
    pub diff_image_path: Option<PathBuf>,
    pub message: Option<RenderCaptureTerminalReason>,
}

#[derive(Debug, Clone, Default, runen_ecs::Component, runen_ecs::Resource)]
pub struct RenderDebugFrameReportState {
    pub latest: Option<RenderDebugFrameReport>,
}

impl RenderDebugFrameReportState {
    pub fn observe_frame(&mut self, report: RenderDebugFrameReport) {
        self.latest = Some(report);
    }
}

#[derive(Debug, Clone, Default)]
pub struct RenderDebugFrameReport {
    pub frame_index: u64,
    pub provenance: Vec<crate::plugins::render::inspect::RenderPassProvenanceRecord>,
    pub capture_plan: ResolvedRenderCapturePlan,
    pub capture_results: Vec<RenderCaptureSelectorResult>,
    pub artifact_manifest_path: Option<PathBuf>,
    pub pixel_probe_results: Vec<RenderPixelProbeResult>,
    pub texture_diff_results: Vec<RenderTextureDiffResult>,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
}

impl RenderDebugFrameReport {
    pub fn validate_invariants(&self) -> Vec<String> {
        match validate_selector_terminal_invariant(&self.capture_plan, &self.capture_results) {
            Ok(()) => Vec::new(),
            Err(violations) => violations
                .into_iter()
                .map(|violation| {
                    format!(
                        "selector invariant violation at index {}: {}",
                        violation.selector_index, violation.message
                    )
                })
                .collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugins::render::inspect::{
        CaptureStage, CaptureTextureClass, RenderCaptureSelector, RenderCaptureTerminal,
    };

    fn selector_result(
        selector_index: usize,
        selector: RenderCaptureSelector,
    ) -> RenderCaptureSelectorResult {
        RenderCaptureSelectorResult {
            selector_index,
            capture_point: selector.stable_point_fallback(),
            selector,
            frame_identity: None,
            terminal: RenderCaptureTerminal::completed(),
            artifact_path: None,
        }
    }

    #[test]
    fn frame_report_state_keeps_latest_only_by_default() {
        let selector = RenderCaptureSelector {
            flow_id: Some("flow".to_string()),
            pass_id: Some("pass".to_string()),
            stage: CaptureStage::After,
            resource_id: "surface.color".to_string(),
            texture_class: CaptureTextureClass::ImportedTexture,
        };
        let mut state = RenderDebugFrameReportState::default();

        state.observe_frame(RenderDebugFrameReport {
            frame_index: 1,
            capture_results: vec![selector_result(0, selector.clone())],
            ..RenderDebugFrameReport::default()
        });
        state.observe_frame(RenderDebugFrameReport {
            frame_index: 2,
            capture_results: vec![selector_result(0, selector.clone())],
            ..RenderDebugFrameReport::default()
        });

        let latest = state
            .latest
            .as_ref()
            .expect("latest report should be present");
        assert_eq!(latest.frame_index, 2);
        assert_eq!(latest.capture_results.len(), 1);
    }
}
