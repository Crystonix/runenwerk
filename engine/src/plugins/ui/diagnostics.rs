use super::{
    UiActionDispatchFailureReason, UiMountFailureReason, UiMountSource, UiTypedIdentityError,
};

use crate::plugins::render::RenderFrameProducerId;
use crate::plugins::render::backend::RenderSurfaceId;
use ui_hosts::HostKind;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum UiRuntimeDiagnosticSeverity {
    Info,
    Warning,
    Error,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum UiRuntimeDiagnosticCode {
    PluginInstall,
    ResourceInitialization,
    MountRequestRejected,
    TypedContractRejected,
    ActionDispatchRejected,
    RuntimeEvaluationRejected,
    FramePublicationRejected,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiMountDiagnostic {
    pub screen_identity: String,
    pub mount_source: UiMountSource,
    pub failure_reason: UiMountFailureReason,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum UiTypedContractKind {
    Screen,
    Source,
    Action,
    HostIntent,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UiTypedContractFailureReason {
    InvalidIdentity(UiTypedIdentityError),
    MissingSourceFact,
    MissingActionCapability,
    HostIntentRejected,
}

impl UiTypedContractFailureReason {
    pub fn message(&self) -> &'static str {
        match self {
            Self::InvalidIdentity(_) => "typed UI contract identity is invalid",
            Self::MissingSourceFact => "typed UI source did not produce required source facts",
            Self::MissingActionCapability => "typed UI action did not declare a route capability",
            Self::HostIntentRejected => "typed UI host intent was rejected",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiTypedContractDiagnostic {
    pub contract: UiTypedContractKind,
    pub identity: String,
    pub failure_reason: UiTypedContractFailureReason,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiActionDispatchDiagnostic {
    pub action_id: String,
    pub route: String,
    pub host: HostKind,
    pub failure_reason: UiActionDispatchFailureReason,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum UiRuntimeEvaluationFailureReason {
    ArtifactDiagnostics,
    RuntimeViewDiagnostics,
}

impl UiRuntimeEvaluationFailureReason {
    pub const fn message(self) -> &'static str {
        match self {
            Self::ArtifactDiagnostics => "UI runtime artifact evaluation produced diagnostics",
            Self::RuntimeViewDiagnostics => "UI runtime view projection produced diagnostics",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiRuntimeEvaluationDiagnostic {
    pub runtime_id: String,
    pub source_id: String,
    pub program_id: String,
    pub failure_reason: UiRuntimeEvaluationFailureReason,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum UiRuntimeFramePublicationFailureReason {
    MissingRuntimeEvaluation,
}

impl UiRuntimeFramePublicationFailureReason {
    pub const fn message(self) -> &'static str {
        match self {
            Self::MissingRuntimeEvaluation => {
                "UI runtime frame publication has no evaluated frame payload"
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiRuntimeFramePublicationDiagnostic {
    pub producer_id: RenderFrameProducerId,
    pub render_surface_id: RenderSurfaceId,
    pub failure_reason: UiRuntimeFramePublicationFailureReason,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiRuntimeDiagnostic {
    pub code: UiRuntimeDiagnosticCode,
    pub severity: UiRuntimeDiagnosticSeverity,
    pub message: &'static str,
    pub mount: Option<UiMountDiagnostic>,
    pub typed_contract: Option<UiTypedContractDiagnostic>,
    pub action_dispatch: Option<UiActionDispatchDiagnostic>,
    pub runtime_evaluation: Option<UiRuntimeEvaluationDiagnostic>,
    pub frame_publication: Option<UiRuntimeFramePublicationDiagnostic>,
}

impl UiRuntimeDiagnostic {
    pub fn new(
        code: UiRuntimeDiagnosticCode,
        severity: UiRuntimeDiagnosticSeverity,
        message: &'static str,
    ) -> Self {
        Self {
            code,
            severity,
            message,
            mount: None,
            typed_contract: None,
            action_dispatch: None,
            runtime_evaluation: None,
            frame_publication: None,
        }
    }

    pub fn mount_rejected(
        screen_identity: impl Into<String>,
        mount_source: UiMountSource,
        failure_reason: UiMountFailureReason,
    ) -> Self {
        Self {
            code: UiRuntimeDiagnosticCode::MountRequestRejected,
            severity: UiRuntimeDiagnosticSeverity::Error,
            message: failure_reason.message(),
            mount: Some(UiMountDiagnostic {
                screen_identity: screen_identity.into(),
                mount_source,
                failure_reason,
            }),
            typed_contract: None,
            action_dispatch: None,
            runtime_evaluation: None,
            frame_publication: None,
        }
    }

    pub fn typed_contract_rejected(
        contract: UiTypedContractKind,
        identity: impl Into<String>,
        failure_reason: UiTypedContractFailureReason,
    ) -> Self {
        Self {
            code: UiRuntimeDiagnosticCode::TypedContractRejected,
            severity: UiRuntimeDiagnosticSeverity::Error,
            message: failure_reason.message(),
            mount: None,
            typed_contract: Some(UiTypedContractDiagnostic {
                contract,
                identity: identity.into(),
                failure_reason,
            }),
            action_dispatch: None,
            runtime_evaluation: None,
            frame_publication: None,
        }
    }

    pub fn action_dispatch_rejected(
        action_id: impl Into<String>,
        route: impl Into<String>,
        host: HostKind,
        failure_reason: UiActionDispatchFailureReason,
    ) -> Self {
        Self {
            code: UiRuntimeDiagnosticCode::ActionDispatchRejected,
            severity: UiRuntimeDiagnosticSeverity::Error,
            message: failure_reason.message(),
            mount: None,
            typed_contract: None,
            action_dispatch: Some(UiActionDispatchDiagnostic {
                action_id: action_id.into(),
                route: route.into(),
                host,
                failure_reason,
            }),
            runtime_evaluation: None,
            frame_publication: None,
        }
    }

    pub fn runtime_evaluation_rejected(
        runtime_id: impl Into<String>,
        source_id: impl Into<String>,
        program_id: impl Into<String>,
        failure_reason: UiRuntimeEvaluationFailureReason,
    ) -> Self {
        Self {
            code: UiRuntimeDiagnosticCode::RuntimeEvaluationRejected,
            severity: UiRuntimeDiagnosticSeverity::Error,
            message: failure_reason.message(),
            mount: None,
            typed_contract: None,
            action_dispatch: None,
            runtime_evaluation: Some(UiRuntimeEvaluationDiagnostic {
                runtime_id: runtime_id.into(),
                source_id: source_id.into(),
                program_id: program_id.into(),
                failure_reason,
            }),
            frame_publication: None,
        }
    }

    pub fn frame_publication_rejected(
        producer_id: RenderFrameProducerId,
        render_surface_id: RenderSurfaceId,
        failure_reason: UiRuntimeFramePublicationFailureReason,
    ) -> Self {
        Self {
            code: UiRuntimeDiagnosticCode::FramePublicationRejected,
            severity: UiRuntimeDiagnosticSeverity::Error,
            message: failure_reason.message(),
            mount: None,
            typed_contract: None,
            action_dispatch: None,
            runtime_evaluation: None,
            frame_publication: Some(UiRuntimeFramePublicationDiagnostic {
                producer_id,
                render_surface_id,
                failure_reason,
            }),
        }
    }
}

/// Diagnostics collected by the UI runtime foundation.
#[derive(Debug, Clone, PartialEq, Eq, Default, runen_ecs::Resource)]
pub struct UiRuntimeDiagnosticsResource {
    entries: Vec<UiRuntimeDiagnostic>,
}

impl UiRuntimeDiagnosticsResource {
    pub fn entries(&self) -> &[UiRuntimeDiagnostic] {
        &self.entries
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn push(&mut self, diagnostic: UiRuntimeDiagnostic) {
        self.entries.push(diagnostic);
    }
}
