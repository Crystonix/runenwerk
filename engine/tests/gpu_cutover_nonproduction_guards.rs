//! Guard non-production Runenwerk consumers against stale predecessor RunenGPU paths and retired
//! renderer execution bridges.

use std::fs;
use std::path::{Path, PathBuf};

fn collect_rust_sources(root: &Path, output: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_rust_sources(&path, output);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            output.push(path);
        }
    }
}

fn nonproduction_rust_sources(workspace: &Path, engine: &Path) -> Vec<PathBuf> {
    let roots = [
        engine.join("tests"),
        engine.join("examples"),
        engine.join("benches"),
        workspace.join("apps/runenwerk_draw/tests"),
        workspace.join("apps/runenwerk_draw/examples"),
        workspace.join("apps/runenwerk_editor/tests"),
        workspace.join("apps/runenwerk_editor/examples"),
    ];
    let mut paths = Vec::new();
    for root in roots {
        collect_rust_sources(&root, &mut paths);
    }
    paths.sort();
    paths
}

fn is_cutover_guard_source(relative: &Path) -> bool {
    [
        "engine/tests/gpu_cutover_guards.rs",
        "engine/tests/gpu_cutover_nonproduction_guards.rs",
        "engine/tests/gpu_g4c1_cutover_guards.rs",
        "engine/tests/gpu_g7a2_surface_authority.rs",
        "engine/tests/render_cutoff_guard.rs",
        "engine/tests/runengpu_g5a_execution_authority.rs",
        "engine/tests/runengpu_g5a_execution_authority/renderer_timing_boundary.rs",
        "engine/tests/runengpu_g5c2_observation_authority.rs",
    ]
    .iter()
    .any(|guard| relative == Path::new(guard))
}

#[test]
fn tests_examples_and_benches_do_not_read_or_import_the_predecessor_gpu_tree() {
    let engine = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace = engine.parent().expect("engine must be a workspace member");
    let retired = [
        concat!("engine/src/plugins/", "gpu"),
        concat!("src/plugins/", "gpu"),
        concat!("crate::plugins::", "gpu"),
        concat!("engine::plugins::", "gpu"),
    ];
    let mut offenders = Vec::new();

    for path in nonproduction_rust_sources(workspace, &engine) {
        let relative = path.strip_prefix(workspace).unwrap_or(&path);
        if relative == Path::new("engine/tests/gpu_cutover_guards.rs")
            || relative == Path::new("engine/tests/gpu_cutover_nonproduction_guards.rs")
        {
            continue;
        }
        let source = fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()));
        for token in retired {
            if source.contains(token) {
                offenders.push(format!("{}: {token}", relative.display()));
            }
        }
    }

    assert!(
        offenders.is_empty(),
        "non-production consumers retained predecessor RunenGPU source/module paths: {offenders:#?}"
    );
}

#[test]
fn tests_examples_and_benches_do_not_retain_renderer_execution_bridges() {
    let engine = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace = engine.parent().expect("engine must be a workspace member");
    let retired = [
        concat!("CurrentRender", "ExecutionBridge"),
        concat!("current_render_", "execution_bridge"),
        concat!("CurrentRender", "DeviceQueue"),
        concat!("current_render_", "device_queue"),
        concat!("CurrentHost", "SurfaceBridge"),
        concat!("current_host_", "surface_bridge"),
        concat!("request_for_", "current_host("),
    ];
    let mut offenders = Vec::new();

    for path in nonproduction_rust_sources(workspace, &engine) {
        let relative = path.strip_prefix(workspace).unwrap_or(&path);
        if is_cutover_guard_source(relative) {
            continue;
        }
        let source = fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()));
        for token in retired {
            if source.contains(token) {
                offenders.push(format!("{}: {token}", relative.display()));
            }
        }
    }

    assert!(
        offenders.is_empty(),
        "non-production consumers retained retired renderer execution/surface bridges: {offenders:#?}"
    );
}
