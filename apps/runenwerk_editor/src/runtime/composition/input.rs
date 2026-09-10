use std::collections::BTreeMap;

use engine::plugins::TouchInputPhase;
use engine::runtime::platform::{PlatformEvent, PlatformWindowEventQueueResource};
use engine::runtime::{NativeWindowId, Res, ResMut, WindowStateRegistryResource};
use ui_input::{
    Key, KeyState, KeyboardEvent, Modifiers, PointerButton, PointerEvent, PointerEventKind,
    PointerSourceKind, PointerToolKind, SemanticActionEvent, SemanticInputSource, TextInputEvent,
    UiInputEvent, UiSemanticAction,
};
use ui_math::{UiPoint, UiRect, UiVector};
use winit::event::{ElementState, MouseButton as WinitMouseButton};
use winit::keyboard::KeyCode;

use crate::runtime::resources::{EditorHostResource, scaled_shell_theme};
use crate::runtime::viewport::{
    ToolSurfaceRuntimeBindingRegistryResource, ViewportArtifactObservationResource,
    ViewportInstanceRegistryResource, ViewportPresentationStateResource,
    ViewportRenderStateCommandQueueResource,
};

#[derive(Clone, Copy, Debug, Default)]
struct TargetInputState {
    cursor: UiPoint,
    modifiers: Modifiers,
}

#[derive(Debug, Default, runen_ecs::Resource)]
pub struct EditorTargetInputRuntimeResource {
    by_window: BTreeMap<NativeWindowId, TargetInputState>,
}

#[allow(clippy::too_many_arguments)]
pub fn dispatch_editor_target_input_system(
    mut host: ResMut<EditorHostResource>,
    mut runtime: ResMut<EditorTargetInputRuntimeResource>,
    mut events: ResMut<PlatformWindowEventQueueResource>,
    windows: Res<WindowStateRegistryResource>,
    mut viewport_presentations: ResMut<ViewportPresentationStateResource>,
    viewport_observations: Res<ViewportArtifactObservationResource>,
    tool_surface_bindings: Res<ToolSurfaceRuntimeBindingRegistryResource>,
    viewport_instances: Res<ViewportInstanceRegistryResource>,
    mut viewport_render_commands: ResMut<ViewportRenderStateCommandQueueResource>,
) {
    for window_event in events.drain() {
        let native_window_id = window_event.native_window_id;
        if native_window_id == NativeWindowId::primary() {
            continue;
        }
        let Some(target_id) = host
            .shell_state
            .composition_target_bindings()
            .find(|entry| entry.binding.native_window_id == native_window_id)
            .map(|entry| entry.target_id)
        else {
            continue;
        };
        let Some(window) = windows.record(native_window_id) else {
            continue;
        };
        if matches!(
            window_event.event,
            PlatformEvent::Focused { focused: false }
        ) {
            host.shell_state
                .runtime_for_target_mut(target_id)
                .set_focused_widget(None);
            host.shell_state.clear_tab_drag_for_target(target_id);
            continue;
        }
        let state = runtime.by_window.entry(native_window_id).or_default();
        let ui_events = translate_event(state, window_event.event);
        if ui_events.is_empty() {
            continue;
        }
        let bounds = UiRect::new(
            0.0,
            0.0,
            window.size_px.0.max(1) as f32,
            window.size_px.1.max(1) as f32,
        );
        let theme = scaled_shell_theme(&host.theme, window.scale_factor);
        for event in ui_events {
            let EditorHostResource {
                app, shell_state, ..
            } = &mut *host;
            if let Err(error) = app.dispatch_shell_input_for_target(
                shell_state,
                target_id,
                bounds,
                &theme,
                &event,
                Some(&mut *viewport_presentations),
                Some(&viewport_observations),
                Some(&tool_surface_bindings),
                Some(&viewport_instances),
                Some(&mut *viewport_render_commands),
            ) {
                app.append_console_line(format!(
                    "[editor_composition.input.target_dispatch_failed] {error}"
                ));
            }
        }
    }
}

fn translate_event(state: &mut TargetInputState, event: PlatformEvent) -> Vec<UiInputEvent> {
    match event {
        PlatformEvent::CursorMoved { x, y } => {
            let next = UiPoint::new(x, y);
            let delta = next - state.cursor;
            state.cursor = next;
            vec![pointer_event(state, PointerEventKind::Move, delta, None)]
        }
        PlatformEvent::MouseWheel { delta } => vec![pointer_event(
            state,
            PointerEventKind::Scroll,
            UiVector::new(0.0, delta),
            None,
        )],
        PlatformEvent::MouseInput {
            state: element_state,
            button,
        } => pointer_button(button)
            .map(|button| {
                vec![pointer_event(
                    state,
                    if element_state == ElementState::Pressed {
                        PointerEventKind::Down
                    } else {
                        PointerEventKind::Up
                    },
                    UiVector::ZERO,
                    Some(button),
                )]
            })
            .unwrap_or_default(),
        PlatformEvent::KeyboardInput {
            key,
            state: element_state,
            text,
        } => {
            update_modifiers(&mut state.modifiers, key, element_state);
            let mut events = key_from_winit(key)
                .map(|key| {
                    vec![UiInputEvent::Keyboard(KeyboardEvent {
                        key,
                        state: if element_state == ElementState::Pressed {
                            KeyState::Pressed
                        } else {
                            KeyState::Released
                        },
                        modifiers: state.modifiers,
                    })]
                })
                .unwrap_or_default();
            if element_state == ElementState::Pressed
                && let Some(text) = text.filter(|value| !value.is_empty())
            {
                events.push(UiInputEvent::Text(TextInputEvent { text }));
            }
            events
        }
        PlatformEvent::Touch { phase, x, y, .. } => {
            let next = UiPoint::new(x, y);
            let delta = next - state.cursor;
            state.cursor = next;
            let kind = match phase {
                TouchInputPhase::Started => PointerEventKind::Down,
                TouchInputPhase::Moved => PointerEventKind::Move,
                TouchInputPhase::Ended => PointerEventKind::Up,
                TouchInputPhase::Cancelled => {
                    return vec![UiInputEvent::Semantic(SemanticActionEvent::new(
                        SemanticInputSource::Touch,
                        UiSemanticAction::Cancel,
                    ))];
                }
            };
            let mut event = pointer_event(state, kind, delta, Some(PointerButton::Primary));
            let UiInputEvent::Pointer(pointer) = &mut event else {
                unreachable!()
            };
            pointer.packet.source_kind = PointerSourceKind::Touch;
            pointer.packet.tool_kind = PointerToolKind::Finger;
            vec![event]
        }
        PlatformEvent::Focused { .. }
        | PlatformEvent::Resumed
        | PlatformEvent::CloseRequested
        | PlatformEvent::Resized { .. }
        | PlatformEvent::ScaleFactorChanged { .. }
        | PlatformEvent::MouseMotion { .. }
        | PlatformEvent::RedrawRequested => Vec::new(),
    }
}

fn pointer_event(
    state: &TargetInputState,
    kind: PointerEventKind,
    delta: UiVector,
    button: Option<PointerButton>,
) -> UiInputEvent {
    UiInputEvent::Pointer(PointerEvent {
        kind,
        position: state.cursor,
        delta,
        button,
        modifiers: state.modifiers,
        click_count: 1,
        ..PointerEvent::default()
    })
}

fn pointer_button(button: WinitMouseButton) -> Option<PointerButton> {
    match button {
        WinitMouseButton::Left => Some(PointerButton::Primary),
        WinitMouseButton::Right => Some(PointerButton::Secondary),
        WinitMouseButton::Middle => Some(PointerButton::Middle),
        WinitMouseButton::Back => Some(PointerButton::Other(4)),
        WinitMouseButton::Forward => Some(PointerButton::Other(5)),
        WinitMouseButton::Other(value) => Some(PointerButton::Other(value)),
    }
}

fn update_modifiers(modifiers: &mut Modifiers, key: KeyCode, state: ElementState) {
    let pressed = state == ElementState::Pressed;
    match key {
        KeyCode::ShiftLeft | KeyCode::ShiftRight => modifiers.shift = pressed,
        KeyCode::ControlLeft | KeyCode::ControlRight => modifiers.ctrl = pressed,
        KeyCode::AltLeft | KeyCode::AltRight => modifiers.alt = pressed,
        KeyCode::SuperLeft | KeyCode::SuperRight => modifiers.meta = pressed,
        _ => {}
    }
}

fn key_from_winit(key: KeyCode) -> Option<Key> {
    Some(match key {
        KeyCode::Enter | KeyCode::NumpadEnter => Key::Enter,
        KeyCode::Escape => Key::Escape,
        KeyCode::Backspace => Key::Backspace,
        KeyCode::Delete => Key::Delete,
        KeyCode::Tab => Key::Tab,
        KeyCode::Space => Key::Space,
        KeyCode::ArrowLeft => Key::Left,
        KeyCode::ArrowRight => Key::Right,
        KeyCode::ArrowUp => Key::Up,
        KeyCode::ArrowDown => Key::Down,
        KeyCode::Home => Key::Home,
        KeyCode::End => Key::End,
        KeyCode::PageUp => Key::PageUp,
        KeyCode::PageDown => Key::PageDown,
        KeyCode::Insert => Key::Insert,
        KeyCode::F1 => Key::F(1),
        KeyCode::F2 => Key::F(2),
        KeyCode::F3 => Key::F(3),
        KeyCode::F4 => Key::F(4),
        KeyCode::F5 => Key::F(5),
        KeyCode::F6 => Key::F(6),
        KeyCode::F7 => Key::F(7),
        KeyCode::F8 => Key::F(8),
        KeyCode::F9 => Key::F(9),
        KeyCode::F10 => Key::F(10),
        KeyCode::F11 => Key::F(11),
        KeyCode::F12 => Key::F(12),
        KeyCode::KeyA => Key::Character("a".to_owned()),
        KeyCode::KeyC => Key::Character("c".to_owned()),
        KeyCode::KeyD => Key::Character("d".to_owned()),
        KeyCode::KeyV => Key::Character("v".to_owned()),
        KeyCode::KeyX => Key::Character("x".to_owned()),
        KeyCode::KeyY => Key::Character("y".to_owned()),
        KeyCode::KeyZ => Key::Character("z".to_owned()),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn touch_events_preserve_touch_identity_and_cancel_semantically() {
        let mut state = TargetInputState::default();
        let started = translate_event(
            &mut state,
            PlatformEvent::Touch {
                phase: TouchInputPhase::Started,
                id: 1,
                x: 12.0,
                y: 18.0,
                pressure: None,
            },
        );
        assert!(matches!(
            started.as_slice(),
            [UiInputEvent::Pointer(PointerEvent {
                packet,
                ..
            })] if packet.source_kind == PointerSourceKind::Touch
                && packet.tool_kind == PointerToolKind::Finger
        ));

        let cancelled = translate_event(
            &mut state,
            PlatformEvent::Touch {
                phase: TouchInputPhase::Cancelled,
                id: 1,
                x: 12.0,
                y: 18.0,
                pressure: None,
            },
        );
        assert_eq!(
            cancelled,
            vec![UiInputEvent::Semantic(SemanticActionEvent::new(
                SemanticInputSource::Touch,
                UiSemanticAction::Cancel,
            ))]
        );
    }
}
