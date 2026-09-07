use super::source;

#[test]
fn canonical_renderer_gpu_boundary_rejects_legacy_ui_semantics() {
    let adapter = source("src/plugins/render/adapters/gpu_work.rs");
    let canonical = source("src/plugins/render/renderer/render_flow/canonical_work.rs");

    for (owner, contents) in [
        ("render-work adapter", &adapter),
        ("canonical work", &canonical),
    ] {
        for forbidden in [
            "UiNode",
            "UiFrame",
            "UiPreparedDraws",
            "ui_render_data",
            "ui_runtime",
            "ui_tree",
            "ui_widgets",
        ] {
            assert!(
                !contents.contains(forbidden),
                "{owner} must not absorb deletion-bound Runenwerk UI authority through {forbidden}"
            );
        }
    }

    assert!(
        canonical.contains("builtin_ui_draws: Option<&'a [GpuRenderDraw]>"),
        "the temporary current-UI handoff must stay execution-complete and generic"
    );
    assert!(
        canonical.contains("not RunenUI or future RunenRender semantic authority"),
        "the deletion condition for the temporary current-UI handoff must remain explicit"
    );
}
