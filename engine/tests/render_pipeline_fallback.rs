use engine::plugins::render::inspect::{
    CaptureStage, CaptureTextureClass, RenderCapturePointIdentity,
    RenderPassMaterialBindingEvidence, RenderPassProvenanceRecord,
    RenderPipelineFallbackDiagnostic, RenderPipelineFallbackDiagnosticSeverity,
    RenderPipelineFallbackInspectionRequest, inspect_render_pipeline_fallback,
};
use engine::plugins::render::pipelines::{FlowPassKind, PipelineCacheStats};
use engine::plugins::render::shader::{
    ShaderRegistryEvent, ShaderRegistryEventKind, ShaderReloadPollReport, ShaderReloadPollStatus,
};
use runen_gpu::{GpuPrimitiveTopology, GpuTextureFormat};

#[test]
fn render_pipeline_fallback_reports_ready_cache_and_prior_valid_shader_failure() {
    let pass = pipeline_pass(
        "material.scene",
        true,
        false,
        7,
        "flow:main/pass:material",
        99,
    );
    let inspection = inspect_render_pipeline_fallback(RenderPipelineFallbackInspectionRequest {
        pass_provenance: &[pass],
        pipeline_cache_stats: Some(PipelineCacheStats {
            hits: 2,
            misses: 1,
            failures: 0,
        }),
        shader_reload_poll: Some(polled_shader_report()),
        shader_events: &[shader_event(ShaderRegistryEventKind::Failed, 5)],
        require_pipeline_cache_stats: true,
        require_material_exact_shader: true,
    });

    assert!(inspection.is_ready(), "{:?}", inspection.diagnostics);
    assert_eq!(inspection.counts.pass_count, 1);
    assert_eq!(inspection.counts.pipeline_backed_pass_count, 1);
    assert_eq!(inspection.counts.material_pass_count, 1);
    assert_eq!(inspection.counts.fallback_pass_count, 0);
    assert_eq!(inspection.counts.shader_failure_event_count, 1);
    assert_eq!(inspection.counts.prior_valid_shader_failure_count, 1);
    assert_eq!(inspection.counts.pipeline_cache_hit_count, 2);
    assert_eq!(inspection.counts.pipeline_cache_miss_count, 1);
    assert_eq!(
        inspection.shader_reload_status,
        Some(ShaderReloadPollStatus::Polled)
    );
    assert!(has_warning(
        &inspection.diagnostics,
        "shader_failure_preserved_prior_valid_revision"
    ));
}

#[test]
fn render_pipeline_fallback_fails_when_material_pass_uses_fallback_shader() {
    let pass = pipeline_pass(
        "fallback.scene",
        true,
        true,
        0,
        "flow:main/pass:material",
        0,
    );
    let inspection = inspect_render_pipeline_fallback(RenderPipelineFallbackInspectionRequest {
        pass_provenance: &[pass],
        pipeline_cache_stats: Some(PipelineCacheStats {
            hits: 0,
            misses: 1,
            failures: 0,
        }),
        shader_reload_poll: Some(polled_shader_report()),
        shader_events: &[],
        require_pipeline_cache_stats: true,
        require_material_exact_shader: true,
    });

    assert!(!inspection.is_ready());
    assert!(has_error(
        &inspection.diagnostics,
        "material_shader_fallback_forbidden"
    ));
    assert!(has_error(
        &inspection.diagnostics,
        "missing_material_shader_revision"
    ));
    assert!(has_error(
        &inspection.diagnostics,
        "missing_material_specialization_fragment"
    ));
}

#[test]
fn render_pipeline_fallback_fails_without_pipeline_stats_key() {
    let pass = pipeline_pass("material.scene", true, false, 7, "", 99);
    let inspection = inspect_render_pipeline_fallback(RenderPipelineFallbackInspectionRequest {
        pass_provenance: &[pass],
        pipeline_cache_stats: Some(PipelineCacheStats {
            hits: 1,
            misses: 0,
            failures: 0,
        }),
        shader_reload_poll: Some(polled_shader_report()),
        shader_events: &[],
        require_pipeline_cache_stats: true,
        require_material_exact_shader: true,
    });

    assert!(!inspection.is_ready());
    assert!(has_error(
        &inspection.diagnostics,
        "missing_pipeline_stats_key"
    ));
}

#[test]
fn render_pipeline_fallback_fails_without_cache_stats_or_prior_valid_shader() {
    let pass = pipeline_pass(
        "material.scene",
        true,
        false,
        7,
        "flow:main/pass:material",
        99,
    );
    let inspection = inspect_render_pipeline_fallback(RenderPipelineFallbackInspectionRequest {
        pass_provenance: &[pass],
        pipeline_cache_stats: None,
        shader_reload_poll: Some(polled_shader_report()),
        shader_events: &[shader_event(ShaderRegistryEventKind::SkippedEmpty, 0)],
        require_pipeline_cache_stats: true,
        require_material_exact_shader: true,
    });

    assert!(!inspection.is_ready());
    assert!(has_error(
        &inspection.diagnostics,
        "missing_pipeline_cache_stats"
    ));
    assert!(has_error(
        &inspection.diagnostics,
        "shader_failure_without_prior_valid_revision"
    ));
}

#[test]
fn render_pipeline_fallback_reports_throttled_poll_as_warning_only() {
    let pass = pipeline_pass("world.draw", false, true, 0, "flow:main/pass:world", 0);
    let inspection = inspect_render_pipeline_fallback(RenderPipelineFallbackInspectionRequest {
        pass_provenance: &[pass],
        pipeline_cache_stats: Some(PipelineCacheStats {
            hits: 1,
            misses: 0,
            failures: 0,
        }),
        shader_reload_poll: Some(ShaderReloadPollReport {
            status: ShaderReloadPollStatus::Throttled,
            elapsed_ms: 100.0,
            interval_ms: 500.0,
            force_reload: false,
        }),
        shader_events: &[],
        require_pipeline_cache_stats: true,
        require_material_exact_shader: true,
    });

    assert!(inspection.is_ready(), "{:?}", inspection.diagnostics);
    assert_eq!(inspection.counts.fallback_pass_count, 1);
    assert_eq!(inspection.counts.material_fallback_pass_count, 0);
    assert!(has_warning(
        &inspection.diagnostics,
        "shader_reload_throttled"
    ));
    assert!(has_warning(
        &inspection.diagnostics,
        "non_material_shader_fallback"
    ));
}

fn pipeline_pass(
    shader_id: &str,
    consumes_material_resources: bool,
    fallback_used: bool,
    shader_revision: u64,
    pipeline_stats_key: &str,
    material_specialization_fragment_hash: u64,
) -> RenderPassProvenanceRecord {
    RenderPassProvenanceRecord {
        frame_index: 3,
        flow_id: "runenwerk.editor.main".to_string(),
        pass_id: "runenwerk.editor.material".to_string(),
        pass_label: "runenwerk.editor.material".to_string(),
        pass_kind: FlowPassKind::Fullscreen,
        authoring_index: 1,
        feature_id: consumes_material_resources.then(|| "feature.material".to_string()),
        shader_id: shader_id.to_string(),
        shader_revision,
        fallback_used,
        pipeline_stats_key: pipeline_stats_key.to_string(),
        bind_group_layout_signature_hash: 42,
        material_specialization_fragment_hash,
        view_signature_hash: 33,
        feature_runtime_version: 12,
        color_formats: vec![GpuTextureFormat::Rgba8Unorm],
        depth_format: None,
        sample_count: 1,
        primitive_topology: Some(GpuPrimitiveTopology::TriangleList),
        material_binding: RenderPassMaterialBindingEvidence {
            consumes_material_resources,
            prepared_material_available: consumes_material_resources,
            material_table_identity: consumes_material_resources
                .then(|| "material.table:scene".to_string()),
            scene_shader_identity: consumes_material_resources
                .then(|| "shader.identity:scene".to_string()),
            scene_shader_path: consumes_material_resources.then(|| shader_id.to_string()),
            material_instance_count: usize::from(consumes_material_resources),
            material_binding_slot_count: usize::from(consumes_material_resources),
            prepared_model_mesh_material_selection_count: usize::from(consumes_material_resources),
            model_mesh_material_selections_available_to_pass: Vec::new(),
        },
        render_targets: vec!["surface.color".to_string()],
        sampled_textures: Vec::new(),
        storage_textures: Vec::new(),
        depth_targets: Vec::new(),
        capture_points_available: vec![RenderCapturePointIdentity {
            flow_id: "runenwerk.editor.main".to_string(),
            pass_id: "runenwerk.editor.material".to_string(),
            stage: CaptureStage::After,
            resource_id: "surface.color".to_string(),
            texture_class: CaptureTextureClass::ImportedTexture,
        }],
    }
}

fn shader_event(kind: ShaderRegistryEventKind, revision: u64) -> ShaderRegistryEvent {
    ShaderRegistryEvent {
        kind,
        id: "material.scene".to_string(),
        path: "generated/material.scene.wgsl".to_string(),
        revision,
        error: Some("compile failed".to_string()),
        details: None,
    }
}

fn polled_shader_report() -> ShaderReloadPollReport {
    ShaderReloadPollReport {
        status: ShaderReloadPollStatus::Polled,
        elapsed_ms: 500.0,
        interval_ms: 500.0,
        force_reload: true,
    }
}

fn has_error(diagnostics: &[RenderPipelineFallbackDiagnostic], code: &str) -> bool {
    diagnostics.iter().any(|diagnostic| {
        diagnostic.severity == RenderPipelineFallbackDiagnosticSeverity::Error
            && diagnostic.code == code
    })
}

fn has_warning(diagnostics: &[RenderPipelineFallbackDiagnostic], code: &str) -> bool {
    diagnostics.iter().any(|diagnostic| {
        diagnostic.severity == RenderPipelineFallbackDiagnosticSeverity::Warning
            && diagnostic.code == code
    })
}
