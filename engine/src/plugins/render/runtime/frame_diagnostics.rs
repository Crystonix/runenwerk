use super::debug_eval::{evaluate_pixel_probes, evaluate_texture_diffs};
use crate::plugins::render::inspect::{
    RenderCaptureIdentity, RenderCaptureSelector, RenderCaptureSelectorResult,
    RenderCaptureTerminal, RenderCaptureTerminalCode, RenderCaptureTerminalReason,
    RenderCapturedTexture, RenderDebugFrameReport, RenderPassProvenanceRecord,
    RenderPixelProbeRequest, RenderPixelProbeResult, RenderPixelProbeStatus,
    RenderSelectorResolution, RenderTextureDiffRequest, RenderTextureDiffResult,
    RenderTextureDiffStatus, ResolvedRenderCapturePlan, export_captured_textures,
};
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

const MAX_PENDING_DIAGNOSTIC_TRANSACTIONS: usize = 8;
const MAX_RETAINED_DIAGNOSTIC_CAPTURE_BYTES: usize = 64 * 1024 * 1024;
const CAPTURE_BYTE_CAPACITY_REASON: &str = "diagnostics_capture_byte_capacity_exceeded";

#[derive(Debug, runen_ecs::Component, runen_ecs::Resource)]
pub(crate) struct RenderFrameDiagnosticsTransactionState {
    pending: BTreeMap<u64, RenderFrameDiagnosticsTransaction>,
    max_pending_transactions: usize,
    max_retained_capture_bytes: usize,
}

impl Default for RenderFrameDiagnosticsTransactionState {
    fn default() -> Self {
        Self {
            pending: BTreeMap::new(),
            max_pending_transactions: MAX_PENDING_DIAGNOSTIC_TRANSACTIONS,
            max_retained_capture_bytes: MAX_RETAINED_DIAGNOSTIC_CAPTURE_BYTES,
        }
    }
}

#[derive(Debug)]
pub(crate) struct RenderFrameDiagnosticsSnapshot {
    pub frame_index: u64,
    pub simulation_tick: u64,
    pub provenance: Vec<RenderPassProvenanceRecord>,
    pub capture_plan: ResolvedRenderCapturePlan,
    pub pixel_probes: Vec<RenderPixelProbeRequest>,
    pub texture_diffs: Vec<RenderTextureDiffRequest>,
    pub artifact_output_dir: Option<PathBuf>,
}

#[derive(Debug)]
pub(crate) struct CompletedRenderFrameDiagnostics {
    pub report: RenderDebugFrameReport,
    pub simulation_tick: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RenderFrameDiagnosticsCompletionMode {
    AwaitingNormalCompletion,
    ProductAborted,
}

#[derive(Debug)]
struct RenderFrameDiagnosticsTransaction {
    frame_index: u64,
    simulation_tick: u64,
    provenance: Vec<RenderPassProvenanceRecord>,
    capture_plan: ResolvedRenderCapturePlan,
    capture_results: BTreeMap<usize, RenderCaptureSelectorResult>,
    captures: Vec<RenderCapturedTexture>,
    pixel_probes: Vec<PendingPixelProbe>,
    texture_diffs: Vec<PendingTextureDiff>,
    artifact: PendingArtifactExport,
    payload_capacity_exceeded: BTreeSet<RenderCaptureIdentity>,
    completion_mode: RenderFrameDiagnosticsCompletionMode,
    warnings: Vec<String>,
    errors: Vec<String>,
}

#[derive(Debug)]
struct PendingPixelProbe {
    request: RenderPixelProbeRequest,
    dependencies: ConsumerDependencies,
    result: Option<RenderPixelProbeResult>,
}

#[derive(Debug)]
struct PendingTextureDiff {
    request: RenderTextureDiffRequest,
    dependencies: ConsumerDependencies,
    result: Option<RenderTextureDiffResult>,
}

#[derive(Debug, Default)]
struct ConsumerDependencies {
    identities: Vec<RenderCaptureIdentity>,
    unavailable: bool,
}

#[derive(Debug)]
struct PendingArtifactExport {
    output_dir: Option<PathBuf>,
    dependencies: Vec<RenderCaptureIdentity>,
    terminal: bool,
    manifest_path: Option<PathBuf>,
}

impl RenderFrameDiagnosticsTransactionState {
    pub(crate) fn observe_terminal_captures(
        &mut self,
        captures: Vec<RenderCapturedTexture>,
        results: Vec<RenderCaptureSelectorResult>,
    ) -> Vec<CompletedRenderFrameDiagnostics> {
        for capture in captures {
            let frame_index = capture.identity.frame_index;
            if let Some(transaction) = self.pending.get_mut(&frame_index) {
                transaction.observe_capture(capture, usize::MAX);
            }
        }

        for result in results {
            let Some(frame_index) = result
                .frame_identity
                .as_ref()
                .map(|identity| identity.frame_index)
            else {
                continue;
            };
            if let Some(transaction) = self.pending.get_mut(&frame_index) {
                transaction.observe_capture_result(result);
            }
        }

        for transaction in self.pending.values_mut() {
            transaction.advance();
        }
        let mut remaining_bytes = self.max_retained_capture_bytes;
        for transaction in self.pending.values_mut() {
            transaction.enforce_retained_byte_capacity(remaining_bytes);
            transaction.advance();
            remaining_bytes = remaining_bytes.saturating_sub(transaction.retained_capture_bytes());
        }
        self.take_completed_reports()
    }

    pub(crate) fn begin_frame(
        &mut self,
        snapshot: RenderFrameDiagnosticsSnapshot,
        captures: Vec<RenderCapturedTexture>,
        results: Vec<RenderCaptureSelectorResult>,
    ) -> Vec<CompletedRenderFrameDiagnostics> {
        let frame_index = snapshot.frame_index;
        let mut transaction = RenderFrameDiagnosticsTransaction::new(snapshot);
        for result in results {
            transaction.observe_capture_result(result);
        }
        for capture in captures {
            transaction.observe_capture(capture, usize::MAX);
        }
        transaction.advance();

        if transaction.is_complete() {
            return vec![transaction.into_completion()];
        }

        if self.pending.contains_key(&frame_index) {
            transaction.fail_capacity(
                "diagnostics_frame_already_pending",
                format!(
                    "a product diagnostics transaction for semantic frame {frame_index} already exists"
                ),
            );
            return vec![transaction.into_completion()];
        }

        if self.pending.len() >= self.max_pending_transactions {
            transaction.fail_capacity(
                "diagnostics_transaction_capacity_exceeded",
                format!(
                    "product diagnostics transaction bound {} was exhausted for semantic frame {frame_index}",
                    self.max_pending_transactions
                ),
            );
            return vec![transaction.into_completion()];
        }

        let available_bytes = self
            .max_retained_capture_bytes
            .saturating_sub(self.retained_capture_bytes());
        transaction.enforce_retained_byte_capacity(available_bytes);
        transaction.advance();
        if transaction.is_complete() {
            vec![transaction.into_completion()]
        } else {
            self.pending.insert(frame_index, transaction);
            Vec::new()
        }
    }

    fn retained_capture_bytes(&self) -> usize {
        self.pending
            .values()
            .map(RenderFrameDiagnosticsTransaction::retained_capture_bytes)
            .fold(0usize, usize::saturating_add)
    }

    fn take_completed_reports(&mut self) -> Vec<CompletedRenderFrameDiagnostics> {
        let completed = self
            .pending
            .iter()
            .filter_map(|(frame_index, transaction)| {
                transaction.is_complete().then_some(*frame_index)
            })
            .collect::<Vec<_>>();
        completed
            .into_iter()
            .filter_map(|frame_index| self.pending.remove(&frame_index))
            .map(RenderFrameDiagnosticsTransaction::into_completion)
            .collect()
    }

    #[cfg(test)]
    fn with_limits(max_pending_transactions: usize, max_retained_capture_bytes: usize) -> Self {
        Self {
            pending: BTreeMap::new(),
            max_pending_transactions,
            max_retained_capture_bytes,
        }
    }

    #[cfg(test)]
    fn pending_count(&self) -> usize {
        self.pending.len()
    }
}

impl RenderFrameDiagnosticsTransaction {
    fn new(snapshot: RenderFrameDiagnosticsSnapshot) -> Self {
        let pixel_probes = snapshot
            .pixel_probes
            .into_iter()
            .map(|request| PendingPixelProbe {
                dependencies: resolve_probe_dependencies(&snapshot.capture_plan, &request),
                request,
                result: None,
            })
            .collect();
        let texture_diffs = snapshot
            .texture_diffs
            .into_iter()
            .map(|request| PendingTextureDiff {
                dependencies: resolve_diff_dependencies(&snapshot.capture_plan, &request),
                request,
                result: None,
            })
            .collect();
        let artifact_dependencies = snapshot
            .capture_plan
            .selectors
            .iter()
            .filter_map(|selector| resolution_identity(&selector.resolution).cloned())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        let artifact_terminal =
            snapshot.artifact_output_dir.is_none() || artifact_dependencies.is_empty();

        Self {
            frame_index: snapshot.frame_index,
            simulation_tick: snapshot.simulation_tick,
            provenance: snapshot.provenance,
            capture_plan: snapshot.capture_plan,
            capture_results: BTreeMap::new(),
            captures: Vec::new(),
            pixel_probes,
            texture_diffs,
            artifact: PendingArtifactExport {
                output_dir: snapshot.artifact_output_dir,
                dependencies: artifact_dependencies,
                terminal: artifact_terminal,
                manifest_path: None,
            },
            payload_capacity_exceeded: BTreeSet::new(),
            completion_mode: RenderFrameDiagnosticsCompletionMode::AwaitingNormalCompletion,
            warnings: Vec::new(),
            errors: Vec::new(),
        }
    }

    fn observe_capture(&mut self, mut capture: RenderCapturedTexture, available_bytes: usize) {
        if capture.identity.frame_index != self.frame_index {
            return;
        }
        if self
            .captures
            .iter()
            .any(|existing| existing.identity == capture.identity)
        {
            self.errors.push(format!(
                "duplicate terminal capture for semantic frame {} identity {:?}",
                self.frame_index, capture.identity
            ));
            return;
        }

        let payload_needed = self.capture_payload_is_needed(&capture.identity);
        let byte_len = capture.bytes_rgba8.as_ref().map_or(0usize, Vec::len);
        if !payload_needed {
            capture.bytes_rgba8 = None;
        } else if byte_len > available_bytes {
            capture.bytes_rgba8 = None;
            self.payload_capacity_exceeded
                .insert(capture.identity.clone());
            self.errors.push(format!(
                "{CAPTURE_BYTE_CAPACITY_REASON}: semantic frame {} capture {:?} required {} retained bytes with {} available",
                self.frame_index, capture.identity, byte_len, available_bytes,
            ));
        }
        self.captures.push(capture);
    }

    fn observe_capture_result(&mut self, result: RenderCaptureSelectorResult) {
        let result_frame = result
            .frame_identity
            .as_ref()
            .map(|identity| identity.frame_index)
            .unwrap_or(self.frame_index);
        if result_frame != self.frame_index {
            return;
        }
        if self.capture_results.contains_key(&result.selector_index) {
            self.errors.push(format!(
                "duplicate terminal selector result for semantic frame {} selector {}",
                self.frame_index, result.selector_index
            ));
            return;
        }
        self.capture_results.insert(result.selector_index, result);
    }

    fn advance(&mut self) {
        let capture_plan = &self.capture_plan;
        let captures = &self.captures;
        let payload_capacity_exceeded = &self.payload_capacity_exceeded;
        for probe in &mut self.pixel_probes {
            if probe.result.is_some() || !probe.dependencies.is_terminal(captures) {
                continue;
            }
            probe.result = if let Some(detail) = probe
                .dependencies
                .byte_capacity_failure_detail(payload_capacity_exceeded)
            {
                Some(capacity_probe_result(
                    capture_plan,
                    &probe.request,
                    CAPTURE_BYTE_CAPACITY_REASON,
                    &detail,
                ))
            } else {
                evaluate_pixel_probes(std::slice::from_ref(&probe.request), captures).pop()
            };
        }

        for diff in &mut self.texture_diffs {
            if diff.result.is_some() || !diff.dependencies.is_terminal(captures) {
                continue;
            }
            diff.result = if let Some(detail) = diff
                .dependencies
                .byte_capacity_failure_detail(payload_capacity_exceeded)
            {
                Some(capacity_diff_result(
                    capture_plan,
                    &diff.request,
                    CAPTURE_BYTE_CAPACITY_REASON,
                    &detail,
                ))
            } else {
                evaluate_texture_diffs(std::slice::from_ref(&diff.request), captures).pop()
            };
        }

        if !self.artifact.terminal
            && self
                .artifact
                .dependencies
                .iter()
                .all(|identity| self.has_terminal_capture(identity))
        {
            let capacity_blocked = self
                .artifact
                .dependencies
                .iter()
                .any(|identity| self.payload_capacity_exceeded.contains(identity));
            if capacity_blocked {
                self.artifact.manifest_path = None;
                self.errors.push(format!(
                    "{CAPTURE_BYTE_CAPACITY_REASON}: semantic frame {} artifact export requires capture bytes that product diagnostics could not retain",
                    self.frame_index
                ));
            } else {
                let output_dir = self
                    .artifact
                    .output_dir
                    .as_ref()
                    .expect("pending artifact export owns its output directory");
                match export_captured_textures(output_dir, self.frame_index, &self.captures) {
                    Ok(export) => {
                        self.artifact.manifest_path = Some(export.manifest_path);
                        for exported in export.exported_capture_images {
                            for result in self.capture_results.values_mut() {
                                if result.frame_identity.as_ref() == Some(&exported.frame_identity)
                                    && result.terminal.code == RenderCaptureTerminalCode::Completed
                                {
                                    result.artifact_path = Some(exported.image_path.clone());
                                }
                            }
                        }
                    }
                    Err(err) => {
                        let reason = RenderCaptureTerminalReason::new(
                            "artifact_export_failed",
                            err.to_string(),
                        );
                        for result in self.capture_results.values_mut() {
                            if result.terminal.code == RenderCaptureTerminalCode::Completed {
                                result.terminal = RenderCaptureTerminal::new(
                                    RenderCaptureTerminalCode::ExportFailed,
                                    Some(reason.clone()),
                                );
                            }
                        }
                        self.errors.push(format!("artifact_export_failed: {err}"));
                    }
                }
            }
            self.artifact.terminal = true;
        }

        self.release_unneeded_captures();
    }

    fn release_unneeded_captures(&mut self) {
        let mut needed = BTreeSet::<RenderCaptureIdentity>::new();
        for probe in &self.pixel_probes {
            if probe.result.is_none() {
                needed.extend(probe.dependencies.identities.iter().cloned());
            }
        }
        for diff in &self.texture_diffs {
            if diff.result.is_none() {
                needed.extend(diff.dependencies.identities.iter().cloned());
            }
        }
        if !self.artifact.terminal {
            needed.extend(self.artifact.dependencies.iter().cloned());
        }
        self.captures
            .retain(|capture| needed.contains(&capture.identity));
    }

    fn enforce_retained_byte_capacity(&mut self, available_bytes: usize) {
        let retained = self.retained_capture_bytes();
        if retained <= available_bytes {
            return;
        }
        let mut remaining = available_bytes;
        for capture in &mut self.captures {
            let byte_len = capture.bytes_rgba8.as_ref().map_or(0usize, Vec::len);
            if byte_len <= remaining {
                remaining -= byte_len;
                continue;
            }
            capture.bytes_rgba8 = None;
            self.payload_capacity_exceeded
                .insert(capture.identity.clone());
            self.errors.push(format!(
                "{CAPTURE_BYTE_CAPACITY_REASON}: semantic frame {} capture {:?} could not retain {} bytes",
                self.frame_index, capture.identity, byte_len,
            ));
        }
    }

    fn fail_capacity(&mut self, reason_code: &str, detail: String) {
        self.errors.push(format!("{reason_code}: {detail}"));
        for probe in &mut self.pixel_probes {
            if probe.result.is_none() {
                probe.result = Some(capacity_probe_result(
                    &self.capture_plan,
                    &probe.request,
                    reason_code,
                    &detail,
                ));
            }
        }
        for diff in &mut self.texture_diffs {
            if diff.result.is_none() {
                diff.result = Some(capacity_diff_result(
                    &self.capture_plan,
                    &diff.request,
                    reason_code,
                    &detail,
                ));
            }
        }
        self.artifact.terminal = true;
        self.artifact.manifest_path = None;
        self.captures.clear();
        self.payload_capacity_exceeded.clear();
        self.completion_mode = RenderFrameDiagnosticsCompletionMode::ProductAborted;
    }

    fn has_terminal_capture(&self, identity: &RenderCaptureIdentity) -> bool {
        self.captures
            .iter()
            .any(|capture| &capture.identity == identity)
    }

    fn capture_payload_is_needed(&self, identity: &RenderCaptureIdentity) -> bool {
        self.pixel_probes
            .iter()
            .any(|probe| probe.result.is_none() && probe.dependencies.identities.contains(identity))
            || self.texture_diffs.iter().any(|diff| {
                diff.result.is_none() && diff.dependencies.identities.contains(identity)
            })
            || (!self.artifact.terminal && self.artifact.dependencies.contains(identity))
    }

    fn retained_capture_bytes(&self) -> usize {
        self.captures
            .iter()
            .filter_map(|capture| capture.bytes_rgba8.as_ref())
            .map(Vec::len)
            .fold(0usize, usize::saturating_add)
    }

    fn capture_results_complete(&self) -> bool {
        self.capture_plan
            .selectors
            .iter()
            .all(|selector| self.capture_results.contains_key(&selector.selector_index))
    }

    fn is_complete(&self) -> bool {
        let product_consumers_terminal =
            self.pixel_probes.iter().all(|probe| probe.result.is_some())
                && self.texture_diffs.iter().all(|diff| diff.result.is_some())
                && self.artifact.terminal;
        product_consumers_terminal
            && (self.completion_mode == RenderFrameDiagnosticsCompletionMode::ProductAborted
                || self.capture_results_complete())
    }

    fn into_completion(self) -> CompletedRenderFrameDiagnostics {
        debug_assert!(self.is_complete());
        CompletedRenderFrameDiagnostics {
            simulation_tick: self.simulation_tick,
            report: RenderDebugFrameReport {
                frame_index: self.frame_index,
                provenance: self.provenance,
                capture_plan: self.capture_plan,
                capture_results: self.capture_results.into_values().collect(),
                artifact_manifest_path: self.artifact.manifest_path,
                pixel_probe_results: self
                    .pixel_probes
                    .into_iter()
                    .filter_map(|probe| probe.result)
                    .collect(),
                texture_diff_results: self
                    .texture_diffs
                    .into_iter()
                    .filter_map(|diff| diff.result)
                    .collect(),
                warnings: self.warnings,
                errors: self.errors,
            },
        }
    }
}

impl ConsumerDependencies {
    fn is_terminal(&self, captures: &[RenderCapturedTexture]) -> bool {
        self.unavailable
            || self
                .identities
                .iter()
                .all(|identity| captures.iter().any(|capture| &capture.identity == identity))
    }

    fn byte_capacity_failure_detail(
        &self,
        capacity_exceeded: &BTreeSet<RenderCaptureIdentity>,
    ) -> Option<String> {
        let affected = self
            .identities
            .iter()
            .filter(|identity| capacity_exceeded.contains(*identity))
            .collect::<Vec<_>>();
        (!affected.is_empty()).then(|| {
            format!(
                "product diagnostics could not retain capture bytes for dependencies {affected:?}"
            )
        })
    }
}

fn resolve_probe_dependencies(
    plan: &ResolvedRenderCapturePlan,
    request: &RenderPixelProbeRequest,
) -> ConsumerDependencies {
    let mut selectors = vec![&request.selector];
    if let crate::plugins::render::inspect::RenderPixelProbeAssertionMode::CompareToCapture {
        other_selector,
        ..
    } = &request.assertion
    {
        selectors.push(other_selector);
    }
    resolve_consumer_dependencies(plan, selectors)
}

fn resolve_diff_dependencies(
    plan: &ResolvedRenderCapturePlan,
    request: &RenderTextureDiffRequest,
) -> ConsumerDependencies {
    resolve_consumer_dependencies(plan, [&request.left_selector, &request.right_selector])
}

fn resolve_consumer_dependencies<'a>(
    plan: &ResolvedRenderCapturePlan,
    selectors: impl IntoIterator<Item = &'a RenderCaptureSelector>,
) -> ConsumerDependencies {
    let mut identities = BTreeSet::<RenderCaptureIdentity>::new();
    let mut unavailable = false;
    for selector in selectors {
        let resolutions = plan
            .selectors
            .iter()
            .filter(|planned| &planned.selector == selector)
            .filter_map(|planned| resolution_identity(&planned.resolution))
            .cloned()
            .collect::<BTreeSet<_>>();
        if resolutions.len() == 1 {
            identities.extend(resolutions);
        } else {
            unavailable = true;
        }
    }
    ConsumerDependencies {
        identities: identities.into_iter().collect(),
        unavailable,
    }
}

fn resolution_identity(resolution: &RenderSelectorResolution) -> Option<&RenderCaptureIdentity> {
    match resolution {
        RenderSelectorResolution::Pending { frame_identity, .. }
        | RenderSelectorResolution::Matched { frame_identity, .. } => Some(frame_identity),
        RenderSelectorResolution::Unmatched { .. }
        | RenderSelectorResolution::Disabled { .. }
        | RenderSelectorResolution::Unsupported { .. }
        | RenderSelectorResolution::Skipped { .. } => None,
    }
}

fn capacity_probe_result(
    plan: &ResolvedRenderCapturePlan,
    request: &RenderPixelProbeRequest,
    reason_code: &str,
    detail: &str,
) -> RenderPixelProbeResult {
    RenderPixelProbeResult {
        probe_id: request.id.clone(),
        capture_point_identity: request.selector.stable_point_fallback(),
        frame_identity: selector_identity(plan, &request.selector).cloned(),
        sample_mode: request.sample_mode,
        resolved_coordinate: None,
        comparison_mode: request.assertion.clone(),
        sampled_rgba8: None,
        compared_rgba8: None,
        status: RenderPixelProbeStatus::Skipped,
        message: Some(RenderCaptureTerminalReason::new(reason_code, detail)),
    }
}

fn capacity_diff_result(
    plan: &ResolvedRenderCapturePlan,
    request: &RenderTextureDiffRequest,
    reason_code: &str,
    detail: &str,
) -> RenderTextureDiffResult {
    RenderTextureDiffResult {
        diff_id: request.id.clone(),
        request: request.clone(),
        left_capture_point: request.left_selector.stable_point_fallback(),
        right_capture_point: request.right_selector.stable_point_fallback(),
        left_frame_identity: selector_identity(plan, &request.left_selector).cloned(),
        right_frame_identity: selector_identity(plan, &request.right_selector).cloned(),
        status: RenderTextureDiffStatus::Skipped,
        metrics: None,
        mismatch_samples: Vec::new(),
        diff_image_path: None,
        message: Some(RenderCaptureTerminalReason::new(reason_code, detail)),
    }
}

fn selector_identity<'a>(
    plan: &'a ResolvedRenderCapturePlan,
    selector: &RenderCaptureSelector,
) -> Option<&'a RenderCaptureIdentity> {
    let mut identities = plan
        .selectors
        .iter()
        .filter(|planned| &planned.selector == selector)
        .filter_map(|planned| resolution_identity(&planned.resolution));
    let first = identities.next()?;
    identities
        .all(|identity| identity == first)
        .then_some(first)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugins::render::inspect::{
        CaptureStage, CaptureTextureClass, RenderPixelProbeAssertionMode, RenderPixelSampleMode,
        RenderSelectorResolution, ResolvedRenderCaptureSelector,
        validate_selector_terminal_invariant,
    };

    fn selector(pass_id: &str) -> RenderCaptureSelector {
        RenderCaptureSelector {
            flow_id: Some("flow.main".to_string()),
            pass_id: Some(pass_id.to_string()),
            stage: CaptureStage::After,
            resource_id: "surface.color".to_string(),
            texture_class: CaptureTextureClass::ImportedTexture,
        }
    }

    fn identity(frame_index: u64, selector: &RenderCaptureSelector) -> RenderCaptureIdentity {
        RenderCaptureIdentity {
            frame_index,
            pass_label: selector
                .pass_id
                .clone()
                .unwrap_or_else(|| "pass".to_string()),
            capture_point: selector.stable_point_fallback(),
        }
    }

    fn pending_plan(
        frame_index: u64,
        selectors: &[RenderCaptureSelector],
    ) -> ResolvedRenderCapturePlan {
        ResolvedRenderCapturePlan {
            frame_index,
            selectors: selectors
                .iter()
                .cloned()
                .enumerate()
                .map(|(selector_index, selector)| ResolvedRenderCaptureSelector {
                    selector_index,
                    resolution: RenderSelectorResolution::Pending {
                        capture_point: selector.stable_point_fallback(),
                        frame_identity: identity(frame_index, &selector),
                    },
                    selector,
                })
                .collect(),
        }
    }

    fn unmatched_plan(
        frame_index: u64,
        selector: RenderCaptureSelector,
    ) -> ResolvedRenderCapturePlan {
        ResolvedRenderCapturePlan {
            frame_index,
            selectors: vec![ResolvedRenderCaptureSelector {
                selector_index: 0,
                selector,
                resolution: RenderSelectorResolution::Unmatched {
                    reason: RenderCaptureTerminalReason::new(
                        "selector_unmatched",
                        "selector matched no capture point in this frame",
                    ),
                },
            }],
        }
    }

    fn snapshot(
        frame_index: u64,
        capture_plan: ResolvedRenderCapturePlan,
        pixel_probes: Vec<RenderPixelProbeRequest>,
        texture_diffs: Vec<RenderTextureDiffRequest>,
    ) -> RenderFrameDiagnosticsSnapshot {
        RenderFrameDiagnosticsSnapshot {
            frame_index,
            simulation_tick: frame_index.saturating_mul(10),
            provenance: Vec::new(),
            capture_plan,
            pixel_probes,
            texture_diffs,
            artifact_output_dir: None,
        }
    }

    fn capture(
        frame_index: u64,
        selector: &RenderCaptureSelector,
        pixels: [u8; 4],
    ) -> RenderCapturedTexture {
        RenderCapturedTexture {
            identity: identity(frame_index, selector),
            width: 1,
            height: 1,
            format: "Rgba8Unorm".to_string(),
            bytes_rgba8: Some(pixels.to_vec()),
            terminal: RenderCaptureTerminal::completed(),
        }
    }

    fn failed_capture(frame_index: u64, selector: &RenderCaptureSelector) -> RenderCapturedTexture {
        RenderCapturedTexture {
            identity: identity(frame_index, selector),
            width: 1,
            height: 1,
            format: "Rgba8Unorm".to_string(),
            bytes_rgba8: None,
            terminal: RenderCaptureTerminal::with_reason(
                RenderCaptureTerminalCode::ReadbackFailed,
                "capture_readback_failed",
                "test readback failed",
            ),
        }
    }

    fn result_for(
        capture: &RenderCapturedTexture,
        selector_index: usize,
    ) -> RenderCaptureSelectorResult {
        RenderCaptureSelectorResult {
            selector_index,
            selector: RenderCaptureSelector {
                flow_id: Some(capture.identity.flow_id().to_string()),
                pass_id: Some(capture.identity.pass_id().to_string()),
                stage: capture.identity.stage(),
                resource_id: capture.identity.resource_id().to_string(),
                texture_class: capture.identity.texture_class(),
            },
            capture_point: capture.identity.capture_point.clone(),
            frame_identity: Some(capture.identity.clone()),
            terminal: capture.terminal.clone(),
            artifact_path: None,
        }
    }

    fn unmatched_result(
        frame_index: u64,
        selector: RenderCaptureSelector,
    ) -> RenderCaptureSelectorResult {
        let _ = frame_index;
        RenderCaptureSelectorResult {
            selector_index: 0,
            capture_point: selector.stable_point_fallback(),
            selector,
            frame_identity: None,
            terminal: RenderCaptureTerminal::with_reason(
                RenderCaptureTerminalCode::Unmatched,
                "selector_unmatched",
                "selector matched no capture point in this frame",
            ),
            artifact_path: None,
        }
    }

    #[test]
    fn delayed_probe_uses_snapshotted_request_and_original_identity() {
        let selector = selector("pass.probe");
        let probe = RenderPixelProbeRequest {
            id: "frame-n-probe".to_string(),
            selector: selector.clone(),
            sample_mode: RenderPixelSampleMode::Center,
            assertion: RenderPixelProbeAssertionMode::Exact([1, 2, 3, 255]),
        };
        let mut mutable_config_probe = probe.clone();
        let mut state = RenderFrameDiagnosticsTransactionState::default();
        assert!(
            state
                .begin_frame(
                    snapshot(
                        7,
                        pending_plan(7, std::slice::from_ref(&selector)),
                        vec![probe],
                        vec![]
                    ),
                    vec![],
                    vec![],
                )
                .is_empty()
        );

        mutable_config_probe.id = "later-frame-probe".to_string();
        mutable_config_probe.assertion = RenderPixelProbeAssertionMode::Exact([9, 9, 9, 255]);
        let ready = capture(7, &selector, [1, 2, 3, 255]);
        let reports =
            state.observe_terminal_captures(vec![ready.clone()], vec![result_for(&ready, 0)]);

        assert_eq!(reports.len(), 1);
        assert_eq!(reports[0].report.frame_index, 7);
        assert_eq!(
            reports[0].report.pixel_probe_results[0].probe_id,
            "frame-n-probe"
        );
        assert_eq!(
            reports[0].report.pixel_probe_results[0].frame_identity,
            Some(identity(7, &selector))
        );
        assert_eq!(
            reports[0].report.pixel_probe_results[0].status,
            RenderPixelProbeStatus::Passed
        );
    }

    #[test]
    fn delayed_diff_uses_snapshotted_request_after_config_mutates() {
        let left = selector("pass.left");
        let right = selector("pass.right");
        let original = RenderTextureDiffRequest::new("frame-n-diff", left.clone(), right.clone())
            .with_thresholds(8, 1_000_000);
        let mut mutable_config_diff = original.clone();
        let mut state = RenderFrameDiagnosticsTransactionState::default();
        state.begin_frame(
            snapshot(
                8,
                pending_plan(8, &[left.clone(), right.clone()]),
                vec![],
                vec![original],
            ),
            vec![],
            vec![],
        );
        mutable_config_diff.id = "later-frame-diff".to_string();
        mutable_config_diff.max_channel_delta = Some(0);

        let left_capture = capture(8, &left, [1, 2, 3, 255]);
        let right_capture = capture(8, &right, [1, 2, 9, 255]);
        let reports = state.observe_terminal_captures(
            vec![left_capture.clone(), right_capture.clone()],
            vec![result_for(&left_capture, 0), result_for(&right_capture, 1)],
        );
        assert_eq!(
            reports[0].report.texture_diff_results[0].diff_id,
            "frame-n-diff"
        );
        assert_eq!(
            reports[0].report.texture_diff_results[0].status,
            RenderTextureDiffStatus::Compared
        );
        assert_eq!(
            reports[0].report.texture_diff_results[0].left_frame_identity,
            Some(identity(8, &left))
        );
    }

    #[test]
    fn same_selector_on_two_frames_cannot_cross_pair() {
        let selector = selector("pass.same");
        let probe = |id: &str| RenderPixelProbeRequest::center(id, selector.clone());
        let mut state = RenderFrameDiagnosticsTransactionState::default();
        state.begin_frame(
            snapshot(
                10,
                pending_plan(10, std::slice::from_ref(&selector)),
                vec![probe("n")],
                vec![],
            ),
            vec![],
            vec![],
        );
        state.begin_frame(
            snapshot(
                11,
                pending_plan(11, std::slice::from_ref(&selector)),
                vec![probe("n1")],
                vec![],
            ),
            vec![],
            vec![],
        );

        let n1 = capture(11, &selector, [11, 0, 0, 255]);
        let first = state.observe_terminal_captures(vec![n1.clone()], vec![result_for(&n1, 0)]);
        assert_eq!(first.len(), 1);
        assert_eq!(first[0].report.frame_index, 11);
        assert_eq!(
            first[0].report.pixel_probe_results[0].sampled_rgba8,
            Some([11, 0, 0, 255])
        );
        assert_eq!(state.pending_count(), 1);

        let n = capture(10, &selector, [10, 0, 0, 255]);
        let second = state.observe_terminal_captures(vec![n.clone()], vec![result_for(&n, 0)]);
        assert_eq!(second[0].report.frame_index, 10);
        assert_eq!(
            second[0].report.pixel_probe_results[0].sampled_rgba8,
            Some([10, 0, 0, 255])
        );
    }

    #[test]
    fn one_capture_probe_finishes_without_unrelated_capture_and_releases_bytes() {
        let probe_selector = selector("pass.probe");
        let unrelated = selector("pass.unrelated");
        let probe = RenderPixelProbeRequest::center("probe", probe_selector.clone());
        let ready = capture(12, &probe_selector, [1, 2, 3, 255]);
        let mut state = RenderFrameDiagnosticsTransactionState::default();
        assert!(
            state
                .begin_frame(
                    snapshot(
                        12,
                        pending_plan(12, &[probe_selector.clone(), unrelated]),
                        vec![probe],
                        vec![],
                    ),
                    vec![ready.clone()],
                    vec![result_for(&ready, 0)],
                )
                .is_empty()
        );
        let pending = state
            .pending
            .get(&12)
            .expect("frame report should await the unrelated selector");
        assert!(pending.pixel_probes[0].result.is_some());
        assert_eq!(pending.retained_capture_bytes(), 0);
        assert!(pending.captures.is_empty());
    }

    #[test]
    fn diff_waits_for_exactly_two_dependencies_not_unrelated_third() {
        let left = selector("pass.left");
        let right = selector("pass.right");
        let unrelated = selector("pass.unrelated");
        let diff = RenderTextureDiffRequest::new("diff", left.clone(), right.clone());
        let left_capture = capture(13, &left, [1, 2, 3, 255]);
        let mut state = RenderFrameDiagnosticsTransactionState::default();
        state.begin_frame(
            snapshot(
                13,
                pending_plan(13, &[left.clone(), right.clone(), unrelated]),
                vec![],
                vec![diff],
            ),
            vec![left_capture.clone()],
            vec![result_for(&left_capture, 0)],
        );
        assert!(
            state.pending.get(&13).unwrap().texture_diffs[0]
                .result
                .is_none()
        );

        let right_capture = capture(13, &right, [1, 2, 3, 255]);
        assert!(
            state
                .observe_terminal_captures(
                    vec![right_capture.clone()],
                    vec![result_for(&right_capture, 1)],
                )
                .is_empty()
        );
        let pending = state.pending.get(&13).unwrap();
        assert!(pending.texture_diffs[0].result.is_some());
        assert!(pending.captures.is_empty());
    }

    #[test]
    fn failed_dependency_terminalizes_probe_and_diff_truthfully() {
        let failed_selector = selector("pass.failed");
        let ready_selector = selector("pass.ready");
        let mut state = RenderFrameDiagnosticsTransactionState::default();
        state.begin_frame(
            snapshot(
                14,
                pending_plan(14, &[failed_selector.clone(), ready_selector.clone()]),
                vec![RenderPixelProbeRequest::center(
                    "probe",
                    failed_selector.clone(),
                )],
                vec![RenderTextureDiffRequest::new(
                    "diff",
                    failed_selector.clone(),
                    ready_selector.clone(),
                )],
            ),
            vec![],
            vec![],
        );
        let failed = failed_capture(14, &failed_selector);
        let ready = capture(14, &ready_selector, [1, 2, 3, 255]);
        let reports = state.observe_terminal_captures(
            vec![failed.clone(), ready.clone()],
            vec![result_for(&failed, 0), result_for(&ready, 1)],
        );
        let probe = &reports[0].report.pixel_probe_results[0];
        assert_eq!(probe.status, RenderPixelProbeStatus::Skipped);
        assert_eq!(probe.frame_identity, Some(identity(14, &failed_selector)));
        assert_eq!(
            probe.message.as_ref().unwrap().code,
            "capture_not_completed"
        );
        let diff = &reports[0].report.texture_diff_results[0];
        assert_eq!(diff.status, RenderTextureDiffStatus::Skipped);
        assert_eq!(
            diff.left_frame_identity,
            Some(identity(14, &failed_selector))
        );
        assert_eq!(
            diff.right_frame_identity,
            Some(identity(14, &ready_selector))
        );
        assert_eq!(
            diff.message.as_ref().unwrap().code,
            "diff_capture_not_completed"
        );
    }

    #[test]
    fn unmatched_original_selector_is_immediate_and_never_waits_for_future_frame() {
        let selector = selector("pass.unmatched");
        let mut state = RenderFrameDiagnosticsTransactionState::default();
        let reports = state.begin_frame(
            snapshot(
                15,
                unmatched_plan(15, selector.clone()),
                vec![RenderPixelProbeRequest::center("probe", selector.clone())],
                vec![],
            ),
            vec![],
            vec![unmatched_result(15, selector)],
        );
        assert_eq!(reports.len(), 1);
        assert_eq!(
            reports[0].report.pixel_probe_results[0].status,
            RenderPixelProbeStatus::Skipped
        );
        assert_eq!(state.pending_count(), 0);
    }

    #[test]
    fn multiple_frames_can_be_pending_and_terminal_results_emit_once() {
        let a = selector("pass.a");
        let b = selector("pass.b");
        let mut state = RenderFrameDiagnosticsTransactionState::default();
        state.begin_frame(
            snapshot(
                16,
                pending_plan(16, std::slice::from_ref(&a)),
                vec![],
                vec![],
            ),
            vec![],
            vec![],
        );
        state.begin_frame(
            snapshot(
                17,
                pending_plan(17, std::slice::from_ref(&b)),
                vec![],
                vec![],
            ),
            vec![],
            vec![],
        );
        assert_eq!(state.pending_count(), 2);

        let ready = capture(16, &a, [1, 1, 1, 255]);
        assert_eq!(
            state
                .observe_terminal_captures(vec![ready.clone()], vec![result_for(&ready, 0)])
                .len(),
            1
        );
        assert!(
            state
                .observe_terminal_captures(vec![ready.clone()], vec![result_for(&ready, 0)])
                .is_empty()
        );
        assert_eq!(state.pending_count(), 1);
    }

    #[test]
    fn immediate_ready_and_lightweight_paths_retain_no_transaction() {
        let selector = selector("pass.ready");
        let ready = capture(18, &selector, [1, 2, 3, 255]);
        let mut state = RenderFrameDiagnosticsTransactionState::default();
        let reports = state.begin_frame(
            snapshot(
                18,
                pending_plan(18, std::slice::from_ref(&selector)),
                vec![RenderPixelProbeRequest::center("probe", selector.clone())],
                vec![],
            ),
            vec![ready.clone()],
            vec![result_for(&ready, 0)],
        );
        assert_eq!(reports.len(), 1);
        assert_eq!(state.pending_count(), 0);

        let lightweight_capture = capture(19, &selector, [9, 9, 9, 255]);
        assert!(
            state
                .observe_terminal_captures(
                    vec![lightweight_capture.clone()],
                    vec![result_for(&lightweight_capture, 0)],
                )
                .is_empty()
        );
        assert_eq!(state.pending_count(), 0);
    }

    #[test]
    fn artifact_and_report_use_original_semantic_frame() {
        let selector = selector("pass.artifact");
        let ready = capture(20, &selector, [4, 5, 6, 255]);
        let output = std::env::temp_dir().join(format!(
            "runenwerk_frame_diagnostics_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let mut frame = snapshot(
            20,
            pending_plan(20, std::slice::from_ref(&selector)),
            vec![],
            vec![],
        );
        frame.artifact_output_dir = Some(output.clone());
        let mut state = RenderFrameDiagnosticsTransactionState::default();
        let reports = state.begin_frame(frame, vec![ready.clone()], vec![result_for(&ready, 0)]);
        assert_eq!(reports[0].report.frame_index, 20);
        let manifest_path = reports[0].report.artifact_manifest_path.as_ref().unwrap();
        assert!(manifest_path.ends_with("frame_20__manifest.json"));
        let manifest = std::fs::read_to_string(manifest_path).unwrap();
        assert!(manifest.contains("\"frame_index\": 20"));
        assert!(
            reports[0].report.capture_results[0]
                .artifact_path
                .as_ref()
                .unwrap()
                .to_string_lossy()
                .contains("frame_20__")
        );
        let _ = std::fs::remove_dir_all(output);
    }

    #[test]
    fn one_publication_cycle_never_mixes_two_original_frames() {
        let selector = selector("pass.mixed");
        let mut state = RenderFrameDiagnosticsTransactionState::default();
        state.begin_frame(
            snapshot(
                21,
                pending_plan(21, std::slice::from_ref(&selector)),
                vec![],
                vec![],
            ),
            vec![],
            vec![],
        );
        state.begin_frame(
            snapshot(
                22,
                pending_plan(22, std::slice::from_ref(&selector)),
                vec![],
                vec![],
            ),
            vec![],
            vec![],
        );
        let first = capture(21, &selector, [21, 0, 0, 255]);
        let second = capture(22, &selector, [22, 0, 0, 255]);
        let reports = state.observe_terminal_captures(
            vec![second.clone(), first.clone()],
            vec![result_for(&second, 0), result_for(&first, 0)],
        );
        assert_eq!(reports.len(), 2);
        assert_eq!(reports[0].report.frame_index, 21);
        assert_eq!(reports[1].report.frame_index, 22);
        for completed in reports {
            assert!(completed.report.capture_results.iter().all(|result| {
                result.frame_identity.as_ref().unwrap().frame_index == completed.report.frame_index
            }));
        }
    }

    #[test]
    fn delayed_report_retains_original_provenance() {
        let selector = selector("pass.provenance");
        let mut frame = snapshot(
            23,
            pending_plan(23, std::slice::from_ref(&selector)),
            vec![],
            vec![],
        );
        frame.provenance.push(RenderPassProvenanceRecord {
            frame_index: 23,
            flow_id: "original.flow".to_string(),
            pass_id: "original.pass".to_string(),
            pass_label: "Original Pass".to_string(),
            pass_kind: crate::plugins::render::pipelines::FlowPassKind::Compute,
            authoring_index: 0,
            feature_id: None,
            shader_id: "shader.original".to_string(),
            shader_revision: 1,
            fallback_used: false,
            pipeline_stats_key: String::new(),
            bind_group_layout_signature_hash: 0,
            material_specialization_fragment_hash: 0,
            view_signature_hash: 0,
            feature_runtime_version: 0,
            color_formats: Vec::new(),
            depth_format: None,
            sample_count: 1,
            primitive_topology: None,
            material_binding: Default::default(),
            render_targets: Vec::new(),
            sampled_textures: Vec::new(),
            storage_textures: Vec::new(),
            depth_targets: Vec::new(),
            capture_points_available: Vec::new(),
        });
        let mut state = RenderFrameDiagnosticsTransactionState::default();
        state.begin_frame(frame, vec![], vec![]);
        let ready = capture(23, &selector, [1, 1, 1, 255]);
        let reports =
            state.observe_terminal_captures(vec![ready.clone()], vec![result_for(&ready, 0)]);
        assert_eq!(reports[0].report.provenance[0].frame_index, 23);
        assert_eq!(reports[0].report.provenance[0].shader_id, "shader.original");
    }

    #[test]
    fn transaction_capacity_aborts_product_without_fabricating_capture_terminal() {
        let occupying_selector = selector("pass.occupying");
        let completed_selector = selector("pass.completed");
        let pending_selector = selector("pass.pending");
        let mut state = RenderFrameDiagnosticsTransactionState::with_limits(1, 2);
        state.begin_frame(
            snapshot(
                24,
                pending_plan(24, std::slice::from_ref(&occupying_selector)),
                vec![],
                vec![],
            ),
            vec![],
            vec![],
        );
        let observed_capture = capture(25, &completed_selector, [1, 2, 3, 255]);
        let aborted = state.begin_frame(
            snapshot(
                25,
                pending_plan(25, &[completed_selector, pending_selector.clone()]),
                vec![RenderPixelProbeRequest::center(
                    "overflow",
                    pending_selector.clone(),
                )],
                vec![],
            ),
            vec![observed_capture.clone()],
            vec![result_for(&observed_capture, 0)],
        );
        assert_eq!(state.pending_count(), 1);
        assert_eq!(aborted.len(), 1);
        let aborted_report = &aborted[0].report;
        assert!(
            aborted_report
                .errors
                .iter()
                .any(|error| error.contains("diagnostics_transaction_capacity_exceeded"))
        );
        assert_eq!(
            aborted_report.pixel_probe_results[0].status,
            RenderPixelProbeStatus::Skipped
        );
        assert!(matches!(
            aborted_report.capture_plan.selectors[1].resolution,
            RenderSelectorResolution::Pending { .. }
        ));
        assert_eq!(aborted_report.capture_results.len(), 1);
        assert_eq!(
            aborted_report.capture_results[0].terminal.code,
            RenderCaptureTerminalCode::Completed
        );
        assert!(
            aborted_report
                .capture_results
                .iter()
                .all(|result| result.selector_index != 1)
        );
        assert!(aborted_report.validate_invariants().is_empty());
        assert_eq!(aborted[0].simulation_tick, 250);

        let later_real_capture = capture(25, &pending_selector, [9, 8, 7, 255]);
        let later_real_result = result_for(&later_real_capture, 1);
        assert_eq!(
            later_real_result.terminal.code,
            RenderCaptureTerminalCode::Completed
        );
        assert!(
            validate_selector_terminal_invariant(
                &aborted_report.capture_plan,
                &[
                    aborted_report.capture_results[0].clone(),
                    later_real_result.clone(),
                ],
            )
            .is_ok()
        );
        assert!(
            state
                .observe_terminal_captures(vec![later_real_capture], vec![later_real_result],)
                .is_empty()
        );
        assert_eq!(state.pending_count(), 1);
    }

    #[test]
    fn byte_capacity_preserves_completed_capture_and_terminalizes_product_consumers() {
        let left = selector("pass.bytes.left");
        let right = selector("pass.bytes.right");
        let probe = RenderPixelProbeRequest {
            id: "capacity-probe".to_string(),
            selector: left.clone(),
            sample_mode: RenderPixelSampleMode::Center,
            assertion: RenderPixelProbeAssertionMode::CompareToCapture {
                other_selector: right.clone(),
                tolerance: 0,
            },
        };
        let diff = RenderTextureDiffRequest::new("capacity-diff", left.clone(), right.clone());
        let output = std::env::temp_dir().join(format!(
            "runenwerk_frame_diagnostics_capacity_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let mut frame = snapshot(
            26,
            pending_plan(26, &[left.clone(), right.clone()]),
            vec![probe],
            vec![diff],
        );
        frame.artifact_output_dir = Some(output.clone());

        let left_capture = capture(26, &left, [1, 2, 3, 255]);
        let mut state = RenderFrameDiagnosticsTransactionState::with_limits(8, 2);
        assert!(
            state
                .begin_frame(
                    frame,
                    vec![left_capture.clone()],
                    vec![result_for(&left_capture, 0)],
                )
                .is_empty()
        );
        let pending = state.pending.get(&26).unwrap();
        assert_eq!(pending.retained_capture_bytes(), 0);
        assert!(
            pending
                .payload_capacity_exceeded
                .contains(&identity(26, &left))
        );
        assert_eq!(
            pending.capture_results[&0].terminal.code,
            RenderCaptureTerminalCode::Completed
        );

        let right_capture = capture(26, &right, [1, 2, 3, 255]);
        let completed = state.observe_terminal_captures(
            vec![right_capture.clone()],
            vec![result_for(&right_capture, 1)],
        );
        assert_eq!(completed.len(), 1);
        assert_eq!(state.pending_count(), 0);
        let report = &completed[0].report;
        assert!(report.validate_invariants().is_empty());
        assert!(
            report
                .capture_results
                .iter()
                .all(|result| { result.terminal.code == RenderCaptureTerminalCode::Completed })
        );
        assert_eq!(
            report.pixel_probe_results[0].message.as_ref().unwrap().code,
            CAPTURE_BYTE_CAPACITY_REASON
        );
        assert_eq!(
            report.texture_diff_results[0]
                .message
                .as_ref()
                .unwrap()
                .code,
            CAPTURE_BYTE_CAPACITY_REASON
        );
        assert!(report.artifact_manifest_path.is_none());
        assert!(report.errors.iter().any(|error| {
            error.contains(CAPTURE_BYTE_CAPACITY_REASON)
                && error.contains("artifact export requires capture bytes")
        }));
        assert!(!output.exists());
    }

    #[test]
    fn delayed_and_immediate_completions_preserve_original_simulation_tick() {
        let delayed_selector = selector("pass.tick.delayed");
        let mut delayed_snapshot = snapshot(
            27,
            pending_plan(27, std::slice::from_ref(&delayed_selector)),
            vec![],
            vec![],
        );
        delayed_snapshot.simulation_tick = 42;
        let mut state = RenderFrameDiagnosticsTransactionState::default();
        assert!(
            state
                .begin_frame(delayed_snapshot, vec![], vec![])
                .is_empty()
        );

        let delayed_capture = capture(27, &delayed_selector, [1, 2, 3, 255]);
        let delayed = state.observe_terminal_captures(
            vec![delayed_capture.clone()],
            vec![result_for(&delayed_capture, 0)],
        );
        assert_eq!(delayed[0].report.frame_index, 27);
        assert_eq!(delayed[0].simulation_tick, 42);

        let immediate_selector = selector("pass.tick.immediate");
        let immediate_capture = capture(28, &immediate_selector, [4, 5, 6, 255]);
        let mut immediate_snapshot = snapshot(
            28,
            pending_plan(28, std::slice::from_ref(&immediate_selector)),
            vec![],
            vec![],
        );
        immediate_snapshot.simulation_tick = 43;
        let immediate = state.begin_frame(
            immediate_snapshot,
            vec![immediate_capture.clone()],
            vec![result_for(&immediate_capture, 0)],
        );
        assert_eq!(immediate[0].report.frame_index, 28);
        assert_eq!(immediate[0].simulation_tick, 43);
    }
}
