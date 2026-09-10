use crate::SceneCatalog;
use crate::plugins::SceneManager;
use crate::plugins::scene::ui::{UiStyle, UiStyleTemplate, UiTextTemplate};
use anyhow::{Context, anyhow};
use serde::Deserialize;
use serde::de::Deserializer;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

const DEFAULT_MAIN_MENU_SCENE_ID: &str = "main_menu";
const DEFAULT_LOADING_SCENE_ID: &str = "loading_scene";
const DEFAULT_PANEL_STYLE: UiStyle = UiStyle {
    bg_color: [0.08, 0.09, 0.11, 1.0],
    border_color: [0.20, 0.22, 0.28, 1.0],
    border_width: 1.0,
    radius: 6.0,
};
const DEFAULT_BUTTON_STYLE: UiStyle = UiStyle {
    bg_color: [0.16, 0.30, 0.22, 1.0],
    border_color: [0.32, 0.56, 0.42, 1.0],
    border_width: 1.0,
    radius: 4.0,
};
const DEFAULT_BODY_TEXT_STYLE: SceneTemplateTextStyle = SceneTemplateTextStyle {
    color: [0.94, 0.98, 1.0, 1.0],
    size: 30.0,
};
const DEFAULT_BUTTON_TEXT_STYLE: SceneTemplateTextStyle = SceneTemplateTextStyle {
    color: [0.95, 0.99, 1.0, 1.0],
    size: 15.0,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum SceneTemplateButtonSlot {
    Primary,
    Secondary,
}

impl SceneTemplateButtonSlot {
    pub(crate) fn trigger_name(self) -> &'static str {
        match self {
            Self::Primary => "primary_click",
            Self::Secondary => "secondary_click",
        }
    }

    pub(crate) fn button_name(self) -> &'static str {
        match self {
            Self::Primary => "primary_button",
            Self::Secondary => "secondary_button",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct SceneTemplateTextStyle {
    pub color: [f32; 4],
    pub size: f32,
}

#[derive(Debug, Clone)]
pub(crate) struct SceneTemplateButtonSpec {
    pub label: String,
    pub style: UiStyle,
    pub text_style: SceneTemplateTextStyle,
    pub on_click: Option<SceneTemplateAction>,
    pub on_hold: Option<SceneTemplateHoldSpec>,
}

#[derive(Debug, Clone)]
pub(crate) struct SceneTemplateSceneSpec {
    pub body: String,
    pub panel_style: UiStyle,
    pub body_text_style: SceneTemplateTextStyle,
    pub primary_button: Option<SceneTemplateButtonSpec>,
    pub secondary_button: Option<SceneTemplateButtonSpec>,
}

impl SceneTemplateSceneSpec {
    pub(crate) fn button(&self, slot: SceneTemplateButtonSlot) -> Option<&SceneTemplateButtonSpec> {
        match slot {
            SceneTemplateButtonSlot::Primary => self.primary_button.as_ref(),
            SceneTemplateButtonSlot::Secondary => self.secondary_button.as_ref(),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) enum SceneTemplateAction {
    GoTo(String),
    Back,
    MainMenu,
    Emit(String),
}

#[derive(Debug, Clone)]
pub(crate) struct SceneTemplateHoldSpec {
    pub threshold_ms: f32,
    pub repeat_ms: Option<f32>,
    pub action: SceneTemplateAction,
}

#[derive(Debug, Clone, Default)]
struct HoldProgress {
    elapsed_ms: f32,
    hold_triggered: bool,
    repeat_elapsed_ms: f32,
}

#[derive(Debug, Clone, Default)]
struct SceneTemplateInteractionState {
    pressed_slot: Option<SceneTemplateButtonSlot>,
    hold: BTreeMap<SceneTemplateButtonSlot, HoldProgress>,
}

#[derive(Debug, Clone, Default, runen_ecs::Component, runen_ecs::Resource)]
pub(crate) struct SceneTemplateFlowResource {
    scenes: BTreeMap<String, SceneTemplateSceneSpec>,
    active_scene_id: Option<String>,
    back_stack: Vec<String>,
    interaction: SceneTemplateInteractionState,
}

impl SceneTemplateFlowResource {
    pub(crate) fn ensure_loaded_from_catalog(
        &mut self,
        catalog: &SceneCatalog,
    ) -> anyhow::Result<()> {
        if !self.scenes.is_empty() || catalog.is_empty() {
            return Ok(());
        }
        self.reload_from_catalog(catalog)
    }

    fn reload_from_catalog(&mut self, catalog: &SceneCatalog) -> anyhow::Result<()> {
        self.scenes.clear();
        self.active_scene_id = None;
        self.back_stack.clear();
        self.interaction = SceneTemplateInteractionState::default();

        for registration in catalog.iter() {
            let scene = load_scene_template_spec(&registration.id, &registration.template_path)?;
            self.scenes.insert(registration.id.clone(), scene);
        }

        self.active_scene_id = self
            .scenes
            .contains_key(DEFAULT_MAIN_MENU_SCENE_ID)
            .then(|| DEFAULT_MAIN_MENU_SCENE_ID.to_string())
            .or_else(|| {
                self.scenes
                    .contains_key(DEFAULT_LOADING_SCENE_ID)
                    .then(|| DEFAULT_LOADING_SCENE_ID.to_string())
            })
            .or_else(|| self.scenes.keys().next().cloned());

        Ok(())
    }

    pub(crate) fn has_scenes(&self) -> bool {
        self.active_scene_id
            .as_ref()
            .is_some_and(|id| self.scenes.contains_key(id))
    }

    pub(crate) fn active_scene_id(&self) -> Option<&str> {
        self.active_scene_id.as_deref()
    }

    pub(crate) fn active_scene(&self) -> Option<&SceneTemplateSceneSpec> {
        let id = self.active_scene_id.as_ref()?;
        self.scenes.get(id)
    }

    pub(crate) fn begin_press(&mut self, slot: Option<SceneTemplateButtonSlot>) {
        self.interaction.pressed_slot = slot;
        if slot.is_none() {
            self.interaction.hold.clear();
        }
    }

    pub(crate) fn release_press(
        &mut self,
        slot: Option<SceneTemplateButtonSlot>,
    ) -> Option<SceneTemplateButtonSlot> {
        let pressed = self.interaction.pressed_slot.take();
        self.interaction.hold.clear();
        match (pressed, slot) {
            (Some(pressed), Some(released)) if pressed == released => Some(released),
            _ => None,
        }
    }

    pub(crate) fn update_hold(
        &mut self,
        slot: Option<SceneTemplateButtonSlot>,
        mouse_down: bool,
        delta_ms: f32,
    ) -> Option<SceneTemplateButtonSlot> {
        if !mouse_down {
            self.interaction.hold.clear();
            return None;
        }
        let slot = slot?;
        let hold_spec = self
            .active_scene()
            .and_then(|scene| scene.button(slot))
            .and_then(|button| button.on_hold.clone())?;

        let state = self.interaction.hold.entry(slot).or_default();
        state.elapsed_ms += delta_ms.max(0.0);
        if !state.hold_triggered && state.elapsed_ms >= hold_spec.threshold_ms {
            state.hold_triggered = true;
            state.repeat_elapsed_ms = 0.0;
            return Some(slot);
        }
        if state.hold_triggered
            && let Some(repeat_ms) = hold_spec.repeat_ms
        {
            state.repeat_elapsed_ms += delta_ms.max(0.0);
            if state.repeat_elapsed_ms >= repeat_ms {
                state.repeat_elapsed_ms = 0.0;
                return Some(slot);
            }
        }
        None
    }

    pub(crate) fn click_action_for(
        &self,
        slot: SceneTemplateButtonSlot,
    ) -> Option<&SceneTemplateAction> {
        self.active_scene()
            .and_then(|scene| scene.button(slot))
            .and_then(|button| button.on_click.as_ref())
    }

    pub(crate) fn hold_action_for(
        &self,
        slot: SceneTemplateButtonSlot,
    ) -> Option<&SceneTemplateAction> {
        self.active_scene()
            .and_then(|scene| scene.button(slot))
            .and_then(|button| button.on_hold.as_ref())
            .map(|hold| &hold.action)
    }

    pub(crate) fn apply_action(
        &mut self,
        action: &SceneTemplateAction,
        manager: &mut SceneManager,
        trigger: &'static str,
        button: Option<&str>,
    ) -> anyhow::Result<()> {
        let current_scene = self.active_scene_id.clone().unwrap_or_default();
        let mut changed_scene = false;

        match action {
            SceneTemplateAction::GoTo(target) => {
                if !self.scenes.contains_key(target) {
                    return Err(anyhow!(
                        "scene template action target '{target}' is unknown"
                    ));
                }
                if current_scene != *target {
                    if !current_scene.is_empty() {
                        self.back_stack.push(current_scene);
                    }
                    self.active_scene_id = Some(target.clone());
                    changed_scene = true;
                }
            }
            SceneTemplateAction::Back => {
                if let Some(previous) = self.back_stack.pop()
                    && self.scenes.contains_key(&previous)
                {
                    self.active_scene_id = Some(previous);
                    changed_scene = true;
                }
            }
            SceneTemplateAction::MainMenu => {
                if self.scenes.contains_key(DEFAULT_MAIN_MENU_SCENE_ID) {
                    self.active_scene_id = Some(DEFAULT_MAIN_MENU_SCENE_ID.to_string());
                    self.back_stack.clear();
                    changed_scene = true;
                }
            }
            SceneTemplateAction::Emit(name) => {
                let scene_id = self.active_scene_id.clone().unwrap_or_default();
                publish_scene_template_event(manager, name, &scene_id, button, trigger);
            }
        }

        if changed_scene {
            manager.overlay_runtime.ui.layout_dirty = true;
            let active = self.active_scene_id.clone().unwrap_or_default();
            let is_gameplay_scene = active == "game_scene";
            manager.world.paused = !is_gameplay_scene;
            manager.set_active_overlay_visible(true);
        }

        Ok(())
    }
}

fn publish_scene_template_event(
    manager: &mut SceneManager,
    name: &str,
    _scene_id: &str,
    _button: Option<&str>,
    _trigger: &'static str,
) {
    manager
        .channels
        .overlay_console_lines
        .push(format!("[scene-event] {name}"));
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
struct SceneAssetTemplate {
    body: String,
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    panel_component: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    text_component: Option<String>,
    primary_button: Option<SceneButtonAssetTemplate>,
    secondary_button: Option<SceneButtonAssetTemplate>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
struct SceneButtonAssetTemplate {
    label: String,
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    component: Option<String>,
    on_click: Option<SceneActionAsset>,
    action: Option<SceneActionAsset>,
    on_hold: Option<SceneHoldAssetTemplate>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
struct SceneHoldAssetTemplate {
    threshold_ms: Option<f32>,
    repeat_ms: Option<f32>,
    action: Option<SceneActionAsset>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
enum SceneActionAsset {
    GoTo(String),
    Back,
    MainMenu,
    Emit(String),
}

impl SceneActionAsset {
    fn into_action(self) -> SceneTemplateAction {
        match self {
            Self::GoTo(value) => SceneTemplateAction::GoTo(value),
            Self::Back => SceneTemplateAction::Back,
            Self::MainMenu => SceneTemplateAction::MainMenu,
            Self::Emit(value) => SceneTemplateAction::Emit(value),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
struct PanelComponentAssetTemplate {
    bg_color: Option<(f32, f32, f32, f32)>,
    border_color: Option<(f32, f32, f32, f32)>,
    border_width: Option<f32>,
    radius: Option<f32>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
struct TextComponentAssetTemplate {
    color: Option<(f32, f32, f32, f32)>,
    size: Option<f32>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
struct ButtonComponentAssetTemplate {
    style: Option<UiStyleTemplate>,
    text: Option<UiTextTemplate>,
}

fn load_scene_template_spec(scene_id: &str, path: &str) -> anyhow::Result<SceneTemplateSceneSpec> {
    let scene_path = resolve_scene_template_path(path)?;
    let raw = fs::read_to_string(&scene_path)
        .with_context(|| format!("failed reading scene template '{}'", scene_path.display()))?;
    let scene: SceneAssetTemplate = ron::from_str(&raw).with_context(|| {
        format!(
            "failed parsing scene template '{}' as ron object",
            scene_path.display()
        )
    })?;

    let scene_dir = scene_path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));

    let panel_style = scene
        .panel_component
        .as_ref()
        .map(|component| load_panel_style(&scene_dir, component))
        .transpose()?
        .unwrap_or(DEFAULT_PANEL_STYLE);
    let body_text_style = scene
        .text_component
        .as_ref()
        .map(|component| load_text_style(&scene_dir, component, DEFAULT_BODY_TEXT_STYLE))
        .transpose()?
        .unwrap_or(DEFAULT_BODY_TEXT_STYLE);

    let primary_button = scene
        .primary_button
        .as_ref()
        .map(|button| load_button_spec(&scene_dir, button))
        .transpose()?;
    let secondary_button = scene
        .secondary_button
        .as_ref()
        .map(|button| load_button_spec(&scene_dir, button))
        .transpose()?;

    Ok(SceneTemplateSceneSpec {
        body: if scene.body.trim().is_empty() {
            scene_id.replace('_', " ")
        } else {
            scene.body
        },
        panel_style,
        body_text_style,
        primary_button,
        secondary_button,
    })
}

fn resolve_scene_template_path(template_path: &str) -> anyhow::Result<PathBuf> {
    let raw = PathBuf::from(template_path);
    if raw.exists() {
        return Ok(raw);
    }

    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut candidates = Vec::new();
    candidates.push(manifest_dir.join(template_path));

    if let Some(workspace_root) = manifest_dir.parent() {
        candidates.push(workspace_root.join(template_path));
    }

    if let Ok(stripped_engine_prefix) = raw.strip_prefix("engine") {
        candidates.push(manifest_dir.join(stripped_engine_prefix));
    }

    if let Some(found) = candidates.iter().find(|candidate| candidate.exists()) {
        return Ok(found.clone());
    }

    let checked = candidates
        .iter()
        .map(|candidate| candidate.display().to_string())
        .collect::<Vec<_>>()
        .join(", ");
    Err(anyhow!(
        "failed to locate scene template '{template_path}' (checked: {checked})"
    ))
}

fn load_panel_style(base_dir: &Path, component_path: &str) -> anyhow::Result<UiStyle> {
    let path = resolve_relative_component_path(base_dir, component_path);
    let raw = fs::read_to_string(&path)
        .with_context(|| format!("failed reading panel component '{}'", path.display()))?;
    let template: PanelComponentAssetTemplate = ron::from_str(&raw).with_context(|| {
        format!(
            "failed parsing panel component '{}' as ron object",
            path.display()
        )
    })?;

    let mut style = DEFAULT_PANEL_STYLE;
    if let Some(color) = template.bg_color {
        style.bg_color = [color.0, color.1, color.2, color.3];
    }
    if let Some(color) = template.border_color {
        style.border_color = [color.0, color.1, color.2, color.3];
    }
    if let Some(width) = template.border_width {
        style.border_width = width.max(0.0);
    }
    if let Some(radius) = template.radius {
        style.radius = radius.max(0.0);
    }
    Ok(style)
}

fn deserialize_optional_string<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum OptionalString {
        Direct(String),
        Wrapped(Option<String>),
    }

    Ok(match OptionalString::deserialize(deserializer)? {
        OptionalString::Direct(value) => Some(value),
        OptionalString::Wrapped(value) => value,
    })
}

fn load_text_style(
    base_dir: &Path,
    component_path: &str,
    fallback: SceneTemplateTextStyle,
) -> anyhow::Result<SceneTemplateTextStyle> {
    let path = resolve_relative_component_path(base_dir, component_path);
    let raw = fs::read_to_string(&path)
        .with_context(|| format!("failed reading text component '{}'", path.display()))?;
    let template: TextComponentAssetTemplate = ron::from_str(&raw).with_context(|| {
        format!(
            "failed parsing text component '{}' as ron object",
            path.display()
        )
    })?;

    let mut style = fallback;
    if let Some(color) = template.color {
        style.color = [color.0, color.1, color.2, color.3];
    }
    if let Some(size) = template.size {
        style.size = size.max(1.0);
    }
    Ok(style)
}

fn load_button_spec(
    base_dir: &Path,
    button: &SceneButtonAssetTemplate,
) -> anyhow::Result<SceneTemplateButtonSpec> {
    let mut style = DEFAULT_BUTTON_STYLE;
    let mut text_style = DEFAULT_BUTTON_TEXT_STYLE;

    if let Some(component) = button.component.as_ref() {
        let path = resolve_relative_component_path(base_dir, component);
        let raw = fs::read_to_string(&path)
            .with_context(|| format!("failed reading button component '{}'", path.display()))?;
        let template: ButtonComponentAssetTemplate = ron::from_str(&raw).with_context(|| {
            format!(
                "failed parsing button component '{}' as ron object",
                path.display()
            )
        })?;
        if let Some(patch) = template.style.as_ref() {
            apply_style_patch(&mut style, patch);
        }
        if let Some(patch) = template.text.as_ref() {
            apply_text_patch(&mut text_style, patch);
        }
    }

    let on_click = button
        .on_click
        .clone()
        .or(button.action.clone())
        .map(SceneActionAsset::into_action);
    let on_hold = button.on_hold.as_ref().and_then(|hold| {
        hold.action.clone().map(|action| SceneTemplateHoldSpec {
            threshold_ms: hold.threshold_ms.unwrap_or(650.0).max(1.0),
            repeat_ms: hold.repeat_ms.filter(|value| *value > 0.0),
            action: action.into_action(),
        })
    });

    Ok(SceneTemplateButtonSpec {
        label: button.label.clone(),
        style,
        text_style,
        on_click,
        on_hold,
    })
}

fn resolve_relative_component_path(base_dir: &Path, component_path: &str) -> PathBuf {
    let path = Path::new(component_path);
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        base_dir.join(path)
    }
}

fn apply_style_patch(style: &mut UiStyle, patch: &UiStyleTemplate) {
    if let Some(color) = patch.bg_color {
        style.bg_color = color;
    }
    if let Some(color) = patch.border_color {
        style.border_color = color;
    }
    if let Some(width) = patch.border_width {
        style.border_width = width.max(0.0);
    }
    if let Some(radius) = patch.radius {
        style.radius = radius.max(0.0);
    }
}

fn apply_text_patch(style: &mut SceneTemplateTextStyle, patch: &UiTextTemplate) {
    if let Some(color) = patch.color {
        style.color = color;
    }
    if let Some(size) = patch.size {
        style.size = size.max(1.0);
    }
}
