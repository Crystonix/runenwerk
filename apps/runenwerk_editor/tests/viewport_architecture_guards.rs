use std::fs;
use std::path::Path;

use editor_core::RealityVersion;
use editor_shell::{
    PanelInstanceId, StructuralCommandTarget, StructuralWidgetRoutingContext, TabStackId,
    ToolSurfaceInstanceId, VIEWPORT_SURFACE_DEFINITION_ID, WidgetId, viewport_embed_slot_for,
};
use editor_viewport::{
    ExpressionDimensions, ExpressionProductDescriptor, ExpressionProductId, ExpressionProductKind,
    ExpressionSourceRealityClass, ViewportId, ViewportPresentationState,
    ViewportSurfacePresentationSlot,
};
use engine::plugins::render::backend::RenderSurfaceId;
use engine::plugins::render::{
    EditorPickingHit, EditorPickingTarget, RenderFlowRegistryResource, RenderFrameProducerId,
    SurfaceFrameSubmissionRegistryResource,
};
use runenwerk_editor::runtime::viewport::{
    EDITOR_MAIN_FLOW_ID, MAIN_VIEWPORT_ID, MountedSurfaceRegistryResource,
    SurfaceDefinitionRegistryResource, ToolSurfaceRuntimeBindingRecord,
    ToolSurfaceRuntimeBindingRegistryResource, ViewportArtifactObservationResource,
    ViewportLayoutEntry, ViewportLayoutMapResource, ViewportPickingResultsResource,
    ViewportPresentationStateResource, ViewportProductRegistryResource,
    ViewportProductTargetRegistryResource, ViewportRuntimeSettingsHydrationResource,
    ViewportSurfaceHandle, ViewportSurfaceSetResource, ViewportSurfaceSlot,
    build_surface_binding_registry, initial_product_descriptors,
};
use ui_render_data::{UiPrimitive, ViewportSurfaceBindingSource};

const EDITOR_SHELL_UI_PRODUCER_ID: RenderFrameProducerId = ui_frame_producer_id(1001);
const TEST_RESOURCE_SCENE_COLOR: &str = "test.viewport.scene_color";
const TEST_RESOURCE_SCENE_COLOR_B: &str = "test.viewport.scene_color.b";

const fn ui_frame_producer_id(raw: u64) -> RenderFrameProducerId {
    match RenderFrameProducerId::try_from_raw(raw) {
        Ok(id) => id,
        Err(_) => panic!("ui frame producer id constants must be non-zero"),
    }
}

#[test]
fn viewport_presentation_state_is_product_addressed() {
    let viewport_id = ViewportId(77);
    let product_id = ExpressionProductId(177);
    let state = ViewportPresentationState::new(viewport_id, product_id);

    assert_eq!(state.viewport_id, viewport_id);
    assert_eq!(state.selected_primary_product_id, product_id);
}

#[test]
fn phase_one_product_kind_subset_is_locked() {
    let descriptors =
        initial_product_descriptors(ExpressionDimensions::new(1280, 720), RealityVersion(1));
    let kinds = descriptors
        .iter()
        .map(|descriptor| descriptor.kind)
        .collect::<Vec<_>>();

    assert!(kinds.contains(&ExpressionProductKind::SceneColor2D));
    assert!(kinds.contains(&ExpressionProductKind::PickingIds2D));
    assert!(kinds.contains(&ExpressionProductKind::Overlay2D));
}

#[test]
fn viewport_product_catalog_includes_field_asset_volume_brickmap_and_history_descriptors() {
    let descriptors =
        initial_product_descriptors(ExpressionDimensions::new(1280, 720), RealityVersion(1));
    let kinds = descriptors
        .iter()
        .map(|descriptor| descriptor.kind)
        .collect::<Vec<_>>();

    for expected in [
        ExpressionProductKind::ScalarField2D,
        ExpressionProductKind::VectorField2D,
        ExpressionProductKind::Atlas2D,
        ExpressionProductKind::VolumeSlice2D,
        ExpressionProductKind::BrickmapDebug2D,
        ExpressionProductKind::HistoryColor2D,
    ] {
        assert!(
            kinds.contains(&expected),
            "viewport product catalog should expose future producer descriptor {expected:?}",
        );
    }
}

#[test]
fn viewport_product_descriptor_requires_explicit_product_identity() {
    let descriptor = ExpressionProductDescriptor::new(
        ExpressionProductId(77),
        ExpressionProductKind::SceneColor2D,
        ExpressionDimensions::new(64, 64),
        editor_viewport::ExpressionFormat::Rgba8Unorm,
        "test.producer",
        ExpressionSourceRealityClass::ObservedScene,
        RealityVersion(2),
        editor_viewport::ExpressionFreshness::Current,
        editor_viewport::ExpressionPresentationHints::default(),
        None,
    );

    assert_eq!(descriptor.id, ExpressionProductId(77));
}

#[test]
fn runtime_viewport_resources_start_empty_before_bootstrap() {
    assert!(
        ViewportSurfaceSetResource::default()
            .viewport_ids()
            .next()
            .is_none()
    );
    assert!(ViewportProductRegistryResource::default().is_empty());
    assert!(ViewportPresentationStateResource::default().is_empty());
    assert!(ViewportRuntimeSettingsHydrationResource::default().is_empty());
    assert!(ViewportArtifactObservationResource::default().is_empty());
    assert!(ViewportPickingResultsResource::default().is_empty());
}

#[test]
fn bootstrap_seeding_is_explicit_and_main_viewport_scoped() {
    let app = runenwerk_editor::runtime::build_headless_app()
        .expect("headless app construction should succeed")
        .run_for_frames(1)
        .expect("headless editor app should run");
    let surface_sets = app
        .world()
        .resource::<ViewportSurfaceSetResource>()
        .expect("surface set resource should exist");
    let presentations = app
        .world()
        .resource::<ViewportPresentationStateResource>()
        .expect("presentation state resource should exist");

    assert!(
        surface_sets.surface_set(MAIN_VIEWPORT_ID).is_some(),
        "startup seeding should register the main viewport surface set explicitly",
    );
    assert!(
        presentations.state_for(MAIN_VIEWPORT_ID).is_some(),
        "startup seeding should register explicit presentation state for the main viewport",
    );
}

#[test]
fn runtime_mounts_tool_surfaces_through_ui_surface_contracts() {
    let app = runenwerk_editor::runtime::build_headless_app()
        .expect("headless app construction should succeed")
        .run_for_frames(1)
        .expect("headless editor app should run");
    let definitions = app
        .world()
        .resource::<SurfaceDefinitionRegistryResource>()
        .expect("surface definition registry should exist");
    let mounted = app
        .world()
        .resource::<MountedSurfaceRegistryResource>()
        .expect("mounted surface registry should exist");

    assert!(
        definitions
            .definition(VIEWPORT_SURFACE_DEFINITION_ID)
            .is_some(),
        "viewport surface definition must be registered in ui_surface definition registry",
    );
    assert!(
        mounted.mounted_surfaces().next().is_some(),
        "at least one production tool surface should be mounted through ui_surface contracts",
    );
}

#[test]
fn derived_bindings_support_multiple_viewports_without_main_fallback() {
    let viewport_a = ViewportId(2);
    let viewport_b = ViewportId(3);

    let mut surface_sets = ViewportSurfaceSetResource::default();
    surface_sets.set_surface(
        viewport_a,
        ViewportSurfaceSlot::PrimaryColor,
        ViewportSurfaceHandle::dynamic_texture("test.viewport", TEST_RESOURCE_SCENE_COLOR, true),
    );
    surface_sets.set_surface(
        viewport_b,
        ViewportSurfaceSlot::PrimaryColor,
        ViewportSurfaceHandle::dynamic_texture("test.viewport", TEST_RESOURCE_SCENE_COLOR_B, true),
    );

    let mut presentations = ViewportPresentationStateResource::default();
    presentations.upsert_state(ViewportPresentationState::new(
        viewport_a,
        ExpressionProductId(1),
    ));
    presentations.upsert_state(ViewportPresentationState::new(
        viewport_b,
        ExpressionProductId(1),
    ));

    let descriptors =
        initial_product_descriptors(ExpressionDimensions::new(320, 200), RealityVersion(1));
    let product_targets = ViewportProductTargetRegistryResource::from_descriptors_for_viewports(
        [viewport_a, viewport_b],
        &descriptors,
    );

    let registry = build_surface_binding_registry(&surface_sets, &presentations, &product_targets);
    let primary_a = registry
        .get(
            viewport_a.0,
            viewport_embed_slot_for(ViewportSurfacePresentationSlot::Primary),
        )
        .expect("viewport A should retain its primary binding");
    let primary_b = registry
        .get(
            viewport_b.0,
            viewport_embed_slot_for(ViewportSurfacePresentationSlot::Primary),
        )
        .expect("viewport B should retain its primary binding");

    assert!(matches!(
        &primary_a.source,
        ViewportSurfaceBindingSource::DynamicTexture { target_id, .. }
            if target_id == TEST_RESOURCE_SCENE_COLOR
    ));
    assert!(matches!(
        &primary_b.source,
        ViewportSurfaceBindingSource::DynamicTexture { target_id, .. }
            if target_id == TEST_RESOURCE_SCENE_COLOR_B
    ));
    assert!(
        registry
            .get(
                MAIN_VIEWPORT_ID.0,
                viewport_embed_slot_for(ViewportSurfacePresentationSlot::Primary),
            )
            .is_none(),
        "derived registry must not synthesize an implicit main viewport fallback",
    );
}

#[test]
fn viewport_picking_results_do_not_overwrite_across_viewports() {
    let mut picking_results = ViewportPickingResultsResource::default();
    let viewport_a = ViewportId(11);
    let viewport_b = ViewportId(12);
    picking_results.set_viewport_result(
        viewport_a,
        (100.0, 120.0),
        (0.0, 0.0, 300.0, 200.0),
        EditorPickingHit {
            target: EditorPickingTarget::Entity(42),
            distance: 1.0,
        },
    );
    picking_results.set_viewport_result(
        viewport_b,
        (600.0, 180.0),
        (400.0, 0.0, 300.0, 200.0),
        EditorPickingHit {
            target: EditorPickingTarget::Grid,
            distance: 2.0,
        },
    );

    assert_eq!(
        picking_results
            .result_for(viewport_a)
            .map(|value| value.hit.target),
        Some(EditorPickingTarget::Entity(42)),
        "viewport A picking result should remain intact after viewport B update",
    );
    assert_eq!(
        picking_results
            .result_for(viewport_b)
            .map(|value| value.hit.target),
        Some(EditorPickingTarget::Grid),
        "viewport B picking result should be stored independently",
    );
}

#[test]
fn active_flow_excludes_legacy_fullscreen_mask_architecture() {
    let app = runenwerk_editor::runtime::build_headless_app()
        .expect("headless app construction should succeed")
        .run_for_frames(1)
        .expect("headless editor app should run");
    let flow_registry = app
        .world()
        .resource::<RenderFlowRegistryResource>()
        .expect("render flow registry should exist");

    let editor_flow = flow_registry
        .compiled_flows()
        .iter()
        .find(|flow| flow.flow_label == EDITOR_MAIN_FLOW_ID)
        .expect("editor main flow should exist");
    let pass_ids = editor_flow
        .render_passes
        .iter()
        .map(|pass| pass.pass_label().to_string())
        .collect::<Vec<_>>();

    assert!(
        !pass_ids
            .iter()
            .any(|id| id == "runenwerk.editor.viewport.sdf"),
        "legacy fullscreen viewport-mask pass must not remain active",
    );
    let color_target_alias_count = editor_flow
        .resources
        .resources
        .iter()
        .filter(|resource| {
            matches!(
                resource,
                engine::plugins::render::RenderResourceDeclaration::TargetAlias(alias)
                    if alias.kind() == engine::plugins::render::RenderTargetAliasKind::Color
            )
        })
        .count();
    assert!(
        color_target_alias_count >= 3,
        "editor main flow must declare the three viewport product color target aliases",
    );
}

#[test]
fn viewport_scene_and_overlay_products_clear_before_drawing() {
    let app = runenwerk_editor::runtime::build_headless_app()
        .expect("headless app construction should succeed")
        .run_for_frames(1)
        .expect("headless editor app should run");
    let flow_registry = app
        .world()
        .resource::<RenderFlowRegistryResource>()
        .expect("render flow registry should exist");
    let editor_flow = flow_registry
        .compiled_flows()
        .iter()
        .find(|flow| flow.flow_label == EDITOR_MAIN_FLOW_ID)
        .expect("editor main flow should exist");

    for pass_label in [
        "runenwerk.editor.viewport.product.scene",
        "runenwerk.editor.viewport.product.overlay",
    ] {
        let pass = editor_flow
            .render_passes
            .iter()
            .find(|pass| pass.pass_label() == pass_label)
            .unwrap_or_else(|| panic!("editor flow should contain pass {pass_label}"));
        assert!(
            pass.node().clear_color.is_some(),
            "viewport product pass '{pass_label}' must clear its target before drawing so transparent miss pixels cannot retain stale frame contents",
        );
    }
}

#[test]
fn shell_frame_uses_viewport_embed_primitive_instead_of_raw_image_path() {
    let app = runenwerk_editor::runtime::build_headless_app()
        .expect("headless app construction should succeed")
        .run_for_frames(1)
        .expect("headless editor app should run");
    let submissions = app
        .world()
        .resource::<SurfaceFrameSubmissionRegistryResource>()
        .expect("ui submission registry should exist");
    let submission = submissions
        .get_for_surface(&EDITOR_SHELL_UI_PRODUCER_ID, RenderSurfaceId::primary())
        .expect("editor shell primary surface submission should exist");
    let has_embed = submission
        .frame
        .surfaces
        .iter()
        .flat_map(|surface| surface.layers.iter())
        .flat_map(|layer| layer.primitives.iter())
        .any(|primitive| matches!(primitive, UiPrimitive::ViewportSurfaceEmbed(_)));
    let has_image = submission
        .frame
        .surfaces
        .iter()
        .flat_map(|surface| surface.layers.iter())
        .flat_map(|layer| layer.primitives.iter())
        .any(|primitive| matches!(primitive, UiPrimitive::Image(_)));

    assert!(
        has_embed,
        "viewport panel must render through ViewportSurfaceEmbed primitive",
    );
    assert!(
        !has_image,
        "viewport panel must not use generic raw image texture path",
    );
}

#[test]
fn final_shell_frame_body_path_does_not_fan_out_by_panel_kind() {
    let shell_builder =
        include_str!("../../../domain/editor/editor_shell/src/composition/build_editor_shell.rs");
    let body_start = shell_builder
        .find("fn build_tab_stack_host_from_frame")
        .expect("final frame tab-stack host function should exist");
    let body_end = shell_builder[body_start..]
        .find("fn build_tab_strip")
        .map(|offset| body_start + offset)
        .expect("next function after final frame tab-stack host should exist");
    let final_body_path = &shell_builder[body_start..body_end];

    assert!(
        final_body_path.contains("frame_model.surface(surface_id)"),
        "final shell body path must resolve panel content through mounted ToolSurfaceInstanceId",
    );
    assert!(
        !final_body_path.contains("panel.panel_kind"),
        "final shell body path must not switch on legacy PanelKind",
    );
    for concrete_builder in [
        "build_outliner_panel(",
        "build_entity_table_panel(",
        "build_viewport_panel(",
        "build_inspector_panel(",
        "build_console_panel(",
    ] {
        assert!(
            !final_body_path.contains(concrete_builder),
            "final shell body path must not directly fan out to {concrete_builder}",
        );
    }
}

#[test]
fn production_shell_paths_use_app_owned_registry_and_surface_sessions() {
    let controller = include_str!("../src/shell/controller.rs");
    let dispatcher = include_str!("../src/shell/dispatch_shell_command.rs");
    let app_state = include_str!("../src/editor_app/state.rs");
    let shell_mod = include_str!("../src/shell/mod.rs");
    let providers = provider_sources();

    assert!(
        !controller.contains("EditorSurfaceProviderRegistry::runenwerk_default()"),
        "production controller paths must use the app/plugin-host-owned provider registry",
    );
    for forbidden in [
        concat!("EditorShell", "ViewModel"),
        concat!("build_editor_shell", "_view_model"),
        concat!("rebuild", "_view_model"),
    ] {
        assert!(
            !controller.contains(forbidden)
                && !dispatcher.contains(forbidden)
                && !shell_mod.contains(forbidden),
            "production shell modules must not expose or call old aggregate API {forbidden}",
        );
    }
    for forbidden in [
        "inspector_ui_state:",
        "entity_table_ui_state:",
        "viewport_interaction_state:",
        "console_follow_enabled:",
        "viewport_details_visible:",
        "fn inspector_ui_state",
        "fn entity_table_ui_state",
        "fn viewport_interaction_state",
        "fn set_console_follow_enabled",
        "fn set_viewport_details_visible",
        "fn toggle_viewport_details_visible",
    ] {
        assert!(
            !app_state.contains(forbidden),
            "RunenwerkEditorApp must not expose app-global provider-local state through {forbidden}",
        );
    }
    assert!(
        !providers.contains(concat!("SurfaceCommandProposal::", "ShellCommand"))
            && !providers.contains(concat!("SurfaceCommandProposal::", "shell")),
        "providers must return typed SurfaceCommandProposal variants, not boxed shell commands",
    );
    assert!(
        !providers.contains(concat!("SurfaceCommandProposal::", "NoOp")),
        "providers must represent no proposal as None, not a provider-side NoOp proposal",
    );
    for deleted_adapter_module in [
        "console_adapter",
        "entity_table_adapter",
        "inspector_adapter",
        "outliner_adapter",
        "viewport_adapter",
    ] {
        assert!(
            !shell_mod.contains(deleted_adapter_module)
                && !providers.contains(deleted_adapter_module),
            "old shell adapter module {deleted_adapter_module} must not remain wired into the final provider path",
        );
    }
    for forbidden in [
        "inspector_ui_state_mut",
        "entity_table_ui_state_mut",
        "viewport_interaction_state_mut",
        "set_console_follow_enabled",
        "set_viewport_details_visible",
        "toggle_viewport_details_visible",
    ] {
        assert!(
            !controller.contains(forbidden) && !dispatcher.contains(forbidden),
            "final shell controller/dispatcher paths must not mutate app-global provider-local state through {forbidden}",
        );
    }
}

#[test]
fn production_sources_do_not_keep_deleted_shell_adapter_modules() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    for deleted_path in [
        "src/shell/console_adapter.rs",
        "src/shell/entity_table_adapter.rs",
        "src/shell/inspector_adapter.rs",
        "src/shell/outliner_adapter.rs",
        "src/shell/viewport_adapter.rs",
    ] {
        assert!(
            !manifest_dir.join(deleted_path).exists(),
            "obsolete shell adapter file must be deleted: {deleted_path}",
        );
    }
}

#[test]
fn provider_seam_has_no_legacy_aggregate_or_boxed_shell_proposal_escape_hatch() {
    let surface_provider =
        include_str!("../../../domain/editor/editor_shell/src/surface_provider.rs");
    let shell_command =
        include_str!("../../../domain/editor/editor_shell/src/commands/shell_command.rs");
    let providers = provider_sources();
    let controller = include_str!("../src/shell/controller.rs");

    for source in [
        surface_provider.to_string(),
        shell_command.to_string(),
        providers,
        controller.to_string(),
    ] {
        for forbidden in [
            concat!("EditorShell", "ViewModel"),
            concat!("build_editor_shell", "_view_model"),
            concat!("rebuild", "_view_model"),
            concat!("ShellCommand", "(Box<ShellCommand>)"),
            concat!("SurfaceCommandProposal::", "NoOp"),
        ] {
            assert!(
                !source.contains(forbidden),
                "final provider/shell source must not contain obsolete path marker {forbidden}",
            );
        }
    }
}

#[test]
fn production_sources_do_not_implement_deferred_ui_execution_strategies() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir
        .parent()
        .and_then(Path::parent)
        .expect("runenwerk_editor lives under apps/");
    let production_roots = [
        workspace_root.join("domain/ui"),
        workspace_root.join("domain/editor/editor_shell/src"),
        manifest_dir.join("src"),
    ];
    let mut offenders = Vec::new();
    for root in &production_roots {
        collect_source_files(root, &mut offenders);
    }

    let forbidden_terms = [
        concat!("compiled ", "reactive"),
        concat!("Sv", "elte"),
        concat!("ECS-driven ", "UI"),
        concat!("multiple execution ", "strategies"),
    ];
    let offenders = offenders
        .into_iter()
        .filter_map(|path| {
            let contents = fs::read_to_string(&path).ok()?;
            forbidden_terms
                .iter()
                .any(|term| contents.contains(term))
                .then_some(path)
        })
        .collect::<Vec<_>>();

    assert!(
        offenders.is_empty(),
        "deferred UI execution strategies must not be implemented in production sources: {offenders:?}",
    );
}

#[test]
fn production_input_bridge_routes_viewport_interaction_by_tool_surface_session() {
    let input_bridge = include_str!("../src/runtime/systems/input_bridge.rs");

    for forbidden in [
        "viewport_interaction_state()",
        "viewport_interaction_state_mut()",
        concat!("dispatch_viewport_interaction", "_command("),
    ] {
        assert!(
            !input_bridge.contains(forbidden),
            "production input bridge must not use app-global viewport interaction state through {forbidden}",
        );
    }
    assert!(
        input_bridge.contains("tool_surface_id"),
        "viewport input route must carry ToolSurfaceInstanceId",
    );
    assert!(
        input_bridge.contains("dispatch_viewport_interaction_for_mounted_unit"),
        "viewport input must dispatch interaction commands to a mounted-unit targeted surface session",
    );
    assert!(
        input_bridge.contains("active_viewport_drag_mounted_unit"),
        "viewport drag continuation must resolve captured mounted-unit session state",
    );
    assert!(
        input_bridge.contains("viewport_scene_binding_for_widget"),
        "viewport input must route through an explicit scene-region binding helper",
    );
    assert!(
        !input_bridge.contains("resolve_structural_context(structural_context)\n        && binding.bounds.contains(position)"),
        "viewport input must not treat arbitrary viewport-surface chrome as scene interaction",
    );
}

#[test]
fn production_input_bridge_allows_viewport_scroll_only_after_ui_declines_ownership() {
    let input_bridge = include_str!("../src/runtime/systems/input_bridge.rs");

    assert!(
        input_bridge.contains("pointer_event_consumed_by_ui(&outcome)"),
        "viewport scroll fallback must consult the ui_runtime dispatch outcome",
    );
    assert!(
        input_bridge.contains("value.dispatch.response.propagation == EventPropagation::Stop"),
        "viewport scroll fallback must treat ui_runtime stop propagation as UI ownership",
    );
    assert!(
        input_bridge.contains("ViewportRenderStateCommand::ZoomCamera")
            && input_bridge.contains("!pointer_event_consumed_by_ui(&outcome)"),
        "camera zoom should be enqueued only after UI scroll ownership declines the wheel event",
    );
}

#[test]
fn viewport_status_arbitration_is_formed_before_scene_fallback() {
    let shell_builder =
        include_str!("../../../domain/editor/editor_shell/src/composition/build_editor_shell.rs");
    let input_bridge = include_str!("../src/runtime/systems/input_bridge.rs");

    assert!(
        shell_builder.contains("viewport_surface_interaction_model(frame_model)"),
        "shell projection must form viewport popup/status arbitration records before app input fallback",
    );
    assert!(
        shell_builder.contains(
            "UiViewportInputArbitrationPolicyDefinition::UiOwnsStatusBeforeViewportFallback"
        ),
        "status records must declare UI ownership before viewport fallback",
    );
    assert!(
        input_bridge.contains("viewport_scene_binding_for_widget")
            && input_bridge.contains("binding.host_widget_id == widget_id"),
        "viewport input fallback must still accept only the scene embed binding, not status/chrome widgets",
    );
}

#[test]
fn production_picking_routes_only_through_viewport_scene_region() {
    let picking = include_str!("../src/runtime/systems/picking.rs");

    assert!(
        picking.contains("viewport_scene_binding_for_widget"),
        "picking must distinguish viewport scene region widgets from provider chrome",
    );
    assert!(
        picking.contains("binding_containing_cursor(cursor)"),
        "picking fallback must use explicit viewport scene-region bounds",
    );
    assert!(
        !picking.contains("runtime_state.hovered_widget.and_then"),
        "picking must not resolve arbitrary hovered viewport-surface widgets as scene input",
    );
}

#[test]
fn editor_frame_submission_runs_after_input_bridge() {
    let plugin = include_str!("../src/runtime/plugin.rs");

    assert!(
        plugin.contains("sync_viewport_instances_system")
            && plugin.contains(".after(EditorRuntimeSet::InputBridge)"),
        "viewport lifecycle must consume the same-frame split/input layout mutations before frame projection",
    );
    assert!(
        plugin.contains("submit_editor_frame_system")
            && plugin.contains(".after(EditorRuntimeSet::ViewportLifecycle)"),
        "editor frame submission must consume resolved viewport lifecycle before product targets are prepared",
    );
}

#[test]
fn viewport_details_dispatch_has_no_first_active_viewport_fallback() {
    let dispatcher = read_workspace_source("apps/runenwerk_editor/src/shell/dispatch/viewport.rs");
    let providers = provider_sources();
    let shell_command =
        include_str!("../../../domain/editor/editor_shell/src/commands/shell_command.rs");

    assert!(
        !dispatcher.contains("active_surface_by_kind"),
        "viewport details dispatch must not pick the first active viewport surface",
    );
    assert!(
        !dispatcher.contains("ToolSurfaceKind::Viewport)\n            .map"),
        "viewport details dispatch must not map from viewport kind to an arbitrary active surface",
    );
    assert!(
        shell_command.contains("ApplySurfaceSessionMutation")
            && dispatcher.contains("ViewportSessionMutation::ToggleDetails"),
        "viewport details command must route through the generic surface-session mutation wrapper",
    );
    assert!(
        dispatcher.contains("target.active_tool_surface"),
        "viewport details mutation must use the routed ToolSurfaceInstanceId",
    );
    assert!(
        dispatcher.contains("session.viewport_details_visible = !session.viewport_details_visible"),
        "viewport details mutation must update the routed surface session",
    );
    assert!(
        providers.contains("SurfaceLocalAction::Viewport")
            && providers.contains("ViewportSurfaceAction::ToggleDetails"),
        "viewport provider must emit a provider-local details-toggle action",
    );
    assert!(
        providers.contains("VIEWPORT_DETAILS_TOGGLE_WIDGET_ID")
            && !providers.contains("VIEWPORT_PANEL_WIDGET_ID,\n            SurfaceLocalRoute::new(SurfaceLocalAction::Viewport"),
        "viewport details route must be attached to the dedicated details-toggle widget id",
    );
}

#[test]
fn runtime_tool_surface_binding_tracks_rebind_without_mutating_structural_identity() {
    let tool_surface_id = ToolSurfaceInstanceId::try_from_raw(91).unwrap();
    let panel_instance_id = PanelInstanceId::try_from_raw(41).unwrap();
    let tab_stack_id = TabStackId::try_from_raw(51).unwrap();
    let mut layout = ViewportLayoutMapResource::default();
    layout.upsert_entry(ViewportLayoutEntry {
        viewport_id: ViewportId(1),
        host_widget_id: WidgetId(1001),
        structural_context: StructuralWidgetRoutingContext {
            mounted_unit_id: None,
            panel_instance_id,
            active_tool_surface: Some(tool_surface_id),
            tab_stack_id,
        },
        bounds: ui_math::UiRect::new(0.0, 0.0, 640.0, 360.0),
    });

    let mut bindings = ToolSurfaceRuntimeBindingRegistryResource::default();
    bindings.rebuild_from_layout_map(&layout);

    layout.clear();
    layout.upsert_entry(ViewportLayoutEntry {
        viewport_id: ViewportId(2),
        host_widget_id: WidgetId(1002),
        structural_context: StructuralWidgetRoutingContext {
            mounted_unit_id: None,
            panel_instance_id,
            active_tool_surface: Some(tool_surface_id),
            tab_stack_id,
        },
        bounds: ui_math::UiRect::new(0.0, 0.0, 640.0, 360.0),
    });
    bindings.rebuild_from_layout_map(&layout);

    let binding = bindings
        .binding_for_tool_surface(tool_surface_id)
        .expect("binding should exist after rebind");
    assert_eq!(binding.viewport_id, ViewportId(2));
    assert_eq!(binding.panel_instance_id, panel_instance_id);
    assert_eq!(binding.tab_stack_id, tab_stack_id);
    assert_eq!(
        bindings
            .latest_rebind_for_tool_surface(tool_surface_id)
            .expect("rebind should be tracked")
            .from_viewport_id,
        ViewportId(1),
    );
}

#[test]
fn runtime_binding_resolution_rejects_structural_mismatch_even_when_viewport_matches() {
    let mut bindings = ToolSurfaceRuntimeBindingRegistryResource::default();
    bindings.upsert_binding(ToolSurfaceRuntimeBindingRecord {
        tool_surface_id: ToolSurfaceInstanceId::try_from_raw(7).unwrap(),
        panel_instance_id: PanelInstanceId::try_from_raw(11).unwrap(),
        tab_stack_id: TabStackId::try_from_raw(21).unwrap(),
        viewport_id: ViewportId(1),
        host_widget_id: WidgetId(301),
        bounds: ui_math::UiRect::new(0.0, 0.0, 320.0, 200.0),
        generation: 1,
    });

    let error = bindings
        .resolve_command_target(
            StructuralCommandTarget {
                mounted_unit_id: None,
                panel_instance_id: PanelInstanceId::try_from_raw(99).unwrap(),
                active_tool_surface: Some(ToolSurfaceInstanceId::try_from_raw(7).unwrap()),
                tab_stack_id: TabStackId::try_from_raw(199).unwrap(),
            },
            ViewportId(1),
        )
        .expect_err("structural mismatch should fail closed");

    assert!(matches!(
        error,
        runenwerk_editor::runtime::viewport::ToolSurfaceRuntimeBindingResolveError::StructuralBindingMismatch { .. }
    ));
}

#[test]
fn runtime_systems_do_not_route_viewports_through_first_frame_fallback() {
    let frame_submit = include_str!("../src/runtime/systems/frame_submit.rs");
    let input_bridge = include_str!("../src/runtime/systems/input_bridge.rs");
    let product_registry = include_str!("../src/runtime/viewport/product_registry.rs");

    assert!(
        !frame_submit.contains("first_frame("),
        "frame_submit routing must not depend on first_frame fallback selection",
    );
    assert!(
        !input_bridge.contains("first_frame("),
        "input bridge routing must not depend on first_frame fallback selection",
    );
    assert!(
        !product_registry.contains("pub fn first_frame"),
        "viewport observation resource must not expose first_frame fallback helpers",
    );
}

#[test]
fn runtime_systems_share_single_viewport_bootstrap_routing_seam() {
    let frame_submit = include_str!("../src/runtime/systems/frame_submit.rs");
    let input_bridge = include_str!("../src/runtime/systems/input_bridge.rs");
    let routing = include_str!("../src/runtime/viewport/routing.rs");

    assert!(
        !frame_submit.contains("fn resolve_structural_viewport_products"),
        "frame_submit should consume shared viewport routing policy instead of declaring a local copy",
    );
    assert!(
        !input_bridge.contains("fn resolve_structural_viewport_products"),
        "input_bridge should consume shared viewport routing policy instead of declaring a local copy",
    );
    assert!(
        routing.contains("projection_artifacts_available"),
        "shared routing seam must explicitly gate bootstrap fallback on projection-artifact availability",
    );
}

#[test]
fn viewport_provider_does_not_use_viewport_id_zero_fallback() {
    let viewport_provider = provider_sources();
    assert!(
        !viewport_provider.contains("ViewportId(0)"),
        "viewport provider must not synthesize ViewportId(0) fallback identities",
    );
}

#[test]
fn viewport_product_selection_routes_through_surface_presentation_and_ratification() {
    let viewport_dispatch =
        read_workspace_source("apps/runenwerk_editor/src/shell/dispatch/viewport.rs");

    assert!(
        viewport_dispatch.contains("SurfacePresentationModel"),
        "viewport product command path should build from a prepared surface presentation model",
    );
    assert!(
        viewport_dispatch.contains("ratify_surface_intent"),
        "viewport product command path should route mutation through a ratification adapter",
    );
}

#[test]
fn rb0_rb7_runtime_sources_do_not_depend_on_static_shared_viewport_product_ids() {
    let sources = read_workspace_sources(&[
        "apps/runenwerk_editor/src/runtime/app.rs",
        "apps/runenwerk_editor/src/runtime/systems/bootstrap.rs",
        "apps/runenwerk_editor/src/runtime/systems/frame_submit.rs",
        "apps/runenwerk_editor/src/runtime/viewport/product_registry.rs",
        "apps/runenwerk_editor/src/runtime/viewport/producer_scene.rs",
        "apps/runenwerk_editor/src/runtime/viewport/surface_set.rs",
        "apps/runenwerk_editor/src/runtime/viewport/presentation_resolver.rs",
    ]);
    let forbidden_terms = [
        "PRODUCT_ID_SCENE_COLOR",
        "PRODUCT_ID_PICKING_IDS",
        "PRODUCT_ID_OVERLAY",
        "VIEWPORT_RESOURCE_SCENE_COLOR",
        "VIEWPORT_RESOURCE_PICKING_IDS",
        "VIEWPORT_RESOURCE_OVERLAY",
        "\"editor.viewport.v1.scene_color\"",
        "\"editor.viewport.v1.picking_ids\"",
        "\"editor.viewport.v1.overlay\"",
    ];
    let offenders = forbidden_source_markers(&sources, &forbidden_terms);

    assert!(
        offenders.is_empty(),
        "RB0/RB7 viewport products must be viewport-scoped dynamic targets, not shared static ids: {offenders:?}",
    );
}

#[test]
fn rb0_runtime_sources_do_not_build_render_flows_per_viewport() {
    let app_source = read_workspace_source("apps/runenwerk_editor/src/runtime/app.rs");
    for forbidden in [
        "RenderFlow::new(format!",
        "RenderFlow::new(&format!",
        "format!(\"runenwerk.editor.viewport",
        "format!(\"runenwerk.editor.{}",
    ] {
        assert!(
            !app_source.contains(forbidden),
            "editor app render setup must not synthesize one RenderFlow per viewport through marker '{forbidden}'",
        );
    }

    let viewport_sources = read_workspace_source_tree("apps/runenwerk_editor/src/runtime/viewport");
    let offenders = forbidden_source_markers(
        &viewport_sources,
        &[
            "RenderFlow::new(",
            "add_render_flow(",
            "CompiledRenderFlowPlan",
        ],
    );
    assert!(
        offenders.is_empty(),
        "viewport runtime modules must not work around product surfaces by owning render-flow construction: {offenders:?}",
    );
}

#[test]
fn rb7_viewport_scene_product_shader_has_no_multi_rect_containment() {
    let sources = read_workspace_sources(&[
        "apps/runenwerk_editor/src/runtime/resources.rs",
        "assets/shaders/editor_viewport_scene_product.wgsl",
    ]);
    let forbidden_terms = [
        "viewport_b :",
        "viewport_c :",
        "viewport_d :",
        "pub viewport_b:",
        "pub viewport_c:",
        "pub viewport_d:",
        "u.viewport_b",
        "u.viewport_c",
        "u.viewport_d",
        "reserved_rect_",
        "_unused_rect_",
        "additional_viewport_bounds_px",
        "set_viewport_bounds_list",
        "viewport_contains_rect",
    ];
    let offenders = forbidden_source_markers(&sources, &forbidden_terms);

    assert!(
        offenders.is_empty(),
        "RB7 viewport scene products must render one viewport-local job instead of shader-side multi-rect containment: {offenders:?}",
    );
}

#[test]
fn rb7_editor_frame_submit_does_not_seed_or_extract_viewport_product_lifecycle() {
    let frame_submit =
        read_workspace_source("apps/runenwerk_editor/src/runtime/systems/frame_submit.rs");
    for forbidden in [
        "seed_viewport_binding_for_active_workspace",
        "sync_viewport_presentation_products_system",
        "build_surface_binding_registry",
        "viewport_products_registry.update_viewport_descriptors",
        "viewport_surface_bindings.replace_registry",
        "ensure_editor_main_surface_set",
        "initial_product_descriptors(",
        "viewport_surface_sets.retain_viewports",
        "viewport_presentations.retain_viewports",
        "ViewportSurfaceBindingRegistryResource",
        "ViewportProductRegistryResource",
    ] {
        assert!(
            !frame_submit.contains(forbidden),
            "editor frame submit must not own viewport identity/product binding lifecycle through marker '{forbidden}'",
        );
    }
}

#[test]
fn outliner_selection_routes_through_surface_presentation_and_ratification() {
    let dispatch_shell_command =
        read_workspace_source("apps/runenwerk_editor/src/shell/dispatch/outliner.rs");

    assert!(
        dispatch_shell_command.contains("build_outliner_surface_presentation_model"),
        "outliner selection should build from a prepared surface presentation model",
    );
    assert!(
        dispatch_shell_command.contains("SurfaceIntent::select_entity"),
        "outliner selection should route through typed surface intent emission",
    );
    assert!(
        dispatch_shell_command.contains("OutlinerEntitySelectionRatificationAdapter"),
        "outliner selection should ratify through adapter boundary, not direct mutation shortcut",
    );
}

#[test]
fn inspector_activation_routes_through_surface_presentation_and_ratification() {
    let dispatch_shell_command =
        read_workspace_source("apps/runenwerk_editor/src/shell/dispatch/inspector.rs");

    assert!(
        dispatch_shell_command.contains("build_inspector_surface_presentation_model"),
        "inspector activation should build from a prepared surface presentation model",
    );
    assert!(
        dispatch_shell_command.contains("SurfaceIntent::activate_field"),
        "inspector activation should route through typed surface intent emission",
    );
    assert!(
        dispatch_shell_command.contains("InspectorFieldActivationRatificationAdapter"),
        "inspector selection should ratify through adapter boundary, not direct mutation shortcut",
    );
}

#[test]
fn inspector_enum_mutation_routes_through_domain_contract_and_provider_dispatch() {
    let inspector_contract = read_workspace_source("domain/editor/editor_inspector/src/editing.rs");
    let shell_contract =
        read_workspace_source("domain/editor/editor_shell/src/surfaces/inspector.rs");
    let interaction_mapper =
        read_workspace_source("domain/editor/editor_shell/src/commands/map_interactions.rs");
    let provider =
        read_workspace_source("apps/runenwerk_editor/src/shell/providers/scene/inspector.rs");
    let dispatch = read_workspace_source("apps/runenwerk_editor/src/shell/dispatch/inspector.rs");

    assert!(
        inspector_contract.contains("EnumSymbol")
            && inspector_contract.contains("enum_symbol_edit_value_for_field"),
        "enum inspector mutation must be a reusable editor_inspector contract",
    );
    assert!(
        shell_contract.contains("SelectFieldEnum") && shell_contract.contains("SetFieldEnum"),
        "editor_shell must expose typed enum select and mutation actions",
    );
    assert!(
        interaction_mapper.contains("UiInteraction::SelectChanged")
            && interaction_mapper.contains("InspectorSurfaceAction::SelectFieldEnum")
            && interaction_mapper.contains("InspectorSurfaceAction::SetFieldEnum"),
        "enum select widgets must map selected indexes through shell routing, not provider-side UI parsing",
    );
    assert!(
        provider.contains("InspectorSessionMutation::SetFieldEnum")
            && dispatch.contains("commit_inspector_enum_field"),
        "runenwerk_editor providers and dispatch must carry enum mutation through session proposals",
    );
}

#[test]
fn reflected_ecs_enums_remain_unit_variant_typed_and_inspector_backed() {
    let type_info = read_workspace_source("domain/ecs/src/reflect/type_info.rs");
    let enum_info = read_workspace_source("domain/ecs/src/reflect/enum_info.rs");
    let value = read_workspace_source("domain/ecs/src/reflect/value.rs");
    let macros = read_workspace_source("domain/ecs_macros/src/lib.rs");
    let inspector_adapter =
        read_workspace_source("domain/editor/editor_inspector/src/bridge/ecs_adapter.rs");
    let inspector_editing = read_workspace_source("domain/editor/editor_inspector/src/editing.rs");

    assert!(
        type_info.contains("ReflectShape::Enum") && enum_info.contains("EnumVariantInfo"),
        "ECS reflection should expose enum shape metadata instead of stringly special cases",
    );
    assert!(
        enum_info.contains("EnumCurrentVariant") && enum_info.contains("EnumSetUnitVariant"),
        "reflected enums must expose typed current-variant and mutation function pointers",
    );
    assert!(
        value.contains("enum_ref(") && value.contains("enum_mut("),
        "dynamic reflected values should expose enum accessors beside struct accessors",
    );
    assert!(
        macros.contains("Data::Enum")
            && macros.contains("unit/no-payload variants")
            && macros.contains("ReflectShape::Enum"),
        "derive support must be intentionally limited to no-payload enum variants",
    );
    assert!(
        inspector_adapter.contains("InspectorValue::Enum")
            && inspector_editing.contains("set_unit_variant"),
        "editor_inspector should render and mutate reflected ECS enums through the typed enum contract",
    );
}

#[test]
fn viewport_surface_binary_options_use_reusable_toggles_with_typed_routing() {
    let builder =
        read_workspace_source("domain/editor/editor_shell/src/composition/build_viewport_panel.rs");
    let mapper =
        read_workspace_source("domain/editor/editor_shell/src/commands/map_interactions.rs");

    assert!(
        builder.contains("compact_surface_toggle(")
            && builder.contains("VIEWPORT_DETAILS_TOGGLE_WIDGET_ID")
            && builder.contains("VIEWPORT_ROOT_OPAQUE_TOGGLE_WIDGET_ID"),
        "viewport binary options should be projected as reusable toggle controls",
    );
    assert!(
        mapper.contains("ViewportSurfaceAction::SetRootBackgroundOpaque")
            && mapper.contains("enabled: checked"),
        "viewport toggle interactions must preserve typed provider-local routing",
    );
}

#[test]
fn self_authoring_live_activation_uses_domain_workspace_formation_and_versioned_exports() {
    let activation = read_workspace_source(
        "apps/runenwerk_editor/src/shell/applied_editor_definition/activation.rs",
    );
    let resources = read_workspace_source("apps/runenwerk_editor/src/runtime/resources.rs");
    let shell_workspace =
        read_workspace_source("domain/editor/editor_shell/src/workspace/definition_form.rs");
    let shell_surface_contract =
        read_workspace_source("domain/editor/editor_shell/src/workspace/surface_contract.rs");
    let self_authoring =
        read_workspace_source_tree_contents("apps/runenwerk_editor/src/shell/self_authoring");

    assert!(
        activation.contains("WorkspaceLayoutChanged")
            && activation.contains("EditorDefinitionDocumentContent::WorkspaceLayout"),
        "workspace layout definitions should produce a non-theme live activation contract",
    );
    assert!(
        resources.contains("form_workspace_state_from_definition_with_registry")
            && resources.contains("install_composition_runtime"),
        "app runtime should apply authored workspace layouts through editor_shell formation",
    );
    assert!(
        shell_workspace.contains("WorkspaceDefinitionFormationError")
            && shell_workspace.contains("tool_surface_kind_from_definition_key")
            && shell_surface_contract.contains("panel_kind_definition_key")
            && shell_surface_contract.contains("tool_surface_kind_definition_key"),
        "workspace definition formation and authored panel/tool-surface key mapping must live in editor_shell",
    );
    assert!(
        self_authoring.contains("EditorDefinitionExportPackage")
            && self_authoring.contains("EDITOR_DEFINITION_EXPORT_PACKAGE_VERSION"),
        "definition export should be a versioned package, not a bare document dump",
    );
}

#[test]
fn self_authoring_live_activation_updates_definition_catalogs_not_ui_definition_behavior() {
    let activation = read_workspace_source(
        "apps/runenwerk_editor/src/shell/applied_editor_definition/activation.rs",
    );
    let catalogs = read_workspace_source(
        "apps/runenwerk_editor/src/shell/applied_editor_definition/catalogs.rs",
    );
    let compatibility = read_workspace_source(
        "apps/runenwerk_editor/src/shell/applied_editor_definition/compatibility.rs",
    );
    let facade =
        read_workspace_source("apps/runenwerk_editor/src/shell/applied_editor_definition.rs");
    let resources = read_workspace_source("apps/runenwerk_editor/src/runtime/resources.rs");
    let shell_state = read_workspace_source("apps/runenwerk_editor/src/shell/state.rs");
    let frame_model = read_workspace_source("domain/editor/editor_shell/src/surface_provider.rs");
    let shell_composition =
        read_workspace_source("domain/editor/editor_shell/src/composition/build_editor_shell.rs");

    assert!(
        facade.contains("pub use activation")
            && facade.contains("pub use catalogs")
            && catalogs.contains("ActiveEditorDefinitionCatalogs")
            && activation.contains("UiTemplateCatalogChanged")
            && activation.contains("CommandBindingCatalogChanged")
            && activation.contains("PanelRegistryCatalogChanged")
            && activation.contains("ToolSurfaceRegistryCatalogChanged"),
        "applied definitions should produce explicit catalog activation contracts",
    );
    assert!(
        resources.contains("install_editor_bindings")
            && resources.contains("install_panel_registry")
            && resources.contains("install_tool_surface_registry"),
        "runtime activation must install formed catalogs and block invalid registry replacement",
    );
    assert!(
        shell_state.contains("active_editor_definitions")
            && frame_model.contains("with_active_ui_definitions")
            && frame_model.contains("with_available_tool_surface_create_candidates")
            && shell_composition.contains("active_toolbar_template"),
        "live catalog activation should feed the next shell frame instead of remaining a snapshot only",
    );
    assert!(
        catalogs.contains("command_for_route_target")
            && catalogs.contains("available_tool_surface_keys")
            && compatibility.contains("known_tool_surface_keys_in_authored_order")
            && compatibility.contains("panel_registry_covers_composition")
            && compatibility.contains("tool_surface_registry_covers_panel_defaults"),
        "active command and tool-surface catalogs should expose app-owned routing/future-creation seams",
    );
}

#[test]
fn surface_dispatch_facade_delegates_to_surface_handlers() {
    let facade = read_workspace_source("apps/runenwerk_editor/src/shell/dispatch_shell_command.rs");
    let handlers = [
        "apps/runenwerk_editor/src/shell/dispatch/outliner.rs",
        "apps/runenwerk_editor/src/shell/dispatch/entity_table.rs",
        "apps/runenwerk_editor/src/shell/dispatch/inspector.rs",
        "apps/runenwerk_editor/src/shell/dispatch/viewport.rs",
    ];

    assert!(
        facade.contains("ShellCommand::ApplySurfaceSessionMutation")
            && facade.contains("ShellCommand::ApplyEditorDomainMutation")
            && facade.contains("dispatch_surface_session_mutation")
            && facade.contains("dispatch_editor_domain_mutation"),
        "surface command facade must delegate through the typed generic wrappers",
    );
    for forbidden in [
        "ShellCommand::SelectEntityTableEntity",
        "ShellCommand::ToggleEntityTableSort",
        "ShellCommand::SelectOutlinerEntity",
        "ShellCommand::SelectViewportProduct",
        "ShellCommand::ToggleViewportDetails",
        "ShellCommand::ActivateInspectorField",
        "ShellCommand::AppendInspectorFieldText",
    ] {
        assert!(
            !facade.contains(forbidden),
            "surface-specific command branch must not remain in dispatch facade: {forbidden}",
        );
    }
    for path in handlers {
        let source = read_workspace_source(path);
        assert!(
            source.contains("dispatch_"),
            "surface handler '{path}' must own concrete dispatch behavior",
        );
    }
}

#[test]
fn surface_session_mutations_are_structurally_gated_before_session_state_access() {
    let entity_table =
        read_workspace_source("apps/runenwerk_editor/src/shell/dispatch/entity_table.rs");
    let inspector = read_workspace_source("apps/runenwerk_editor/src/shell/dispatch/inspector.rs");
    let viewport = read_workspace_source("apps/runenwerk_editor/src/shell/dispatch/viewport.rs");
    let sdf_operations =
        read_workspace_source("apps/runenwerk_editor/src/shell/dispatch/sdf_operations.rs");

    for (surface, source, expected_kind_marker) in [
        (
            "entity_table",
            &entity_table,
            "ToolSurfaceKind::EntityTable",
        ),
        ("inspector", &inspector, "ToolSurfaceKind::Inspector"),
        ("viewport", &viewport, "ToolSurfaceKind::Viewport"),
    ] {
        assert!(
            source.contains("resolve_legacy_surface_command_contract")
                && source.contains(expected_kind_marker)
                && source.contains("surface-kind mismatch")
                && source.contains("SurfaceCapability::Observe")
                && source.contains("SurfaceCapability::Interact"),
            "{surface} session mutations must resolve the structural target, reject wrong surface kinds, and gate interactive state changes by capability",
        );
    }
    assert!(
        !entity_table.contains(
            "resolve_legacy_surface_command_contract(shell_state, target, ToolSurfaceKind::EntityTable).is_none()"
        ),
        "entity-table session dispatch must not treat any existing active tool surface as an entity-table session target",
    );
    assert!(
        sdf_operations.contains("ToolSurfaceKind::FieldLayerStack")
            && sdf_operations.contains("ToolSurfaceKind::SdfGraphCanvas")
            && sdf_operations.contains("SurfaceCapability::RequestMutation")
            && sdf_operations.contains("contract.capabilities.allows"),
        "SDF operation dispatch must stay routed to SDF-owned surfaces and require mutation capability for domain changes",
    );
}

#[test]
fn ui_definition_does_not_import_editor_provider_behavior() {
    let sources = read_workspace_source_tree("domain/ui/ui_definition/src");
    let forbidden_terms = [
        "RunenwerkEditorApp",
        "SurfaceLocalAction",
        "SurfaceSessionMutation",
        "EditorDomainMutation",
        "EntityTableSurfaceAction",
        "InspectorSurfaceAction",
        "ViewportSurfaceAction",
        "OutlinerSurfaceAction",
    ];
    let offenders = forbidden_source_markers(&sources, &forbidden_terms);
    assert!(
        offenders.is_empty(),
        "ui_definition must stay generic and provider-behavior free: {offenders:?}",
    );
}

#[test]
fn app_surface_providers_delegate_reusable_control_composition_to_editor_shell() {
    let provider_sources = read_workspace_source_tree("apps/runenwerk_editor/src/shell/providers");
    let forbidden_terms = [
        "editor_shell::button(",
        "editor_shell::button_selected(",
        "editor_shell::toggle(",
        "editor_shell::select(",
        "editor_shell::table(",
        "editor_shell::tree(",
        "editor_shell::numeric_input(",
        "editor_shell::text_input(",
    ];
    let offenders = forbidden_source_markers(&provider_sources, &forbidden_terms);
    assert!(
        offenders.is_empty(),
        "app providers should pass DTOs and route proposals to editor_shell builders instead of constructing reusable controls directly: {offenders:?}",
    );

    let provider_source_text =
        read_workspace_source_tree_contents("apps/runenwerk_editor/src/shell/providers");
    let shell_builder = read_workspace_source(
        "domain/editor/editor_shell/src/composition/build_self_authoring_control_panel.rs",
    );
    assert!(
        provider_source_text.contains("build_self_authoring_control_panel(")
            && shell_builder.contains("compact_surface_action_button")
            && shell_builder.contains("SurfaceRouteTable"),
        "self-authoring control panel composition should live in editor_shell while provider code supplies actions and routes",
    );
}

#[test]
fn wr021_material_product_spine_runtime_boundaries_are_consumed() {
    let app_flow = read_workspace_source("apps/runenwerk_editor/src/runtime/app.rs");
    assert!(
        app_flow.contains("material_scene_shader_asset(EDITOR_VIEWPORT_SCENE_PRODUCT_SHADER_ID)")
            && !app_flow.contains("material_scene_shader_asset(EDITOR_VIEWPORT_SCENE_PRODUCT_SHADER_ID)\n        .for_feature"),
        "WR-021 scene pass must resolve generated scene bundles without feature-gating the fallback scene producer",
    );

    let provenance =
        read_workspace_source("engine/src/plugins/render/renderer/render_flow/provenance.rs");
    assert!(
        provenance.contains("RenderShaderReference::MaterialSceneBundle")
            && provenance.contains("material.scene_bundle.as_ref()")
            && provenance.contains("scene_bundle.shader_path")
            && provenance.contains("fallback_used: true"),
        "PreparedSceneMaterialBundle must be consumed during shader resolution, and missing generated bundles must be treated as forbidden fallbacks",
    );

    let pipeline_realization = read_workspace_source(
        "engine/src/plugins/render/renderer/render_flow/pipeline_realization.rs",
    );
    let bindings =
        read_workspace_source("engine/src/plugins/render/renderer/render_flow/bindings.rs");
    assert!(
        pipeline_realization.contains("resolve_shader_material_for_packet")
            && pipeline_realization.contains("reject_material_shader_fallback(")
            && pipeline_realization.contains("reject_unresident_material_textures(")
            && pipeline_realization.contains("pass_consumes_material_resources")
            && pipeline_realization.contains("builtin or scene-bundle fallback is forbidden")
            && bindings.contains("gpu_material_runtime_binding_values_for_pass")
            && bindings.contains("GpuRuntimeBindingSet::new(")
            && bindings.contains("runtime_bindings,")
            && !bindings.contains("context.realize_bind_group("),
        "material feature passes must enter the complete typed runtime binding set for G5 preparation and fail closed instead of falling back to old scene shaders",
    );

    let material_runtime_state =
        read_workspace_source("apps/runenwerk_editor/src/material_lab/state/runtime.rs");
    let editor_runtime =
        read_workspace_source("apps/runenwerk_editor/src/editor_runtime/runtime.rs");
    let frame_submit =
        read_workspace_source("apps/runenwerk_editor/src/runtime/systems/frame_submit.rs");
    assert!(
        !material_runtime_state.contains("SceneMaterialAssignmentState")
            && !material_runtime_state.contains("primitive_material_slots:")
            && editor_runtime.contains("SceneMaterialAssignmentState")
            && editor_runtime.contains("scene_material_assignments")
            && frame_submit.contains("runtime.material_slot_index_for_entity(entity)")
            && frame_submit.contains("with_material_slot_index"),
        "editor_scene SceneMaterialAssignmentState must drive viewport scene packet material slot indices, not a MaterialLabRuntime-owned primitive slot map",
    );
}

#[test]
fn wr029_phase4_requires_real_model_mesh_renderable_contract_before_pixel_claim() {
    let wr029_contract = read_workspace_source(
        "docs-site/src/content/docs/reports/implementation-plans/wr-029-model-mesh-material-binding/plan.md",
    );
    let rendered_world_design = read_workspace_source(
        "docs-site/src/content/docs/design/implemented/editor-rendered-world-and-multi-entity-viewport-design.md",
    );
    let wr030_contract = read_workspace_source(
        "docs-site/src/content/docs/reports/implementation-plans/wr-030-model-mesh-renderable-scene-contract/plan.md",
    );

    assert!(
        wr029_contract.contains("WR-030")
            && wr029_contract.contains("must not use the current SDF pass")
            && wr029_contract.contains("descriptor-only")
            && wr029_contract.contains("proof as a substitute"),
        "WR-029 Phase 4 must stay blocked on a real model/mesh renderable contract instead of claiming SDF or descriptor-only evidence",
    );
    assert!(
        rendered_world_design.contains("WR-018 V1 originally excluded")
            && rendered_world_design.contains("general mesh scene extraction"),
        "the implemented rendered-world design must retain WR-018's historical no-general-mesh-scene-extraction boundary before WR-029 claims model/mesh pixels",
    );
    assert!(
        wr030_contract.contains("first implementation surface is a Mesh Preview product surface")
            && wr030_contract.contains("Do not add general mesh scene extraction")
            && wr030_contract.contains("add Mesh Preview beside the existing SDF scene path"),
        "WR-030 must stay product-surface-first until a broader mesh scene extraction ADR/design is accepted",
    );
}

#[test]
fn wr021_material_descriptors_and_ui_do_not_fall_back_to_old_shortcuts() {
    let resource_resolution =
        read_workspace_source("apps/runenwerk_editor/src/material_lab/resource_resolution.rs");
    assert!(
        resource_resolution.contains("catalog-persisted texture descriptor")
            && !resource_resolution.contains("TextureExtent::new(1, 1, 1)")
            && !resource_resolution.contains("TextureExtent::new(1, 1, 2)"),
        "material resource resolution must use catalog texture descriptors, not fabricated texture metadata",
    );

    let material_provider =
        read_workspace_source("apps/runenwerk_editor/src/shell/providers/material_graph_canvas.rs");
    let material_shell_builder = read_workspace_source(
        "domain/editor/editor_shell/src/composition/build_material_graph_surface.rs",
    );
    assert!(
        !material_provider.contains("build_self_authoring_control_panel(")
            && material_provider.contains("build_material_graph_surface")
            && material_shell_builder.contains("view_model.graph.graph_editor.canvas"),
        "Material Graph Canvas must project typed graph surface data instead of the generic line/control-panel helper",
    );

    let material_state =
        read_workspace_source_tree_contents("apps/runenwerk_editor/src/material_lab/state");
    let material_catalog = read_workspace_source("domain/material_graph/src/catalog.rs");
    assert!(
        material_catalog.contains("from_port_type_id")
            && !material_state.contains("_ => MaterialValueType::Float"),
        "unknown material port types must not be silently projected as Float",
    );
}

#[test]
fn shell_chrome_stable_key_only_surfaces_do_not_fallback_to_viewport() {
    let shell_builder =
        read_workspace_source("domain/editor/editor_shell/src/composition/build_editor_shell.rs");
    let shell_commands =
        read_workspace_source("domain/editor/editor_shell/src/commands/shell_command.rs");
    let controller = read_workspace_source("apps/runenwerk_editor/src/shell/controller.rs");

    assert!(
        shell_builder.contains("SplitTabStackAreaStableKey")
            && shell_builder.contains("ResetTabStackAreaStableKey")
            && shell_builder.contains("LockTabStackAreaStableKey")
            && shell_builder.contains("CreatePanelTabStableKey")
            && shell_builder.contains("ToolSurfaceStableKey")
            && shell_commands.contains("SplitTabStackAreaStableKey")
            && shell_commands.contains("ResetTabStackAreaStableKey")
            && shell_commands.contains("LockTabStackAreaStableKey"),
        "shell chrome must have stable-key-native create/split/reset/lock paths",
    );
    assert!(
        !shell_builder.contains("tab_stack_supports_legacy_switch_menu")
            && !shell_builder.contains("legacy_active_tool_surface_kind")
            && !shell_commands.contains("SwitchPanelToolSurfaceKind"),
        "shell chrome must not retain legacy switch action routing",
    );
    assert!(
        !shell_builder.contains("unwrap_or(ToolSurfaceKind::Viewport)")
            && !controller.contains("unwrap_or(ToolSurfaceKind::Viewport)")
            && !controller.contains("return ToolSurfaceKind::Viewport"),
        "stable-key-only shell chrome must not silently remap missing legacy metadata to viewport",
    );
}

#[test]
fn production_workspace_layout_readers_are_registry_aware() {
    let dispatch =
        read_workspace_source("apps/runenwerk_editor/src/shell/dispatch_shell_command.rs");
    let persistence =
        read_workspace_source("apps/runenwerk_editor/src/persistence/workspace_layout.rs");

    assert!(
        dispatch.contains("load_editor_composition_layout")
            && !dispatch.contains("read_workspace_layout_legacy_no_registry")
            && !dispatch.contains("read_workspace_layout_with_metadata_legacy_no_registry"),
        "production shell workspace load paths must use composition layout readers",
    );
    assert!(
        persistence.contains("read_workspace_layout_legacy_no_registry")
            && persistence.contains("Legacy/test compatibility reader"),
        "no-registry readers must be explicitly named and documented as legacy/test compatibility",
    );
}

#[test]
fn viewport_embed_shader_uses_full_prepared_product_rect() {
    let shader = read_workspace_source("engine/src/plugins/render/renderer/mod.rs");

    assert!(
        shader.contains("input.uv_rect.x + input.uv_rect.z * input.local.x")
            && shader.contains("input.uv_rect.y + input.uv_rect.w * input.local.y"),
        "viewport embed shader must sample the full prepared product rect; product sizing owns aspect correctness",
    );
    assert!(
        !shader.contains("textureDimensions(viewport_texture)")
            && !shader.contains("fit_origin")
            && !shader.contains("fit_size"),
        "viewport embed shader must not apply a second implicit fit that diverges from picking bounds",
    );
}

#[test]
fn viewport_surface_bindings_accept_only_dynamic_texture_sources() {
    let sources = read_workspace_sources(&[
        "domain/ui/ui_render_data/src/lib.rs",
        "domain/ui/ui_render_data/src/primitives/viewport_surface_embed.rs",
        "engine/src/plugins/render/renderer/setup.rs",
        "engine/src/plugins/render/renderer/render_flow/runtime_resources/resolve.rs",
        "apps/runenwerk_editor/tests/startup_render_smoke.rs",
    ]);
    let forbidden_terms = [
        concat!("ViewportSurfaceBinding", "::new("),
        concat!("ViewportSurfaceBinding", "::flow_", "resource("),
        concat!("ViewportSurfaceBindingSource", "::flow_", "resource("),
        concat!("ViewportSurfaceBindingSource", "::Flow", "Resource"),
        concat!("Legacy", "ViewportSurfaceBindingShape"),
        concat!("flow_", "resource_parts"),
        concat!("resolve_ui_", "texture_view("),
    ];
    let offenders = forbidden_source_markers(&sources, &forbidden_terms);

    assert!(
        offenders.is_empty(),
        "viewport surface bindings must resolve only dynamic texture targets, not legacy flow resources: {offenders:?}",
    );
}

#[test]
fn viewport_render_product_publishers_use_producer_scoped_contributions() {
    let sources = read_workspace_sources(&[
        "apps/runenwerk_editor/src/runtime/viewport/render_jobs.rs",
        "apps/runenwerk_editor/src/runtime/viewport/product_targets.rs",
    ]);
    let forbidden_terms = [
        "prepared_frame_requests.clear(",
        ".add_view(",
        ".add_flow_invocation(",
        ".replace_requests(",
    ];
    let offenders = forbidden_source_markers(&sources, &forbidden_terms);

    assert!(
        offenders.is_empty(),
        "viewport render product publishers must replace their own producer contribution instead of mutating global request state: {offenders:?}",
    );
    assert!(
        sources
            .iter()
            .any(|(_, source)| source.contains(".replace_contribution(")),
        "viewport render product publishers must use producer-scoped contribution APIs",
    );
}

#[test]
fn renderer_uniform_uploads_are_invocation_scoped() {
    let execute =
        read_workspace_source("engine/src/plugins/render/renderer/render_flow/execute.rs");
    let runtime_resources = read_workspace_source(
        "engine/src/plugins/render/renderer/render_flow/runtime_resources.rs",
    );
    let realize = read_workspace_source(
        "engine/src/plugins/render/renderer/render_flow/runtime_resources/realize.rs",
    );
    let resolve = read_workspace_source(
        "engine/src/plugins/render/renderer/render_flow/runtime_resources/resolve.rs",
    );
    let inspect = read_workspace_source(
        "engine/src/plugins/render/renderer/render_flow/runtime_resources/inspect.rs",
    );
    let bindings =
        read_workspace_source("engine/src/plugins/render/renderer/render_flow/bindings.rs");
    let logical_operations = read_workspace_source(
        "engine/src/plugins/render/renderer/render_flow/logical_operations.rs",
    );

    assert!(
        execute.contains("set_active_invocation_uniform_scope")
            && execute.contains("realize_invocation_uniform_buffer"),
        "renderer must bind invocation-local uniform storage before encoding each prepared flow invocation",
    );
    assert!(
        runtime_resources.contains("invocation_uniform_buffers")
            && runtime_resources.contains("active_invocation_uniform_scope")
            && realize.contains("retain_invocation_uniform_scopes")
            && inspect.contains("RuntimeResourceKey::InvocationUniform"),
        "flow runtime resources must retain renderer-owned uniform buffers per invocation, not one mutable shared upload target",
    );
    assert!(
        resolve.contains("RuntimeResourceKey::InvocationUniform")
            && bindings.contains("handle: runen_gpu::GpuBufferHandle")
            && bindings.contains("GpuRuntimeBufferBinding::new(")
            && bindings.contains("GpuRuntimeBindingValue::new(value.key, [resource])")
            && bindings.contains("GpuRuntimeBindingSet::new(")
            && bindings.contains("runtime_bindings,")
            && !bindings.contains("context.realize_bind_group(")
            && logical_operations.contains("pipeline.bindings.runtime_bindings.clone()"),
        "canonical G5 operations must preserve invocation-local typed handles through public RunenGPU runtime bindings so another invocation cannot reuse the wrong binding state",
    );
}

#[test]
fn viewport_slot_semantics_stay_in_editor_viewport_and_payload_slots_stay_opaque() {
    let viewport_surface_embed =
        include_str!("../../../domain/ui/ui_render_data/src/primitives/viewport_surface_embed.rs");
    let viewport_surface_semantics =
        include_str!("../../../domain/editor/editor_viewport/src/expression/surface_set.rs");

    assert!(
        viewport_surface_embed.contains("struct ViewportSurfaceEmbedSlotId"),
        "ui_render_data should expose an opaque renderer-facing embed slot id",
    );
    assert!(
        !viewport_surface_embed.contains("enum ViewportSurfaceSlot"),
        "ui_render_data must not own viewport semantic slot taxonomy",
    );
    assert!(
        viewport_surface_semantics.contains("enum ViewportSurfaceSlot"),
        "editor_viewport remains the semantic owner for viewport slot taxonomy",
    );
}

#[test]
fn viewport_slot_mapping_happens_at_integration_edge() {
    let presentation_resolver = include_str!("../src/runtime/viewport/presentation_resolver.rs");
    let viewport_embed_mapping =
        include_str!("../../../domain/editor/editor_shell/src/workspace/viewport_embed_slot.rs");

    assert!(
        viewport_embed_mapping.contains("viewport_embed_slot_for"),
        "editor_shell should own integration-edge mapping from semantic slots to payload slot IDs",
    );
    assert!(
        presentation_resolver.contains("viewport_embed_slot_for"),
        "runenwerk_editor viewport resolver should consume adapter mapping rather than defining parallel slot taxonomy",
    );
    assert!(
        !presentation_resolver.contains("UiViewportSurfaceSlot::"),
        "presentation resolver must not define a second canonical semantic slot enum in runenwerk_editor",
    );
}

fn workspace_root() -> std::path::PathBuf {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .and_then(Path::parent)
        .expect("runenwerk_editor lives under apps/")
        .to_path_buf()
}

fn read_workspace_source(path: &str) -> String {
    let full_path = workspace_root().join(path);
    fs::read_to_string(&full_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", full_path.display()))
}

fn read_workspace_sources(paths: &[&'static str]) -> Vec<(String, String)> {
    paths
        .iter()
        .map(|path| ((*path).to_string(), read_workspace_source(path)))
        .collect()
}

fn read_workspace_source_tree(path: &str) -> Vec<(String, String)> {
    let root = workspace_root();
    let mut files = Vec::new();
    collect_source_files(&root.join(path), &mut files);
    files.sort();
    files
        .into_iter()
        .filter_map(|file| {
            let display_path = file
                .strip_prefix(&root)
                .unwrap_or(&file)
                .display()
                .to_string();
            fs::read_to_string(&file)
                .ok()
                .map(|source| (display_path, source))
        })
        .collect()
}

fn read_workspace_source_tree_contents(path: &str) -> String {
    read_workspace_source_tree(path)
        .into_iter()
        .map(|(_, source)| source)
        .collect::<Vec<_>>()
        .join("\n")
}

fn forbidden_source_markers(sources: &[(String, String)], forbidden_terms: &[&str]) -> Vec<String> {
    sources
        .iter()
        .flat_map(|(file, source)| {
            forbidden_terms
                .iter()
                .filter(move |term| source.contains(**term))
                .map(move |term| format!("{file}: {term}"))
        })
        .collect()
}

fn collect_source_files(root: &Path, files: &mut Vec<std::path::PathBuf>) {
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_source_files(&path, files);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            files.push(path);
        }
    }
}

fn provider_sources() -> String {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let provider_root = manifest_dir.join("src/shell/providers");
    let mut files = Vec::new();
    collect_source_files(&provider_root, &mut files);
    files.sort();
    files
        .into_iter()
        .filter_map(|path| fs::read_to_string(path).ok())
        .collect::<Vec<_>>()
        .join("\n")
}
