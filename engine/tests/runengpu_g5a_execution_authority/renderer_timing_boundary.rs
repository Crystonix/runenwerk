use super::source;

#[test]
fn renderer_timing_metadata_is_realization_owned() {
    let execute = source("src/plugins/render/renderer/render_flow/execute.rs");
    let observation = source("src/plugins/render/renderer/render_flow/observation.rs");
    let timing = source("src/plugins/render/renderer/render_flow/gpu_timing.rs");
    let realization_start = execute
        .find("fn realize_render_batch<'a>(")
        .expect("renderer realization boundary must remain explicit");
    let realization_end = execute[realization_start..]
        .find("fn realize_projected_uniform_uploads(")
        .map(|offset| realization_start + offset)
        .expect("uniform upload realization must follow batch realization");
    let realization = &execute[realization_start..realization_end];

    assert!(
        realization.contains("register_pass_metadata("),
        "renderer timing evidence identity must be fixed during realization"
    );
    assert!(execute.contains("self.gpu_observations.accept("));
    assert!(execute.contains("filter_map(|invocation| invocation.timing_frame.take())"));
    assert!(!execute.contains("timing_frame.pending_evidence()"));
    assert!(observation.contains("submission: GpuSubmission"));
    assert!(observation.contains("GpuReadbackStatus::Pending"));
    assert!(observation.contains("timing.ready_evidence(&bytes)"));
    assert!(timing.contains("timestamp_period_ns: f32"));
    for retired in [
        "fn execute_realized_batch(",
        "timestamp_scale_available",
        "current_render_execution_bridge",
    ] {
        assert!(
            !execute.contains(retired),
            "timing must remain on the canonical submission lifecycle: {retired}"
        );
    }
}
