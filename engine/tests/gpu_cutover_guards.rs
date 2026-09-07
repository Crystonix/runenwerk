//! Final consumer-side guards for the exact-SHA RunenGPU cutover.

use std::fs;
use std::path::{Path, PathBuf};

fn read(path: &Path) -> String {
    fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()))
}

fn rust_sources(root: &Path, output: &mut Vec<PathBuf>) {
    for entry in
        fs::read_dir(root).unwrap_or_else(|error| panic!("cannot read {}: {error}", root.display()))
    {
        let path = entry.expect("source entry should be readable").path();
        if path.is_dir() {
            rust_sources(&path, output);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            output.push(path);
        }
    }
}

fn sources_below(root: &Path) -> String {
    let mut paths = Vec::new();
    rust_sources(root, &mut paths);
    paths.sort();
    paths
        .into_iter()
        .map(|path| read(&path))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn runenwerk_consumes_only_the_accepted_runengpu_revision() {
    let engine = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace = engine.parent().expect("engine must be a workspace member");
    let root_manifest = read(&workspace.join("Cargo.toml"));
    let engine_manifest = read(&engine.join("Cargo.toml"));
    let lockfile = read(&workspace.join("Cargo.lock"));

    assert!(root_manifest.contains(
        "runen-gpu = { git = \"https://github.com/dornglut/runen-gpu\", rev = \"77c7c8d5ad6922b6f46c6b25e31b1a224c1314a4\" }"
    ));
    assert!(engine_manifest.contains("runen-gpu.workspace = true"));
    assert!(engine_manifest.contains("naga ="));
    assert!(engine_manifest.contains("pollster ="));
    assert!(engine_manifest.contains("bytemuck ="));
    assert!(engine_manifest.contains("tracing ="));
    for retired in ["raw-window-handle", "wgpu =", "tokio ="] {
        assert!(
            !engine_manifest.contains(retired),
            "engine retained direct dependency {retired}"
        );
    }
    assert!(lockfile.contains(
        "source = \"git+https://github.com/dornglut/runen-gpu?rev=77c7c8d5ad6922b6f46c6b25e31b1a224c1314a4#77c7c8d5ad6922b6f46c6b25e31b1a224c1314a4\""
    ));
}

#[test]
fn predecessor_gpu_authority_and_raw_renderer_values_are_absent() {
    let engine = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace = engine.parent().expect("engine must be a workspace member");
    assert!(!engine.join("src/plugins/gpu").exists());

    let plugins = read(&engine.join("src/plugins/mod.rs"));
    assert!(!plugins.contains("pub mod gpu"));
    assert!(!plugins.contains("pub use gpu"));

    let render_source = sources_below(&engine.join("src/plugins/render"));
    for retired in [
        "wgpu::Device",
        "wgpu::Queue",
        "wgpu::CommandEncoder",
        "wgpu::CommandBuffer",
        "wgpu::Surface<",
        "wgpu::SurfaceTexture",
        "queue.submit(",
        "get_mapped_range(",
    ] {
        assert!(
            !render_source.contains(retired),
            "renderer retained raw WGPU authority {retired}"
        );
    }

    let production_sources = sources_below(&engine.join("src")).to_owned()
        + &sources_below(&workspace.join("apps/runenwerk_draw/src"))
        + &sources_below(&workspace.join("apps/runenwerk_editor/src"));
    for retired in ["crate::plugins::gpu", "engine::plugins::gpu"] {
        assert!(
            !production_sources.contains(retired),
            "consumer retained predecessor path {retired}"
        );
    }

    let host_adapter = read(&engine.join("src/plugins/render/backend/wgpu_ctx.rs"));
    assert!(host_adapter.contains("GpuContext"));
    assert!(!host_adapter.contains("wgpu::"));
}
