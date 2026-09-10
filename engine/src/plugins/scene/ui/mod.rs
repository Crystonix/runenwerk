use anyhow::Result;
use runen_ecs::{Entity, World};
use std::path::Path;
use ui_render_data::UiFrame;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum UiPresentationMode {
    #[default]
    Standard,
    CenteredDemo,
}

#[derive(Debug, Clone, Copy, PartialEq, Default, runen_ecs::Component)]
pub struct UiTransform {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, runen_ecs::Component)]
pub struct UiStyle {
    pub bg_color: [f32; 4],
    pub border_color: [f32; 4],
    pub border_width: f32,
    pub radius: f32,
}

impl Default for UiStyle {
    fn default() -> Self {
        Self {
            bg_color: [0.0, 0.0, 0.0, 0.0],
            border_color: [0.0, 0.0, 0.0, 0.0],
            border_width: 0.0,
            radius: 0.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, runen_ecs::Component)]
pub struct UiText {
    pub content: String,
    pub color: [f32; 4],
    pub size: f32,
}

impl Default for UiText {
    fn default() -> Self {
        Self {
            content: String::new(),
            color: [1.0, 1.0, 1.0, 1.0],
            size: 14.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, runen_ecs::Component)]
pub struct UiNode {
    pub visible: bool,
}

impl Default for UiNode {
    fn default() -> Self {
        Self { visible: true }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, runen_ecs::Component)]
pub struct UiDirty;

#[derive(Debug, Clone, PartialEq, serde::Deserialize, Default)]
pub struct UiStyleTemplate {
    pub bg_color: Option<[f32; 4]>,
    pub border_color: Option<[f32; 4]>,
    pub border_width: Option<f32>,
    pub radius: Option<f32>,
}

#[derive(Debug, Clone, PartialEq, serde::Deserialize, Default)]
pub struct UiTextTemplate {
    pub content: Option<String>,
    pub color: Option<[f32; 4]>,
    pub size: Option<f32>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, runen_ecs::Resource)]
pub struct UiRenderShaderConfig {
    pub rect_shader_asset_id: String,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct ConsoleInputEditorState {
    pub text: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConsoleUiLayoutConfig {
    pub outer_margin: f32,
    pub inner_padding: f32,
    pub footer_offset: f32,
    pub input_height: f32,
    pub button_width: f32,
    pub input_button_gap: f32,
    pub panel_width_ratio: f32,
    pub panel_height_ratio: f32,
    pub panel_min_width: f32,
    pub panel_min_height: f32,
}

impl Default for ConsoleUiLayoutConfig {
    fn default() -> Self {
        Self {
            outer_margin: 16.0,
            inner_padding: 12.0,
            footer_offset: 0.0,
            input_height: 32.0,
            button_width: 120.0,
            input_button_gap: 8.0,
            panel_width_ratio: 0.72,
            panel_height_ratio: 0.72,
            panel_min_width: 320.0,
            panel_min_height: 220.0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ConsoleUiRuntimeState {
    pub root: Entity,
    pub scrollback: Entity,
    pub input: Entity,
    pub confirm_button: Entity,
    pub frame: UiFrame,
    pub presentation_mode: UiPresentationMode,
    pub layout_dirty: bool,
    pub screen_size: (f32, f32),
    pub scale: f32,
    pub layout: ConsoleUiLayoutConfig,
    pub input_editor: ConsoleInputEditorState,
    pub log_lines: Vec<String>,
    pub log_scroll_lines_from_bottom: usize,
    pub max_lines: usize,
    pub template_path: Option<std::path::PathBuf>,
    pub template_modified: Option<std::time::SystemTime>,
}

pub fn initialize_console_ui(world: &mut World) -> Result<ConsoleUiRuntimeState> {
    let root = world.spawn((
        UiNode { visible: true },
        UiTransform::default(),
        UiStyle {
            bg_color: [0.08, 0.09, 0.11, 0.94],
            border_color: [0.20, 0.22, 0.28, 1.0],
            border_width: 1.0,
            radius: 8.0,
        },
        UiDirty,
    ))?;

    let scrollback = world.spawn((
        UiNode { visible: true },
        UiTransform::default(),
        UiStyle::default(),
        UiText {
            content: String::new(),
            color: [0.90, 0.96, 1.0, 1.0],
            size: 14.0,
        },
        UiDirty,
    ))?;

    let input = world.spawn((
        UiNode { visible: true },
        UiTransform::default(),
        UiStyle {
            bg_color: [0.11, 0.13, 0.16, 1.0],
            border_color: [0.24, 0.28, 0.34, 1.0],
            border_width: 1.0,
            radius: 6.0,
        },
        UiText {
            content: "grotto> ".to_string(),
            color: [0.94, 0.98, 1.0, 1.0],
            size: 14.0,
        },
        UiDirty,
    ))?;

    let confirm_button = world.spawn((
        UiNode { visible: true },
        UiTransform::default(),
        UiStyle {
            bg_color: [0.16, 0.30, 0.22, 1.0],
            border_color: [0.32, 0.56, 0.42, 1.0],
            border_width: 1.0,
            radius: 6.0,
        },
        UiText {
            content: "Send".to_string(),
            color: [0.96, 0.99, 1.0, 1.0],
            size: 14.0,
        },
        UiDirty,
    ))?;

    world.insert_resource(UiRenderShaderConfig::default());

    let mut ui = ConsoleUiRuntimeState {
        root,
        scrollback,
        input,
        confirm_button,
        frame: UiFrame::default(),
        presentation_mode: UiPresentationMode::Standard,
        layout_dirty: true,
        screen_size: (1280.0, 720.0),
        scale: 1.0,
        layout: ConsoleUiLayoutConfig::default(),
        input_editor: ConsoleInputEditorState::default(),
        log_lines: Vec::new(),
        log_scroll_lines_from_bottom: 0,
        max_lines: 256,
        template_path: None,
        template_modified: None,
    };
    ui.log_lines.push("[world] scene overlay ready".to_string());
    Ok(ui)
}

pub fn load_console_template(
    _world: &mut World,
    ui: &mut ConsoleUiRuntimeState,
    path: &Path,
) -> Result<()> {
    let modified = std::fs::metadata(path)
        .and_then(|meta| meta.modified())
        .ok();
    ui.template_path = Some(path.to_path_buf());
    ui.template_modified = modified;
    ui.layout_dirty = true;
    Ok(())
}

pub fn reload_console_template_if_changed(
    world: &mut World,
    ui: &mut ConsoleUiRuntimeState,
    force: bool,
) -> Result<()> {
    let Some(path) = ui.template_path.clone() else {
        return Ok(());
    };

    let modified = std::fs::metadata(&path)
        .and_then(|meta| meta.modified())
        .ok();
    let changed = force || modified != ui.template_modified;

    if changed {
        load_console_template(world, ui, &path)?;
    }

    Ok(())
}
