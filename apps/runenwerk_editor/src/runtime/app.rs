use anyhow::Result;
use engine::plugins::{
    DiagnosticsConfigResource, RenderFlow, RenderPlugin, ScenePlugin, SchedulerDiagnosticsPlugin,
    default_plugins,
};
use engine::prelude::*;
use runen_gpu::GpuBindingKey;
#[cfg(test)]
use runen_gpu::GpuWorkResourceId;
use winit::keyboard::KeyCode;

use crate::runtime::plugin::EditorAppPlugin;
use crate::runtime::resources::{EditorHostResource, EditorViewportRenderState};
use crate::runtime::ui_gallery::UiGalleryPlugin;
use crate::runtime::viewport::{
    EDITOR_MAIN_FLOW_ID, EDITOR_VIEWPORT_SCENE_PRODUCT_UNIFORM_ID,
    VIEWPORT_TARGET_ALIAS_MATERIAL_PREVIEW, VIEWPORT_TARGET_ALIAS_OVERLAY,
    VIEWPORT_TARGET_ALIAS_PICKING_IDS, VIEWPORT_TARGET_ALIAS_SCENE_COLOR,
};

pub const ACTION_EDITOR_UNDO: &str = "editor.undo";
pub const ACTION_EDITOR_REDO: &str = "editor.redo";
pub const ACTION_EDITOR_TOOL_SELECT: &str = "editor.tool.select";
pub const ACTION_EDITOR_TOOL_TRANSLATE: &str = "editor.tool.translate";
pub const ACTION_EDITOR_TOOL_ROTATE: &str = "editor.tool.rotate";
pub const ACTION_EDITOR_TOOL_SCALE: &str = "editor.tool.scale";
pub const ACTION_EDITOR_VIEWPORT_FOCUS: &str = "editor.viewport.focus_selected";
pub const ACTION_EDITOR_VIEWPORT_TOOL_RADIAL: &str = "editor.viewport.tool_radial";

const EDITOR_WINDOW_TITLE: &str = "Runenwerk Editor";
const MATERIAL_LAB_WINDOW_TITLE: &str = "Runenwerk Material Lab";
const UI_DESIGNER_WINDOW_TITLE: &str = "Runenwerk UI Designer";
const UI_GALLERY_WINDOW_TITLE: &str = "Runenwerk UI Gallery";
const EDITOR_SURFACE_CLEAR_PASS_ID: &str = "runenwerk.editor.surface.clear";
const EDITOR_VIEWPORT_SCENE_PRODUCT_PASS_ID: &str = "runenwerk.editor.viewport.product.scene";
const EDITOR_VIEWPORT_PICKING_PRODUCT_PASS_ID: &str = "runenwerk.editor.viewport.product.picking";
const EDITOR_VIEWPORT_OVERLAY_PRODUCT_PASS_ID: &str = "runenwerk.editor.viewport.product.overlay";
const EDITOR_MAIN_UI_PASS_ID: &str = "runenwerk.editor.main.ui";
pub const EDITOR_MATERIAL_PREVIEW_FLOW_ID: &str = "runenwerk.editor.material.preview.flow";
const EDITOR_MATERIAL_PREVIEW_PASS_ID: &str = "runenwerk.editor.material.preview.pass";
pub const EDITOR_VIEWPORT_SCENE_PRODUCT_SHADER_ID: &str = "editor_viewport_scene_product";
pub const EDITOR_VIEWPORT_PICKING_PRODUCT_SHADER_ID: &str = "editor_viewport_picking_product";
pub const EDITOR_VIEWPORT_OVERLAY_PRODUCT_SHADER_ID: &str = "editor_viewport_overlay_product";
pub const EDITOR_MATERIAL_PREVIEW_SHADER_ID: &str = "editor_material_preview_generated";
const EDITOR_VIEWPORT_BACKGROUND_CLEAR: [f32; 4] = [0.09, 0.10, 0.12, 1.0];
const EDITOR_VIEWPORT_TRANSPARENT_CLEAR: [f32; 4] = [0.0, 0.0, 0.0, 0.0];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunenwerkRuntimeWorkbench {
    FullEditor,
    MaterialLab,
    UiDesigner,
    UiGallery,
}

impl RunenwerkRuntimeWorkbench {
    pub const fn window_title(self) -> &'static str {
        match self {
            Self::FullEditor => EDITOR_WINDOW_TITLE,
            Self::MaterialLab => MATERIAL_LAB_WINDOW_TITLE,
            Self::UiDesigner => UI_DESIGNER_WINDOW_TITLE,
            Self::UiGallery => UI_GALLERY_WINDOW_TITLE,
        }
    }

    fn editor_host_resource(self) -> EditorHostResource {
        match self {
            Self::FullEditor => EditorHostResource::new(),
            Self::MaterialLab => EditorHostResource::material_lab_workbench(),
            Self::UiDesigner => EditorHostResource::ui_designer_workbench(),
            Self::UiGallery => EditorHostResource::ui_designer_workbench(),
        }
    }
}

#[cfg(test)]
pub(crate) fn test_gpu_work_resource_id(label: &str) -> GpuWorkResourceId {
    let flow = RenderFlow::new("editor.runtime.test-resource")
        .with_color_target(label.to_string())
        .expect("test render flow authoring should succeed");
    flow.resource_id(label)
        .expect("test render resource should exist")
}

fn configure_app(app: &mut App) -> Result<()> {
    configure_app_for_workbench(app, RunenwerkRuntimeWorkbench::FullEditor)
}

fn configure_app_for_workbench(app: &mut App, workbench: RunenwerkRuntimeWorkbench) -> Result<()> {
    app.set_title(workbench.window_title());
    app.add_plugins(default_plugins());
    app.add_plugin(SchedulerDiagnosticsPlugin);
    app.add_plugin(ScenePlugin);
    app.add_plugin(RenderPlugin);
    register_editor_render_flow(app)?;
    if workbench == RunenwerkRuntimeWorkbench::UiGallery {
        app.init_resource::<EditorViewportRenderState>();
        app.add_plugin(UiGalleryPlugin);
    } else {
        app.insert_resource(workbench.editor_host_resource());
        app.add_plugin(EditorAppPlugin);
    }
    configure_editor_diagnostics(app);

    app.add_input_bindings([
        (ACTION_EDITOR_UNDO, KeyCode::KeyZ),
        (ACTION_EDITOR_REDO, KeyCode::KeyY),
        (ACTION_EDITOR_TOOL_SELECT, KeyCode::Digit1),
        (ACTION_EDITOR_TOOL_TRANSLATE, KeyCode::Digit2),
        (ACTION_EDITOR_TOOL_ROTATE, KeyCode::Digit3),
        (ACTION_EDITOR_TOOL_SCALE, KeyCode::Digit4),
        (ACTION_EDITOR_VIEWPORT_FOCUS, KeyCode::KeyF),
        (ACTION_EDITOR_VIEWPORT_TOOL_RADIAL, KeyCode::Tab),
    ]);
    Ok(())
}

fn configure_editor_diagnostics(app: &mut App) {
    if let Ok(config) = app.world_mut().resource_mut::<DiagnosticsConfigResource>() {
        config.adapters.console_enabled = env_flag_or("RUNENWERK_EDITOR_DIAGNOSTICS_CONSOLE", true);
        config.adapters.stdout_enabled = env_flag_or("RUNENWERK_EDITOR_DIAGNOSTICS_STDOUT", true);
        config.adapters.file_json_enabled = env_flag_or("RUNENWERK_EDITOR_DIAGNOSTICS_JSON", false);
    }
}

fn env_flag_or(key: &str, default: bool) -> bool {
    std::env::var(key)
        .ok()
        .map(|value| {
            matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
        .unwrap_or(default)
}

fn register_editor_render_flow(app: &mut App) -> Result<()> {
    let scene_product_uniform_binding = GpuBindingKey::try_new(0, 0)?;
    let (flow, scene_product_uniform) = RenderFlow::new(EDITOR_MAIN_FLOW_ID)
        .with_state::<EditorViewportRenderState>()
        .uniform_buffer::<crate::runtime::resources::EditorViewportSceneProductUniform>(
            EDITOR_VIEWPORT_SCENE_PRODUCT_UNIFORM_ID,
        )?;
    let flow = flow
        .with_color_target_alias(VIEWPORT_TARGET_ALIAS_SCENE_COLOR)?
        .with_color_target_alias(VIEWPORT_TARGET_ALIAS_PICKING_IDS)?
        .with_color_target_alias(VIEWPORT_TARGET_ALIAS_OVERLAY)?
        .with_surface_color()?
        .fullscreen_pass(EDITOR_SURFACE_CLEAR_PASS_ID)
        .main_surface_only()
        .clear_color(EDITOR_VIEWPORT_BACKGROUND_CLEAR)
        .write_surface_color()?
        .finish()
        .fullscreen_pass(EDITOR_VIEWPORT_SCENE_PRODUCT_PASS_ID)
        .offscreen_products_only()
        .material_scene_shader_asset(EDITOR_VIEWPORT_SCENE_PRODUCT_SHADER_ID)
        .clear_color(EDITOR_VIEWPORT_BACKGROUND_CLEAR)
        .uniform_from_state_with_surface_to(
            scene_product_uniform_binding,
            scene_product_uniform.clone(),
            EditorViewportRenderState::compose_scene_product_uniform,
        )
        .write_target_alias(VIEWPORT_TARGET_ALIAS_SCENE_COLOR)
        .finish()
        .fullscreen_pass(EDITOR_VIEWPORT_PICKING_PRODUCT_PASS_ID)
        .offscreen_products_only()
        .order_after(EDITOR_VIEWPORT_SCENE_PRODUCT_PASS_ID)
        .shader_asset(EDITOR_VIEWPORT_PICKING_PRODUCT_SHADER_ID)
        .uniform_from_state_with_surface_to(
            scene_product_uniform_binding,
            scene_product_uniform.clone(),
            EditorViewportRenderState::compose_scene_product_uniform,
        )
        .write_target_alias(VIEWPORT_TARGET_ALIAS_PICKING_IDS)
        .finish()
        .fullscreen_pass(EDITOR_VIEWPORT_OVERLAY_PRODUCT_PASS_ID)
        .offscreen_products_only()
        .order_after(EDITOR_VIEWPORT_PICKING_PRODUCT_PASS_ID)
        .shader_asset(EDITOR_VIEWPORT_OVERLAY_PRODUCT_SHADER_ID)
        .clear_color(EDITOR_VIEWPORT_TRANSPARENT_CLEAR)
        .uniform_from_state_with_surface_to(
            scene_product_uniform_binding,
            scene_product_uniform,
            EditorViewportRenderState::compose_scene_product_uniform,
        )
        .write_target_alias(VIEWPORT_TARGET_ALIAS_OVERLAY)
        .finish()
        .builtin_ui_composite_pass(EDITOR_MAIN_UI_PASS_ID)?
        .main_surface_only()
        .order_after(EDITOR_VIEWPORT_OVERLAY_PRODUCT_PASS_ID)
        .finish()
        .validate()
        .expect("editor render flow should validate");
    app.add_render_flow(flow);

    let material_preview_flow = RenderFlow::new(EDITOR_MATERIAL_PREVIEW_FLOW_ID)
        .with_color_target_alias(VIEWPORT_TARGET_ALIAS_MATERIAL_PREVIEW)?
        .fullscreen_pass(EDITOR_MATERIAL_PREVIEW_PASS_ID)
        .offscreen_products_only()
        .shader_asset(EDITOR_MATERIAL_PREVIEW_SHADER_ID)
        .for_feature(engine::plugins::render::MATERIAL_RENDER_FEATURE_ID)
        .write_target_alias(VIEWPORT_TARGET_ALIAS_MATERIAL_PREVIEW)
        .finish()
        .validate()
        .expect("editor material preview render flow should validate");
    app.add_render_flow(material_preview_flow);
    Ok(())
}

pub fn build_headless_app() -> Result<App> {
    build_headless_app_for_workbench(RunenwerkRuntimeWorkbench::FullEditor)
}

pub fn build_headless_app_for_workbench(workbench: RunenwerkRuntimeWorkbench) -> Result<App> {
    let mut app = App::headless();
    configure_app_for_workbench(&mut app, workbench)?;
    Ok(app)
}

pub fn build_material_lab_workbench_headless_app() -> Result<App> {
    build_headless_app_for_workbench(RunenwerkRuntimeWorkbench::MaterialLab)
}

pub fn build_ui_designer_workbench_headless_app() -> Result<App> {
    build_headless_app_for_workbench(RunenwerkRuntimeWorkbench::UiDesigner)
}

pub fn build_ui_gallery_workbench_headless_app() -> Result<App> {
    build_headless_app_for_workbench(RunenwerkRuntimeWorkbench::UiGallery)
}

pub fn run() -> Result<()> {
    let mut app = App::new();
    configure_app(&mut app)?;
    app.run()
}

pub fn run_material_lab_workbench() -> Result<()> {
    let mut app = App::new();
    configure_app_for_workbench(&mut app, RunenwerkRuntimeWorkbench::MaterialLab)?;
    app.run()
}

pub fn run_ui_designer_workbench() -> Result<()> {
    let mut app = App::new();
    configure_app_for_workbench(&mut app, RunenwerkRuntimeWorkbench::UiDesigner)?;
    app.run()
}

pub fn run_ui_gallery_workbench() -> Result<()> {
    let mut app = App::new();
    configure_app_for_workbench(&mut app, RunenwerkRuntimeWorkbench::UiGallery)?;
    app.run()
}

#[cfg(test)]
mod tests {
    use editor_shell::{EDITOR_DESIGN_WORKSPACE_PROFILE_ID, MATERIAL_WORKSPACE_PROFILE_ID};
    use engine::plugins::render::SurfaceFrameSubmissionRegistryResource;
    use ui_render_data::UiPrimitive;

    use crate::runtime::resources::EditorHostResource;
    use crate::runtime::ui_gallery::UI_GALLERY_UI_PRODUCER_ID;
    use crate::shell::RunenwerkWorkbenchComposition;

    use super::*;

    #[test]
    fn material_lab_headless_app_installs_material_lab_workbench_host() {
        let app = build_material_lab_workbench_headless_app()
            .expect("headless app construction should succeed");
        let host = app
            .world()
            .resource::<EditorHostResource>()
            .expect("runtime app should install editor host resource");

        assert_eq!(
            host.app.workbench_host().composition(),
            RunenwerkWorkbenchComposition::MaterialLab
        );
        assert_eq!(
            host.shell_state.active_workspace_profile_id(),
            MATERIAL_WORKSPACE_PROFILE_ID
        );
        assert_eq!(
            host.shell_state.open_workspace_profile_ids(),
            &[MATERIAL_WORKSPACE_PROFILE_ID]
        );
    }

    #[test]
    fn ui_designer_headless_app_installs_ui_designer_workbench_host() {
        let app = build_ui_designer_workbench_headless_app()
            .expect("headless app construction should succeed");
        let host = app
            .world()
            .resource::<EditorHostResource>()
            .expect("runtime app should install editor host resource");

        assert_eq!(
            host.app.workbench_host().composition(),
            RunenwerkWorkbenchComposition::UiDesigner
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
    fn ui_gallery_headless_app_installs_gallery_resource_without_editor_host() {
        let app = build_ui_gallery_workbench_headless_app()
            .expect("headless app construction should succeed");

        assert!(
            app.world()
                .resource::<crate::runtime::ui_gallery::UiGalleryResource>()
                .is_ok(),
            "UI gallery workbench should install the gallery resource"
        );
        assert!(
            app.world().resource::<EditorHostResource>().is_err(),
            "UI gallery should not submit through the editor shell host"
        );
    }

    #[test]
    fn ui_gallery_headless_app_submits_artifact_backed_frame() {
        let app = build_ui_gallery_workbench_headless_app()
            .expect("headless app construction should succeed")
            .run_for_frames(1)
            .expect("UI gallery headless app should run");
        let gallery = app
            .world()
            .resource::<crate::runtime::ui_gallery::UiGalleryResource>()
            .expect("UI gallery resource should exist");
        let submissions = app
            .world()
            .resource::<SurfaceFrameSubmissionRegistryResource>()
            .expect("UI frame submission registry should exist");
        let submission = submissions
            .get(&UI_GALLERY_UI_PRODUCER_ID)
            .expect("UI gallery should submit a frame");

        assert!(gallery.passed(), "{:?}", gallery.diagnostics());
        assert_eq!(gallery.button_count(), 2);
        assert!(!submission.frame.is_empty());
        assert!(
            submission
                .frame
                .surfaces
                .iter()
                .flat_map(|surface| surface.layers.iter())
                .flat_map(|layer| layer.primitives.iter())
                .any(|primitive| matches!(primitive, UiPrimitive::GlyphRun(_))),
            "gallery frame should contain shaped label text"
        );
    }
}
