use engine::plugins::render::{
    RenderFlow, RenderFlowGraph, RenderFlowId, RenderFlowValidationIssue,
    RenderGpuResourceLowering, RenderImportedTextureSemantic, RenderPassId, RenderPassKind,
    RenderPassNode, RenderResourceDeclaration, validate_flow_graph,
};
use runen_gpu::{GpuTextureFormat, GpuWorkResourceId};

fn test_resource_ids(count: usize) -> Vec<GpuWorkResourceId> {
    let labels = (0..count)
        .map(|index| format!("test.import.resource.{index}"))
        .collect::<Vec<_>>();
    let flow = labels
        .iter()
        .fold(RenderFlow::new("test.import.resources"), |flow, label| {
            flow.with_color_target(label.clone())
                .expect("render flow authoring should succeed")
        });
    labels
        .iter()
        .map(|label| flow.resource_id(label).expect("test resource should exist"))
        .collect()
}

#[test]
fn external_imported_texture_is_rejected_in_active_runtime_path() {
    let mut graph = RenderFlowGraph::new(
        RenderFlowId::try_from_raw(1).unwrap(),
        "import.contract.external",
    );
    let external_id = test_resource_ids(1)[0];
    graph.add_resource(
        RenderResourceDeclaration::declare_imported_external_texture(external_id, "external"),
    );

    let mut pass = RenderPassNode::new(
        RenderPassId::try_from_raw(1).unwrap(),
        "post.present",
        RenderPassKind::Present,
    );
    pass.present_source = Some(external_id);
    graph.add_pass(pass);

    let err = validate_flow_graph(&graph).expect_err("flow must reject external imports");
    assert!(
        err.issues.iter().any(|issue| matches!(
            issue,
            RenderFlowValidationIssue::UnsupportedExternalImportedTexture { .. }
        )),
        "expected external import rejection issue, got {:?}",
        err.issues
    );
}

#[test]
fn builtin_ui_composite_requires_canonical_read_write_contract() {
    let mut graph =
        RenderFlowGraph::new(RenderFlowId::try_from_raw(2).unwrap(), "import.contract.ui");
    let ids = test_resource_ids(2);
    let surface_color = ids[0];
    let ui_output = ids[1];
    graph.add_resource(RenderResourceDeclaration::declare_imported_surface_color(
        surface_color,
        "surface color",
    ));
    graph.add_resource(RenderResourceDeclaration::declare_color_attachment(
        ui_output,
        "ui output",
    ));

    let mut pass = RenderPassNode::new(
        RenderPassId::try_from_raw(2).unwrap(),
        "ui.composite",
        RenderPassKind::BuiltinUiComposite,
    );
    pass.storage_reads.push(surface_color);
    pass.color_outputs.push(ui_output);
    graph.add_pass(pass);

    let err = validate_flow_graph(&graph).expect_err("flow must enforce UI composite contract");
    assert!(
        err.issues.iter().any(|issue| matches!(
            issue,
            RenderFlowValidationIssue::BuiltinUiHasInvalidResourceRoles { .. }
        )),
        "expected UI reads contract issue, got {:?}",
        err.issues
    );
}

#[test]
fn typed_surface_imports_require_canonical_resource_ids() {
    let mut graph = RenderFlowGraph::new(
        RenderFlowId::try_from_raw(3).unwrap(),
        "import.contract.canonical",
    );
    let ids = test_resource_ids(2);
    graph.add_resource(RenderResourceDeclaration::declare_imported_surface_color(
        ids[0],
        "surface color first",
    ));
    graph.add_resource(RenderResourceDeclaration::declare_imported_surface_color(
        ids[1],
        "surface color second",
    ));

    let err = validate_flow_graph(&graph).expect_err("flow must enforce unique surface imports");
    assert!(
        err.issues.iter().any(|issue| matches!(
            issue,
            RenderFlowValidationIssue::MultipleSurfaceColorImports { .. }
        )),
        "expected duplicate surface color import issue, got {:?}",
        err.issues
    );
}

#[test]
fn surface_import_lowering_preserves_g7_owned_acquisition_intent() {
    let surface_id = test_resource_ids(1)[0];
    let declaration =
        RenderResourceDeclaration::declare_imported_surface_color(surface_id, "surface color");

    let lowering = declaration
        .lower_gpu_resource((1920, 1080), GpuTextureFormat::Bgra8UnormSrgb)
        .unwrap();

    assert!(matches!(
        lowering,
        RenderGpuResourceLowering::ImportedTexture(intent)
            if intent.id == surface_id
                && intent.label == "surface color"
                && intent.semantic == RenderImportedTextureSemantic::SurfaceColor
    ));
}
