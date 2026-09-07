use engine::plugins::render::inspect::{
    PassTimingSample, RenderDebugTimingsState, RenderGpuTimingCapability,
    RenderGpuTimingDiagnostic, RenderPassTimingEvidence, RenderReadinessBudgetKind,
    RenderReadinessBudgetMeasurements, RenderReadinessBudgetThreshold, RenderTimingSource,
    evaluate_render_readiness_budgets, summarize_gpu_pass_timing_evidence, summarize_pass_timings,
};
use runen_gpu::{
    GpuCapabilities, GpuCapabilityAdmission, GpuCapabilityFeature, GpuCapabilityRequirement,
    GpuCapabilityRequirements, GpuLimits,
};

#[test]
fn render_gpu_timing_admission_tracks_enabled_timestamp_query_feature() {
    let mut requirements = GpuCapabilityRequirements::new();
    requirements
        .insert(GpuCapabilityRequirement::Required(
            GpuCapabilityFeature::TimestampQuery,
        ))
        .unwrap();
    let capabilities = GpuCapabilities::from_normalized_facts(
        [GpuCapabilityFeature::TimestampQuery],
        GpuLimits::new(1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 1).unwrap(),
        [],
    );
    assert!(
        GpuCapabilityAdmission::evaluate(
            "timing",
            &requirements,
            &capabilities,
            [GpuCapabilityFeature::TimestampQuery],
        )
        .is_ok()
    );
}

#[test]
fn render_gpu_timing_summarizes_supported_timestamp_evidence() {
    let samples = vec![
        RenderPassTimingEvidence::gpu_sample(
            Some(21),
            Some(5),
            "flow.timing",
            "timing.compute",
            "compute",
            0.75,
        ),
        RenderPassTimingEvidence::gpu_sample(
            Some(21),
            Some(5),
            "flow.timing",
            "timing.compose",
            "fullscreen",
            1.25,
        ),
    ];
    let snapshot = summarize_gpu_pass_timing_evidence(&samples);

    assert_eq!(snapshot.capability, RenderGpuTimingCapability::Supported);
    assert_eq!(snapshot.measured_pass_count, 2);
    assert_eq!(snapshot.total_millis, 2.0);
    assert_eq!(snapshot.slowest_pass_id.as_deref(), Some("timing.compose"));
    assert_eq!(
        snapshot.per_pass[0].source,
        RenderTimingSource::GpuTimestampQuery
    );
    assert!(snapshot.diagnostics.is_empty());
}

#[test]
fn render_gpu_timing_reports_unsupported_and_pending_states_as_diagnostics() {
    let samples = vec![
        RenderPassTimingEvidence::gpu_diagnostic(
            Some(22),
            Some(5),
            "flow.timing",
            "timing.compute",
            "compute",
            RenderGpuTimingDiagnostic::readback_pending("timestamp resolve is still pending"),
        ),
        RenderPassTimingEvidence::gpu_diagnostic(
            Some(22),
            Some(5),
            "flow.timing",
            "timing.compose",
            "fullscreen",
            RenderGpuTimingDiagnostic::unsupported("timestamp queries are unsupported"),
        ),
    ];
    let snapshot = summarize_gpu_pass_timing_evidence(&samples);
    let mut state = RenderDebugTimingsState::default();
    state.observe_gpu_pass_timing_evidence(&samples);

    assert_eq!(snapshot.capability, RenderGpuTimingCapability::Unsupported);
    assert_eq!(snapshot.measured_pass_count, 0);
    assert_eq!(snapshot.diagnostics.len(), 2);
    assert_eq!(
        state.gpu_timing_capability,
        RenderGpuTimingCapability::Unsupported
    );
    assert_eq!(state.gpu_timing_diagnostics.len(), 2);
    assert_eq!(
        state.gpu_timing_diagnostics[1].pass_id.as_deref(),
        Some("timing.compose")
    );
}

#[test]
fn render_gpu_timing_budgets_do_not_treat_cpu_samples_as_gpu_measurements() {
    let cpu_snapshot = summarize_pass_timings(&[PassTimingSample {
        flow_id: "flow.timing".to_string(),
        pass_id: "timing.cpu".to_string(),
        pass_kind: "graphics".to_string(),
        millis: 3.0,
        dispatch_workgroups: None,
    }]);
    let mut state = RenderDebugTimingsState::default();
    state.observe_pass_timings(&cpu_snapshot.per_pass);

    let measurements =
        RenderReadinessBudgetMeasurements::from_reports(None, &[], None, &[], None, Some(&state));
    let report = evaluate_render_readiness_budgets(
        &measurements,
        &[RenderReadinessBudgetThreshold::max(
            RenderReadinessBudgetKind::GpuPassTotalMillis,
            1.0,
        )],
    );

    assert_eq!(
        cpu_snapshot.evidence[0].source,
        RenderTimingSource::CpuEncodeSubmit
    );
    assert_eq!(measurements.pass_total_ms, Some(3.0));
    assert_eq!(measurements.gpu_pass_total_ms, None);
    assert_eq!(report.not_measured_count(), 1);
}
