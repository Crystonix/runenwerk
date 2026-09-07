use crate::plugins::render::{
    PreparedFlowInvocationId, RenderDynamicTextureTargetKey, RenderFlowId, RenderPassId,
    RenderTargetAliasKey, RenderTargetAliasKind,
};
use runen_gpu::GpuWorkResourceId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RenderExecutionGraphDiagnosticSeverity {
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RenderExecutionGraphDiagnosticKind {
    FlowValidationIssue,
    InvalidResource,
    InvalidPassOrder,
    TargetAliasMissingBinding,
    TargetAliasKindMismatch,
    DynamicTargetMissingDescriptor,
    DynamicTargetInvalidDescriptor,
    DynamicTargetUsageMismatch,
    UniformOverrideUnknown,
    UniformOverrideEmpty,
    UniformMissing,
    DispatchMissing,
    DispatchInvalid,
    PreparedViewMissing,
    PreparedInvocationDuplicate,
    FeatureGateMissing,
    HistorySignatureConflict,
    BackendCapabilityMismatch,
    UnsupportedImportedResource,
    FullscreenInstancedWork,
    AmbiguousProceduralShape,
    InvalidPassShapeIntent,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderExecutionGraphDiagnostic {
    pub severity: RenderExecutionGraphDiagnosticSeverity,
    pub kind: RenderExecutionGraphDiagnosticKind,
    pub flow_id: Option<RenderFlowId>,
    pub flow_label: Option<String>,
    pub pass_id: Option<RenderPassId>,
    pub pass_label: Option<String>,
    pub resource_id: Option<GpuWorkResourceId>,
    pub resource_label: Option<String>,
    pub invocation_id: Option<PreparedFlowInvocationId>,
    pub view_id: Option<String>,
    pub alias_binding_key: Option<RenderTargetAliasKey>,
    pub alias_kind: Option<RenderTargetAliasKind>,
    pub dynamic_target_key: Option<RenderDynamicTextureTargetKey>,
    pub history_signature: Option<String>,
    pub capability: Option<String>,
    pub message: String,
}

impl RenderExecutionGraphDiagnostic {
    pub fn new(
        severity: RenderExecutionGraphDiagnosticSeverity,
        kind: RenderExecutionGraphDiagnosticKind,
        message: impl Into<String>,
    ) -> Self {
        Self {
            severity,
            kind,
            flow_id: None,
            flow_label: None,
            pass_id: None,
            pass_label: None,
            resource_id: None,
            resource_label: None,
            invocation_id: None,
            view_id: None,
            alias_binding_key: None,
            alias_kind: None,
            dynamic_target_key: None,
            history_signature: None,
            capability: None,
            message: message.into(),
        }
    }

    pub fn error(kind: RenderExecutionGraphDiagnosticKind, message: impl Into<String>) -> Self {
        Self::new(RenderExecutionGraphDiagnosticSeverity::Error, kind, message)
    }

    pub fn warning(kind: RenderExecutionGraphDiagnosticKind, message: impl Into<String>) -> Self {
        Self::new(
            RenderExecutionGraphDiagnosticSeverity::Warning,
            kind,
            message,
        )
    }

    pub fn with_flow(mut self, flow_id: RenderFlowId, flow_label: impl Into<String>) -> Self {
        self.flow_id = Some(flow_id);
        self.flow_label = Some(flow_label.into());
        self
    }

    pub fn with_pass(mut self, pass_id: RenderPassId, pass_label: impl Into<String>) -> Self {
        self.pass_id = Some(pass_id);
        self.pass_label = Some(pass_label.into());
        self
    }

    pub fn with_resource(
        mut self,
        resource_id: GpuWorkResourceId,
        resource_label: Option<impl Into<String>>,
    ) -> Self {
        self.resource_id = Some(resource_id);
        self.resource_label = resource_label.map(Into::into);
        self
    }

    pub fn with_invocation(mut self, invocation_id: PreparedFlowInvocationId) -> Self {
        self.invocation_id = Some(invocation_id);
        self
    }

    pub fn with_view(mut self, view_id: impl Into<String>) -> Self {
        self.view_id = Some(view_id.into());
        self
    }

    pub fn with_alias(
        mut self,
        alias_binding_key: RenderTargetAliasKey,
        alias_kind: RenderTargetAliasKind,
    ) -> Self {
        self.alias_binding_key = Some(alias_binding_key);
        self.alias_kind = Some(alias_kind);
        self
    }

    pub fn with_dynamic_target(mut self, key: RenderDynamicTextureTargetKey) -> Self {
        self.dynamic_target_key = Some(key);
        self
    }

    pub fn with_history_signature(mut self, signature: impl Into<String>) -> Self {
        self.history_signature = Some(signature.into());
        self
    }

    pub fn with_capability(mut self, capability: impl Into<String>) -> Self {
        self.capability = Some(capability.into());
        self
    }

    pub fn is_error(&self) -> bool {
        self.severity == RenderExecutionGraphDiagnosticSeverity::Error
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RenderExecutionGraphDiagnosticReport {
    pub diagnostics: Vec<RenderExecutionGraphDiagnostic>,
}

impl RenderExecutionGraphDiagnosticReport {
    pub fn new(diagnostics: Vec<RenderExecutionGraphDiagnostic>) -> Self {
        Self { diagnostics }
    }

    pub fn is_ok(&self) -> bool {
        !self.has_errors()
    }

    pub fn has_errors(&self) -> bool {
        self.diagnostics
            .iter()
            .any(RenderExecutionGraphDiagnostic::is_error)
    }

    pub fn error_count(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.is_error())
            .count()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{message}")]
pub struct RenderExecutionGraphCompileError {
    pub diagnostics: Vec<RenderExecutionGraphDiagnostic>,
    pub message: String,
}

impl RenderExecutionGraphCompileError {
    pub fn new(diagnostics: Vec<RenderExecutionGraphDiagnostic>) -> Self {
        let message = diagnostic_message("render execution graph compilation failed", &diagnostics);
        Self {
            diagnostics,
            message,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{message}")]
pub struct RenderExecutionGraphPreparedError {
    pub diagnostics: Vec<RenderExecutionGraphDiagnostic>,
    pub message: String,
}

impl RenderExecutionGraphPreparedError {
    pub fn new(diagnostics: Vec<RenderExecutionGraphDiagnostic>) -> Self {
        let message = diagnostic_message("prepared render frame preflight failed", &diagnostics);
        Self {
            diagnostics,
            message,
        }
    }
}

fn diagnostic_message(prefix: &str, diagnostics: &[RenderExecutionGraphDiagnostic]) -> String {
    let errors = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.is_error())
        .map(|diagnostic| diagnostic.message.as_str())
        .collect::<Vec<_>>();
    if errors.is_empty() {
        prefix.to_string()
    } else {
        format!("{}: {}", prefix, errors.join("; "))
    }
}
