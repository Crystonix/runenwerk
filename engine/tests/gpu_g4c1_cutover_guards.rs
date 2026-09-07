//! RunenRender-side guards for the external RunenGPU realization/execution boundary.

use std::fs;
use std::path::{Path, PathBuf};

fn read(manifest: &Path, relative: &str) -> String {
    fs::read_to_string(manifest.join(relative))
        .unwrap_or_else(|error| panic!("cannot read {relative}: {error}"))
}

fn collect_rust_sources(root: &Path, paths: &mut Vec<PathBuf>) {
    let entries = fs::read_dir(root)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", root.display()));
    for entry in entries {
        let path = entry.expect("source entry should be readable").path();
        if path.is_dir() {
            collect_rust_sources(&path, paths);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            paths.push(path);
        }
    }
}

#[test]
fn retired_renderer_execution_bridges_remain_absent() {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut paths = Vec::new();
    collect_rust_sources(&manifest.join("src/plugins/render"), &mut paths);
    paths.sort();

    for path in paths {
        let source = fs::read_to_string(&path).expect("renderer source should be readable");
        for forbidden in [
            "CurrentRenderExecutionBridge",
            "current_render_execution_bridge",
            "CurrentRenderDeviceQueue",
            "current_render_device_queue",
            "queue.submit(",
            "wgpu::CommandEncoder",
            "wgpu::CommandBuffer",
        ] {
            assert!(
                !source.contains(forbidden),
                "{} regained retired/raw execution authority through {forbidden}",
                path.display()
            );
        }
    }
}

#[test]
fn renderer_completes_realization_before_one_canonical_runengpu_acceptance() {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let execute = read(
        &manifest,
        "src/plugins/render/renderer/render_flow/execute.rs",
    );

    let realization = execute
        .find("let mut batch = self.realize_render_batch(")
        .expect("render packet must begin with complete renderer realization");
    let graph = execute
        .find("prepare_render_gpu_frame_work(")
        .expect("renderer must prepare one complete frame graph");
    let prepare = execute
        .find("context.prepare_submission(graph)")
        .expect("renderer must delegate physical preparation to RunenGPU");
    let accept = execute
        .find("context.submit_prepared(prepared)")
        .expect("renderer must cross one RunenGPU submission boundary");
    assert!(
        realization < graph && graph < prepare && prepare < accept,
        "renderer realization and graph formation must complete before RunenGPU acceptance"
    );

    for forbidden in [
        "current_render_device_queue()",
        "current_render_execution_bridge()",
        "CommandEncoder",
        "queue.submit(",
        ".create_compute_pipeline(",
        ".create_render_pipeline(",
    ] {
        assert!(
            !execute.contains(forbidden),
            "renderer production execution retained raw authority via {forbidden}"
        );
    }
}

#[test]
fn renderer_pipeline_realization_uses_only_public_runengpu_context_operations() {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let setup = read(&manifest, "src/plugins/render/renderer/setup.rs");
    let flow = read(
        &manifest,
        "src/plugins/render/renderer/render_flow/pipeline_realization.rs",
    );

    for required in [
        "context.realize_program",
        "context.realize_pipeline_layout",
        "context.realize_render_pipeline",
    ] {
        assert!(
            setup.contains(required),
            "UI realization must delegate {required} to RunenGPU"
        );
    }
    assert!(flow.contains("context.realize_compute_pipeline("));
    assert!(flow.contains("context.realize_render_pipeline("));

    for forbidden in [
        "current_render_device_queue(",
        ".create_compute_pipeline(",
        ".create_render_pipeline(",
        "wgpu::",
    ] {
        assert!(
            !setup.contains(forbidden) && !flow.contains(forbidden),
            "renderer pipeline realization regained raw authority through {forbidden}"
        );
    }
}
