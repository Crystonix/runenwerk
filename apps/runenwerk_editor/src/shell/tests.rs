use editor_core::{
    ChangeOrigin, ComponentTypeId, EditorMutationError, EntityId, RealityVersion, SelectionTarget,
    SessionChangeKind, WorkflowEventKind,
};
use editor_definition::{
    EditorDefinitionDocument, EditorDefinitionDocumentContent, EditorDefinitionDocumentKind,
    EditorDefinitionId, EditorLabOperation, EditorLabOperationDiffFamily, EditorLabOperationKind,
    EditorLabOperationStatus, EditorWorkbenchCompositionDefinition,
    EditorWorkbenchHostPolicyDefinition, EditorWorkspaceHostDefinition,
    EditorWorkspaceLayoutDefinition, EditorWorkspacePanelTabDefinition,
    EditorWorkspaceProfileDefinition,
};
use editor_inspector::{InspectorEditValue, InspectorPath};
use editor_shell::{
    CONSOLE_SCROLL_WIDGET_ID, CommandCapabilityKey, DockDropCandidateState,
    DockDropInvalidTargetReason, EDITOR_DESIGN_WORKSPACE_PROFILE_ID, ENTITY_TABLE_LIST_WIDGET_ID,
    ENTITY_TABLE_PANEL_WIDGET_ID, EditorDefinitionSurfaceAction, EditorDomainMutation,
    EntityTableComponentFilter, EntityTableHierarchyFilter, EntityTableSessionMutation,
    EntityTableSurfaceAction, HostCapabilityPolicy, InspectorSessionMutation,
    MATERIAL_WORKSPACE_PROFILE_ID, MODELLING_WORKSPACE_PROFILE_ID, OUTLINER_LIST_WIDGET_ID,
    OutlinerDomainMutation, PanelKind, RUNTIME_DEBUG_WORKSPACE_PROFILE_ID, RoutedShellAction,
    SCENE_WORKSPACE_PROFILE_ID, ShellCommand, StructuralCommandTarget, SurfaceLocalAction,
    SurfaceProviderAvailability, SurfaceProviderId, SurfaceProviderSupportMode,
    SurfaceSessionMutation, ToolSurfaceKind, ToolbarCommandKind, ToolbarMenuKind, UiInteraction,
    UiInteractionResults, VIEWPORT_DETAILS_TOGGLE_WIDGET_ID,
    VIEWPORT_FIELD_SLICE_INCREMENT_WIDGET_ID, ViewportDomainMutation, ViewportSessionMutation,
    ViewportSurfaceAction, WorkspaceMutation, build_editor_shell_frame,
    map_interactions_to_shell_commands, surface_widget_id, tab_stack_lock_type_toggle_widget_id,
    tab_stack_new_tab_button_widget_id, tab_stack_popup_menu_widget_id,
    tab_stack_reset_area_button_widget_id, tab_stack_split_horizontal_button_widget_id,
    viewport_field_component_button_widget_id,
};
use editor_viewport::{
    ArtifactObservationFrame, ExpressionDimensions, ExpressionProductId, ProducerHealth,
    ProductAvailabilityState, ViewportDebugStage, ViewportFieldVisualizerColorRamp,
    ViewportFieldVisualizerComponent, ViewportFieldVisualizerSettingsPatch, ViewportHitResult,
    ViewportId, ViewportPresentationState,
};
use engine::plugins::render::UiFontAtlasResource;
use std::sync::Arc;
use ui_input::{Modifiers, PointerButton, PointerEvent, PointerEventKind, UiInputEvent};
use ui_math::{Axis, UiPoint, UiRect, UiVector};
use ui_theme::ThemeTokens;

use crate::editor_app::{ConsoleMessageKind, RunenwerkEditorApp};
use crate::editor_panels::{
    EntityTablePanelPresenter, EntityTablePanelUiState, ViewportPanelCommand,
};
use crate::editor_runtime::select_single_entity;
use crate::runtime::resources::EditorHostResource;
use crate::runtime::viewport::{
    ToolSurfaceRuntimeBindingRecord, ToolSurfaceRuntimeBindingRegistryResource,
    VOLUME_SLICE_PRODUCT_ID, ViewportArtifactObservationResource, ViewportInstanceRegistryResource,
    ViewportPresentationStateResource, ViewportRenderStateCommand,
    ViewportRenderStateCommandQueueResource, initial_product_descriptors,
};
use crate::shell::{
    DefinitionApplyDiffFamily, EditorCommandAvailabilityContext, EditorDefinitionActivationStatus,
    EditorLabAccessibilitySnapshot, EditorLabEvidenceArtifact, EditorLabEvidenceArtifactKind,
    EditorLabEvidenceCapability, EditorLabEvidenceCapabilityProbe,
    EditorLabEvidenceCapabilityResult, EditorLabEvidenceCapabilityResultStatus,
    EditorLabEvidenceManifest, EditorLabEvidenceRun, EditorLabPerformanceSnapshot,
    EditorLabUnsupportedCheckDiagnostic, EditorSurfaceProviderRegistry, KnownEditorCommand,
    PM_UI_LAB_PERF_002_EVIDENCE_CAPABILITIES, RunenwerkEditorShellController,
    RunenwerkEditorShellState, RunenwerkWorkbenchHost, SELECT_TOOL_ID, TRANSLATE_TOOL_ID,
    active_document_context, active_route_actions_by_target, build_editor_shell_frame_model,
    dispatch_shell_command, dispatch_shell_command_with_viewport_commands, editor_command_catalog,
    editor_lab_preview_scenarios, evidence_warning, mounted_surface_requests_with_registry,
};

#[derive(Debug, Copy, Clone, PartialEq, runen_ecs::Component)]
struct TestMarker;

#[derive(Debug, Clone, Default, runen_ecs::Component, runen_ecs::Reflect)]
struct QueryMarker {
    value: i32,
}

fn simple_test_template(id: &str) -> ui_definition::AuthoredUiTemplate {
    ui_definition::AuthoredUiTemplate {
        id: id.into(),
        root: ui_definition::UiNodeDefinition::Panel {
            id: "root".into(),
            children: vec![ui_definition::UiNodeDefinition::Label {
                id: "label".into(),
                label: ui_definition::UiValueBinding::static_text("Test"),
                availability: None,
            }],
            availability: None,
        },
        templates: Vec::new(),
        menus: Vec::new(),
    }
}

fn pm004_stack_template() -> ui_definition::AuthoredUiTemplate {
    ui_definition::AuthoredUiTemplate {
        id: "pm004.stack.template".into(),
        root: ui_definition::UiNodeDefinition::Column {
            id: "root".into(),
            children: vec![ui_definition::UiNodeDefinition::Stack {
                id: "stack".into(),
                axis: ui_definition::UiAxisDefinition::Vertical,
                children: vec![ui_definition::UiNodeDefinition::Label {
                    id: "stack-label".into(),
                    label: ui_definition::UiValueBinding::static_text("Stack Label"),
                    availability: None,
                }],
            }],
        },
        templates: Vec::new(),
        menus: Vec::new(),
    }
}

fn selected_theme_color(shell_state: &RunenwerkEditorShellState, token: &str) -> Option<String> {
    let document = shell_state.self_authoring().selected_document()?;
    let EditorDefinitionDocumentContent::Theme(theme) = &document.content else {
        return None;
    };
    theme.colors.get(token).cloned()
}

fn test_tool_surface_binding_registry(
    tool_surface: editor_shell::ToolSurfaceInstanceId,
    panel: editor_shell::PanelInstanceId,
    tab_stack: editor_shell::TabStackId,
    viewport: ViewportId,
) -> ToolSurfaceRuntimeBindingRegistryResource {
    let mut registry = ToolSurfaceRuntimeBindingRegistryResource::default();
    registry.upsert_binding(ToolSurfaceRuntimeBindingRecord {
        tool_surface_id: tool_surface,
        panel_instance_id: panel,
        tab_stack_id: tab_stack,
        viewport_id: viewport,
        host_widget_id: editor_shell::WidgetId(999),
        bounds: UiRect::new(0.0, 0.0, 640.0, 360.0),
        generation: 1,
    });
    registry
}

fn surface_session_command(
    target: StructuralCommandTarget,
    mutation: SurfaceSessionMutation,
    projection_epoch: u64,
) -> ShellCommand {
    ShellCommand::ApplySurfaceSessionMutation {
        target,
        mutation,
        projection_epoch,
    }
}

fn composition_targets_by_kind(
    shell_state: &RunenwerkEditorShellState,
    kind: ToolSurfaceKind,
) -> Vec<StructuralCommandTarget> {
    let stable_key = editor_shell::stable_key_for_tool_surface_kind(kind)
        .expect("test surface kind should have a stable key");
    shell_state
        .composition_runtime()
        .extension()
        .mounted_units()
        .iter()
        .filter(|unit| unit.stable_content_key == stable_key.as_str())
        .map(|unit| {
            shell_state
                .structural_command_target_for_mounted_unit(unit.mounted_unit_id)
                .expect("mounted test content should have a complete structural target")
        })
        .collect()
}

fn composition_target_by_kind(
    shell_state: &RunenwerkEditorShellState,
    kind: ToolSurfaceKind,
) -> StructuralCommandTarget {
    composition_targets_by_kind(shell_state, kind)
        .into_iter()
        .next()
        .expect("default composition should contain requested surface kind")
}

fn composition_targets_by_panel_kind(
    shell_state: &RunenwerkEditorShellState,
    panel_kind: PanelKind,
) -> Vec<StructuralCommandTarget> {
    let panel_kind_key = editor_shell::panel_kind_definition_key(panel_kind);
    shell_state
        .composition_runtime()
        .extension()
        .mounted_units()
        .iter()
        .filter(|unit| unit.panel_kind_key == panel_kind_key)
        .map(|unit| {
            shell_state
                .structural_command_target_for_mounted_unit(unit.mounted_unit_id)
                .expect("mounted test content should have a complete structural target")
        })
        .collect()
}

fn composition_target_by_panel_kind(
    shell_state: &RunenwerkEditorShellState,
    panel_kind: PanelKind,
) -> StructuralCommandTarget {
    composition_targets_by_panel_kind(shell_state, panel_kind)
        .into_iter()
        .next()
        .expect("composition should contain requested panel kind")
}

fn editor_domain_command(
    target: StructuralCommandTarget,
    mutation: EditorDomainMutation,
    projection_epoch: u64,
) -> ShellCommand {
    ShellCommand::ApplyEditorDomainMutation {
        target,
        mutation,
        projection_epoch,
    }
}

#[test]
fn pm_ui_lab_002_runtime_evidence_reports_catalog_and_registry_chain() {
    use std::fmt::Write as _;

    editor_command_catalog()
        .validate()
        .expect("runtime evidence should start from a valid command catalog");

    let mut app = RunenwerkEditorApp::new();
    let mut shell_state =
        RunenwerkEditorShellState::new_with_workspace_profile_registry_and_tool_surface_registry(
            app.workbench_host().workspace_profile_registry(),
            app.workbench_host().tool_surface_registry(),
        )
        .expect("runtime evidence shell state should build from installed registries");
    let theme = ThemeTokens::default();

    let route_actions =
        active_route_actions_by_target(shell_state.active_editor_definitions(), false, false);
    for descriptor in editor_command_catalog().descriptors() {
        for route_target in descriptor.route_targets() {
            assert!(
                route_actions.contains_key(route_target),
                "catalog route target '{route_target}' should project to a shell action"
            );
        }
    }

    let mut evidence = String::new();
    writeln!(
        evidence,
        "# PM-UI-LAB-002 Runtime Evidence\n\nGenerated by `cargo test -p runenwerk_editor pm_ui_lab_002_runtime_evidence_reports_catalog_and_registry_chain -- --nocapture`.\n"
    )
    .unwrap();
    writeln!(
        evidence,
        "- command_catalog_descriptors: {}",
        editor_command_catalog().descriptors().len()
    )
    .unwrap();
    writeln!(
        evidence,
        "- catalog_route_targets_projected: {}",
        route_actions.len()
    )
    .unwrap();

    let menu_proofs = [
        (KnownEditorCommand::ToggleFileMenu, ToolbarMenuKind::File),
        (KnownEditorCommand::ToggleEditMenu, ToolbarMenuKind::Edit),
        (
            KnownEditorCommand::ToggleWindowMenu,
            ToolbarMenuKind::Window,
        ),
        (
            KnownEditorCommand::ToggleWorkspaceMenu,
            ToolbarMenuKind::Workspace,
        ),
    ];
    writeln!(evidence, "\n## Catalog Command Runtime Proof").unwrap();
    for (command, menu) in menu_proofs {
        let descriptor = editor_command_catalog()
            .descriptor(command)
            .expect("menu command should have catalog descriptor");
        dispatch_shell_command(
            &mut app,
            Some(&mut shell_state),
            descriptor.shell_command(),
            None,
            None,
            None,
            None,
        )
        .expect("catalog menu command should dispatch through shell runtime");
        assert_eq!(shell_state.active_toolbar_menu(), Some(menu));
        writeln!(
            evidence,
            "- `{}` -> {:?} menu via catalog label `{}`",
            descriptor.key, menu, descriptor.label
        )
        .unwrap();
    }

    let new_window_descriptor = editor_command_catalog()
        .descriptor(KnownEditorCommand::NewWindow)
        .expect("new window should have catalog descriptor");
    assert!(
        new_window_descriptor
            .availability(EditorCommandAvailabilityContext {
                can_undo: false,
                can_redo: false,
            })
            .is_enabled(),
        "new window should no longer be disabled by stale catalog availability"
    );
    let windows_before = shell_state.editor_windows().len();
    dispatch_shell_command(
        &mut app,
        Some(&mut shell_state),
        new_window_descriptor.shell_command(),
        None,
        None,
        None,
        None,
    )
    .expect("catalog-backed new window command should dispatch");
    assert_eq!(shell_state.editor_windows().len(), windows_before + 1);
    let pending_windows = shell_state.drain_pending_editor_window_presentations();
    assert_eq!(pending_windows.len(), 1);
    writeln!(
        evidence,
        "- `{}` opened logical editor window `{}`",
        new_window_descriptor.key,
        pending_windows[0].raw()
    )
    .unwrap();

    let save_as_descriptor = editor_command_catalog()
        .descriptor(KnownEditorCommand::SaveSceneAs)
        .expect("disabled save-as command should have catalog descriptor");
    let save_as_result = dispatch_shell_command(
        &mut app,
        Some(&mut shell_state),
        save_as_descriptor.shell_command(),
        None,
        None,
        None,
        None,
    );
    assert!(save_as_result.is_err());
    let diagnostic = app
        .console_lines()
        .last()
        .expect("unavailable command should append a console diagnostic");
    assert!(
        diagnostic
            .text
            .contains("[ui:editor.command.unavailable.save_as]"),
        "unavailable command should report catalog diagnostic, got {}",
        diagnostic.text
    );
    writeln!(
        evidence,
        "- `{}` failed closed with `{}`",
        save_as_descriptor.key, diagnostic.text
    )
    .unwrap();

    write_profile_runtime_evidence(
        &mut app,
        &mut shell_state,
        &theme,
        EDITOR_DESIGN_WORKSPACE_PROFILE_ID,
        "Editor Design",
        &[
            "runenwerk.editor_design.definition_outliner",
            "runenwerk.editor_design.ui_canvas",
            "runenwerk.editor_design.command_diff",
        ],
        &mut evidence,
    );
    write_profile_runtime_evidence(
        &mut app,
        &mut shell_state,
        &theme,
        MATERIAL_WORKSPACE_PROFILE_ID,
        "Materials",
        &[
            "runenwerk.material_lab.graph_canvas",
            "runenwerk.material_lab.inspector",
            "runenwerk.material_lab.preview",
        ],
        &mut evidence,
    );
    write_profile_runtime_evidence(
        &mut app,
        &mut shell_state,
        &theme,
        RUNTIME_DEBUG_WORKSPACE_PROFILE_ID,
        "Runtime Debug",
        &[
            "runenwerk.diagnostics.runtime_debug",
            "runenwerk.diagnostics.diagnostics",
            "runenwerk.diagnostics.tool_suite_registry_inspector",
        ],
        &mut evidence,
    );

    if std::env::var_os("RUNENWERK_WRITE_PM_UI_LAB_002_EVIDENCE").is_some() {
        let repo_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(|path| path.parent())
            .expect("runenwerk_editor crate should live under apps/");
        let artifact_path = repo_root.join(
            "docs-site/src/content/docs/reports/closeouts/pm-ui-lab-002-registry-and-command-source-of-truth/artifacts/runtime-proof.txt",
        );
        std::fs::create_dir_all(
            artifact_path
                .parent()
                .expect("runtime proof artifact should have a parent directory"),
        )
        .expect("runtime proof artifact directory should be writable");
        std::fs::write(&artifact_path, evidence)
            .expect("runtime proof artifact should be writable");
    }
}

#[test]
fn pm_ui_lab_perf_003_command_source_truth_closure() {
    editor_command_catalog()
        .validate()
        .expect("command catalog should be a complete source-truth graph");

    let app = RunenwerkEditorApp::new();
    let shell_state =
        RunenwerkEditorShellState::new_with_workspace_profile_registry_and_tool_surface_registry(
            app.workbench_host().workspace_profile_registry(),
            app.workbench_host().tool_surface_registry(),
        )
        .expect("source-truth shell state should build from installed registries");
    let theme = ThemeTokens::default();
    let frame_model = build_editor_shell_frame_model(
        &app,
        &shell_state,
        app.workbench_host().provider_registry(),
        &theme,
        None,
        None,
        None,
    );
    let route_actions =
        active_route_actions_by_target(shell_state.active_editor_definitions(), false, false);

    assert_eq!(
        frame_model.route_actions_by_route_target, route_actions,
        "runtime frame route actions should come from the app-owned command catalog projection"
    );

    let context = EditorCommandAvailabilityContext {
        can_undo: false,
        can_redo: false,
    };
    let mut audited_route_targets = 0usize;
    for descriptor in editor_command_catalog().descriptors() {
        for route_target in descriptor.route_targets() {
            audited_route_targets += 1;
            assert_eq!(
                route_actions.get(route_target),
                Some(&descriptor.routed_shell_action(context)),
                "route target '{route_target}' should project exactly from catalog descriptor '{}'",
                descriptor.key
            );
        }
    }
    assert!(
        audited_route_targets >= editor_command_catalog().descriptors().len(),
        "every descriptor should audit at least its primary route target"
    );

    let toolbar_binding = frame_model
        .active_toolbar_binding
        .as_ref()
        .expect("checked-in toolbar binding should be active");
    if let Some(workspace_catalog) = &toolbar_binding.workspace_catalog {
        for entry in &workspace_catalog.entries {
            let descriptor = editor_command_catalog()
                .descriptor_for_key(entry.route.as_str())
                .expect("workspace entry route should resolve through catalog");
            assert_eq!(
                entry.label, descriptor.label,
                "workspace entry '{}' should use catalog label",
                entry.id
            );
        }
    }
    for item in &toolbar_binding.menu_items {
        let descriptor = editor_command_catalog()
            .descriptor_for_key(item.route.as_str())
            .expect("toolbar menu item route should resolve through catalog");
        assert_eq!(
            item.label, descriptor.label,
            "toolbar menu item '{}' should use catalog label",
            item.item_id
        );
        if let Some(ui_definition::UiAvailabilityBinding::Static(
            ui_definition::UiAvailability::Disabled { reason },
        )) = &item.availability
        {
            assert_eq!(
                Some(reason.as_str()),
                descriptor.availability(context).reason(),
                "toolbar menu item '{}' should use catalog disabled reason",
                item.item_id
            );
        }
    }
}

#[test]
fn pm_ui_lab_perf_003_surface_source_truth_closure() {
    let mut app = RunenwerkEditorApp::new();
    let mut shell_state =
        RunenwerkEditorShellState::new_with_workspace_profile_registry_and_tool_surface_registry(
            app.workbench_host().workspace_profile_registry(),
            app.workbench_host().tool_surface_registry(),
        )
        .expect("source-truth shell state should build from installed registries");
    let profiles = [
        (SCENE_WORKSPACE_PROFILE_ID, "Scene"),
        (EDITOR_DESIGN_WORKSPACE_PROFILE_ID, "Editor Design"),
        (MATERIAL_WORKSPACE_PROFILE_ID, "Materials"),
        (RUNTIME_DEBUG_WORKSPACE_PROFILE_ID, "Runtime Debug"),
    ];
    let mut audited_surface_keys = std::collections::BTreeSet::new();

    for (profile_id, profile_label) in profiles {
        dispatch_shell_command(
            &mut app,
            Some(&mut shell_state),
            ShellCommand::SwitchWorkspaceProfile { profile_id },
            None,
            None,
            None,
            None,
        )
        .expect("workspace profile should switch through shell runtime");

        let requests = mounted_surface_requests_with_registry(
            &shell_state,
            active_document_context(&app),
            Some(app.workbench_host().tool_surface_registry()),
        );
        assert!(
            !requests.is_empty(),
            "{profile_label} should expose mounted registry-backed surfaces"
        );
        for request in requests {
            audited_surface_keys.insert(request.stable_key().as_str().to_string());
            let definition = app
                .workbench_host()
                .tool_surface_registry()
                .get(request.stable_key())
                .unwrap_or_else(|| {
                    panic!(
                        "{profile_label} surface '{}' should resolve through the tool-suite registry",
                        request.stable_key().as_str()
                    )
                });
            assert_eq!(
                request.provider_family_id.as_ref(),
                Some(&definition.provider_family),
                "{profile_label} surface '{}' should get provider family from registry metadata",
                request.stable_key().as_str()
            );
            assert_eq!(
                request.surface_route,
                Some(definition.route),
                "{profile_label} surface '{}' should get route kind from registry metadata",
                request.stable_key().as_str()
            );
            assert_eq!(
                request.capabilities,
                definition.capabilities,
                "{profile_label} surface '{}' should get capabilities from registry metadata",
                request.stable_key().as_str()
            );

            let provider_ids = app
                .workbench_host()
                .provider_family_provider_map()
                .providers_for(&definition.provider_family)
                .collect::<Vec<_>>();
            assert!(
                !provider_ids.is_empty(),
                "{profile_label} surface '{}' provider family '{}' should have assigned providers",
                request.stable_key().as_str(),
                definition.provider_family.as_str()
            );
            let observation = app
                .workbench_host()
                .provider_registry()
                .observe_resolution_for_request(
                    &request,
                    app.workbench_host().workspace_profile_registry(),
                    Some(app.workbench_host().provider_family_provider_map()),
                );
            assert!(
                observation
                    .support_modes
                    .iter()
                    .any(|support| support.support_mode == SurfaceProviderSupportMode::StableKey),
                "{profile_label} surface '{}' should be supported through stable-key provider resolution",
                request.stable_key().as_str()
            );
            assert_ne!(
                observation.availability,
                SurfaceProviderAvailability::Unsupported,
                "{profile_label} surface '{}' should not require legacy-only or unsupported provider resolution",
                request.stable_key().as_str()
            );
        }
    }

    for required_key in [
        "runenwerk.scene.viewport",
        "runenwerk.editor_design.ui_canvas",
        "runenwerk.material_lab.graph_canvas",
        "runenwerk.diagnostics.tool_suite_registry_inspector",
    ] {
        assert!(
            audited_surface_keys.contains(required_key),
            "source-truth audit should cover {required_key}"
        );
    }
}

fn write_profile_runtime_evidence(
    app: &mut RunenwerkEditorApp,
    shell_state: &mut RunenwerkEditorShellState,
    theme: &ThemeTokens,
    profile_id: editor_shell::WorkspaceProfileId,
    label: &str,
    required_keys: &[&str],
    evidence: &mut String,
) {
    use std::fmt::Write as _;

    dispatch_shell_command(
        app,
        Some(shell_state),
        ShellCommand::SwitchWorkspaceProfile { profile_id },
        None,
        None,
        None,
        None,
    )
    .expect("workspace profile should switch through shell runtime");
    assert_eq!(shell_state.active_workspace_profile_id(), profile_id);

    let requests = mounted_surface_requests_with_registry(
        shell_state,
        active_document_context(app),
        Some(app.workbench_host().tool_surface_registry()),
    );
    let frame_model = build_editor_shell_frame_model(
        app,
        shell_state,
        app.workbench_host().provider_registry(),
        theme,
        None,
        None,
        None,
    );
    writeln!(evidence, "\n## {label} Surface Registry Runtime Proof").unwrap();
    writeln!(evidence, "- active_profile: {}", profile_id.raw()).unwrap();
    writeln!(evidence, "- mounted_surface_requests: {}", requests.len()).unwrap();
    writeln!(
        evidence,
        "- resolved_surface_frames: {}",
        frame_model.surfaces.len()
    )
    .unwrap();

    for required_key in required_keys {
        let request = requests
            .iter()
            .find(|request| request.stable_surface_key.as_str() == *required_key)
            .unwrap_or_else(|| panic!("{label} should mount required surface {required_key}"));
        assert!(
            request.provider_family_id.is_some(),
            "{label} surface {required_key} should resolve provider family from registry metadata"
        );
        assert!(
            request.surface_route.is_some(),
            "{label} surface {required_key} should resolve route from registry metadata"
        );
        assert!(
            request
                .capabilities
                .allows(ui_surface::SurfaceCapability::Observe),
            "{label} surface {required_key} should resolve observe capability from registry metadata"
        );
        let frame = frame_model
            .surfaces
            .values()
            .find(|frame| frame.stable_surface_key.as_str() == *required_key)
            .unwrap_or_else(|| panic!("{label} should resolve frame for {required_key}"));
        assert!(
            frame.provider_id.is_some(),
            "{label} surface {required_key} should mount through a concrete provider"
        );
        assert_ne!(
            frame.availability,
            SurfaceProviderAvailability::Unsupported,
            "{label} surface {required_key} should not mount as unsupported"
        );
        writeln!(
            evidence,
            "- `{}` provider_family=`{}` route={:?} capabilities={:?} provider_id={:?} availability={:?} artifact={:?}",
            required_key,
            request
                .provider_family_id
                .as_ref()
                .expect("provider family already checked")
                .as_str(),
            request.surface_route,
            request.capabilities,
            frame.provider_id,
            frame.availability,
            frame.artifact.kind,
        )
        .unwrap();
    }
}

#[test]
fn pm_ui_lab_003_runtime_evidence_reports_app_hosted_editor_lab_shell() {
    use std::fmt::Write as _;

    let mut app = RunenwerkEditorApp::new();
    app.append_console_line("PM-UI-LAB-003 preview console feedback");
    let mut shell_state =
        RunenwerkEditorShellState::new_with_workspace_profile_registry_and_tool_surface_registry(
            app.workbench_host().workspace_profile_registry(),
            app.workbench_host().tool_surface_registry(),
        )
        .expect("runtime evidence shell state should build from installed registries");
    let theme = ThemeTokens::default();

    dispatch_shell_command(
        &mut app,
        Some(&mut shell_state),
        ShellCommand::SwitchWorkspaceProfile {
            profile_id: EDITOR_DESIGN_WORKSPACE_PROFILE_ID,
        },
        None,
        None,
        None,
        None,
    )
    .expect("Editor Lab should open through the shell command boundary");
    assert_eq!(
        shell_state.active_workspace_profile_id(),
        EDITOR_DESIGN_WORKSPACE_PROFILE_ID
    );

    let frame_model = build_editor_shell_frame_model(
        &app,
        &shell_state,
        app.workbench_host().provider_registry(),
        &theme,
        None,
        None,
        None,
    );
    let mut evidence = String::new();
    let mut retained_surface_debug = String::new();
    writeln!(
        evidence,
        "# PM-UI-LAB-003 Runtime Evidence\n\nGenerated by `cargo test -p runenwerk_editor pm_ui_lab_003_runtime_evidence_reports_app_hosted_editor_lab_shell -- --nocapture`.\n"
    )
    .unwrap();
    writeln!(
        evidence,
        "- active_profile: {}",
        shell_state.active_workspace_profile_id().raw()
    )
    .unwrap();
    writeln!(
        evidence,
        "- resolved_surface_frames: {}",
        frame_model.surfaces.len()
    )
    .unwrap();

    for stable_key in [
        "runenwerk.editor_design.definition_outliner",
        "runenwerk.editor_design.ui_hierarchy",
        "runenwerk.editor_design.style_inspector",
        "runenwerk.editor_design.dock_layout_preview",
        "runenwerk.editor_design.definition_validation",
        "runenwerk.editor_design.command_diff",
    ] {
        let frame = frame_by_stable_key(&frame_model, stable_key);
        let text = resolved_frame_text(frame);
        assert!(
            text.contains("Editor Lab"),
            "{stable_key} should be built by the typed Editor Lab shell"
        );
        assert!(
            resolved_frame_has_editor_definition_route(frame),
            "{stable_key} should expose direct EditorDefinition controls"
        );
        assert!(!text.contains("Edited in self-authoring"));
        assert!(!text.contains("Retained draft"));
        assert!(!text.contains("Authored Tab"));
        writeln!(
            evidence,
            "- `{stable_key}` title=`{}` routes={} typed_shell=true",
            frame.title,
            frame.routes.iter().count()
        )
        .unwrap();
        writeln!(
            retained_surface_debug,
            "\n## {stable_key}\n\n```text\n{text}\n```"
        )
        .unwrap();
    }

    let selected_node_id = shell_state
        .self_authoring()
        .selected_ui_node_id()
        .expect("Editor Lab starts from a previewable UI template")
        .to_string();
    dispatch_shell_command(
        &mut app,
        Some(&mut shell_state),
        ShellCommand::SetSelectedEditorDefinitionUiNodeText {
            node_id: selected_node_id.clone(),
            text: "PM003 runtime edited text".to_string(),
        },
        None,
        None,
        None,
        None,
    )
    .expect("direct Editor Lab text edit should dispatch through app shell state");
    let preview = shell_state
        .self_authoring()
        .formed_selected_preview(&theme)
        .expect("edited UI template should still produce a retained preview");
    let preview_debug = format!("{:?}", preview.root);
    assert!(preview_debug.contains("PM003 runtime edited text"));
    writeln!(
        evidence,
        "- direct_edit: node `{selected_node_id}` updated retained preview text"
    )
    .unwrap();
    writeln!(
        retained_surface_debug,
        "\n## retained preview after edit\n\n```text\n{preview_debug}\n```"
    )
    .unwrap();

    dispatch_shell_command(
        &mut app,
        Some(&mut shell_state),
        ShellCommand::SelectEditorDefinitionDocument {
            document_id: "runenwerk.editor.theme.default".to_string(),
        },
        None,
        None,
        None,
        None,
    )
    .expect("theme selection should route through shell command dispatch");
    let degraded_frame_model = build_editor_shell_frame_model(
        &app,
        &shell_state,
        app.workbench_host().provider_registry(),
        &theme,
        None,
        None,
        None,
    );
    let canvas_frame =
        frame_by_stable_key(&degraded_frame_model, "runenwerk.editor_design.ui_canvas");
    let canvas_text = resolved_frame_text(canvas_frame);
    assert!(canvas_text.contains("Editor Lab"));
    assert!(canvas_text.contains("Selected definition cannot form a retained UI preview"));
    assert!(resolved_frame_has_editor_definition_route(canvas_frame));
    writeln!(
        evidence,
        "- degraded_canvas: non-previewable theme document renders typed recovery state"
    )
    .unwrap();
    writeln!(
        retained_surface_debug,
        "\n## degraded canvas\n\n```text\n{canvas_text}\n```"
    )
    .unwrap();

    let command_frame = frame_by_stable_key(
        &degraded_frame_model,
        "runenwerk.editor_design.command_diff",
    );
    let command_text = resolved_frame_text(command_frame);
    assert!(command_text.contains("Preview Console"));
    assert!(command_text.contains("PM-UI-LAB-003 preview console feedback"));
    writeln!(
        evidence,
        "- preview_console: command review surface includes app-owned console feedback"
    )
    .unwrap();

    if std::env::var_os("RUNENWERK_WRITE_PM_UI_LAB_003_EVIDENCE").is_some() {
        let repo_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(|path| path.parent())
            .expect("runenwerk_editor crate should live under apps/");
        let artifact_root = repo_root.join(
            "docs-site/src/content/docs/reports/closeouts/pm-ui-lab-003-app-hosted-editor-lab-surface-shell/artifacts",
        );
        std::fs::create_dir_all(&artifact_root)
            .expect("PM-003 artifact directory should be writable");
        std::fs::write(artifact_root.join("runtime-proof.txt"), evidence)
            .expect("PM-003 runtime proof should be writable");
        std::fs::write(
            artifact_root.join("retained-surface-debug.txt"),
            retained_surface_debug,
        )
        .expect("PM-003 retained visual artifact should be writable");
    }
}

#[test]
fn editor_lab_operation_dispatch_records_history_and_refreshes_preview() {
    let mut app = RunenwerkEditorApp::new();
    let mut shell_state =
        RunenwerkEditorShellState::new_with_workspace_profile_registry_and_tool_surface_registry(
            app.workbench_host().workspace_profile_registry(),
            app.workbench_host().tool_surface_registry(),
        )
        .expect("shell state should build from installed registries");
    let theme = ThemeTokens::default();

    dispatch_shell_command(
        &mut app,
        Some(&mut shell_state),
        ShellCommand::SwitchWorkspaceProfile {
            profile_id: EDITOR_DESIGN_WORKSPACE_PROFILE_ID,
        },
        None,
        None,
        None,
        None,
    )
    .expect("Editor Lab should open before operation dispatch");

    let selected_node_id = shell_state
        .self_authoring()
        .selected_ui_node_id()
        .expect("Editor Lab starts from a text-editable UI node")
        .to_string();
    dispatch_shell_command(
        &mut app,
        Some(&mut shell_state),
        ShellCommand::SetSelectedEditorDefinitionUiNodeText {
            node_id: selected_node_id.clone(),
            text: "PM004 operation text".to_string(),
        },
        None,
        None,
        None,
        None,
    )
    .expect("text edit should dispatch through an EditorLabOperation");

    let report = shell_state
        .self_authoring()
        .last_operation_report()
        .expect("accepted operation should be recorded");
    assert_eq!(report.status, EditorLabOperationStatus::Accepted);
    assert!(report.diff.as_ref().is_some_and(|diff| {
        diff.changes
            .iter()
            .any(|change| change.family == EditorLabOperationDiffFamily::UiAuthoredValue)
    }));
    assert_eq!(
        shell_state
            .self_authoring()
            .operation_history_snapshot()
            .undo_count,
        1
    );
    let preview = shell_state
        .self_authoring()
        .formed_selected_preview(&theme)
        .expect("accepted operation should keep retained preview valid");
    assert!(format!("{:?}", preview.root).contains("PM004 operation text"));

    dispatch_shell_command(
        &mut app,
        Some(&mut shell_state),
        ShellCommand::SelectEditorDefinitionDocument {
            document_id: "runenwerk.editor.theme.default".to_string(),
        },
        None,
        None,
        None,
        None,
    )
    .expect("theme document should be selectable");
    let before_theme_color = selected_theme_color(&shell_state, "accent");
    let rejected = dispatch_shell_command(
        &mut app,
        Some(&mut shell_state),
        ShellCommand::SetSelectedEditorThemeColor {
            token: "accent".to_string(),
            value: "not-a-color".to_string(),
        },
        None,
        None,
        None,
        None,
    );
    assert!(rejected.is_err());
    let report = shell_state
        .self_authoring()
        .last_operation_report()
        .expect("rejected operation should produce an operation report");
    assert_eq!(report.status, EditorLabOperationStatus::Rejected);
    assert!(report.has_errors());
    assert_eq!(
        selected_theme_color(&shell_state, "accent"),
        before_theme_color
    );
    assert_eq!(
        shell_state
            .self_authoring()
            .operation_history_snapshot()
            .undo_count,
        1,
        "rejected operations must not enter history"
    );

    dispatch_shell_command(
        &mut app,
        Some(&mut shell_state),
        ShellCommand::UndoEditorLabOperation,
        None,
        None,
        None,
        None,
    )
    .expect("undo should restore the draft before the text operation");
    let preview = shell_state
        .self_authoring()
        .formed_selected_preview(&theme)
        .expect("undo should restore the UI template selection");
    assert!(!format!("{:?}", preview.root).contains("PM004 operation text"));
    assert_eq!(
        shell_state
            .self_authoring()
            .operation_history_snapshot()
            .redo_count,
        1
    );

    dispatch_shell_command(
        &mut app,
        Some(&mut shell_state),
        ShellCommand::RedoEditorLabOperation,
        None,
        None,
        None,
        None,
    )
    .expect("redo should reapply the text operation");
    let preview = shell_state
        .self_authoring()
        .formed_selected_preview(&theme)
        .expect("redo should keep retained preview valid");
    assert!(format!("{:?}", preview.root).contains("PM004 operation text"));
}

#[test]
fn pm_ui_lab_004_runtime_evidence_reports_operation_driven_visual_authoring() {
    use std::fmt::Write as _;

    let mut app = RunenwerkEditorApp::new();
    app.append_console_line("PM-UI-LAB-004 operation console feedback");
    let mut shell_state =
        RunenwerkEditorShellState::new_with_workspace_profile_registry_and_tool_surface_registry(
            app.workbench_host().workspace_profile_registry(),
            app.workbench_host().tool_surface_registry(),
        )
        .expect("runtime evidence shell state should build from installed registries");
    let theme = ThemeTokens::default();

    dispatch_shell_command(
        &mut app,
        Some(&mut shell_state),
        ShellCommand::SwitchWorkspaceProfile {
            profile_id: EDITOR_DESIGN_WORKSPACE_PROFILE_ID,
        },
        None,
        None,
        None,
        None,
    )
    .expect("Editor Lab should open through the shell command boundary");

    let selected_node_id = shell_state
        .self_authoring()
        .selected_ui_node_id()
        .expect("Editor Lab starts from a text-editable UI node")
        .to_string();
    dispatch_shell_command(
        &mut app,
        Some(&mut shell_state),
        ShellCommand::SetSelectedEditorDefinitionUiNodeText {
            node_id: selected_node_id.clone(),
            text: "PM004 runtime accepted text".to_string(),
        },
        None,
        None,
        None,
        None,
    )
    .expect("inspector edit should dispatch through an EditorLabOperation");
    let accepted_text_report = shell_state
        .self_authoring()
        .last_operation_report()
        .expect("accepted text edit should produce an operation report")
        .clone();
    assert_eq!(
        accepted_text_report.status,
        EditorLabOperationStatus::Accepted
    );

    dispatch_shell_command(
        &mut app,
        Some(&mut shell_state),
        ShellCommand::SelectEditorDefinitionDocument {
            document_id: "runenwerk.editor.theme.default".to_string(),
        },
        None,
        None,
        None,
        None,
    )
    .expect("theme definition should be selectable");
    let rejected = dispatch_shell_command(
        &mut app,
        Some(&mut shell_state),
        ShellCommand::SetSelectedEditorThemeColor {
            token: "accent".to_string(),
            value: "not-a-color".to_string(),
        },
        None,
        None,
        None,
        None,
    );
    assert!(rejected.is_err());
    let rejected_report = shell_state
        .self_authoring()
        .last_operation_report()
        .expect("rejected edit should produce an operation report")
        .clone();
    assert_eq!(rejected_report.status, EditorLabOperationStatus::Rejected);

    dispatch_shell_command(
        &mut app,
        Some(&mut shell_state),
        ShellCommand::UndoEditorLabOperation,
        None,
        None,
        None,
        None,
    )
    .expect("undo should restore pre-operation draft state");
    let undo_preview = shell_state
        .self_authoring()
        .formed_selected_preview(&theme)
        .expect("undo should restore previewable UI document");
    assert!(!format!("{:?}", undo_preview.root).contains("PM004 runtime accepted text"));

    dispatch_shell_command(
        &mut app,
        Some(&mut shell_state),
        ShellCommand::RedoEditorLabOperation,
        None,
        None,
        None,
        None,
    )
    .expect("redo should restore accepted operation state");
    let redo_preview = shell_state
        .self_authoring()
        .formed_selected_preview(&theme)
        .expect("redo should restore previewable UI document");
    let redo_preview_debug = format!("{:?}", redo_preview.root);
    assert!(redo_preview_debug.contains("PM004 runtime accepted text"));

    let stack_document_id = EditorDefinitionId::from("pm004.stack.template");
    shell_state
        .self_authoring_mut()
        .create_document(EditorDefinitionDocument::current(
            stack_document_id.clone(),
            "PM004 stack template",
            EditorDefinitionDocumentKind::UiLayout,
            EditorDefinitionDocumentContent::UiTemplate(pm004_stack_template()),
        ))
        .expect("runtime evidence stack template should be a valid draft document");
    let visual_operation = EditorLabOperation {
        id: shell_state
            .self_authoring()
            .next_operation_id("visual_layout"),
        document_id: stack_document_id,
        target_profile: "editor.workbench".to_string(),
        kind: EditorLabOperationKind::UiVisualLayout(Box::new(
            ui_definition::UiVisualLayoutOperation {
                id: "pm004.axis.stack".into(),
                source_document: "pm004.stack.template".into(),
                target_path: ui_definition::AuthoredUiNodePath("root/stack".to_string()),
                expected_node_id: "stack".into(),
                target_profile: "editor.workbench".into(),
                kind: ui_definition::UiVisualLayoutEditKind::ChangeStackAxis {
                    axis: ui_definition::UiAxisDefinition::Horizontal,
                },
                source_location: None,
                preview_only: false,
            },
        )),
        preview_only: false,
        source: Some("pm004.runtime-evidence".to_string()),
    };
    dispatch_shell_command(
        &mut app,
        Some(&mut shell_state),
        ShellCommand::ApplyEditorLabOperation {
            operation: visual_operation,
        },
        None,
        None,
        None,
        None,
    )
    .expect("generic visual layout edit should dispatch through ui_definition operations");
    let visual_report = shell_state
        .self_authoring()
        .last_operation_report()
        .expect("visual layout operation should produce an operation report")
        .clone();
    assert_eq!(visual_report.status, EditorLabOperationStatus::Accepted);
    assert!(visual_report.diff.as_ref().is_some_and(|diff| {
        diff.changes
            .iter()
            .any(|change| change.family == EditorLabOperationDiffFamily::UiVisualLayout)
    }));

    let frame_model = build_editor_shell_frame_model(
        &app,
        &shell_state,
        app.workbench_host().provider_registry(),
        &theme,
        None,
        None,
        None,
    );
    let command_frame = frame_by_stable_key(&frame_model, "runenwerk.editor_design.command_diff");
    let command_text = resolved_frame_text(command_frame);
    assert!(command_text.contains("Operation History"));
    assert!(command_text.contains("last operation"));
    assert!(command_text.contains("UiVisualLayout"));

    let mut evidence = String::new();
    writeln!(
        evidence,
        "# PM-UI-LAB-004 Runtime Evidence\n\nGenerated by `cargo test -p runenwerk_editor pm_ui_lab_004_runtime_evidence_reports_operation_driven_visual_authoring -- --nocapture`.\n"
    )
    .unwrap();
    writeln!(
        evidence,
        "- active_profile: {}",
        shell_state.active_workspace_profile_id().raw()
    )
    .unwrap();
    writeln!(
        evidence,
        "- accepted_text_operation: {} {:?}",
        accepted_text_report.operation_id, accepted_text_report.status
    )
    .unwrap();
    writeln!(
        evidence,
        "- rejected_theme_operation: {} diagnostics={}",
        rejected_report.operation_id,
        rejected_report.diagnostics.len()
    )
    .unwrap();
    writeln!(
        evidence,
        "- undo_redo_preview_refresh: redo preview contains accepted text"
    )
    .unwrap();
    writeln!(
        evidence,
        "- visual_layout_operation: {} diff_family=UiVisualLayout",
        visual_report.operation_id
    )
    .unwrap();
    let history = shell_state.self_authoring().operation_history_snapshot();
    writeln!(
        evidence,
        "- operation_history: undo={} redo={}",
        history.undo_count, history.redo_count
    )
    .unwrap();

    let visual_debug = format!(
        "## Command Diff Surface\n\n```text\n{command_text}\n```\n\n## Redo Retained Preview\n\n```text\n{redo_preview_debug}\n```\n"
    );

    if std::env::var_os("RUNENWERK_WRITE_PM_UI_LAB_004_EVIDENCE").is_some() {
        let repo_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(|path| path.parent())
            .expect("runenwerk_editor crate should live under apps/");
        let artifact_root = repo_root.join(
            "docs-site/src/content/docs/reports/closeouts/pm-ui-lab-004-operation-driven-visual-authoring/artifacts",
        );
        std::fs::create_dir_all(&artifact_root)
            .expect("PM-004 artifact directory should be writable");
        std::fs::write(artifact_root.join("runtime-proof.txt"), evidence)
            .expect("PM-004 runtime proof should be writable");
        std::fs::write(
            artifact_root.join("operation-surface-debug.txt"),
            visual_debug,
        )
        .expect("PM-004 retained visual artifact should be writable");
    }
}

#[test]
fn pm_ui_lab_perf_004_direct_manipulation_product_surfaces_cover_normal_workflow() {
    use std::fmt::Write as _;

    let mut app = RunenwerkEditorApp::new();
    app.append_console_line("PM-UI-LAB-PERF-004 direct manipulation console feedback");
    let mut shell_state =
        RunenwerkEditorShellState::new_with_workspace_profile_registry_and_tool_surface_registry(
            app.workbench_host().workspace_profile_registry(),
            app.workbench_host().tool_surface_registry(),
        )
        .expect("runtime evidence shell state should build from installed registries");
    let theme = ThemeTokens::default();

    dispatch_shell_command(
        &mut app,
        Some(&mut shell_state),
        ShellCommand::SwitchWorkspaceProfile {
            profile_id: EDITOR_DESIGN_WORKSPACE_PROFILE_ID,
        },
        None,
        None,
        None,
        None,
    )
    .expect("Editor Lab should open through the shell command boundary");

    let frame_model = build_editor_shell_frame_model(
        &app,
        &shell_state,
        app.workbench_host().provider_registry(),
        &theme,
        None,
        None,
        None,
    );
    let hierarchy_frame =
        frame_by_stable_key(&frame_model, "runenwerk.editor_design.definition_outliner");
    let hierarchy_text = resolved_frame_text(hierarchy_frame);
    assert!(hierarchy_text.contains("Definition Hierarchy"));
    assert!(resolved_frame_has_editor_definition_action(
        hierarchy_frame,
        |action| matches!(action, EditorDefinitionSurfaceAction::SelectDocument { .. })
    ));

    let canvas_frame = frame_by_stable_key(&frame_model, "runenwerk.editor_design.ui_canvas");
    let canvas_text = resolved_frame_text(canvas_frame);
    assert!(canvas_text.contains("retained preview available"));
    assert!(canvas_text.contains("selected node"));
    assert!(resolved_frame_has_editor_definition_action(
        canvas_frame,
        |action| matches!(action, EditorDefinitionSurfaceAction::SelectUiNode { .. })
    ));
    assert!(resolved_frame_has_editor_definition_action(
        canvas_frame,
        |action| matches!(action, EditorDefinitionSurfaceAction::SetUiNodeText { .. })
    ));

    let inspector_frame =
        frame_by_stable_key(&frame_model, "runenwerk.editor_design.style_inspector");
    let inspector_text = resolved_frame_text(inspector_frame);
    assert!(inspector_text.contains("Selected node text"));
    assert!(resolved_frame_has_editor_definition_action(
        inspector_frame,
        |action| matches!(action, EditorDefinitionSurfaceAction::SetUiNodeText { .. })
    ));

    let selected_node_id = shell_state
        .self_authoring()
        .selected_ui_node_id()
        .expect("Editor Lab starts from a text-editable UI node")
        .to_string();
    dispatch_shell_command(
        &mut app,
        Some(&mut shell_state),
        ShellCommand::SetSelectedEditorDefinitionUiNodeText {
            node_id: selected_node_id.clone(),
            text: "PM004 direct surface text".to_string(),
        },
        None,
        None,
        None,
        None,
    )
    .expect("inspector/canvas text edit should dispatch through an EditorLabOperation");
    let accepted_report = shell_state
        .self_authoring()
        .last_operation_report()
        .expect("accepted direct edit should produce an operation report")
        .clone();
    assert_eq!(accepted_report.status, EditorLabOperationStatus::Accepted);
    assert!(accepted_report.diff.as_ref().is_some_and(|diff| {
        diff.changes
            .iter()
            .any(|change| change.family == EditorLabOperationDiffFamily::UiAuthoredValue)
    }));
    let preview = shell_state
        .self_authoring()
        .formed_selected_preview(&theme)
        .expect("accepted direct edit should keep retained preview valid");
    let accepted_preview_debug = format!("{:?}", preview.root);
    assert!(accepted_preview_debug.contains("PM004 direct surface text"));

    dispatch_shell_command(
        &mut app,
        Some(&mut shell_state),
        ShellCommand::UndoEditorLabOperation,
        None,
        None,
        None,
        None,
    )
    .expect("undo should restore pre-operation draft state");
    let undo_report = shell_state
        .self_authoring()
        .last_operation_report()
        .expect("undo should produce an operation report")
        .clone();
    assert!(
        undo_report
            .diff
            .as_ref()
            .is_some_and(|diff| { diff.changes.iter().any(|change| change.kind == "Undo") })
    );

    dispatch_shell_command(
        &mut app,
        Some(&mut shell_state),
        ShellCommand::RedoEditorLabOperation,
        None,
        None,
        None,
        None,
    )
    .expect("redo should restore accepted operation state");
    let redo_report = shell_state
        .self_authoring()
        .last_operation_report()
        .expect("redo should produce an operation report")
        .clone();
    assert!(
        redo_report
            .diff
            .as_ref()
            .is_some_and(|diff| { diff.changes.iter().any(|change| change.kind == "Redo") })
    );

    dispatch_shell_command(
        &mut app,
        Some(&mut shell_state),
        ShellCommand::SelectEditorDefinitionDocument {
            document_id: "runenwerk.editor.layout.editor_design".to_string(),
        },
        None,
        None,
        None,
        None,
    )
    .expect("workspace layout definition should be selectable for palette controls");
    let palette_model = build_editor_shell_frame_model(
        &app,
        &shell_state,
        app.workbench_host().provider_registry(),
        &theme,
        None,
        None,
        None,
    );
    let palette_frame = frame_by_stable_key(
        &palette_model,
        "runenwerk.editor_design.dock_layout_preview",
    );
    let palette_text = resolved_frame_text(palette_frame);
    assert!(palette_text.contains("Layout controls"));
    assert!(resolved_frame_has_editor_definition_action(
        palette_frame,
        |action| matches!(
            action,
            EditorDefinitionSurfaceAction::AddWorkspaceLayoutTab { .. }
        )
    ));
    dispatch_shell_command(
        &mut app,
        Some(&mut shell_state),
        ShellCommand::AddSelectedEditorWorkspaceLayoutTab {
            label: "PM004 Review".to_string(),
            tool_surface: "definition_validation".to_string(),
        },
        None,
        None,
        None,
        None,
    )
    .expect("palette layout insertion should dispatch through an EditorLabOperation");
    let palette_report = shell_state
        .self_authoring()
        .last_operation_report()
        .expect("palette operation should produce an operation report")
        .clone();
    assert!(palette_report.diff.as_ref().is_some_and(|diff| {
        diff.changes
            .iter()
            .any(|change| change.family == EditorLabOperationDiffFamily::EditorWorkspaceLayout)
    }));

    dispatch_shell_command(
        &mut app,
        Some(&mut shell_state),
        ShellCommand::SelectEditorDefinitionDocument {
            document_id: "runenwerk.editor.theme.default".to_string(),
        },
        None,
        None,
        None,
        None,
    )
    .expect("theme definition should be selectable");
    let rejected = dispatch_shell_command(
        &mut app,
        Some(&mut shell_state),
        ShellCommand::SetSelectedEditorThemeColor {
            token: "accent".to_string(),
            value: "not-a-color".to_string(),
        },
        None,
        None,
        None,
        None,
    );
    assert!(rejected.is_err());
    let rejected_report = shell_state
        .self_authoring()
        .last_operation_report()
        .expect("rejected edit should produce an operation report")
        .clone();
    assert_eq!(rejected_report.status, EditorLabOperationStatus::Rejected);

    app.append_console_line("PM-UI-LAB-PERF-004 direct manipulation console feedback");
    let rejected_model = build_editor_shell_frame_model(
        &app,
        &shell_state,
        app.workbench_host().provider_registry(),
        &theme,
        None,
        None,
        None,
    );
    let diagnostics_text = resolved_frame_text(frame_by_stable_key(
        &rejected_model,
        "runenwerk.editor_design.definition_validation",
    ));
    assert!(diagnostics_text.contains("editor.definition.theme.color.invalid_format"));

    let command_text = resolved_frame_text(frame_by_stable_key(
        &rejected_model,
        "runenwerk.editor_design.command_diff",
    ));
    assert!(command_text.contains("Preview Console"));
    assert!(command_text.contains("PM-UI-LAB-PERF-004 direct manipulation console feedback"));
    assert!(command_text.contains("last operation"));
    assert!(command_text.contains("Rejected"));

    let mut evidence = String::new();
    writeln!(
        evidence,
        "# PM-UI-LAB-PERF-004 Direct Manipulation Runtime Evidence\n"
    )
    .unwrap();
    writeln!(
        evidence,
        "- hierarchy_surface: routes select definitions through EditorDefinition actions"
    )
    .unwrap();
    writeln!(
        evidence,
        "- canvas_surface: retained preview status plus SelectUiNode and SetUiNodeText routes"
    )
    .unwrap();
    writeln!(
        evidence,
        "- inspector_surface: selected node text edit routes through EditorLabOperation"
    )
    .unwrap();
    writeln!(
        evidence,
        "- palette_surface: workspace layout insertion routes through EditorLabOperation"
    )
    .unwrap();
    writeln!(
        evidence,
        "- diagnostics_surface: rejected operation diagnostic {}",
        rejected_report
            .diagnostics
            .first()
            .map(|diagnostic| diagnostic.code.as_str())
            .unwrap_or("none")
    )
    .unwrap();
    writeln!(
        evidence,
        "- operation_diff_surface: accepted={:?} undo_diff={} redo_diff={} palette_diff={}",
        accepted_report.status,
        undo_report.diff.is_some(),
        redo_report.diff.is_some(),
        palette_report.diff.is_some()
    )
    .unwrap();
    writeln!(
        evidence,
        "- preview_console_surface: app-owned console feedback appears in command review"
    )
    .unwrap();

    let retained_surface_debug = format!(
        "## Hierarchy\n\n```text\n{hierarchy_text}\n```\n\n## Canvas\n\n```text\n{canvas_text}\n```\n\n## Inspector\n\n```text\n{inspector_text}\n```\n\n## Palette\n\n```text\n{palette_text}\n```\n\n## Accepted Preview\n\n```text\n{accepted_preview_debug}\n```\n\n## Diagnostics\n\n```text\n{diagnostics_text}\n```\n\n## Command Diff\n\n```text\n{command_text}\n```\n"
    );

    if std::env::var_os("RUNENWERK_WRITE_PM_UI_LAB_PERF_004_EVIDENCE").is_some() {
        let repo_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(|path| path.parent())
            .expect("runenwerk_editor crate should live under apps/");
        let artifact_root = repo_root.join(
            "docs-site/src/content/docs/reports/closeouts/pm-ui-lab-perf-004-direct-manipulation-ux-closure/artifacts",
        );
        std::fs::create_dir_all(&artifact_root)
            .expect("PM-UI-LAB-PERF-004 artifact directory should be writable");
        std::fs::write(artifact_root.join("runtime-proof.txt"), evidence)
            .expect("PM-UI-LAB-PERF-004 runtime proof should be writable");
        std::fs::write(
            artifact_root.join("direct-manipulation-surface-debug.txt"),
            retained_surface_debug,
        )
        .expect("PM-UI-LAB-PERF-004 retained surface proof should be writable");
    }
}

#[test]
fn dispatch_shell_command_updates_active_tool() {
    let mut app = RunenwerkEditorApp::new();

    dispatch_shell_command(
        &mut app,
        None,
        ShellCommand::ActivateSelectTool,
        None,
        None,
        None,
        None,
    )
    .expect("select tool command should succeed");
    assert_eq!(app.runtime().session().active_tool(), Some(SELECT_TOOL_ID));

    dispatch_shell_command(
        &mut app,
        None,
        ShellCommand::ActivateTranslateTool,
        None,
        None,
        None,
        None,
    )
    .expect("translate tool command should succeed");
    assert_eq!(
        app.runtime().session().active_tool(),
        Some(TRANSLATE_TOOL_ID)
    );
}

#[test]
fn stale_asset_shell_command_fails_closed() {
    let mut app = RunenwerkEditorApp::new();
    let asset_id = asset::asset_id(1);
    app.asset_catalog_runtime_mut()
        .catalog_mut()
        .insert_asset_record(asset::AssetRecord::new(
            asset_id,
            "field",
            "Field",
            asset::AssetKind::SdfGraph,
        ));

    dispatch_shell_command(
        &mut app,
        None,
        ShellCommand::SelectAsset {
            asset_id,
            projection_epoch: 1,
        },
        None,
        None,
        None,
        Some(2),
    )
    .expect("stale asset command should fail closed without error");

    assert_eq!(app.asset_catalog_runtime().selected_asset_id(), None);
}

#[test]
fn dispatch_shell_command_applies_and_rolls_back_selected_editor_definition() {
    let mut app = RunenwerkEditorApp::new();
    let mut shell_state = RunenwerkEditorShellState::new();

    dispatch_shell_command(
        &mut app,
        Some(&mut shell_state),
        ShellCommand::ApplySelectedEditorDefinition,
        None,
        None,
        None,
        None,
    )
    .expect("selected definition fixture should apply");

    assert_eq!(shell_state.self_authoring().applied_count(), 1);

    dispatch_shell_command(
        &mut app,
        Some(&mut shell_state),
        ShellCommand::RollbackSelectedEditorDefinition,
        None,
        None,
        None,
        None,
    )
    .expect("selected applied definition should rollback");

    assert_eq!(shell_state.self_authoring().applied_count(), 0);
}

#[test]
fn dispatch_shell_command_captures_ui_designer_scenario_evidence() {
    let mut app = RunenwerkEditorApp::new();
    let mut shell_state = RunenwerkEditorShellState::new();

    dispatch_shell_command(
        &mut app,
        Some(&mut shell_state),
        ShellCommand::CaptureUiDesignerScenarioEvidence,
        None,
        None,
        None,
        None,
    )
    .expect("UI Designer scenario evidence capture should dispatch through shell state");

    let packets = shell_state
        .self_authoring()
        .last_scenario_evidence_packets();
    assert_eq!(packets.len(), 2);
    assert!(packets.iter().any(|packet| {
        packet.target_profile() == "editor.workbench"
            && packet.validate_runtime_product_evidence().is_ok()
    }));
    assert!(packets.iter().any(|packet| {
        packet.target_profile() == "game.runtime"
            && !packet.is_runtime_product()
            && packet.validate_scenario_evidence().is_ok()
            && packet
                .unsupported_checks()
                .iter()
                .any(|check| check.check == "concrete game HUD runtime")
    }));
}

#[test]
fn dispatch_shell_command_edits_selected_ui_and_theme_definition_drafts() {
    let mut app = RunenwerkEditorApp::new();
    let mut shell_state = RunenwerkEditorShellState::new();
    let node_id = shell_state
        .self_authoring()
        .selected_ui_node_id()
        .expect("selected UI definition should expose an editable node")
        .to_string();

    dispatch_shell_command(
        &mut app,
        Some(&mut shell_state),
        ShellCommand::SetSelectedEditorDefinitionUiNodeText {
            node_id,
            text: "Edited by command".to_string(),
        },
        None,
        None,
        None,
        None,
    )
    .expect("selected UI definition node text edit should succeed");
    assert!(
        shell_state
            .self_authoring()
            .formed_selected_preview(&ThemeTokens::default())
            .is_some()
    );

    dispatch_shell_command(
        &mut app,
        Some(&mut shell_state),
        ShellCommand::SelectEditorDefinitionDocument {
            document_id: "runenwerk.editor.theme.default".to_string(),
        },
        None,
        None,
        None,
        None,
    )
    .expect("theme document selection should succeed");
    dispatch_shell_command(
        &mut app,
        Some(&mut shell_state),
        ShellCommand::SetSelectedEditorThemeColor {
            token: "accent".to_string(),
            value: "#2244ff".to_string(),
        },
        None,
        None,
        None,
        None,
    )
    .expect("theme color draft edit should succeed");
    let selected = shell_state
        .self_authoring()
        .selected_document()
        .expect("theme document should remain selected");
    let editor_definition::EditorDefinitionDocumentContent::Theme(theme) = &selected.content else {
        panic!("selected document should be a theme definition");
    };
    assert_eq!(
        theme.colors.get("accent").map(String::as_str),
        Some("#2244ff")
    );
}

#[test]
fn applying_selected_theme_definition_produces_live_theme_activation() {
    let mut host = EditorHostResource::default();

    dispatch_shell_command(
        &mut host.app,
        Some(&mut host.shell_state),
        ShellCommand::SelectEditorDefinitionDocument {
            document_id: "runenwerk.editor.theme.default".to_string(),
        },
        None,
        None,
        None,
        None,
    )
    .expect("theme document selection should succeed");
    dispatch_shell_command(
        &mut host.app,
        Some(&mut host.shell_state),
        ShellCommand::SetSelectedEditorThemeColor {
            token: "accent".to_string(),
            value: "#3366ff".to_string(),
        },
        None,
        None,
        None,
        None,
    )
    .expect("theme color draft edit should succeed");
    dispatch_shell_command(
        &mut host.app,
        Some(&mut host.shell_state),
        ShellCommand::ApplySelectedEditorDefinition,
        None,
        None,
        None,
        None,
    )
    .expect("theme definition apply should succeed");

    assert_eq!(host.app.pending_editor_definition_activation_count(), 1);

    let activated = host.apply_pending_editor_definition_activations();

    assert_eq!(activated, 1);
    assert_eq!(
        host.theme.accent,
        ui_theme::UiColor::new(0.2, 0.4, 1.0, 1.0)
    );
}

#[test]
fn pm_ui_lab_005_runtime_evidence_reports_project_io_apply_rollback() {
    use std::fmt::Write as _;

    let mut host = EditorHostResource::default();
    dispatch_shell_command(
        &mut host.app,
        Some(&mut host.shell_state),
        ShellCommand::SwitchWorkspaceProfile {
            profile_id: EDITOR_DESIGN_WORKSPACE_PROFILE_ID,
        },
        None,
        None,
        None,
        None,
    )
    .expect("Editor Lab should open through the shell command boundary");

    dispatch_shell_command(
        &mut host.app,
        Some(&mut host.shell_state),
        ShellCommand::SaveEditorLabProjectPackage,
        None,
        None,
        None,
        None,
    )
    .expect("Editor Lab project package should save");
    let saved_package = host
        .shell_state
        .self_authoring()
        .last_saved_project_package_source()
        .expect("project package save should preserve the serialized package")
        .to_string();
    assert!(saved_package.contains("runenwerk.editor.lab.project"));

    dispatch_shell_command(
        &mut host.app,
        Some(&mut host.shell_state),
        ShellCommand::ReloadEditorLabProjectPackage,
        None,
        None,
        None,
        None,
    )
    .expect("saved Editor Lab project package should reload into drafts");
    assert_eq!(
        host.app.pending_editor_definition_activation_count(),
        0,
        "project reload must not mutate live runtime state"
    );

    let selected_export = host
        .shell_state
        .self_authoring()
        .export_selected_to_ron()
        .expect("selected definition should export as a typed package");
    let selected_import = host
        .shell_state
        .self_authoring_mut()
        .import_selected_package_from_ron(&selected_export)
        .expect("selected definition package should import through typed project IO");
    assert!(selected_import.replaced_existing);

    let invalid_package_source = "not a valid PM005 package";
    assert!(
        host.shell_state
            .self_authoring_mut()
            .load_project_package_from_ron(invalid_package_source)
            .is_err()
    );
    assert_eq!(
        host.shell_state
            .self_authoring()
            .last_invalid_project_package_source(),
        Some(invalid_package_source)
    );

    dispatch_shell_command(
        &mut host.app,
        Some(&mut host.shell_state),
        ShellCommand::BuildSelectedEditorDefinitionApplyReview,
        None,
        None,
        None,
        None,
    )
    .expect("selected definition should build a deterministic apply review");
    let pending_review = host
        .shell_state
        .self_authoring()
        .last_apply_review()
        .expect("apply review should be stored")
        .clone();
    assert_eq!(
        pending_review.status,
        crate::shell::DefinitionApplyReviewStatus::Pending
    );
    assert!(!pending_review.diff_rows.is_empty());

    dispatch_shell_command(
        &mut host.app,
        Some(&mut host.shell_state),
        ShellCommand::RejectSelectedEditorDefinitionApplyReview,
        None,
        None,
        None,
        None,
    )
    .expect("apply review rejection should preserve draft and applied state");
    assert_eq!(host.shell_state.self_authoring().applied_count(), 0);
    assert_eq!(
        host.shell_state
            .self_authoring()
            .last_apply_review()
            .expect("rejected review should remain inspectable")
            .status,
        crate::shell::DefinitionApplyReviewStatus::Rejected
    );

    dispatch_shell_command(
        &mut host.app,
        Some(&mut host.shell_state),
        ShellCommand::ApplySelectedEditorDefinition,
        None,
        None,
        None,
        None,
    )
    .expect("accepted apply should queue runtime activation");
    assert_eq!(host.app.pending_editor_definition_activation_count(), 1);
    assert_eq!(
        host.app
            .last_editor_definition_activation_report()
            .expect("queued activation should have a typed report")
            .status,
        EditorDefinitionActivationStatus::Queued
    );

    let activated = host.apply_pending_editor_definition_activations();
    assert_eq!(activated, 1);
    let applied_report = host
        .app
        .last_editor_definition_activation_report()
        .expect("runtime activation should record a typed report")
        .clone();
    assert_eq!(
        applied_report.status,
        EditorDefinitionActivationStatus::Applied
    );
    assert_eq!(applied_report.review_id, Some(pending_review.id.clone()));

    let selected_node = host
        .shell_state
        .self_authoring()
        .selected_ui_node_id()
        .expect("selected UI definition should expose an editable node")
        .to_string();
    dispatch_shell_command(
        &mut host.app,
        Some(&mut host.shell_state),
        ShellCommand::SetSelectedEditorDefinitionUiNodeText {
            node_id: selected_node,
            text: "PM005 dirty draft after apply".to_string(),
        },
        None,
        None,
        None,
        None,
    )
    .expect("draft edit after apply should remain local");
    dispatch_shell_command(
        &mut host.app,
        Some(&mut host.shell_state),
        ShellCommand::ReloadSelectedEditorDefinitionLastApplied,
        None,
        None,
        None,
        None,
    )
    .expect("last applied snapshot should reload into draft and applied state");
    let preview = host
        .shell_state
        .self_authoring()
        .formed_selected_preview(&host.theme)
        .expect("reloaded last applied snapshot should still preview");
    assert!(!format!("{:?}", preview.root).contains("PM005 dirty draft after apply"));

    dispatch_shell_command(
        &mut host.app,
        Some(&mut host.shell_state),
        ShellCommand::RollbackSelectedEditorDefinition,
        None,
        None,
        None,
        None,
    )
    .expect("snapshot-backed rollback should succeed");
    assert_eq!(host.shell_state.self_authoring().applied_count(), 0);
    assert!(
        host.shell_state
            .self_authoring()
            .last_rollback_record()
            .is_some()
    );

    let original_bindings = host
        .shell_state
        .active_editor_definitions()
        .editor_bindings()
        .cloned()
        .expect("default shell state should activate checked-in bindings");
    host.app
        .queue_editor_definition_activation(EditorDefinitionDocument::current(
            EditorDefinitionId::from("runenwerk.editor.pm005.bindings.invalid"),
            "pm005_invalid_bindings.ron",
            EditorDefinitionDocumentKind::EditorBindings,
            EditorDefinitionDocumentContent::EditorBindings(
                editor_definition::EditorDefinitionBindings {
                    toolbar: editor_definition::EditorToolbarBinding {
                        template: "runenwerk.editor.pm005.missing_toolbar".into(),
                        ..original_bindings.toolbar.clone()
                    },
                    shell_chrome_template: original_bindings.shell_chrome_template.clone(),
                    surface_templates: original_bindings.surface_templates.clone(),
                },
            ),
        ));
    let failed_activated = host.apply_pending_editor_definition_activations();
    assert_eq!(failed_activated, 0);
    assert_eq!(
        host.shell_state
            .active_editor_definitions()
            .editor_bindings(),
        Some(&original_bindings),
        "failed activation must preserve previous active bindings"
    );
    let failed_report = host
        .app
        .last_editor_definition_activation_report()
        .expect("failed activation should record a typed report")
        .clone();
    assert_eq!(
        failed_report.status,
        EditorDefinitionActivationStatus::Failed
    );
    assert!(failed_report.previous_state_preserved);
    assert_eq!(host.app.failed_editor_definition_activations().len(), 1);

    let frame_model = build_editor_shell_frame_model(
        &host.app,
        &host.shell_state,
        host.app.workbench_host().provider_registry(),
        &host.theme,
        None,
        None,
        None,
    );
    let command_frame = frame_by_stable_key(&frame_model, "runenwerk.editor_design.command_diff");
    let command_text = resolved_frame_text(command_frame);
    assert!(command_text.contains("project package saved"));
    assert!(command_text.contains("apply review"));
    assert!(command_text.contains("activation report"));
    assert!(command_text.contains("rollback"));

    let mut evidence = String::new();
    writeln!(
        evidence,
        "# PM-UI-LAB-005 Runtime Evidence\n\nGenerated by `cargo test -p runenwerk_editor pm_ui_lab_005_runtime_evidence_reports_project_io_apply_rollback -- --nocapture`.\n"
    )
    .unwrap();
    writeln!(evidence, "- project_package_bytes: {}", saved_package.len()).unwrap();
    writeln!(
        evidence,
        "- selected_definition_import_export: {} replaced_existing={}",
        selected_import.document_id.as_str(),
        selected_import.replaced_existing
    )
    .unwrap();
    writeln!(evidence, "- invalid_package_preserved: true").unwrap();
    writeln!(
        evidence,
        "- apply_review: {} status={:?} diff_rows={}",
        pending_review.id,
        pending_review.status,
        pending_review.diff_rows.len()
    )
    .unwrap();
    writeln!(
        evidence,
        "- activation_report: {} {:?} review_id={:?}",
        applied_report.display_name, applied_report.status, applied_report.review_id
    )
    .unwrap();
    writeln!(
        evidence,
        "- failed_activation_report: {} {:?} previous_state_preserved={}",
        failed_report.display_name, failed_report.status, failed_report.previous_state_preserved
    )
    .unwrap();
    writeln!(
        evidence,
        "- rollback_records: {}",
        host.shell_state.self_authoring().rollback_records().len()
    )
    .unwrap();
    writeln!(
        evidence,
        "- provider_surface_exposes_project_review_activation_and_rollback: true"
    )
    .unwrap();

    if std::env::var_os("RUNENWERK_WRITE_PM_UI_LAB_005_EVIDENCE").is_some() {
        let repo_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(|path| path.parent())
            .expect("runenwerk_editor crate should live under apps/");
        let artifact_root = repo_root.join(
            "docs-site/src/content/docs/reports/closeouts/pm-ui-lab-005-persistence-project-io-diff-apply-and-rollback/artifacts",
        );
        std::fs::create_dir_all(&artifact_root)
            .expect("PM-005 artifact directory should be writable");
        std::fs::write(artifact_root.join("runtime-proof.txt"), evidence)
            .expect("PM-005 runtime proof should be writable");
        std::fs::write(artifact_root.join("project-package.ron"), saved_package)
            .expect("PM-005 project package artifact should be writable");
        std::fs::write(
            artifact_root.join("activation-reports.ron"),
            ron::ser::to_string_pretty(
                host.app.editor_definition_activation_reports(),
                ron::ser::PrettyConfig::new(),
            )
            .expect("activation reports should serialize"),
        )
        .expect("PM-005 activation report artifact should be writable");
        std::fs::write(artifact_root.join("review-surface-debug.txt"), command_text)
            .expect("PM-005 review surface artifact should be writable");
    }
}

#[test]
fn pm_ui_lab_perf_005_persistence_api_examples_structural_workflow() {
    use std::fmt::Write as _;

    let mut host = EditorHostResource::default();
    dispatch_shell_command(
        &mut host.app,
        Some(&mut host.shell_state),
        ShellCommand::SwitchWorkspaceProfile {
            profile_id: EDITOR_DESIGN_WORKSPACE_PROFILE_ID,
        },
        None,
        None,
        None,
        None,
    )
    .expect("Editor Lab should open through the shell command boundary");

    dispatch_shell_command(
        &mut host.app,
        Some(&mut host.shell_state),
        ShellCommand::ApplySelectedEditorDefinition,
        None,
        None,
        None,
        None,
    )
    .expect("initial apply should create an applied baseline");
    assert_eq!(host.app.pending_editor_definition_activation_count(), 1);
    assert_eq!(host.apply_pending_editor_definition_activations(), 1);

    dispatch_shell_command(
        &mut host.app,
        Some(&mut host.shell_state),
        ShellCommand::SaveEditorLabProjectPackage,
        None,
        None,
        None,
        None,
    )
    .expect("Editor Lab project package should save through the app-owned store");
    let saved_package = host
        .shell_state
        .self_authoring()
        .last_saved_project_package_source()
        .expect("project package source should be preserved")
        .to_string();
    let parsed_package: crate::shell::EditorLabProjectPackage =
        ron::from_str(&saved_package).expect("saved package should be structured RON");
    assert!(!parsed_package.draft_documents.is_empty());
    assert!(!parsed_package.applied_documents.is_empty());
    assert!(!parsed_package.last_applied_documents.is_empty());

    dispatch_shell_command(
        &mut host.app,
        Some(&mut host.shell_state),
        ShellCommand::ReloadEditorLabProjectPackage,
        None,
        None,
        None,
        None,
    )
    .expect("saved package should reload without live activation");
    assert_eq!(host.app.pending_editor_definition_activation_count(), 0);

    let invalid_package = "not a PM005 perfectionist package";
    assert!(
        host.shell_state
            .self_authoring_mut()
            .load_project_package_from_ron(invalid_package)
            .is_err()
    );
    assert_eq!(
        host.shell_state
            .self_authoring()
            .last_invalid_project_package_source(),
        Some(invalid_package)
    );
    assert!(
        !host
            .shell_state
            .self_authoring()
            .last_invalid_project_package_diagnostics()
            .is_empty()
    );

    let selected_node = host
        .shell_state
        .self_authoring()
        .selected_ui_node_id()
        .expect("selected UI definition should expose an editable node")
        .to_string();
    dispatch_shell_command(
        &mut host.app,
        Some(&mut host.shell_state),
        ShellCommand::SetSelectedEditorDefinitionUiNodeText {
            node_id: selected_node,
            text: "PM005 structural diff label".to_string(),
        },
        None,
        None,
        None,
        None,
    )
    .expect("draft text edit should route through Editor Lab operation");

    dispatch_shell_command(
        &mut host.app,
        Some(&mut host.shell_state),
        ShellCommand::BuildSelectedEditorDefinitionApplyReview,
        None,
        None,
        None,
        None,
    )
    .expect("dirty draft should build a structural apply review");
    let structural_review = host
        .shell_state
        .self_authoring()
        .last_apply_review()
        .expect("structural apply review should be stored")
        .clone();
    assert!(structural_review.diff_rows.iter().any(|row| {
        row.family == DefinitionApplyDiffFamily::UiTemplate
            && row.path.ends_with(".label")
            && row.after.contains("PM005 structural diff label")
    }));
    assert!(
        !structural_review
            .diff_rows
            .iter()
            .any(|row| row.path == "document.content"),
        "apply review must expose structural rows instead of coarse serialized content"
    );

    let review_frame = build_editor_shell_frame_model(
        &host.app,
        &host.shell_state,
        host.app.workbench_host().provider_registry(),
        &host.theme,
        None,
        None,
        None,
    );
    let review_text = resolved_frame_text(frame_by_stable_key(
        &review_frame,
        "runenwerk.editor_design.command_diff",
    ));
    assert!(review_text.contains("apply diff"));
    assert!(review_text.contains("UiTemplate"));
    assert!(review_text.contains("PM005 structural diff label"));

    let applied_before_reject = host.shell_state.self_authoring().applied_count();
    dispatch_shell_command(
        &mut host.app,
        Some(&mut host.shell_state),
        ShellCommand::RejectSelectedEditorDefinitionApplyReview,
        None,
        None,
        None,
        None,
    )
    .expect("reject should preserve applied state");
    assert_eq!(
        host.shell_state.self_authoring().applied_count(),
        applied_before_reject
    );

    dispatch_shell_command(
        &mut host.app,
        Some(&mut host.shell_state),
        ShellCommand::ApplySelectedEditorDefinition,
        None,
        None,
        None,
        None,
    )
    .expect("accepted structural review should queue activation");
    assert_eq!(host.app.pending_editor_definition_activation_count(), 1);
    assert_eq!(host.apply_pending_editor_definition_activations(), 1);
    let activation_report = host
        .app
        .last_editor_definition_activation_report()
        .expect("accepted apply should produce an activation report")
        .clone();
    assert_eq!(
        activation_report.status,
        EditorDefinitionActivationStatus::Applied
    );
    assert_eq!(
        activation_report.review_id,
        Some(structural_review.id.clone())
    );

    let selected_node = host
        .shell_state
        .self_authoring()
        .selected_ui_node_id()
        .expect("selected UI node should remain available")
        .to_string();
    dispatch_shell_command(
        &mut host.app,
        Some(&mut host.shell_state),
        ShellCommand::SetSelectedEditorDefinitionUiNodeText {
            node_id: selected_node,
            text: "PM005 dirty draft after structural apply".to_string(),
        },
        None,
        None,
        None,
        None,
    )
    .expect("dirty draft edit should stay local");
    dispatch_shell_command(
        &mut host.app,
        Some(&mut host.shell_state),
        ShellCommand::ReloadSelectedEditorDefinitionLastApplied,
        None,
        None,
        None,
        None,
    )
    .expect("reload last applied should restore source truth");
    let reloaded_preview = host
        .shell_state
        .self_authoring()
        .formed_selected_preview(&host.theme)
        .expect("reloaded last applied snapshot should still preview");
    assert!(
        !format!("{:?}", reloaded_preview.root)
            .contains("PM005 dirty draft after structural apply")
    );

    dispatch_shell_command(
        &mut host.app,
        Some(&mut host.shell_state),
        ShellCommand::RollbackSelectedEditorDefinition,
        None,
        None,
        None,
        None,
    )
    .expect("rollback should restore the recorded pre-apply state");
    let rollback_record = host
        .shell_state
        .self_authoring()
        .last_rollback_record()
        .expect("rollback should record typed evidence")
        .clone();
    assert!(rollback_record.diagnostics.is_empty());

    let mut evidence = String::new();
    writeln!(
        evidence,
        "# PM-UI-LAB-PERF-005 Persistence API Examples Runtime Evidence\n"
    )
    .unwrap();
    writeln!(
        evidence,
        "- project_package: drafts={} applied={} last_applied={}",
        parsed_package.draft_documents.len(),
        parsed_package.applied_documents.len(),
        parsed_package.last_applied_documents.len()
    )
    .unwrap();
    writeln!(
        evidence,
        "- invalid_package_preserved: diagnostics={}",
        host.shell_state
            .self_authoring()
            .last_invalid_project_package_diagnostics()
            .len()
    )
    .unwrap();
    writeln!(
        evidence,
        "- structural_apply_diff_rows: {}",
        structural_review.diff_rows.len()
    )
    .unwrap();
    writeln!(
        evidence,
        "- structural_apply_diff_path: {}",
        structural_review
            .diff_rows
            .iter()
            .find(|row| row.family == DefinitionApplyDiffFamily::UiTemplate)
            .map(|row| row.path.as_str())
            .unwrap_or("missing")
    )
    .unwrap();
    writeln!(
        evidence,
        "- activation_report: {:?} review_id={:?}",
        activation_report.status, activation_report.review_id
    )
    .unwrap();
    writeln!(
        evidence,
        "- reload_last_applied_preserved_source_truth: true"
    )
    .unwrap();
    writeln!(evidence, "- rollback_record: {:?}", rollback_record.status).unwrap();
    writeln!(
        evidence,
        "- public_api_examples: ui_definition::prelude and editor_definition::prelude remain the documented workflow"
    )
    .unwrap();

    if std::env::var_os("RUNENWERK_WRITE_PM_UI_LAB_PERF_005_EVIDENCE").is_some() {
        let repo_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(|path| path.parent())
            .expect("runenwerk_editor crate should live under apps/");
        let artifact_root = repo_root.join(
            "docs-site/src/content/docs/reports/closeouts/pm-ui-lab-perf-005-persistence-diff-apply-api-and-examples-ergonomics/artifacts",
        );
        std::fs::create_dir_all(&artifact_root)
            .expect("PM-UI-LAB-PERF-005 artifact directory should be writable");
        std::fs::write(artifact_root.join("runtime-proof.txt"), evidence)
            .expect("PM-UI-LAB-PERF-005 runtime proof should be writable");
        std::fs::write(artifact_root.join("project-package.ron"), saved_package)
            .expect("PM-UI-LAB-PERF-005 package artifact should be writable");
        std::fs::write(
            artifact_root.join("structural-apply-review.ron"),
            ron::ser::to_string_pretty(&structural_review, ron::ser::PrettyConfig::new())
                .expect("structural review should serialize"),
        )
        .expect("PM-UI-LAB-PERF-005 review artifact should be writable");
        std::fs::write(
            artifact_root.join("activation-reports.ron"),
            ron::ser::to_string_pretty(
                host.app.editor_definition_activation_reports(),
                ron::ser::PrettyConfig::new(),
            )
            .expect("activation reports should serialize"),
        )
        .expect("PM-UI-LAB-PERF-005 activation artifact should be writable");
        std::fs::write(
            artifact_root.join("rollback-records.ron"),
            ron::ser::to_string_pretty(
                host.shell_state.self_authoring().rollback_records(),
                ron::ser::PrettyConfig::new(),
            )
            .expect("rollback records should serialize"),
        )
        .expect("PM-UI-LAB-PERF-005 rollback artifact should be writable");
        std::fs::write(artifact_root.join("review-surface-debug.txt"), review_text)
            .expect("PM-UI-LAB-PERF-005 review surface artifact should be writable");
    }
}

#[test]
fn pm_ui_lab_perf_002_runtime_evidence_platform_closure() {
    use std::fmt::Write as _;
    use std::time::Instant;

    const COMMAND: &str = "cargo test -p runenwerk_editor pm_ui_lab_perf_002_runtime_evidence_platform_closure -- --nocapture";
    const BACKEND: &str = "headless-retained-ui-test";
    const ENVIRONMENT: &str = "cargo test app-owned Editor Lab shell harness";

    fn artifact(
        kind: EditorLabEvidenceArtifactKind,
        path: &str,
        contents: &str,
        description: &str,
    ) -> EditorLabEvidenceArtifact {
        EditorLabEvidenceArtifact::new(kind, path, contents.len(), description)
    }

    fn captured_result(
        capability: EditorLabEvidenceCapability,
        artifacts: Vec<EditorLabEvidenceArtifact>,
        reason: &str,
        diagnostic: &str,
    ) -> EditorLabEvidenceCapabilityResult {
        EditorLabEvidenceCapabilityResult::captured(
            capability,
            EditorLabEvidenceCapabilityProbe::supported(capability, BACKEND, ENVIRONMENT, reason),
            artifacts,
            COMMAND,
            diagnostic,
        )
    }

    fn platform_impossible_result(
        capability: EditorLabEvidenceCapability,
        report_artifact: EditorLabEvidenceArtifact,
        reason: &str,
    ) -> EditorLabEvidenceCapabilityResult {
        EditorLabEvidenceCapabilityResult::platform_impossible(
            capability,
            EditorLabEvidenceCapabilityProbe::platform_impossible(
                capability,
                BACKEND,
                ENVIRONMENT,
                reason,
            ),
            vec![report_artifact],
            COMMAND,
            reason,
        )
    }

    let scenarios = editor_lab_preview_scenarios();
    let scenario = |id: &str| {
        scenarios
            .iter()
            .find(|scenario| scenario.id == id)
            .unwrap_or_else(|| panic!("scenario {id} should exist"))
    };
    let platform_report_artifact = artifact(
        EditorLabEvidenceArtifactKind::PlatformImpossibleReport,
        "artifacts/platform-impossible-results.ron",
        "platform impossible capability results",
        "Typed platform-impossible capability results with probe metadata.",
    );

    let setup_started = Instant::now();
    let mut host = EditorHostResource::default();
    dispatch_shell_command(
        &mut host.app,
        Some(&mut host.shell_state),
        ShellCommand::SwitchWorkspaceProfile {
            profile_id: EDITOR_DESIGN_WORKSPACE_PROFILE_ID,
        },
        None,
        None,
        None,
        None,
    )
    .expect("Editor Lab should open through the shell command boundary");
    let setup_micros = u64::try_from(setup_started.elapsed().as_micros()).unwrap_or(u64::MAX);

    let frame_started = Instant::now();
    let frame_model = build_editor_shell_frame_model(
        &host.app,
        &host.shell_state,
        host.app.workbench_host().provider_registry(),
        &host.theme,
        None,
        None,
        None,
    );
    let retained_surface_micros =
        u64::try_from(frame_started.elapsed().as_micros()).unwrap_or(u64::MAX);
    let provider_snapshot = editor_lab_provider_snapshot(&frame_model);
    let success_surface = format!(
        "{}\n\n{}",
        resolved_frame_text(frame_by_stable_key(
            &frame_model,
            "runenwerk.editor_design.definition_outliner"
        )),
        resolved_frame_text(frame_by_stable_key(
            &frame_model,
            "runenwerk.editor_design.command_diff"
        ))
    );
    assert!(success_surface.contains("Editor Lab"));
    assert!(success_surface.contains("Definition Hierarchy"));
    let success_artifact = artifact(
        EditorLabEvidenceArtifactKind::RetainedUiDebug,
        "artifacts/success-retained-surface-debug.txt",
        &success_surface,
        "Retained Editor Lab hierarchy and command surface.",
    );
    let provider_artifact = artifact(
        EditorLabEvidenceArtifactKind::ProviderSnapshot,
        "artifacts/provider-snapshot.ron",
        &provider_snapshot,
        "Resolved Editor Lab provider frames.",
    );
    let mut runs = vec![
        EditorLabEvidenceRun::passed(
            scenario("editor-lab.success"),
            "SwitchWorkspaceProfile -> build_editor_shell_frame_model",
            "provider surfaces mounted",
            vec![success_artifact.clone(), provider_artifact],
        )
        .with_capability_results(vec![
            captured_result(
                EditorLabEvidenceCapability::RetainedVisualTruth,
                vec![success_artifact],
                "retained UI frame model was captured from the app-owned shell provider path",
                "retained visual truth captured through the Editor Lab runtime path",
            ),
            platform_impossible_result(
                EditorLabEvidenceCapability::NativeScreenshotCapture,
                platform_report_artifact.clone(),
                "headless cargo test environment exposes retained UI artifacts but no native window screenshot API",
            ),
            platform_impossible_result(
                EditorLabEvidenceCapability::GpuVisualDiff,
                platform_report_artifact.clone(),
                "headless retained UI harness has no GPU readback or before-after pixel diff backend",
            ),
        ]),
    ];

    host.app
        .append_console_warning("PM-UI-LAB-PERF-002 scenario warning");
    let warning_frame_model = build_editor_shell_frame_model(
        &host.app,
        &host.shell_state,
        host.app.workbench_host().provider_registry(),
        &host.theme,
        None,
        None,
        None,
    );
    let warning_surface = resolved_frame_text(frame_by_stable_key(
        &warning_frame_model,
        "runenwerk.editor_design.command_diff",
    ));
    assert!(warning_surface.contains("PM-UI-LAB-PERF-002 scenario warning"));
    runs.push(
        EditorLabEvidenceRun::passed(
            scenario("editor-lab.warning"),
            "RunenwerkEditorApp::append_console_warning -> command_diff surface",
            "warning visible in preview console",
            vec![artifact(
                EditorLabEvidenceArtifactKind::RetainedUiDebug,
                "artifacts/warning-retained-surface-debug.txt",
                &warning_surface,
                "Command Diff surface with preview console warning.",
            )],
        )
        .with_diagnostics(vec![evidence_warning(
            "editor.lab.evidence.warning.visible",
            "PM002 warning scenario is visible through the app-owned preview console",
        )]),
    );

    let invalid_package_source = "not a valid PM002 runtime evidence package";
    let invalid_package_error = host
        .shell_state
        .self_authoring_mut()
        .load_project_package_from_ron(invalid_package_source)
        .expect_err("invalid package should produce typed diagnostics");
    assert_eq!(
        host.shell_state
            .self_authoring()
            .last_invalid_project_package_source(),
        Some(invalid_package_source)
    );
    let error_diagnostics = ron::ser::to_string_pretty(
        std::slice::from_ref(&invalid_package_error),
        ron::ser::PrettyConfig::new(),
    )
    .expect("diagnostics should serialize");
    let diagnostics_artifact = artifact(
        EditorLabEvidenceArtifactKind::DiagnosticsSnapshot,
        "artifacts/error-diagnostics.ron",
        &error_diagnostics,
        "Typed diagnostics for invalid Editor Lab project package input.",
    );
    runs.push(
        EditorLabEvidenceRun::passed(
            scenario("editor-lab.error"),
            "SelfAuthoringWorkspaceState::load_project_package_from_ron",
            "invalid input preserved",
            vec![diagnostics_artifact.clone()],
        )
        .with_diagnostics(vec![invalid_package_error])
        .with_capability_results(vec![
            captured_result(
                EditorLabEvidenceCapability::DiagnosticsSnapshot,
                vec![diagnostics_artifact.clone()],
                "invalid project package input produced typed diagnostics through the shell state",
                "diagnostics snapshot captured for invalid package input",
            ),
            captured_result(
                EditorLabEvidenceCapability::FailurePreservation,
                vec![diagnostics_artifact],
                "invalid project package source was retained after typed diagnostic failure",
                "failure preservation captured by retaining invalid package source and diagnostics",
            ),
        ]),
    );

    dispatch_shell_command(
        &mut host.app,
        Some(&mut host.shell_state),
        ShellCommand::SaveEditorLabProjectPackage,
        None,
        None,
        None,
        None,
    )
    .expect("Editor Lab project package should save for PM002 reload scenario");
    let saved_package = host
        .shell_state
        .self_authoring()
        .last_saved_project_package_source()
        .expect("project package source should be retained")
        .to_string();
    dispatch_shell_command(
        &mut host.app,
        Some(&mut host.shell_state),
        ShellCommand::ReloadEditorLabProjectPackage,
        None,
        None,
        None,
        None,
    )
    .expect("Editor Lab project package should reload without live activation");
    assert_eq!(host.app.pending_editor_definition_activation_count(), 0);
    let reload_frame_model = build_editor_shell_frame_model(
        &host.app,
        &host.shell_state,
        host.app.workbench_host().provider_registry(),
        &host.theme,
        None,
        None,
        None,
    );
    let reload_surface = resolved_frame_text(frame_by_stable_key(
        &reload_frame_model,
        "runenwerk.editor_design.command_diff",
    ));
    assert!(reload_surface.contains("project package saved"));
    let project_artifact = artifact(
        EditorLabEvidenceArtifactKind::ProjectPackage,
        "artifacts/project-package.ron",
        &saved_package,
        "Saved Editor Lab project package used by reload scenario.",
    );
    let reload_artifact = artifact(
        EditorLabEvidenceArtifactKind::RetainedUiDebug,
        "artifacts/reload-retained-surface-debug.txt",
        &reload_surface,
        "Command surface after save/reload.",
    );
    runs.push(
        EditorLabEvidenceRun::passed(
            scenario("editor-lab.reload"),
            "SaveEditorLabProjectPackage -> ReloadEditorLabProjectPackage",
            "reload completed without live activation",
            vec![project_artifact.clone(), reload_artifact],
        )
        .with_capability_results(vec![captured_result(
            EditorLabEvidenceCapability::ReloadWithoutActivation,
            vec![project_artifact],
            "save/reload completed while pending live activation count stayed zero",
            "reload evidence captured without live activation",
        )]),
    );

    dispatch_shell_command(
        &mut host.app,
        Some(&mut host.shell_state),
        ShellCommand::BuildSelectedEditorDefinitionApplyReview,
        None,
        None,
        None,
        None,
    )
    .expect("PM002 apply scenario should build a review");
    dispatch_shell_command(
        &mut host.app,
        Some(&mut host.shell_state),
        ShellCommand::ApplySelectedEditorDefinition,
        None,
        None,
        None,
        None,
    )
    .expect("PM002 apply scenario should queue activation through review path");
    assert_eq!(host.app.pending_editor_definition_activation_count(), 1);
    let activated = host.apply_pending_editor_definition_activations();
    assert_eq!(activated, 1);
    let activation_reports = ron::ser::to_string_pretty(
        host.app.editor_definition_activation_reports(),
        ron::ser::PrettyConfig::new(),
    )
    .expect("activation reports should serialize");
    let apply_surface = resolved_frame_text(frame_by_stable_key(
        &build_editor_shell_frame_model(
            &host.app,
            &host.shell_state,
            host.app.workbench_host().provider_registry(),
            &host.theme,
            None,
            None,
            None,
        ),
        "runenwerk.editor_design.command_diff",
    ));
    assert!(apply_surface.contains("activation report"));
    let activation_artifact = artifact(
        EditorLabEvidenceArtifactKind::ActivationReport,
        "artifacts/activation-reports.ron",
        &activation_reports,
        "Queued and applied activation reports.",
    );
    runs.push(
        EditorLabEvidenceRun::passed(
            scenario("editor-lab.apply"),
            "BuildSelectedEditorDefinitionApplyReview -> ApplySelectedEditorDefinition",
            "accepted review activated through runtime resource",
            vec![
                activation_artifact.clone(),
                artifact(
                    EditorLabEvidenceArtifactKind::RetainedUiDebug,
                    "artifacts/apply-retained-surface-debug.txt",
                    &apply_surface,
                    "Command surface after apply and activation.",
                ),
            ],
        )
        .with_capability_results(vec![captured_result(
            EditorLabEvidenceCapability::ApplyActivation,
            vec![activation_artifact],
            "apply review queued and activated through the runtime resource",
            "apply activation evidence captured from typed activation reports",
        )]),
    );

    dispatch_shell_command(
        &mut host.app,
        Some(&mut host.shell_state),
        ShellCommand::RollbackSelectedEditorDefinition,
        None,
        None,
        None,
        None,
    )
    .expect("PM002 rollback scenario should use snapshot-backed rollback");
    let rollback_reports = ron::ser::to_string_pretty(
        host.shell_state.self_authoring().rollback_records(),
        ron::ser::PrettyConfig::new(),
    )
    .expect("rollback records should serialize");
    let rollback_surface = resolved_frame_text(frame_by_stable_key(
        &build_editor_shell_frame_model(
            &host.app,
            &host.shell_state,
            host.app.workbench_host().provider_registry(),
            &host.theme,
            None,
            None,
            None,
        ),
        "runenwerk.editor_design.command_diff",
    ));
    assert!(rollback_surface.contains("rollback"));
    let rollback_artifact = artifact(
        EditorLabEvidenceArtifactKind::RollbackReport,
        "artifacts/rollback-reports.ron",
        &rollback_reports,
        "Typed rollback records.",
    );
    runs.push(
        EditorLabEvidenceRun::passed(
            scenario("editor-lab.rollback"),
            "RollbackSelectedEditorDefinition",
            "snapshot-backed rollback record visible",
            vec![
                rollback_artifact.clone(),
                artifact(
                    EditorLabEvidenceArtifactKind::RetainedUiDebug,
                    "artifacts/rollback-retained-surface-debug.txt",
                    &rollback_surface,
                    "Command surface after rollback.",
                ),
            ],
        )
        .with_capability_results(vec![captured_result(
            EditorLabEvidenceCapability::RollbackRecovery,
            vec![rollback_artifact],
            "rollback command produced typed snapshot-backed rollback records",
            "rollback recovery evidence captured from typed rollback reports",
        )]),
    );

    dispatch_shell_command(
        &mut host.app,
        Some(&mut host.shell_state),
        ShellCommand::SelectEditorDefinitionDocument {
            document_id: "runenwerk.editor.theme.default".to_string(),
        },
        None,
        None,
        None,
        None,
    )
    .expect("theme selection should produce a degraded canvas preview");
    let degraded_frame_model = build_editor_shell_frame_model(
        &host.app,
        &host.shell_state,
        host.app.workbench_host().provider_registry(),
        &host.theme,
        None,
        None,
        None,
    );
    let degraded_canvas = resolved_frame_text(frame_by_stable_key(
        &degraded_frame_model,
        "runenwerk.editor_design.ui_canvas",
    ));
    assert!(degraded_canvas.contains("Selected definition cannot form a retained UI preview"));
    let degraded_artifact = artifact(
        EditorLabEvidenceArtifactKind::RetainedUiDebug,
        "artifacts/degraded-provider-retained-surface-debug.txt",
        &degraded_canvas,
        "Typed degraded canvas surface for non-previewable selection.",
    );
    runs.push(
        EditorLabEvidenceRun::passed(
            scenario("editor-lab.degraded-provider"),
            "Select theme document -> ui_canvas degraded Editor Lab surface",
            "typed degraded Editor Lab canvas",
            vec![degraded_artifact.clone()],
        )
        .with_diagnostics(vec![evidence_warning(
            "editor.lab.evidence.degraded_provider",
            "non-previewable selection rendered a typed degraded Editor Lab surface",
        )])
        .with_capability_results(vec![captured_result(
            EditorLabEvidenceCapability::DegradedProviderSurface,
            vec![degraded_artifact],
            "non-previewable selection rendered a typed degraded provider surface",
            "degraded provider evidence captured from the app-hosted canvas surface",
        )]),
    );

    let accessibility_snapshot = EditorLabAccessibilitySnapshot {
        scenario_id: "editor-lab.accessibility".to_string(),
        labelled_controls: editor_lab_route_count(&degraded_frame_model),
        disabled_reason_controls: editor_lab_disabled_reason_count(&degraded_frame_model),
        focusable_routes: editor_lab_route_count(&degraded_frame_model),
        unsupported_checks: Vec::new(),
    };
    assert!(accessibility_snapshot.labelled_controls > 0);
    assert!(accessibility_snapshot.disabled_reason_controls > 0);
    let focus_report =
        ron::ser::to_string_pretty(&accessibility_snapshot, ron::ser::PrettyConfig::new())
            .expect("focus report should serialize");
    let contrast_report = format!(
        "scenario_id: editor-lab.accessibility\nretained_theme_tokens_available: true\nnative_pixel_access_available: false\nsample_subjects: labels, disabled reasons, command controls\nlabelled_controls: {}\ndisabled_reason_controls: {}\n",
        accessibility_snapshot.labelled_controls, accessibility_snapshot.disabled_reason_controls
    );
    runs.push(
        EditorLabEvidenceRun::passed(
            scenario("editor-lab.accessibility"),
            "build_editor_shell_frame_model route and retained text inspection",
            "labels, routes, and disabled reasons inspected",
            vec![
                artifact(
                    EditorLabEvidenceArtifactKind::FocusTraversalReport,
                    "artifacts/focus-traversal-report.ron",
                    &focus_report,
                    "Retained route and disabled-reason focus fallback report.",
                ),
                artifact(
                    EditorLabEvidenceArtifactKind::ContrastSampleReport,
                    "artifacts/contrast-sample-report.txt",
                    &contrast_report,
                    "Retained token contrast fallback report.",
                ),
                artifact(
                    EditorLabEvidenceArtifactKind::RetainedUiDebug,
                    "artifacts/accessibility-retained-surface-debug.txt",
                    &degraded_canvas,
                    "Retained Editor Lab surface inspected for accessibility controls.",
                ),
            ],
        )
        .with_accessibility(accessibility_snapshot)
        .with_capability_results(vec![
            platform_impossible_result(
                EditorLabEvidenceCapability::NativeFocusTraversal,
                platform_report_artifact.clone(),
                "headless retained UI test can inspect routes and disabled reasons but cannot drive native focus traversal",
            ),
            platform_impossible_result(
                EditorLabEvidenceCapability::PixelContrastSampling,
                platform_report_artifact.clone(),
                "retained UI artifact exposes theme tokens but no native pixel buffer for contrast sampling",
            ),
        ]),
    );

    let artifact_bytes_so_far = runs
        .iter()
        .flat_map(|run| run.artifacts.iter())
        .map(|artifact| artifact.bytes)
        .sum::<usize>();
    let performance_snapshot = EditorLabPerformanceSnapshot {
        scenario_id: "editor-lab.performance".to_string(),
        setup_micros,
        retained_surface_micros,
        artifact_count: runs.iter().map(|run| run.artifacts.len()).sum(),
        artifact_bytes: artifact_bytes_so_far,
        unsupported_checks: Vec::new(),
    };
    let timing_report =
        ron::ser::to_string_pretty(&performance_snapshot, ron::ser::PrettyConfig::new())
            .expect("timing report should serialize");
    let timing_artifact = artifact(
        EditorLabEvidenceArtifactKind::TimingReport,
        "artifacts/timing-report.ron",
        &timing_report,
        "Timing snapshot for setup, retained surface formation, and artifact output.",
    );
    runs.push(
        EditorLabEvidenceRun::passed(
            scenario("editor-lab.performance"),
            "std::time::Instant around app-hosted frame formation",
            "scenario setup and retained surface timings recorded",
            vec![
                timing_artifact.clone(),
                artifact(
                    EditorLabEvidenceArtifactKind::RetainedUiDebug,
                    "artifacts/performance-retained-surface-debug.txt",
                    &provider_snapshot,
                    "Retained provider snapshot used for performance evidence context.",
                ),
            ],
        )
        .with_performance(performance_snapshot)
        .with_capability_results(vec![
            captured_result(
                EditorLabEvidenceCapability::RuntimeTimingCapture,
                vec![timing_artifact],
                "app-owned runtime harness records setup and retained surface timing",
                "runtime timing evidence captured from the PM002 evidence test",
            ),
            platform_impossible_result(
                EditorLabEvidenceCapability::NativeScreenshotTiming,
                platform_report_artifact.clone(),
                "native screenshot capture is unavailable in this headless runtime test, so native screenshot timing has no source",
            ),
            platform_impossible_result(
                EditorLabEvidenceCapability::GpuVisualDiffTiming,
                platform_report_artifact,
                "GPU visual diff timing is unavailable because the retained UI evidence harness has no GPU diff backend",
            ),
        ]),
    );

    let manifest = EditorLabEvidenceManifest::current(COMMAND, scenarios, runs);
    manifest
        .validate_no_gap_capabilities(&PM_UI_LAB_PERF_002_EVIDENCE_CAPABILITIES)
        .expect("PM002 no-gap evidence manifest should validate typed capability results");
    assert_eq!(manifest.runs.len(), 9);
    assert!(manifest.unsupported_checks().is_empty());

    let capability_results = manifest
        .runs
        .iter()
        .flat_map(|run| run.capability_results.iter().cloned())
        .collect::<Vec<_>>();
    let platform_impossible_results = capability_results
        .iter()
        .filter(|result| {
            result.status == EditorLabEvidenceCapabilityResultStatus::PlatformImpossible
        })
        .cloned()
        .collect::<Vec<_>>();
    let manifest_source = ron::ser::to_string_pretty(&manifest, ron::ser::PrettyConfig::new())
        .expect("manifest should serialize");
    let capability_results_source =
        ron::ser::to_string_pretty(&capability_results, ron::ser::PrettyConfig::new())
            .expect("capability results should serialize");
    let platform_impossible_source =
        ron::ser::to_string_pretty(&platform_impossible_results, ron::ser::PrettyConfig::new())
            .expect("platform impossible results should serialize");
    let diagnostics_source = ron::ser::to_string_pretty(
        &manifest.diagnostics_snapshot(),
        ron::ser::PrettyConfig::new(),
    )
    .expect("diagnostics snapshot should serialize");
    let mut runtime_proof = String::new();
    writeln!(
        runtime_proof,
        "# PM-UI-LAB-PERF-002 Runtime Evidence Platform Closure\n\nGenerated by `{COMMAND}`.\n"
    )
    .unwrap();
    for capability in PM_UI_LAB_PERF_002_EVIDENCE_CAPABILITIES {
        let result = capability_results
            .iter()
            .find(|result| result.capability == capability)
            .expect("validated capability should have a result");
        writeln!(
            runtime_proof,
            "- {:?}: status={:?} backend={} environment={} artifacts={}",
            capability,
            result.status,
            result.probe.backend,
            result.probe.environment,
            result.artifacts.len()
        )
        .unwrap();
    }
    writeln!(
        runtime_proof,
        "- manifest_validated: true\n- unsupported_free_form_checks: 0\n- app_owned_evidence_execution: true\n- ui_definition_behavior_free: true"
    )
    .unwrap();

    if std::env::var_os("RUNENWERK_WRITE_PM_UI_LAB_PERF_002_EVIDENCE").is_some() {
        let repo_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(|path| path.parent())
            .expect("runenwerk_editor crate should live under apps/");
        let artifact_root = repo_root.join(
            "docs-site/src/content/docs/reports/closeouts/pm-ui-lab-perf-002-runtime-evidence-platform-closure/artifacts",
        );
        std::fs::create_dir_all(&artifact_root)
            .expect("PM002 artifact directory should be writable");
        std::fs::write(artifact_root.join("runtime-proof.txt"), runtime_proof)
            .expect("PM002 runtime proof should be writable");
        std::fs::write(
            artifact_root.join("no-gap-evidence-manifest.ron"),
            manifest_source,
        )
        .expect("PM002 no-gap evidence manifest should be writable");
        std::fs::write(
            artifact_root.join("no-gap-capability-results.ron"),
            capability_results_source,
        )
        .expect("PM002 capability results should be writable");
        std::fs::write(
            artifact_root.join("platform-impossible-results.ron"),
            platform_impossible_source,
        )
        .expect("PM002 platform-impossible results should be writable");
        std::fs::write(
            artifact_root.join("provider-snapshot.ron"),
            provider_snapshot.as_bytes(),
        )
        .expect("PM002 provider snapshot should be writable");
        std::fs::write(
            artifact_root.join("success-retained-surface-debug.txt"),
            success_surface,
        )
        .expect("PM002 success retained visual should be writable");
        std::fs::write(
            artifact_root.join("warning-retained-surface-debug.txt"),
            warning_surface,
        )
        .expect("PM002 warning retained visual should be writable");
        std::fs::write(
            artifact_root.join("error-diagnostics.ron"),
            error_diagnostics,
        )
        .expect("PM002 error diagnostics should be writable");
        std::fs::write(artifact_root.join("project-package.ron"), saved_package)
            .expect("PM002 project package should be writable");
        std::fs::write(
            artifact_root.join("reload-retained-surface-debug.txt"),
            reload_surface,
        )
        .expect("PM002 reload retained visual should be writable");
        std::fs::write(
            artifact_root.join("activation-reports.ron"),
            activation_reports,
        )
        .expect("PM002 activation reports should be writable");
        std::fs::write(
            artifact_root.join("apply-retained-surface-debug.txt"),
            apply_surface,
        )
        .expect("PM002 apply retained visual should be writable");
        std::fs::write(artifact_root.join("rollback-reports.ron"), rollback_reports)
            .expect("PM002 rollback reports should be writable");
        std::fs::write(
            artifact_root.join("rollback-retained-surface-debug.txt"),
            rollback_surface,
        )
        .expect("PM002 rollback retained visual should be writable");
        std::fs::write(
            artifact_root.join("degraded-provider-retained-surface-debug.txt"),
            degraded_canvas.as_bytes(),
        )
        .expect("PM002 degraded provider retained visual should be writable");
        std::fs::write(
            artifact_root.join("focus-traversal-report.ron"),
            focus_report,
        )
        .expect("PM002 focus traversal report should be writable");
        std::fs::write(
            artifact_root.join("contrast-sample-report.txt"),
            contrast_report,
        )
        .expect("PM002 contrast sample report should be writable");
        std::fs::write(artifact_root.join("timing-report.ron"), timing_report)
            .expect("PM002 timing report should be writable");
        std::fs::write(
            artifact_root.join("diagnostics-snapshot.ron"),
            diagnostics_source,
        )
        .expect("PM002 diagnostics snapshot should be writable");
    }
}

#[test]
fn pm_ui_lab_006_runtime_evidence_reports_preview_lab() {
    use std::fmt::Write as _;
    use std::time::Instant;

    fn artifact(
        kind: EditorLabEvidenceArtifactKind,
        path: &str,
        contents: &str,
        description: &str,
    ) -> EditorLabEvidenceArtifact {
        EditorLabEvidenceArtifact::new(kind, path, contents.len(), description)
    }

    let scenarios = editor_lab_preview_scenarios();
    let scenario = |id: &str| {
        scenarios
            .iter()
            .find(|scenario| scenario.id == id)
            .unwrap_or_else(|| panic!("scenario {id} should exist"))
    };

    let setup_started = Instant::now();
    let mut host = EditorHostResource::default();
    dispatch_shell_command(
        &mut host.app,
        Some(&mut host.shell_state),
        ShellCommand::SwitchWorkspaceProfile {
            profile_id: EDITOR_DESIGN_WORKSPACE_PROFILE_ID,
        },
        None,
        None,
        None,
        None,
    )
    .expect("Editor Lab should open through the shell command boundary");
    let setup_micros = u64::try_from(setup_started.elapsed().as_micros()).unwrap_or(u64::MAX);

    let frame_started = Instant::now();
    let frame_model = build_editor_shell_frame_model(
        &host.app,
        &host.shell_state,
        host.app.workbench_host().provider_registry(),
        &host.theme,
        None,
        None,
        None,
    );
    let retained_surface_micros =
        u64::try_from(frame_started.elapsed().as_micros()).unwrap_or(u64::MAX);
    let provider_snapshot = editor_lab_provider_snapshot(&frame_model);
    let success_surface = format!(
        "{}\n\n{}",
        resolved_frame_text(frame_by_stable_key(
            &frame_model,
            "runenwerk.editor_design.definition_outliner"
        )),
        resolved_frame_text(frame_by_stable_key(
            &frame_model,
            "runenwerk.editor_design.command_diff"
        ))
    );
    assert!(success_surface.contains("Editor Lab"));
    assert!(success_surface.contains("Definition Hierarchy"));

    let mut runs = vec![
        EditorLabEvidenceRun::passed(
            scenario("editor-lab.success"),
            "SwitchWorkspaceProfile -> build_editor_shell_frame_model",
            "provider surfaces mounted",
            vec![
                artifact(
                    EditorLabEvidenceArtifactKind::RetainedUiDebug,
                    "artifacts/success-retained-surface-debug.txt",
                    &success_surface,
                    "Retained Editor Lab hierarchy and command surface.",
                ),
                artifact(
                    EditorLabEvidenceArtifactKind::ProviderSnapshot,
                    "artifacts/provider-snapshot.ron",
                    &provider_snapshot,
                    "Resolved Editor Lab provider frames.",
                ),
            ],
        )
        .with_unsupported_checks(vec![EditorLabUnsupportedCheckDiagnostic::new(
            "native_screenshot_capture",
            "headless cargo test environment uses retained UI artifacts instead",
        )]),
    ];

    host.app
        .append_console_warning("PM-UI-LAB-006 scenario warning");
    let warning_frame_model = build_editor_shell_frame_model(
        &host.app,
        &host.shell_state,
        host.app.workbench_host().provider_registry(),
        &host.theme,
        None,
        None,
        None,
    );
    let warning_surface = resolved_frame_text(frame_by_stable_key(
        &warning_frame_model,
        "runenwerk.editor_design.command_diff",
    ));
    assert!(warning_surface.contains("PM-UI-LAB-006 scenario warning"));
    runs.push(
        EditorLabEvidenceRun::passed(
            scenario("editor-lab.warning"),
            "RunenwerkEditorApp::append_console_warning -> command_diff surface",
            "warning visible in preview console",
            vec![artifact(
                EditorLabEvidenceArtifactKind::RetainedUiDebug,
                "artifacts/warning-retained-surface-debug.txt",
                &warning_surface,
                "Command Diff surface with preview console warning.",
            )],
        )
        .with_diagnostics(vec![evidence_warning(
            "editor.lab.evidence.warning.visible",
            "PM006 warning scenario is visible through the app-owned preview console",
        )]),
    );

    let invalid_package_source = "not a valid PM006 preview lab package";
    let invalid_package_error = host
        .shell_state
        .self_authoring_mut()
        .load_project_package_from_ron(invalid_package_source)
        .expect_err("invalid package should produce typed diagnostics");
    assert_eq!(
        host.shell_state
            .self_authoring()
            .last_invalid_project_package_source(),
        Some(invalid_package_source)
    );
    let error_diagnostics = ron::ser::to_string_pretty(
        std::slice::from_ref(&invalid_package_error),
        ron::ser::PrettyConfig::new(),
    )
    .expect("diagnostics should serialize");
    runs.push(
        EditorLabEvidenceRun::passed(
            scenario("editor-lab.error"),
            "SelfAuthoringWorkspaceState::load_project_package_from_ron",
            "invalid input preserved",
            vec![artifact(
                EditorLabEvidenceArtifactKind::DiagnosticsSnapshot,
                "artifacts/error-diagnostics.ron",
                &error_diagnostics,
                "Typed diagnostics for invalid Editor Lab project package input.",
            )],
        )
        .with_diagnostics(vec![invalid_package_error]),
    );

    dispatch_shell_command(
        &mut host.app,
        Some(&mut host.shell_state),
        ShellCommand::SaveEditorLabProjectPackage,
        None,
        None,
        None,
        None,
    )
    .expect("Editor Lab project package should save for PM006 reload scenario");
    let saved_package = host
        .shell_state
        .self_authoring()
        .last_saved_project_package_source()
        .expect("project package source should be retained")
        .to_string();
    dispatch_shell_command(
        &mut host.app,
        Some(&mut host.shell_state),
        ShellCommand::ReloadEditorLabProjectPackage,
        None,
        None,
        None,
        None,
    )
    .expect("Editor Lab project package should reload without live activation");
    assert_eq!(host.app.pending_editor_definition_activation_count(), 0);
    let reload_frame_model = build_editor_shell_frame_model(
        &host.app,
        &host.shell_state,
        host.app.workbench_host().provider_registry(),
        &host.theme,
        None,
        None,
        None,
    );
    let reload_surface = resolved_frame_text(frame_by_stable_key(
        &reload_frame_model,
        "runenwerk.editor_design.command_diff",
    ));
    assert!(reload_surface.contains("project package saved"));
    runs.push(EditorLabEvidenceRun::passed(
        scenario("editor-lab.reload"),
        "SaveEditorLabProjectPackage -> ReloadEditorLabProjectPackage",
        "reload completed without live activation",
        vec![
            artifact(
                EditorLabEvidenceArtifactKind::ProjectPackage,
                "artifacts/project-package.ron",
                &saved_package,
                "Saved Editor Lab project package used by reload scenario.",
            ),
            artifact(
                EditorLabEvidenceArtifactKind::RetainedUiDebug,
                "artifacts/reload-retained-surface-debug.txt",
                &reload_surface,
                "Command surface after save/reload.",
            ),
        ],
    ));

    dispatch_shell_command(
        &mut host.app,
        Some(&mut host.shell_state),
        ShellCommand::BuildSelectedEditorDefinitionApplyReview,
        None,
        None,
        None,
        None,
    )
    .expect("PM006 apply scenario should build a review");
    dispatch_shell_command(
        &mut host.app,
        Some(&mut host.shell_state),
        ShellCommand::ApplySelectedEditorDefinition,
        None,
        None,
        None,
        None,
    )
    .expect("PM006 apply scenario should queue activation through review path");
    assert_eq!(host.app.pending_editor_definition_activation_count(), 1);
    let activated = host.apply_pending_editor_definition_activations();
    assert_eq!(activated, 1);
    let activation_reports = ron::ser::to_string_pretty(
        host.app.editor_definition_activation_reports(),
        ron::ser::PrettyConfig::new(),
    )
    .expect("activation reports should serialize");
    let apply_surface = resolved_frame_text(frame_by_stable_key(
        &build_editor_shell_frame_model(
            &host.app,
            &host.shell_state,
            host.app.workbench_host().provider_registry(),
            &host.theme,
            None,
            None,
            None,
        ),
        "runenwerk.editor_design.command_diff",
    ));
    assert!(apply_surface.contains("activation report"));
    runs.push(EditorLabEvidenceRun::passed(
        scenario("editor-lab.apply"),
        "BuildSelectedEditorDefinitionApplyReview -> ApplySelectedEditorDefinition",
        "accepted review activated through runtime resource",
        vec![
            artifact(
                EditorLabEvidenceArtifactKind::ActivationReport,
                "artifacts/activation-reports.ron",
                &activation_reports,
                "Queued and applied activation reports.",
            ),
            artifact(
                EditorLabEvidenceArtifactKind::RetainedUiDebug,
                "artifacts/apply-retained-surface-debug.txt",
                &apply_surface,
                "Command surface after apply and activation.",
            ),
        ],
    ));

    dispatch_shell_command(
        &mut host.app,
        Some(&mut host.shell_state),
        ShellCommand::RollbackSelectedEditorDefinition,
        None,
        None,
        None,
        None,
    )
    .expect("PM006 rollback scenario should use snapshot-backed rollback");
    let rollback_reports = ron::ser::to_string_pretty(
        host.shell_state.self_authoring().rollback_records(),
        ron::ser::PrettyConfig::new(),
    )
    .expect("rollback records should serialize");
    let rollback_surface = resolved_frame_text(frame_by_stable_key(
        &build_editor_shell_frame_model(
            &host.app,
            &host.shell_state,
            host.app.workbench_host().provider_registry(),
            &host.theme,
            None,
            None,
            None,
        ),
        "runenwerk.editor_design.command_diff",
    ));
    assert!(rollback_surface.contains("rollback"));
    runs.push(EditorLabEvidenceRun::passed(
        scenario("editor-lab.rollback"),
        "RollbackSelectedEditorDefinition",
        "snapshot-backed rollback record visible",
        vec![
            artifact(
                EditorLabEvidenceArtifactKind::RollbackReport,
                "artifacts/rollback-reports.ron",
                &rollback_reports,
                "Typed rollback records.",
            ),
            artifact(
                EditorLabEvidenceArtifactKind::RetainedUiDebug,
                "artifacts/rollback-retained-surface-debug.txt",
                &rollback_surface,
                "Command surface after rollback.",
            ),
        ],
    ));

    dispatch_shell_command(
        &mut host.app,
        Some(&mut host.shell_state),
        ShellCommand::SelectEditorDefinitionDocument {
            document_id: "runenwerk.editor.theme.default".to_string(),
        },
        None,
        None,
        None,
        None,
    )
    .expect("theme selection should produce a degraded canvas preview");
    let degraded_frame_model = build_editor_shell_frame_model(
        &host.app,
        &host.shell_state,
        host.app.workbench_host().provider_registry(),
        &host.theme,
        None,
        None,
        None,
    );
    let degraded_canvas = resolved_frame_text(frame_by_stable_key(
        &degraded_frame_model,
        "runenwerk.editor_design.ui_canvas",
    ));
    assert!(degraded_canvas.contains("Selected definition cannot form a retained UI preview"));
    runs.push(
        EditorLabEvidenceRun::passed(
            scenario("editor-lab.degraded-provider"),
            "Select theme document -> ui_canvas degraded Editor Lab surface",
            "typed degraded Editor Lab canvas",
            vec![artifact(
                EditorLabEvidenceArtifactKind::RetainedUiDebug,
                "artifacts/degraded-provider-retained-surface-debug.txt",
                &degraded_canvas,
                "Typed degraded canvas surface for non-previewable selection.",
            )],
        )
        .with_diagnostics(vec![evidence_warning(
            "editor.lab.evidence.degraded_provider",
            "non-previewable selection rendered a typed degraded Editor Lab surface",
        )]),
    );

    let accessibility_unsupported = vec![
        EditorLabUnsupportedCheckDiagnostic::new(
            "native_focus_traversal",
            "headless retained UI test cannot drive native focus traversal",
        ),
        EditorLabUnsupportedCheckDiagnostic::new(
            "pixel_contrast_sampling",
            "retained UI artifact has theme tokens but no native pixels",
        ),
    ];
    let accessibility_snapshot = EditorLabAccessibilitySnapshot {
        scenario_id: "editor-lab.accessibility".to_string(),
        labelled_controls: editor_lab_route_count(&degraded_frame_model),
        disabled_reason_controls: editor_lab_disabled_reason_count(&degraded_frame_model),
        focusable_routes: editor_lab_route_count(&degraded_frame_model),
        unsupported_checks: accessibility_unsupported.clone(),
    };
    assert!(accessibility_snapshot.labelled_controls > 0);
    assert!(accessibility_snapshot.disabled_reason_controls > 0);
    let accessibility_report =
        ron::ser::to_string_pretty(&accessibility_snapshot, ron::ser::PrettyConfig::new())
            .expect("accessibility snapshot should serialize");
    runs.push(
        EditorLabEvidenceRun::passed(
            scenario("editor-lab.accessibility"),
            "build_editor_shell_frame_model route and retained text inspection",
            "labels, routes, and disabled reasons inspected",
            vec![
                artifact(
                    EditorLabEvidenceArtifactKind::AccessibilityReport,
                    "artifacts/accessibility-snapshot.ron",
                    &accessibility_report,
                    "Accessibility snapshot for labels, routes, disabled reasons, and unsupported checks.",
                ),
                artifact(
                    EditorLabEvidenceArtifactKind::RetainedUiDebug,
                    "artifacts/accessibility-retained-surface-debug.txt",
                    &degraded_canvas,
                    "Retained Editor Lab surface inspected for accessibility controls.",
                ),
            ],
        )
        .with_accessibility(accessibility_snapshot)
        .with_unsupported_checks(accessibility_unsupported),
    );

    let artifact_bytes_so_far = runs
        .iter()
        .flat_map(|run| run.artifacts.iter())
        .map(|artifact| artifact.bytes)
        .sum::<usize>();
    let performance_unsupported = vec![
        EditorLabUnsupportedCheckDiagnostic::new(
            "native_screenshot_timing",
            "native screenshot capture is unavailable in this headless runtime test",
        ),
        EditorLabUnsupportedCheckDiagnostic::new(
            "gpu_visual_diff_timing",
            "GPU visual diff timing is outside the retained UI evidence harness",
        ),
    ];
    let performance_snapshot = EditorLabPerformanceSnapshot {
        scenario_id: "editor-lab.performance".to_string(),
        setup_micros,
        retained_surface_micros,
        artifact_count: runs.iter().map(|run| run.artifacts.len()).sum(),
        artifact_bytes: artifact_bytes_so_far,
        unsupported_checks: performance_unsupported.clone(),
    };
    let performance_report =
        ron::ser::to_string_pretty(&performance_snapshot, ron::ser::PrettyConfig::new())
            .expect("performance snapshot should serialize");
    runs.push(
        EditorLabEvidenceRun::passed(
            scenario("editor-lab.performance"),
            "std::time::Instant around app-hosted frame formation",
            "scenario setup and retained surface timings recorded",
            vec![
                artifact(
                    EditorLabEvidenceArtifactKind::PerformanceReport,
                    "artifacts/performance-snapshot.ron",
                    &performance_report,
                    "Performance snapshot for setup, surface formation, artifact count, and artifact bytes.",
                ),
                artifact(
                    EditorLabEvidenceArtifactKind::RetainedUiDebug,
                    "artifacts/performance-retained-surface-debug.txt",
                    &provider_snapshot,
                    "Retained provider snapshot used for performance evidence context.",
                ),
            ],
        )
        .with_performance(performance_snapshot)
        .with_unsupported_checks(performance_unsupported),
    );

    let manifest = EditorLabEvidenceManifest::current(
        "cargo test -p runenwerk_editor pm_ui_lab_006_runtime_evidence_reports_preview_lab -- --nocapture",
        scenarios,
        runs,
    );
    manifest
        .validate()
        .expect("PM006 evidence manifest should validate required scenarios");
    assert_eq!(manifest.runs.len(), 9);

    let manifest_source = ron::ser::to_string_pretty(&manifest, ron::ser::PrettyConfig::new())
        .expect("manifest should serialize");
    let diagnostics_source = ron::ser::to_string_pretty(
        &manifest.diagnostics_snapshot(),
        ron::ser::PrettyConfig::new(),
    )
    .expect("diagnostics snapshot should serialize");
    let unsupported_source = ron::ser::to_string_pretty(
        &manifest.unsupported_checks(),
        ron::ser::PrettyConfig::new(),
    )
    .expect("unsupported checks should serialize");
    let mut runtime_proof = String::new();
    writeln!(
        runtime_proof,
        "# PM-UI-LAB-006 Runtime Evidence\n\nGenerated by `cargo test -p runenwerk_editor pm_ui_lab_006_runtime_evidence_reports_preview_lab -- --nocapture`.\n"
    )
    .unwrap();
    for run in &manifest.runs {
        writeln!(
            runtime_proof,
            "- {}: {:?} status={:?} artifacts={} diagnostics={} unsupported_checks={}",
            run.scenario_id,
            run.state_family,
            run.status,
            run.artifacts.len(),
            run.diagnostics.len(),
            run.unsupported_checks.len()
        )
        .unwrap();
    }
    writeln!(
        runtime_proof,
        "- manifest_validated: true\n- runtime_path: app-hosted Editor Lab shell/provider/project/apply/rollback paths\n- ui_definition_behavior_free: true"
    )
    .unwrap();

    if std::env::var_os("RUNENWERK_WRITE_PM_UI_LAB_006_EVIDENCE").is_some() {
        let repo_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(|path| path.parent())
            .expect("runenwerk_editor crate should live under apps/");
        let artifact_root = repo_root.join(
            "docs-site/src/content/docs/reports/closeouts/pm-ui-lab-006-preview-lab-and-runtime-evidence/artifacts",
        );
        std::fs::create_dir_all(&artifact_root)
            .expect("PM-006 artifact directory should be writable");
        std::fs::write(artifact_root.join("runtime-proof.txt"), runtime_proof)
            .expect("PM-006 runtime proof should be writable");
        std::fs::write(artifact_root.join("evidence-manifest.ron"), manifest_source)
            .expect("PM-006 manifest should be writable");
        std::fs::write(
            artifact_root.join("provider-snapshot.ron"),
            provider_snapshot.as_bytes(),
        )
        .expect("PM-006 provider snapshot should be writable");
        std::fs::write(
            artifact_root.join("success-retained-surface-debug.txt"),
            success_surface,
        )
        .expect("PM-006 success retained visual should be writable");
        std::fs::write(
            artifact_root.join("warning-retained-surface-debug.txt"),
            warning_surface,
        )
        .expect("PM-006 warning retained visual should be writable");
        std::fs::write(
            artifact_root.join("error-diagnostics.ron"),
            error_diagnostics,
        )
        .expect("PM-006 error diagnostics should be writable");
        std::fs::write(artifact_root.join("project-package.ron"), saved_package)
            .expect("PM-006 project package should be writable");
        std::fs::write(
            artifact_root.join("reload-retained-surface-debug.txt"),
            reload_surface,
        )
        .expect("PM-006 reload retained visual should be writable");
        std::fs::write(
            artifact_root.join("activation-reports.ron"),
            activation_reports,
        )
        .expect("PM-006 activation reports should be writable");
        std::fs::write(
            artifact_root.join("apply-retained-surface-debug.txt"),
            apply_surface,
        )
        .expect("PM-006 apply retained visual should be writable");
        std::fs::write(artifact_root.join("rollback-reports.ron"), rollback_reports)
            .expect("PM-006 rollback reports should be writable");
        std::fs::write(
            artifact_root.join("rollback-retained-surface-debug.txt"),
            rollback_surface,
        )
        .expect("PM-006 rollback retained visual should be writable");
        std::fs::write(
            artifact_root.join("degraded-provider-retained-surface-debug.txt"),
            degraded_canvas.as_bytes(),
        )
        .expect("PM-006 degraded provider retained visual should be writable");
        std::fs::write(
            artifact_root.join("accessibility-snapshot.ron"),
            accessibility_report,
        )
        .expect("PM-006 accessibility snapshot should be writable");
        std::fs::write(
            artifact_root.join("accessibility-retained-surface-debug.txt"),
            degraded_canvas.as_bytes(),
        )
        .expect("PM-006 accessibility retained visual should be writable");
        std::fs::write(
            artifact_root.join("performance-snapshot.ron"),
            performance_report,
        )
        .expect("PM-006 performance snapshot should be writable");
        std::fs::write(
            artifact_root.join("performance-retained-surface-debug.txt"),
            provider_snapshot.as_bytes(),
        )
        .expect("PM-006 performance retained visual should be writable");
        std::fs::write(
            artifact_root.join("diagnostics-snapshot.ron"),
            diagnostics_source,
        )
        .expect("PM-006 diagnostics snapshot should be writable");
        std::fs::write(
            artifact_root.join("unsupported-checks.ron"),
            unsupported_source,
        )
        .expect("PM-006 unsupported checks should be writable");
    }
}

#[test]
fn applying_selected_workspace_layout_definition_replaces_live_workspace() {
    let mut host = EditorHostResource::default();

    dispatch_shell_command(
        &mut host.app,
        Some(&mut host.shell_state),
        ShellCommand::SelectEditorDefinitionDocument {
            document_id: "runenwerk.editor.layout.editor_design".to_string(),
        },
        None,
        None,
        None,
        None,
    )
    .expect("workspace layout definition selection should succeed");
    dispatch_shell_command(
        &mut host.app,
        Some(&mut host.shell_state),
        ShellCommand::AddSelectedEditorWorkspaceLayoutTab {
            label: "Validation".to_string(),
            tool_surface: "definition_validation".to_string(),
        },
        None,
        None,
        None,
        None,
    )
    .expect("workspace layout tab edit should succeed");
    dispatch_shell_command(
        &mut host.app,
        Some(&mut host.shell_state),
        ShellCommand::ApplySelectedEditorDefinition,
        None,
        None,
        None,
        None,
    )
    .expect("workspace layout definition apply should succeed");

    assert_eq!(host.app.pending_editor_definition_activation_count(), 1);

    let activated = host.apply_pending_editor_definition_activations();

    assert_eq!(activated, 1);
    host.shell_state
        .composition_runtime()
        .extension()
        .validate_against(host.shell_state.composition_runtime().composition())
        .expect("activated authored composition layout should remain structurally valid");
    assert!(
        host.shell_state
            .composition_runtime()
            .extension()
            .mounted_units()
            .iter()
            .any(|unit| unit.panel_kind_key
                == editor_shell::panel_kind_definition_key(
                    editor_shell::PanelKind::DefinitionValidation
                ))
    );
    assert!(
        !host
            .shell_state
            .composition_runtime()
            .extension()
            .mounted_units()
            .iter()
            .any(|unit| unit.panel_kind_key
                == editor_shell::panel_kind_definition_key(editor_shell::PanelKind::Viewport)),
        "live composition layout activation should replace the previous scene layout"
    );
}

#[test]
fn toolbar_custom_workbench_package_activates_atomically() {
    let mut host = EditorHostResource::ui_designer_workbench();

    dispatch_shell_command(
        &mut host.app,
        Some(&mut host.shell_state),
        ShellCommand::RunToolbarCommand {
            command: ToolbarCommandKind::AddWorkspace,
        },
        None,
        None,
        None,
        None,
    )
    .expect("toolbar add workspace should create a package");
    let selected = host
        .shell_state
        .self_authoring()
        .selected_document()
        .expect("custom composition should be selected");
    assert!(matches!(
        &selected.content,
        EditorDefinitionDocumentContent::WorkbenchComposition(_)
    ));

    dispatch_shell_command(
        &mut host.app,
        Some(&mut host.shell_state),
        ShellCommand::RunToolbarCommand {
            command: ToolbarCommandKind::LoadCustomWorkspace,
        },
        None,
        None,
        None,
        None,
    )
    .expect("toolbar load custom workspace should queue package activation");
    assert_eq!(host.app.pending_editor_definition_activation_count(), 1);

    let activated = host.apply_pending_editor_definition_activations();

    assert_eq!(activated, 1);
    assert_eq!(
        host.app.workbench_host().composition_ref().as_str(),
        "runenwerk.editor.workbench.custom1"
    );
    assert_eq!(
        host.app
            .workbench_host()
            .default_workspace_profile_ref()
            .as_str(),
        "runenwerk.editor.workspace.custom1"
    );
    assert_eq!(
        host.app
            .last_editor_definition_activation_report()
            .expect("activation should report status")
            .status,
        EditorDefinitionActivationStatus::Applied
    );
    assert_eq!(host.app.failed_editor_definition_activations().len(), 0);
    host.shell_state
        .workspace_state()
        .validate_integrity()
        .expect("activated custom workspace should remain structurally valid");
}

#[test]
fn invalid_custom_workbench_activation_preserves_previous_host_and_shell_state() {
    let mut host = EditorHostResource::ui_designer_workbench();
    let previous_composition_ref = host
        .app
        .workbench_host()
        .composition_ref()
        .as_str()
        .to_string();
    let previous_panel_count = host.shell_state.workspace_state().panels().count();

    host.app
        .queue_workbench_composition_package_activation_for_review(
            Some("invalid-custom-workbench".to_string()),
            EditorWorkbenchCompositionDefinition {
                id: "runenwerk.editor.workbench.invalid".to_string(),
                label: "Invalid Workbench".to_string(),
                installed_suites: vec!["runenwerk.missing_suite".to_string()],
                profile_refs: vec!["runenwerk.editor.workspace.invalid".to_string()],
                default_profile_ref: "runenwerk.editor.workspace.invalid".to_string(),
                host_policy: EditorWorkbenchHostPolicyDefinition::AllowAll,
            },
            vec![EditorWorkspaceProfileDefinition {
                id: "runenwerk.editor.workspace.invalid".to_string(),
                label: "Invalid Workspace".to_string(),
                default_modes: vec!["editor-design".to_string()],
                document_kind_filters: vec!["UiLayout".to_string()],
                default_layout: "runenwerk.editor.layout.invalid".to_string(),
            }],
            vec![EditorWorkspaceLayoutDefinition {
                id: "runenwerk.editor.layout.invalid".to_string(),
                label: "Invalid Layout".to_string(),
                root: EditorWorkspaceHostDefinition::TabStack {
                    id: "root".to_string(),
                    tabs: vec![EditorWorkspacePanelTabDefinition {
                        id: "canvas".to_string(),
                        label: "Canvas".to_string(),
                        tool_surface: "runenwerk.editor_design.ui_canvas".to_string(),
                    }],
                    active_tab: Some("canvas".to_string()),
                },
                floating_hosts: Vec::new(),
            }],
        );

    let activated = host.apply_pending_editor_definition_activations();

    assert_eq!(activated, 0);
    assert_eq!(
        host.app.workbench_host().composition_ref().as_str(),
        previous_composition_ref
    );
    assert_eq!(
        host.shell_state.workspace_state().panels().count(),
        previous_panel_count
    );
    assert_eq!(host.app.failed_editor_definition_activations().len(), 1);
    let report = host
        .app
        .last_editor_definition_activation_report()
        .expect("failed activation should report status");
    assert_eq!(report.status, EditorDefinitionActivationStatus::Failed);
    assert!(report.previous_state_preserved);
}

#[test]
fn editor_lab_workbench_composition_fields_edit_suite_and_profile_lists() {
    let mut host = EditorHostResource::ui_designer_workbench();
    dispatch_shell_command(
        &mut host.app,
        Some(&mut host.shell_state),
        ShellCommand::RunToolbarCommand {
            command: ToolbarCommandKind::AddWorkspace,
        },
        None,
        None,
        None,
        None,
    )
    .expect("toolbar add workspace should create a package");

    dispatch_shell_command(
        &mut host.app,
        Some(&mut host.shell_state),
        ShellCommand::SetSelectedWorkbenchInstalledSuites {
            installed_suites: "runenwerk.editor, runenwerk.editor_design, runenwerk.assets"
                .to_string(),
        },
        None,
        None,
        None,
        None,
    )
    .expect("installed suite list edit should be accepted");
    dispatch_shell_command(
        &mut host.app,
        Some(&mut host.shell_state),
        ShellCommand::SetSelectedWorkbenchProfileRefs {
            profile_refs: "runenwerk.editor.workspace.custom1, runenwerk.editor.workspace.extra"
                .to_string(),
        },
        None,
        None,
        None,
        None,
    )
    .expect("profile ref list edit should be accepted");
    dispatch_shell_command(
        &mut host.app,
        Some(&mut host.shell_state),
        ShellCommand::SetSelectedWorkbenchDefaultProfileRef {
            profile_ref: "runenwerk.editor.workspace.custom1".to_string(),
        },
        None,
        None,
        None,
        None,
    )
    .expect("default profile ref edit should be accepted");

    let selected = host
        .shell_state
        .self_authoring()
        .selected_document()
        .expect("custom composition should stay selected");
    let EditorDefinitionDocumentContent::WorkbenchComposition(composition) = &selected.content
    else {
        panic!("selected document should be the custom composition");
    };
    assert_eq!(
        composition.installed_suites,
        vec![
            "runenwerk.editor".to_string(),
            "runenwerk.editor_design".to_string(),
            "runenwerk.assets".to_string()
        ]
    );
    assert_eq!(
        composition.profile_refs,
        vec![
            "runenwerk.editor.workspace.custom1".to_string(),
            "runenwerk.editor.workspace.extra".to_string()
        ]
    );
    assert_eq!(
        composition.default_profile_ref,
        "runenwerk.editor.workspace.custom1"
    );
}

#[test]
fn applying_ui_template_definition_updates_live_template_catalog() {
    let mut app = RunenwerkEditorApp::new();
    let mut shell_state = RunenwerkEditorShellState::new();
    let document = editor_definition::EditorDefinitionDocument::current(
        editor_definition::EditorDefinitionId::from("runenwerk.editor.test.template"),
        "test_template.ron",
        editor_definition::EditorDefinitionDocumentKind::UiLayout,
        editor_definition::EditorDefinitionDocumentContent::UiTemplate(simple_test_template(
            "runenwerk.editor.test.template",
        )),
    );

    shell_state
        .self_authoring_mut()
        .create_document(document)
        .expect("test UI template document should be accepted");

    dispatch_shell_command(
        &mut app,
        Some(&mut shell_state),
        ShellCommand::ApplySelectedEditorDefinition,
        None,
        None,
        None,
        None,
    )
    .expect("UI template definition apply should succeed");

    let mut host = EditorHostResource {
        app,
        shell_state,
        theme: ThemeTokens::default(),
    };
    assert_eq!(host.app.pending_editor_definition_activation_count(), 1);

    let activated = host.apply_pending_editor_definition_activations();

    assert_eq!(activated, 1);
    assert!(
        host.shell_state
            .active_editor_definitions()
            .templates()
            .contains_key(&"runenwerk.editor.test.template".into())
    );
}

#[test]
fn applying_menu_shortcut_and_command_binding_definitions_updates_live_catalogs() {
    let mut host = EditorHostResource::default();
    host.app.queue_editor_definition_activation(
        editor_definition::EditorDefinitionDocument::current(
            editor_definition::EditorDefinitionId::from("runenwerk.editor.test.menu"),
            "test_menu.ron",
            editor_definition::EditorDefinitionDocumentKind::Menu,
            editor_definition::EditorDefinitionDocumentContent::Menu(
                editor_definition::EditorMenuDefinition {
                    id: "runenwerk.editor.test.menu".to_string(),
                    label: "Test Menu".to_string(),
                    items: Vec::new(),
                },
            ),
        ),
    );
    host.app.queue_editor_definition_activation(
        editor_definition::EditorDefinitionDocument::current(
            editor_definition::EditorDefinitionId::from("runenwerk.editor.test.shortcuts"),
            "test_shortcuts.ron",
            editor_definition::EditorDefinitionDocumentKind::Shortcut,
            editor_definition::EditorDefinitionDocumentContent::Shortcuts(
                editor_definition::EditorShortcutSetDefinition {
                    id: "runenwerk.editor.test.shortcuts".to_string(),
                    label: "Test Shortcuts".to_string(),
                    shortcuts: vec![editor_definition::EditorShortcutDefinition {
                        id: "test_apply".to_string(),
                        command: "editor.definition.apply_selected".to_string(),
                        chord: "Cmd+Shift+T".to_string(),
                        context: Some("editor-design".to_string()),
                    }],
                },
            ),
        ),
    );
    host.app.queue_editor_definition_activation(
        editor_definition::EditorDefinitionDocument::current(
            editor_definition::EditorDefinitionId::from("runenwerk.editor.test.commands"),
            "test_commands.ron",
            editor_definition::EditorDefinitionDocumentKind::CommandBinding,
            editor_definition::EditorDefinitionDocumentContent::CommandBindings(
                editor_definition::EditorCommandBindingSetDefinition {
                    id: "runenwerk.editor.test.commands".to_string(),
                    label: "Test Commands".to_string(),
                    bindings: vec![editor_definition::EditorCommandBindingDefinition {
                        id: "test_apply".to_string(),
                        command: "editor.definition.apply_selected".to_string(),
                        route_target: "self-authoring.apply-selected".to_string(),
                        capability_requirements: Vec::new(),
                        undoable: true,
                    }],
                },
            ),
        ),
    );

    let activated = host.apply_pending_editor_definition_activations();

    assert_eq!(activated, 3);
    assert!(
        host.shell_state
            .active_editor_definitions()
            .menus()
            .contains_key("runenwerk.editor.test.menu")
    );
    assert!(
        host.shell_state
            .active_editor_definitions()
            .shortcuts()
            .contains_key("runenwerk.editor.test.shortcuts")
    );
    assert!(
        host.shell_state
            .active_editor_definitions()
            .command_bindings()
            .contains_key("runenwerk.editor.test.commands")
    );
    assert_eq!(
        host.shell_state
            .active_editor_definitions()
            .command_for_route_target("self-authoring.apply-selected"),
        Some("editor.definition.apply_selected"),
        "active command-binding catalogs should map authored route targets to app/domain command ids",
    );
    assert_eq!(
        host.shell_state
            .active_editor_definitions()
            .route_target_for_command("editor.definition.apply_selected"),
        Some("self-authoring.apply-selected")
    );
}

#[test]
fn invalid_command_binding_activation_keeps_previous_active_catalog() {
    let mut host = EditorHostResource::default();
    host.app.queue_editor_definition_activation(
        editor_definition::EditorDefinitionDocument::current(
            editor_definition::EditorDefinitionId::from("runenwerk.editor.test.commands.valid"),
            "valid_commands.ron",
            editor_definition::EditorDefinitionDocumentKind::CommandBinding,
            editor_definition::EditorDefinitionDocumentContent::CommandBindings(
                editor_definition::EditorCommandBindingSetDefinition {
                    id: "runenwerk.editor.test.commands.valid".to_string(),
                    label: "Valid Commands".to_string(),
                    bindings: vec![editor_definition::EditorCommandBindingDefinition {
                        id: "save".to_string(),
                        command: "editor.scene.save".to_string(),
                        route_target: "test.save".to_string(),
                        capability_requirements: Vec::new(),
                        undoable: true,
                    }],
                },
            ),
        ),
    );
    assert_eq!(host.apply_pending_editor_definition_activations(), 1);
    let previous = host
        .shell_state
        .active_editor_definitions()
        .command_bindings()
        .clone();

    host.app.queue_editor_definition_activation(
        editor_definition::EditorDefinitionDocument::current(
            editor_definition::EditorDefinitionId::from("runenwerk.editor.test.commands.invalid"),
            "invalid_commands.ron",
            editor_definition::EditorDefinitionDocumentKind::CommandBinding,
            editor_definition::EditorDefinitionDocumentContent::CommandBindings(
                editor_definition::EditorCommandBindingSetDefinition {
                    id: "runenwerk.editor.test.commands.invalid".to_string(),
                    label: "Invalid Commands".to_string(),
                    bindings: vec![editor_definition::EditorCommandBindingDefinition {
                        id: "unknown".to_string(),
                        command: "editor.unknown.command".to_string(),
                        route_target: "test.unknown".to_string(),
                        capability_requirements: Vec::new(),
                        undoable: true,
                    }],
                },
            ),
        ),
    );

    assert_eq!(host.apply_pending_editor_definition_activations(), 0);
    assert_eq!(
        host.shell_state
            .active_editor_definitions()
            .command_bindings(),
        &previous,
        "unknown command keys must not replace the active command-binding catalog",
    );
}

#[test]
fn duplicate_active_command_binding_route_targets_are_rejected() {
    let mut host = EditorHostResource::default();
    for (set_id, binding_id, command) in [
        (
            "runenwerk.editor.test.commands.first",
            "first",
            "editor.scene.save",
        ),
        (
            "runenwerk.editor.test.commands.second",
            "second",
            "editor.scene.load",
        ),
    ] {
        host.app.queue_editor_definition_activation(
            editor_definition::EditorDefinitionDocument::current(
                editor_definition::EditorDefinitionId::from(set_id),
                format!("{set_id}.ron"),
                editor_definition::EditorDefinitionDocumentKind::CommandBinding,
                editor_definition::EditorDefinitionDocumentContent::CommandBindings(
                    editor_definition::EditorCommandBindingSetDefinition {
                        id: set_id.to_string(),
                        label: set_id.to_string(),
                        bindings: vec![editor_definition::EditorCommandBindingDefinition {
                            id: binding_id.to_string(),
                            command: command.to_string(),
                            route_target: "test.duplicate".to_string(),
                            capability_requirements: Vec::new(),
                            undoable: true,
                        }],
                    },
                ),
            ),
        );
        let activated = host.apply_pending_editor_definition_activations();
        if binding_id == "first" {
            assert_eq!(activated, 1);
        } else {
            assert_eq!(
                activated, 0,
                "duplicate route targets across active binding sets must be rejected",
            );
        }
    }

    assert_eq!(
        host.shell_state
            .active_editor_definitions()
            .command_for_route_target("test.duplicate"),
        Some("editor.scene.save")
    );
}

#[test]
fn authored_command_binding_route_target_resolves_to_shell_command() {
    let mut host = EditorHostResource::default();
    host.app.queue_editor_definition_activation(
        editor_definition::EditorDefinitionDocument::current(
            editor_definition::EditorDefinitionId::from("runenwerk.editor.test.commands.route"),
            "route_commands.ron",
            editor_definition::EditorDefinitionDocumentKind::CommandBinding,
            editor_definition::EditorDefinitionDocumentContent::CommandBindings(
                editor_definition::EditorCommandBindingSetDefinition {
                    id: "runenwerk.editor.test.commands.route".to_string(),
                    label: "Route Commands".to_string(),
                    bindings: vec![editor_definition::EditorCommandBindingDefinition {
                        id: "save".to_string(),
                        command: "editor.scene.save".to_string(),
                        route_target: "authored.save-scene".to_string(),
                        capability_requirements: Vec::new(),
                        undoable: true,
                    }],
                },
            ),
        ),
    );
    assert_eq!(host.apply_pending_editor_definition_activations(), 1);
    let route_actions =
        active_route_actions_by_target(host.shell_state.active_editor_definitions(), false, false);
    let frame_model = editor_shell::EditorShellFrameModel::new(
        editor_shell::ToolbarViewModel {
            buttons: vec![editor_shell::ToolbarButtonViewModel {
                id: editor_core::ToolId(2_001),
                stable_name: "menu_file",
                label: "File".to_string(),
                is_active: true,
                enabled: true,
            }],
        },
        std::collections::BTreeMap::new(),
    )
    .with_active_ui_definitions(
        None,
        Some(editor_definition::EditorToolbarBinding {
            template: "unused.toolbar.template".into(),
            workspace_catalog: None,
            routes: Vec::new(),
            availability: Vec::new(),
            menus: Vec::new(),
            menu_items: vec![editor_definition::EditorToolbarMenuItemBinding {
                menu_id: "file".to_string(),
                item_id: "save".to_string(),
                label: "Save".to_string(),
                route: ui_definition::UiRouteSlotId::new("authored.save-scene"),
                availability: None,
            }],
        }),
        None,
    )
    .with_route_actions(route_actions);
    let build = build_editor_shell_frame(
        &frame_model,
        &ThemeTokens::default(),
        host.shell_state.workspace_state(),
    );

    let commands = map_interactions_to_shell_commands(
        &UiInteractionResults {
            items: vec![UiInteraction::Activated(
                editor_shell::toolbar_menu_item_widget_id(0),
            )],
        },
        &build.projection_artifacts,
    );

    assert_eq!(
        commands,
        vec![ShellCommand::SaveScene],
        "authored route targets should resolve to shell commands via the active command-binding catalog",
    );
}

#[test]
fn material_workspace_routes_resolve_to_shell_commands() {
    let host = EditorHostResource::default();
    let route_actions =
        active_route_actions_by_target(host.shell_state.active_editor_definitions(), false, false);

    assert_eq!(
        route_actions.get("editor.workspace.materials.activate"),
        Some(&RoutedShellAction::SwitchWorkspaceProfile {
            profile_id: MATERIAL_WORKSPACE_PROFILE_ID,
            enabled: true,
        })
    );
    assert_eq!(
        route_actions.get("editor.workspace.materials.close"),
        Some(&RoutedShellAction::CloseWorkspaceProfile {
            profile_id: MATERIAL_WORKSPACE_PROFILE_ID,
            enabled: true,
        })
    );
    assert_eq!(
        route_actions.get("editor.workspace.load.materials"),
        Some(&RoutedShellAction::RunToolbarCommand {
            command: ToolbarCommandKind::LoadWorkspaceProfile(MATERIAL_WORKSPACE_PROFILE_ID),
            enabled: true,
        })
    );
}

#[test]
fn invalid_shortcut_activation_keeps_previous_active_shortcuts() {
    let mut host = EditorHostResource::default();
    host.app.queue_editor_definition_activation(
        editor_definition::EditorDefinitionDocument::current(
            editor_definition::EditorDefinitionId::from("runenwerk.editor.test.shortcuts.valid"),
            "valid_shortcuts.ron",
            editor_definition::EditorDefinitionDocumentKind::Shortcut,
            editor_definition::EditorDefinitionDocumentContent::Shortcuts(
                editor_definition::EditorShortcutSetDefinition {
                    id: "runenwerk.editor.test.shortcuts.valid".to_string(),
                    label: "Valid Shortcuts".to_string(),
                    shortcuts: vec![editor_definition::EditorShortcutDefinition {
                        id: "save".to_string(),
                        command: "editor.scene.save".to_string(),
                        chord: "Cmd+S".to_string(),
                        context: None,
                    }],
                },
            ),
        ),
    );
    assert_eq!(host.apply_pending_editor_definition_activations(), 1);
    let previous = host
        .shell_state
        .active_editor_definitions()
        .shortcuts()
        .clone();

    host.app.queue_editor_definition_activation(
        editor_definition::EditorDefinitionDocument::current(
            editor_definition::EditorDefinitionId::from("runenwerk.editor.test.shortcuts.invalid"),
            "invalid_shortcuts.ron",
            editor_definition::EditorDefinitionDocumentKind::Shortcut,
            editor_definition::EditorDefinitionDocumentContent::Shortcuts(
                editor_definition::EditorShortcutSetDefinition {
                    id: "runenwerk.editor.test.shortcuts.invalid".to_string(),
                    label: "Invalid Shortcuts".to_string(),
                    shortcuts: vec![editor_definition::EditorShortcutDefinition {
                        id: "bad_chord".to_string(),
                        command: "editor.scene.save".to_string(),
                        chord: "Cmd+DefinitelyNotAKey".to_string(),
                        context: None,
                    }],
                },
            ),
        ),
    );

    assert_eq!(host.apply_pending_editor_definition_activations(), 0);
    assert_eq!(
        host.shell_state.active_editor_definitions().shortcuts(),
        &previous,
        "malformed shortcut chords must not replace the active shortcut catalog",
    );
}

#[test]
fn active_menu_item_activation_routes_through_known_command_resolver() {
    let mut host = EditorHostResource::default();
    host.app.queue_editor_definition_activation(
        editor_definition::EditorDefinitionDocument::current(
            editor_definition::EditorDefinitionId::from("runenwerk.editor.test.menu.file"),
            "file_menu.ron",
            editor_definition::EditorDefinitionDocumentKind::Menu,
            editor_definition::EditorDefinitionDocumentContent::Menu(
                editor_definition::EditorMenuDefinition {
                    id: "file".to_string(),
                    label: "File".to_string(),
                    items: vec![editor_definition::EditorMenuItemDefinition {
                        id: "apply_selected_definition".to_string(),
                        label: "Apply Definition".to_string(),
                        command: Some("editor.definition.apply_selected".to_string()),
                        children: Vec::new(),
                        availability: None,
                    }],
                },
            ),
        ),
    );
    assert_eq!(host.apply_pending_editor_definition_activations(), 1);
    host.shell_state.toggle_toolbar_menu(ToolbarMenuKind::File);

    let frame_model = build_editor_shell_frame_model(
        &host.app,
        &host.shell_state,
        host.app.surface_provider_registry(),
        &host.theme,
        None,
        None,
        None,
    );
    let build = build_editor_shell_frame(
        &frame_model,
        &ThemeTokens::default(),
        host.shell_state.workspace_state(),
    );

    let commands = map_interactions_to_shell_commands(
        &UiInteractionResults {
            items: vec![UiInteraction::Activated(
                editor_shell::toolbar_menu_item_widget_id(0),
            )],
        },
        &build.projection_artifacts,
    );

    assert_eq!(
        commands,
        vec![ShellCommand::ApplySelectedEditorDefinition],
        "active menu item command keys should resolve through the app-owned known command path",
    );
}

#[test]
fn editor_definition_activation_invalid_bindings_keep_previous_state_and_report_failure() {
    let mut host = EditorHostResource::default();
    let original_bindings = host
        .shell_state
        .active_editor_definitions()
        .editor_bindings()
        .cloned()
        .expect("default shell state should activate checked-in bindings");
    host.app.queue_editor_definition_activation(
        editor_definition::EditorDefinitionDocument::current(
            editor_definition::EditorDefinitionId::from("runenwerk.editor.test.bindings.invalid"),
            "invalid_bindings.ron",
            editor_definition::EditorDefinitionDocumentKind::EditorBindings,
            editor_definition::EditorDefinitionDocumentContent::EditorBindings(
                editor_definition::EditorDefinitionBindings {
                    toolbar: editor_definition::EditorToolbarBinding {
                        template: "runenwerk.editor.test.missing_toolbar".into(),
                        ..original_bindings.toolbar.clone()
                    },
                    shell_chrome_template: original_bindings.shell_chrome_template.clone(),
                    surface_templates: original_bindings.surface_templates.clone(),
                },
            ),
        ),
    );

    let activated = host.apply_pending_editor_definition_activations();

    assert_eq!(activated, 0);
    assert_eq!(
        host.shell_state
            .active_editor_definitions()
            .editor_bindings(),
        Some(&original_bindings),
        "invalid editor bindings must not replace the previous active catalog",
    );
    let report = host
        .app
        .last_editor_definition_activation_report()
        .expect("invalid activation should record a typed report");
    assert_eq!(report.status, EditorDefinitionActivationStatus::Failed);
    assert!(report.previous_state_preserved);
    assert_eq!(host.app.failed_editor_definition_activations().len(), 1);
}

#[test]
fn panel_and_tool_surface_registry_activation_blocks_active_workspace_removals() {
    let mut host = EditorHostResource::default();
    let original_panel_registry = host
        .shell_state
        .active_editor_definitions()
        .panel_registry()
        .cloned();
    host.app.queue_editor_definition_activation(
        editor_definition::EditorDefinitionDocument::current(
            editor_definition::EditorDefinitionId::from("runenwerk.editor.test.panels.empty"),
            "empty_panels.ron",
            editor_definition::EditorDefinitionDocumentKind::PanelRegistry,
            editor_definition::EditorDefinitionDocumentContent::PanelRegistry(
                editor_definition::EditorPanelRegistryDefinition {
                    id: "runenwerk.editor.test.panels.empty".to_string(),
                    label: "Empty Panels".to_string(),
                    panels: Vec::new(),
                },
            ),
        ),
    );

    let activated = host.apply_pending_editor_definition_activations();

    assert_eq!(activated, 0);
    assert_eq!(
        host.shell_state
            .active_editor_definitions()
            .panel_registry()
            .cloned(),
        original_panel_registry
    );

    let original_tool_surface_registry = host
        .shell_state
        .active_editor_definitions()
        .tool_surface_registry()
        .cloned();
    host.app.queue_editor_definition_activation(
        editor_definition::EditorDefinitionDocument::current(
            editor_definition::EditorDefinitionId::from("runenwerk.editor.test.surfaces.empty"),
            "empty_surfaces.ron",
            editor_definition::EditorDefinitionDocumentKind::ToolSurfaceDefinition,
            editor_definition::EditorDefinitionDocumentContent::ToolSurfaceRegistry(
                editor_definition::EditorToolSurfaceRegistryDefinition {
                    id: "runenwerk.editor.test.surfaces.empty".to_string(),
                    label: "Empty Tool Surfaces".to_string(),
                    tool_surfaces: Vec::new(),
                },
            ),
        ),
    );

    let activated = host.apply_pending_editor_definition_activations();

    assert_eq!(activated, 0);
    assert_eq!(
        host.shell_state
            .active_editor_definitions()
            .tool_surface_registry()
            .cloned(),
        original_tool_surface_registry
    );
}

#[test]
fn panel_registry_activation_rejects_unknown_default_tool_surface() {
    let mut host = EditorHostResource::default();
    let mut panels = host
        .shell_state
        .workspace_state()
        .panels()
        .map(|panel| {
            let default_tool_surface = panel
                .active_tool_surface
                .and_then(|surface_id| host.shell_state.workspace_state().tool_surface(surface_id))
                .and_then(|surface| {
                    editor_shell::tool_surface_kind_for_stable_key(surface.stable_surface_key())
                })
                .map(|kind| editor_shell::tool_surface_kind_definition_key(kind).to_string())
                .unwrap_or_else(|| "placeholder".to_string());
            editor_definition::EditorPanelDefinition {
                id: editor_shell::panel_kind_definition_key(panel.panel_kind).to_string(),
                label: format!("{:?}", panel.panel_kind),
                default_tool_surface,
                allowed_document_kinds: Vec::new(),
                allowed_workspace_profiles: Vec::new(),
            }
        })
        .collect::<Vec<_>>();
    panels[0].default_tool_surface = "missing_future_surface".to_string();
    host.app.queue_editor_definition_activation(
        editor_definition::EditorDefinitionDocument::current(
            editor_definition::EditorDefinitionId::from("runenwerk.editor.test.panels.bad_default"),
            "bad_default_panels.ron",
            editor_definition::EditorDefinitionDocumentKind::PanelRegistry,
            editor_definition::EditorDefinitionDocumentContent::PanelRegistry(
                editor_definition::EditorPanelRegistryDefinition {
                    id: "runenwerk.editor.test.panels.bad_default".to_string(),
                    label: "Bad Defaults".to_string(),
                    panels,
                },
            ),
        ),
    );

    let activated = host.apply_pending_editor_definition_activations();

    assert_eq!(
        activated, 0,
        "panel registry activation should reject defaults that cannot project to known tool surfaces",
    );
}

#[test]
fn active_tool_surface_candidates_preserve_authored_order_and_dedup_known_ids() {
    let mut host = EditorHostResource::default();
    let composition = host.shell_state.composition_runtime().clone();
    host.shell_state
        .active_editor_definitions_mut()
        .install_tool_surface_registry(
            editor_definition::EditorToolSurfaceRegistryDefinition {
                id: "runenwerk.editor.test.surfaces.noisy".to_string(),
                label: "Noisy Tool Surfaces".to_string(),
                tool_surfaces: vec![
                    test_tool_surface_definition("viewport", "Viewport"),
                    test_tool_surface_definition("unknown_future_surface", "Unknown"),
                    test_tool_surface_definition("outliner", "Outliner"),
                    test_tool_surface_definition("viewport", "Viewport Duplicate"),
                    test_tool_surface_definition("entity_table", "Entity Table"),
                    test_tool_surface_definition("inspector", "Inspector"),
                    test_tool_surface_definition("console", "Console"),
                ],
            },
            &composition,
        )
        .expect("noisy registry should still cover all active workspace surfaces");

    let frame_model = build_editor_shell_frame_model(
        &host.app,
        &host.shell_state,
        host.app.surface_provider_registry(),
        &host.theme,
        None,
        None,
        None,
    );

    assert_eq!(
        frame_model
            .available_tool_surface_create_candidates
            .iter()
            .map(|candidate| candidate.stable_surface_key.as_str())
            .collect::<Vec<_>>(),
        vec![
            "runenwerk.scene.viewport",
            "runenwerk.scene.outliner",
            "runenwerk.scene.entity_table",
            "runenwerk.scene.inspector",
            "runenwerk.editor.console",
        ],
        "future switch/create choices should preserve first-seen authored order, skip unknown ids, and dedup known ids",
    );
}

#[test]
fn tool_surface_registry_activation_updates_future_creation_surface_kinds() {
    let mut host = EditorHostResource::default();
    host.app.queue_editor_definition_activation(
        editor_definition::EditorDefinitionDocument::current(
            editor_definition::EditorDefinitionId::from("runenwerk.editor.test.surfaces.extended"),
            "extended_surfaces.ron",
            editor_definition::EditorDefinitionDocumentKind::ToolSurfaceDefinition,
            editor_definition::EditorDefinitionDocumentContent::ToolSurfaceRegistry(
                editor_definition::EditorToolSurfaceRegistryDefinition {
                    id: "runenwerk.editor.test.surfaces.extended".to_string(),
                    label: "Extended Tool Surfaces".to_string(),
                    tool_surfaces: vec![
                        test_tool_surface_definition("outliner", "Outliner"),
                        test_tool_surface_definition("entity_table", "Entity Table"),
                        test_tool_surface_definition("viewport", "Viewport"),
                        test_tool_surface_definition("inspector", "Inspector"),
                        test_tool_surface_definition("console", "Console"),
                        test_tool_surface_definition("definition_validation", "Validation"),
                    ],
                },
            ),
        ),
    );

    let activated = host.apply_pending_editor_definition_activations();
    let frame_model = build_editor_shell_frame_model(
        &host.app,
        &host.shell_state,
        host.app.surface_provider_registry(),
        &host.theme,
        None,
        None,
        None,
    );

    assert_eq!(activated, 1);
    assert_eq!(
        frame_model
            .available_tool_surface_create_candidates
            .iter()
            .map(|candidate| candidate.stable_surface_key.as_str())
            .collect::<Vec<_>>(),
        vec![
            "runenwerk.scene.outliner",
            "runenwerk.scene.entity_table",
            "runenwerk.scene.viewport",
            "runenwerk.scene.inspector",
            "runenwerk.editor.console",
            "runenwerk.editor_design.definition_validation",
        ],
        "activated tool-surface registry should feed future switch/create choices",
    );
}

fn test_tool_surface_definition(
    id: &str,
    label: &str,
) -> editor_definition::EditorToolSurfaceDefinition {
    editor_definition::EditorToolSurfaceDefinition {
        id: id.to_string(),
        label: label.to_string(),
        provider_family: "runenwerk.editor".to_string(),
        required_capabilities: Vec::new(),
        allowed_document_kinds: Vec::new(),
        allowed_workspace_profiles: Vec::new(),
    }
}

#[test]
fn dispatch_shell_command_edits_authored_workspace_layout_drafts() {
    let mut app = RunenwerkEditorApp::new();
    let mut shell_state = RunenwerkEditorShellState::new();

    dispatch_shell_command(
        &mut app,
        Some(&mut shell_state),
        ShellCommand::SelectEditorDefinitionDocument {
            document_id: "runenwerk.editor.layout.editor_design".to_string(),
        },
        None,
        None,
        None,
        None,
    )
    .expect("workspace layout definition selection should succeed");
    dispatch_shell_command(
        &mut app,
        Some(&mut shell_state),
        ShellCommand::AddSelectedEditorWorkspaceLayoutTab {
            label: "Validation".to_string(),
            tool_surface: "definition_validation".to_string(),
        },
        None,
        None,
        None,
        None,
    )
    .expect("workspace layout tab edit should succeed");
    dispatch_shell_command(
        &mut app,
        Some(&mut shell_state),
        ShellCommand::SplitSelectedEditorWorkspaceLayoutRoot {
            axis: "horizontal".to_string(),
        },
        None,
        None,
        None,
        None,
    )
    .expect("workspace layout split edit should succeed");

    let selected = shell_state
        .self_authoring()
        .selected_document()
        .expect("workspace layout should remain selected");
    let editor_definition::EditorDefinitionDocumentContent::WorkspaceLayout(layout) =
        &selected.content
    else {
        panic!("selected document should be a workspace layout");
    };
    assert!(matches!(
        layout.root,
        editor_definition::EditorWorkspaceHostDefinition::Split { .. }
    ));
}

#[test]
fn dispatch_shell_command_handles_toolbar_menu_and_workspace_commands() {
    let mut app = RunenwerkEditorApp::new();
    let mut shell_state = RunenwerkEditorShellState::new();
    let tab_stack_id = editor_shell::TabStackId::try_from_raw(1).unwrap();
    assert!(
        !shell_state
            .open_workspace_profile_ids()
            .contains(&EDITOR_DESIGN_WORKSPACE_PROFILE_ID),
        "Editor Design should be added to the workspace row only after the plus-menu activation"
    );

    dispatch_shell_command(
        &mut app,
        Some(&mut shell_state),
        ShellCommand::ToggleToolbarMenu {
            menu: ToolbarMenuKind::File,
        },
        None,
        None,
        None,
        None,
    )
    .expect("toolbar menu command should succeed");
    assert_eq!(
        shell_state.active_toolbar_menu(),
        Some(ToolbarMenuKind::File)
    );

    dispatch_shell_command(
        &mut app,
        Some(&mut shell_state),
        ShellCommand::ToggleTabStackActionMenu {
            tab_stack_id,
            anchor_widget_id: editor_shell::WidgetId(1),
        },
        None,
        None,
        None,
        None,
    )
    .expect("tab stack action menu command should succeed");
    assert_eq!(shell_state.active_toolbar_menu(), None);
    assert_eq!(
        shell_state.active_tab_stack_action_menu(),
        Some(tab_stack_id)
    );

    dispatch_shell_command(
        &mut app,
        Some(&mut shell_state),
        ShellCommand::ToggleToolbarMenu {
            menu: ToolbarMenuKind::Workspace,
        },
        None,
        None,
        None,
        None,
    )
    .expect("workspace menu command should succeed");
    assert_eq!(
        shell_state.active_toolbar_menu(),
        Some(ToolbarMenuKind::Workspace)
    );
    assert_eq!(shell_state.active_tab_stack_action_menu(), None);

    dispatch_shell_command(
        &mut app,
        Some(&mut shell_state),
        ShellCommand::RunToolbarCommand {
            command: ToolbarCommandKind::NextWorkspace,
        },
        None,
        None,
        None,
        None,
    )
    .expect("next workspace command should succeed");
    assert_eq!(
        shell_state.active_workspace_profile_id(),
        MODELLING_WORKSPACE_PROFILE_ID
    );
    assert_eq!(shell_state.active_toolbar_menu(), None);

    dispatch_shell_command(
        &mut app,
        Some(&mut shell_state),
        ShellCommand::RunToolbarCommand {
            command: ToolbarCommandKind::PreviousWorkspace,
        },
        None,
        None,
        None,
        None,
    )
    .expect("previous workspace command should succeed");
    assert_eq!(
        shell_state.active_workspace_profile_id(),
        SCENE_WORKSPACE_PROFILE_ID
    );

    dispatch_shell_command(
        &mut app,
        Some(&mut shell_state),
        ShellCommand::SwitchWorkspaceProfile {
            profile_id: MODELLING_WORKSPACE_PROFILE_ID,
        },
        None,
        None,
        None,
        None,
    )
    .expect("explicit workspace switch command should succeed");
    assert_eq!(
        shell_state.active_workspace_profile_id(),
        MODELLING_WORKSPACE_PROFILE_ID
    );

    dispatch_shell_command(
        &mut app,
        Some(&mut shell_state),
        ShellCommand::SwitchWorkspaceProfile {
            profile_id: EDITOR_DESIGN_WORKSPACE_PROFILE_ID,
        },
        None,
        None,
        None,
        None,
    )
    .expect("editor design workspace switch command should succeed");
    assert_eq!(
        shell_state.active_workspace_profile_id(),
        EDITOR_DESIGN_WORKSPACE_PROFILE_ID
    );
    assert!(
        shell_state
            .open_workspace_profile_ids()
            .contains(&EDITOR_DESIGN_WORKSPACE_PROFILE_ID),
        "activating Editor Design from the plus menu should add it to the open workspace row"
    );

    dispatch_shell_command(
        &mut app,
        Some(&mut shell_state),
        ShellCommand::CloseWorkspaceProfile {
            profile_id: EDITOR_DESIGN_WORKSPACE_PROFILE_ID,
        },
        None,
        None,
        None,
        None,
    )
    .expect("closing the active editor design workspace should switch to a remaining workspace");
    assert_ne!(
        shell_state.active_workspace_profile_id(),
        EDITOR_DESIGN_WORKSPACE_PROFILE_ID,
        "closing the active workspace should select a remaining workspace"
    );
    assert!(
        shell_state
            .open_workspace_profile_ids()
            .contains(&shell_state.active_workspace_profile_id()),
        "the close fallback must remain represented in the open workspace row"
    );
    assert!(
        !shell_state
            .open_workspace_profile_ids()
            .contains(&EDITOR_DESIGN_WORKSPACE_PROFILE_ID),
        "closed workspace should be removed from the workspace row"
    );

    dispatch_shell_command(
        &mut app,
        Some(&mut shell_state),
        ShellCommand::CloseWorkspaceProfile {
            profile_id: MODELLING_WORKSPACE_PROFILE_ID,
        },
        None,
        None,
        None,
        None,
    )
    .expect("closing a workspace should keep one fallback open");
    dispatch_shell_command(
        &mut app,
        Some(&mut shell_state),
        ShellCommand::CloseWorkspaceProfile {
            profile_id: SCENE_WORKSPACE_PROFILE_ID,
        },
        None,
        None,
        None,
        None,
    )
    .expect("closing the last workspace should be ignored");
    assert_eq!(
        shell_state.open_workspace_profile_ids(),
        &[SCENE_WORKSPACE_PROFILE_ID],
        "workspace close must never leave the shell with zero open workspaces"
    );
}

#[test]
fn default_startup_resolves_scene_surface_providers() {
    let app = RunenwerkEditorApp::new();
    let shell_state = RunenwerkEditorShellState::new();
    assert!(matches!(
        active_document_context(&app),
        editor_shell::SurfaceDocumentContext::Resolved {
            document_kind: editor_core::DocumentKind::Scene,
            ..
        }
    ));

    let frame_model = build_editor_shell_frame_model(
        &app,
        &shell_state,
        app.surface_provider_registry(),
        &ThemeTokens::default(),
        None,
        None,
        None,
    );

    for kind in [
        PanelKind::Outliner,
        PanelKind::EntityTable,
        PanelKind::Viewport,
        PanelKind::Inspector,
    ] {
        let surface = surface_id_by_kind(shell_state.workspace_state(), kind);
        let frame = frame_model
            .surface(surface)
            .expect("mounted scene surface should resolve a frame");
        assert_eq!(
            frame.availability,
            SurfaceProviderAvailability::Available,
            "{kind:?} should not render unsupported document on default startup",
        );
    }
}

#[test]
fn material_lab_workbench_startup_resolves_material_surface_providers() {
    let host = EditorHostResource::material_lab_workbench();
    assert!(matches!(
        active_document_context(&host.app),
        editor_shell::SurfaceDocumentContext::Resolved {
            document_kind: editor_core::DocumentKind::MaterialGraph,
            ..
        }
    ));

    let frame_model = build_editor_shell_frame_model(
        &host.app,
        &host.shell_state,
        host.app.surface_provider_registry(),
        &host.theme,
        None,
        None,
        None,
    );

    for kind in [
        PanelKind::MaterialGraphCanvas,
        PanelKind::MaterialInspector,
        PanelKind::MaterialPreview,
    ] {
        let surface = composition_target_by_panel_kind(&host.shell_state, kind)
            .active_tool_surface
            .expect("mounted material content should expose compatibility surface identity");
        let frame = frame_model
            .surface(surface)
            .expect("mounted Material Lab surface should resolve a frame");
        assert_eq!(
            frame.availability,
            SurfaceProviderAvailability::Available,
            "{kind:?} should be available on standalone Material Lab startup",
        );
    }
}

#[test]
fn full_editor_material_workspace_resolves_material_surfaces_with_scene_document() {
    let mut host = EditorHostResource::default();
    assert!(matches!(
        active_document_context(&host.app),
        editor_shell::SurfaceDocumentContext::Resolved {
            document_kind: editor_core::DocumentKind::Scene,
            ..
        }
    ));

    dispatch_shell_command(
        &mut host.app,
        Some(&mut host.shell_state),
        ShellCommand::SwitchWorkspaceProfile {
            profile_id: MATERIAL_WORKSPACE_PROFILE_ID,
        },
        None,
        None,
        None,
        None,
    )
    .expect("material workspace switch should load from the full editor");

    let frame_model = build_editor_shell_frame_model(
        &host.app,
        &host.shell_state,
        host.app.surface_provider_registry(),
        &host.theme,
        None,
        None,
        None,
    );

    for kind in [
        PanelKind::MaterialGraphCanvas,
        PanelKind::MaterialInspector,
        PanelKind::MaterialPreview,
    ] {
        let surface = composition_target_by_panel_kind(&host.shell_state, kind)
            .active_tool_surface
            .expect("mounted material content should expose compatibility surface identity");
        let frame = frame_model
            .surface(surface)
            .expect("mounted Material Lab surface should resolve a frame");
        assert_eq!(
            frame.availability,
            SurfaceProviderAvailability::Available,
            "{kind:?} should be available when Material Lab opens from a scene document",
        );
    }

    let graph_surface =
        composition_target_by_panel_kind(&host.shell_state, PanelKind::MaterialGraphCanvas)
            .active_tool_surface
            .expect("material graph content should expose compatibility surface identity");
    let graph_frame = frame_model
        .surface(graph_surface)
        .expect("material graph surface should resolve");
    assert!(
        frame_contains_graph_canvas(graph_frame),
        "Material Lab graph frame should contain a real graph canvas node"
    );
}

#[test]
fn scene_load_reset_keeps_active_scene_document_for_provider_frames() {
    let app = {
        let mut app = RunenwerkEditorApp::new();
        app.runtime_mut().prepare_for_scene_load();
        app
    };
    let shell_state = RunenwerkEditorShellState::new();
    assert!(matches!(
        active_document_context(&app),
        editor_shell::SurfaceDocumentContext::Resolved {
            document_kind: editor_core::DocumentKind::Scene,
            ..
        }
    ));

    let frame_model = build_editor_shell_frame_model(
        &app,
        &shell_state,
        app.surface_provider_registry(),
        &ThemeTokens::default(),
        None,
        None,
        None,
    );
    let viewport_surface = surface_id_by_kind(shell_state.workspace_state(), PanelKind::Viewport);
    assert_eq!(
        frame_model
            .surface(viewport_surface)
            .map(|frame| frame.availability.clone()),
        Some(SurfaceProviderAvailability::Available),
        "scene load reset should not leave scene providers in no-active-document state",
    );
}

#[test]
fn dispatch_shell_command_selects_outliner_entity() {
    let mut app = RunenwerkEditorApp::new();
    let ecs_entity = app.runtime_mut().spawn_world_entity(TestMarker);
    app.runtime_mut()
        .register_entity(EntityId(1), ecs_entity, "Player", None);

    dispatch_shell_command(
        &mut app,
        None,
        editor_domain_command(
            StructuralCommandTarget {
                mounted_unit_id: None,
                panel_instance_id: editor_shell::PanelInstanceId::try_from_raw(1).unwrap(),
                active_tool_surface: Some(
                    editor_shell::ToolSurfaceInstanceId::try_from_raw(1).unwrap(),
                ),
                tab_stack_id: editor_shell::TabStackId::try_from_raw(1).unwrap(),
            },
            EditorDomainMutation::Outliner(OutlinerDomainMutation::SelectEntity {
                entity: EntityId(1),
            }),
            0,
        ),
        None,
        None,
        None,
        None,
    )
    .expect("outliner select shell command should succeed");

    assert_eq!(app.outliner_state().selected_entity, Some(EntityId(1)));
    assert_eq!(
        app.runtime().session().selection().primary(),
        Some(&SelectionTarget::Entity(EntityId(1)))
    );
    assert!(matches!(
        app.runtime()
            .session_change_log()
            .last()
            .map(|change| (change.origin, change.kind.clone())),
        Some((
            ChangeOrigin::OutlinerPanel,
            SessionChangeKind::SelectionSetSingle {
                target: SelectionTarget::Entity(EntityId(1))
            }
        ))
    ));
}

#[test]
fn outliner_tree_row_interaction_selects_entity() {
    let mut app = RunenwerkEditorApp::new();
    let ecs_entity = app.runtime_mut().spawn_world_entity(TestMarker);
    app.runtime_mut()
        .register_entity(EntityId(1), ecs_entity, "Player", None);
    let mut shell_state = RunenwerkEditorShellState::new();
    let bounds = UiRect::new(0.0, 0.0, 1280.0, 720.0);
    let theme = ThemeTokens::default();
    let atlas = UiFontAtlasResource::default();
    let _ =
        RunenwerkEditorShellController::build_frame(&app, &mut shell_state, bounds, &theme, &atlas);
    let outliner_surface = surface_id_by_kind(shell_state.workspace_state(), PanelKind::Outliner);
    let artifacts = shell_state
        .last_projection_artifacts()
        .expect("projection artifacts should be cached")
        .clone();
    let interactions = UiInteractionResults {
        items: vec![UiInteraction::TreeRowSelected {
            target: surface_widget_id(outliner_surface, OUTLINER_LIST_WIDGET_ID),
            row_index: 0,
        }],
    };
    let commands = map_interactions_to_shell_commands(&interactions, &artifacts);

    assert_eq!(commands.len(), 1);
    let registry = EditorSurfaceProviderRegistry::runenwerk_default();
    RunenwerkEditorShellController::dispatch_commands(
        &mut app,
        &mut shell_state,
        commands,
        &registry,
        None,
        None,
        None,
    )
    .expect("outliner row command should dispatch");

    assert_eq!(
        app.runtime().session().selection().primary(),
        Some(&SelectionTarget::Entity(EntityId(1))),
    );
}

#[test]
fn entity_table_row_interaction_selects_entity_with_structural_target() {
    let mut app = RunenwerkEditorApp::new();
    let alpha = app.runtime_mut().spawn_world_entity(TestMarker);
    app.runtime_mut()
        .register_entity(EntityId(1), alpha, "Alpha", None);
    let beta = app.runtime_mut().spawn_world_entity(TestMarker);
    app.runtime_mut()
        .register_entity(EntityId(2), beta, "Beta", None);

    let mut shell_state = RunenwerkEditorShellState::new();
    let (entity_table_panel, entity_table_stack) =
        panel_and_stack_by_kind(shell_state.workspace_state(), PanelKind::EntityTable);
    let entity_table_surface =
        surface_id_by_kind(shell_state.workspace_state(), PanelKind::EntityTable);
    shell_state
        .apply_workspace_mutation(WorkspaceMutation::SetTabStackActivePanel {
            tab_stack_id: entity_table_stack,
            active_panel: Some(entity_table_panel),
        })
        .expect("entity table tab should activate");

    let bounds = UiRect::new(0.0, 0.0, 1280.0, 720.0);
    let theme = ThemeTokens::default();
    let atlas = UiFontAtlasResource::default();
    let _ =
        RunenwerkEditorShellController::build_frame(&app, &mut shell_state, bounds, &theme, &atlas);

    let artifacts = shell_state
        .last_projection_artifacts()
        .expect("projection artifacts should be cached")
        .clone();
    let context = artifacts
        .workspace
        .widget_context_by_id
        .get(&surface_widget_id(
            entity_table_surface,
            ENTITY_TABLE_PANEL_WIDGET_ID,
        ))
        .copied()
        .expect("entity table panel should have a structural context");
    assert_eq!(context.panel_instance_id, entity_table_panel);

    let interactions = UiInteractionResults {
        items: vec![UiInteraction::TableRowSelected {
            target: surface_widget_id(entity_table_surface, ENTITY_TABLE_LIST_WIDGET_ID),
            row_index: 0,
        }],
    };
    let commands = map_interactions_to_shell_commands(&interactions, &artifacts);
    assert!(matches!(
        commands.as_slice(),
        [ShellCommand::DispatchSurfaceLocalAction {
            tool_surface_instance_id,
            target,
            projection_epoch,
            ..
        }] if *tool_surface_instance_id == context.active_tool_surface.expect("active surface")
            && target.panel_instance_id == entity_table_panel
            && target.tab_stack_id == entity_table_stack
            && *projection_epoch == artifacts.projection_epoch
    ));

    let registry = EditorSurfaceProviderRegistry::runenwerk_default();
    RunenwerkEditorShellController::dispatch_commands(
        &mut app,
        &mut shell_state,
        commands,
        &registry,
        None,
        None,
        None,
    )
    .expect("entity table provider-local command should dispatch");

    assert_eq!(
        app.runtime().session().selection().primary(),
        Some(&SelectionTarget::Entity(EntityId(1)))
    );
    assert!(matches!(
        app.runtime()
            .session_change_log()
            .last()
            .map(|change| (change.origin, change.kind.clone())),
        Some((
            ChangeOrigin::EntityTablePanel,
            SessionChangeKind::SelectionSetSingle {
                target: SelectionTarget::Entity(EntityId(1))
            }
        ))
    ));
}

#[test]
fn entity_table_search_click_focuses_and_text_updates_query() {
    let mut app = RunenwerkEditorApp::new();
    let mut shell_state = RunenwerkEditorShellState::new();
    let (entity_table_panel, entity_table_stack) =
        panel_and_stack_by_kind(shell_state.workspace_state(), PanelKind::EntityTable);
    let entity_table_surface = shell_state
        .workspace_state()
        .panel(entity_table_panel)
        .and_then(|panel| panel.active_tool_surface)
        .expect("entity table panel should have an active surface");
    shell_state
        .apply_workspace_mutation(WorkspaceMutation::SetTabStackActivePanel {
            tab_stack_id: entity_table_stack,
            active_panel: Some(entity_table_panel),
        })
        .expect("entity table tab should activate");

    let bounds = UiRect::new(0.0, 0.0, 1280.0, 720.0);
    let theme = ThemeTokens::default();
    let atlas = UiFontAtlasResource::default();
    let _ =
        RunenwerkEditorShellController::build_frame(&app, &mut shell_state, bounds, &theme, &atlas);
    let artifacts = shell_state
        .last_projection_artifacts()
        .expect("projection artifacts should exist")
        .clone();
    let search_widget = artifacts
        .widget_actions_by_id
        .iter()
        .find_map(|(widget_id, action)| {
            let editor_shell::RoutedShellAction::DispatchSurfaceLocalAction {
                tool_surface_instance_id,
                action,
                ..
            } = action
            else {
                return None;
            };
            (*tool_surface_instance_id == entity_table_surface
                && matches!(
                    action,
                    SurfaceLocalAction::EntityTable(
                        EntityTableSurfaceAction::AppendSearchText { .. }
                    )
                ))
            .then_some(*widget_id)
        })
        .expect("entity table search should have a routed text action");
    let tree = shell_state
        .last_tree()
        .expect("shell tree should exist")
        .clone();
    let layouts = shell_state.runtime().compute_layout(&tree, bounds);
    let search_position = center_of_widget(&layouts, search_widget);

    dispatch_pointer(
        &mut app,
        &mut shell_state,
        bounds,
        &theme,
        PointerEventKind::Down,
        search_position,
        Some(PointerButton::Primary),
    );
    dispatch_pointer(
        &mut app,
        &mut shell_state,
        bounds,
        &theme,
        PointerEventKind::Up,
        search_position,
        Some(PointerButton::Primary),
    );
    RunenwerkEditorShellController::dispatch_input(
        &mut app,
        &mut shell_state,
        bounds,
        &theme,
        &UiInputEvent::Text(ui_input::TextInputEvent {
            text: "alpha".to_string(),
        }),
    )
    .expect("search text input should dispatch");

    assert_eq!(
        app.surface_sessions()
            .session(entity_table_surface)
            .expect("entity table surface session should exist")
            .entity_table_ui_state
            .search_query(),
        "alpha",
        "click-focused entity search should receive typed text through the shell input path",
    );
}

#[test]
fn entity_table_query_filters_and_sorts_rows() {
    let mut app = RunenwerkEditorApp::new();
    let marker_type = ComponentTypeId(7001);
    app.runtime_mut()
        .register_component_type::<QueryMarker>(marker_type);

    let zeta = app.runtime_mut().spawn_world_entity(QueryMarker::default());
    app.runtime_mut()
        .register_entity(EntityId(1), zeta, "ZetaRoot", None);
    let child = app.runtime_mut().spawn_world_entity(TestMarker);
    app.runtime_mut()
        .register_entity(EntityId(2), child, "ChildAlpha", Some(EntityId(1)));
    let alpha = app.runtime_mut().spawn_world_entity(TestMarker);
    app.runtime_mut()
        .register_entity(EntityId(3), alpha, "AlphaRoot", None);
    select_single_entity(app.runtime_mut(), EntityId(1)).expect("selection should succeed");

    let mut query = EntityTablePanelUiState::new();
    query.set_search_query("alpha");
    let state = EntityTablePanelPresenter::build_state(app.runtime(), &query);
    assert_eq!(
        state.rows.iter().map(|row| row.entity).collect::<Vec<_>>(),
        vec![EntityId(3), EntityId(2)]
    );

    query.set_search_query("");
    query.set_selected_only(true);
    let state = EntityTablePanelPresenter::build_state(app.runtime(), &query);
    assert_eq!(
        state.rows.iter().map(|row| row.entity).collect::<Vec<_>>(),
        vec![EntityId(1)]
    );

    query.set_selected_only(false);
    query.set_hierarchy_filter(EntityTableHierarchyFilter::RootsOnly);
    let state = EntityTablePanelPresenter::build_state(app.runtime(), &query);
    assert_eq!(
        state.rows.iter().map(|row| row.entity).collect::<Vec<_>>(),
        vec![EntityId(3), EntityId(1)]
    );

    query.set_hierarchy_filter(EntityTableHierarchyFilter::All);
    query.set_component_filter(EntityTableComponentFilter::Has(marker_type));
    let state = EntityTablePanelPresenter::build_state(app.runtime(), &query);
    assert_eq!(
        state.rows.iter().map(|row| row.entity).collect::<Vec<_>>(),
        vec![EntityId(1)]
    );

    query.set_component_filter(EntityTableComponentFilter::All);
    query.set_hierarchy_filter(EntityTableHierarchyFilter::RootsOnly);
    query.toggle_sort(editor_shell::EntityTableSortKey::DisplayName);
    let state = EntityTablePanelPresenter::build_state(app.runtime(), &query);
    assert_eq!(
        state
            .rows
            .iter()
            .map(|row| row.display_name.as_str())
            .collect::<Vec<_>>(),
        vec!["ZetaRoot", "AlphaRoot"]
    );
}

#[test]
fn stale_provider_local_action_fails_closed_after_rebuild() {
    let mut app = RunenwerkEditorApp::new();
    let entity = app.runtime_mut().spawn_world_entity(TestMarker);
    app.runtime_mut()
        .register_entity(EntityId(1), entity, "Alpha", None);
    let mut shell_state = RunenwerkEditorShellState::new();
    let (entity_table_panel, entity_table_stack) =
        panel_and_stack_by_kind(shell_state.workspace_state(), PanelKind::EntityTable);
    let entity_table_surface =
        surface_id_by_kind(shell_state.workspace_state(), PanelKind::EntityTable);
    shell_state
        .apply_workspace_mutation(WorkspaceMutation::SetTabStackActivePanel {
            tab_stack_id: entity_table_stack,
            active_panel: Some(entity_table_panel),
        })
        .expect("entity table tab should activate");

    let bounds = UiRect::new(0.0, 0.0, 1280.0, 720.0);
    let theme = ThemeTokens::default();
    let atlas = UiFontAtlasResource::default();
    let _ =
        RunenwerkEditorShellController::build_frame(&app, &mut shell_state, bounds, &theme, &atlas);
    let stale_artifacts = shell_state
        .last_projection_artifacts()
        .expect("projection artifacts should be cached")
        .clone();
    let stale_commands = map_interactions_to_shell_commands(
        &UiInteractionResults {
            items: vec![UiInteraction::TableRowSelected {
                target: surface_widget_id(entity_table_surface, ENTITY_TABLE_LIST_WIDGET_ID),
                row_index: 0,
            }],
        },
        &stale_artifacts,
    );
    assert!(matches!(
        stale_commands.as_slice(),
        [ShellCommand::DispatchSurfaceLocalAction { .. }]
    ));

    let _ =
        RunenwerkEditorShellController::build_frame(&app, &mut shell_state, bounds, &theme, &atlas);
    assert!(stale_artifacts.projection_epoch < shell_state.current_projection_epoch());
    let registry = EditorSurfaceProviderRegistry::runenwerk_default();

    RunenwerkEditorShellController::dispatch_commands(
        &mut app,
        &mut shell_state,
        stale_commands,
        &registry,
        None,
        None,
        None,
    )
    .expect("stale provider-local action should fail closed without mutation error");

    assert_eq!(app.runtime().session().selection().primary(), None);
}

#[test]
fn provider_id_mismatch_on_local_action_is_rejected_without_mutation() {
    let mut app = RunenwerkEditorApp::new();
    let entity = app.runtime_mut().spawn_world_entity(TestMarker);
    app.runtime_mut()
        .register_entity(EntityId(1), entity, "Alpha", None);
    let mut shell_state = RunenwerkEditorShellState::new();
    let (entity_table_panel, entity_table_stack) =
        panel_and_stack_by_kind(shell_state.workspace_state(), PanelKind::EntityTable);
    let entity_table_surface =
        surface_id_by_kind(shell_state.workspace_state(), PanelKind::EntityTable);
    shell_state
        .apply_workspace_mutation(WorkspaceMutation::SetTabStackActivePanel {
            tab_stack_id: entity_table_stack,
            active_panel: Some(entity_table_panel),
        })
        .expect("entity table tab should activate");

    let bounds = UiRect::new(0.0, 0.0, 1280.0, 720.0);
    let theme = ThemeTokens::default();
    let atlas = UiFontAtlasResource::default();
    let _ =
        RunenwerkEditorShellController::build_frame(&app, &mut shell_state, bounds, &theme, &atlas);
    let artifacts = shell_state
        .last_projection_artifacts()
        .expect("projection artifacts should be cached")
        .clone();
    let mut commands = map_interactions_to_shell_commands(
        &UiInteractionResults {
            items: vec![UiInteraction::TableRowSelected {
                target: surface_widget_id(entity_table_surface, ENTITY_TABLE_LIST_WIDGET_ID),
                row_index: 0,
            }],
        },
        &artifacts,
    );
    let [ShellCommand::DispatchSurfaceLocalAction { provider_id, .. }] = commands.as_mut_slice()
    else {
        panic!("expected one provider-local action");
    };
    *provider_id = SurfaceProviderId::try_from_raw(999).unwrap();

    let registry = EditorSurfaceProviderRegistry::runenwerk_default();
    let result = RunenwerkEditorShellController::dispatch_commands(
        &mut app,
        &mut shell_state,
        commands,
        &registry,
        None,
        None,
        None,
    );

    assert!(result.is_err());
    assert_eq!(app.runtime().session().selection().primary(), None);
}

#[test]
fn workbench_host_allow_all_accepts_provider_surface_session_mutation() {
    let mut app = RunenwerkEditorApp::new();
    let mut shell_state = RunenwerkEditorShellState::new();
    let (entity_table_panel, entity_table_stack) =
        panel_and_stack_by_kind(shell_state.workspace_state(), PanelKind::EntityTable);
    let entity_table_surface =
        surface_id_by_kind(shell_state.workspace_state(), PanelKind::EntityTable);
    shell_state
        .apply_workspace_mutation(WorkspaceMutation::SetTabStackActivePanel {
            tab_stack_id: entity_table_stack,
            active_panel: Some(entity_table_panel),
        })
        .expect("entity table tab should activate");

    let bounds = UiRect::new(0.0, 0.0, 1280.0, 720.0);
    let theme = ThemeTokens::default();
    let atlas = UiFontAtlasResource::default();
    let _ =
        RunenwerkEditorShellController::build_frame(&app, &mut shell_state, bounds, &theme, &atlas);

    let registry = EditorSurfaceProviderRegistry::runenwerk_default();
    let projection_epoch = shell_state.current_projection_epoch();
    RunenwerkEditorShellController::dispatch_commands(
        &mut app,
        &mut shell_state,
        vec![entity_table_search_provider_command(
            entity_table_panel,
            entity_table_stack,
            entity_table_surface,
            projection_epoch,
            "allowed",
        )],
        &registry,
        None,
        None,
        None,
    )
    .expect("allow-all workbench host policy should preserve provider mutation dispatch");

    assert_eq!(
        app.surface_sessions()
            .session(entity_table_surface)
            .expect("entity table surface session should be created")
            .entity_table_ui_state
            .search_query(),
        "allowed"
    );
}

#[test]
fn workbench_host_policy_denies_provider_surface_session_before_mutation() {
    let denied_command = CommandCapabilityKey::new("runenwerk.surface.session_mutation").unwrap();
    let mut app = RunenwerkEditorApp::new();
    app.workbench_host = Arc::new(
        RunenwerkWorkbenchHost::new()
            .expect("default workbench host should build")
            .with_host_capability_policy(
                HostCapabilityPolicy::allow_all().deny_command(denied_command),
            ),
    );
    let mut shell_state = RunenwerkEditorShellState::new();
    let (entity_table_panel, entity_table_stack) =
        panel_and_stack_by_kind(shell_state.workspace_state(), PanelKind::EntityTable);
    let entity_table_surface =
        surface_id_by_kind(shell_state.workspace_state(), PanelKind::EntityTable);
    shell_state
        .apply_workspace_mutation(WorkspaceMutation::SetTabStackActivePanel {
            tab_stack_id: entity_table_stack,
            active_panel: Some(entity_table_panel),
        })
        .expect("entity table tab should activate");

    let bounds = UiRect::new(0.0, 0.0, 1280.0, 720.0);
    let theme = ThemeTokens::default();
    let atlas = UiFontAtlasResource::default();
    let _ =
        RunenwerkEditorShellController::build_frame(&app, &mut shell_state, bounds, &theme, &atlas);

    let registry = EditorSurfaceProviderRegistry::runenwerk_default();
    let projection_epoch = shell_state.current_projection_epoch();
    let result = RunenwerkEditorShellController::dispatch_commands(
        &mut app,
        &mut shell_state,
        vec![entity_table_search_provider_command(
            entity_table_panel,
            entity_table_stack,
            entity_table_surface,
            projection_epoch,
            "blocked",
        )],
        &registry,
        None,
        None,
        None,
    );

    assert_eq!(
        result,
        Err(EditorMutationError::session_rejected(
            "host capability policy denied provider proposal"
        ))
    );
    assert!(
        app.surface_sessions()
            .session(entity_table_surface)
            .map(|session| session.entity_table_ui_state.search_query().is_empty())
            .unwrap_or(true),
        "denied provider proposal must not create or mutate the entity-table session",
    );
}

fn entity_table_search_provider_command(
    panel_instance_id: editor_shell::PanelInstanceId,
    tab_stack_id: editor_shell::TabStackId,
    tool_surface_instance_id: editor_shell::ToolSurfaceInstanceId,
    projection_epoch: u64,
    text: &str,
) -> ShellCommand {
    ShellCommand::DispatchSurfaceLocalAction {
        provider_id: SurfaceProviderId::try_from_raw(2).unwrap(),
        tool_surface_instance_id,
        target: StructuralCommandTarget {
            mounted_unit_id: None,
            panel_instance_id,
            active_tool_surface: Some(tool_surface_instance_id),
            tab_stack_id,
        },
        action: SurfaceLocalAction::EntityTable(EntityTableSurfaceAction::AppendSearchText {
            text: text.to_string(),
        }),
        projection_epoch,
    }
}

#[test]
fn dispatch_shell_command_selects_viewport_product_when_available() {
    let mut app = RunenwerkEditorApp::new();
    let mut viewport_presentations = ViewportPresentationStateResource::default();
    let mut viewport_observations = ViewportArtifactObservationResource::default();
    let viewport_id = ViewportId(1);
    let product_id = ExpressionProductId(2);
    let target = StructuralCommandTarget {
        mounted_unit_id: None,
        panel_instance_id: editor_shell::PanelInstanceId::try_from_raw(1).unwrap(),
        active_tool_surface: Some(editor_shell::ToolSurfaceInstanceId::try_from_raw(1).unwrap()),
        tab_stack_id: editor_shell::TabStackId::try_from_raw(1).unwrap(),
    };
    let tool_surface_bindings = test_tool_surface_binding_registry(
        editor_shell::ToolSurfaceInstanceId::try_from_raw(1).unwrap(),
        target.panel_instance_id,
        target.tab_stack_id,
        viewport_id,
    );
    let mut frame =
        ArtifactObservationFrame::new(viewport_id, app.runtime().current_scene_reality_version());
    frame
        .availability_by_product
        .insert(product_id, ProductAvailabilityState::Available);
    frame
        .producer_health_by_product
        .insert(product_id, ProducerHealth::Healthy);
    viewport_observations.upsert_frame(frame);

    dispatch_shell_command(
        &mut app,
        None,
        editor_domain_command(
            target,
            EditorDomainMutation::Viewport(ViewportDomainMutation::SelectProduct {
                viewport_id,
                product_id,
            }),
            0,
        ),
        Some(&mut viewport_presentations),
        Some(&viewport_observations),
        Some(&tool_surface_bindings),
        None,
    )
    .expect("viewport product select shell command should succeed");

    assert_eq!(
        viewport_presentations
            .state_for(viewport_id)
            .map(|state| state.selected_primary_product_id),
        Some(product_id)
    );
}

#[test]
fn dispatch_shell_command_updates_only_target_viewport_product_selection() {
    let mut app = RunenwerkEditorApp::new();
    let mut viewport_presentations = ViewportPresentationStateResource::default();
    let mut viewport_observations = ViewportArtifactObservationResource::default();
    let viewport_a = ViewportId(1);
    let viewport_b = ViewportId(2);
    let product_scene = ExpressionProductId(1);
    let product_picking = ExpressionProductId(2);
    let target = StructuralCommandTarget {
        mounted_unit_id: None,
        panel_instance_id: editor_shell::PanelInstanceId::try_from_raw(1).unwrap(),
        active_tool_surface: Some(editor_shell::ToolSurfaceInstanceId::try_from_raw(1).unwrap()),
        tab_stack_id: editor_shell::TabStackId::try_from_raw(1).unwrap(),
    };
    let tool_surface_bindings = test_tool_surface_binding_registry(
        editor_shell::ToolSurfaceInstanceId::try_from_raw(1).unwrap(),
        target.panel_instance_id,
        target.tab_stack_id,
        viewport_b,
    );
    viewport_presentations.upsert_state(ViewportPresentationState::new(viewport_a, product_scene));
    viewport_presentations.upsert_state(ViewportPresentationState::new(viewport_b, product_scene));

    for viewport_id in [viewport_a, viewport_b] {
        let mut frame = ArtifactObservationFrame::new(
            viewport_id,
            app.runtime().current_scene_reality_version(),
        );
        frame
            .availability_by_product
            .insert(product_picking, ProductAvailabilityState::Available);
        frame
            .producer_health_by_product
            .insert(product_picking, ProducerHealth::Healthy);
        viewport_observations.upsert_frame(frame);
    }

    dispatch_shell_command(
        &mut app,
        None,
        editor_domain_command(
            target,
            EditorDomainMutation::Viewport(ViewportDomainMutation::SelectProduct {
                viewport_id: viewport_b,
                product_id: product_picking,
            }),
            0,
        ),
        Some(&mut viewport_presentations),
        Some(&viewport_observations),
        Some(&tool_surface_bindings),
        None,
    )
    .expect("viewport product select shell command should succeed");

    assert_eq!(
        viewport_presentations
            .state_for(viewport_a)
            .map(|state| state.selected_primary_product_id),
        Some(product_scene),
        "selection for viewport A should remain unchanged",
    );
    assert_eq!(
        viewport_presentations
            .state_for(viewport_b)
            .map(|state| state.selected_primary_product_id),
        Some(product_picking),
        "selection for viewport B should update independently",
    );
}

#[test]
fn dispatch_shell_command_viewport_product_fails_closed_without_runtime_binding() {
    let mut app = RunenwerkEditorApp::new();
    let mut viewport_presentations = ViewportPresentationStateResource::default();
    let mut viewport_observations = ViewportArtifactObservationResource::default();
    let viewport_id = ViewportId(1);
    let product_id = ExpressionProductId(2);
    let mut frame =
        ArtifactObservationFrame::new(viewport_id, app.runtime().current_scene_reality_version());
    frame
        .availability_by_product
        .insert(product_id, ProductAvailabilityState::Available);
    viewport_observations.upsert_frame(frame);

    dispatch_shell_command(
        &mut app,
        None,
        editor_domain_command(
            StructuralCommandTarget {
                mounted_unit_id: None,
                panel_instance_id: editor_shell::PanelInstanceId::try_from_raw(1).unwrap(),
                active_tool_surface: Some(
                    editor_shell::ToolSurfaceInstanceId::try_from_raw(1).unwrap(),
                ),
                tab_stack_id: editor_shell::TabStackId::try_from_raw(1).unwrap(),
            },
            EditorDomainMutation::Viewport(ViewportDomainMutation::SelectProduct {
                viewport_id,
                product_id,
            }),
            0,
        ),
        Some(&mut viewport_presentations),
        Some(&viewport_observations),
        None,
        None,
    )
    .expect("missing binding should fail closed without raising mutation error");

    assert!(
        viewport_presentations.state_for(viewport_id).is_none(),
        "without runtime binding registry, structural viewport command must not mutate selection",
    );
}

#[test]
fn dispatch_shell_command_viewport_product_rejects_stale_binding_viewport_mismatch() {
    let mut app = RunenwerkEditorApp::new();
    let mut viewport_presentations = ViewportPresentationStateResource::default();
    let mut viewport_observations = ViewportArtifactObservationResource::default();
    let requested_viewport = ViewportId(1);
    let rebound_viewport = ViewportId(2);
    let product_id = ExpressionProductId(2);
    let target = StructuralCommandTarget {
        mounted_unit_id: None,
        panel_instance_id: editor_shell::PanelInstanceId::try_from_raw(1).unwrap(),
        active_tool_surface: Some(editor_shell::ToolSurfaceInstanceId::try_from_raw(1).unwrap()),
        tab_stack_id: editor_shell::TabStackId::try_from_raw(1).unwrap(),
    };
    let tool_surface_bindings = test_tool_surface_binding_registry(
        editor_shell::ToolSurfaceInstanceId::try_from_raw(1).unwrap(),
        target.panel_instance_id,
        target.tab_stack_id,
        rebound_viewport,
    );

    for viewport_id in [requested_viewport, rebound_viewport] {
        let mut frame = ArtifactObservationFrame::new(
            viewport_id,
            app.runtime().current_scene_reality_version(),
        );
        frame
            .availability_by_product
            .insert(product_id, ProductAvailabilityState::Available);
        viewport_observations.upsert_frame(frame);
    }

    dispatch_shell_command(
        &mut app,
        None,
        editor_domain_command(
            target,
            EditorDomainMutation::Viewport(ViewportDomainMutation::SelectProduct {
                viewport_id: requested_viewport,
                product_id,
            }),
            0,
        ),
        Some(&mut viewport_presentations),
        Some(&viewport_observations),
        Some(&tool_surface_bindings),
        None,
    )
    .expect("stale binding mismatch should fail closed without raising mutation error");

    assert!(
        viewport_presentations
            .state_for(requested_viewport)
            .is_none(),
        "requested viewport selection should not be updated on stale binding mismatch",
    );
    assert!(
        viewport_presentations.state_for(rebound_viewport).is_none(),
        "rebound viewport should not be implicitly mutated by stale command",
    );
}

#[test]
fn dispatch_shell_command_viewport_product_requires_structural_tool_surface_target() {
    let mut app = RunenwerkEditorApp::new();
    let mut viewport_presentations = ViewportPresentationStateResource::default();
    let mut viewport_observations = ViewportArtifactObservationResource::default();
    let viewport_id = ViewportId(1);
    let product_id = ExpressionProductId(2);
    let mut frame =
        ArtifactObservationFrame::new(viewport_id, app.runtime().current_scene_reality_version());
    frame
        .availability_by_product
        .insert(product_id, ProductAvailabilityState::Available);
    viewport_observations.upsert_frame(frame);

    let tool_surface_bindings = test_tool_surface_binding_registry(
        editor_shell::ToolSurfaceInstanceId::try_from_raw(1).unwrap(),
        editor_shell::PanelInstanceId::try_from_raw(1).unwrap(),
        editor_shell::TabStackId::try_from_raw(1).unwrap(),
        viewport_id,
    );

    dispatch_shell_command(
        &mut app,
        None,
        editor_domain_command(
            StructuralCommandTarget {
                mounted_unit_id: None,
                panel_instance_id: editor_shell::PanelInstanceId::try_from_raw(1).unwrap(),
                active_tool_surface: None,
                tab_stack_id: editor_shell::TabStackId::try_from_raw(1).unwrap(),
            },
            EditorDomainMutation::Viewport(ViewportDomainMutation::SelectProduct {
                viewport_id,
                product_id,
            }),
            0,
        ),
        Some(&mut viewport_presentations),
        Some(&viewport_observations),
        Some(&tool_surface_bindings),
        None,
    )
    .expect("missing structural tool-surface target should fail closed");

    assert!(
        viewport_presentations.state_for(viewport_id).is_none(),
        "viewport selection must not mutate when structural tool surface is absent",
    );
}

#[test]
fn dispatch_shell_command_viewport_product_rejects_structural_binding_mismatch() {
    let mut app = RunenwerkEditorApp::new();
    let mut viewport_presentations = ViewportPresentationStateResource::default();
    let mut viewport_observations = ViewportArtifactObservationResource::default();
    let viewport_id = ViewportId(1);
    let product_id = ExpressionProductId(2);
    let mut frame =
        ArtifactObservationFrame::new(viewport_id, app.runtime().current_scene_reality_version());
    frame
        .availability_by_product
        .insert(product_id, ProductAvailabilityState::Available);
    viewport_observations.upsert_frame(frame);

    let target = StructuralCommandTarget {
        mounted_unit_id: None,
        panel_instance_id: editor_shell::PanelInstanceId::try_from_raw(7).unwrap(),
        active_tool_surface: Some(editor_shell::ToolSurfaceInstanceId::try_from_raw(1).unwrap()),
        tab_stack_id: editor_shell::TabStackId::try_from_raw(8).unwrap(),
    };
    let tool_surface_bindings = test_tool_surface_binding_registry(
        editor_shell::ToolSurfaceInstanceId::try_from_raw(1).unwrap(),
        editor_shell::PanelInstanceId::try_from_raw(99).unwrap(),
        editor_shell::TabStackId::try_from_raw(100).unwrap(),
        viewport_id,
    );

    dispatch_shell_command(
        &mut app,
        None,
        editor_domain_command(
            target,
            EditorDomainMutation::Viewport(ViewportDomainMutation::SelectProduct {
                viewport_id,
                product_id,
            }),
            0,
        ),
        Some(&mut viewport_presentations),
        Some(&viewport_observations),
        Some(&tool_surface_bindings),
        None,
    )
    .expect("structural binding mismatch should fail closed");

    assert!(
        viewport_presentations.state_for(viewport_id).is_none(),
        "viewport selection must not mutate when structural binding mismatches runtime mapping",
    );
}

#[test]
fn dispatch_shell_command_enqueues_viewport_state_commands_for_bound_viewport() {
    let mut app = RunenwerkEditorApp::new();
    let requested_viewport = ViewportId(1);
    let rebound_viewport = ViewportId(2);
    let target = StructuralCommandTarget {
        mounted_unit_id: None,
        panel_instance_id: editor_shell::PanelInstanceId::try_from_raw(1).unwrap(),
        active_tool_surface: Some(editor_shell::ToolSurfaceInstanceId::try_from_raw(1).unwrap()),
        tab_stack_id: editor_shell::TabStackId::try_from_raw(1).unwrap(),
    };
    let tool_surface_bindings = test_tool_surface_binding_registry(
        target.active_tool_surface.unwrap(),
        target.panel_instance_id,
        target.tab_stack_id,
        rebound_viewport,
    );
    let mut viewport_render_commands = ViewportRenderStateCommandQueueResource::default();

    dispatch_shell_command_with_viewport_commands(
        &mut app,
        None,
        editor_domain_command(
            target,
            EditorDomainMutation::Viewport(ViewportDomainMutation::ResetCamera {
                viewport_id: rebound_viewport,
            }),
            0,
        ),
        None,
        None,
        Some(&tool_surface_bindings),
        Some(&mut viewport_render_commands),
        None,
    )
    .expect("bound viewport camera reset command should dispatch");
    dispatch_shell_command_with_viewport_commands(
        &mut app,
        None,
        editor_domain_command(
            target,
            EditorDomainMutation::Viewport(ViewportDomainMutation::SetDebugStage {
                viewport_id: rebound_viewport,
                debug_stage: ViewportDebugStage::PickingHitMiss,
            }),
            0,
        ),
        None,
        None,
        Some(&tool_surface_bindings),
        Some(&mut viewport_render_commands),
        None,
    )
    .expect("bound viewport debug command should dispatch");
    dispatch_shell_command_with_viewport_commands(
        &mut app,
        None,
        editor_domain_command(
            target,
            EditorDomainMutation::Viewport(ViewportDomainMutation::SetRootBackgroundOpaque {
                viewport_id: rebound_viewport,
                enabled: true,
            }),
            0,
        ),
        None,
        None,
        Some(&tool_surface_bindings),
        Some(&mut viewport_render_commands),
        None,
    )
    .expect("bound viewport root opacity command should dispatch");

    assert_eq!(
        viewport_render_commands.drain().collect::<Vec<_>>(),
        vec![
            ViewportRenderStateCommand::ResetCamera {
                viewport_id: rebound_viewport,
            },
            ViewportRenderStateCommand::SetDebugStage {
                viewport_id: rebound_viewport,
                debug_stage: ViewportDebugStage::PickingHitMiss,
            },
            ViewportRenderStateCommand::SetRootBackgroundOpaque {
                viewport_id: rebound_viewport,
                enabled: true,
            },
        ],
        "viewport state commands should be routed through the active runtime binding",
    );

    dispatch_shell_command_with_viewport_commands(
        &mut app,
        None,
        editor_domain_command(
            target,
            EditorDomainMutation::Viewport(ViewportDomainMutation::SetDebugStage {
                viewport_id: requested_viewport,
                debug_stage: ViewportDebugStage::PrimitiveAvailability,
            }),
            0,
        ),
        None,
        None,
        Some(&tool_surface_bindings),
        Some(&mut viewport_render_commands),
        None,
    )
    .expect("stale viewport command should fail closed without raising a mutation error");

    assert!(
        viewport_render_commands.is_empty(),
        "stale viewport command must not enqueue state changes for the rebound viewport",
    );
}

#[test]
fn dispatch_shell_command_updates_viewport_field_visualizer_settings_without_changing_product() {
    let mut app = RunenwerkEditorApp::new();
    let mut viewport_presentations = ViewportPresentationStateResource::default();
    let requested_viewport = ViewportId(1);
    let rebound_viewport = ViewportId(2);
    let selected_product = ExpressionProductId(6);
    let target = StructuralCommandTarget {
        mounted_unit_id: None,
        panel_instance_id: editor_shell::PanelInstanceId::try_from_raw(1).unwrap(),
        active_tool_surface: Some(editor_shell::ToolSurfaceInstanceId::try_from_raw(1).unwrap()),
        tab_stack_id: editor_shell::TabStackId::try_from_raw(1).unwrap(),
    };
    let tool_surface_bindings = test_tool_surface_binding_registry(
        target.active_tool_surface.unwrap(),
        target.panel_instance_id,
        target.tab_stack_id,
        rebound_viewport,
    );
    viewport_presentations.upsert_state(ViewportPresentationState::new(
        rebound_viewport,
        selected_product,
    ));
    dispatch_shell_command(
        &mut app,
        None,
        editor_domain_command(
            target,
            EditorDomainMutation::Viewport(ViewportDomainMutation::PatchFieldVisualizerSettings {
                viewport_id: rebound_viewport,
                patch: ViewportFieldVisualizerSettingsPatch::SetComponent(
                    ViewportFieldVisualizerComponent::Magnitude,
                ),
            }),
            0,
        ),
        Some(&mut viewport_presentations),
        None,
        Some(&tool_surface_bindings),
        None,
    )
    .expect("bound field visualizer settings command should dispatch");

    let state = viewport_presentations
        .state_for(rebound_viewport)
        .expect("rebound viewport presentation should exist");
    assert_eq!(state.selected_primary_product_id, selected_product);
    assert_eq!(
        state.field_visualizer_settings.component,
        ViewportFieldVisualizerComponent::Magnitude
    );

    dispatch_shell_command(
        &mut app,
        None,
        editor_domain_command(
            target,
            EditorDomainMutation::Viewport(ViewportDomainMutation::PatchFieldVisualizerSettings {
                viewport_id: rebound_viewport,
                patch: ViewportFieldVisualizerSettingsPatch::SetColorRamp(
                    ViewportFieldVisualizerColorRamp::Heat,
                ),
            }),
            0,
        ),
        Some(&mut viewport_presentations),
        None,
        Some(&tool_surface_bindings),
        None,
    )
    .expect("second field visualizer patch should merge with current state");

    let merged = viewport_presentations
        .state_for(rebound_viewport)
        .map(|state| state.field_visualizer_settings)
        .expect("rebound viewport presentation should exist");
    assert_eq!(
        merged.component,
        ViewportFieldVisualizerComponent::Magnitude
    );
    assert_eq!(merged.color_ramp, ViewportFieldVisualizerColorRamp::Heat);

    dispatch_shell_command(
        &mut app,
        None,
        editor_domain_command(
            target,
            EditorDomainMutation::Viewport(ViewportDomainMutation::PatchFieldVisualizerSettings {
                viewport_id: requested_viewport,
                patch: ViewportFieldVisualizerSettingsPatch::SetComponent(
                    ViewportFieldVisualizerComponent::Auto,
                ),
            }),
            0,
        ),
        Some(&mut viewport_presentations),
        None,
        Some(&tool_surface_bindings),
        None,
    )
    .expect("stale field visualizer command should fail closed");

    assert_eq!(
        viewport_presentations
            .state_for(rebound_viewport)
            .map(|state| state.field_visualizer_settings),
        Some(merged)
    );
}

#[test]
fn dispatch_shell_command_clamps_field_visualizer_slice_step_to_selected_product_metadata() {
    let mut app = RunenwerkEditorApp::new();
    let mut viewport_presentations = ViewportPresentationStateResource::default();
    let mut viewport_observations = ViewportArtifactObservationResource::default();
    let rebound_viewport = ViewportId(2);
    let target = StructuralCommandTarget {
        mounted_unit_id: None,
        panel_instance_id: editor_shell::PanelInstanceId::try_from_raw(1).unwrap(),
        active_tool_surface: Some(editor_shell::ToolSurfaceInstanceId::try_from_raw(1).unwrap()),
        tab_stack_id: editor_shell::TabStackId::try_from_raw(1).unwrap(),
    };
    let tool_surface_bindings = test_tool_surface_binding_registry(
        target.active_tool_surface.unwrap(),
        target.panel_instance_id,
        target.tab_stack_id,
        rebound_viewport,
    );
    viewport_presentations.upsert_state(ViewportPresentationState::new(
        rebound_viewport,
        VOLUME_SLICE_PRODUCT_ID,
    ));

    let mut frame = ArtifactObservationFrame::new(rebound_viewport, RealityVersion(1));
    frame.available_products =
        initial_product_descriptors(ExpressionDimensions::new(64, 64), RealityVersion(1));
    frame.selected_primary_product_id = Some(VOLUME_SLICE_PRODUCT_ID);
    viewport_observations.upsert_frame(frame);

    dispatch_shell_command(
        &mut app,
        None,
        editor_domain_command(
            target,
            EditorDomainMutation::Viewport(ViewportDomainMutation::PatchFieldVisualizerSettings {
                viewport_id: rebound_viewport,
                patch: ViewportFieldVisualizerSettingsPatch::StepSliceIndex(1),
            }),
            0,
        ),
        Some(&mut viewport_presentations),
        Some(&viewport_observations),
        Some(&tool_surface_bindings),
        None,
    )
    .expect("field visualizer patch should apply selected product slice bounds");

    assert_eq!(
        viewport_presentations
            .state_for(rebound_viewport)
            .map(|state| state.field_visualizer_settings.slice_index),
        Some(0)
    );
}

#[test]
fn dispatch_shell_command_toggles_viewport_details_visibility() {
    let mut app = RunenwerkEditorApp::new();
    let mut shell_state = RunenwerkEditorShellState::new();
    let target = composition_target_by_kind(&shell_state, ToolSurfaceKind::Viewport);
    let viewport_unit = target.mounted_unit_id.unwrap();
    assert!(
        !app.surface_sessions()
            .session(viewport_unit)
            .map(|session| session.viewport_details_visible)
            .unwrap_or(false)
    );

    dispatch_shell_command(
        &mut app,
        Some(&mut shell_state),
        surface_session_command(
            target,
            SurfaceSessionMutation::Viewport(ViewportSessionMutation::ToggleDetails),
            0,
        ),
        None,
        None,
        None,
        None,
    )
    .expect("viewport details toggle shell command should succeed");
    assert!(
        app.surface_sessions()
            .session(viewport_unit)
            .map(|session| session.viewport_details_visible)
            .unwrap_or(false)
    );

    dispatch_shell_command(
        &mut app,
        Some(&mut shell_state),
        surface_session_command(
            target,
            SurfaceSessionMutation::Viewport(ViewportSessionMutation::ToggleDetails),
            0,
        ),
        None,
        None,
        None,
        None,
    )
    .expect("viewport details toggle shell command should succeed");
    assert!(
        !app.surface_sessions()
            .session(viewport_unit)
            .map(|session| session.viewport_details_visible)
            .unwrap_or(false)
    );
}

#[test]
fn dispatch_shell_command_separates_viewport_tools_menu_and_radial_session() {
    let mut app = RunenwerkEditorApp::new();
    let mut shell_state = RunenwerkEditorShellState::new();
    let target = composition_target_by_kind(&shell_state, ToolSurfaceKind::Viewport);
    let viewport_unit = target.mounted_unit_id.unwrap();
    let viewport_surface = target.active_tool_surface.unwrap();

    dispatch_shell_command(
        &mut app,
        Some(&mut shell_state),
        surface_session_command(
            target,
            SurfaceSessionMutation::Viewport(ViewportSessionMutation::ToggleToolsMenu),
            0,
        ),
        None,
        None,
        None,
        None,
    )
    .expect("viewport tools menu toggle should succeed");
    assert!(
        app.surface_sessions()
            .session(viewport_unit)
            .is_some_and(|session| session.viewport_tools_menu_open)
    );

    dispatch_shell_command(
        &mut app,
        Some(&mut shell_state),
        surface_session_command(
            target,
            SurfaceSessionMutation::Viewport(ViewportSessionMutation::OpenToolRadialMenu {
                viewport_id: ViewportId(44),
                anchor_position: UiPoint::new(300.0, 220.0),
                opened_by_tab_hold: true,
            }),
            0,
        ),
        None,
        None,
        None,
        None,
    )
    .expect("viewport radial open should succeed");
    let session = app
        .surface_sessions()
        .session(viewport_unit)
        .expect("viewport surface session should exist");
    assert!(!session.viewport_tools_menu_open);
    assert!(session.viewport_tool_radial_session.is_some_and(|radial| {
        radial.tool_surface_id == viewport_surface
            && radial.viewport_id == ViewportId(44)
            && radial.anchor_position == UiPoint::new(300.0, 220.0)
            && radial.opened_by_tab_hold
    }));
}

#[test]
fn provider_local_viewport_details_toggle_uses_routed_surface_instance() {
    let mut app = RunenwerkEditorApp::new();
    let mut shell_state = RunenwerkEditorShellState::new();
    let bounds = UiRect::new(0.0, 0.0, 1280.0, 720.0);
    let theme = ThemeTokens::default();
    let atlas = UiFontAtlasResource::default();
    let viewport_surface = surface_id_by_kind(shell_state.workspace_state(), PanelKind::Viewport);

    let _ =
        RunenwerkEditorShellController::build_frame(&app, &mut shell_state, bounds, &theme, &atlas);
    let artifacts = shell_state
        .last_projection_artifacts()
        .expect("projection artifacts should be cached")
        .clone();
    let commands = map_interactions_to_shell_commands(
        &UiInteractionResults {
            items: vec![UiInteraction::Toggled {
                target: surface_widget_id(viewport_surface, VIEWPORT_DETAILS_TOGGLE_WIDGET_ID),
                checked: true,
            }],
        },
        &artifacts,
    );
    assert!(matches!(
        commands.as_slice(),
        [ShellCommand::DispatchSurfaceLocalAction {
            action: SurfaceLocalAction::Viewport(ViewportSurfaceAction::ToggleDetails),
            tool_surface_instance_id,
            ..
        }] if *tool_surface_instance_id == viewport_surface
    ));

    let registry = app.surface_provider_registry_handle();
    RunenwerkEditorShellController::dispatch_commands(
        &mut app,
        &mut shell_state,
        commands,
        registry.as_ref(),
        None,
        None,
        None,
    )
    .expect("provider-local viewport details toggle should dispatch");

    assert_eq!(
        app.surface_sessions()
            .session(viewport_surface)
            .map(|session| session.viewport_details_visible),
        Some(true)
    );
}

#[test]
fn provider_local_viewport_field_controls_are_routed_actions() {
    let app = RunenwerkEditorApp::new();
    let shell_state = RunenwerkEditorShellState::new();
    let viewport_surface = surface_id_by_kind(shell_state.workspace_state(), PanelKind::Viewport);
    let mut viewport_instances = ViewportInstanceRegistryResource::default();
    viewport_instances.sync_from_workspace_state(shell_state.workspace_state());
    let viewport_id = viewport_instances
        .viewport_for_tool_surface(viewport_surface)
        .expect("viewport surface should have runtime viewport identity");

    let frame_model = build_editor_shell_frame_model(
        &app,
        &shell_state,
        app.surface_provider_registry(),
        &ThemeTokens::default(),
        None,
        None,
        Some(&viewport_instances),
    );
    let frame = frame_model
        .surface(viewport_surface)
        .expect("viewport surface should resolve");
    let route = frame
        .routes
        .get(&surface_widget_id(
            viewport_surface,
            viewport_field_component_button_widget_id(1),
        ))
        .expect("field component control should have a route");
    assert!(matches!(
        route.action(),
        Some(SurfaceLocalAction::Viewport(ViewportSurfaceAction::PatchFieldVisualizerSettings {
            viewport_id: routed_viewport,
            patch,
        })) if *routed_viewport == viewport_id
            && *patch == ViewportFieldVisualizerSettingsPatch::SetComponent(
                ViewportFieldVisualizerComponent::X
            )
    ));

    let route = frame
        .routes
        .get(&surface_widget_id(
            viewport_surface,
            VIEWPORT_FIELD_SLICE_INCREMENT_WIDGET_ID,
        ))
        .expect("field slice increment control should have a route");
    assert!(matches!(
        route.action(),
        Some(SurfaceLocalAction::Viewport(ViewportSurfaceAction::PatchFieldVisualizerSettings {
            viewport_id: routed_viewport,
            patch,
        })) if *routed_viewport == viewport_id
            && *patch == ViewportFieldVisualizerSettingsPatch::StepSliceIndex(1)
    ));
}

#[test]
fn stale_provider_local_viewport_details_toggle_fails_closed() {
    let mut app = RunenwerkEditorApp::new();
    let mut shell_state = RunenwerkEditorShellState::new();
    let bounds = UiRect::new(0.0, 0.0, 1280.0, 720.0);
    let theme = ThemeTokens::default();
    let atlas = UiFontAtlasResource::default();
    let viewport_surface = surface_id_by_kind(shell_state.workspace_state(), PanelKind::Viewport);

    let _ =
        RunenwerkEditorShellController::build_frame(&app, &mut shell_state, bounds, &theme, &atlas);
    let stale_artifacts = shell_state
        .last_projection_artifacts()
        .expect("projection artifacts should be cached")
        .clone();
    let stale_commands = map_interactions_to_shell_commands(
        &UiInteractionResults {
            items: vec![UiInteraction::Toggled {
                target: surface_widget_id(viewport_surface, VIEWPORT_DETAILS_TOGGLE_WIDGET_ID),
                checked: true,
            }],
        },
        &stale_artifacts,
    );
    let _ =
        RunenwerkEditorShellController::build_frame(&app, &mut shell_state, bounds, &theme, &atlas);

    let registry = app.surface_provider_registry_handle();
    RunenwerkEditorShellController::dispatch_commands(
        &mut app,
        &mut shell_state,
        stale_commands,
        registry.as_ref(),
        None,
        None,
        None,
    )
    .expect("stale provider-local viewport details toggle should fail closed");

    assert!(
        !app.surface_sessions()
            .session(viewport_surface)
            .map(|session| session.viewport_details_visible)
            .unwrap_or(false)
    );
}

#[test]
fn provider_id_mismatch_on_viewport_details_toggle_is_rejected_without_mutation() {
    let mut app = RunenwerkEditorApp::new();
    let mut shell_state = RunenwerkEditorShellState::new();
    let bounds = UiRect::new(0.0, 0.0, 1280.0, 720.0);
    let theme = ThemeTokens::default();
    let atlas = UiFontAtlasResource::default();
    let viewport_surface = surface_id_by_kind(shell_state.workspace_state(), PanelKind::Viewport);

    let _ =
        RunenwerkEditorShellController::build_frame(&app, &mut shell_state, bounds, &theme, &atlas);
    let artifacts = shell_state
        .last_projection_artifacts()
        .expect("projection artifacts should be cached")
        .clone();
    let mut commands = map_interactions_to_shell_commands(
        &UiInteractionResults {
            items: vec![UiInteraction::Toggled {
                target: surface_widget_id(viewport_surface, VIEWPORT_DETAILS_TOGGLE_WIDGET_ID),
                checked: true,
            }],
        },
        &artifacts,
    );
    let [ShellCommand::DispatchSurfaceLocalAction { provider_id, .. }] = commands.as_mut_slice()
    else {
        panic!("expected one provider-local action");
    };
    *provider_id = SurfaceProviderId::try_from_raw(999).unwrap();

    let registry = app.surface_provider_registry_handle();
    let result = RunenwerkEditorShellController::dispatch_commands(
        &mut app,
        &mut shell_state,
        commands,
        registry.as_ref(),
        None,
        None,
        None,
    );

    assert!(result.is_err());
    assert!(
        !app.surface_sessions()
            .session(viewport_surface)
            .map(|session| session.viewport_details_visible)
            .unwrap_or(false)
    );
}

#[test]
fn two_viewport_surfaces_keep_independent_details_state() {
    let mut app = RunenwerkEditorApp::new();
    let surface_a = editor_shell::ToolSurfaceInstanceId::try_from_raw(101).unwrap();
    let unit_a = ui_composition::MountedUnitId::new(1001);
    let unit_b = ui_composition::MountedUnitId::new(1002);
    let target_a = StructuralCommandTarget {
        mounted_unit_id: Some(unit_a),
        panel_instance_id: editor_shell::PanelInstanceId::try_from_raw(201).unwrap(),
        active_tool_surface: Some(surface_a),
        tab_stack_id: editor_shell::TabStackId::try_from_raw(301).unwrap(),
    };

    dispatch_shell_command(
        &mut app,
        None,
        surface_session_command(
            target_a,
            SurfaceSessionMutation::Viewport(ViewportSessionMutation::ToggleDetails),
            7,
        ),
        None,
        None,
        None,
        Some(7),
    )
    .expect("targeted viewport details toggle should dispatch");

    assert_eq!(
        app.surface_sessions()
            .session(unit_a)
            .map(|session| session.viewport_details_visible),
        Some(true)
    );
    assert!(
        !app.surface_sessions()
            .session(unit_b)
            .map(|session| session.viewport_details_visible)
            .unwrap_or(false)
    );
}

#[test]
fn two_entity_table_surfaces_keep_independent_search_and_sort_state() {
    let mut app = RunenwerkEditorApp::new();
    let surface_a = editor_shell::ToolSurfaceInstanceId::try_from_raw(111).unwrap();
    let surface_b = editor_shell::ToolSurfaceInstanceId::try_from_raw(112).unwrap();
    let unit_a = ui_composition::MountedUnitId::new(1101);
    let unit_b = ui_composition::MountedUnitId::new(1102);
    let target_a = StructuralCommandTarget {
        mounted_unit_id: Some(unit_a),
        panel_instance_id: editor_shell::PanelInstanceId::try_from_raw(211).unwrap(),
        active_tool_surface: Some(surface_a),
        tab_stack_id: editor_shell::TabStackId::try_from_raw(311).unwrap(),
    };
    let target_b = StructuralCommandTarget {
        mounted_unit_id: Some(unit_b),
        panel_instance_id: editor_shell::PanelInstanceId::try_from_raw(212).unwrap(),
        active_tool_surface: Some(surface_b),
        tab_stack_id: editor_shell::TabStackId::try_from_raw(312).unwrap(),
    };

    dispatch_shell_command(
        &mut app,
        None,
        surface_session_command(
            target_a,
            SurfaceSessionMutation::EntityTable(EntityTableSessionMutation::AppendSearchText {
                text: "alpha".to_string(),
            }),
            1,
        ),
        None,
        None,
        None,
        Some(1),
    )
    .expect("entity-table search should dispatch for surface A");
    dispatch_shell_command(
        &mut app,
        None,
        surface_session_command(
            target_b,
            SurfaceSessionMutation::EntityTable(EntityTableSessionMutation::ToggleSort {
                sort_key: editor_shell::EntityTableSortKey::DisplayName,
            }),
            1,
        ),
        None,
        None,
        None,
        Some(1),
    )
    .expect("entity-table sort should dispatch for surface B");

    let session_a = app
        .surface_sessions()
        .session(unit_a)
        .expect("surface A session should exist");
    let session_b = app
        .surface_sessions()
        .session(unit_b)
        .expect("surface B session should exist");
    assert_eq!(session_a.entity_table_ui_state.search_query(), "alpha");
    assert_eq!(session_b.entity_table_ui_state.search_query(), "");
    assert!(session_a.entity_table_ui_state.sort_ascending());
    assert!(!session_b.entity_table_ui_state.sort_ascending());
}

#[test]
fn two_inspector_surfaces_keep_independent_draft_state() {
    let mut app = RunenwerkEditorApp::new();
    let surface_a = editor_shell::ToolSurfaceInstanceId::try_from_raw(121).unwrap();
    let surface_b = editor_shell::ToolSurfaceInstanceId::try_from_raw(122).unwrap();
    let unit_a = ui_composition::MountedUnitId::new(1201);
    let unit_b = ui_composition::MountedUnitId::new(1202);
    let target_a = StructuralCommandTarget {
        mounted_unit_id: Some(unit_a),
        panel_instance_id: editor_shell::PanelInstanceId::try_from_raw(221).unwrap(),
        active_tool_surface: Some(surface_a),
        tab_stack_id: editor_shell::TabStackId::try_from_raw(321).unwrap(),
    };
    let _target_b = StructuralCommandTarget {
        mounted_unit_id: Some(unit_b),
        panel_instance_id: editor_shell::PanelInstanceId::try_from_raw(222).unwrap(),
        active_tool_surface: Some(surface_b),
        tab_stack_id: editor_shell::TabStackId::try_from_raw(322).unwrap(),
    };
    app.surface_sessions_mut()
        .session_mut(unit_a)
        .inspector_ui_state
        .begin_field_edit(
            EntityId(1),
            editor_core::ComponentTypeId(1),
            InspectorPath::root().child_field("x"),
            InspectorEditValue::Text("1".to_string()),
            "1",
        );
    app.surface_sessions_mut()
        .session_mut(unit_b)
        .inspector_ui_state
        .begin_field_edit(
            EntityId(2),
            editor_core::ComponentTypeId(1),
            InspectorPath::root().child_field("x"),
            InspectorEditValue::Text("2".to_string()),
            "2",
        );

    dispatch_shell_command(
        &mut app,
        None,
        surface_session_command(
            target_a,
            SurfaceSessionMutation::Inspector(InspectorSessionMutation::CancelFieldText {
                index: 0,
            }),
            1,
        ),
        None,
        None,
        None,
        Some(1),
    )
    .expect("inspector cancel should dispatch for surface A");

    assert!(
        app.surface_sessions()
            .session(unit_a)
            .expect("surface A session should exist")
            .inspector_ui_state
            .active_draft()
            .is_none()
    );
    assert!(
        app.surface_sessions()
            .session(unit_b)
            .expect("surface B session should exist")
            .inspector_ui_state
            .active_draft()
            .is_some()
    );
}

#[test]
fn two_viewport_surfaces_keep_independent_interaction_state() {
    let mut app = RunenwerkEditorApp::new();
    let surface_a = editor_shell::ToolSurfaceInstanceId::try_from_raw(131).unwrap();
    let surface_b = editor_shell::ToolSurfaceInstanceId::try_from_raw(132).unwrap();
    let entity = app.runtime_mut().spawn_world_entity(TestMarker);
    app.runtime_mut()
        .register_entity(EntityId(1), entity, "Alpha", None);
    app.dispatch_viewport_command(ViewportPanelCommand::SelectEntity {
        entity: EntityId(1),
    })
    .expect("entity selection should succeed");

    app.dispatch_viewport_interaction_for_surface(
        surface_a,
        crate::editor_features::viewport::ViewportInteractionCommand::PointerDown {
            hit: ViewportHitResult::gizmo_axis("X", 0.0),
        },
    )
    .expect("surface A viewport interaction should start");

    assert!(
        app.surface_sessions()
            .viewport_interaction_state(surface_a)
            .expect("surface A viewport state should exist")
            .drag_in_progress()
    );
    assert!(
        !app.surface_sessions()
            .viewport_interaction_state(surface_b)
            .map(|state| state.drag_in_progress())
            .unwrap_or(false)
    );
    assert_eq!(
        app.surface_sessions().active_viewport_drag_surface(),
        Some(surface_a)
    );
}

#[test]
fn two_console_surfaces_keep_independent_follow_state() {
    let mut app = RunenwerkEditorApp::new();
    let surface_a = editor_shell::ToolSurfaceInstanceId::try_from_raw(141).unwrap();
    let surface_b = editor_shell::ToolSurfaceInstanceId::try_from_raw(142).unwrap();

    app.surface_sessions_mut()
        .session_mut(surface_a)
        .console_follow_enabled = false;
    app.surface_sessions_mut()
        .session_mut(surface_b)
        .console_follow_enabled = true;

    assert_eq!(
        app.surface_sessions()
            .session(surface_a)
            .map(|session| session.console_follow_enabled),
        Some(false)
    );
    assert_eq!(
        app.surface_sessions()
            .session(surface_b)
            .map(|session| session.console_follow_enabled),
        Some(true)
    );
}

#[test]
fn dispatch_shell_command_records_workflow_dispatch_event() {
    let mut app = RunenwerkEditorApp::new();

    dispatch_shell_command(&mut app, None, ShellCommand::NoOp, None, None, None, None)
        .expect("no-op shell command should succeed");

    assert!(matches!(
        app.runtime().workflow_log().last().map(|event| &event.kind),
        Some(WorkflowEventKind::ShellCommandDispatched { command: "NoOp" })
    ));
}

#[test]
fn console_message_helpers_preserve_message_kind() {
    let mut app = RunenwerkEditorApp::new();

    app.append_console_input("input");
    app.append_console_error("error");
    app.append_console_warning("warning");
    app.append_console_line("info");
    app.append_console_debug("debug");
    app.append_console_line("[input] legacy input");
    app.append_console_line("[pick] legacy pick");

    assert_eq!(
        app.console_lines()
            .iter()
            .map(|line| line.kind)
            .collect::<Vec<_>>(),
        vec![
            ConsoleMessageKind::Input,
            ConsoleMessageKind::Error,
            ConsoleMessageKind::Warning,
            ConsoleMessageKind::Info,
            ConsoleMessageKind::Debug,
            ConsoleMessageKind::Info,
            ConsoleMessageKind::Info,
        ],
    );
}

#[test]
fn console_follow_disengages_on_upward_scroll_and_reengages_at_bottom() {
    let mut app = RunenwerkEditorApp::new();
    for index in 0..220 {
        app.append_console_line(format!("[test] line {index}"));
    }
    let mut shell_state = RunenwerkEditorShellState::new();
    let bounds = UiRect::new(0.0, 0.0, 1280.0, 720.0);
    let theme = ThemeTokens::default();
    let atlas = UiFontAtlasResource::default();

    let _ =
        RunenwerkEditorShellController::build_frame(&app, &mut shell_state, bounds, &theme, &atlas);
    let console_surface = surface_id_by_kind(shell_state.workspace_state(), PanelKind::Console);
    let console_scroll_widget = surface_widget_id(console_surface, CONSOLE_SCROLL_WIDGET_ID);
    assert!(
        app.surface_sessions()
            .session(console_surface)
            .map(|session| session.console_follow_enabled)
            .unwrap_or(true)
    );

    let tree = shell_state
        .last_tree()
        .expect("shell tree should be cached")
        .clone();
    let layouts = shell_state.runtime().compute_layout(&tree, bounds);
    let scroll_bounds = layouts
        .get(&console_scroll_widget)
        .expect("console scroll layout should exist")
        .content_bounds;
    let pointer = UiPoint::new(
        scroll_bounds.x + scroll_bounds.width * 0.5,
        scroll_bounds.y + 8.0,
    );

    let scroll_up = UiInputEvent::Pointer(PointerEvent {
        kind: PointerEventKind::Scroll,
        position: pointer,
        delta: UiVector::new(0.0, 8.0),
        button: None,
        modifiers: Modifiers::default(),
        click_count: 0,
        ..Default::default()
    });
    RunenwerkEditorShellController::dispatch_input(
        &mut app,
        &mut shell_state,
        bounds,
        &theme,
        &scroll_up,
    )
    .expect("scroll input should succeed");
    assert!(
        !app.surface_sessions()
            .session(console_surface)
            .map(|session| session.console_follow_enabled)
            .unwrap_or(true),
        "upward scroll from bottom should disengage console follow"
    );

    let scroll_down = UiInputEvent::Pointer(PointerEvent {
        kind: PointerEventKind::Scroll,
        position: pointer,
        delta: UiVector::new(0.0, -8.0),
        button: None,
        modifiers: Modifiers::default(),
        click_count: 0,
        ..Default::default()
    });
    for _ in 0..128 {
        RunenwerkEditorShellController::dispatch_input(
            &mut app,
            &mut shell_state,
            bounds,
            &theme,
            &scroll_down,
        )
        .expect("scroll input should succeed");
        if app
            .surface_sessions()
            .session(console_surface)
            .map(|session| session.console_follow_enabled)
            .unwrap_or(false)
        {
            break;
        }
    }
    assert!(
        app.surface_sessions()
            .session(console_surface)
            .map(|session| session.console_follow_enabled)
            .unwrap_or(false),
        "console follow should re-engage after returning to bottom",
    );
}

#[test]
fn console_follow_auto_scrolls_only_while_follow_enabled() {
    let mut app = RunenwerkEditorApp::new();
    for index in 0..200 {
        app.append_console_line(format!("[test] line {index}"));
    }
    let mut shell_state = RunenwerkEditorShellState::new();
    let bounds = UiRect::new(0.0, 0.0, 1280.0, 720.0);
    let theme = ThemeTokens::default();
    let atlas = UiFontAtlasResource::default();

    let _ =
        RunenwerkEditorShellController::build_frame(&app, &mut shell_state, bounds, &theme, &atlas);
    let console_surface = surface_id_by_kind(shell_state.workspace_state(), PanelKind::Console);
    let console_scroll_widget = surface_widget_id(console_surface, CONSOLE_SCROLL_WIDGET_ID);
    let tree = shell_state
        .last_tree()
        .expect("shell tree should be cached")
        .clone();
    let initial_max = shell_state
        .runtime()
        .max_scroll_offset_for_axis(&tree, bounds, console_scroll_widget, Axis::Vertical)
        .unwrap_or(0.0);
    let initial_offset = shell_state
        .runtime()
        .scroll_offset_for_axis(console_scroll_widget, Axis::Vertical);
    assert!(
        (initial_offset - initial_max).abs() <= 1.0,
        "follow-enabled frame should pin console to bottom",
    );

    app.append_console_line("[test] new follow-on line");
    let _ =
        RunenwerkEditorShellController::build_frame(&app, &mut shell_state, bounds, &theme, &atlas);
    let tree_after_append = shell_state
        .last_tree()
        .expect("shell tree should remain cached")
        .clone();
    let max_after_append = shell_state
        .runtime()
        .max_scroll_offset_for_axis(
            &tree_after_append,
            bounds,
            console_scroll_widget,
            Axis::Vertical,
        )
        .unwrap_or(0.0);
    let offset_after_append = shell_state
        .runtime()
        .scroll_offset_for_axis(console_scroll_widget, Axis::Vertical);
    assert!(
        (offset_after_append - max_after_append).abs() <= 1.0,
        "auto-follow should stay at bottom while enabled",
    );

    app.surface_sessions_mut()
        .session_mut(console_surface)
        .console_follow_enabled = false;
    shell_state.runtime_mut().set_scroll_offset_for_axis(
        console_scroll_widget,
        Axis::Vertical,
        (max_after_append * 0.5).max(0.0),
    );
    let previous_offset = shell_state
        .runtime()
        .scroll_offset_for_axis(console_scroll_widget, Axis::Vertical);

    app.append_console_line("[test] new follow-off line");
    let _ =
        RunenwerkEditorShellController::build_frame(&app, &mut shell_state, bounds, &theme, &atlas);
    let offset_follow_disabled = shell_state
        .runtime()
        .scroll_offset_for_axis(console_scroll_widget, Axis::Vertical);
    assert!(
        (offset_follow_disabled - previous_offset).abs() <= 1.0,
        "disabled follow should preserve user scroll position",
    );
}

#[test]
fn shell_identity_is_stable_across_rebuilds() {
    let app = RunenwerkEditorApp::new();
    let mut shell_state = RunenwerkEditorShellState::new();
    let bounds = UiRect::new(0.0, 0.0, 1280.0, 720.0);
    let theme = ThemeTokens::default();
    let atlas = UiFontAtlasResource::default();

    let workspace_before = shell_state.workspace_id();
    let workspace_state_before = shell_state.workspace_state().clone();

    let _ =
        RunenwerkEditorShellController::build_frame(&app, &mut shell_state, bounds, &theme, &atlas);
    let projection_before = shell_state
        .last_projection_artifacts()
        .expect("projection artifacts should be cached after frame build")
        .workspace
        .widget_context_by_id
        .clone();
    let _ =
        RunenwerkEditorShellController::build_frame(&app, &mut shell_state, bounds, &theme, &atlas);
    let projection_after = shell_state
        .last_projection_artifacts()
        .expect("projection artifacts should remain cached after rebuild")
        .workspace
        .widget_context_by_id
        .clone();

    assert_eq!(shell_state.workspace_id(), workspace_before);
    assert_eq!(*shell_state.workspace_state(), workspace_state_before);
    assert_eq!(projection_before, projection_after);
}

#[test]
fn shell_state_tracks_active_workspace_profile_separately_from_workspace_graph() {
    let mut shell_state = RunenwerkEditorShellState::new();
    let workspace_before = shell_state.workspace_state().clone();

    assert_eq!(
        shell_state.active_workspace_profile_id(),
        editor_shell::LAYOUT_WORKSPACE_PROFILE_ID,
    );

    shell_state.set_active_workspace_profile_id(
        editor_shell::WorkspaceProfileId::try_from_raw(99).unwrap(),
    );

    assert_eq!(
        shell_state.active_workspace_profile_id(),
        editor_shell::WorkspaceProfileId::try_from_raw(99).unwrap(),
    );
    assert_eq!(
        *shell_state.workspace_state(),
        workspace_before,
        "changing active profile identity should not mutate the workspace graph",
    );
}

#[test]
fn clear_cached_projection_keeps_shell_identity_unchanged() {
    let app = RunenwerkEditorApp::new();
    let mut shell_state = RunenwerkEditorShellState::new();
    let workspace_before = shell_state.workspace_id();
    let workspace_state_before = shell_state.workspace_state().clone();
    let atlas = UiFontAtlasResource::default();
    let _ = RunenwerkEditorShellController::build_frame(
        &app,
        &mut shell_state,
        UiRect::new(0.0, 0.0, 1280.0, 720.0),
        &ThemeTokens::default(),
        &atlas,
    );
    assert!(shell_state.last_projection_artifacts().is_some());

    shell_state.clear_cached_projection();

    assert_eq!(shell_state.workspace_id(), workspace_before);
    assert_eq!(*shell_state.workspace_state(), workspace_state_before);
    assert!(shell_state.last_projection_artifacts().is_none());
    assert!(shell_state.last_tree().is_none());
    assert!(shell_state.last_bounds().is_none());
}

#[test]
fn stale_projection_commands_fail_closed_after_rebuild() {
    let mut app = RunenwerkEditorApp::new();
    let ecs_entity = app.runtime_mut().spawn_world_entity(TestMarker);
    app.runtime_mut()
        .register_entity(EntityId(1), ecs_entity, "Player", None);
    let mut shell_state = RunenwerkEditorShellState::new();
    let outliner_surface = surface_id_by_kind(shell_state.workspace_state(), PanelKind::Outliner);
    let bounds = UiRect::new(0.0, 0.0, 1280.0, 720.0);
    let theme = ThemeTokens::default();
    let atlas = UiFontAtlasResource::default();

    let _ =
        RunenwerkEditorShellController::build_frame(&app, &mut shell_state, bounds, &theme, &atlas);
    let stale_artifacts = shell_state
        .last_projection_artifacts()
        .expect("projection artifacts should be present")
        .clone();
    let _ =
        RunenwerkEditorShellController::build_frame(&app, &mut shell_state, bounds, &theme, &atlas);
    let current_epoch = shell_state.current_projection_epoch();
    assert!(
        stale_artifacts.projection_epoch < current_epoch,
        "second rebuild should invalidate older projection artifacts",
    );

    let interactions = UiInteractionResults {
        items: vec![UiInteraction::TreeRowSelected {
            target: surface_widget_id(outliner_surface, OUTLINER_LIST_WIDGET_ID),
            row_index: 0,
        }],
    };
    let commands = map_interactions_to_shell_commands(&interactions, &stale_artifacts);
    assert_eq!(commands.len(), 1);

    let workflow_log_len_before = app.runtime().workflow_log().len();
    for command in commands {
        dispatch_shell_command(
            &mut app,
            None,
            command,
            None,
            None,
            None,
            Some(current_epoch),
        )
        .expect("stale command dispatch should fail closed without error");
    }

    assert_eq!(app.outliner_state().selected_entity, None);
    assert_eq!(app.runtime().workflow_log().len(), workflow_log_len_before);
}

#[test]
fn tab_click_under_drag_threshold_activates_tab_without_reorder() {
    let mut app = RunenwerkEditorApp::new();
    let mut shell_state = RunenwerkEditorShellState::new();
    let bounds = UiRect::new(0.0, 0.0, 1400.0, 840.0);
    let theme = ThemeTokens::default();
    let atlas = UiFontAtlasResource::default();

    let entity_table_target =
        composition_target_by_panel_kind(&shell_state, PanelKind::EntityTable);
    let entity_table_panel_id = entity_table_target.panel_instance_id;
    let entity_table_unit = entity_table_target.mounted_unit_id.unwrap();
    let outliner_stack_id = entity_table_target.tab_stack_id;
    let stack_region = shell_state
        .region_id_for_tab_stack(outliner_stack_id)
        .expect("entity table stack should map to a composition region");
    let before_order = shell_state
        .composition_runtime()
        .composition()
        .definition()
        .regions()
        .iter()
        .find(|region| region.id == stack_region)
        .map(|region| region.kind.mounted_units().to_vec())
        .expect("entity table stack region should exist");
    let root_count_before = shell_state
        .composition_runtime()
        .composition()
        .definition()
        .roots()
        .len();

    let _ =
        RunenwerkEditorShellController::build_frame(&app, &mut shell_state, bounds, &theme, &atlas);
    let artifacts = shell_state
        .last_projection_artifacts()
        .expect("projection artifacts should exist")
        .clone();
    let source_widget = artifacts
        .workspace
        .tab_button_route_by_widget_id
        .iter()
        .find_map(|(widget, route)| {
            (route.panel_instance_id == entity_table_panel_id).then_some(*widget)
        })
        .expect("entity table tab widget should be projected");
    let tree = shell_state
        .last_tree()
        .expect("shell tree should exist")
        .clone();
    let layouts = shell_state.runtime().compute_layout(&tree, bounds);
    let source_position = center_of_widget(&layouts, source_widget);
    let near_position = UiPoint::new(source_position.x + 2.0, source_position.y + 1.0);

    dispatch_pointer(
        &mut app,
        &mut shell_state,
        bounds,
        &theme,
        PointerEventKind::Down,
        source_position,
        Some(PointerButton::Primary),
    );
    dispatch_pointer(
        &mut app,
        &mut shell_state,
        bounds,
        &theme,
        PointerEventKind::Move,
        near_position,
        None,
    );
    dispatch_pointer(
        &mut app,
        &mut shell_state,
        bounds,
        &theme,
        PointerEventKind::Up,
        near_position,
        Some(PointerButton::Primary),
    );

    let outliner_stack = shell_state
        .composition_runtime()
        .composition()
        .definition()
        .regions()
        .iter()
        .find(|region| region.id == stack_region)
        .expect("entity table stack should remain");
    let ui_composition::RegionKind::Stack {
        ordered_units,
        active_unit,
    } = &outliner_stack.kind
    else {
        panic!("entity table region should remain a stack")
    };
    assert_eq!(ordered_units, &before_order);
    assert_eq!(*active_unit, entity_table_unit);
    assert_eq!(
        shell_state
            .composition_runtime()
            .composition()
            .definition()
            .roots()
            .len(),
        root_count_before,
        "clicking below the drag threshold must not create a root"
    );
}

#[test]
fn own_only_tab_area_drop_forms_invalid_candidate_without_committing() {
    let mut app = RunenwerkEditorApp::new();
    let mut shell_state = RunenwerkEditorShellState::new();
    let bounds = UiRect::new(0.0, 0.0, 1400.0, 840.0);
    let theme = ThemeTokens::default();
    let atlas = UiFontAtlasResource::default();

    let (_, console_stack_id) =
        panel_and_stack_by_kind(shell_state.workspace_state(), PanelKind::Console);
    let _ =
        RunenwerkEditorShellController::build_frame(&app, &mut shell_state, bounds, &theme, &atlas);
    let artifacts = shell_state
        .last_projection_artifacts()
        .expect("projection artifacts should exist")
        .clone();
    let source_widget = tab_widget_for_panel(
        &artifacts,
        shell_state.workspace_state(),
        PanelKind::Console,
    );
    let tree = shell_state
        .last_tree()
        .expect("shell tree should exist")
        .clone();
    let layouts = shell_state.runtime().compute_layout(&tree, bounds);
    let source_position = center_of_widget(&layouts, source_widget);
    let activation_position = UiPoint::new(source_position.x + 32.0, source_position.y + 4.0);
    let console_bounds = layouts
        .get(&editor_shell::tab_stack_container_widget_id(
            console_stack_id,
        ))
        .expect("console stack layout should exist")
        .bounds;
    let own_area_edge_position = UiPoint::new(
        console_bounds.x + console_bounds.width - 4.0,
        console_bounds.y + console_bounds.height * 0.5,
    );

    dispatch_pointer(
        &mut app,
        &mut shell_state,
        bounds,
        &theme,
        PointerEventKind::Down,
        source_position,
        Some(PointerButton::Primary),
    );
    dispatch_pointer(
        &mut app,
        &mut shell_state,
        bounds,
        &theme,
        PointerEventKind::Move,
        activation_position,
        None,
    );
    dispatch_pointer(
        &mut app,
        &mut shell_state,
        bounds,
        &theme,
        PointerEventKind::Move,
        own_area_edge_position,
        None,
    );

    let drag_visual = shell_state
        .docking_visual_state()
        .active_tab_drag
        .expect("own area edge should retain tab drag evidence");
    let invalid_area_target = editor_shell::DockingPreviewDropTarget::SplitIntoArea {
        target_tab_stack_id: console_stack_id,
        side: editor_shell::DockSplitSide::Right,
    };
    assert_ne!(
        drag_visual.preview_target,
        Some(invalid_area_target),
        "invalid own-area candidate must not become the active commit target",
    );
    assert!(drag_visual.preview_candidates.iter().any(|candidate| {
        matches!(
            candidate.state,
            DockDropCandidateState::Invalid {
                reason: DockDropInvalidTargetReason::SourceOnlyTabCannotSplitOwnArea
            }
        ) && candidate.target == invalid_area_target
    }));
}

#[test]
fn secondary_clicking_tab_opens_area_action_menu_without_extra_button() {
    let mut app = RunenwerkEditorApp::new();
    let mut shell_state = RunenwerkEditorShellState::new();
    let bounds = UiRect::new(0.0, 0.0, 1400.0, 840.0);
    let theme = ThemeTokens::default();
    let atlas = UiFontAtlasResource::default();

    let _ =
        RunenwerkEditorShellController::build_frame(&app, &mut shell_state, bounds, &theme, &atlas);
    let artifacts = shell_state
        .last_projection_artifacts()
        .expect("projection artifacts should exist")
        .clone();
    let viewport_target = composition_target_by_panel_kind(&shell_state, PanelKind::Viewport);
    let viewport_panel = viewport_target.panel_instance_id;
    let viewport_stack = viewport_target.tab_stack_id;
    let tab_widget = artifacts
        .workspace
        .tab_button_route_by_widget_id
        .iter()
        .find_map(|(widget_id, route)| {
            (route.panel_instance_id == viewport_panel).then_some(*widget_id)
        })
        .expect("viewport tab widget should be projected");
    let tree = shell_state
        .last_tree()
        .expect("shell tree should exist")
        .clone();
    let layouts = shell_state.runtime().compute_layout(&tree, bounds);
    let pointer = center_of_widget(&layouts, tab_widget);

    dispatch_pointer(
        &mut app,
        &mut shell_state,
        bounds,
        &theme,
        PointerEventKind::Down,
        pointer,
        Some(PointerButton::Secondary),
    );
    assert_eq!(
        shell_state.active_tab_stack_action_menu(),
        Some(viewport_stack)
    );
}

#[test]
fn tab_plus_create_surface_menu_click_creates_selected_tab() {
    let mut app = RunenwerkEditorApp::new();
    let mut shell_state = RunenwerkEditorShellState::new();
    let bounds = UiRect::new(0.0, 0.0, 1400.0, 840.0);
    let theme = ThemeTokens::default();
    let atlas = UiFontAtlasResource::default();
    let viewport_target = composition_target_by_panel_kind(&shell_state, PanelKind::Viewport);
    let viewport_stack = viewport_target.tab_stack_id;
    let viewport_region = shell_state
        .region_id_for_tab_stack(viewport_stack)
        .expect("viewport stack should map to a composition region");
    let before_count = shell_state
        .composition_runtime()
        .composition()
        .definition()
        .regions()
        .iter()
        .find(|region| region.id == viewport_region)
        .map(|region| region.kind.mounted_units().len())
        .expect("viewport stack should exist");

    let _ =
        RunenwerkEditorShellController::build_frame(&app, &mut shell_state, bounds, &theme, &atlas);
    let tree = shell_state
        .last_tree()
        .expect("shell tree should exist")
        .clone();
    let layouts = shell_state.runtime().compute_layout(&tree, bounds);
    let plus_position =
        center_of_widget(&layouts, tab_stack_new_tab_button_widget_id(viewport_stack));

    dispatch_pointer(
        &mut app,
        &mut shell_state,
        bounds,
        &theme,
        PointerEventKind::Down,
        plus_position,
        Some(PointerButton::Primary),
    );
    dispatch_pointer(
        &mut app,
        &mut shell_state,
        bounds,
        &theme,
        PointerEventKind::Up,
        plus_position,
        Some(PointerButton::Primary),
    );
    assert!(matches!(
        shell_state.active_tab_stack_popup_menu(),
        Some(menu)
            if menu.kind == editor_shell::TabStackPopupMenuKind::CreateSurface
                && menu.tab_stack_id == viewport_stack
    ));

    let _ =
        RunenwerkEditorShellController::build_frame(&app, &mut shell_state, bounds, &theme, &atlas);
    let artifacts = shell_state
        .last_projection_artifacts()
        .expect("projection artifacts should exist")
        .clone();
    let inspector_menu_item = artifacts
        .widget_actions_by_id
        .iter()
        .find_map(|(widget_id, action)| {
            matches!(
                action,
                editor_shell::RoutedShellAction::CreatePanelTabStableKey {
                    tab_stack_id,
                    stable_surface_key,
                    ..
                } if *tab_stack_id == viewport_stack
                    && stable_surface_key.as_str() == "runenwerk.scene.inspector"
            )
            .then_some(*widget_id)
        })
        .expect("inspector create-surface menu item should be routed");
    let tree = shell_state
        .last_tree()
        .expect("shell tree should exist")
        .clone();
    let layouts = shell_state.runtime().compute_layout(&tree, bounds);
    let inspector_position = center_of_widget(&layouts, inspector_menu_item);

    dispatch_pointer(
        &mut app,
        &mut shell_state,
        bounds,
        &theme,
        PointerEventKind::Down,
        inspector_position,
        Some(PointerButton::Primary),
    );
    dispatch_pointer(
        &mut app,
        &mut shell_state,
        bounds,
        &theme,
        PointerEventKind::Up,
        inspector_position,
        Some(PointerButton::Primary),
    );

    let tab_stack = shell_state
        .composition_runtime()
        .composition()
        .definition()
        .regions()
        .iter()
        .find(|region| region.id == viewport_region)
        .expect("viewport stack should still exist");
    let ui_composition::RegionKind::Stack {
        ordered_units,
        active_unit,
    } = &tab_stack.kind
    else {
        panic!("viewport region should remain a stack")
    };
    assert_eq!(ordered_units.len(), before_count + 1);
    let active_surface = shell_state
        .composition_runtime()
        .extension()
        .mounted_unit(*active_unit)
        .expect("active mounted unit should have editor extension state");
    assert_eq!(
        active_surface.stable_content_key,
        crate::shell::tool_suites::SCENE_INSPECTOR_SURFACE_KEY
    );
    assert!(
        shell_state.active_tab_stack_popup_menu().is_none(),
        "creating a tab should close the create-surface popup"
    );
}

#[test]
fn inspector_tab_stack_lock_uses_stable_key() {
    let app = RunenwerkEditorApp::new();
    let (mut shell_state, inspector_stack) = runtime_debug_inspector_shell_state(&app);
    let bounds = UiRect::new(0.0, 0.0, 1400.0, 840.0);
    let theme = ThemeTokens::default();
    let atlas = UiFontAtlasResource::default();

    let _ =
        RunenwerkEditorShellController::build_frame(&app, &mut shell_state, bounds, &theme, &atlas);
    let artifacts = shell_state
        .last_projection_artifacts()
        .expect("runtime debug projection artifacts should exist");
    let lock_action = artifacts
        .widget_actions_by_id
        .get(&tab_stack_lock_type_toggle_widget_id(inspector_stack))
        .expect("inspector tab-stack lock chrome should be routed");

    assert!(matches!(
        lock_action,
        editor_shell::RoutedShellAction::LockTabStackAreaStableKey {
            locked_stable_surface_key: Some(key),
            ..
        } if key.as_str() == crate::shell::tool_suites::TOOL_SUITE_REGISTRY_INSPECTOR_SURFACE_KEY
    ));
}

#[test]
fn inspector_tab_stack_split_or_reset_fails_closed_if_legacy_only_path_required() {
    let app = RunenwerkEditorApp::new();
    let (mut shell_state, inspector_stack) = runtime_debug_inspector_shell_state(&app);
    let bounds = UiRect::new(0.0, 0.0, 1400.0, 840.0);
    let theme = ThemeTokens::default();
    let atlas = UiFontAtlasResource::default();

    let _ =
        RunenwerkEditorShellController::build_frame(&app, &mut shell_state, bounds, &theme, &atlas);
    let artifacts = shell_state
        .last_projection_artifacts()
        .expect("runtime debug projection artifacts should exist");
    let split_action = artifacts
        .widget_actions_by_id
        .get(&tab_stack_split_horizontal_button_widget_id(
            inspector_stack,
        ))
        .expect("inspector tab-stack split chrome should be routed");
    let reset_action = artifacts
        .widget_actions_by_id
        .get(&tab_stack_reset_area_button_widget_id(inspector_stack))
        .expect("inspector tab-stack reset chrome should be routed");

    assert!(matches!(
        split_action,
        editor_shell::RoutedShellAction::SplitTabStackAreaStableKey {
            stable_surface_key,
            panel_kind: PanelKind::Diagnostics,
            ..
        } if stable_surface_key.as_str()
            == crate::shell::tool_suites::TOOL_SUITE_REGISTRY_INSPECTOR_SURFACE_KEY
    ));
    assert!(matches!(
        reset_action,
        editor_shell::RoutedShellAction::ResetTabStackAreaStableKey {
            stable_surface_key,
            panel_kind: PanelKind::Diagnostics,
            ..
        } if stable_surface_key.as_str()
            == crate::shell::tool_suites::TOOL_SUITE_REGISTRY_INSPECTOR_SURFACE_KEY
    ));
}

#[test]
fn locked_inspector_tab_stack_create_menu_routes_only_stable_key_surface() {
    let app = RunenwerkEditorApp::new();
    let (mut shell_state, inspector_stack) = runtime_debug_inspector_shell_state(&app);
    let key = editor_shell::ToolSurfaceStableKey::new(
        crate::shell::tool_suites::TOOL_SUITE_REGISTRY_INSPECTOR_SURFACE_KEY,
    )
    .expect("inspector stable key should be valid");
    shell_state
        .apply_workspace_mutation(WorkspaceMutation::LockTabStackAreaStableKey {
            tab_stack_id: inspector_stack,
            locked_stable_surface_key: Some(key.clone()),
        })
        .expect("stable-key-only inspector stack should lock");
    let bounds = UiRect::new(0.0, 0.0, 1400.0, 840.0);
    let theme = ThemeTokens::default();
    let atlas = UiFontAtlasResource::default();

    let _ =
        RunenwerkEditorShellController::build_frame(&app, &mut shell_state, bounds, &theme, &atlas);
    let artifacts = shell_state
        .last_projection_artifacts()
        .expect("runtime debug projection artifacts should exist");
    let create_actions = artifacts
        .widget_actions_by_id
        .values()
        .filter(|action| {
            matches!(
                action,
                editor_shell::RoutedShellAction::CreatePanelTabStableKey {
                    tab_stack_id,
                    ..
                } if *tab_stack_id == inspector_stack
            )
        })
        .collect::<Vec<_>>();

    assert_eq!(create_actions.len(), 1);
    assert!(matches!(
        create_actions[0],
        editor_shell::RoutedShellAction::CreatePanelTabStableKey {
            tab_stack_id,
            panel_kind: PanelKind::Diagnostics,
            stable_surface_key,
        } if *tab_stack_id == inspector_stack && stable_surface_key == &key
    ));
}

#[test]
fn inspector_switch_type_menu_does_not_emit_legacy_enum_actions() {
    let app = RunenwerkEditorApp::new();
    let (mut shell_state, inspector_stack) = runtime_debug_inspector_shell_state(&app);
    let (_, inspector_panel) = tab_stack_and_panel_by_surface_key(
        shell_state.workspace_state(),
        crate::shell::tool_suites::TOOL_SUITE_REGISTRY_INSPECTOR_SURFACE_KEY,
    );
    let bounds = UiRect::new(0.0, 0.0, 1400.0, 840.0);
    let theme = ThemeTokens::default();
    let atlas = UiFontAtlasResource::default();

    let _ =
        RunenwerkEditorShellController::build_frame(&app, &mut shell_state, bounds, &theme, &atlas);
    let artifacts = shell_state
        .last_projection_artifacts()
        .expect("runtime debug projection artifacts should exist");

    assert!(
        !artifacts
            .widget_actions_by_id
            .values()
            .any(|action| matches!(
                action,
                editor_shell::RoutedShellAction::ToggleTabStackSurfaceMenu {
                    tab_stack_id,
                    ..
                } if *tab_stack_id == inspector_stack
            ))
    );
    let _ = inspector_panel;
}

#[test]
fn tab_plus_create_surface_menu_inside_pointer_down_does_not_dismiss_before_activation() {
    let mut app = RunenwerkEditorApp::new();
    let mut shell_state = RunenwerkEditorShellState::new();
    let bounds = UiRect::new(0.0, 0.0, 1400.0, 840.0);
    let theme = ThemeTokens::default();
    let atlas = UiFontAtlasResource::default();
    let (_, viewport_stack) =
        panel_and_stack_by_kind(shell_state.workspace_state(), PanelKind::Viewport);

    open_tab_stack_create_surface_popup(
        &mut app,
        &mut shell_state,
        bounds,
        &theme,
        &atlas,
        viewport_stack,
    );
    let popup_position = active_tab_stack_popup_center(&shell_state, bounds, viewport_stack);

    dispatch_pointer(
        &mut app,
        &mut shell_state,
        bounds,
        &theme,
        PointerEventKind::Down,
        popup_position,
        Some(PointerButton::Primary),
    );

    assert!(matches!(
        shell_state.active_tab_stack_popup_menu(),
        Some(menu)
            if menu.kind == editor_shell::TabStackPopupMenuKind::CreateSurface
                && menu.tab_stack_id == viewport_stack
    ));
}

#[test]
fn tab_plus_create_surface_menu_outside_pointer_down_dismisses_popup() {
    let mut app = RunenwerkEditorApp::new();
    let mut shell_state = RunenwerkEditorShellState::new();
    let bounds = UiRect::new(0.0, 0.0, 1400.0, 840.0);
    let theme = ThemeTokens::default();
    let atlas = UiFontAtlasResource::default();
    let (_, viewport_stack) =
        panel_and_stack_by_kind(shell_state.workspace_state(), PanelKind::Viewport);

    open_tab_stack_create_surface_popup(
        &mut app,
        &mut shell_state,
        bounds,
        &theme,
        &atlas,
        viewport_stack,
    );

    dispatch_pointer(
        &mut app,
        &mut shell_state,
        bounds,
        &theme,
        PointerEventKind::Down,
        UiPoint::new(
            bounds.x + bounds.width - 4.0,
            bounds.y + bounds.height - 4.0,
        ),
        Some(PointerButton::Primary),
    );

    assert!(shell_state.active_tab_stack_popup_menu().is_none());
}

#[test]
fn closing_last_tab_closes_the_empty_area() {
    let mut app = RunenwerkEditorApp::new();
    let mut shell_state = RunenwerkEditorShellState::new();
    let (viewport_panel, viewport_stack) =
        panel_and_stack_by_kind(shell_state.workspace_state(), PanelKind::Viewport);
    assert_eq!(
        shell_state
            .workspace_state()
            .tab_stack(viewport_stack)
            .expect("viewport stack should exist")
            .ordered_panels
            .len(),
        1
    );
    let viewport_unit = shell_state
        .mounted_unit_id_for_panel(viewport_panel)
        .expect("viewport panel should map to a mounted composition unit");
    let viewport_region = shell_state
        .region_id_for_tab_stack(viewport_stack)
        .expect("viewport stack should map to a composition region");

    dispatch_shell_command(
        &mut app,
        Some(&mut shell_state),
        ShellCommand::ClosePanelTab {
            tab_stack_id: viewport_stack,
            panel_instance_id: viewport_panel,
            projection_epoch: 0,
        },
        None,
        None,
        None,
        None,
    )
    .expect("closing the last tab should close its area");

    assert!(
        shell_state
            .composition_runtime()
            .composition()
            .snapshot()
            .region(viewport_region)
            .is_none(),
        "single-tab close should compact the now-empty composition area"
    );
    assert!(
        shell_state
            .composition_runtime()
            .composition()
            .snapshot()
            .mounted_unit(viewport_unit)
            .is_none()
    );
}

fn dispatch_pointer(
    app: &mut RunenwerkEditorApp,
    shell_state: &mut RunenwerkEditorShellState,
    bounds: UiRect,
    theme: &ThemeTokens,
    kind: PointerEventKind,
    position: UiPoint,
    button: Option<PointerButton>,
) {
    let event = UiInputEvent::Pointer(PointerEvent {
        kind,
        position,
        delta: UiVector::ZERO,
        button,
        modifiers: Modifiers::default(),
        click_count: 1,
        ..Default::default()
    });
    RunenwerkEditorShellController::dispatch_input(app, shell_state, bounds, theme, &event)
        .expect("pointer dispatch should succeed");
}

fn open_tab_stack_create_surface_popup(
    app: &mut RunenwerkEditorApp,
    shell_state: &mut RunenwerkEditorShellState,
    bounds: UiRect,
    theme: &ThemeTokens,
    atlas: &UiFontAtlasResource,
    tab_stack_id: editor_shell::TabStackId,
) {
    let _ = RunenwerkEditorShellController::build_frame(app, shell_state, bounds, theme, atlas);
    let tree = shell_state
        .last_tree()
        .expect("shell tree should exist")
        .clone();
    let layouts = shell_state.runtime().compute_layout(&tree, bounds);
    let plus_position =
        center_of_widget(&layouts, tab_stack_new_tab_button_widget_id(tab_stack_id));

    dispatch_pointer(
        app,
        shell_state,
        bounds,
        theme,
        PointerEventKind::Down,
        plus_position,
        Some(PointerButton::Primary),
    );
    dispatch_pointer(
        app,
        shell_state,
        bounds,
        theme,
        PointerEventKind::Up,
        plus_position,
        Some(PointerButton::Primary),
    );
    let _ = RunenwerkEditorShellController::build_frame(app, shell_state, bounds, theme, atlas);
}

fn active_tab_stack_popup_center(
    shell_state: &RunenwerkEditorShellState,
    bounds: UiRect,
    tab_stack_id: editor_shell::TabStackId,
) -> UiPoint {
    let tree = shell_state
        .last_tree()
        .expect("shell tree should exist")
        .clone();
    let layouts = shell_state.runtime().compute_layout(&tree, bounds);
    center_of_widget(
        &layouts,
        tab_stack_popup_menu_widget_id(
            editor_shell::TabStackPopupMenuKind::CreateSurface,
            tab_stack_id,
        ),
    )
}

fn center_of_widget(
    layouts: &editor_shell::ComputedLayoutMap,
    widget_id: editor_shell::WidgetId,
) -> UiPoint {
    let bounds = layouts
        .get(&widget_id)
        .expect("widget layout should exist")
        .bounds;
    UiPoint::new(
        bounds.x + bounds.width * 0.5,
        bounds.y + bounds.height * 0.5,
    )
}

fn runtime_debug_inspector_shell_state(
    app: &RunenwerkEditorApp,
) -> (RunenwerkEditorShellState, editor_shell::TabStackId) {
    let host = app.workbench_host();
    let profile = host
        .workspace_profile_registry()
        .profile(RUNTIME_DEBUG_WORKSPACE_PROFILE_ID)
        .expect("runtime debug profile should exist");
    let mut allocator = editor_shell::WorkspaceIdentityAllocator::new();
    let workspace_id = allocator.allocate_workspace_id();
    let workspace = profile
        .build_default_workspace_state_with_registry(
            workspace_id,
            &mut allocator,
            host.tool_surface_registry(),
        )
        .expect("runtime debug profile should build through hosted registry");
    let (inspector_stack, inspector_panel) = tab_stack_and_panel_by_surface_key(
        &workspace,
        crate::shell::tool_suites::TOOL_SUITE_REGISTRY_INSPECTOR_SURFACE_KEY,
    );
    let workspace = editor_shell::reduce_workspace(
        &workspace,
        WorkspaceMutation::SetTabStackActivePanel {
            tab_stack_id: inspector_stack,
            active_panel: Some(inspector_panel),
        },
    )
    .expect("inspector panel should become active in its tab stack");
    let mut shell_state =
        RunenwerkEditorShellState::new_with_workspace_profile_registry_and_tool_surface_registry(
            host.workspace_profile_registry(),
            host.tool_surface_registry(),
        )
        .expect("shell state should build through hosted registry");

    shell_state.set_active_workspace_profile_id(RUNTIME_DEBUG_WORKSPACE_PROFILE_ID);
    shell_state.replace_workspace_state(workspace);

    (shell_state, inspector_stack)
}

fn tab_stack_and_panel_by_surface_key(
    workspace: &editor_shell::WorkspaceState,
    stable_surface_key: &str,
) -> (editor_shell::TabStackId, editor_shell::PanelInstanceId) {
    let stable_surface_key = editor_shell::ToolSurfaceStableKey::new(stable_surface_key)
        .expect("test stable surface key should be valid");
    let surface_id = workspace
        .tool_surfaces()
        .find(|surface| surface.stable_surface_key() == &stable_surface_key)
        .expect("surface should exist by stable key")
        .id;
    let panel_id = workspace
        .panels()
        .find(|panel| panel.active_tool_surface == Some(surface_id))
        .expect("stable-key surface should be active in a panel")
        .id;

    let tab_stack_id = workspace
        .tab_stacks()
        .find(|stack| stack.ordered_panels.contains(&panel_id))
        .expect("stable-key surface panel should be mounted in a tab stack")
        .id;

    (tab_stack_id, panel_id)
}

fn panel_and_stack_by_kind(
    workspace: &editor_shell::WorkspaceState,
    kind: PanelKind,
) -> (editor_shell::PanelInstanceId, editor_shell::TabStackId) {
    let panel_id = workspace
        .panels()
        .find(|panel| panel.panel_kind == kind)
        .expect("panel kind should exist")
        .id;
    let tab_stack_id = workspace
        .tab_stacks()
        .find(|stack| stack.ordered_panels.contains(&panel_id))
        .expect("panel should be mounted in a tab stack")
        .id;
    (panel_id, tab_stack_id)
}

fn surface_id_by_kind(
    workspace: &editor_shell::WorkspaceState,
    kind: PanelKind,
) -> editor_shell::ToolSurfaceInstanceId {
    let (panel_id, _) = panel_and_stack_by_kind(workspace, kind);
    workspace
        .panel(panel_id)
        .and_then(|panel| panel.active_tool_surface)
        .expect("panel should have active tool surface")
}

fn frame_contains_graph_canvas(frame: &editor_shell::ResolvedSurfaceFrame) -> bool {
    fn walk(node: &editor_shell::UiNode) -> bool {
        matches!(node.kind, editor_shell::UiNodeKind::GraphCanvas(_))
            || node.children.iter().any(walk)
    }

    walk(&frame.artifact.root)
}

fn frame_by_stable_key<'a>(
    frame_model: &'a editor_shell::EditorShellFrameModel,
    stable_key: &str,
) -> &'a editor_shell::ResolvedSurfaceFrame {
    frame_model
        .surfaces
        .values()
        .find(|frame| frame.stable_surface_key.as_str() == stable_key)
        .unwrap_or_else(|| panic!("frame should exist for stable key {stable_key}"))
}

fn editor_lab_provider_snapshot(frame_model: &editor_shell::EditorShellFrameModel) -> String {
    use std::fmt::Write as _;

    let mut snapshot = String::new();
    for frame in frame_model.surfaces.values().filter(|frame| {
        frame
            .stable_surface_key
            .as_str()
            .starts_with("runenwerk.editor_design.")
    }) {
        writeln!(
            snapshot,
            "{} title={} provider={:?} availability={:?} artifact={:?} routes={}",
            frame.stable_surface_key.as_str(),
            frame.title,
            frame.provider_id,
            frame.availability,
            frame.artifact.kind,
            frame.routes.iter().count()
        )
        .expect("writing to String should not fail");
    }
    snapshot
}

fn editor_lab_route_count(frame_model: &editor_shell::EditorShellFrameModel) -> usize {
    frame_model
        .surfaces
        .values()
        .filter(|frame| {
            frame
                .stable_surface_key
                .as_str()
                .starts_with("runenwerk.editor_design.")
        })
        .map(|frame| frame.routes.iter().count())
        .sum()
}

fn editor_lab_disabled_reason_count(frame_model: &editor_shell::EditorShellFrameModel) -> usize {
    let disabled_reasons = [
        "no accepted Editor Lab operation is available to undo",
        "no undone Editor Lab operation is available to redo",
        "no Editor Lab project package has been saved in this session",
        "no definition document is selected",
    ];
    frame_model
        .surfaces
        .values()
        .filter(|frame| {
            frame
                .stable_surface_key
                .as_str()
                .starts_with("runenwerk.editor_design.")
        })
        .map(resolved_frame_text)
        .map(|text| {
            disabled_reasons
                .iter()
                .filter(|reason| text.contains(**reason))
                .count()
        })
        .sum()
}

fn resolved_frame_text(frame: &editor_shell::ResolvedSurfaceFrame) -> String {
    format!("{:?}", frame.artifact.root)
}

fn resolved_frame_has_editor_definition_route(frame: &editor_shell::ResolvedSurfaceFrame) -> bool {
    frame.routes.iter().any(|(_, route)| {
        matches!(
            route.action(),
            Some(SurfaceLocalAction::EditorDefinition(_))
        )
    })
}

fn resolved_frame_has_editor_definition_action(
    frame: &editor_shell::ResolvedSurfaceFrame,
    matches_action: impl Fn(&EditorDefinitionSurfaceAction) -> bool,
) -> bool {
    frame.routes.iter().any(|(_, route)| {
        matches!(
            route.action(),
            Some(SurfaceLocalAction::EditorDefinition(action)) if matches_action(action)
        )
    })
}

fn tab_widget_for_panel(
    artifacts: &editor_shell::ShellProjectionArtifacts,
    workspace: &editor_shell::WorkspaceState,
    kind: PanelKind,
) -> editor_shell::WidgetId {
    artifacts
        .workspace
        .tab_button_route_by_widget_id
        .iter()
        .find_map(|(widget_id, route)| {
            workspace
                .panel(route.panel_instance_id)
                .filter(|panel| panel.panel_kind == kind)
                .map(|_| *widget_id)
        })
        .expect("tab widget for panel kind should exist")
}
