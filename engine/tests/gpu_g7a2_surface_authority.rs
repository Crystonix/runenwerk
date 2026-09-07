//! Runenwerk-side surface cutover guard after RunenGPU became the surface execution authority.

use std::fs;
use std::path::PathBuf;

#[test]
fn renderer_surface_host_routes_through_public_runengpu_without_a_raw_renderer_bridge() {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let host = fs::read_to_string(manifest.join("src/plugins/render/backend/wgpu_ctx.rs"))
        .expect("Runenwerk surface host adapter should remain readable");
    let execute =
        fs::read_to_string(manifest.join("src/plugins/render/renderer/render_flow/execute.rs"))
            .expect("renderer execution source should remain readable");

    for required in [
        "GpuContext",
        "GpuSurfaceHandle",
        "GpuContext::request_for_surface",
        ".configure_surface(",
        ".acquire_surface_image(",
    ] {
        assert!(
            host.contains(required),
            "Runenwerk surface host must delegate through public RunenGPU contract {required}"
        );
    }
    assert!(execute.contains("ResolvedRenderGpuWorkNode::present("));

    for forbidden in [
        "wgpu::",
        "get_current_texture(",
        "queue.present(",
        "CurrentRenderDeviceQueue",
        "current_render_device_queue",
        "CurrentRenderExecutionBridge",
        "current_render_execution_bridge",
    ] {
        assert!(
            !host.contains(forbidden) && !execute.contains(forbidden),
            "Runenwerk regained private surface/execution authority through {forbidden}"
        );
    }
}
