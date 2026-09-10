use std::sync::OnceLock;

use super::template_flow::{
    SceneTemplateButtonSlot, SceneTemplateButtonSpec, SceneTemplateFlowResource,
    SceneTemplateSceneSpec,
};
use crate::plugins::render::features::{DEFAULT_EDITOR_FONT_ID, UiFontAtlasResource};
use crate::plugins::scene::ui::{
    ConsoleUiRuntimeState, UiNode as SceneUiNode, UiPresentationMode, UiStyle, UiText, UiTransform,
    reload_console_template_if_changed,
};
use crate::plugins::{InputState, SceneManager};
use ui_math::{UiInsets, UiRect, UiSize};
use ui_runtime::{
    ComputedLayout, ComputedLayoutMap, InteractionVisualState, LabelNode, PanelNode, UiNode,
    UiNodeKind, UiTree, WidgetId, build_ui_frame,
};
use ui_text::{FontId, TextLineHeightPolicy, TextStyle};
use ui_theme::{ThemeTokens, UiColor};

const TEXT_PADDING_X: f32 = 10.0;
const TEXT_PADDING_Y: f32 = 8.0;
const MIN_INPUT_WIDTH: f32 = 120.0;

const SCENE_OVERLAY_ROOT_WIDGET_ID: u64 = 70_000;
const SCENE_OVERLAY_SCROLLBACK_WIDGET_ID: u64 = 70_001;
const SCENE_OVERLAY_INPUT_WIDGET_ID: u64 = 70_002;
const SCENE_OVERLAY_CONFIRM_WIDGET_ID: u64 = 70_003;
const SCENE_OVERLAY_LABEL_BASE_WIDGET_ID: u64 = 70_100;

pub(crate) fn rebuild_overlay_ui_frame(
    manager: &mut SceneManager,
    scene_templates: &SceneTemplateFlowResource,
) -> anyhow::Result<()> {
    let overlay_visible = manager.overlay_visible();
    let world = &mut manager.overlay_runtime.world;
    let ui = &mut manager.overlay_runtime.ui;
    let template_mode = scene_templates.has_scenes();

    if template_mode {
        if let Some(scene) = scene_templates.active_scene() {
            apply_scene_template_visuals(world, ui, scene);
        }
    } else {
        if let Err(err) = reload_console_template_if_changed(world, ui, false) {
            ui.log_lines
                .push(format!("[ui] template reload failed: {err:#}"));
        }
        sync_runtime_text(world, ui);
    }

    apply_runtime_layout(world, ui);

    if !overlay_visible {
        ui.frame = ui_render_data::UiFrame::default();
        return Ok(());
    }

    ui.frame = build_overlay_frame_via_substrate(world, ui, template_mode);
    Ok(())
}

pub(crate) fn process_overlay_pointer_input(
    manager: &mut SceneManager,
    input: &mut InputState,
    scene_templates: &mut SceneTemplateFlowResource,
    delta_seconds: f32,
) -> anyhow::Result<()> {
    if !manager.overlay_visible() || !scene_templates.has_scenes() {
        return Ok(());
    }

    let hovered_slot = hit_test_button_slot(
        &manager.overlay_runtime.world,
        &manager.overlay_runtime.ui,
        input.mouse_position,
    );
    if hovered_slot.is_some() {
        input.overlay_consumed = true;
    }

    if input.left_mouse_pressed() {
        scene_templates.begin_press(hovered_slot);
    }

    if let Some(slot) = scene_templates.update_hold(
        hovered_slot,
        input.left_mouse_down(),
        delta_seconds.max(0.0) * 1000.0,
    ) && let Some(action) = scene_templates.hold_action_for(slot).cloned()
    {
        scene_templates.apply_action(
            &action,
            manager,
            hold_trigger_name(slot),
            Some(slot.button_name()),
        )?;
    }

    if input.left_mouse_released()
        && let Some(slot) = scene_templates.release_press(hovered_slot)
        && let Some(action) = scene_templates.click_action_for(slot).cloned()
    {
        scene_templates.apply_action(
            &action,
            manager,
            slot.trigger_name(),
            Some(slot.button_name()),
        )?;
    }

    Ok(())
}

fn hold_trigger_name(slot: SceneTemplateButtonSlot) -> &'static str {
    match slot {
        SceneTemplateButtonSlot::Primary => "primary_hold",
        SceneTemplateButtonSlot::Secondary => "secondary_hold",
    }
}

fn hit_test_button_slot(
    world: &runen_ecs::World,
    ui: &ConsoleUiRuntimeState,
    pointer: (f32, f32),
) -> Option<SceneTemplateButtonSlot> {
    if contains_point(world, ui.confirm_button, pointer) {
        return Some(SceneTemplateButtonSlot::Secondary);
    }
    if contains_point(world, ui.input, pointer) {
        return Some(SceneTemplateButtonSlot::Primary);
    }
    None
}

fn contains_point(
    world: &runen_ecs::World,
    entity: runen_ecs::Entity,
    pointer: (f32, f32),
) -> bool {
    let Some(node) = world.get::<SceneUiNode>(entity) else {
        return false;
    };
    if !node.visible {
        return false;
    }
    let Some(transform) = world.get::<UiTransform>(entity) else {
        return false;
    };
    pointer.0 >= transform.x
        && pointer.0 <= transform.x + transform.w
        && pointer.1 >= transform.y
        && pointer.1 <= transform.y + transform.h
}

fn apply_scene_template_visuals(
    world: &mut runen_ecs::World,
    ui: &mut ConsoleUiRuntimeState,
    scene: &SceneTemplateSceneSpec,
) {
    ui.presentation_mode = UiPresentationMode::CenteredDemo;
    ui.layout_dirty = true;

    if let Ok(mut entity_ref) = world.entity_mut(ui.root) {
        if let Some(mut node) = entity_ref.get_mut::<SceneUiNode>() {
            node.visible = true;
        }
        if let Some(mut style) = entity_ref.get_mut::<UiStyle>() {
            *style = scene.panel_style;
        }
    }
    if let Ok(mut entity_ref) = world.entity_mut(ui.scrollback) {
        if let Some(mut node) = entity_ref.get_mut::<SceneUiNode>() {
            node.visible = true;
        }
        if let Some(mut text) = entity_ref.get_mut::<UiText>() {
            text.content = scene.body.clone();
            text.color = scene.body_text_style.color;
            text.size = scene.body_text_style.size;
        }
    }

    apply_template_button(world, ui.input, scene.primary_button.as_ref());
    apply_template_button(world, ui.confirm_button, scene.secondary_button.as_ref());
}

fn apply_template_button(
    world: &mut runen_ecs::World,
    entity: runen_ecs::Entity,
    button: Option<&SceneTemplateButtonSpec>,
) {
    if let Ok(mut entity_ref) = world.entity_mut(entity) {
        if let Some(mut node) = entity_ref.get_mut::<SceneUiNode>() {
            node.visible = button.is_some();
        }
        if let Some(button) = button {
            if let Some(mut style) = entity_ref.get_mut::<UiStyle>() {
                *style = button.style;
            }
            if let Some(mut text) = entity_ref.get_mut::<UiText>() {
                text.content = button.label.clone();
                text.color = button.text_style.color;
                text.size = button.text_style.size;
            }
        }
    }
}

fn apply_runtime_layout(world: &mut runen_ecs::World, ui: &mut ConsoleUiRuntimeState) {
    if !ui.layout_dirty {
        return;
    }

    let safe_width = ui.screen_size.0.max(1.0);
    let safe_height = ui.screen_size.1.max(1.0);
    let outer_margin = ui.layout.outer_margin.max(0.0);
    let inner_padding = ui.layout.inner_padding.max(0.0);
    let footer_offset = ui.layout.footer_offset.max(0.0);
    let input_height = ui.layout.input_height.max(1.0);
    let button_width = ui.layout.button_width.max(1.0);
    let input_button_gap = ui.layout.input_button_gap.max(0.0);

    let available_w = (safe_width - outer_margin * 2.0).max(1.0);
    let available_h = (safe_height - outer_margin * 2.0).max(1.0);
    let panel_w = (safe_width * ui.layout.panel_width_ratio)
        .max(ui.layout.panel_min_width)
        .min(available_w);
    let panel_h = (safe_height * ui.layout.panel_height_ratio)
        .max(ui.layout.panel_min_height)
        .min(available_h);

    let (panel_x, panel_y) = match ui.presentation_mode {
        UiPresentationMode::CenteredDemo => {
            ((safe_width - panel_w) * 0.5, (safe_height - panel_h) * 0.5)
        }
        UiPresentationMode::Standard => (outer_margin, outer_margin),
    };

    let logs_x = panel_x + inner_padding;
    let logs_y = panel_y + inner_padding;
    let logs_w = (panel_w - inner_padding * 2.0).max(1.0);
    let logs_h = (panel_h - inner_padding * 3.0 - input_height - footer_offset).max(1.0);

    let input_y = logs_y + logs_h + inner_padding;
    let target_input_width = (logs_w - button_width - input_button_gap)
        .clamp(MIN_INPUT_WIDTH, logs_w.max(MIN_INPUT_WIDTH));
    let final_button_width = (logs_w - target_input_width - input_button_gap).max(1.0);

    set_transform(
        world,
        ui.root,
        UiTransform {
            x: panel_x,
            y: panel_y,
            w: panel_w,
            h: panel_h,
        },
    );
    set_transform(
        world,
        ui.scrollback,
        UiTransform {
            x: logs_x,
            y: logs_y,
            w: logs_w,
            h: logs_h,
        },
    );
    set_transform(
        world,
        ui.input,
        UiTransform {
            x: logs_x,
            y: input_y,
            w: target_input_width,
            h: input_height,
        },
    );
    set_transform(
        world,
        ui.confirm_button,
        UiTransform {
            x: logs_x + target_input_width + input_button_gap,
            y: input_y,
            w: final_button_width,
            h: input_height,
        },
    );

    ui.layout_dirty = false;
}

fn set_transform(world: &mut runen_ecs::World, entity: runen_ecs::Entity, transform: UiTransform) {
    if let Ok(mut entity_ref) = world.entity_mut(entity)
        && let Some(mut current) = entity_ref.get_mut::<UiTransform>()
    {
        *current = transform;
    }
}

fn sync_runtime_text(world: &mut runen_ecs::World, ui: &mut ConsoleUiRuntimeState) {
    if ui.log_lines.is_empty() {
        ui.log_lines.push("[world] scene overlay ready".to_string());
    }

    let scrollback_transform = world.get::<UiTransform>(ui.scrollback).copied();
    let scrollback_style = world.get::<UiText>(ui.scrollback).cloned();
    let visible_lines = scrollback_transform
        .zip(scrollback_style.as_ref())
        .map(|(transform, text)| {
            let line_height = (text.size.max(1.0) * 1.25).max(1.0);
            ((transform.h / line_height).floor() as usize).max(1)
        })
        .unwrap_or(8);
    let scroll_offset = ui
        .log_scroll_lines_from_bottom
        .min(ui.log_lines.len().saturating_sub(1));
    let end = ui.log_lines.len().saturating_sub(scroll_offset);
    let start = end.saturating_sub(visible_lines);
    let scrollback_content = ui.log_lines[start..end].join("\n");

    if let Ok(mut entity_ref) = world.entity_mut(ui.scrollback)
        && let Some(mut text) = entity_ref.get_mut::<UiText>()
    {
        text.content = scrollback_content;
    }

    let input_prefix = if ui.input_editor.text.is_empty() {
        "grotto> ".to_string()
    } else {
        format!("grotto> {}", ui.input_editor.text)
    };
    if let Ok(mut entity_ref) = world.entity_mut(ui.input)
        && let Some(mut text) = entity_ref.get_mut::<UiText>()
    {
        text.content = input_prefix;
    }
}

fn build_overlay_frame_via_substrate(
    world: &runen_ecs::World,
    ui: &ConsoleUiRuntimeState,
    template_mode: bool,
) -> ui_render_data::UiFrame {
    let surface_size = UiSize::new(ui.screen_size.0.max(1.0), ui.screen_size.1.max(1.0));
    let Some(root_node) = world.get::<SceneUiNode>(ui.root) else {
        return ui_render_data::UiFrame::default();
    };
    if !root_node.visible {
        return ui_render_data::UiFrame::default();
    }

    let Some(root_transform) = world.get::<UiTransform>(ui.root).copied() else {
        return ui_render_data::UiFrame::default();
    };
    let root_style = world.get::<UiStyle>(ui.root).copied().unwrap_or_default();

    let root_bounds = rect_from_transform(root_transform);
    let mut layouts = ComputedLayoutMap::new();
    let mut children = Vec::new();
    let mut next_label_id = SCENE_OVERLAY_LABEL_BASE_WIDGET_ID;

    if let Some(node) = append_text_panel_node(
        world,
        ui.scrollback,
        WidgetId(SCENE_OVERLAY_SCROLLBACK_WIDGET_ID),
        !template_mode,
        false,
        surface_size,
        ui.scale,
        &mut next_label_id,
        &mut layouts,
    ) {
        children.push(node);
    }

    if let Some(node) = append_text_panel_node(
        world,
        ui.input,
        WidgetId(SCENE_OVERLAY_INPUT_WIDGET_ID),
        !template_mode,
        template_mode,
        surface_size,
        ui.scale,
        &mut next_label_id,
        &mut layouts,
    ) {
        children.push(node);
    }

    if let Some(node) = append_text_panel_node(
        world,
        ui.confirm_button,
        WidgetId(SCENE_OVERLAY_CONFIRM_WIDGET_ID),
        false,
        true,
        surface_size,
        ui.scale,
        &mut next_label_id,
        &mut layouts,
    ) {
        children.push(node);
    }

    let mut root_panel = PanelNode::new(theme_from_scene_style(root_style, ui.scale));
    root_panel.padding = UiInsets::ZERO;
    root_panel.gap = 0.0;
    let root_id = WidgetId(SCENE_OVERLAY_ROOT_WIDGET_ID);
    let root = UiNode::with_children(root_id, UiNodeKind::Panel(root_panel), children);
    layouts.insert(
        root_id,
        ComputedLayout::new(root_bounds, root_bounds, root_bounds.size()),
    );

    build_ui_frame(
        &UiTree::new(root),
        &layouts,
        surface_size,
        InteractionVisualState::default(),
        overlay_font_atlas(),
    )
}

#[allow(clippy::too_many_arguments)]
fn append_text_panel_node(
    world: &runen_ecs::World,
    entity: runen_ecs::Entity,
    panel_id: WidgetId,
    clip_text: bool,
    center_text: bool,
    surface_size: UiSize,
    scale: f32,
    next_label_id: &mut u64,
    layouts: &mut ComputedLayoutMap,
) -> Option<UiNode> {
    let node = world.get::<SceneUiNode>(entity)?;
    if !node.visible {
        return None;
    }
    let transform = world.get::<UiTransform>(entity).copied()?;
    let style = world.get::<UiStyle>(entity).copied().unwrap_or_default();
    let text = world.get::<UiText>(entity).cloned().unwrap_or_default();

    let bounds = rect_from_transform(transform);
    let clip_bounds = if clip_text {
        bounds
    } else {
        UiRect::new(0.0, 0.0, surface_size.width, surface_size.height)
    };

    let text_style = text_style_from_scene_text(&text);
    let line_height = text_style
        .line_height
        .resolve((text.size.max(1.0) * 1.25).max(1.0), text_style.font_size);
    let lines = if text.content.is_empty() {
        vec![String::new()]
    } else {
        text.content
            .lines()
            .map(ToOwned::to_owned)
            .collect::<Vec<_>>()
    };

    let mut label_children = Vec::with_capacity(lines.len());
    for (index, line) in lines.iter().enumerate() {
        let label_id = WidgetId(*next_label_id);
        *next_label_id = next_label_id.saturating_add(1);

        let x = if center_text {
            let approx_width = line.chars().count() as f32 * text_style.font_size * 0.55;
            bounds.x + (bounds.width - approx_width).max(0.0) * 0.5
        } else {
            bounds.x + TEXT_PADDING_X
        };
        let y = bounds.y + TEXT_PADDING_Y + line_height * index as f32;
        let available_width = (bounds.x + bounds.width - x - TEXT_PADDING_X).max(0.0);
        let label_bounds = UiRect::new(x, y, available_width, line_height.max(1.0));

        layouts.insert(
            label_id,
            ComputedLayout::new(label_bounds, label_bounds, label_bounds.size()),
        );
        label_children.push(UiNode::new(
            label_id,
            UiNodeKind::Label(LabelNode::new(line, text_style.clone())),
        ));
    }

    let mut panel = PanelNode::new(theme_from_scene_style(style, scale));
    panel.padding = UiInsets::ZERO;
    panel.gap = 0.0;

    layouts.insert(
        panel_id,
        ComputedLayout::new(bounds, clip_bounds, bounds.size()),
    );

    Some(UiNode::with_children(
        panel_id,
        UiNodeKind::Panel(panel),
        label_children,
    ))
}

fn rect_from_transform(transform: UiTransform) -> UiRect {
    UiRect::new(
        transform.x,
        transform.y,
        transform.w.max(0.0),
        transform.h.max(0.0),
    )
}

fn theme_from_scene_style(style: UiStyle, scale: f32) -> ThemeTokens {
    let mut theme = ThemeTokens::default().scaled_by(scale.max(0.5));
    theme.background_panel = to_ui_color(style.bg_color);
    theme.border = to_ui_color(style.border_color);
    theme.border_width = style.border_width.max(0.0);
    let radius = style.radius.max(0.0);
    theme.radius.sm = radius;
    theme.radius.md = radius;
    theme.radius.lg = radius;
    theme
}

fn to_ui_color(color: [f32; 4]) -> UiColor {
    UiColor::new(color[0], color[1], color[2], color[3])
}

fn text_style_from_scene_text(text: &UiText) -> TextStyle {
    TextStyle {
        font_id: FontId(DEFAULT_EDITOR_FONT_ID.0),
        font_size: text.size.max(1.0),
        color: text.color,
        line_height: TextLineHeightPolicy::Absolute((text.size.max(1.0) * 1.25).max(1.0)),
        ..TextStyle::default()
    }
}

fn overlay_font_atlas() -> &'static UiFontAtlasResource {
    static ATLAS: OnceLock<UiFontAtlasResource> = OnceLock::new();
    ATLAS.get_or_init(UiFontAtlasResource::default)
}
