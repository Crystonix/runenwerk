use super::*;
use runen_gpu::{
    GpuContext, GpuContextAffinity, GpuExecutionLifecycleState, GpuReadbackId, GpuReadbackStatus,
    GpuSubmission, GpuSubmissionFailure,
};

#[derive(Debug, Default)]
pub(in crate::plugins::render::renderer) struct RendererGpuObservationOutput {
    pub timing_evidence: Vec<RenderPassTimingEvidence>,
    pub captured_textures: Vec<RenderCapturedTexture>,
    pub capture_results: Vec<RenderCaptureSelectorResult>,
}

#[derive(Debug, Default)]
pub(in crate::plugins::render::renderer) struct RendererGpuObservationState {
    accepted: Vec<AcceptedRendererObservation>,
}

#[derive(Debug)]
struct AcceptedRendererObservation {
    submission: GpuSubmission,
    timings: Vec<GpuPassTimingFrame>,
    captures: Vec<CaptureObservation>,
}

#[derive(Debug)]
struct CaptureObservation {
    readback_id: GpuReadbackId,
    selector_index: usize,
    selector: RenderCaptureSelector,
    identity: RenderCaptureIdentity,
    width: u32,
    height: u32,
    source_format: GpuTextureFormat,
    readback_format: TextureReadbackFormat,
}

impl CaptureObservation {
    fn from_prepared(prepared: &PreparedCaptureReadback) -> Self {
        Self {
            readback_id: prepared.canonical_operation().id(),
            selector_index: prepared.selector_index,
            selector: prepared.selector.clone(),
            identity: prepared.identity.clone(),
            width: prepared.width,
            height: prepared.height,
            source_format: prepared.source_format,
            readback_format: texture_readback_format(prepared.source_format)
                .expect("prepared capture already admitted its renderer readback format"),
        }
    }

    fn ready(&self, readback: &runen_gpu::GpuReadbackBytes) -> RenderCapturedTexture {
        let expected_format = self.source_format;
        if readback.texture_format() != Some(expected_format) {
            return self.failed(
                "capture_format_mismatch",
                format!(
                    "capture readback format mismatch: expected {expected_format:?}, got {:?}",
                    readback.texture_format()
                ),
            );
        }
        let expected_len = u64::from(self.width)
            .checked_mul(u64::from(self.height))
            .and_then(|pixels| pixels.checked_mul(4))
            .and_then(|bytes| usize::try_from(bytes).ok());
        let Some(expected_len) = expected_len else {
            return self.failed(
                "capture_byte_length_overflow",
                "capture dimensions exceed the renderer byte-length domain",
            );
        };
        if readback.as_bytes().len() != expected_len {
            return self.failed(
                "capture_byte_length_mismatch",
                format!(
                    "capture readback byte length mismatch: expected {expected_len}, got {}",
                    readback.as_bytes().len()
                ),
            );
        }

        let mut bytes_rgba8 = readback.as_bytes().to_vec();
        if self.readback_format.mode == TextureReadbackMode::Bgra8 {
            for pixel in bytes_rgba8.as_chunks_mut::<4>().0 {
                pixel.swap(0, 2);
            }
        }
        RenderCapturedTexture {
            identity: self.identity.clone(),
            width: self.width,
            height: self.height,
            format: format!("{:?}", self.source_format),
            bytes_rgba8: Some(bytes_rgba8),
            terminal: RenderCaptureTerminal::completed(),
        }
    }

    fn failed(
        &self,
        reason_code: impl Into<String>,
        detail: impl Into<String>,
    ) -> RenderCapturedTexture {
        RenderCapturedTexture {
            identity: self.identity.clone(),
            width: self.width,
            height: self.height,
            format: format!("{:?}", self.source_format),
            bytes_rgba8: None,
            terminal: RenderCaptureTerminal::with_reason(
                RenderCaptureTerminalCode::ReadbackFailed,
                reason_code,
                detail,
            ),
        }
    }

    fn result(&self, capture: &RenderCapturedTexture) -> RenderCaptureSelectorResult {
        RenderCaptureSelectorResult {
            selector_index: self.selector_index,
            selector: self.selector.clone(),
            capture_point: self.identity.capture_point.clone(),
            frame_identity: Some(self.identity.clone()),
            terminal: capture.terminal.clone(),
            artifact_path: None,
        }
    }
}

impl RendererGpuObservationState {
    pub fn accept(
        &mut self,
        context: &GpuContext,
        submission: GpuSubmission,
        timings: Vec<GpuPassTimingFrame>,
        captures: Vec<PreparedCaptureReadback>,
        capture_runtime: &mut FrameCaptureRuntime,
    ) -> RendererGpuObservationOutput {
        self.accept_with_bound(
            submission,
            timings,
            captures,
            capture_runtime,
            context.execution_policy().max_in_flight_submissions().get(),
        )
    }

    fn accept_with_bound(
        &mut self,
        submission: GpuSubmission,
        timings: Vec<GpuPassTimingFrame>,
        captures: Vec<PreparedCaptureReadback>,
        capture_runtime: &mut FrameCaptureRuntime,
        bound: usize,
    ) -> RendererGpuObservationOutput {
        let mut output = RendererGpuObservationOutput::default();
        let mut accepted_timings = Vec::with_capacity(timings.len());
        for timing in timings {
            if submission.readback(timing.readback_id()).is_some() {
                accepted_timings.push(timing);
            } else {
                output.timing_evidence.extend(timing.diagnostic_evidence(
                    "accepted GPU submission omitted the renderer timing readback",
                ));
            }
        }

        let mut accepted_captures = Vec::with_capacity(captures.len());
        for prepared in captures {
            let capture = CaptureObservation::from_prepared(&prepared);
            if submission.readback(capture.readback_id).is_some() {
                capture_runtime.set_readback_pending(capture.selector_index);
                accepted_captures.push(capture);
            } else {
                let terminal = capture.failed(
                    "accepted_capture_readback_missing",
                    "accepted GPU submission omitted the renderer capture readback",
                );
                capture_runtime.set_terminal(capture.selector_index, terminal.terminal.clone());
                output.captured_textures.push(terminal);
            }
        }

        if accepted_timings.is_empty() && accepted_captures.is_empty() {
            return output;
        }

        if self.accepted.len() >= bound {
            let detail = format!(
                "renderer GPU observation bound {bound} was exhausted for accepted submission {}",
                submission.id()
            );
            for timing in accepted_timings {
                output
                    .timing_evidence
                    .extend(timing.diagnostic_evidence(detail.clone()));
            }
            for capture in accepted_captures {
                let terminal = capture.failed("renderer_observation_capacity_exceeded", &detail);
                capture_runtime.set_terminal(capture.selector_index, terminal.terminal.clone());
                output.captured_textures.push(terminal);
            }
            return output;
        }

        for timing in &accepted_timings {
            output.timing_evidence.extend(timing.pending_evidence());
        }
        self.accepted.push(AcceptedRendererObservation {
            submission,
            timings: accepted_timings,
            captures: accepted_captures,
        });
        output
    }

    pub fn progress(&mut self, context: &GpuContext) -> RendererGpuObservationOutput {
        self.progress_with_context(context.affinity(), context.execution_lifecycle_state())
    }

    fn progress_with_context(
        &mut self,
        affinity: GpuContextAffinity,
        lifecycle: GpuExecutionLifecycleState,
    ) -> RendererGpuObservationOutput {
        let mut output = RendererGpuObservationOutput::default();

        for accepted in &mut self.accepted {
            let invalid_context = (accepted.submission.affinity() != affinity)
                .then_some("renderer GPU observation belongs to another context/device generation");

            accepted.timings.retain(|timing| {
                let status = accepted
                    .submission
                    .readback(timing.readback_id())
                    .map(|readback| readback.status());
                match status {
                    Some(GpuReadbackStatus::Pending)
                        if invalid_context.is_none()
                            && lifecycle != GpuExecutionLifecycleState::Closed =>
                    {
                        true
                    }
                    Some(GpuReadbackStatus::Ready(bytes)) => {
                        output.timing_evidence.extend(timing.ready_evidence(&bytes));
                        false
                    }
                    Some(GpuReadbackStatus::Failed(failure)) => {
                        output
                            .timing_evidence
                            .extend(timing.failed_evidence(&failure));
                        false
                    }
                    _ => {
                        output.timing_evidence.extend(timing.diagnostic_evidence(
                            invalid_context.unwrap_or(
                                "GPU context closed before timing readback became terminal",
                            ),
                        ));
                        false
                    }
                }
            });

            accepted.captures.retain(|capture| {
                let status = accepted
                    .submission
                    .readback(capture.readback_id)
                    .map(|readback| readback.status());
                let terminal = match status {
                    Some(GpuReadbackStatus::Pending)
                        if invalid_context.is_none()
                            && lifecycle != GpuExecutionLifecycleState::Closed =>
                    {
                        return true;
                    }
                    Some(GpuReadbackStatus::Ready(bytes)) => capture.ready(&bytes),
                    Some(GpuReadbackStatus::Failed(failure)) => {
                        capture.failed_from_submission(&failure)
                    }
                    _ => capture.failed(
                        "capture_context_unavailable",
                        invalid_context.unwrap_or(
                            "GPU context closed before capture readback became terminal",
                        ),
                    ),
                };
                output.capture_results.push(capture.result(&terminal));
                output.captured_textures.push(terminal);
                false
            });
        }
        self.accepted
            .retain(|accepted| !accepted.timings.is_empty() || !accepted.captures.is_empty());
        output
    }
}

impl CaptureObservation {
    fn failed_from_submission(&self, failure: &GpuSubmissionFailure) -> RenderCapturedTexture {
        self.failed(
            "capture_readback_failed",
            format!(
                "GPU capture readback failed ({:?}): {}",
                failure.kind(),
                failure.detail()
            ),
        )
    }
}
