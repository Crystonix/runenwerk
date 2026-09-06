use editor_core::{ChangeOrigin, ComponentTypeId, EntityId};
use editor_inspector::{InspectorEditValue, InspectorPath};
use editor_scene::SceneCommandIntent;
use editor_shell::ShellCommand;
use editor_viewport::ViewportHitResult;
use scene::{LocalTransform, Vec3Value};

use runenwerk_editor::editor_app::RunenwerkEditorApp;
use runenwerk_editor::editor_features::viewport::ViewportInteractionCommand;
use runenwerk_editor::editor_features::{
    execute_intent_with_history, redo_last_scene_change, undo_last_scene_change,
};
use runenwerk_editor::editor_panels::OutlinerPanelCommand;
use runenwerk_editor::shell::dispatch_shell_command;

#[derive(Debug, Clone, Default, ecs::Reflect)]
struct Vec2 {
    x: f32,
    y: f32,
}

#[derive(Debug, Clone, Default, ecs::Component, ecs::Reflect)]
struct Position {
    value: Vec2,
    speed: f32,
}

#[test]
fn scene_authoring_workflow_smoke_select_edit_translate_undo_redo() {
    let mut app = RunenwerkEditorApp::new();
    let position_type = ComponentTypeId(101);
    let transform_type = ComponentTypeId(102);

    app.runtime_mut()
        .register_component_type::<Position>(position_type);
    app.runtime_mut()
        .register_component_type::<LocalTransform>(transform_type);

    execute_intent_with_history(
        app.runtime_mut(),
        "Create Entity",
        SceneCommandIntent::CreateEntity {
            parent: None,
            display_name: "Entity".to_string(),
        },
    )
    .expect("entity create should succeed");

    execute_intent_with_history(
        app.runtime_mut(),
        "Add Position",
        SceneCommandIntent::AddComponent {
            entity: EntityId(1),
            component_type: position_type,
        },
    )
    .expect("position add should succeed");

    execute_intent_with_history(
        app.runtime_mut(),
        "Add LocalTransform",
        SceneCommandIntent::AddComponent {
            entity: EntityId(1),
            component_type: transform_type,
        },
    )
    .expect("transform add should succeed");

    app.dispatch_outliner_command(OutlinerPanelCommand::SelectEntity {
        entity: EntityId(1),
    })
    .expect("outliner selection should succeed");

    execute_intent_with_history(
        app.runtime_mut(),
        "Edit Position Speed",
        SceneCommandIntent::EditComponentField {
            entity: EntityId(1),
            component_type: position_type,
            path: InspectorPath::root().child_field("speed"),
            value: InspectorEditValue::Float(9.5),
        },
    )
    .expect("inspector edit should succeed");

    let workspace = default_workspace_state();
    let viewport_surface = default_surface_by_kind(&workspace, editor_shell::PanelKind::Viewport);
    let viewport_mounted_unit =
        ui_composition::MountedUnitId::try_from_raw(viewport_surface.raw()).unwrap();
    app.dispatch_viewport_interaction_for_mounted_unit(
        viewport_mounted_unit,
        ViewportInteractionCommand::PointerDown {
            hit: ViewportHitResult::gizmo_axis("X", 0.0),
        },
    )
    .expect("viewport gizmo down should succeed");
    app.dispatch_viewport_interaction_for_mounted_unit(
        viewport_mounted_unit,
        ViewportInteractionCommand::PointerDragAxis { amount: 6.0 },
    )
    .expect("viewport drag should succeed");
    app.dispatch_viewport_interaction_for_mounted_unit(
        viewport_mounted_unit,
        ViewportInteractionCommand::PointerUp,
    )
    .expect("viewport up should succeed");

    let ecs_entity = app
        .runtime()
        .ids()
        .resolve_entity(EntityId(1))
        .expect("entity mapping should exist");

    let transform = app
        .runtime()
        .world()
        .get::<LocalTransform>(ecs_entity)
        .expect("local transform should exist");
    assert_eq!(transform.translation, Vec3Value::new(6.0, 0.0, 0.0));

    dispatch_shell_command(
        &mut app,
        None,
        ShellCommand::ActivateRotateTool,
        None,
        None,
        None,
        None,
    )
    .expect("rotate tool activation should succeed");
    app.dispatch_viewport_interaction_for_mounted_unit(
        viewport_mounted_unit,
        ViewportInteractionCommand::PointerDown {
            hit: ViewportHitResult::gizmo_axis("Y", 0.0),
        },
    )
    .expect("viewport rotate gizmo down should succeed");
    app.dispatch_viewport_interaction_for_mounted_unit(
        viewport_mounted_unit,
        ViewportInteractionCommand::PointerDragAxis {
            amount: std::f32::consts::FRAC_PI_2,
        },
    )
    .expect("viewport rotate drag should succeed");
    app.dispatch_viewport_interaction_for_mounted_unit(
        viewport_mounted_unit,
        ViewportInteractionCommand::PointerUp,
    )
    .expect("viewport rotate up should succeed");

    dispatch_shell_command(
        &mut app,
        None,
        ShellCommand::ActivateScaleTool,
        None,
        None,
        None,
        None,
    )
    .expect("scale tool activation should succeed");
    app.dispatch_viewport_interaction_for_mounted_unit(
        viewport_mounted_unit,
        ViewportInteractionCommand::PointerDown {
            hit: ViewportHitResult::gizmo_axis("Z", 0.0),
        },
    )
    .expect("viewport scale gizmo down should succeed");
    app.dispatch_viewport_interaction_for_mounted_unit(
        viewport_mounted_unit,
        ViewportInteractionCommand::PointerDragAxis { amount: 1.5 },
    )
    .expect("viewport scale drag should succeed");
    app.dispatch_viewport_interaction_for_mounted_unit(
        viewport_mounted_unit,
        ViewportInteractionCommand::PointerUp,
    )
    .expect("viewport scale up should succeed");
    dispatch_shell_command(
        &mut app,
        None,
        ShellCommand::ActivateTranslateTool,
        None,
        None,
        None,
        None,
    )
    .expect("translate tool activation should succeed");

    let transform = app
        .runtime()
        .world()
        .get::<LocalTransform>(ecs_entity)
        .expect("local transform should exist");
    assert_eq!(transform.scale, Vec3Value::new(1.0, 1.0, 2.5));

    let undone = undo_last_scene_change(app.runtime_mut(), ChangeOrigin::Runtime)
        .expect("undo should succeed")
        .expect("undo should return history entry");
    assert!(
        !undone.transaction.label.is_empty(),
        "undo should carry transaction metadata"
    );

    let transform_after_undo = app
        .runtime()
        .world()
        .get::<LocalTransform>(ecs_entity)
        .expect("local transform should exist after undo");
    assert_eq!(
        transform_after_undo.translation,
        Vec3Value::new(6.0, 0.0, 0.0)
    );

    let redone = redo_last_scene_change(app.runtime_mut(), ChangeOrigin::Runtime)
        .expect("redo should succeed")
        .expect("redo should return history entry");
    assert!(
        !redone.transaction.label.is_empty(),
        "redo should carry transaction metadata"
    );

    let transform_after_redo = app
        .runtime()
        .world()
        .get::<LocalTransform>(ecs_entity)
        .expect("local transform should exist after redo");
    assert_eq!(
        transform_after_redo.translation,
        Vec3Value::new(6.0, 0.0, 0.0)
    );

    let position = app
        .runtime()
        .world()
        .get::<Position>(ecs_entity)
        .expect("position should exist");
    assert_eq!(position.speed, 9.5);
}

fn default_workspace_state() -> editor_shell::WorkspaceState {
    let mut allocator = editor_shell::WorkspaceIdentityAllocator::new();
    let workspace_id = allocator.allocate_workspace_id();
    editor_shell::WorkspaceState::bootstrap_current_layout(workspace_id, &mut allocator)
}

fn default_surface_by_kind(
    workspace_state: &editor_shell::WorkspaceState,
    panel_kind: editor_shell::PanelKind,
) -> editor_shell::ToolSurfaceInstanceId {
    workspace_state
        .panels()
        .find(|panel| panel.panel_kind == panel_kind)
        .and_then(|panel| panel.active_tool_surface)
        .expect("default workspace should mount requested surface")
}
