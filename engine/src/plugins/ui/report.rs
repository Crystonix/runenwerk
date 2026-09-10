use super::UiRuntimeInstallState;
use super::{UiHostMutationReceipt, UiRuntimeSourceProgramFacts, UiTypedActionId};

use crate::plugins::render::RenderFrameProducerId;
use crate::plugins::render::backend::RenderSurfaceId;
use ui_evaluator::UiOutput;
use ui_hosts::{DomainCommand, HostCommand, HostKind};
use ui_program::RouteId;
use ui_runtime_view::UiRuntimeViewReport;
use ui_schema::UiSchemaValue;
use ui_state::UiStateModel;
use ui_surface::SurfaceInstanceId;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct UiRuntimeReport {
    pub install_state: UiRuntimeInstallState,
    pub diagnostic_count: usize,
}

impl Default for UiRuntimeReport {
    fn default() -> Self {
        Self {
            install_state: UiRuntimeInstallState::Uninstalled,
            diagnostic_count: 0,
        }
    }
}

/// Latest lightweight UI runtime status report.
#[derive(Debug, Clone, PartialEq, Eq, Default, runen_ecs::Resource)]
pub struct UiRuntimeReportResource {
    latest: UiRuntimeReport,
}

impl UiRuntimeReportResource {
    pub fn latest(&self) -> UiRuntimeReport {
        self.latest
    }

    pub(crate) fn record_plugin_installed(&mut self, diagnostic_count: usize) {
        self.latest = UiRuntimeReport {
            install_state: UiRuntimeInstallState::Installed,
            diagnostic_count,
        };
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum UiActionDispatchFailureReason {
    UnknownRoute,
    SchemaMismatch,
    CapabilityMismatch,
    PayloadMismatch,
    MissingHostData,
    HostRejected,
}

impl UiActionDispatchFailureReason {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::UnknownRoute => "unknown_route",
            Self::SchemaMismatch => "schema_mismatch",
            Self::CapabilityMismatch => "capability_mismatch",
            Self::PayloadMismatch => "payload_mismatch",
            Self::MissingHostData => "missing_host_data",
            Self::HostRejected => "host_rejected",
        }
    }

    pub fn message(self) -> &'static str {
        match self {
            Self::UnknownRoute => "UI action route is not mapped by the host",
            Self::SchemaMismatch => "UI action schema version is not accepted by the host",
            Self::CapabilityMismatch => "UI action is missing a required route capability",
            Self::PayloadMismatch => "UI action payload does not match the typed action contract",
            Self::MissingHostData => "UI action host data required for mutation is missing",
            Self::HostRejected => "UI action host mutation was rejected",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiActionDispatchReport {
    action_id: UiTypedActionId,
    route: RouteId,
    host: HostKind,
    accepted: bool,
    failure_reason: Option<UiActionDispatchFailureReason>,
    host_command: Option<HostCommand>,
    domain_command: Option<DomainCommand>,
}

impl UiActionDispatchReport {
    pub(crate) fn accepted(
        action_id: UiTypedActionId,
        route: RouteId,
        host: HostKind,
        receipt: UiHostMutationReceipt,
    ) -> Self {
        Self {
            action_id,
            route,
            host,
            accepted: true,
            failure_reason: None,
            host_command: Some(receipt.host_command().clone()),
            domain_command: receipt.domain_command().cloned(),
        }
    }

    pub(crate) fn rejected(
        action_id: UiTypedActionId,
        route: RouteId,
        host: HostKind,
        failure_reason: UiActionDispatchFailureReason,
    ) -> Self {
        Self {
            action_id,
            route,
            host,
            accepted: false,
            failure_reason: Some(failure_reason),
            host_command: None,
            domain_command: None,
        }
    }

    pub fn action_id(&self) -> &UiTypedActionId {
        &self.action_id
    }

    pub fn route(&self) -> &RouteId {
        &self.route
    }

    pub fn host(&self) -> HostKind {
        self.host
    }

    pub fn is_accepted(&self) -> bool {
        self.accepted
    }

    pub fn failure_reason(&self) -> Option<UiActionDispatchFailureReason> {
        self.failure_reason
    }

    pub fn host_command(&self) -> Option<&HostCommand> {
        self.host_command.as_ref()
    }

    pub fn domain_command(&self) -> Option<&DomainCommand> {
        self.domain_command.as_ref()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default, runen_ecs::Resource)]
pub struct UiActionDispatchReportsResource {
    reports: Vec<UiActionDispatchReport>,
}

impl UiActionDispatchReportsResource {
    pub fn reports(&self) -> &[UiActionDispatchReport] {
        &self.reports
    }

    pub fn latest_report(&self) -> Option<&UiActionDispatchReport> {
        self.reports.last()
    }

    pub fn len(&self) -> usize {
        self.reports.len()
    }

    pub fn is_empty(&self) -> bool {
        self.reports.is_empty()
    }

    pub(crate) fn record(&mut self, report: UiActionDispatchReport) {
        self.reports.push(report);
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum UiRuntimeDirtyCause {
    Source,
    HostData,
    Session,
    Layout,
    Text,
    Theme,
    Primitive,
    Surface,
    RenderPublication,
}

impl UiRuntimeDirtyCause {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Source => "source",
            Self::HostData => "host-data",
            Self::Session => "session",
            Self::Layout => "layout",
            Self::Text => "text",
            Self::Theme => "theme",
            Self::Primitive => "primitive",
            Self::Surface => "surface",
            Self::RenderPublication => "render-publication",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiRuntimeDirtyRecord {
    runtime_id: String,
    source_id: String,
    cause: UiRuntimeDirtyCause,
    detail: String,
}

impl UiRuntimeDirtyRecord {
    pub fn new(
        runtime_id: impl Into<String>,
        source_id: impl Into<String>,
        cause: UiRuntimeDirtyCause,
        detail: impl Into<String>,
    ) -> Self {
        Self {
            runtime_id: runtime_id.into(),
            source_id: source_id.into(),
            cause,
            detail: detail.into(),
        }
    }

    pub fn runtime_id(&self) -> &str {
        &self.runtime_id
    }

    pub fn source_id(&self) -> &str {
        &self.source_id
    }

    pub fn cause(&self) -> UiRuntimeDirtyCause {
        self.cause
    }

    pub fn detail(&self) -> &str {
        &self.detail
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct UiRuntimeStateValueFact {
    key: String,
    revision: u64,
    value: UiSchemaValue,
}

impl UiRuntimeStateValueFact {
    pub fn new(key: impl Into<String>, revision: u64, value: UiSchemaValue) -> Self {
        Self {
            key: key.into(),
            revision,
            value,
        }
    }

    pub fn key(&self) -> &str {
        &self.key
    }

    pub fn revision(&self) -> u64 {
        self.revision
    }

    pub fn value(&self) -> &UiSchemaValue {
        &self.value
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct UiRuntimeOutputFacts {
    control_count: usize,
    layout_count: usize,
    style_count: usize,
    state_count: usize,
    binding_count: usize,
    dirty_binding_count: usize,
    visual_operator_count: usize,
    text_layout_request_count: usize,
    diagnostic_count: usize,
    state_values: Vec<UiRuntimeStateValueFact>,
}

impl UiRuntimeOutputFacts {
    pub fn from_output(output: &UiOutput, state: &UiStateModel) -> Self {
        Self {
            control_count: output.controls.rows.len(),
            layout_count: output.layout.rows.len(),
            style_count: output.style.rows.len(),
            state_count: output.state.rows.len(),
            binding_count: output.binding.table_rows.len(),
            dirty_binding_count: output.binding.dirty_report.dirty_bindings.len(),
            visual_operator_count: output.visual.operators.len(),
            text_layout_request_count: output.visual.text_layout_requests.len(),
            diagnostic_count: output.diagnostics.len(),
            state_values: state
                .cells()
                .filter_map(|cell| {
                    cell.value.clone().map(|value| {
                        UiRuntimeStateValueFact::new(
                            cell.key.as_str().to_owned(),
                            cell.revision,
                            value,
                        )
                    })
                })
                .collect(),
        }
    }

    pub fn control_count(&self) -> usize {
        self.control_count
    }

    pub fn layout_count(&self) -> usize {
        self.layout_count
    }

    pub fn style_count(&self) -> usize {
        self.style_count
    }

    pub fn state_count(&self) -> usize {
        self.state_count
    }

    pub fn binding_count(&self) -> usize {
        self.binding_count
    }

    pub fn dirty_binding_count(&self) -> usize {
        self.dirty_binding_count
    }

    pub fn visual_operator_count(&self) -> usize {
        self.visual_operator_count
    }

    pub fn text_layout_request_count(&self) -> usize {
        self.text_layout_request_count
    }

    pub fn diagnostic_count(&self) -> usize {
        self.diagnostic_count
    }

    pub fn state_values(&self) -> &[UiRuntimeStateValueFact] {
        &self.state_values
    }

    pub fn has_text_value(&self) -> bool {
        self.state_values
            .iter()
            .any(|fact| matches!(fact.value(), UiSchemaValue::String(_)))
    }

    pub fn state_value(&self, key: &str) -> Option<&UiSchemaValue> {
        self.state_values
            .iter()
            .find(|fact| fact.key() == key)
            .map(UiRuntimeStateValueFact::value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiRuntimeViewFacts {
    control_count: usize,
    diagnostic_count: usize,
    artifact_diagnostic_count: usize,
    passed: bool,
}

impl UiRuntimeViewFacts {
    pub fn from_report(report: &UiRuntimeViewReport) -> Self {
        Self {
            control_count: report.view.controls.len(),
            diagnostic_count: report.view.diagnostics.len(),
            artifact_diagnostic_count: report.artifact_diagnostics.len(),
            passed: report.passed(),
        }
    }

    pub fn control_count(&self) -> usize {
        self.control_count
    }

    pub fn diagnostic_count(&self) -> usize {
        self.diagnostic_count
    }

    pub fn artifact_diagnostic_count(&self) -> usize {
        self.artifact_diagnostic_count
    }

    pub fn passed(&self) -> bool {
        self.passed
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiRuntimeFramePayload {
    frame_revision: u64,
    surface_count: usize,
    layer_count: usize,
    primitive_count: usize,
    text_layout_request_count: usize,
    visual_operator_count: usize,
}

impl UiRuntimeFramePayload {
    pub fn from_output(
        frame_revision: u64,
        output: &UiRuntimeOutputFacts,
        mounted_surface: Option<SurfaceInstanceId>,
    ) -> Self {
        let primitive_count = output
            .visual_operator_count()
            .saturating_add(output.text_layout_request_count());
        Self {
            frame_revision,
            surface_count: usize::from(mounted_surface.is_some() || output.control_count() > 0),
            layer_count: usize::from(primitive_count > 0),
            primitive_count,
            text_layout_request_count: output.text_layout_request_count(),
            visual_operator_count: output.visual_operator_count(),
        }
    }

    pub fn frame_revision(&self) -> u64 {
        self.frame_revision
    }

    pub fn surface_count(&self) -> usize {
        self.surface_count
    }

    pub fn layer_count(&self) -> usize {
        self.layer_count
    }

    pub fn primitive_count(&self) -> usize {
        self.primitive_count
    }

    pub fn text_layout_request_count(&self) -> usize {
        self.text_layout_request_count
    }

    pub fn visual_operator_count(&self) -> usize {
        self.visual_operator_count
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum UiRuntimeFramePublicationStatus {
    Published,
    MissingRuntimeEvaluation,
}

impl UiRuntimeFramePublicationStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Published => "published",
            Self::MissingRuntimeEvaluation => "missing-runtime-evaluation",
        }
    }

    pub const fn is_published(self) -> bool {
        matches!(self, Self::Published)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiRuntimeFramePublicationReport {
    runtime_id: Option<String>,
    source_id: Option<String>,
    program_id: Option<String>,
    producer_id: RenderFrameProducerId,
    render_surface_id: RenderSurfaceId,
    frame_revision: Option<u64>,
    dirty_causes: Vec<UiRuntimeDirtyCause>,
    primitive_count: usize,
    status: UiRuntimeFramePublicationStatus,
}

impl UiRuntimeFramePublicationReport {
    pub fn published(
        evaluation: &UiRuntimeEvaluationReport,
        producer_id: RenderFrameProducerId,
        render_surface_id: RenderSurfaceId,
    ) -> Self {
        Self {
            runtime_id: Some(evaluation.runtime_id().to_owned()),
            source_id: Some(evaluation.source().source_id().to_owned()),
            program_id: Some(evaluation.source().program_id().to_owned()),
            producer_id,
            render_surface_id,
            frame_revision: Some(evaluation.frame_payload().frame_revision()),
            dirty_causes: evaluation.dirty_causes().collect(),
            primitive_count: evaluation.frame_payload().primitive_count(),
            status: UiRuntimeFramePublicationStatus::Published,
        }
    }

    pub fn missing_runtime_evaluation(
        producer_id: RenderFrameProducerId,
        render_surface_id: RenderSurfaceId,
    ) -> Self {
        Self {
            runtime_id: None,
            source_id: None,
            program_id: None,
            producer_id,
            render_surface_id,
            frame_revision: None,
            dirty_causes: Vec::new(),
            primitive_count: 0,
            status: UiRuntimeFramePublicationStatus::MissingRuntimeEvaluation,
        }
    }

    pub fn runtime_id(&self) -> Option<&str> {
        self.runtime_id.as_deref()
    }

    pub fn source_id(&self) -> Option<&str> {
        self.source_id.as_deref()
    }

    pub fn program_id(&self) -> Option<&str> {
        self.program_id.as_deref()
    }

    pub fn producer_id(&self) -> RenderFrameProducerId {
        self.producer_id
    }

    pub fn render_surface_id(&self) -> RenderSurfaceId {
        self.render_surface_id
    }

    pub fn frame_revision(&self) -> Option<u64> {
        self.frame_revision
    }

    pub fn dirty_causes(&self) -> &[UiRuntimeDirtyCause] {
        &self.dirty_causes
    }

    pub fn primitive_count(&self) -> usize {
        self.primitive_count
    }

    pub fn status(&self) -> UiRuntimeFramePublicationStatus {
        self.status
    }

    pub fn is_published(&self) -> bool {
        self.status.is_published()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default, runen_ecs::Resource)]
pub struct UiRuntimeFramePublicationResource {
    reports: Vec<UiRuntimeFramePublicationReport>,
}

impl UiRuntimeFramePublicationResource {
    pub fn reports(&self) -> &[UiRuntimeFramePublicationReport] {
        &self.reports
    }

    pub fn latest_report(&self) -> Option<&UiRuntimeFramePublicationReport> {
        self.reports.last()
    }

    pub fn record(&mut self, report: UiRuntimeFramePublicationReport) {
        self.reports.push(report);
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct UiRuntimeSessionSnapshot {
    runtime_id: String,
    source_id: String,
    program_id: String,
    artifact_id: String,
    surface_instance_id: Option<SurfaceInstanceId>,
    session_scope_id: Option<u64>,
    state_values: Vec<UiRuntimeStateValueFact>,
}

impl UiRuntimeSessionSnapshot {
    pub fn new(
        runtime_id: impl Into<String>,
        source: &UiRuntimeSourceProgramFacts,
        surface_instance_id: Option<SurfaceInstanceId>,
        session_scope_id: Option<u64>,
        output: &UiRuntimeOutputFacts,
    ) -> Self {
        Self {
            runtime_id: runtime_id.into(),
            source_id: source.source_id().to_owned(),
            program_id: source.program_id().to_owned(),
            artifact_id: source.artifact_id().to_owned(),
            surface_instance_id,
            session_scope_id,
            state_values: output.state_values().to_vec(),
        }
    }

    pub fn runtime_id(&self) -> &str {
        &self.runtime_id
    }

    pub fn source_id(&self) -> &str {
        &self.source_id
    }

    pub fn program_id(&self) -> &str {
        &self.program_id
    }

    pub fn artifact_id(&self) -> &str {
        &self.artifact_id
    }

    pub fn surface_instance_id(&self) -> Option<SurfaceInstanceId> {
        self.surface_instance_id
    }

    pub fn session_scope_id(&self) -> Option<u64> {
        self.session_scope_id
    }

    pub fn state_values(&self) -> &[UiRuntimeStateValueFact] {
        &self.state_values
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct UiRuntimeEvaluationReport {
    runtime_id: String,
    source: UiRuntimeSourceProgramFacts,
    runtime_view: UiRuntimeViewFacts,
    output: UiRuntimeOutputFacts,
    frame_payload: UiRuntimeFramePayload,
    snapshot: UiRuntimeSessionSnapshot,
    dirty_records: Vec<UiRuntimeDirtyRecord>,
}

impl UiRuntimeEvaluationReport {
    pub fn new(
        runtime_id: impl Into<String>,
        source: UiRuntimeSourceProgramFacts,
        runtime_view: UiRuntimeViewFacts,
        output: UiRuntimeOutputFacts,
        frame_payload: UiRuntimeFramePayload,
        snapshot: UiRuntimeSessionSnapshot,
        dirty_records: Vec<UiRuntimeDirtyRecord>,
    ) -> Self {
        Self {
            runtime_id: runtime_id.into(),
            source,
            runtime_view,
            output,
            frame_payload,
            snapshot,
            dirty_records,
        }
    }

    pub fn runtime_id(&self) -> &str {
        &self.runtime_id
    }

    pub fn source(&self) -> &UiRuntimeSourceProgramFacts {
        &self.source
    }

    pub fn runtime_view(&self) -> &UiRuntimeViewFacts {
        &self.runtime_view
    }

    pub fn output(&self) -> &UiRuntimeOutputFacts {
        &self.output
    }

    pub fn frame_payload(&self) -> &UiRuntimeFramePayload {
        &self.frame_payload
    }

    pub fn snapshot(&self) -> &UiRuntimeSessionSnapshot {
        &self.snapshot
    }

    pub fn dirty_records(&self) -> &[UiRuntimeDirtyRecord] {
        &self.dirty_records
    }

    pub fn dirty_causes(&self) -> impl Iterator<Item = UiRuntimeDirtyCause> + '_ {
        self.dirty_records.iter().map(UiRuntimeDirtyRecord::cause)
    }
}
