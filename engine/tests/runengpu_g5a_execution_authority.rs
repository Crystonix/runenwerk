use std::fs;
use std::path::{Path, PathBuf};

#[path = "runengpu_g5a_execution_authority/renderer_timing_boundary.rs"]
mod renderer_timing_boundary;
#[path = "runengpu_g5a_execution_authority/renderer_ui_boundary.rs"]
mod renderer_ui_boundary;

fn engine_path(path: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(path)
}

fn source(path: &str) -> String {
    fs::read_to_string(engine_path(path))
        .unwrap_or_else(|error| panic!("failed to read {path}: {error}"))
}

#[test]
fn live_renderer_frame_submits_one_prepared_graph_without_raw_execution_fallback() {
    let execute = source("src/plugins/render/renderer/render_flow/execute.rs");
    let adapter = source("src/plugins/render/adapters/gpu_work.rs");

    for required in [
        "prepare_render_gpu_frame_work(",
        "context.prepare_submission(graph)",
        ".submit_prepared(prepared)",
        "ResolvedRenderGpuWorkNode::present(",
    ] {
        assert!(
            execute.contains(required),
            "live renderer execution must use the canonical G5 frame boundary through {required}"
        );
    }
    for retired in [
        "encode_compiled_pass(",
        "schedule_legacy_invocation_work(",
        "current_render_execution_bridge",
        "current_render_device_queue",
        "queue.submit",
        "RenderGpuWorkSidecar",
    ] {
        assert!(
            !execute.contains(retired) && !adapter.contains(retired),
            "raw or sidecar renderer execution authority must remain deleted: {retired}"
        );
    }
}

#[test]
fn g4_compiled_pass_realization_does_not_require_physical_surface_objects() {
    let pipeline = source("src/plugins/render/renderer/render_flow/pipeline_realization.rs");
    let realization_start = pipeline
        .find("fn realize_compiled_pass(")
        .expect("G4 compiled-pass realization boundary must remain explicit");
    let realization_end = pipeline[realization_start..]
        .find("fn resolve_color_target_format_from_plan(")
        .map(|offset| realization_start + offset)
        .expect("format projection helpers must follow G4 realization");
    let realization = &pipeline[realization_start..realization_end];

    for forbidden in [
        "frame_texture: &Texture",
        "frame_view: &TextureView",
        "current_render_execution_bridge",
        "resolve_color_target_from_plan(",
        "resolve_depth_target_from_plan(",
    ] {
        assert!(
            !realization.contains(forbidden),
            "G4 compiled-pass realization must not recover physical surface authority through {forbidden}"
        );
    }
    assert!(realization.contains("resolve_color_target_format_from_plan("));
    assert!(realization.contains("resolve_depth_target_format_from_plan("));

    let bindings = source("src/plugins/render/renderer/render_flow/bindings.rs");
    let binding_start = bindings
        .find("fn resolve_compiled_bind_group(")
        .expect("G4C2 bind-group realization boundary must remain explicit");
    let binding_end = bindings[binding_start..]
        .find("fn resolved_binding_texture_view(")
        .map(|offset| binding_start + offset)
        .expect("G4C2 binding helper must follow the realization boundary");
    let binding_realization = &bindings[binding_start..binding_end];
    assert!(!binding_realization.contains("frame_texture: &Texture"));
    assert!(!binding_realization.contains("resolve_texture("));
    assert!(binding_realization.contains("resolve_logical_texture_binding("));
}

#[test]
fn canonical_timing_tail_owns_one_readback_identity_without_a_logical_staging_copy() {
    let operations = source("src/plugins/render/renderer/render_flow/logical_operations.rs");
    let timing = source("src/plugins/render/renderer/render_flow/logical_timing.rs");
    let canonical = source("src/plugins/render/renderer/render_flow/canonical_work.rs");
    let execute = source("src/plugins/render/renderer/render_flow/execute.rs");
    let adapter = source("src/plugins/render/adapters/gpu_work.rs");

    assert!(operations.contains("readback: GpuReadbackOperation"));
    assert!(operations.contains("readback_id: GpuReadbackId"));
    assert!(operations.contains("GpuReadbackOperation::new("));
    assert!(!operations.contains("GpuReadbackId::allocate()?"));
    assert!(!operations.contains("readback_copy: GpuCopyOperation"));

    assert!(timing.contains("readback_id: GpuReadbackId"));
    assert!(timing.contains("let readback_id = GpuReadbackId::allocate()?;"));
    assert!(timing.contains("pub(super) const fn readback_id(&self) -> GpuReadbackId"));
    assert!(!timing.contains("readback_buffer: GpuBufferHandle"));

    assert!(canonical.contains("timing.readback_id(),"));
    assert!(execute.contains("timing.readback_id(),"));
    assert!(adapter.contains("pub(crate) fn timing_readback("));
    assert!(adapter.contains("operation: GpuWorkOperation::Readback(operation)"));
    assert!(!adapter.contains("TimingReadbackCopy"));
}

#[test]
fn canonical_physical_execution_is_owned_only_by_runengpu() {
    for retired in [
        "src/plugins/render/renderer/render_flow/canonical_compute.rs",
        "src/plugins/render/renderer/render_flow/canonical_copy.rs",
        "src/plugins/render/renderer/render_flow/canonical_render.rs",
        "src/plugins/render/renderer/render_flow/canonical_upload.rs",
    ] {
        assert!(
            !engine_path(retired).exists(),
            "renderer physical adapter must remain deleted: {retired}"
        );
    }

    let operations = source("src/plugins/render/renderer/render_flow/logical_operations.rs");
    for operation in [
        "GpuComputeOperation",
        "GpuRenderOperation",
        "GpuUploadOperation",
    ] {
        assert!(
            operations.contains(operation),
            "renderer must project execution-complete logical {operation} values"
        );
    }
    for forbidden in [
        "CommandEncoder",
        "RenderPass<'",
        "ComputePass<'",
        "queue.submit",
        "current_render_execution_bridge",
    ] {
        assert!(
            !operations.contains(forbidden),
            "logical operation projection must not regain physical execution through {forbidden}"
        );
    }
}
