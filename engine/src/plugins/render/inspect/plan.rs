use super::{
    RenderCaptureIdentity, RenderCapturePointIdentity, RenderCaptureSelector,
    RenderCaptureTerminal, RenderCaptureTerminalCode, RenderCaptureTerminalReason,
};
use crate::plugins::render::current_runtime_gpu_capabilities;
use crate::plugins::render::graph::{
    CompiledRenderFlowPlan, RenderExecutionGraphDiagnostic, RenderExecutionGraphPreparedReport,
    RenderPreparedFramePreflightCacheState,
};
use runen_gpu::GpuCapabilities;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenderSelectorResolution {
    Pending {
        capture_point: RenderCapturePointIdentity,
        frame_identity: RenderCaptureIdentity,
    },
    Matched {
        capture_point: RenderCapturePointIdentity,
        frame_identity: RenderCaptureIdentity,
    },
    Unmatched {
        reason: RenderCaptureTerminalReason,
    },
    Disabled {
        reason: RenderCaptureTerminalReason,
    },
    Unsupported {
        reason: RenderCaptureTerminalReason,
    },
    Skipped {
        reason: RenderCaptureTerminalReason,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedRenderCaptureSelector {
    pub selector_index: usize,
    pub selector: RenderCaptureSelector,
    pub resolution: RenderSelectorResolution,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ResolvedRenderCapturePlan {
    pub frame_index: u64,
    pub selectors: Vec<ResolvedRenderCaptureSelector>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderExecutionGraphPlanInspection {
    pub flow_id: String,
    pub flow_label: String,
    pub pass_count: usize,
    pub resource_count: usize,
    pub compiler_diagnostics: Vec<RenderExecutionGraphDiagnosticInspection>,
    pub backend_capabilities: GpuCapabilities,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderExecutionGraphPreflightInspection {
    pub diagnostic_count: usize,
    pub error_count: usize,
    pub cache_mode: Option<String>,
    pub cache_status: Option<String>,
    pub report_source: Option<String>,
    pub diagnostics: Vec<RenderExecutionGraphDiagnosticInspection>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderExecutionGraphDiagnosticInspection {
    pub severity: String,
    pub kind: String,
    pub flow_id: Option<String>,
    pub flow_label: Option<String>,
    pub pass_id: Option<String>,
    pub pass_label: Option<String>,
    pub resource_id: Option<String>,
    pub resource_label: Option<String>,
    pub invocation_id: Option<String>,
    pub view_id: Option<String>,
    pub alias_binding_key: Option<String>,
    pub alias_kind: Option<String>,
    pub dynamic_target_key: Option<String>,
    pub history_signature: Option<String>,
    pub capability: Option<String>,
    pub message: String,
}

pub fn inspect_compiled_render_flow_plan(
    plan: &CompiledRenderFlowPlan,
) -> RenderExecutionGraphPlanInspection {
    RenderExecutionGraphPlanInspection {
        flow_id: plan.flow_id.to_string(),
        flow_label: plan.flow_label.clone(),
        pass_count: plan.render_passes.len(),
        resource_count: plan.resources.resources.len(),
        compiler_diagnostics: plan
            .compiler_diagnostics
            .iter()
            .map(inspect_render_execution_graph_diagnostic)
            .collect(),
        backend_capabilities: current_runtime_gpu_capabilities(),
    }
}

pub fn inspect_render_execution_graph_preflight(
    report: &RenderExecutionGraphPreparedReport,
) -> RenderExecutionGraphPreflightInspection {
    inspect_render_execution_graph_preflight_with_cache(report, None)
}

pub fn inspect_render_execution_graph_preflight_with_cache(
    report: &RenderExecutionGraphPreparedReport,
    cache_state: Option<&RenderPreparedFramePreflightCacheState>,
) -> RenderExecutionGraphPreflightInspection {
    RenderExecutionGraphPreflightInspection {
        diagnostic_count: report.diagnostics.len(),
        error_count: report.error_count(),
        cache_mode: cache_state.map(|state| state.mode.as_str().to_string()),
        cache_status: cache_state.map(|state| state.status.as_str().to_string()),
        report_source: cache_state.map(|state| state.report_source.as_str().to_string()),
        diagnostics: report
            .diagnostics
            .iter()
            .map(inspect_render_execution_graph_diagnostic)
            .collect(),
    }
}

pub fn inspect_render_execution_graph_diagnostic(
    diagnostic: &RenderExecutionGraphDiagnostic,
) -> RenderExecutionGraphDiagnosticInspection {
    RenderExecutionGraphDiagnosticInspection {
        severity: format!("{:?}", diagnostic.severity),
        kind: format!("{:?}", diagnostic.kind),
        flow_id: diagnostic.flow_id.map(|id| id.to_string()),
        flow_label: diagnostic.flow_label.clone(),
        pass_id: diagnostic.pass_id.map(|id| id.to_string()),
        pass_label: diagnostic.pass_label.clone(),
        resource_id: diagnostic.resource_id.map(|id| id.to_string()),
        resource_label: diagnostic.resource_label.clone(),
        invocation_id: diagnostic.invocation_id.as_ref().map(|id| id.0.clone()),
        view_id: diagnostic.view_id.clone(),
        alias_binding_key: diagnostic
            .alias_binding_key
            .as_ref()
            .map(|key| key.as_str().to_string()),
        alias_kind: diagnostic.alias_kind.map(|kind| format!("{:?}", kind)),
        dynamic_target_key: diagnostic
            .dynamic_target_key
            .as_ref()
            .map(ToString::to_string),
        history_signature: diagnostic.history_signature.clone(),
        capability: diagnostic.capability.clone(),
        message: diagnostic.message.clone(),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderCaptureSelectorResult {
    pub selector_index: usize,
    pub selector: RenderCaptureSelector,
    pub capture_point: RenderCapturePointIdentity,
    pub frame_identity: Option<RenderCaptureIdentity>,
    pub terminal: RenderCaptureTerminal,
    pub artifact_path: Option<PathBuf>,
}

impl RenderCaptureSelectorResult {
    pub fn for_unmatched(selector_index: usize, selector: RenderCaptureSelector) -> Self {
        let capture_point = selector.stable_point_fallback();
        Self {
            selector_index,
            selector,
            capture_point,
            frame_identity: None,
            terminal: RenderCaptureTerminal::new(
                RenderCaptureTerminalCode::Unmatched,
                Some(RenderCaptureTerminalReason::new(
                    "selector_unmatched",
                    "selector matched no capture point in this frame",
                )),
            ),
            artifact_path: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderCaptureInvariantViolation {
    pub selector_index: usize,
    pub message: String,
}

pub fn validate_selector_terminal_invariant(
    plan: &ResolvedRenderCapturePlan,
    results: &[RenderCaptureSelectorResult],
) -> Result<(), Vec<RenderCaptureInvariantViolation>> {
    let mut counts = vec![0usize; plan.selectors.len()];
    let mut violations = Vec::<RenderCaptureInvariantViolation>::new();

    for result in results {
        let result_frame = result
            .frame_identity
            .as_ref()
            .map(|identity| identity.frame_index)
            .unwrap_or(plan.frame_index);
        if result_frame != plan.frame_index {
            continue;
        }
        if result.selector_index >= plan.selectors.len() {
            violations.push(RenderCaptureInvariantViolation {
                selector_index: result.selector_index,
                message: format!(
                    "selector result index {} is out of bounds (selectors={})",
                    result.selector_index,
                    plan.selectors.len()
                ),
            });
            continue;
        }
        counts[result.selector_index] = counts[result.selector_index].saturating_add(1);
    }

    for (selector_index, count) in counts.into_iter().enumerate() {
        let selector = &plan.selectors[selector_index];
        let readback_pending = matches!(
            selector.resolution,
            RenderSelectorResolution::Pending { .. }
        );
        if count == 0 && !readback_pending {
            violations.push(RenderCaptureInvariantViolation {
                selector_index,
                message: format!(
                    "selector {} has no terminal outcome (silent drop)",
                    selector.selector.describe()
                ),
            });
        } else if count > 1 {
            violations.push(RenderCaptureInvariantViolation {
                selector_index,
                message: format!(
                    "selector {} has {} terminal outcomes",
                    selector.selector.describe(),
                    count
                ),
            });
        }
    }

    if violations.is_empty() {
        Ok(())
    } else {
        Err(violations)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugins::render::inspect::{
        CaptureStage, CaptureTextureClass, RenderCaptureIdentity, RenderCaptureSelector,
        RenderCaptureTerminal,
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

    fn completed_result(
        selector_index: usize,
        selector: RenderCaptureSelector,
    ) -> RenderCaptureSelectorResult {
        let capture_point = selector.stable_point_fallback();
        RenderCaptureSelectorResult {
            selector_index,
            selector,
            frame_identity: Some(RenderCaptureIdentity {
                frame_index: 7,
                pass_label: "pass.label".to_string(),
                capture_point: capture_point.clone(),
            }),
            capture_point,
            terminal: RenderCaptureTerminal::completed(),
            artifact_path: None,
        }
    }

    #[test]
    fn selector_terminal_invariant_accepts_exactly_one_terminal_per_selector() {
        let selectors = [selector("pass.a"), selector("pass.b")];
        let results = vec![
            completed_result(0, selectors[0].clone()),
            completed_result(1, selectors[1].clone()),
        ];

        let plan = ResolvedRenderCapturePlan {
            frame_index: 7,
            selectors: selectors
                .iter()
                .cloned()
                .enumerate()
                .map(|(selector_index, selector)| ResolvedRenderCaptureSelector {
                    selector_index,
                    resolution: RenderSelectorResolution::Unmatched {
                        reason: RenderCaptureTerminalReason::new("test", "test"),
                    },
                    selector,
                })
                .collect(),
        };
        assert!(validate_selector_terminal_invariant(&plan, &results).is_ok());
    }

    #[test]
    fn selector_terminal_invariant_rejects_missing_and_duplicate_terminals() {
        let selectors = [selector("pass.a"), selector("pass.b")];
        let results = vec![
            completed_result(0, selectors[0].clone()),
            completed_result(0, selectors[0].clone()),
        ];

        let plan = ResolvedRenderCapturePlan {
            frame_index: 7,
            selectors: selectors
                .iter()
                .cloned()
                .enumerate()
                .map(|(selector_index, selector)| ResolvedRenderCaptureSelector {
                    selector_index,
                    resolution: RenderSelectorResolution::Unmatched {
                        reason: RenderCaptureTerminalReason::new("test", "test"),
                    },
                    selector,
                })
                .collect(),
        };
        let violations = validate_selector_terminal_invariant(&plan, &results)
            .expect_err("duplicate + missing selector outcomes should violate invariant");
        assert!(
            violations
                .iter()
                .any(|value| value.message.contains("has 2 terminal outcomes"))
        );
        assert!(
            violations
                .iter()
                .any(|value| value.message.contains("no terminal outcome"))
        );
    }

    #[test]
    fn selector_terminal_invariant_accepts_matched_pending_readback_without_terminal_result() {
        let selector = selector("pass.pending");
        let capture_point = selector.stable_point_fallback();
        let plan = ResolvedRenderCapturePlan {
            frame_index: 9,
            selectors: vec![ResolvedRenderCaptureSelector {
                selector_index: 0,
                selector,
                resolution: RenderSelectorResolution::Pending {
                    capture_point: capture_point.clone(),
                    frame_identity: RenderCaptureIdentity {
                        frame_index: 9,
                        pass_label: "pass.pending".to_string(),
                        capture_point,
                    },
                },
            }],
        };

        assert!(validate_selector_terminal_invariant(&plan, &[]).is_ok());
    }
}
