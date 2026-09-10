use std::collections::BTreeMap;

use editor_core::EntityId;
use editor_definition::{
    EditorDefinitionDocument, EditorDefinitionDocumentContent, EditorDefinitionDocumentKind,
    EditorDefinitionId, EditorWorkbenchCompositionDefinition, EditorWorkspaceLayoutDefinition,
    EditorWorkspaceProfileDefinition, editor_definition_has_blocking_diagnostics,
    validate_editor_definition_document,
};
use editor_scene::{
    SceneMaterialAssignmentState, SceneMaterialBindingDiagnostic,
    SceneMaterialBindingDiagnosticCode, SceneMaterialBindingDiagnosticSubject,
    SceneMaterialResolution, SceneMaterialSlotId, SceneModelMeshMaterialRegionSourceId,
    SceneModelMeshSourceId,
};
use editor_shell::{
    EDITOR_DESIGN_WORKSPACE_PROFILE_ID, MATERIAL_WORKSPACE_PROFILE_ID, ToolSurfaceInstanceId,
    workbench_composition_manifest_from_definition,
    workspace_profile_manifests_from_authored_documents,
};
use editor_viewport::{
    ViewportCameraSettings, ViewportFieldVisualizerSettings, ViewportId, ViewportRuntimeSettings,
};
use engine::plugins::render::{
    GpuParams, GpuUniform, PreparedModelMeshMaterialRegionIdentity,
    PreparedModelMeshMaterialSelection, PreparedModelMeshMaterialSourceIdentity,
};
use glam::{Vec3, vec3};
use scene::{LocalTransform, Vec3Value};
use ui_definition::UiDefinitionDiagnostic;
use ui_math::UiVector;
use ui_theme::ThemeTokens;

use crate::editor_app::RunenwerkEditorApp;
use crate::editor_runtime::{EditorPrimitive, EditorPrimitiveKind};
use crate::material_lab::ensure_default_material_source_document;
use crate::runtime::preview_process::PreviewProcessManager;
use crate::shell::{
    EditorDefinitionActivation, EditorDefinitionActivationPayload,
    EditorDefinitionActivationReport, EditorDefinitionActivationStatus,
    PendingEditorDefinitionActivation, RunenwerkEditorShellState, RunenwerkWorkbenchHost,
    activate_editor_definition_document, is_known_editor_command_key, validate_editor_shortcuts,
};

const SHELL_READABILITY_BUMP: f32 = 1.15;
const SHELL_SCALE_MIN: f32 = 1.0;
const SHELL_SCALE_MAX: f32 = 3.0;
const VIEWPORT_BOUNDS_EPSILON: f32 = 0.25;
const BRANCH_TRACE_FLOAT_EPSILON: f32 = 0.0005;
const CAMERA_MIN_DISTANCE: f32 = 0.25;
const CAMERA_MAX_DISTANCE: f32 = 500.0;
const CAMERA_ORBIT_SENSITIVITY: f32 = 0.006;
const CAMERA_PAN_SENSITIVITY: f32 = 0.0015;
const CAMERA_ZOOM_SENSITIVITY: f32 = 0.08;
const CAMERA_MAX_PITCH_RADIANS: f32 = 1.553_343;
pub const EDITOR_VIEWPORT_MAX_PRIMITIVE_INSTANCES: usize = 64;
pub const EDITOR_VIEWPORT_MAX_MODEL_MESH_MATERIAL_REGIONS: usize = 16;

pub use editor_viewport::ViewportDebugStage as EditorViewportDebugStage;

#[derive(runen_ecs::Component, runen_ecs::Resource)]
pub struct EditorHostResource {
    pub app: RunenwerkEditorApp,
    pub shell_state: RunenwerkEditorShellState,
    pub theme: ThemeTokens,
}

#[derive(Default, runen_ecs::Resource)]
pub struct RuntimePreviewProcessResource {
    pub manager: PreviewProcessManager,
}

impl Default for EditorHostResource {
    fn default() -> Self {
        Self::new()
    }
}

impl EditorHostResource {
    pub fn new() -> Self {
        let mut app = RunenwerkEditorApp::new();
        ensure_default_material_source_document(&mut app);
        let shell_state = RunenwerkEditorShellState::new_with_workspace_profile_registry_and_tool_surface_registry(
            app.workbench_host().workspace_profile_registry(),
            app.workbench_host().tool_surface_registry(),
        )
        .expect("default shell workspace should be compatible with workbench registry");
        Self {
            app,
            shell_state,
            theme: ThemeTokens::default(),
        }
    }

    pub fn material_lab_workbench() -> Self {
        let mut app = RunenwerkEditorApp::new_material_lab_workbench();
        ensure_default_material_source_document(&mut app);
        app.runtime_mut()
            .activate_default_material_graph_document()
            .expect("Material Lab workbench should open a default material graph document");
        let shell_state =
            RunenwerkEditorShellState::new_for_workspace_profile_with_workspace_profile_registry_and_tool_surface_registry(
                MATERIAL_WORKSPACE_PROFILE_ID,
                app.workbench_host().workspace_profile_registry(),
                app.workbench_host().tool_surface_registry(),
            )
            .expect("Material Lab shell workspace should be compatible with workbench registry");
        Self {
            app,
            shell_state,
            theme: ThemeTokens::default(),
        }
    }

    pub fn ui_designer_workbench() -> Self {
        let app = RunenwerkEditorApp::new_ui_designer_workbench();
        let shell_state =
            RunenwerkEditorShellState::new_for_workspace_profile_with_workspace_profile_registry_and_tool_surface_registry(
                EDITOR_DESIGN_WORKSPACE_PROFILE_ID,
                app.workbench_host().workspace_profile_registry(),
                app.workbench_host().tool_surface_registry(),
            )
            .expect("UI Designer shell workspace should be compatible with workbench registry");
        Self {
            app,
            shell_state,
            theme: ThemeTokens::default(),
        }
    }

    pub fn apply_theme(&mut self, theme: ThemeTokens) {
        self.theme = theme;
    }

    pub fn apply_pending_editor_definition_activations(&mut self) -> usize {
        let pending = self.app.take_pending_editor_definition_activations();
        let mut activated = 0;
        for request in pending {
            match request.payload.clone() {
                EditorDefinitionActivationPayload::Document(document) => {
                    activated +=
                        self.apply_editor_definition_document_activation(request, document);
                }
                EditorDefinitionActivationPayload::WorkbenchCompositionPackage {
                    composition,
                    profiles,
                    layouts,
                } => {
                    activated += self.apply_workbench_composition_package_activation(
                        request,
                        composition,
                        profiles,
                        layouts,
                    );
                }
            }
        }
        activated
    }

    fn apply_editor_definition_document_activation(
        &mut self,
        request: PendingEditorDefinitionActivation,
        document: EditorDefinitionDocument,
    ) -> usize {
        let mut activated = 0;
        match activate_editor_definition_document(&document, &self.theme) {
            Ok(EditorDefinitionActivation::ThemeChanged(theme)) => {
                self.apply_theme(theme);
                self.app.append_console_line(format!(
                    "[editor-definition] activated live theme {}",
                    document.display_name
                ));
                self.app.record_editor_definition_activation_report(
                    EditorDefinitionActivationReport::from_request(
                        &request,
                        EditorDefinitionActivationStatus::Applied,
                        vec![format!("activated live theme {}", document.display_name)],
                        Vec::new(),
                        false,
                    ),
                );
                activated += 1;
            }
            Ok(EditorDefinitionActivation::NoLiveActivation) => {
                self.app.append_console_line(format!(
                    "[editor-definition] no live activation for {}",
                    document.display_name
                ));
                self.app.record_editor_definition_activation_report(
                    EditorDefinitionActivationReport::from_request(
                        &request,
                        EditorDefinitionActivationStatus::NoLiveActivation,
                        vec![format!("no live activation for {}", document.display_name)],
                        Vec::new(),
                        true,
                    ),
                );
            }
            Ok(EditorDefinitionActivation::UiTemplateCatalogChanged {
                template_id,
                template,
            }) => {
                self.shell_state
                    .active_editor_definitions_mut()
                    .install_template(template);
                self.app.append_console_line(format!(
                    "[editor-definition] activated UI template {template_id}"
                ));
                self.app.record_editor_definition_activation_report(
                    EditorDefinitionActivationReport::from_request(
                        &request,
                        EditorDefinitionActivationStatus::Applied,
                        vec![format!("activated UI template {template_id}")],
                        Vec::new(),
                        false,
                    ),
                );
                activated += 1;
            }
            Ok(EditorDefinitionActivation::EditorBindingsCatalogChanged(bindings)) => {
                match self
                    .shell_state
                    .active_editor_definitions_mut()
                    .install_editor_bindings(bindings)
                {
                    Ok(()) => {
                        self.app.append_console_line(
                            "[editor-definition] activated editor bindings".to_string(),
                        );
                        self.app.record_editor_definition_activation_report(
                            EditorDefinitionActivationReport::from_request(
                                &request,
                                EditorDefinitionActivationStatus::Applied,
                                vec!["activated editor bindings".to_string()],
                                Vec::new(),
                                false,
                            ),
                        );
                        activated += 1;
                    }
                    Err(diagnostics) => {
                        for diagnostic in &diagnostics {
                            self.app.append_console_line(format!(
                                "[editor-definition] editor bindings activation blocked: {}",
                                diagnostic.message
                            ));
                        }
                        self.app
                            .preserve_failed_editor_definition_activation(request.clone());
                        self.app.record_editor_definition_activation_report(
                            EditorDefinitionActivationReport::from_request(
                                &request,
                                EditorDefinitionActivationStatus::Failed,
                                vec!["editor bindings activation blocked".to_string()],
                                diagnostics,
                                true,
                            ),
                        );
                    }
                }
            }
            Ok(EditorDefinitionActivation::MenuCatalogChanged { menu_id, menu }) => {
                self.shell_state
                    .active_editor_definitions_mut()
                    .install_menu(menu);
                self.app
                    .append_console_line(format!("[editor-definition] activated menu {menu_id}"));
                self.app.record_editor_definition_activation_report(
                    EditorDefinitionActivationReport::from_request(
                        &request,
                        EditorDefinitionActivationStatus::Applied,
                        vec![format!("activated menu {menu_id}")],
                        Vec::new(),
                        false,
                    ),
                );
                activated += 1;
            }
            Ok(EditorDefinitionActivation::ShortcutCatalogChanged {
                shortcut_set_id,
                shortcuts,
            }) => {
                match self
                    .shell_state
                    .active_editor_definitions_mut()
                    .install_shortcuts(shortcuts, validate_editor_shortcuts)
                {
                    Ok(()) => {
                        self.app.append_console_line(format!(
                            "[editor-definition] activated shortcut set {shortcut_set_id}"
                        ));
                        self.app.record_editor_definition_activation_report(
                            EditorDefinitionActivationReport::from_request(
                                &request,
                                EditorDefinitionActivationStatus::Applied,
                                vec![format!("activated shortcut set {shortcut_set_id}")],
                                Vec::new(),
                                false,
                            ),
                        );
                        activated += 1;
                    }
                    Err(diagnostics) => {
                        for diagnostic in &diagnostics {
                            self.app.append_console_line(format!(
                                "[editor-definition] shortcut activation blocked: {}",
                                diagnostic.message
                            ));
                        }
                        self.app
                            .preserve_failed_editor_definition_activation(request.clone());
                        self.app.record_editor_definition_activation_report(
                            EditorDefinitionActivationReport::from_request(
                                &request,
                                EditorDefinitionActivationStatus::Failed,
                                vec!["shortcut activation blocked".to_string()],
                                diagnostics,
                                true,
                            ),
                        );
                    }
                }
            }
            Ok(EditorDefinitionActivation::CommandBindingCatalogChanged {
                command_binding_set_id,
                command_bindings,
            }) => {
                match self
                    .shell_state
                    .active_editor_definitions_mut()
                    .install_command_bindings(command_bindings, is_known_editor_command_key)
                {
                    Ok(()) => {
                        self.app.append_console_line(format!(
                                "[editor-definition] activated command binding set {command_binding_set_id}"
                            ));
                        self.app.record_editor_definition_activation_report(
                            EditorDefinitionActivationReport::from_request(
                                &request,
                                EditorDefinitionActivationStatus::Applied,
                                vec![format!(
                                    "activated command binding set {command_binding_set_id}"
                                )],
                                Vec::new(),
                                false,
                            ),
                        );
                        activated += 1;
                    }
                    Err(diagnostics) => {
                        for diagnostic in &diagnostics {
                            self.app.append_console_line(format!(
                                "[editor-definition] command binding activation blocked: {}",
                                diagnostic.message
                            ));
                        }
                        self.app
                            .preserve_failed_editor_definition_activation(request.clone());
                        self.app.record_editor_definition_activation_report(
                            EditorDefinitionActivationReport::from_request(
                                &request,
                                EditorDefinitionActivationStatus::Failed,
                                vec!["command binding activation blocked".to_string()],
                                diagnostics,
                                true,
                            ),
                        );
                    }
                }
            }
            Ok(EditorDefinitionActivation::PanelRegistryCatalogChanged {
                registry_id,
                registry,
            }) => {
                let composition = self.shell_state.composition_runtime().clone();
                match self
                    .shell_state
                    .active_editor_definitions_mut()
                    .install_panel_registry(registry, &composition)
                {
                    Ok(()) => {
                        self.app.append_console_line(format!(
                            "[editor-definition] activated panel registry {registry_id}"
                        ));
                        self.app.record_editor_definition_activation_report(
                            EditorDefinitionActivationReport::from_request(
                                &request,
                                EditorDefinitionActivationStatus::Applied,
                                vec![format!("activated panel registry {registry_id}")],
                                Vec::new(),
                                false,
                            ),
                        );
                        activated += 1;
                    }
                    Err(diagnostic) => {
                        self.app.append_console_line(format!(
                            "[editor-definition] panel registry activation blocked: {}",
                            diagnostic.message
                        ));
                        self.app
                            .preserve_failed_editor_definition_activation(request.clone());
                        self.app.record_editor_definition_activation_report(
                            EditorDefinitionActivationReport::from_request(
                                &request,
                                EditorDefinitionActivationStatus::Failed,
                                vec!["panel registry activation blocked".to_string()],
                                vec![diagnostic],
                                true,
                            ),
                        );
                    }
                }
            }
            Ok(EditorDefinitionActivation::ToolSurfaceRegistryCatalogChanged {
                registry_id,
                registry,
            }) => {
                let composition = self.shell_state.composition_runtime().clone();
                match self
                    .shell_state
                    .active_editor_definitions_mut()
                    .install_tool_surface_registry(registry, &composition)
                {
                    Ok(()) => {
                        self.app.append_console_line(format!(
                            "[editor-definition] activated tool-surface registry {registry_id}"
                        ));
                        self.app.record_editor_definition_activation_report(
                            EditorDefinitionActivationReport::from_request(
                                &request,
                                EditorDefinitionActivationStatus::Applied,
                                vec![format!("activated tool-surface registry {registry_id}")],
                                Vec::new(),
                                false,
                            ),
                        );
                        activated += 1;
                    }
                    Err(diagnostic) => {
                        self.app.append_console_line(format!(
                            "[editor-definition] tool-surface registry activation blocked: {}",
                            diagnostic.message
                        ));
                        self.app
                            .preserve_failed_editor_definition_activation(request.clone());
                        self.app.record_editor_definition_activation_report(
                            EditorDefinitionActivationReport::from_request(
                                &request,
                                EditorDefinitionActivationStatus::Failed,
                                vec!["tool-surface registry activation blocked".to_string()],
                                vec![diagnostic],
                                true,
                            ),
                        );
                    }
                }
            }
            Ok(EditorDefinitionActivation::WorkspaceLayoutChanged {
                workspace_id,
                layout,
            }) => {
                let mut identities = editor_shell::WorkspaceIdentityAllocator::new();
                let legacy_workspace_id = identities.allocate_workspace_id();
                let result = editor_shell::form_workspace_state_from_definition_with_registry(
                    &layout,
                    legacy_workspace_id,
                    &mut identities,
                    self.app.workbench_host().tool_surface_registry(),
                )
                .map_err(|error| format!("authored layout formation failed: {error:?}"))
                .and_then(|workspace| {
                    editor_shell::import_legacy_workspace(
                        self.shell_state.active_workspace_profile_id(),
                        &workspace,
                    )
                    .map_err(|error| format!("composition import failed: {error:?}"))
                })
                .and_then(|runtime| {
                    self.shell_state
                        .install_composition_runtime(runtime)
                        .map_err(|error| format!("composition install failed: {error:?}"))
                });
                match result {
                    Ok(()) => {
                        self.app.append_console_line(format!(
                            "[editor-definition] activated composition layout {workspace_id}"
                        ));
                        self.app.record_editor_definition_activation_report(
                            EditorDefinitionActivationReport::from_request(
                                &request,
                                EditorDefinitionActivationStatus::Applied,
                                vec![format!("activated composition layout {workspace_id}")],
                                Vec::new(),
                                false,
                            ),
                        );
                        activated += 1;
                    }
                    Err(message) => {
                        let diagnostic = UiDefinitionDiagnostic::error(
                            editor_shell::EditorCompositionDiagnosticCode::LayoutActivationFailed
                                .as_str(),
                            message,
                        );
                        self.app
                            .preserve_failed_editor_definition_activation(request.clone());
                        self.app.record_editor_definition_activation_report(
                            EditorDefinitionActivationReport::from_request(
                                &request,
                                EditorDefinitionActivationStatus::Failed,
                                vec!["composition layout activation failed".to_string()],
                                vec![diagnostic],
                                true,
                            ),
                        );
                    }
                }
            }
            Err(diagnostics) => {
                for diagnostic in &diagnostics {
                    self.app.append_console_line(format!(
                        "[editor-definition] activation blocked: {}",
                        diagnostic.message
                    ));
                }
                self.app
                    .preserve_failed_editor_definition_activation(request.clone());
                self.app.record_editor_definition_activation_report(
                    EditorDefinitionActivationReport::from_request(
                        &request,
                        EditorDefinitionActivationStatus::Failed,
                        vec!["activation blocked by definition validation".to_string()],
                        diagnostics,
                        true,
                    ),
                );
            }
        }
        activated
    }

    fn apply_workbench_composition_package_activation(
        &mut self,
        request: PendingEditorDefinitionActivation,
        composition: EditorWorkbenchCompositionDefinition,
        profiles: Vec<EditorWorkspaceProfileDefinition>,
        layouts: Vec<EditorWorkspaceLayoutDefinition>,
    ) -> usize {
        let composition_id = composition.id.clone();
        let composition_label = composition.label.clone();
        let host = match authored_workbench_host_from_package(&composition, &profiles, &layouts) {
            Ok(host) => host,
            Err(diagnostic) => {
                return self.record_failed_workbench_package_activation(
                    request,
                    "workbench composition package activation blocked".to_string(),
                    diagnostic,
                );
            }
        };
        let default_profile_ref = host.default_workspace_profile_ref().clone();
        match self
            .shell_state
            .activate_workspace_profile_ref_with_registry(
                &default_profile_ref,
                host.workspace_profile_registry(),
                host.tool_surface_registry(),
            ) {
            Ok(_) => {
                self.app.replace_workbench_host(host);
                self.shell_state
                    .self_authoring_mut()
                    .record_applied_workbench_composition_payload(&request.payload);
                self.app
                    .prune_surface_sessions_for_composition(self.shell_state.composition_runtime());
                self.app.append_console_line(format!(
                    "[editor-definition] activated workbench composition package {composition_id}"
                ));
                self.app.record_editor_definition_activation_report(
                    EditorDefinitionActivationReport::from_request(
                        &request,
                        EditorDefinitionActivationStatus::Applied,
                        vec![format!(
                            "activated workbench composition package {composition_label}"
                        )],
                        Vec::new(),
                        false,
                    ),
                );
                1
            }
            Err(error) => {
                let diagnostic = UiDefinitionDiagnostic::error(
                    "editor.definition.activation.workbench_shell_state_blocked",
                    format!("workbench shell state activation blocked: {error}"),
                );
                self.record_failed_workbench_package_activation(
                    request,
                    "workbench shell state activation blocked".to_string(),
                    diagnostic,
                )
            }
        }
    }

    fn record_failed_workbench_package_activation(
        &mut self,
        request: PendingEditorDefinitionActivation,
        summary: String,
        diagnostic: UiDefinitionDiagnostic,
    ) -> usize {
        self.app.append_console_line(format!(
            "[editor-definition] {}: {}",
            summary, diagnostic.message
        ));
        self.app
            .preserve_failed_editor_definition_activation(request.clone());
        self.app.record_editor_definition_activation_report(
            EditorDefinitionActivationReport::from_request(
                &request,
                EditorDefinitionActivationStatus::Failed,
                vec![summary],
                vec![diagnostic],
                true,
            ),
        );
        0
    }
}

fn authored_workbench_host_from_package(
    composition: &EditorWorkbenchCompositionDefinition,
    profiles: &[EditorWorkspaceProfileDefinition],
    layouts: &[EditorWorkspaceLayoutDefinition],
) -> Result<RunenwerkWorkbenchHost, UiDefinitionDiagnostic> {
    let composition_document = authored_workbench_composition_document(composition);
    let diagnostics = validate_editor_definition_document(&composition_document);
    if editor_definition_has_blocking_diagnostics(&diagnostics) {
        return Err(UiDefinitionDiagnostic::error(
            "editor.definition.activation.workbench_composition_invalid",
            format!("workbench composition document is invalid: {diagnostics:?}"),
        ));
    }

    let package_documents = authored_workbench_package_documents(profiles, layouts);
    for document in &package_documents {
        let diagnostics = validate_editor_definition_document(document);
        if editor_definition_has_blocking_diagnostics(&diagnostics) {
            return Err(UiDefinitionDiagnostic::error(
                "editor.definition.activation.workbench_package_document_invalid",
                format!(
                    "workbench package document `{}` is invalid: {diagnostics:?}",
                    document.id.as_str()
                ),
            ));
        }
    }

    let composition_manifest = workbench_composition_manifest_from_definition(composition)
        .map_err(|error| {
            UiDefinitionDiagnostic::error(
                "editor.definition.activation.workbench_composition_manifest_blocked",
                format!("workbench composition manifest conversion blocked: {error}"),
            )
        })?;
    let profile_manifests = workspace_profile_manifests_from_authored_documents(
        composition.profile_refs.iter().map(String::as_str),
        package_documents.iter(),
    )
    .map_err(|error| {
        UiDefinitionDiagnostic::error(
            "editor.definition.activation.workbench_profile_manifest_blocked",
            format!("workbench profile manifest conversion blocked: {error}"),
        )
    })?;
    RunenwerkWorkbenchHost::authored(composition_manifest, profile_manifests).map_err(|error| {
        UiDefinitionDiagnostic::error(
            "editor.definition.activation.workbench_host_blocked",
            format!("workbench host activation blocked: {error}"),
        )
    })
}

fn authored_workbench_composition_document(
    composition: &EditorWorkbenchCompositionDefinition,
) -> EditorDefinitionDocument {
    EditorDefinitionDocument::current(
        EditorDefinitionId::from(composition.id.as_str()),
        composition.label.clone(),
        EditorDefinitionDocumentKind::WorkbenchComposition,
        EditorDefinitionDocumentContent::WorkbenchComposition(composition.clone()),
    )
}

fn authored_workbench_package_documents(
    profiles: &[EditorWorkspaceProfileDefinition],
    layouts: &[EditorWorkspaceLayoutDefinition],
) -> Vec<EditorDefinitionDocument> {
    profiles
        .iter()
        .map(|profile| {
            EditorDefinitionDocument::current(
                EditorDefinitionId::from(profile.id.as_str()),
                profile.label.clone(),
                EditorDefinitionDocumentKind::WorkspaceDefinition,
                EditorDefinitionDocumentContent::WorkspaceProfile(profile.clone()),
            )
        })
        .chain(layouts.iter().map(|layout| {
            EditorDefinitionDocument::current(
                EditorDefinitionId::from(layout.id.as_str()),
                layout.label.clone(),
                EditorDefinitionDocumentKind::WorkspaceDefinition,
                EditorDefinitionDocumentContent::WorkspaceLayout(layout.clone()),
            )
        }))
        .collect()
}

pub fn effective_shell_scale(scale_factor: f64) -> f32 {
    (scale_factor as f32).clamp(SHELL_SCALE_MIN, SHELL_SCALE_MAX) * SHELL_READABILITY_BUMP
}

pub fn scaled_shell_theme(theme: &ThemeTokens, scale_factor: f64) -> ThemeTokens {
    theme.scaled_by(effective_shell_scale(scale_factor))
}

#[derive(Debug, Default, Clone, runen_ecs::Component, runen_ecs::Resource)]
pub struct EditorInputBridgeState {
    pub last_mouse_position: (f32, f32),
    pub last_logged_picking_revision: u64,
    pub last_target_viewport: Option<ViewportId>,
    pub active_camera_viewport: Option<ViewportId>,
    pub pointer_owner: EditorPointerOwner,
    pub active_shortcut_catalog_active: bool,
    pub active_shortcut_signature: Vec<(String, String, String)>,
    pub active_shortcut_action_ids: Vec<String>,
    pub active_shortcut_commands: BTreeMap<String, crate::shell::KnownEditorCommand>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EditorPointerOwner {
    #[default]
    None,
    UiMiddleScroll,
    ViewportCamera {
        viewport_id: ViewportId,
        button: EditorCameraPointerButton,
    },
    ViewportTool {
        tool_surface_id: ToolSurfaceInstanceId,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorCameraPointerButton {
    Middle,
    Secondary,
}

#[derive(Debug, Clone, Copy, GpuUniform)]
pub struct EditorViewportSceneProductUniform {
    pub surface: [f32; 4],
    pub viewport: [f32; 4],
    pub camera_position: [f32; 4],
    pub camera_forward: [f32; 4],
    pub camera_right: [f32; 4],
    pub camera_up: [f32; 4],
    pub object_transform: [f32; 4],
    pub primitive_params_a: [f32; 4],
    pub primitive_params_b: [f32; 4],
    pub primitive_flags: [u32; 4],
    pub primitive_slot_transforms: [[f32; 4]; EDITOR_VIEWPORT_MAX_PRIMITIVE_INSTANCES],
    pub primitive_slot_params_a: [[f32; 4]; EDITOR_VIEWPORT_MAX_PRIMITIVE_INSTANCES],
    pub primitive_slot_params_b: [[f32; 4]; EDITOR_VIEWPORT_MAX_PRIMITIVE_INSTANCES],
    pub primitive_slot_flags: [[u32; 4]; EDITOR_VIEWPORT_MAX_PRIMITIVE_INSTANCES],
    pub model_mesh_flags: [u32; 4],
    pub model_mesh_region_rects: [[f32; 4]; EDITOR_VIEWPORT_MAX_MODEL_MESH_MATERIAL_REGIONS],
    pub model_mesh_region_flags: [[u32; 4]; EDITOR_VIEWPORT_MAX_MODEL_MESH_MATERIAL_REGIONS],
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EditorViewportPrimitiveInstance {
    pub entity_id: EntityId,
    pub pick_slot: u32,
    pub translation: Vec3Value,
    pub scale: Vec3Value,
    pub primitive_kind: EditorPrimitiveKind,
    pub box_half_extents: Vec3Value,
    pub sphere_radius: f32,
    pub capsule_radius: f32,
    pub capsule_half_height: f32,
    pub material_slot_index: u32,
    pub selected: bool,
    pub hovered: bool,
}

impl EditorViewportPrimitiveInstance {
    pub fn from_transform_and_primitive(
        entity_id: EntityId,
        transform: LocalTransform,
        primitive: EditorPrimitive,
        selected: bool,
        hovered: bool,
    ) -> Self {
        let safe_scale = Vec3Value::new(
            transform.scale.x.abs().max(0.0001),
            transform.scale.y.abs().max(0.0001),
            transform.scale.z.abs().max(0.0001),
        );
        let radial_scale = safe_scale.x.max(safe_scale.z);
        let sphere_scale = safe_scale.x.max(safe_scale.y).max(safe_scale.z);
        Self {
            entity_id,
            pick_slot: 0,
            translation: transform.translation,
            scale: safe_scale,
            primitive_kind: primitive.kind(),
            box_half_extents: Vec3Value::new(
                primitive.box_half_extents.x.max(0.05) * safe_scale.x,
                primitive.box_half_extents.y.max(0.05) * safe_scale.y,
                primitive.box_half_extents.z.max(0.05) * safe_scale.z,
            ),
            sphere_radius: primitive.sphere_radius.max(0.05) * sphere_scale,
            capsule_radius: primitive.capsule_radius.max(0.05) * radial_scale,
            capsule_half_height: primitive.capsule_half_height.max(0.05) * safe_scale.y,
            material_slot_index: 0,
            selected,
            hovered,
        }
    }

    pub fn with_material_slot_index(mut self, material_slot_index: u32) -> Self {
        self.material_slot_index = material_slot_index;
        self
    }

    pub fn shader_slot_transform(self) -> [f32; 4] {
        [
            self.translation.x,
            self.translation.y,
            self.translation.z,
            0.0,
        ]
    }

    pub fn shader_slot_params_a(self) -> [f32; 4] {
        [
            self.box_half_extents.x.max(0.05),
            self.box_half_extents.y.max(0.05),
            self.box_half_extents.z.max(0.05),
            self.sphere_radius.max(0.05),
        ]
    }

    pub fn shader_slot_params_b(self) -> [f32; 4] {
        [
            self.capsule_radius.max(0.05),
            self.capsule_half_height.max(0.05),
            0.0,
            0.0,
        ]
    }

    pub fn shader_slot_flags(self) -> [u32; 4] {
        [
            self.primitive_kind.as_u32(),
            self.pick_slot,
            u32::from(self.selected),
            self.material_slot_index,
        ]
    }
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct EditorViewportSceneRenderPacket {
    primitives: Vec<EditorViewportPrimitiveInstance>,
    omitted_primitive_count: usize,
}

impl EditorViewportSceneRenderPacket {
    pub fn from_primitives(
        primitives: impl IntoIterator<Item = EditorViewportPrimitiveInstance>,
    ) -> Self {
        let mut primitives = primitives.into_iter().collect::<Vec<_>>();
        primitives.sort_by_key(|primitive| primitive.entity_id.0);
        let omitted_primitive_count = primitives
            .len()
            .saturating_sub(EDITOR_VIEWPORT_MAX_PRIMITIVE_INSTANCES);
        primitives.truncate(EDITOR_VIEWPORT_MAX_PRIMITIVE_INSTANCES);
        for (index, primitive) in primitives.iter_mut().enumerate() {
            primitive.pick_slot = (index + 1) as u32;
        }
        Self {
            primitives,
            omitted_primitive_count,
        }
    }

    pub fn primitives(&self) -> &[EditorViewportPrimitiveInstance] {
        &self.primitives
    }

    pub fn len(&self) -> usize {
        self.primitives.len()
    }

    pub fn is_empty(&self) -> bool {
        self.primitives.is_empty()
    }

    pub fn omitted_primitive_count(&self) -> usize {
        self.omitted_primitive_count
    }

    pub fn has_overflow(&self) -> bool {
        self.omitted_primitive_count != 0
    }

    pub fn entity_for_pick_slot(&self, pick_slot: u32) -> Option<EntityId> {
        if pick_slot == 0 {
            return None;
        }

        self.primitives
            .iter()
            .find(|primitive| primitive.pick_slot == pick_slot)
            .map(|primitive| primitive.entity_id)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditorViewportModelMeshMaterialSelection {
    pub material_region: SceneModelMeshMaterialRegionSourceId,
    pub requested_slot_id: SceneMaterialSlotId,
    pub resolved_slot_id: SceneMaterialSlotId,
    pub material_table_index: u32,
    pub used_default_fallback: bool,
    prepared_selection: PreparedModelMeshMaterialSelection,
}

impl EditorViewportModelMeshMaterialSelection {
    pub fn new(
        material_region: SceneModelMeshMaterialRegionSourceId,
        resolution: SceneMaterialResolution,
    ) -> Result<Self, String> {
        let prepared_selection =
            prepared_model_mesh_material_selection(&material_region, resolution)?;
        Ok(Self {
            material_region,
            requested_slot_id: resolution.requested_slot_id,
            resolved_slot_id: resolution.resolved_slot_id,
            material_table_index: resolution.material_table_index,
            used_default_fallback: resolution.used_default_fallback,
            prepared_selection,
        })
    }

    pub fn prepared_selection(&self) -> &PreparedModelMeshMaterialSelection {
        &self.prepared_selection
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct EditorViewportModelMeshMaterialSelectionPacket {
    selections: Vec<EditorViewportModelMeshMaterialSelection>,
    diagnostics: Vec<SceneMaterialBindingDiagnostic>,
}

impl EditorViewportModelMeshMaterialSelectionPacket {
    pub fn from_model_mesh_regions(
        assignments: &SceneMaterialAssignmentState,
        material_regions: impl IntoIterator<Item = SceneModelMeshMaterialRegionSourceId>,
    ) -> Self {
        let mut selections = Vec::new();
        let mut diagnostics = Vec::new();
        for material_region in material_regions {
            let (resolution, mut region_diagnostics) =
                assignments.resolve_material_binding_for_model_mesh_region(&material_region);
            diagnostics.append(&mut region_diagnostics);
            match EditorViewportModelMeshMaterialSelection::new(material_region.clone(), resolution)
            {
                Ok(selection) => selections.push(selection),
                Err(message) => diagnostics.push(model_mesh_material_selection_diagnostic(
                    material_region,
                    message,
                )),
            }
        }
        selections.sort_by(|left, right| {
            left.material_region
                .cmp(&right.material_region)
                .then_with(|| left.material_table_index.cmp(&right.material_table_index))
        });
        Self {
            selections,
            diagnostics,
        }
    }

    pub fn from_blocking_diagnostics(
        diagnostics: impl IntoIterator<Item = SceneMaterialBindingDiagnostic>,
    ) -> Self {
        Self {
            selections: Vec::new(),
            diagnostics: diagnostics.into_iter().collect(),
        }
    }

    pub fn selections(&self) -> &[EditorViewportModelMeshMaterialSelection] {
        &self.selections
    }

    pub fn diagnostics(&self) -> &[SceneMaterialBindingDiagnostic] {
        &self.diagnostics
    }

    pub fn has_blocking_diagnostics(&self) -> bool {
        !self.diagnostics.is_empty()
    }

    pub fn prepared_material_selections(&self) -> Vec<PreparedModelMeshMaterialSelection> {
        self.selections
            .iter()
            .map(|selection| selection.prepared_selection().clone())
            .collect()
    }
}

pub fn model_mesh_material_selection_diagnostic(
    material_region: SceneModelMeshMaterialRegionSourceId,
    message: impl Into<String>,
) -> SceneMaterialBindingDiagnostic {
    SceneMaterialBindingDiagnostic::new(
        SceneMaterialBindingDiagnosticCode::InvalidMaterialProduct,
        SceneMaterialBindingDiagnosticSubject::ModelMeshMaterialRegion(material_region),
        message,
    )
}

fn prepared_model_mesh_material_selection(
    material_region: &SceneModelMeshMaterialRegionSourceId,
    resolution: SceneMaterialResolution,
) -> Result<PreparedModelMeshMaterialSelection, String> {
    let source = prepared_model_mesh_source_identity(&material_region.model_mesh_source_id)?;
    let region = PreparedModelMeshMaterialRegionIdentity::new(
        source,
        material_region.material_region_id.key().to_string(),
    )
    .map_err(|error| error.to_string())?;
    PreparedModelMeshMaterialSelection::new(
        region,
        resolution.requested_slot_id.raw(),
        resolution.resolved_slot_id.raw(),
        resolution.material_table_index,
        resolution.used_default_fallback,
    )
    .map_err(|error| error.to_string())
}

fn prepared_model_mesh_source_identity(
    source: &SceneModelMeshSourceId,
) -> Result<PreparedModelMeshMaterialSourceIdentity, String> {
    let mut prepared =
        PreparedModelMeshMaterialSourceIdentity::new(source.asset_id.raw(), source.source_id.raw())
            .map_err(|error| error.to_string())?;
    if let Some(revision_id) = source.source_revision_id {
        prepared = prepared.with_source_revision_id(revision_id.raw());
    }
    if let Some(revision) = &source.source_revision {
        prepared = prepared.with_source_revision(revision.clone());
    }
    Ok(prepared)
}

#[derive(Debug, Clone, Copy)]
pub struct EditorViewportBranchTraceSnapshot {
    pub viewport_bounds_px: (f32, f32, f32, f32),
    pub viewport_valid: bool,
    pub shader_loaded: bool,
    pub debug_stage: EditorViewportDebugStage,
    pub has_primitive: bool,
    pub primitive_kind: EditorPrimitiveKind,
    pub primitive_translation: Vec3Value,
    pub surface: [f32; 4],
    pub viewport: [f32; 4],
    pub camera_position: [f32; 4],
    pub camera_forward: [f32; 4],
    pub camera_right: [f32; 4],
    pub camera_up: [f32; 4],
    pub primitive_params_a: [f32; 4],
    pub primitive_params_b: [f32; 4],
    pub primitive_flags: [u32; 4],
    pub model_mesh_flags: [u32; 4],
    pub first_model_mesh_region_flags: [u32; 4],
}

impl EditorViewportBranchTraceSnapshot {
    pub fn approx_eq(&self, other: &Self) -> bool {
        approx_bounds_eq(self.viewport_bounds_px, other.viewport_bounds_px)
            && self.viewport_valid == other.viewport_valid
            && self.shader_loaded == other.shader_loaded
            && self.debug_stage == other.debug_stage
            && self.has_primitive == other.has_primitive
            && self.primitive_kind == other.primitive_kind
            && approx_vec3(self.primitive_translation, other.primitive_translation)
            && approx_vec4(self.surface, other.surface)
            && approx_vec4(self.viewport, other.viewport)
            && approx_vec4(self.camera_position, other.camera_position)
            && approx_vec4(self.camera_forward, other.camera_forward)
            && approx_vec4(self.camera_right, other.camera_right)
            && approx_vec4(self.camera_up, other.camera_up)
            && approx_vec4(self.primitive_params_a, other.primitive_params_a)
            && approx_vec4(self.primitive_params_b, other.primitive_params_b)
            && self.primitive_flags == other.primitive_flags
            && self.model_mesh_flags == other.model_mesh_flags
            && self.first_model_mesh_region_flags == other.first_model_mesh_region_flags
    }

    pub fn summary_line(self) -> String {
        format!(
            "stage={}({}) valid={} shader_loaded={} has_primitive={} kind={:?} bounds=({:.1},{:.1},{:.1},{:.1}) viewport=({:.1},{:.1},{:.1},{:.1}) surface=({:.0},{:.0}) obj=({:.2},{:.2},{:.2}) params_a=({:.2},{:.2},{:.2},{:.2}) params_b=({:.2},{:.2},{:.2},{:.2}) flags={:?} model_mesh_flags={:?} model_mesh_first={:?} cam_pos=({:.2},{:.2},{:.2}) fov={:.3} cam_fwd=({:.3},{:.3},{:.3}) cam_right=({:.3},{:.3},{:.3}) cam_up=({:.3},{:.3},{:.3})",
            self.debug_stage.label(),
            self.debug_stage.as_u32(),
            self.viewport_valid,
            self.shader_loaded,
            self.has_primitive,
            self.primitive_kind,
            self.viewport_bounds_px.0,
            self.viewport_bounds_px.1,
            self.viewport_bounds_px.2,
            self.viewport_bounds_px.3,
            self.viewport[0],
            self.viewport[1],
            self.viewport[2],
            self.viewport[3],
            self.surface[0],
            self.surface[1],
            self.primitive_translation.x,
            self.primitive_translation.y,
            self.primitive_translation.z,
            self.primitive_params_a[0],
            self.primitive_params_a[1],
            self.primitive_params_a[2],
            self.primitive_params_a[3],
            self.primitive_params_b[0],
            self.primitive_params_b[1],
            self.primitive_params_b[2],
            self.primitive_params_b[3],
            self.primitive_flags,
            self.model_mesh_flags,
            self.first_model_mesh_region_flags,
            self.camera_position[0],
            self.camera_position[1],
            self.camera_position[2],
            self.camera_position[3],
            self.camera_forward[0],
            self.camera_forward[1],
            self.camera_forward[2],
            self.camera_right[0],
            self.camera_right[1],
            self.camera_right[2],
            self.camera_up[0],
            self.camera_up[1],
            self.camera_up[2],
        )
    }
}

#[derive(Debug, Clone, runen_ecs::Component, runen_ecs::Resource)]
pub struct EditorViewportRenderState {
    pub viewport_bounds_px: (f32, f32, f32, f32),
    pub effective_shell_scale: f32,
    pub viewport_valid: bool,
    pub shader_loaded: bool,
    pub debug_stage: EditorViewportDebugStage,
    pub root_background_opaque: bool,
    pub has_primitive: bool,
    pub primitive_kind: EditorPrimitiveKind,
    pub primitive_translation: Vec3Value,
    pub box_half_extents: Vec3Value,
    pub sphere_radius: f32,
    pub capsule_radius: f32,
    pub capsule_half_height: f32,
    pub scene_packet: EditorViewportSceneRenderPacket,
    pub model_mesh_material_selection_packet: EditorViewportModelMeshMaterialSelectionPacket,
    pub camera_settings: ViewportCameraSettings,
    pub camera: EditorViewportCamera,
    pub camera_fov_y_radians: f32,
    pub visibility_contradiction_active: bool,
    pub last_reported_viewport_bounds_px: Option<(f32, f32, f32, f32)>,
    pub last_reported_shell_scale: Option<f32>,
    pub last_reported_debug_state: Option<(EditorViewportDebugStage, bool, bool, bool, bool)>,
    pub last_reported_branch_trace: Option<EditorViewportBranchTraceSnapshot>,
}

impl Default for EditorViewportRenderState {
    fn default() -> Self {
        Self {
            viewport_bounds_px: (0.0, 0.0, 0.0, 0.0),
            effective_shell_scale: 1.0,
            viewport_valid: false,
            shader_loaded: false,
            debug_stage: EditorViewportDebugStage::Scene,
            root_background_opaque: false,
            has_primitive: false,
            primitive_kind: EditorPrimitiveKind::Box,
            primitive_translation: Vec3Value::zero(),
            box_half_extents: Vec3Value::new(0.5, 0.5, 0.5),
            sphere_radius: 0.6,
            capsule_radius: 0.35,
            capsule_half_height: 0.75,
            scene_packet: EditorViewportSceneRenderPacket::default(),
            model_mesh_material_selection_packet:
                EditorViewportModelMeshMaterialSelectionPacket::default(),
            camera_settings: ViewportCameraSettings::default(),
            camera: editor_viewport_camera(),
            camera_fov_y_radians: editor_viewport_camera_fov_y_radians(),
            visibility_contradiction_active: false,
            last_reported_viewport_bounds_px: None,
            last_reported_shell_scale: None,
            last_reported_debug_state: None,
            last_reported_branch_trace: None,
        }
    }
}

impl EditorViewportRenderState {
    pub fn set_viewport_bounds(&mut self, bounds: (f32, f32, f32, f32)) -> bool {
        let changed = !approx_bounds_eq(self.viewport_bounds_px, bounds);
        self.viewport_bounds_px = bounds;
        changed
    }

    pub fn set_effective_shell_scale(&mut self, scale: f32) -> bool {
        let changed = (self.effective_shell_scale - scale).abs() > f32::EPSILON;
        self.effective_shell_scale = scale;
        changed
    }

    pub fn set_debug_stage(&mut self, stage: EditorViewportDebugStage) -> bool {
        let changed = self.debug_stage != stage;
        self.debug_stage = stage;
        changed
    }

    pub fn set_root_background_opaque(&mut self, enabled: bool) -> bool {
        let changed = self.root_background_opaque != enabled;
        self.root_background_opaque = enabled;
        changed
    }

    pub fn from_viewport_settings(settings: ViewportRuntimeSettings) -> Self {
        let mut state = Self::default();
        state.apply_viewport_settings(settings);
        state
    }

    pub fn viewport_settings(
        &self,
        selected_primary_product_id: Option<editor_viewport::ExpressionProductId>,
        field_visualizer_settings: ViewportFieldVisualizerSettings,
    ) -> ViewportRuntimeSettings {
        ViewportRuntimeSettings {
            camera: self.camera_settings,
            debug_stage: self.debug_stage,
            root_background_opaque: self.root_background_opaque,
            selected_primary_product_id,
            field_visualizer_settings,
        }
    }

    pub fn apply_viewport_settings(&mut self, settings: ViewportRuntimeSettings) -> bool {
        let camera_changed = self.set_camera_settings(settings.camera);
        let debug_changed = self.set_debug_stage(settings.debug_stage);
        let root_changed = self.set_root_background_opaque(settings.root_background_opaque);
        camera_changed || debug_changed || root_changed
    }

    pub fn set_camera_settings(&mut self, settings: ViewportCameraSettings) -> bool {
        let settings = sanitized_camera_settings(settings);
        let changed = self.camera_settings != settings
            || (self.camera_fov_y_radians - settings.fov_y_radians).abs() > f32::EPSILON;
        self.camera_settings = settings;
        self.camera = editor_viewport_camera_from_settings(settings);
        self.camera_fov_y_radians = settings.fov_y_radians;
        changed
    }

    pub fn reset_camera(&mut self) -> bool {
        self.set_camera_settings(ViewportCameraSettings::default())
    }

    pub fn focus_camera_on(&mut self, orbit_target: [f32; 3]) -> bool {
        let mut settings = self.camera_settings;
        settings.orbit_target = orbit_target;
        self.set_camera_settings(settings)
    }

    pub fn orbit_camera(&mut self, delta: UiVector) -> bool {
        if delta == UiVector::ZERO {
            return false;
        }
        let mut settings = self.camera_settings;
        settings.yaw_radians += delta.x * CAMERA_ORBIT_SENSITIVITY;
        settings.pitch_radians = (settings.pitch_radians + delta.y * CAMERA_ORBIT_SENSITIVITY)
            .clamp(-CAMERA_MAX_PITCH_RADIANS, CAMERA_MAX_PITCH_RADIANS);
        self.set_camera_settings(settings)
    }

    pub fn pan_camera(&mut self, delta: UiVector) -> bool {
        if delta == UiVector::ZERO {
            return false;
        }
        let distance = self
            .camera_settings
            .distance
            .clamp(CAMERA_MIN_DISTANCE, CAMERA_MAX_DISTANCE);
        let amount = distance * CAMERA_PAN_SENSITIVITY;
        let world_delta = (-self.camera.right * delta.x + self.camera.up * delta.y) * amount;
        let mut settings = self.camera_settings;
        settings.orbit_target[0] += world_delta.x;
        settings.orbit_target[1] += world_delta.y;
        settings.orbit_target[2] += world_delta.z;
        self.set_camera_settings(settings)
    }

    pub fn zoom_camera(&mut self, scroll_delta: f32) -> bool {
        if scroll_delta.abs() <= f32::EPSILON {
            return false;
        }
        let mut settings = self.camera_settings;
        settings.distance = (settings.distance * (-scroll_delta * CAMERA_ZOOM_SENSITIVITY).exp())
            .clamp(CAMERA_MIN_DISTANCE, CAMERA_MAX_DISTANCE);
        self.set_camera_settings(settings)
    }

    pub fn should_report_bounds_change(&mut self) -> bool {
        let should_report = match self.last_reported_viewport_bounds_px {
            Some(last) => !approx_bounds_eq(last, self.viewport_bounds_px),
            None => true,
        };
        if should_report {
            self.last_reported_viewport_bounds_px = Some(self.viewport_bounds_px);
        }
        should_report
    }

    pub fn should_report_scale_change(&mut self) -> bool {
        let should_report = match self.last_reported_shell_scale {
            Some(last) => (last - self.effective_shell_scale).abs() > f32::EPSILON,
            None => true,
        };
        if should_report {
            self.last_reported_shell_scale = Some(self.effective_shell_scale);
        }
        should_report
    }

    pub fn set_primitive(&mut self, translation: Vec3Value, primitive: EditorPrimitive) {
        let transform = LocalTransform::from_translation(translation);
        let instance = EditorViewportPrimitiveInstance::from_transform_and_primitive(
            EntityId(0),
            transform,
            primitive,
            false,
            false,
        );
        self.set_scene_packet(EditorViewportSceneRenderPacket::from_primitives([instance]));
    }

    pub fn set_scene_packet(&mut self, packet: EditorViewportSceneRenderPacket) {
        self.has_primitive = !packet.is_empty();
        self.scene_packet = packet;
        self.sync_first_primitive_mirror();
    }

    pub fn set_model_mesh_material_selection_packet(
        &mut self,
        packet: EditorViewportModelMeshMaterialSelectionPacket,
    ) {
        self.model_mesh_material_selection_packet = packet;
    }

    pub fn clear_primitive(&mut self) {
        self.has_primitive = false;
        self.scene_packet = EditorViewportSceneRenderPacket::default();
        self.sync_first_primitive_mirror();
    }

    fn sync_first_primitive_mirror(&mut self) {
        let Some(first) = self.scene_packet.primitives().first().copied() else {
            self.primitive_kind = EditorPrimitiveKind::Box;
            self.primitive_translation = Vec3Value::zero();
            self.box_half_extents = Vec3Value::new(0.5, 0.5, 0.5);
            self.sphere_radius = 0.6;
            self.capsule_radius = 0.35;
            self.capsule_half_height = 0.75;
            return;
        };

        self.primitive_kind = first.primitive_kind;
        self.primitive_translation = first.translation;
        self.box_half_extents = first.box_half_extents;
        self.sphere_radius = first.sphere_radius;
        self.capsule_radius = first.capsule_radius;
        self.capsule_half_height = first.capsule_half_height;
    }

    pub fn update_visibility_diagnostics(&mut self, viewport_valid: bool, shader_loaded: bool) {
        self.viewport_valid = viewport_valid;
        self.shader_loaded = shader_loaded;
    }

    pub fn scene_should_be_invisible(&self) -> bool {
        self.debug_stage == EditorViewportDebugStage::Scene
            && (!self.viewport_valid || !self.shader_loaded || !self.has_primitive)
    }

    pub fn should_report_visibility_contradiction(&mut self, contradiction_active: bool) -> bool {
        let should_report = contradiction_active && !self.visibility_contradiction_active;
        self.visibility_contradiction_active = contradiction_active;
        should_report
    }

    pub fn should_report_debug_state_change(&mut self) -> bool {
        let next = (
            self.debug_stage,
            self.root_background_opaque,
            self.viewport_valid,
            self.shader_loaded,
            self.has_primitive,
        );
        let changed = self.last_reported_debug_state != Some(next);
        if changed {
            self.last_reported_debug_state = Some(next);
        }
        changed
    }

    pub fn branch_trace_snapshot(&self, surface: (u32, u32)) -> EditorViewportBranchTraceSnapshot {
        let uniform = self.compose_scene_product_uniform(surface);
        EditorViewportBranchTraceSnapshot {
            viewport_bounds_px: self.viewport_bounds_px,
            viewport_valid: self.viewport_valid,
            shader_loaded: self.shader_loaded,
            debug_stage: self.debug_stage,
            has_primitive: self.has_primitive,
            primitive_kind: self.primitive_kind,
            primitive_translation: self.primitive_translation,
            surface: uniform.surface,
            viewport: uniform.viewport,
            camera_position: uniform.camera_position,
            camera_forward: uniform.camera_forward,
            camera_right: uniform.camera_right,
            camera_up: uniform.camera_up,
            primitive_params_a: uniform.primitive_params_a,
            primitive_params_b: uniform.primitive_params_b,
            primitive_flags: uniform.primitive_flags,
            model_mesh_flags: uniform.model_mesh_flags,
            first_model_mesh_region_flags: uniform.model_mesh_region_flags[0],
        }
    }

    pub fn should_report_branch_trace_change(
        &mut self,
        snapshot: EditorViewportBranchTraceSnapshot,
    ) -> bool {
        let changed = match self.last_reported_branch_trace {
            Some(previous) => !previous.approx_eq(&snapshot),
            None => true,
        };
        if changed {
            self.last_reported_branch_trace = Some(snapshot);
        }
        changed
    }

    pub fn compose_scene_product_uniform(
        &self,
        surface: (u32, u32),
    ) -> EditorViewportSceneProductUniform {
        let width = surface.0.max(1) as f32;
        let height = surface.1.max(1) as f32;
        let camera = self.camera;
        let mut primitive_slot_transforms = [[0.0; 4]; EDITOR_VIEWPORT_MAX_PRIMITIVE_INSTANCES];
        let mut primitive_slot_params_a = [[0.0; 4]; EDITOR_VIEWPORT_MAX_PRIMITIVE_INSTANCES];
        let mut primitive_slot_params_b = [[0.0; 4]; EDITOR_VIEWPORT_MAX_PRIMITIVE_INSTANCES];
        let mut primitive_slot_flags = [[0; 4]; EDITOR_VIEWPORT_MAX_PRIMITIVE_INSTANCES];
        let mut model_mesh_region_rects =
            [[0.0; 4]; EDITOR_VIEWPORT_MAX_MODEL_MESH_MATERIAL_REGIONS];
        let mut model_mesh_region_flags = [[0; 4]; EDITOR_VIEWPORT_MAX_MODEL_MESH_MATERIAL_REGIONS];
        for (index, primitive) in self
            .scene_packet
            .primitives()
            .iter()
            .take(EDITOR_VIEWPORT_MAX_PRIMITIVE_INSTANCES)
            .enumerate()
        {
            primitive_slot_transforms[index] = primitive.shader_slot_transform();
            primitive_slot_params_a[index] = primitive.shader_slot_params_a();
            primitive_slot_params_b[index] = primitive.shader_slot_params_b();
            primitive_slot_flags[index] = primitive.shader_slot_flags();
        }
        let primitive_count = self
            .scene_packet
            .len()
            .min(EDITOR_VIEWPORT_MAX_PRIMITIVE_INSTANCES) as u32;
        let model_mesh_region_count = self
            .model_mesh_material_selection_packet
            .selections()
            .len()
            .min(EDITOR_VIEWPORT_MAX_MODEL_MESH_MATERIAL_REGIONS);
        let model_mesh_region_omitted_count = self
            .model_mesh_material_selection_packet
            .selections()
            .len()
            .saturating_sub(EDITOR_VIEWPORT_MAX_MODEL_MESH_MATERIAL_REGIONS);
        write_model_mesh_material_region_slots(
            self.model_mesh_material_selection_packet.selections(),
            &mut model_mesh_region_rects,
            &mut model_mesh_region_flags,
        );

        EditorViewportSceneProductUniform {
            surface: [width, height, 1.0 / width, 1.0 / height],
            viewport: [0.0, 0.0, width, height],
            camera_position: [
                camera.position.x,
                camera.position.y,
                camera.position.z,
                self.camera_fov_y_radians,
            ],
            camera_forward: [camera.forward.x, camera.forward.y, camera.forward.z, 0.0],
            camera_right: [camera.right.x, camera.right.y, camera.right.z, 0.0],
            camera_up: [camera.up.x, camera.up.y, camera.up.z, 0.0],
            object_transform: [
                self.primitive_translation.x,
                self.primitive_translation.y,
                self.primitive_translation.z,
                0.0,
            ],
            primitive_params_a: [
                self.box_half_extents.x.max(0.05),
                self.box_half_extents.y.max(0.05),
                self.box_half_extents.z.max(0.05),
                self.sphere_radius.max(0.05),
            ],
            primitive_params_b: [
                self.capsule_radius.max(0.05),
                self.capsule_half_height.max(0.05),
                0.0,
                0.0,
            ],
            primitive_flags: [
                self.primitive_kind.as_u32(),
                primitive_count,
                self.debug_stage.as_u32(),
                if self.root_background_opaque { 1 } else { 0 },
            ],
            primitive_slot_transforms,
            primitive_slot_params_a,
            primitive_slot_params_b,
            primitive_slot_flags,
            model_mesh_flags: [
                model_mesh_region_count as u32,
                model_mesh_region_omitted_count as u32,
                0,
                0,
            ],
            model_mesh_region_rects,
            model_mesh_region_flags,
        }
    }

    pub fn compose_scene_product_uniform_bytes(&self, surface: (u32, u32)) -> Vec<u8> {
        let raw = self.compose_scene_product_uniform(surface).to_gpu();
        engine::plugins::render::bytemuck::bytes_of(&raw).to_vec()
    }
}

fn write_model_mesh_material_region_slots(
    selections: &[EditorViewportModelMeshMaterialSelection],
    rects: &mut [[f32; 4]; EDITOR_VIEWPORT_MAX_MODEL_MESH_MATERIAL_REGIONS],
    flags: &mut [[u32; 4]; EDITOR_VIEWPORT_MAX_MODEL_MESH_MATERIAL_REGIONS],
) {
    let count = selections
        .len()
        .min(EDITOR_VIEWPORT_MAX_MODEL_MESH_MATERIAL_REGIONS);
    if count == 0 {
        return;
    }

    let count_f = count as f32;
    let half_width = (0.42 / count_f).clamp(0.055, 0.18);
    for (index, selection) in selections.iter().take(count).enumerate() {
        let center_x = (index as f32 + 0.5) / count_f;
        rects[index] = [center_x, 0.56, half_width, 0.22];
        flags[index] = [
            selection.material_table_index,
            (index + 1) as u32,
            u32::from(selection.used_default_fallback),
            0,
        ];
    }
}

fn approx_bounds_eq(a: (f32, f32, f32, f32), b: (f32, f32, f32, f32)) -> bool {
    (a.0 - b.0).abs() <= VIEWPORT_BOUNDS_EPSILON
        && (a.1 - b.1).abs() <= VIEWPORT_BOUNDS_EPSILON
        && (a.2 - b.2).abs() <= VIEWPORT_BOUNDS_EPSILON
        && (a.3 - b.3).abs() <= VIEWPORT_BOUNDS_EPSILON
}

fn approx_f32(a: f32, b: f32) -> bool {
    (a - b).abs() <= BRANCH_TRACE_FLOAT_EPSILON
}

fn approx_vec3(a: Vec3Value, b: Vec3Value) -> bool {
    approx_f32(a.x, b.x) && approx_f32(a.y, b.y) && approx_f32(a.z, b.z)
}

fn approx_vec4(a: [f32; 4], b: [f32; 4]) -> bool {
    approx_f32(a[0], b[0])
        && approx_f32(a[1], b[1])
        && approx_f32(a[2], b[2])
        && approx_f32(a[3], b[3])
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EditorViewportCamera {
    pub position: Vec3,
    pub forward: Vec3,
    pub right: Vec3,
    pub up: Vec3,
}

pub fn editor_viewport_camera() -> EditorViewportCamera {
    editor_viewport_camera_from_settings(ViewportCameraSettings::default())
}

pub fn editor_viewport_camera_from_settings(
    settings: ViewportCameraSettings,
) -> EditorViewportCamera {
    let settings = sanitized_camera_settings(settings);
    let target = vec3(
        settings.orbit_target[0],
        settings.orbit_target[1],
        settings.orbit_target[2],
    );
    let (sin_yaw, cos_yaw) = settings.yaw_radians.sin_cos();
    let (sin_pitch, cos_pitch) = settings.pitch_radians.sin_cos();
    let offset = vec3(cos_pitch * cos_yaw, sin_pitch, cos_pitch * sin_yaw) * settings.distance;
    let position = target + offset;
    let world_up = Vec3::Y;
    let forward = (target - position).normalize_or_zero();
    let mut right = forward.cross(world_up).normalize_or_zero();
    if right.length_squared() <= f32::EPSILON {
        right = Vec3::X;
    }
    let up = right.cross(forward).normalize_or_zero();

    EditorViewportCamera {
        position,
        forward,
        right,
        up,
    }
}

pub fn editor_viewport_camera_fov_y_radians() -> f32 {
    ViewportCameraSettings::default().fov_y_radians
}

fn sanitized_camera_settings(settings: ViewportCameraSettings) -> ViewportCameraSettings {
    if !settings.is_valid() {
        return ViewportCameraSettings::default();
    }
    ViewportCameraSettings {
        orbit_target: settings.orbit_target,
        distance: settings
            .distance
            .clamp(CAMERA_MIN_DISTANCE, CAMERA_MAX_DISTANCE),
        yaw_radians: settings.yaw_radians,
        pitch_radians: settings
            .pitch_radians
            .clamp(-CAMERA_MAX_PITCH_RADIANS, CAMERA_MAX_PITCH_RADIANS),
        fov_y_radians: settings
            .fov_y_radians
            .clamp(1.0_f32.to_radians(), 175.0_f32.to_radians()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use asset::{asset_id, asset_source_id, asset_source_revision_id};
    use editor_scene::{
        SceneMaterialPalette, SceneMaterialSlot, SceneMeshMaterialRegionId,
        SceneModelMeshMaterialSlotAssignment,
    };
    use ui_theme::UiColor;

    #[test]
    fn editor_host_resource_apply_theme_replaces_runtime_theme() {
        let mut host = EditorHostResource::default();
        let theme = ThemeTokens {
            accent: UiColor::new(0.2, 0.4, 1.0, 1.0),
            ..ThemeTokens::default()
        };

        host.apply_theme(theme.clone());

        assert_eq!(host.theme, theme);
    }

    #[test]
    fn material_lab_workbench_host_resource_bootstraps_material_profile() {
        let host = EditorHostResource::material_lab_workbench();

        assert_eq!(
            host.app.workbench_host().composition(),
            crate::shell::RunenwerkWorkbenchComposition::MaterialLab
        );
        assert_eq!(
            host.shell_state.active_workspace_profile_id(),
            MATERIAL_WORKSPACE_PROFILE_ID
        );
        assert_eq!(
            host.shell_state.open_workspace_profile_ids(),
            &[MATERIAL_WORKSPACE_PROFILE_ID]
        );
        assert!(matches!(
            host.app.runtime().session().active_document_descriptor(),
            Some(editor_core::DocumentDescriptor {
                kind: editor_core::DocumentKind::MaterialGraph,
                ..
            })
        ));
    }

    #[test]
    fn ui_designer_workbench_host_resource_bootstraps_editor_design_profile() {
        let host = EditorHostResource::ui_designer_workbench();

        assert_eq!(
            host.app.workbench_host().composition(),
            crate::shell::RunenwerkWorkbenchComposition::UiDesigner
        );
        assert_eq!(
            host.shell_state.active_workspace_profile_id(),
            EDITOR_DESIGN_WORKSPACE_PROFILE_ID
        );
        assert_eq!(
            host.shell_state.open_workspace_profile_ids(),
            &[EDITOR_DESIGN_WORKSPACE_PROFILE_ID]
        );
    }

    #[test]
    fn scene_product_uniform_uses_target_local_viewport_for_camera_aspect() {
        let mut state = EditorViewportRenderState::default();
        state.set_viewport_bounds((40.0, 50.0, 320.0, 180.0));

        let uniform = state.compose_scene_product_uniform((1280, 720));

        assert_eq!(uniform.viewport, [0.0, 0.0, 1280.0, 720.0]);
        assert_eq!(state.viewport_bounds_px, (40.0, 50.0, 320.0, 180.0));
    }

    #[test]
    fn scene_product_uniform_uses_viewport_owned_camera_state() {
        let state = EditorViewportRenderState {
            camera_fov_y_radians: 42.0_f32.to_radians(),
            ..Default::default()
        };

        let uniform = state.compose_scene_product_uniform((1280, 720));

        assert_eq!(uniform.viewport, [0.0, 0.0, 1280.0, 720.0]);
        assert_eq!(uniform.camera_position[3], 42.0_f32.to_radians());
    }

    #[test]
    fn scene_packet_sorts_instances_and_serializes_uniform_slots() {
        let mut sphere = EditorPrimitive::default();
        sphere.set_kind(EditorPrimitiveKind::Sphere);
        sphere.sphere_radius = 1.25;
        let box_primitive = EditorPrimitive {
            box_half_extents: Vec3Value::new(0.25, 0.5, 0.75),
            ..Default::default()
        };

        let packet = EditorViewportSceneRenderPacket::from_primitives([
            EditorViewportPrimitiveInstance::from_transform_and_primitive(
                editor_core::EntityId(20),
                LocalTransform::from_translation(Vec3Value::new(2.0, 0.0, 0.0)),
                sphere,
                false,
                true,
            ),
            EditorViewportPrimitiveInstance::from_transform_and_primitive(
                editor_core::EntityId(4),
                LocalTransform::from_translation(Vec3Value::new(-1.0, 0.0, 0.0)),
                box_primitive,
                true,
                false,
            ),
        ]);

        let mut state = EditorViewportRenderState::default();
        state.set_scene_packet(packet);
        let uniform = state.compose_scene_product_uniform((1280, 720));

        assert_eq!(uniform.primitive_flags[1], 2);
        assert_eq!(uniform.primitive_slot_flags[0], [0, 1, 1, 0]);
        assert_eq!(uniform.primitive_slot_flags[1], [1, 2, 0, 0]);
        assert_eq!(uniform.primitive_slot_transforms[0], [-1.0, 0.0, 0.0, 0.0]);
        assert_eq!(uniform.primitive_slot_params_a[0], [0.25, 0.5, 0.75, 0.6]);
        assert_eq!(uniform.primitive_slot_params_a[1][3], 1.25);
        assert_eq!(
            state.scene_packet.entity_for_pick_slot(1),
            Some(EntityId(4))
        );
        assert_eq!(
            state.scene_packet.entity_for_pick_slot(2),
            Some(EntityId(20))
        );
    }

    #[test]
    fn viewport_scene_packet_reports_uniform_slot_overflow() {
        let primitives = (0..EDITOR_VIEWPORT_MAX_PRIMITIVE_INSTANCES + 2).map(|index| {
            EditorViewportPrimitiveInstance::from_transform_and_primitive(
                editor_core::EntityId(index as u64),
                LocalTransform::from_translation(Vec3Value::new(index as f32, 0.0, 0.0)),
                EditorPrimitive::default(),
                false,
                false,
            )
        });

        let packet = EditorViewportSceneRenderPacket::from_primitives(primitives);
        let mut state = EditorViewportRenderState::default();
        state.set_scene_packet(packet);
        let uniform = state.compose_scene_product_uniform((1280, 720));

        assert_eq!(
            state.scene_packet.len(),
            EDITOR_VIEWPORT_MAX_PRIMITIVE_INSTANCES
        );
        assert!(state.scene_packet.has_overflow());
        assert_eq!(state.scene_packet.omitted_primitive_count(), 2);
        assert_eq!(
            uniform.primitive_flags[1],
            EDITOR_VIEWPORT_MAX_PRIMITIVE_INSTANCES as u32
        );
        assert_eq!(
            uniform.primitive_slot_flags[EDITOR_VIEWPORT_MAX_PRIMITIVE_INSTANCES - 1][1],
            EDITOR_VIEWPORT_MAX_PRIMITIVE_INSTANCES as u32
        );
    }

    #[test]
    fn viewport_scene_packet_pick_slots_preserve_full_entity_identity() {
        let large_entity = editor_core::EntityId(u64::from(u32::MAX) + 99);
        let instance = EditorViewportPrimitiveInstance::from_transform_and_primitive(
            large_entity,
            LocalTransform::default(),
            EditorPrimitive::default(),
            false,
            false,
        );

        let packet = EditorViewportSceneRenderPacket::from_primitives([instance]);
        let mut state = EditorViewportRenderState::default();
        state.set_scene_packet(packet);
        let uniform = state.compose_scene_product_uniform((1280, 720));

        assert_eq!(uniform.primitive_slot_flags[0][1], 1);
        assert_eq!(
            state.scene_packet.entity_for_pick_slot(1),
            Some(large_entity)
        );
        assert_eq!(state.scene_packet.entity_for_pick_slot(0), None);
    }

    #[test]
    fn viewport_primitive_slot_helpers_define_the_shader_packet_contract() {
        let mut primitive = EditorPrimitive::default();
        primitive.set_kind(EditorPrimitiveKind::Cylinder);
        primitive.capsule_radius = 0.45;
        primitive.capsule_half_height = 1.25;
        let instance = EditorViewportPrimitiveInstance::from_transform_and_primitive(
            editor_core::EntityId(42),
            LocalTransform::from_translation(Vec3Value::new(1.0, 2.0, 3.0)),
            primitive,
            true,
            true,
        );
        let packet = EditorViewportSceneRenderPacket::from_primitives([instance]);
        let instance = packet.primitives()[0];

        assert_eq!(instance.shader_slot_transform(), [1.0, 2.0, 3.0, 0.0]);
        assert_eq!(instance.shader_slot_params_b(), [0.45, 1.25, 0.0, 0.0]);
        assert_eq!(instance.shader_slot_flags(), [3, 1, 1, 0]);
    }

    #[test]
    fn sdf_material_slot_packet_uses_typed_u32_lane() {
        let instance = EditorViewportPrimitiveInstance::from_transform_and_primitive(
            editor_core::EntityId(42),
            LocalTransform::default(),
            EditorPrimitive::default(),
            false,
            false,
        )
        .with_material_slot_index(17);

        assert_eq!(instance.shader_slot_params_b()[2], 0.0);
        assert_eq!(instance.shader_slot_flags()[3], 17);
    }

    #[test]
    fn model_mesh_material_selection_packet_serializes_scene_product_uniform_regions() {
        let assigned_slot = SceneMaterialSlotId::new(2);
        let palette = SceneMaterialPalette::new([
            SceneMaterialSlot::default_generated(),
            SceneMaterialSlot::new(assigned_slot, "Imported Body").with_material_asset(asset_id(7)),
        ])
        .expect("valid palette");
        let material_region = SceneModelMeshMaterialRegionSourceId::new(
            SceneModelMeshSourceId::new(asset_id(42), asset_source_id(84))
                .with_source_revision_id(asset_source_revision_id(2))
                .with_source_revision("sha256:source"),
            SceneMeshMaterialRegionId::new("source_material_slot:0")
                .expect("source material slot key should be stable"),
        );
        let assignments = SceneMaterialAssignmentState::new_with_model_mesh_assignments(
            palette,
            [],
            [SceneModelMeshMaterialSlotAssignment::new(
                material_region.clone(),
                assigned_slot,
            )],
        )
        .expect("valid material assignment state");
        let packet = EditorViewportModelMeshMaterialSelectionPacket::from_model_mesh_regions(
            &assignments,
            [material_region],
        );

        let mut state = EditorViewportRenderState::default();
        state.set_model_mesh_material_selection_packet(packet);
        let uniform = state.compose_scene_product_uniform((1280, 720));

        assert_eq!(uniform.model_mesh_flags, [1, 0, 0, 0]);
        assert_eq!(uniform.model_mesh_region_flags[0], [1, 1, 0, 0]);
        assert_eq!(uniform.model_mesh_region_rects[0], [0.5, 0.56, 0.18, 0.22]);
        assert_eq!(uniform.model_mesh_region_flags[1], [0, 0, 0, 0]);
        assert_eq!(uniform.model_mesh_region_rects[1], [0.0, 0.0, 0.0, 0.0]);
        assert_eq!(
            state
                .branch_trace_snapshot((1280, 720))
                .first_model_mesh_region_flags,
            [1, 1, 0, 0]
        );
    }

    #[test]
    fn model_mesh_preview_packet_uses_source_backed_material_regions() {
        let assigned_slot = SceneMaterialSlotId::new(2);
        let palette = SceneMaterialPalette::new([
            SceneMaterialSlot::default_generated(),
            SceneMaterialSlot::new(assigned_slot, "Imported Body").with_material_asset(asset_id(7)),
        ])
        .expect("valid palette");
        let material_region = SceneModelMeshMaterialRegionSourceId::new(
            SceneModelMeshSourceId::new(asset_id(42), asset_source_id(84))
                .with_source_revision_id(asset_source_revision_id(2))
                .with_source_revision("sha256:source"),
            SceneMeshMaterialRegionId::new("source_material_slot:0")
                .expect("source material slot key should be stable"),
        );
        let assignments = SceneMaterialAssignmentState::new_with_model_mesh_assignments(
            palette,
            [],
            [SceneModelMeshMaterialSlotAssignment::new(
                material_region.clone(),
                assigned_slot,
            )],
        )
        .expect("valid material assignment state");

        let packet = EditorViewportModelMeshMaterialSelectionPacket::from_model_mesh_regions(
            &assignments,
            [material_region],
        );

        assert!(packet.diagnostics().is_empty());
        assert_eq!(packet.selections().len(), 1);
        let selection = packet.selections().first().expect("selection should exist");
        assert_eq!(selection.material_table_index, 1);
        assert_eq!(selection.requested_slot_id, assigned_slot);
        assert_eq!(selection.resolved_slot_id, assigned_slot);
        let prepared = selection.prepared_selection();
        assert_eq!(prepared.surface.source.asset_id, asset_id(42).raw());
        assert_eq!(prepared.surface.source.source_id, asset_source_id(84).raw());
        assert_eq!(
            prepared.surface.source.source_revision_id,
            Some(asset_source_revision_id(2).raw())
        );
        assert_eq!(
            prepared.surface.source.source_revision.as_deref(),
            Some("sha256:source")
        );
        assert_eq!(prepared.surface.region_key, "source_material_slot:0");
        assert!(
            !prepared.surface.identity_key().contains("renderable_index"),
            "model/mesh preview packets must not use transient renderable identity"
        );
    }

    #[test]
    fn viewport_shaders_decode_scene_primitives_and_keep_grid_in_overlay() {
        let scene_shader =
            include_str!("../../../../assets/shaders/editor_viewport_scene_product.wgsl");
        let picking_shader =
            include_str!("../../../../assets/shaders/editor_viewport_picking_product.wgsl");
        let overlay_shader =
            include_str!("../../../../assets/shaders/editor_viewport_overlay_product.wgsl");
        for shader in [scene_shader, picking_shader] {
            assert_shader_supports_primitive(shader, EditorPrimitiveKind::Sphere);
            assert_shader_supports_primitive(shader, EditorPrimitiveKind::Capsule);
            assert_shader_supports_primitive(shader, EditorPrimitiveKind::Cylinder);
            assert_shader_supports_primitive(shader, EditorPrimitiveKind::Torus);
            assert_shader_supports_primitive(shader, EditorPrimitiveKind::Plane);
            assert!(shader.contains("return sdf_box("));
        }
        for shader in [scene_shader, picking_shader, overlay_shader] {
            assert!(
                shader.contains("model_mesh_region_flags : array<vec4<u32>, 16>"),
                "viewport product shaders must keep the model/mesh material-region uniform ABI aligned"
            );
        }
        assert!(
            scene_shader.contains("fn model_mesh_region_at(viewport_local: vec2<f32>)")
                && scene_shader.contains("fn shade_model_mesh_region"),
            "scene product shader must consume model/mesh material-region uniform slots"
        );
        assert!(
            !scene_shader.contains("sdf_ground_box")
                && !scene_shader.contains("grid_shade")
                && !scene_shader.contains("grid_color"),
            "scene product shader must render only packet-backed authored primitives"
        );
        assert!(
            scene_shader.contains("viewport_background"),
            "scene product misses must resolve to a deterministic background instead of retaining prior target pixels"
        );
        assert!(
            overlay_shader.contains("grid_color") && overlay_shader.contains("grid_overlay"),
            "viewport grid visuals must live in the overlay product shader"
        );
    }

    #[test]
    fn viewport_wgsl_shaders_parse_and_validate() {
        for (label, shader) in [
            (
                "scene",
                include_str!("../../../../assets/shaders/editor_viewport_scene_product.wgsl"),
            ),
            (
                "picking",
                include_str!("../../../../assets/shaders/editor_viewport_picking_product.wgsl"),
            ),
            (
                "overlay",
                include_str!("../../../../assets/shaders/editor_viewport_overlay_product.wgsl"),
            ),
        ] {
            let module = naga::front::wgsl::parse_str(shader)
                .unwrap_or_else(|error| panic!("{label} WGSL should parse: {error}"));
            naga::valid::Validator::new(
                naga::valid::ValidationFlags::all(),
                naga::valid::Capabilities::empty(),
            )
            .validate(&module)
            .unwrap_or_else(|error| panic!("{label} WGSL should validate: {error}"));
        }
    }

    fn assert_shader_supports_primitive(shader: &str, kind: EditorPrimitiveKind) {
        let branch = format!("primitive_kind == {}u", kind.as_u32());
        assert!(
            shader.contains(&branch),
            "shader must decode {:?} packet slots with branch {branch}",
            kind
        );
    }
}
