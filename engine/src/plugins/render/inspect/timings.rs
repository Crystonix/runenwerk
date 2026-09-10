use crate::plugins::render::graph::RenderPreparedFramePreflightCacheState;
use crate::plugins::render::renderer::GfxFrameTimings;
use crate::plugins::render::shader::ShaderReloadPollReport;
use crate::runtime::FramePacingRuntimeStateResource;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RenderTimingSource {
    CpuEncodeSubmit,
    GpuTimestampQuery,
}

impl RenderTimingSource {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::CpuEncodeSubmit => "cpu_encode_submit",
            Self::GpuTimestampQuery => "gpu_timestamp_query",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum RenderGpuTimingCapability {
    Supported,
    Unsupported,
    #[default]
    UnavailableThisFrame,
    ReadbackPending,
}

impl RenderGpuTimingCapability {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Supported => "supported",
            Self::Unsupported => "unsupported",
            Self::UnavailableThisFrame => "unavailable_this_frame",
            Self::ReadbackPending => "readback_pending",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RenderGpuTimingDiagnosticKind {
    TimestampQueriesUnsupported,
    TimestampDataUnavailable,
    TimestampReadbackPending,
}

impl RenderGpuTimingDiagnosticKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::TimestampQueriesUnsupported => "timestamp_queries_unsupported",
            Self::TimestampDataUnavailable => "timestamp_data_unavailable",
            Self::TimestampReadbackPending => "timestamp_readback_pending",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderGpuTimingDiagnostic {
    pub kind: RenderGpuTimingDiagnosticKind,
    pub capability: RenderGpuTimingCapability,
    pub source: RenderTimingSource,
    pub frame_index: Option<u64>,
    pub render_surface_id: Option<u64>,
    pub flow_id: Option<String>,
    pub pass_id: Option<String>,
    pub message: String,
}

impl RenderGpuTimingDiagnostic {
    pub fn new(
        kind: RenderGpuTimingDiagnosticKind,
        capability: RenderGpuTimingCapability,
        message: impl Into<String>,
    ) -> Self {
        Self {
            kind,
            capability,
            source: RenderTimingSource::GpuTimestampQuery,
            frame_index: None,
            render_surface_id: None,
            flow_id: None,
            pass_id: None,
            message: message.into(),
        }
    }

    pub fn unsupported(message: impl Into<String>) -> Self {
        Self::new(
            RenderGpuTimingDiagnosticKind::TimestampQueriesUnsupported,
            RenderGpuTimingCapability::Unsupported,
            message,
        )
    }

    pub fn unavailable_this_frame(message: impl Into<String>) -> Self {
        Self::new(
            RenderGpuTimingDiagnosticKind::TimestampDataUnavailable,
            RenderGpuTimingCapability::UnavailableThisFrame,
            message,
        )
    }

    pub fn readback_pending(message: impl Into<String>) -> Self {
        Self::new(
            RenderGpuTimingDiagnosticKind::TimestampReadbackPending,
            RenderGpuTimingCapability::ReadbackPending,
            message,
        )
    }

    pub fn with_frame(mut self, frame_index: u64) -> Self {
        self.frame_index = Some(frame_index);
        self
    }

    pub fn with_surface(mut self, render_surface_id: u64) -> Self {
        self.render_surface_id = Some(render_surface_id);
        self
    }

    pub fn with_pass(mut self, flow_id: impl Into<String>, pass_id: impl Into<String>) -> Self {
        self.flow_id = Some(flow_id.into());
        self.pass_id = Some(pass_id.into());
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RenderPassTimingEvidence {
    pub frame_index: Option<u64>,
    pub render_surface_id: Option<u64>,
    pub flow_id: String,
    pub pass_id: String,
    pub pass_kind: String,
    pub source: RenderTimingSource,
    pub gpu_capability: RenderGpuTimingCapability,
    pub millis: Option<f32>,
    pub diagnostics: Vec<RenderGpuTimingDiagnostic>,
}

impl RenderPassTimingEvidence {
    pub fn from_cpu_sample(sample: &PassTimingSample) -> Self {
        Self {
            frame_index: None,
            render_surface_id: None,
            flow_id: sample.flow_id.clone(),
            pass_id: sample.pass_id.clone(),
            pass_kind: sample.pass_kind.clone(),
            source: RenderTimingSource::CpuEncodeSubmit,
            gpu_capability: RenderGpuTimingCapability::UnavailableThisFrame,
            millis: Some(sample.millis.max(0.0)),
            diagnostics: Vec::new(),
        }
    }

    pub fn gpu_sample(
        frame_index: Option<u64>,
        render_surface_id: Option<u64>,
        flow_id: impl Into<String>,
        pass_id: impl Into<String>,
        pass_kind: impl Into<String>,
        millis: f32,
    ) -> Self {
        Self {
            frame_index,
            render_surface_id,
            flow_id: flow_id.into(),
            pass_id: pass_id.into(),
            pass_kind: pass_kind.into(),
            source: RenderTimingSource::GpuTimestampQuery,
            gpu_capability: RenderGpuTimingCapability::Supported,
            millis: Some(millis.max(0.0)),
            diagnostics: Vec::new(),
        }
    }

    pub fn gpu_diagnostic(
        frame_index: Option<u64>,
        render_surface_id: Option<u64>,
        flow_id: impl Into<String>,
        pass_id: impl Into<String>,
        pass_kind: impl Into<String>,
        diagnostic: RenderGpuTimingDiagnostic,
    ) -> Self {
        let flow_id = flow_id.into();
        let pass_id = pass_id.into();
        Self {
            frame_index,
            render_surface_id,
            flow_id: flow_id.clone(),
            pass_id: pass_id.clone(),
            pass_kind: pass_kind.into(),
            source: RenderTimingSource::GpuTimestampQuery,
            gpu_capability: diagnostic.capability,
            millis: None,
            diagnostics: vec![RenderGpuTimingDiagnostic {
                frame_index,
                render_surface_id,
                flow_id: Some(flow_id),
                pass_id: Some(pass_id),
                ..diagnostic
            }],
        }
    }
}

#[derive(Debug, Clone, Default, runen_ecs::Component, runen_ecs::Resource)]
pub struct RenderDebugTimingsState {
    pub workload_ms: f32,
    pub total_ms: f32,
    pub preflight_ms: f32,
    pub flow_encode_ms: f32,
    pub encode_submit_ms: f32,
    pub pass_sample_count: usize,
    pub total_pass_millis: f32,
    pub slowest_pass_id: Option<String>,
    pub slowest_pass_millis: f32,
    pub gpu_timing_capability: RenderGpuTimingCapability,
    pub gpu_pass_sample_count: usize,
    pub gpu_total_pass_millis: f32,
    pub gpu_slowest_pass_id: Option<String>,
    pub gpu_slowest_pass_millis: f32,
    pub gpu_timing_diagnostics: Vec<RenderGpuTimingDiagnostic>,
    pub compute_dispatches: Vec<ComputeDispatchSample>,
    pub world_rebuild_latency_ms: f32,
    pub world_rebuild_samples: u64,
    pub world_prepare_samples: u64,
    pub shader_reload_poll_ms: f32,
    pub shader_reload_poll_status: Option<String>,
    pub shader_reload_poll_interval_ms: f32,
    pub diagnostics_report_ms: f32,
    pub diagnostics_report_mode: Option<String>,
    pub preflight_cache_mode: Option<String>,
    pub preflight_cache_status: Option<String>,
    pub preflight_report_source: Option<String>,
    pub frame_pacing_mode: Option<String>,
    pub frame_pacing_target_fps: Option<u32>,
    pub frame_pacing_last_interval_ms: f32,
    pub frame_pacing_next_delay_ms: Option<f32>,
    pub frame_pacing_redraw_requested: bool,
}

impl RenderDebugTimingsState {
    pub fn observe_frame_timings(&mut self, timings: GfxFrameTimings) {
        self.workload_ms = timings.renderer.prepare_ui_ms
            + timings.renderer.prepare_mesh_ms
            + timings.renderer.world_prepare_ms
            + timings.renderer.preflight_ms
            + timings.renderer.flow_encode_ms
            + timings.renderer.encode_submit_ms;
        self.total_ms = timings.acquire_ms + self.workload_ms + timings.present_ms;
        self.preflight_ms = timings.renderer.preflight_ms;
        self.flow_encode_ms = timings.renderer.flow_encode_ms;
        self.encode_submit_ms = timings.renderer.encode_submit_ms;
    }

    pub fn observe_pass_timings(&mut self, samples: &[PassTimingSample]) {
        self.pass_sample_count = samples.len();
        let snapshot = summarize_pass_timings(samples);
        self.total_pass_millis = snapshot.total_millis;
        self.slowest_pass_id = snapshot.slowest_pass_id;
        self.slowest_pass_millis = snapshot.slowest_pass_millis;
        self.compute_dispatches = samples
            .iter()
            .filter_map(|sample| {
                sample
                    .dispatch_workgroups
                    .map(|workgroups| ComputeDispatchSample {
                        flow_id: sample.flow_id.clone(),
                        pass_id: sample.pass_id.clone(),
                        workgroups,
                    })
            })
            .collect();
    }

    pub fn observe_gpu_pass_timing_evidence(&mut self, samples: &[RenderPassTimingEvidence]) {
        let snapshot = summarize_gpu_pass_timing_evidence(samples);
        self.gpu_timing_capability = snapshot.capability;
        self.gpu_pass_sample_count = snapshot.measured_pass_count;
        self.gpu_total_pass_millis = snapshot.total_millis;
        self.gpu_slowest_pass_id = snapshot.slowest_pass_id;
        self.gpu_slowest_pass_millis = snapshot.slowest_pass_millis;
        self.gpu_timing_diagnostics = snapshot.diagnostics;
    }

    pub fn observe_gpu_timing_diagnostic(&mut self, diagnostic: RenderGpuTimingDiagnostic) {
        self.gpu_timing_capability = diagnostic.capability;
        self.gpu_pass_sample_count = 0;
        self.gpu_total_pass_millis = 0.0;
        self.gpu_slowest_pass_id = None;
        self.gpu_slowest_pass_millis = 0.0;
        self.gpu_timing_diagnostics.push(diagnostic);
    }

    pub fn observe_world_rebuild_latency(&mut self, latency_ms: f32) {
        self.world_rebuild_latency_ms = latency_ms.max(0.0);
        self.world_rebuild_samples = self.world_rebuild_samples.saturating_add(1);
    }

    pub fn observe_world_prepare_sample(&mut self) {
        self.world_prepare_samples = self.world_prepare_samples.saturating_add(1);
    }

    pub fn observe_shader_reload_poll(&mut self, report: ShaderReloadPollReport, measured_ms: f32) {
        self.shader_reload_poll_ms = measured_ms.max(0.0);
        self.shader_reload_poll_status = Some(report.status.as_str().to_string());
        self.shader_reload_poll_interval_ms = report.interval_ms.max(0.0);
    }

    pub fn observe_diagnostics_report(&mut self, mode: impl Into<String>, measured_ms: f32) {
        self.diagnostics_report_mode = Some(mode.into());
        self.diagnostics_report_ms = measured_ms.max(0.0);
    }

    pub fn observe_preflight_cache_state(
        &mut self,
        state: &RenderPreparedFramePreflightCacheState,
    ) {
        self.preflight_cache_mode = Some(state.mode.as_str().to_string());
        self.preflight_cache_status = Some(state.status.as_str().to_string());
        self.preflight_report_source = Some(state.report_source.as_str().to_string());
    }

    pub fn observe_frame_pacing(&mut self, pacing: &FramePacingRuntimeStateResource) {
        self.frame_pacing_mode = Some(pacing.mode.as_str().to_string());
        self.frame_pacing_target_fps = pacing.mode.target_fps();
        self.frame_pacing_last_interval_ms = pacing.last_frame_interval_ms;
        self.frame_pacing_next_delay_ms = pacing.next_frame_delay_ms;
        self.frame_pacing_redraw_requested = pacing.redraw_requested;
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PassTimingSample {
    pub flow_id: String,
    pub pass_id: String,
    pub pass_kind: String,
    pub millis: f32,
    pub dispatch_workgroups: Option<[u32; 3]>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComputeDispatchSample {
    pub flow_id: String,
    pub pass_id: String,
    pub workgroups: [u32; 3],
}

#[derive(Debug, Clone, PartialEq)]
pub struct RenderFlowTimingSnapshot {
    pub total_millis: f32,
    pub slowest_pass_id: Option<String>,
    pub slowest_pass_millis: f32,
    pub per_pass: Vec<PassTimingSample>,
    pub evidence: Vec<RenderPassTimingEvidence>,
}

pub fn summarize_pass_timings(samples: &[PassTimingSample]) -> RenderFlowTimingSnapshot {
    let mut total_millis = 0.0f32;
    let mut slowest_pass_id = None::<String>;
    let mut slowest_pass_millis = 0.0f32;

    for sample in samples {
        total_millis += sample.millis.max(0.0);
        if sample.millis > slowest_pass_millis {
            slowest_pass_millis = sample.millis;
            slowest_pass_id = Some(sample.pass_id.clone());
        }
    }

    RenderFlowTimingSnapshot {
        total_millis,
        slowest_pass_id,
        slowest_pass_millis,
        per_pass: samples.to_vec(),
        evidence: samples
            .iter()
            .map(RenderPassTimingEvidence::from_cpu_sample)
            .collect(),
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RenderGpuPassTimingSnapshot {
    pub capability: RenderGpuTimingCapability,
    pub total_millis: f32,
    pub measured_pass_count: usize,
    pub slowest_pass_id: Option<String>,
    pub slowest_pass_millis: f32,
    pub per_pass: Vec<RenderPassTimingEvidence>,
    pub diagnostics: Vec<RenderGpuTimingDiagnostic>,
}

pub fn summarize_gpu_pass_timing_evidence(
    samples: &[RenderPassTimingEvidence],
) -> RenderGpuPassTimingSnapshot {
    let mut capability = RenderGpuTimingCapability::UnavailableThisFrame;
    let mut total_millis = 0.0f32;
    let mut measured_pass_count = 0usize;
    let mut slowest_pass_id = None::<String>;
    let mut slowest_pass_millis = 0.0f32;
    let mut diagnostics = Vec::<RenderGpuTimingDiagnostic>::new();

    for sample in samples {
        capability = merge_gpu_timing_capability(capability, sample.gpu_capability);
        diagnostics.extend(sample.diagnostics.iter().cloned());
        let Some(millis) = sample.millis else {
            continue;
        };
        let millis = millis.max(0.0);
        total_millis += millis;
        measured_pass_count += 1;
        if millis > slowest_pass_millis {
            slowest_pass_millis = millis;
            slowest_pass_id = Some(sample.pass_id.clone());
        }
    }

    RenderGpuPassTimingSnapshot {
        capability,
        total_millis,
        measured_pass_count,
        slowest_pass_id,
        slowest_pass_millis,
        per_pass: samples.to_vec(),
        diagnostics,
    }
}

fn merge_gpu_timing_capability(
    current: RenderGpuTimingCapability,
    next: RenderGpuTimingCapability,
) -> RenderGpuTimingCapability {
    use RenderGpuTimingCapability::{
        ReadbackPending, Supported, UnavailableThisFrame, Unsupported,
    };

    match (current, next) {
        (Unsupported, _) | (_, Unsupported) => Unsupported,
        (ReadbackPending, _) | (_, ReadbackPending) => ReadbackPending,
        (Supported, _) | (_, Supported) => Supported,
        (UnavailableThisFrame, UnavailableThisFrame) => UnavailableThisFrame,
    }
}
