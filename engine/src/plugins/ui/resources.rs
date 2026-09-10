use ui_surface::{
    MountedSurfaceInstance, MountedSurfaceRegistry, SessionScopeHandle, SurfaceDefinitionId,
    SurfaceHostInstanceId, SurfaceInstanceId,
};

use super::{
    UiMountRecord, UiMountReport, UiMountRequest, UiMountSource, UiMountedSessionRecord,
    UiRuntimeDiagnostic, UiRuntimeDiagnosticsResource, UiRuntimeDirtyCause, UiRuntimeDirtyRecord,
    UiRuntimeEvaluationFailureReason, UiRuntimeEvaluationInput, UiRuntimeEvaluationReport,
    UiRuntimeFramePayload, UiRuntimeOutputFacts, UiRuntimeSessionSnapshot, UiRuntimeTraceEvent,
    UiRuntimeTraceResource, UiRuntimeViewFacts, UiUnmountReport,
};
use ui_artifacts::UiRuntimeArtifactDiagnosticSeverity;
use ui_evaluator::{UiEvaluationContext, UiEvaluator};
use ui_runtime_view::UiRuntimeView;
use ui_state::UiStateModel;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Default)]
pub enum UiRuntimeInstallState {
    #[default]
    Uninstalled,
    Installed,
}

/// Foundation resource for UI runtime plugin installation state.
#[derive(Debug, Clone, PartialEq, Eq, runen_ecs::Resource)]
pub struct UiRuntimeResource {
    install_state: UiRuntimeInstallState,
}

impl Default for UiRuntimeResource {
    fn default() -> Self {
        Self {
            install_state: UiRuntimeInstallState::Uninstalled,
        }
    }
}

impl UiRuntimeResource {
    pub fn install_state(&self) -> UiRuntimeInstallState {
        self.install_state
    }

    pub fn is_installed(&self) -> bool {
        self.install_state == UiRuntimeInstallState::Installed
    }

    pub(crate) fn mark_installed(&mut self) {
        self.install_state = UiRuntimeInstallState::Installed;
    }
}

#[derive(Debug, Clone, PartialEq, Eq, runen_ecs::Resource)]
pub struct UiMountRequestsResource {
    records: Vec<UiMountRecord>,
    reports: Vec<UiMountReport>,
    unmount_reports: Vec<UiUnmountReport>,
    mounted_surfaces: MountedSurfaceRegistry,
    mounted_sessions: Vec<UiMountedSessionRecord>,
    next_surface_instance_id: u64,
    next_definition_id: u64,
    next_host_instance_id: u64,
    next_session_scope_id: u64,
}

impl Default for UiMountRequestsResource {
    fn default() -> Self {
        Self {
            records: Vec::new(),
            reports: Vec::new(),
            unmount_reports: Vec::new(),
            mounted_surfaces: MountedSurfaceRegistry::default(),
            mounted_sessions: Vec::new(),
            next_surface_instance_id: 1,
            next_definition_id: 1,
            next_host_instance_id: 1,
            next_session_scope_id: 1,
        }
    }
}

impl UiMountRequestsResource {
    pub fn records(&self) -> &[UiMountRecord] {
        &self.records
    }

    pub fn reports(&self) -> &[UiMountReport] {
        &self.reports
    }

    pub fn latest_report(&self) -> Option<&UiMountReport> {
        self.reports.last()
    }

    pub fn unmount_reports(&self) -> &[UiUnmountReport] {
        &self.unmount_reports
    }

    pub fn latest_unmount_report(&self) -> Option<&UiUnmountReport> {
        self.unmount_reports.last()
    }

    pub fn mounted_sessions(&self) -> &[UiMountedSessionRecord] {
        &self.mounted_sessions
    }

    pub fn mounted_surface(
        &self,
        surface_instance_id: SurfaceInstanceId,
    ) -> Option<MountedSurfaceInstance> {
        self.mounted_surfaces.mounted_surface(surface_instance_id)
    }

    pub fn mounted_surfaces(&self) -> impl Iterator<Item = MountedSurfaceInstance> + '_ {
        self.mounted_surfaces.mounted_surfaces()
    }

    pub fn mounted_generation(&self) -> u64 {
        self.mounted_surfaces.generation()
    }

    pub fn len(&self) -> usize {
        self.records.len()
    }

    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    pub(crate) fn record_mount_request(
        &mut self,
        request: UiMountRequest,
        mount_source: UiMountSource,
    ) -> UiMountReport {
        if let Some(reason) = request.failure_reason() {
            let report = UiMountReport::rejected(&request, mount_source, reason);
            self.reports.push(report.clone());
            return report;
        }

        let record = UiMountRecord::new(request, mount_source);
        let mounted_session = self.mount_session_for_record(&record);
        let report = UiMountReport::accepted(&record, &mounted_session);
        self.records.push(record);
        self.reports.push(report.clone());
        report
    }

    pub fn unmount_surface(&mut self, surface_instance_id: SurfaceInstanceId) -> UiUnmountReport {
        let mounted_count_before = self.mounted_sessions.len();
        self.mounted_sessions
            .retain(|session| session.surface_instance_id() != surface_instance_id);
        let removed = self.mounted_sessions.len() != mounted_count_before;

        if removed {
            self.rebuild_mounted_surfaces();
        }

        let report = UiUnmountReport::new(
            surface_instance_id,
            removed,
            self.mounted_surfaces.generation(),
            self.mounted_sessions.len(),
        );
        self.unmount_reports.push(report.clone());
        report
    }

    fn mount_session_for_record(&mut self, record: &UiMountRecord) -> UiMountedSessionRecord {
        let surface_instance_id = self.next_surface_instance_id();
        let mounted_surface = MountedSurfaceInstance::new(
            surface_instance_id,
            self.next_definition_id(),
            self.next_host_instance_id(),
        );
        let session = SessionScopeHandle::new(
            surface_instance_id,
            self.next_session_scope_id(),
            record.retention_class(),
        );
        self.mounted_sessions.push(UiMountedSessionRecord::new(
            record,
            mounted_surface,
            session,
        ));
        self.rebuild_mounted_surfaces();
        self.mounted_sessions
            .iter()
            .find(|session| session.surface_instance_id() == surface_instance_id)
            .expect("newly mounted UI session should be present after registry rebuild")
            .clone()
    }

    fn rebuild_mounted_surfaces(&mut self) {
        self.mounted_surfaces.rebuild(
            self.mounted_sessions
                .iter()
                .map(|session| session.mounted_surface()),
        );
        for session in &mut self.mounted_sessions {
            if let Some(mounted_surface) = self
                .mounted_surfaces
                .mounted_surface(session.surface_instance_id())
            {
                session.replace_mounted_surface(mounted_surface);
            }
        }
    }

    fn next_surface_instance_id(&mut self) -> SurfaceInstanceId {
        let id = SurfaceInstanceId::new(self.next_surface_instance_id);
        self.next_surface_instance_id = self.next_surface_instance_id.saturating_add(1);
        id
    }

    fn next_definition_id(&mut self) -> SurfaceDefinitionId {
        let id = SurfaceDefinitionId::new(self.next_definition_id);
        self.next_definition_id = self.next_definition_id.saturating_add(1);
        id
    }

    fn next_host_instance_id(&mut self) -> SurfaceHostInstanceId {
        let id = SurfaceHostInstanceId::new(self.next_host_instance_id);
        self.next_host_instance_id = self.next_host_instance_id.saturating_add(1);
        id
    }

    fn next_session_scope_id(&mut self) -> u64 {
        let id = self.next_session_scope_id;
        self.next_session_scope_id = self.next_session_scope_id.saturating_add(1);
        id
    }
}

#[derive(Debug, Clone, PartialEq, Default, runen_ecs::Resource)]
pub struct UiRuntimeEvaluationResource {
    state: UiStateModel,
    reports: Vec<UiRuntimeEvaluationReport>,
    snapshots: Vec<UiRuntimeSessionSnapshot>,
    dirty_records: Vec<UiRuntimeDirtyRecord>,
    next_runtime_id: u64,
    next_frame_revision: u64,
}

impl UiRuntimeEvaluationResource {
    pub fn reports(&self) -> &[UiRuntimeEvaluationReport] {
        &self.reports
    }

    pub fn latest_report(&self) -> Option<&UiRuntimeEvaluationReport> {
        self.reports.last()
    }

    pub fn snapshots(&self) -> &[UiRuntimeSessionSnapshot] {
        &self.snapshots
    }

    pub fn dirty_records(&self) -> &[UiRuntimeDirtyRecord] {
        &self.dirty_records
    }

    pub fn state(&self) -> &UiStateModel {
        &self.state
    }

    pub fn replay_snapshot(
        &self,
        runtime_id: &str,
        source_id: &str,
    ) -> Option<&UiRuntimeSessionSnapshot> {
        self.snapshots.iter().rev().find(|snapshot| {
            snapshot.runtime_id() == runtime_id && snapshot.source_id() == source_id
        })
    }

    pub fn evaluate(
        &mut self,
        input: &UiRuntimeEvaluationInput,
        mounted_session: Option<&UiMountedSessionRecord>,
        context: UiEvaluationContext,
        trace: &mut UiRuntimeTraceResource,
        diagnostics: &mut UiRuntimeDiagnosticsResource,
    ) -> UiRuntimeEvaluationReport {
        let runtime_id = self.next_runtime_id(input.facts().source_id());
        trace.record(UiRuntimeTraceEvent::runtime_evaluation(
            runtime_id.clone(),
            input.facts(),
        ));

        let output = UiEvaluator.evaluate_with_context(input.artifact(), &mut self.state, context);
        let runtime_view_report = UiRuntimeView::from_artifact_report(input.artifact());
        let runtime_view = UiRuntimeViewFacts::from_report(&runtime_view_report);
        let output = UiRuntimeOutputFacts::from_output(&output, &self.state);
        let surface_instance_id = mounted_session.map(UiMountedSessionRecord::surface_instance_id);
        let session_scope_id = mounted_session.map(|session| session.session().scope_id);
        let frame_payload = UiRuntimeFramePayload::from_output(
            self.next_frame_revision(),
            &output,
            surface_instance_id,
        );
        let snapshot = UiRuntimeSessionSnapshot::new(
            runtime_id.clone(),
            input.facts(),
            surface_instance_id,
            session_scope_id,
            &output,
        );
        trace.record(UiRuntimeTraceEvent::state_snapshot(
            runtime_id.clone(),
            input.facts(),
        ));

        let dirty_records =
            self.dirty_records_for_evaluation(&runtime_id, input, mounted_session, &output);
        for record in &dirty_records {
            trace.record(UiRuntimeTraceEvent::invalidation(
                runtime_id.clone(),
                input.facts(),
                record.cause(),
            ));
        }

        if input
            .artifact()
            .manifest
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity == UiRuntimeArtifactDiagnosticSeverity::Error)
        {
            diagnostics.push(UiRuntimeDiagnostic::runtime_evaluation_rejected(
                runtime_id.clone(),
                input.facts().source_id(),
                input.facts().program_id(),
                UiRuntimeEvaluationFailureReason::ArtifactDiagnostics,
            ));
        } else if !runtime_view.passed() {
            diagnostics.push(UiRuntimeDiagnostic::runtime_evaluation_rejected(
                runtime_id.clone(),
                input.facts().source_id(),
                input.facts().program_id(),
                UiRuntimeEvaluationFailureReason::RuntimeViewDiagnostics,
            ));
        }

        let report = UiRuntimeEvaluationReport::new(
            runtime_id,
            input.facts().clone(),
            runtime_view,
            output,
            frame_payload,
            snapshot.clone(),
            dirty_records.clone(),
        );
        self.snapshots.push(snapshot);
        self.dirty_records.extend(dirty_records);
        self.reports.push(report.clone());
        report
    }

    fn dirty_records_for_evaluation(
        &self,
        runtime_id: &str,
        input: &UiRuntimeEvaluationInput,
        mounted_session: Option<&UiMountedSessionRecord>,
        output: &UiRuntimeOutputFacts,
    ) -> Vec<UiRuntimeDirtyRecord> {
        let source_id = input.facts().source_id();
        let mut records = vec![UiRuntimeDirtyRecord::new(
            runtime_id,
            source_id,
            UiRuntimeDirtyCause::Source,
            input.facts().artifact_id(),
        )];

        if output.dirty_binding_count() > 0 {
            records.push(UiRuntimeDirtyRecord::new(
                runtime_id,
                source_id,
                UiRuntimeDirtyCause::HostData,
                format!("{} dirty bindings", output.dirty_binding_count()),
            ));
        }

        if let Some(session) = mounted_session {
            records.push(UiRuntimeDirtyRecord::new(
                runtime_id,
                source_id,
                UiRuntimeDirtyCause::Session,
                session.session().scope_id.to_string(),
            ));
            records.push(UiRuntimeDirtyRecord::new(
                runtime_id,
                source_id,
                UiRuntimeDirtyCause::Surface,
                session.surface_instance_id().raw().to_string(),
            ));
        }

        if output.layout_count() > 0 {
            records.push(UiRuntimeDirtyRecord::new(
                runtime_id,
                source_id,
                UiRuntimeDirtyCause::Layout,
                format!("{} layout rows", output.layout_count()),
            ));
        }
        if output.text_layout_request_count() > 0 || output.has_text_value() {
            records.push(UiRuntimeDirtyRecord::new(
                runtime_id,
                source_id,
                UiRuntimeDirtyCause::Text,
                format!(
                    "{} text layout requests",
                    output.text_layout_request_count()
                ),
            ));
        }
        if output.visual_operator_count() > 0 {
            records.push(UiRuntimeDirtyRecord::new(
                runtime_id,
                source_id,
                UiRuntimeDirtyCause::Primitive,
                format!("{} visual operators", output.visual_operator_count()),
            ));
        }

        records.push(UiRuntimeDirtyRecord::new(
            runtime_id,
            source_id,
            UiRuntimeDirtyCause::Theme,
            format!("{} style rows", output.style_count()),
        ));
        records.push(UiRuntimeDirtyRecord::new(
            runtime_id,
            source_id,
            UiRuntimeDirtyCause::RenderPublication,
            "pending downstream render publication",
        ));
        records
    }

    fn next_runtime_id(&mut self, source_id: &str) -> String {
        let id = self.next_runtime_id.saturating_add(1);
        self.next_runtime_id = id;
        format!("{source_id}.runtime.{id}")
    }

    fn next_frame_revision(&mut self) -> u64 {
        let revision = self.next_frame_revision.saturating_add(1);
        self.next_frame_revision = revision;
        revision
    }
}
