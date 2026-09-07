use engine::plugins::render::inspect::{
    CaptureStage, CaptureTextureClass, PassTimingSample, PreparedRenderFrameInspection,
    ProductSurfaceDiagnosticInspectionEntry, RenderCaptureIdentity, RenderCapturePointIdentity,
    RenderCaptureSelector, RenderCaptureSelectorResult, RenderDebugFrameReport,
    RenderDebugTimingsState, RenderExecutionGraphDiagnosticInspection,
    RenderExecutionGraphPreflightInspection, RenderFragmentMergeInspection,
    RenderGpuTimingCapability, RenderGpuTimingDiagnostic, RenderPassMaterialBindingEvidence,
    RenderPassProvenanceRecord, RenderPassProvenanceState, RenderPassTimingEvidence,
    RenderProductSurfaceManifestInspection, RenderReadinessBudgetKind,
    RenderReadinessBudgetMeasurements, RenderReadinessBudgetStatus, RenderReadinessBudgetThreshold,
    RenderReadinessDiagnosticKind, RenderReadinessDiagnosticSeverity, RenderReadinessReportRequest,
    RenderReplayArtifactReference, RenderReplayManifest, RenderReplayManifestStatus,
    RenderSelectorResolution, RenderTimingSource, ResolvedRenderCapturePlan,
    ResolvedRenderCaptureSelector, deterministic_capture_filename,
    evaluate_render_readiness_budgets, inspect_compiled_render_flow_plan,
    inspect_fragment_pass_provenance, inspect_prepared_render_frame,
    inspect_render_execution_graph_preflight, inspect_render_execution_graph_preflight_with_cache,
    inspect_render_fragment_merge_report, inspect_render_gpu_residency,
    inspect_render_product_surface_manifest, inspect_render_readiness, inspect_resources,
    inspect_texture_resources, resource_kind_name, summarize_gpu_pass_timing_evidence,
    summarize_pass_timings, validate_render_replay_manifest,
};
use engine::plugins::render::pipelines::FlowPassKind;
use engine::plugins::render::{
    FeatureContributionStatus, FeatureFallbackPolicy, GfxFrameTimings, PreparedFlowInputs,
    PreparedFlowInvocation, PreparedFlowInvocationId, PreparedFlowInvocationRequest,
    PreparedFrameContext, PreparedFrameContributions, PreparedRegisteredFeaturePayload,
    PreparedRenderFrame, PreparedShaderSnapshot, PreparedSurfaceInfo, PreparedTargetBinding,
    PreparedViewFrame, RenderDynamicTextureRetention, RenderDynamicTextureTargetDescriptor,
    RenderDynamicTextureTargetKey, RenderDynamicTextureUploadDescriptor, RenderFeatureId,
    RenderFlow, RenderFragmentDescriptor, RenderFragmentPackageDescriptor,
    RenderFragmentPassDescriptor, RenderFragmentResourceDescriptor, RenderFrameProducerId,
    RenderGpuResidencyBudgetResource, RenderGpuResidencyResource,
    RenderPreparedFramePreflightCacheState, RenderPreparedFramePreflightCacheStatus,
    RenderPreparedFramePreflightMode, RenderPreparedFramePreflightReportSource,
    RenderProductSurfaceManifest, RenderProductSurfaceRequest, RenderProductSurfaceRequestBatch,
    RenderProductSurfaceStatusKind, RenderResourceDeclaration, RenderTargetAliasKey,
    RenderTextureSampleMode, RenderTextureTargetFormat, RenderTextureUploadAlphaMode,
    RendererFrameTimings, ShaderReloadPollReport, ShaderReloadPollStatus,
    StaticRegisteredFeaturePayload, compile_flow_plan, current_runtime_gpu_capabilities,
    merge_fragment_package_into_flow, validate_prepared_render_frame,
};
use engine::runtime::{FramePacingPolicyResource, FramePacingRuntimeStateResource};
use product::{
    ProductAuthorityClass, ProductFreshness, ProductIdentity, ProductQueryPolicy, ProductResidency,
    ProductScaleBand, RenderProductSelection, RenderResidencyRequest, RenderSelectedProduct,
    RenderTargetDescriptor,
};
use runen_gpu::{
    GpuCapabilityFeature, GpuPrimitiveTopology, GpuTextureFormat, GpuWorkResourceId,
    GpuWorkResourceIdAllocator,
};
use std::collections::BTreeMap;
use std::path::PathBuf;
use ui_render_data::{ProductSurfaceTextureBindingSource, ViewportSurfaceBindingRegistry};

#[derive(Debug, Clone, Copy, engine::plugins::render::GpuStorage)]
struct InspectStorage {
    value: u32,
}

fn test_resource_ids(count: usize) -> Vec<GpuWorkResourceId> {
    let labels = (0..count)
        .map(|index| format!("test.inspect.resource.{index}"))
        .collect::<Vec<_>>();
    let flow = labels
        .iter()
        .fold(RenderFlow::new("test.inspect.resources"), |flow, label| {
            flow.with_color_target(label.clone())
                .expect("render flow authoring should succeed")
        });
    labels
        .iter()
        .map(|label| flow.resource_id(label).expect("test resource should exist"))
        .collect()
}

fn alias_key(value: &str) -> RenderTargetAliasKey {
    RenderTargetAliasKey::new(value).expect("test alias key should be valid")
}

#[test]
fn render_runtime_inspect_runtime_timing_snapshot_preserves_flow_pass_kind_and_dispatch_metadata() {
    let samples = vec![
        PassTimingSample {
            flow_id: "flow.a".to_string(),
            pass_id: "a.compute".to_string(),
            pass_kind: "compute".to_string(),
            millis: 0.6,
            dispatch_workgroups: Some([20, 12, 1]),
        },
        PassTimingSample {
            flow_id: "flow.a".to_string(),
            pass_id: "a.compose".to_string(),
            pass_kind: "fullscreen".to_string(),
            millis: 1.1,
            dispatch_workgroups: None,
        },
    ];
    let snapshot = summarize_pass_timings(&samples);

    assert_eq!(snapshot.per_pass.len(), 2);
    assert_eq!(snapshot.per_pass[0].flow_id, "flow.a");
    assert_eq!(snapshot.per_pass[0].pass_kind, "compute");
    assert_eq!(snapshot.per_pass[0].dispatch_workgroups, Some([20, 12, 1]));
    assert_eq!(snapshot.slowest_pass_id.as_deref(), Some("a.compose"));
}

#[test]
fn render_runtime_inspect_debug_timing_state_extracts_compute_dispatch_samples() {
    let mut state = RenderDebugTimingsState::default();
    state.observe_pass_timings(&[
        PassTimingSample {
            flow_id: "flow.a".to_string(),
            pass_id: "a.compute".to_string(),
            pass_kind: "compute".to_string(),
            millis: 0.6,
            dispatch_workgroups: Some([10, 4, 1]),
        },
        PassTimingSample {
            flow_id: "flow.a".to_string(),
            pass_id: "a.compose".to_string(),
            pass_kind: "fullscreen".to_string(),
            millis: 0.2,
            dispatch_workgroups: None,
        },
    ]);

    assert_eq!(state.pass_sample_count, 2);
    assert_eq!(state.compute_dispatches.len(), 1);
    assert_eq!(state.compute_dispatches[0].flow_id, "flow.a");
    assert_eq!(state.compute_dispatches[0].pass_id, "a.compute");
    assert_eq!(state.compute_dispatches[0].workgroups, [10, 4, 1]);
}

#[test]
fn render_runtime_inspect_gpu_timing_evidence_keeps_cpu_and_gpu_sources_separate() {
    let cpu_samples = vec![PassTimingSample {
        flow_id: "flow.gpu".to_string(),
        pass_id: "gpu.compute".to_string(),
        pass_kind: "compute".to_string(),
        millis: 0.4,
        dispatch_workgroups: Some([4, 2, 1]),
    }];
    let cpu_snapshot = summarize_pass_timings(&cpu_samples);
    let gpu_samples = vec![RenderPassTimingEvidence::gpu_sample(
        Some(9),
        Some(3),
        "flow.gpu",
        "gpu.compute",
        "compute",
        1.25,
    )];
    let gpu_snapshot = summarize_gpu_pass_timing_evidence(&gpu_samples);

    assert_eq!(
        cpu_snapshot.evidence[0].source,
        RenderTimingSource::CpuEncodeSubmit
    );
    assert_eq!(
        cpu_snapshot.evidence[0].gpu_capability,
        RenderGpuTimingCapability::UnavailableThisFrame
    );
    assert_eq!(
        gpu_snapshot.capability,
        RenderGpuTimingCapability::Supported
    );
    assert_eq!(gpu_snapshot.measured_pass_count, 1);
    assert_eq!(gpu_snapshot.total_millis, 1.25);
    assert_eq!(gpu_snapshot.slowest_pass_id.as_deref(), Some("gpu.compute"));
    assert_eq!(
        gpu_snapshot.per_pass[0].source,
        RenderTimingSource::GpuTimestampQuery
    );
}

#[test]
fn render_runtime_inspect_gpu_timing_state_reports_unsupported_and_readback_pending() {
    let unsupported = RenderPassTimingEvidence::gpu_diagnostic(
        Some(7),
        Some(2),
        "flow.gpu",
        "gpu.compose",
        "fullscreen",
        RenderGpuTimingDiagnostic::unsupported(
            "timestamp queries are not supported by the active backend",
        ),
    );
    let pending = RenderPassTimingEvidence::gpu_diagnostic(
        Some(8),
        Some(2),
        "flow.gpu",
        "gpu.compute",
        "compute",
        RenderGpuTimingDiagnostic::readback_pending("timestamp readback is pending"),
    );
    let mut state = RenderDebugTimingsState::default();
    state.observe_gpu_pass_timing_evidence(&[unsupported, pending]);

    assert_eq!(
        state.gpu_timing_capability,
        RenderGpuTimingCapability::Unsupported
    );
    assert_eq!(state.gpu_pass_sample_count, 0);
    assert_eq!(state.gpu_total_pass_millis, 0.0);
    assert_eq!(state.gpu_timing_diagnostics.len(), 2);
    assert_eq!(
        state.gpu_timing_diagnostics[0].flow_id.as_deref(),
        Some("flow.gpu")
    );
    assert_eq!(
        state.gpu_timing_diagnostics[0].pass_id.as_deref(),
        Some("gpu.compose")
    );
}

#[test]
fn render_runtime_inspect_debug_timing_state_includes_preflight_and_flow_encode() {
    let mut state = RenderDebugTimingsState::default();

    state.observe_frame_timings(GfxFrameTimings {
        acquire_ms: 0.5,
        renderer: RendererFrameTimings {
            prepare_ui_ms: 1.0,
            prepare_mesh_ms: 2.0,
            world_prepare_ms: 3.0,
            preflight_ms: 4.0,
            flow_encode_ms: 5.0,
            encode_submit_ms: 6.0,
            ..RendererFrameTimings::default()
        },
        present_ms: 0.25,
    });

    assert_eq!(state.preflight_ms, 4.0);
    assert_eq!(state.flow_encode_ms, 5.0);
    assert_eq!(state.encode_submit_ms, 6.0);
    assert_eq!(state.workload_ms, 21.0);
    assert_eq!(state.total_ms, 21.75);
}

#[test]
fn render_runtime_inspect_debug_timing_state_exposes_steady_state_runtime_costs() {
    let mut state = RenderDebugTimingsState::default();
    state.observe_shader_reload_poll(
        ShaderReloadPollReport {
            status: ShaderReloadPollStatus::Throttled,
            elapsed_ms: 100.0,
            interval_ms: 500.0,
            force_reload: false,
        },
        0.05,
    );
    state.observe_diagnostics_report("lightweight", 0.02);
    state.observe_preflight_cache_state(&RenderPreparedFramePreflightCacheState {
        mode: RenderPreparedFramePreflightMode::CachedStrict,
        status: RenderPreparedFramePreflightCacheStatus::Hit,
        report_source: RenderPreparedFramePreflightReportSource::CachedReport,
        cache_key: None,
    });
    let mut pacing = FramePacingRuntimeStateResource::default();
    pacing.observe_policy(FramePacingPolicyResource::continuous_capped(60));
    state.observe_frame_pacing(&pacing);

    assert_eq!(
        state.shader_reload_poll_status.as_deref(),
        Some("throttled")
    );
    assert_eq!(state.shader_reload_poll_interval_ms, 500.0);
    assert_eq!(
        state.diagnostics_report_mode.as_deref(),
        Some("lightweight")
    );
    assert_eq!(state.preflight_cache_status.as_deref(), Some("hit"));
    assert_eq!(
        state.preflight_report_source.as_deref(),
        Some("cached_report")
    );
    assert_eq!(
        state.frame_pacing_mode.as_deref(),
        Some("continuous_capped")
    );
    assert_eq!(state.frame_pacing_target_fps, Some(60));
}

#[test]
fn render_runtime_inspect_budget_report_flags_renderer_evidence_without_product_policy() {
    let measurements = RenderReadinessBudgetMeasurements {
        frame_total_ms: Some(18.0),
        capture_failure_count: Some(1.0),
        preflight_error_count: Some(0.0),
        ..RenderReadinessBudgetMeasurements::default()
    };
    let report = evaluate_render_readiness_budgets(
        &measurements,
        &[
            RenderReadinessBudgetThreshold::max(RenderReadinessBudgetKind::FrameTotalMillis, 16.0),
            RenderReadinessBudgetThreshold::max(
                RenderReadinessBudgetKind::CaptureFailureCount,
                0.0,
            ),
            RenderReadinessBudgetThreshold::max(
                RenderReadinessBudgetKind::DynamicTextureUploadBytes,
                4096.0,
            ),
        ],
    );

    assert_eq!(report.results.len(), 3);
    assert_eq!(report.over_budget_count(), 2);
    assert_eq!(report.not_measured_count(), 1);
    assert_eq!(
        report.results[0].status,
        RenderReadinessBudgetStatus::OverBudget
    );
    assert_eq!(
        report.results[2].status,
        RenderReadinessBudgetStatus::NotMeasured
    );
}

#[test]
fn render_runtime_inspect_budget_measurements_report_preflight_cost() {
    let timings = RenderDebugTimingsState {
        preflight_ms: 7.5,
        total_ms: 20.0,
        ..RenderDebugTimingsState::default()
    };
    let measurements =
        RenderReadinessBudgetMeasurements::from_reports(None, &[], None, &[], None, Some(&timings));
    let report = evaluate_render_readiness_budgets(
        &measurements,
        &[RenderReadinessBudgetThreshold::max(
            RenderReadinessBudgetKind::PreflightMillis,
            4.0,
        )],
    );

    assert_eq!(measurements.preflight_ms, Some(7.5));
    assert_eq!(report.over_budget_count(), 1);
    assert_eq!(
        report.results[0].kind,
        RenderReadinessBudgetKind::PreflightMillis
    );
}

#[test]
fn render_runtime_inspect_readiness_budgets_consume_gpu_timing_evidence() {
    let mut timings = RenderDebugTimingsState::default();
    timings.observe_gpu_pass_timing_evidence(&[
        RenderPassTimingEvidence::gpu_sample(
            Some(15),
            Some(4),
            "flow.gpu",
            "gpu.compute",
            "compute",
            3.5,
        ),
        RenderPassTimingEvidence::gpu_diagnostic(
            Some(15),
            Some(4),
            "flow.gpu",
            "gpu.compose",
            "fullscreen",
            RenderGpuTimingDiagnostic::readback_pending("timestamp readback is pending"),
        ),
    ]);
    let measurements =
        RenderReadinessBudgetMeasurements::from_reports(None, &[], None, &[], None, Some(&timings));
    let budget_report = evaluate_render_readiness_budgets(
        &measurements,
        &[
            RenderReadinessBudgetThreshold::max(RenderReadinessBudgetKind::GpuPassTotalMillis, 2.0),
            RenderReadinessBudgetThreshold::max(
                RenderReadinessBudgetKind::GpuTimingDiagnosticCount,
                0.0,
            ),
        ],
    );
    let report = inspect_render_readiness(RenderReadinessReportRequest {
        timings: Some(timings),
        budget_report,
        ..RenderReadinessReportRequest::default()
    });

    assert_eq!(measurements.gpu_pass_total_ms, Some(3.5));
    assert_eq!(measurements.gpu_timing_diagnostic_count, Some(1.0));
    assert_eq!(report.source_reports.gpu_pass_sample_count, 1);
    assert_eq!(report.source_reports.gpu_timing_diagnostic_count, 1);
    assert_eq!(report.budget_report.over_budget_count(), 2);
    assert!(report.diagnostics.iter().any(|diagnostic| {
        diagnostic.kind == RenderReadinessDiagnosticKind::GpuTimingDiagnostics
            && diagnostic.flow_id.as_deref() == Some("flow.gpu")
            && diagnostic.pass_id.as_deref() == Some("gpu.compose")
    }));
}

#[test]
fn render_runtime_inspect_replay_manifest_validation_fails_closed() {
    let capture_point = RenderCapturePointIdentity {
        flow_id: "flow.readiness".to_string(),
        pass_id: "compose".to_string(),
        stage: CaptureStage::After,
        resource_id: "surface.color".to_string(),
        texture_class: CaptureTextureClass::ImportedTexture,
    };
    let invalid = RenderReplayManifest::new("readiness.invalid", 3).with_artifact(
        RenderReplayArtifactReference {
            capture_point: capture_point.clone(),
            artifact_path: None,
            format: None,
        },
    );

    let validation = validate_render_replay_manifest(&invalid);

    assert_eq!(validation.status, RenderReplayManifestStatus::Invalid);
    assert!(validation.diagnostics.iter().any(|diagnostic| {
        diagnostic.kind == RenderReadinessDiagnosticKind::CaptureReplayManifestInvalid
            && diagnostic.capture_point.as_ref() == Some(&capture_point)
    }));
    assert!(
        validation
            .diagnostics
            .iter()
            .all(|diagnostic| { diagnostic.severity == RenderReadinessDiagnosticSeverity::Error })
    );

    let valid = RenderReplayManifest::new("readiness.valid", 4)
        .with_capability_profile("wgpu-portable-v1")
        .with_prepared_frame_digest("prepared-frame:sha256:test")
        .with_artifact(RenderReplayArtifactReference {
            capture_point,
            artifact_path: Some(PathBuf::from("target/render-debug/frame.png")),
            format: Some("Rgba8Unorm".to_string()),
        });

    assert_eq!(
        validate_render_replay_manifest(&valid).status,
        RenderReplayManifestStatus::Valid
    );
}

#[test]
fn render_runtime_inspect_readiness_report_aggregates_existing_source_reports() {
    let prepared = PreparedRenderFrameInspection {
        frame_index: 12,
        prepare_epoch: 2,
        render_surface_id: 1,
        native_window_id: Some(1),
        surface_size: (1280, 720),
        views: Vec::new(),
        flow_invocations: Vec::new(),
        feature_contributions: Vec::new(),
        dynamic_texture_targets: Vec::new(),
        product_selections: Vec::new(),
    };
    let preflight = RenderExecutionGraphPreflightInspection {
        diagnostic_count: 1,
        error_count: 1,
        cache_mode: Some("cached_strict".to_string()),
        cache_status: Some("hit".to_string()),
        report_source: Some("cached_report".to_string()),
        diagnostics: vec![RenderExecutionGraphDiagnosticInspection {
            severity: "Error".to_string(),
            kind: "TargetAliasMissingBinding".to_string(),
            flow_id: Some("flow.readiness".to_string()),
            flow_label: Some("readiness.flow".to_string()),
            pass_id: Some("compose".to_string()),
            pass_label: Some("compose".to_string()),
            resource_id: None,
            resource_label: None,
            invocation_id: Some("viewport.scene".to_string()),
            view_id: Some("viewport".to_string()),
            alias_binding_key: Some("scene_color".to_string()),
            alias_kind: Some("Color".to_string()),
            dynamic_target_key: None,
            history_signature: None,
            capability: None,
            message: "target alias is missing a binding".to_string(),
        }],
    };
    let product_surface = RenderProductSurfaceManifestInspection {
        producer_id: 44,
        product_family: "readiness.product".to_string(),
        dynamic_texture_targets: Vec::new(),
        dynamic_texture_uploads: Vec::new(),
        prepared_views: Vec::new(),
        flow_invocations: Vec::new(),
        product_bindings: Vec::new(),
        viewport_bindings: Vec::new(),
        statuses: Vec::new(),
        diagnostics: vec![ProductSurfaceDiagnosticInspectionEntry {
            producer_id: 44,
            product_family: "readiness.product".to_string(),
            surface_key: Some("surface.stale".to_string()),
            dynamic_target_key: None,
            view_id: Some("viewport".to_string()),
            invocation_id: None,
            request_kind: "product_status".to_string(),
            severity: "warning".to_string(),
            diagnostic_kind: "producer_status".to_string(),
            status: Some("stale".to_string()),
            message: "producer reported stale surface".to_string(),
        }],
    };
    let fragment = RenderFragmentMergeInspection {
        package_id: Some("readiness.fragments".to_string()),
        source_path: Some("render/fragments/readiness.ron".to_string()),
        source_revision: Some(2),
        generated_flow_id: Some("flow.readiness".to_string()),
        provenance_count: 0,
        error_count: 1,
        lines: vec!["diagnostic: Error MissingLocalResource missing target".to_string()],
    };
    let selector = RenderCaptureSelector::named_pass_surface_color("flow.readiness", "compose");
    let capture_reason = engine::plugins::render::inspect::RenderCaptureTerminalReason::new(
        "selector_unmatched",
        "selector matched no capture point in this frame",
    );
    let capture_report = RenderDebugFrameReport {
        frame_index: 12,
        capture_plan: ResolvedRenderCapturePlan {
            frame_index: 12,
            selectors: vec![ResolvedRenderCaptureSelector {
                selector_index: 0,
                selector: selector.clone(),
                resolution: RenderSelectorResolution::Unmatched {
                    reason: capture_reason.clone(),
                },
            }],
        },
        capture_results: vec![RenderCaptureSelectorResult::for_unmatched(0, selector)],
        ..RenderDebugFrameReport::default()
    };
    let budget_report = evaluate_render_readiness_budgets(
        &RenderReadinessBudgetMeasurements {
            frame_total_ms: Some(18.0),
            ..RenderReadinessBudgetMeasurements::default()
        },
        &[RenderReadinessBudgetThreshold::max(
            RenderReadinessBudgetKind::FrameTotalMillis,
            16.0,
        )],
    );
    let replay_validation =
        validate_render_replay_manifest(&RenderReplayManifest::new("readiness.replay.invalid", 12));

    let report = inspect_render_readiness(RenderReadinessReportRequest {
        prepared_frame: Some(prepared),
        product_surfaces: vec![product_surface],
        preflight: Some(preflight),
        fragment_merges: vec![fragment],
        capture_report: Some(capture_report),
        timings: None,
        budget_report,
        replay_validation: Some(replay_validation),
    });

    assert_eq!(report.frame_index, Some(12));
    assert_eq!(report.render_surface_id, Some(1));
    assert_eq!(report.native_window_id, Some(1));
    assert_eq!(report.source_reports.product_surface_diagnostic_count, 1);
    assert_eq!(report.source_reports.preflight_error_count, 1);
    assert_eq!(report.source_reports.fragment_error_count, 1);
    assert_eq!(report.source_reports.capture_failure_count, 1);
    assert_eq!(report.source_reports.budget_overrun_count, 1);
    assert!(!report.is_ready());
    assert!(report.diagnostics.iter().any(|diagnostic| {
        diagnostic.kind == RenderReadinessDiagnosticKind::PreflightErrors
            && diagnostic.flow_id.as_deref() == Some("flow.readiness")
    }));
    assert!(report.diagnostics.iter().any(|diagnostic| {
        diagnostic.kind == RenderReadinessDiagnosticKind::ProductSurfaceDiagnostics
            && diagnostic.producer_id == Some(44)
    }));
    assert!(report.diagnostics.iter().any(|diagnostic| {
        diagnostic.kind == RenderReadinessDiagnosticKind::FragmentErrors
            && diagnostic.fragment_package_id.as_deref() == Some("readiness.fragments")
    }));
    assert!(report.diagnostics.iter().any(|diagnostic| {
        diagnostic.kind == RenderReadinessDiagnosticKind::CaptureFailures
            && diagnostic.pass_id.as_deref() == Some("compose")
    }));
    assert!(report.diagnostics.iter().any(|diagnostic| {
        diagnostic.kind == RenderReadinessDiagnosticKind::BudgetOverrun
            && diagnostic.budget_kind.as_deref() == Some("frame_total_ms")
    }));
    assert!(report.diagnostics.iter().any(|diagnostic| {
        diagnostic.kind == RenderReadinessDiagnosticKind::CaptureReplayManifestInvalid
    }));
}

#[test]
fn render_runtime_inspect_resource_kind_label_matches_descriptor_kind() {
    let mut allocator = GpuWorkResourceIdAllocator::new();
    let descriptor = RenderResourceDeclaration::declare_storage::<InspectStorage>(
        &mut allocator,
        "inspect storage",
    )
    .expect("storage declaration should be valid");
    assert_eq!(resource_kind_name(&descriptor), "storage_buffer");
}

#[test]
fn render_runtime_inspect_resource_inspection_exposes_target_alias_metadata() {
    let flow = RenderFlow::new("inspect.alias")
        .with_color_target_alias("viewport.scene_color")
        .expect("render flow authoring should succeed")
        .with_depth_target_alias("viewport.depth")
        .expect("render flow authoring should succeed");

    let resources = inspect_resources(&flow);
    let color_alias = resources
        .iter()
        .find(|entry| entry.target_alias_binding_key.as_deref() == Some("viewport.scene_color"))
        .expect("color target alias should be inspectable");

    assert_eq!(color_alias.kind, "target_alias(color)");
    assert_eq!(color_alias.target_alias_kind.as_deref(), Some("color"));

    let textures = inspect_texture_resources(&flow);
    assert!(textures.iter().any(|entry| {
        entry.category == "target_alias(depth)"
            && entry.target_alias_binding_key.as_deref() == Some("viewport.depth")
            && entry.target_alias_kind.as_deref() == Some("depth")
    }));
}

#[test]
fn render_runtime_inspect_compiler_plan_and_preflight_reports_are_structured() {
    let flow = RenderFlow::new("inspect.compiler")
        .with_color_target_alias("scene_color")
        .expect("render flow authoring should succeed")
        .fullscreen_pass("compose")
        .offscreen_products_only()
        .write_target_alias("scene_color")
        .finish()
        .validate()
        .expect("flow should validate");
    let compiled = compile_flow_plan(&flow).expect("flow should compile");
    let plan_inspection = inspect_compiled_render_flow_plan(&compiled);

    assert_eq!(plan_inspection.flow_label, "inspect.compiler");
    assert_eq!(plan_inspection.pass_count, 1);
    assert!(
        plan_inspection
            .backend_capabilities
            .supports(GpuCapabilityFeature::RenderPipeline)
    );
    assert_eq!(
        plan_inspection
            .backend_capabilities
            .limits()
            .max_bindings_per_group(),
        16
    );

    let frame = PreparedRenderFrame {
        context: PreparedFrameContext {
            frame_index: 7,
            flow_registry_revision: 1,
            shader_registry_revision: 1,
            prepare_epoch: 1,
        },
        surface: PreparedSurfaceInfo::primary((800, 600)),
        views: vec![PreparedViewFrame::offscreen_product(
            "viewport.1",
            (320, 180),
        )],
        flows: BTreeMap::new(),
        flow_invocations: vec![PreparedFlowInvocation {
            invocation_id: PreparedFlowInvocationId::new("viewport.1.scene"),
            flow_id: compiled.flow_id,
            view_id: "viewport.1".to_string(),
            inputs: PreparedFlowInputs::default(),
            target_alias_bindings: BTreeMap::new(),
            history_signature: None,
        }],
        dynamic_texture_targets: Vec::new(),
        dynamic_texture_uploads: Vec::new(),
        product_selections: Vec::new(),
        viewport_surface_bindings: ViewportSurfaceBindingRegistry::default(),
        contributions: PreparedFrameContributions::default(),
        shader: PreparedShaderSnapshot {
            registry_revision: 1,
        },
    };
    let report =
        validate_prepared_render_frame(&frame, &[compiled], &current_runtime_gpu_capabilities());
    let preflight = inspect_render_execution_graph_preflight(&report);

    assert_eq!(preflight.error_count, 1);
    assert_eq!(preflight.cache_mode, None);
    assert_eq!(preflight.diagnostics[0].kind, "TargetAliasMissingBinding");
    assert_eq!(
        preflight.diagnostics[0].alias_binding_key.as_deref(),
        Some("scene_color")
    );
    assert_eq!(
        preflight.diagnostics[0].view_id.as_deref(),
        Some("viewport.1")
    );
}

#[test]
fn render_runtime_inspect_preflight_cache_state_is_reported_without_backend_handles() {
    let report = engine::plugins::render::RenderExecutionGraphPreparedReport::default();
    let cache_state = RenderPreparedFramePreflightCacheState {
        mode: RenderPreparedFramePreflightMode::CachedStrict,
        status: RenderPreparedFramePreflightCacheStatus::Hit,
        report_source: RenderPreparedFramePreflightReportSource::CachedReport,
        cache_key: None,
    };

    let inspection =
        inspect_render_execution_graph_preflight_with_cache(&report, Some(&cache_state));

    assert_eq!(inspection.cache_mode.as_deref(), Some("cached_strict"));
    assert_eq!(inspection.cache_status.as_deref(), Some("hit"));
    assert_eq!(inspection.report_source.as_deref(), Some("cached_report"));
    assert_eq!(inspection.error_count, 0);
}

#[test]
fn render_runtime_inspect_fragment_merge_reports_expose_package_revision_and_pass_provenance() {
    let package = RenderFragmentPackageDescriptor::new(
        "inspect.fragments",
        "inspect_frag",
        "render/fragments/inspect.ron",
        5,
    )
    .with_fragment(
        RenderFragmentDescriptor::new("compose", "inspect_frag")
            .with_resource(RenderFragmentResourceDescriptor::color_target_exact(
                "scene",
                RenderTextureTargetFormat::Rgba8Unorm,
            ))
            .with_pass(
                RenderFragmentPassDescriptor::fullscreen("compose")
                    .shader_asset("assets/shaders/inspect_fragment.wgsl")
                    .write_local_color_target("scene"),
            ),
    );
    let merged = merge_fragment_package_into_flow(
        RenderFlow::new("inspect.fragment.flow"),
        &package,
        &current_runtime_gpu_capabilities(),
    )
    .expect("valid fragment package should merge");

    let inspection = inspect_render_fragment_merge_report(&merged.report);
    assert_eq!(inspection.package_id.as_deref(), Some("inspect.fragments"));
    assert_eq!(inspection.source_revision, Some(5));
    assert_eq!(inspection.error_count, 0);
    assert!(
        inspection
            .lines
            .iter()
            .any(|line| { line.contains("inspect_frag::compose") && line.contains("compose") })
    );

    let pass_provenance = inspect_fragment_pass_provenance(&merged.report);
    assert_eq!(pass_provenance.len(), 1);
    assert_eq!(pass_provenance[0].package_id, "inspect.fragments");
    assert_eq!(pass_provenance[0].generated_label, "inspect_frag::compose");
}

#[test]
fn render_runtime_inspect_prepared_frame_inspection_exposes_render_product_selection_targets_views_invocations_and_history()
 {
    let target_key = RenderDynamicTextureTargetKey::new("editor.viewport.1", "scene_color");
    let flow_id = engine::plugins::render::RenderFlowId::try_from_raw(3).unwrap();
    let resource_id = test_resource_ids(1)[0];
    let product_surface_request = RenderProductSurfaceRequest::new(
        PreparedViewFrame::offscreen_product("viewport.1", (640, 360))
            .with_history_signature("camera:v1"),
        PreparedFlowInvocationRequest::new("viewport.1.scene", flow_id, "viewport.1")
            .bind_dynamic_texture_alias("viewport.scene_color", target_key.clone())
            .expect("test alias key should be valid")
            .bind_flow_owned_alias("viewport.fallback", resource_id)
            .expect("test alias key should be valid")
            .with_history_signature("camera:v1"),
    )
    .with_dynamic_target(RenderDynamicTextureTargetDescriptor::color_sampled(
        target_key.clone(),
        640,
        360,
        RenderTextureTargetFormat::Rgba8Unorm,
        RenderTextureSampleMode::FilterableFloat,
        RenderDynamicTextureRetention::RetainForFrames(2),
    ));
    let (dynamic_texture_targets, mut helper_views, helper_requests) =
        RenderProductSurfaceRequestBatch::from_request(product_surface_request).into_parts();
    let helper_request = helper_requests
        .into_iter()
        .next()
        .expect("helper should build one invocation request");
    let helper_invocation = PreparedFlowInvocation {
        invocation_id: helper_request.invocation_id,
        flow_id: helper_request.flow_id,
        view_id: helper_request.view_id,
        inputs: PreparedFlowInputs::default(),
        target_alias_bindings: helper_request.target_alias_bindings,
        history_signature: helper_request.history_signature,
    };

    let frame = PreparedRenderFrame {
        context: PreparedFrameContext {
            frame_index: 42,
            flow_registry_revision: 1,
            shader_registry_revision: 2,
            prepare_epoch: 5,
        },
        surface: PreparedSurfaceInfo::primary((1920, 1080)),
        views: {
            let mut views = vec![PreparedViewFrame::main((1920, 1080))];
            views.append(&mut helper_views);
            views
        },
        flows: BTreeMap::new(),
        flow_invocations: vec![helper_invocation],
        dynamic_texture_targets,
        dynamic_texture_uploads: Vec::new(),
        product_selections: vec![
            RenderProductSelection::new("viewport.1")
                .with_selected_product(RenderSelectedProduct {
                    product_id: ProductIdentity::new(77),
                    scale_band: ProductScaleBand::Preview,
                    generation: 12,
                    freshness: ProductFreshness::Current,
                    residency: ProductResidency::Resident,
                    authority_class: ProductAuthorityClass::DeterministicDerived,
                    query_policy: ProductQueryPolicy::StrictCurrentOnly,
                })
                .with_required_target(RenderTargetDescriptor::new(
                    "editor.viewport.1:scene_color",
                    640,
                    360,
                    "rgba8_unorm",
                ))
                .with_residency_request(RenderResidencyRequest::new(
                    ProductIdentity::new(77),
                    ProductResidency::Resident,
                    100,
                    true,
                )),
        ],
        viewport_surface_bindings: ViewportSurfaceBindingRegistry::default(),
        contributions: PreparedFrameContributions::default(),
        shader: PreparedShaderSnapshot {
            registry_revision: 2,
        },
    };

    let inspection = inspect_prepared_render_frame(&frame);
    let history_signatures = frame
        .dynamic_target_history_signatures()
        .expect("prepared frame history signatures should be coherent");

    assert_eq!(inspection.frame_index, 42);
    assert_eq!(
        history_signatures.get(&target_key).map(String::as_str),
        Some("camera:v1")
    );
    assert_eq!(inspection.views.len(), 2);
    assert_eq!(inspection.views[1].kind, "offscreen_product");
    assert_eq!(
        inspection.views[1].history_signature.as_deref(),
        Some("camera:v1")
    );
    assert_eq!(
        inspection.dynamic_texture_targets[0].key,
        "editor.viewport.1:scene_color"
    );
    assert_eq!(
        inspection.dynamic_texture_targets[0].retention,
        "retain_for_frames(2)"
    );
    assert!(inspection.dynamic_texture_targets[0].displayable);
    assert!(inspection.dynamic_texture_targets[0].sampleable);
    assert_eq!(
        inspection.flow_invocations[0].invocation_id,
        "viewport.1.scene"
    );
    assert_eq!(
        inspection.flow_invocations[0].history_signature.as_deref(),
        Some("camera:v1")
    );
    assert!(
        inspection.flow_invocations[0]
            .target_alias_bindings
            .iter()
            .any(|binding| {
                binding.binding_key == "viewport.scene_color"
                    && binding.binding == "dynamic_texture(editor.viewport.1:scene_color)"
            })
    );
    assert_eq!(inspection.product_selections.len(), 1);
    assert_eq!(inspection.product_selections[0].view_id, "viewport.1");
    assert_eq!(
        inspection.product_selections[0].selected_products[0].product_id,
        77
    );
    assert_eq!(
        inspection.product_selections[0].residency_requests[0].priority,
        100
    );
}

#[test]
fn render_runtime_inspect_product_surface_manifest_exposes_upload_binding_status_and_diagnostics() {
    let producer_id = RenderFrameProducerId::try_from_raw(44).unwrap();
    let target_key = RenderDynamicTextureTargetKey::new("inspect.product", "surface");
    let target = RenderDynamicTextureTargetDescriptor::color_sampled(
        target_key.clone(),
        8,
        4,
        RenderTextureTargetFormat::Rgba8Unorm,
        RenderTextureSampleMode::FilterableFloat,
        RenderDynamicTextureRetention::RetainForFrames(3),
    );
    let upload = RenderDynamicTextureUploadDescriptor::rgba8(
        target_key.clone(),
        0,
        0,
        8,
        4,
        RenderTextureUploadAlphaMode::Straight,
        12,
        vec![255; 8 * 4 * 4],
    );
    let missing_upload_key = RenderDynamicTextureTargetKey::new("inspect.product", "missing");
    let manifest = RenderProductSurfaceManifest::new(producer_id, "inspect.product")
        .with_dynamic_target(target)
        .with_dynamic_upload(upload)
        .with_upload_backed_product_surface_binding(
            "surface.ready",
            ProductSurfaceTextureBindingSource::dynamic_texture(
                target_key.namespace.clone(),
                target_key.target_id.clone(),
            ),
        )
        .with_dynamic_target(RenderDynamicTextureTargetDescriptor::color_sampled(
            missing_upload_key.clone(),
            2,
            2,
            RenderTextureTargetFormat::Rgba8Unorm,
            RenderTextureSampleMode::FilterableFloat,
            RenderDynamicTextureRetention::RetainWhileRequested,
        ))
        .with_upload_backed_product_surface_binding(
            "surface.missing_upload",
            ProductSurfaceTextureBindingSource::dynamic_texture(
                missing_upload_key.namespace,
                missing_upload_key.target_id,
            ),
        )
        .with_status(
            "surface.stale",
            RenderProductSurfaceStatusKind::Stale,
            "producer owns stale surface decision",
        );

    let inspection = inspect_render_product_surface_manifest(&manifest);

    assert_eq!(inspection.producer_id, 44);
    assert_eq!(inspection.product_family, "inspect.product");
    assert_eq!(inspection.dynamic_texture_targets.len(), 2);
    assert_eq!(inspection.dynamic_texture_uploads.len(), 1);
    assert_eq!(inspection.dynamic_texture_uploads[0].product_generation, 12);
    assert_eq!(inspection.product_bindings.len(), 2);
    assert!(inspection.product_bindings[0].upload_required);
    assert!(
        inspection
            .product_bindings
            .iter()
            .any(|binding| binding.source == "dynamic_texture(inspect.product:surface)")
    );
    assert_eq!(inspection.statuses[0].status, "stale");
    assert!(inspection.diagnostics.iter().any(|diagnostic| {
        diagnostic.diagnostic_kind == "missing_upload"
            && diagnostic.request_kind == "dynamic_upload"
            && diagnostic.surface_key.as_deref() == Some("surface.missing_upload")
    }));
    assert!(inspection.diagnostics.iter().any(|diagnostic| {
        diagnostic.diagnostic_kind == "producer_status"
            && diagnostic.status.as_deref() == Some("stale")
            && diagnostic.severity == "warning"
    }));
}

#[test]
fn render_runtime_inspect_prepared_frame_rejects_conflicting_dynamic_target_history_signatures() {
    let target_key = RenderDynamicTextureTargetKey::new("editor.viewport.1", "scene_color");
    let flow_id = engine::plugins::render::RenderFlowId::try_from_raw(3).unwrap();
    let binding = PreparedTargetBinding::DynamicTexture(target_key);
    let mut first_bindings = BTreeMap::new();
    first_bindings.insert(alias_key("viewport.scene_color"), binding.clone());
    let mut second_bindings = BTreeMap::new();
    second_bindings.insert(alias_key("viewport.scene_color"), binding);

    let frame = PreparedRenderFrame {
        context: PreparedFrameContext {
            frame_index: 42,
            flow_registry_revision: 1,
            shader_registry_revision: 2,
            prepare_epoch: 5,
        },
        surface: PreparedSurfaceInfo::primary((1920, 1080)),
        views: vec![PreparedViewFrame::offscreen_product(
            "viewport.1",
            (640, 360),
        )],
        flows: BTreeMap::new(),
        flow_invocations: vec![
            PreparedFlowInvocation {
                invocation_id: PreparedFlowInvocationId::new("viewport.1.scene.a"),
                flow_id,
                view_id: "viewport.1".to_string(),
                inputs: PreparedFlowInputs::default(),
                target_alias_bindings: first_bindings,
                history_signature: Some("camera:a".to_string()),
            },
            PreparedFlowInvocation {
                invocation_id: PreparedFlowInvocationId::new("viewport.1.scene.b"),
                flow_id,
                view_id: "viewport.1".to_string(),
                inputs: PreparedFlowInputs::default(),
                target_alias_bindings: second_bindings,
                history_signature: Some("camera:b".to_string()),
            },
        ],
        dynamic_texture_targets: Vec::new(),
        dynamic_texture_uploads: Vec::new(),
        product_selections: Vec::new(),
        viewport_surface_bindings: ViewportSurfaceBindingRegistry::default(),
        contributions: PreparedFrameContributions::default(),
        shader: PreparedShaderSnapshot {
            registry_revision: 2,
        },
    };

    let err = frame
        .dynamic_target_history_signatures()
        .expect_err("one target cannot carry conflicting history signatures");
    assert!(
        err.to_string().contains("incompatible history signatures"),
        "unexpected error: {err}"
    );
}

#[test]
fn render_runtime_inspect_render_gpu_residency_inspection_exposes_logical_cache_without_backend_handles()
 {
    let mut residency = RenderGpuResidencyResource::default();
    let selection = RenderProductSelection::new("viewport.1")
        .with_selected_product(RenderSelectedProduct {
            product_id: ProductIdentity::new(77),
            scale_band: ProductScaleBand::Preview,
            generation: 12,
            freshness: ProductFreshness::Current,
            residency: ProductResidency::Resident,
            authority_class: ProductAuthorityClass::DeterministicDerived,
            query_policy: ProductQueryPolicy::StrictCurrentOnly,
        })
        .with_residency_request(RenderResidencyRequest::new(
            ProductIdentity::new(77),
            ProductResidency::Resident,
            100,
            true,
        ));

    residency.derive_from_selections(&[selection], &RenderGpuResidencyBudgetResource::default());

    let inspection = inspect_render_gpu_residency(&residency);
    assert_eq!(inspection.addressable_count, 1);
    assert_eq!(inspection.selected_count, 1);
    assert_eq!(inspection.requested_count, 1);
    assert_eq!(inspection.accepted_count, 1);
    assert_eq!(inspection.resident_count, 1);
    assert_eq!(inspection.budget.resident_entry_status, "within_budget");
    assert_eq!(inspection.entries[0].product_id, 77);
    assert_eq!(inspection.entries[0].cache_id, "render-gpu-cache:1");
    assert_eq!(inspection.entries[0].resident_bytes, 256 * 1024);
    assert_eq!(inspection.entries[0].diagnostic_count, 0);
    assert_eq!(inspection.journal[0].action, "Allocated");
    assert_eq!(
        inspection.journal[0].cache_id.as_deref(),
        Some("render-gpu-cache:1")
    );
}

#[test]
fn render_runtime_inspect_pass_provenance_state_preserves_required_human_fields() {
    let mut state = RenderPassProvenanceState::default();
    state.observe_frame(
        12,
        &[RenderPassProvenanceRecord {
            frame_index: 12,
            flow_id: "runenwerk.editor.main".to_string(),
            pass_id: "runenwerk.editor.viewport.sdf".to_string(),
            pass_label: "runenwerk.editor.viewport.sdf".to_string(),
            pass_kind: FlowPassKind::Fullscreen,
            authoring_index: 3,
            feature_id: Some("feature.ui".to_string()),
            shader_id: "editor_viewport_sdf".to_string(),
            shader_revision: 7,
            fallback_used: false,
            pipeline_stats_key: "flow:runenwerk.editor.main".to_string(),
            bind_group_layout_signature_hash: 10,
            material_specialization_fragment_hash: 11,
            view_signature_hash: 12,
            feature_runtime_version: 13,
            color_formats: vec![GpuTextureFormat::Rgba8Unorm],
            depth_format: None,
            sample_count: 1,
            primitive_topology: Some(GpuPrimitiveTopology::TriangleList),
            material_binding: RenderPassMaterialBindingEvidence::default(),
            render_targets: vec!["surface.color".to_string()],
            sampled_textures: vec!["surface.color".to_string()],
            storage_textures: Vec::new(),
            depth_targets: Vec::new(),
            capture_points_available: vec![RenderCapturePointIdentity {
                flow_id: "runenwerk.editor.main".to_string(),
                pass_id: "runenwerk.editor.viewport.sdf".to_string(),
                stage: CaptureStage::After,
                resource_id: "surface.color".to_string(),
                texture_class: CaptureTextureClass::ImportedTexture,
            }],
        }],
    );

    assert_eq!(state.frame_index, 12);
    assert_eq!(state.records.len(), 1);
    let record = &state.records[0];
    assert_eq!(record.flow_id, "runenwerk.editor.main");
    assert_eq!(record.pass_id, "runenwerk.editor.viewport.sdf");
    assert_eq!(record.pass_label, "runenwerk.editor.viewport.sdf");
    assert_eq!(record.shader_id, "editor_viewport_sdf");
    assert_eq!(record.shader_revision, 7);
    assert!(!record.fallback_used);
    assert!(!record.material_binding.consumes_material_resources);
}

#[test]
fn render_runtime_inspect_deterministic_capture_filename_uses_required_identity_tuple() {
    let identity = RenderCaptureIdentity {
        frame_index: 9,
        pass_label: "runenwerk.editor.main.ui".to_string(),
        capture_point: RenderCapturePointIdentity {
            flow_id: "runenwerk.editor.main".to_string(),
            pass_id: "runenwerk.editor.main.ui".to_string(),
            stage: CaptureStage::Before,
            resource_id: "surface.color".to_string(),
            texture_class: CaptureTextureClass::ImportedTexture,
        },
    };

    let name = deterministic_capture_filename(&identity, "png");
    assert_eq!(
        name,
        "frame_9__flow_runenwerk_editor_main__pass_runenwerk_editor_main_ui__stage_before__resource_surface_color.png"
    );
}

#[test]
fn render_runtime_inspect_includes_registered_feature_contributions() {
    let feature_id = RenderFeatureId::try_from_raw(42_001).expect("test id should be non-zero");
    let mut contributions = PreparedFrameContributions::default();
    contributions.insert_registered(
        feature_id,
        PreparedRegisteredFeaturePayload::new(
            StaticRegisteredFeaturePayload::new("inspect.payload", "inspectable contribution")
                .with_field("source", "test"),
        ),
        FeatureContributionStatus::Ready,
        FeatureFallbackPolicy::SkipFeaturePasses,
    );
    let frame = PreparedRenderFrame {
        context: PreparedFrameContext {
            frame_index: 1,
            flow_registry_revision: 0,
            shader_registry_revision: 0,
            prepare_epoch: 1,
        },
        surface: PreparedSurfaceInfo::primary((64, 64)),
        views: vec![PreparedViewFrame::main((64, 64))],
        flows: BTreeMap::new(),
        flow_invocations: Vec::new(),
        dynamic_texture_targets: Vec::new(),
        dynamic_texture_uploads: Vec::new(),
        product_selections: Vec::new(),
        viewport_surface_bindings: ViewportSurfaceBindingRegistry::default(),
        contributions,
        shader: PreparedShaderSnapshot {
            registry_revision: 0,
        },
    };

    let inspection = inspect_prepared_render_frame(&frame);

    assert_eq!(inspection.feature_contributions.len(), 1);
    let contribution = &inspection.feature_contributions[0];
    assert_eq!(contribution.feature_id, feature_id.to_string());
    assert_eq!(contribution.payload_kind, "inspect.payload");
    assert_eq!(
        contribution.registered_payload_summary.as_deref(),
        Some("inspectable contribution")
    );
    assert_eq!(
        contribution.registered_payload_fields,
        vec![("source".to_string(), "test".to_string())]
    );
}
