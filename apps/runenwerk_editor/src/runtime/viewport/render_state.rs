//! File: apps/runenwerk_editor/src/runtime/viewport/render_state.rs
//! Purpose: Per-viewport runtime render-state ownership.

use std::collections::BTreeMap;

use editor_shell::ToolSurfaceInstanceId;
use editor_viewport::{ViewportCameraSettings, ViewportId};
use engine::runtime::ResMut;
use ui_math::UiRect;
use ui_math::UiVector;

use crate::runtime::resources::{EditorViewportDebugStage, EditorViewportRenderState};

#[derive(Debug, Clone)]
pub struct ViewportRenderStateEntry {
    pub viewport_id: ViewportId,
    pub tool_surface_id: Option<ToolSurfaceInstanceId>,
    pub bounds: UiRect,
    pub render_state: EditorViewportRenderState,
}

#[derive(Debug, Default, Clone, runen_ecs::Component, runen_ecs::Resource)]
pub struct ViewportRenderStateResource {
    states_by_viewport: BTreeMap<ViewportId, ViewportRenderStateEntry>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ViewportRenderStateCommand {
    SetCameraSettings {
        viewport_id: ViewportId,
        settings: ViewportCameraSettings,
    },
    OrbitCamera {
        viewport_id: ViewportId,
        delta: UiVector,
    },
    PanCamera {
        viewport_id: ViewportId,
        delta: UiVector,
    },
    ZoomCamera {
        viewport_id: ViewportId,
        scroll_delta: f32,
    },
    FocusCameraOn {
        viewport_id: ViewportId,
        orbit_target: [f32; 3],
    },
    ResetCamera {
        viewport_id: ViewportId,
    },
    SetDebugStage {
        viewport_id: ViewportId,
        debug_stage: EditorViewportDebugStage,
    },
    SetRootBackgroundOpaque {
        viewport_id: ViewportId,
        enabled: bool,
    },
}

#[derive(Debug, Default, Clone, runen_ecs::Component, runen_ecs::Resource)]
pub struct ViewportRenderStateCommandQueueResource {
    commands: Vec<ViewportRenderStateCommand>,
}

impl ViewportRenderStateResource {
    pub fn upsert_state(&mut self, state: ViewportRenderStateEntry) {
        self.states_by_viewport.insert(state.viewport_id, state);
    }

    pub fn state_for(&self, viewport_id: ViewportId) -> Option<&ViewportRenderStateEntry> {
        self.states_by_viewport.get(&viewport_id)
    }

    pub fn state_for_mut(
        &mut self,
        viewport_id: ViewportId,
    ) -> Option<&mut ViewportRenderStateEntry> {
        self.states_by_viewport.get_mut(&viewport_id)
    }

    pub fn viewport_ids(&self) -> impl Iterator<Item = ViewportId> + '_ {
        self.states_by_viewport.keys().copied()
    }

    pub fn entries(&self) -> impl Iterator<Item = &ViewportRenderStateEntry> {
        self.states_by_viewport.values()
    }

    pub fn retain_viewports(&mut self, mut keep: impl FnMut(ViewportId) -> bool) {
        self.states_by_viewport
            .retain(|viewport_id, _| keep(*viewport_id));
    }

    pub fn apply_command(&mut self, command: ViewportRenderStateCommand) -> bool {
        let viewport_id = match command {
            ViewportRenderStateCommand::SetCameraSettings { viewport_id, .. }
            | ViewportRenderStateCommand::OrbitCamera { viewport_id, .. }
            | ViewportRenderStateCommand::PanCamera { viewport_id, .. }
            | ViewportRenderStateCommand::ZoomCamera { viewport_id, .. }
            | ViewportRenderStateCommand::FocusCameraOn { viewport_id, .. }
            | ViewportRenderStateCommand::ResetCamera { viewport_id }
            | ViewportRenderStateCommand::SetDebugStage { viewport_id, .. }
            | ViewportRenderStateCommand::SetRootBackgroundOpaque { viewport_id, .. } => {
                viewport_id
            }
        };
        let Some(entry) = self.states_by_viewport.get_mut(&viewport_id) else {
            return false;
        };
        match command {
            ViewportRenderStateCommand::SetCameraSettings { settings, .. } => {
                entry.render_state.set_camera_settings(settings);
            }
            ViewportRenderStateCommand::OrbitCamera { delta, .. } => {
                entry.render_state.orbit_camera(delta);
            }
            ViewportRenderStateCommand::PanCamera { delta, .. } => {
                entry.render_state.pan_camera(delta);
            }
            ViewportRenderStateCommand::ZoomCamera { scroll_delta, .. } => {
                entry.render_state.zoom_camera(scroll_delta);
            }
            ViewportRenderStateCommand::FocusCameraOn { orbit_target, .. } => {
                entry.render_state.focus_camera_on(orbit_target);
            }
            ViewportRenderStateCommand::ResetCamera { .. } => {
                entry.render_state.reset_camera();
            }
            ViewportRenderStateCommand::SetDebugStage { debug_stage, .. } => {
                entry.render_state.set_debug_stage(debug_stage);
            }
            ViewportRenderStateCommand::SetRootBackgroundOpaque { enabled, .. } => {
                entry.render_state.set_root_background_opaque(enabled);
            }
        }
        true
    }

    pub fn apply_commands(
        &mut self,
        commands: impl IntoIterator<Item = ViewportRenderStateCommand>,
    ) -> usize {
        commands
            .into_iter()
            .filter(|command| self.apply_command(*command))
            .count()
    }

    pub fn is_empty(&self) -> bool {
        self.states_by_viewport.is_empty()
    }
}

impl ViewportRenderStateCommandQueueResource {
    pub fn push(&mut self, command: ViewportRenderStateCommand) {
        self.commands.push(command);
    }

    pub fn extend(&mut self, commands: impl IntoIterator<Item = ViewportRenderStateCommand>) {
        self.commands.extend(commands);
    }

    pub fn drain(&mut self) -> impl Iterator<Item = ViewportRenderStateCommand> + '_ {
        self.commands.drain(..)
    }

    pub fn len(&self) -> usize {
        self.commands.len()
    }

    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }
}

pub fn apply_viewport_render_state_commands_system(
    mut viewport_render_states: ResMut<ViewportRenderStateResource>,
    mut commands: ResMut<ViewportRenderStateCommandQueueResource>,
) {
    let commands = commands.drain().collect::<Vec<_>>();
    viewport_render_states.apply_commands(commands);
}

pub fn expression_dimensions_for_bounds(bounds: UiRect) -> editor_viewport::ExpressionDimensions {
    editor_viewport::ExpressionDimensions::new(
        bounds.width.max(1.0).round() as u32,
        bounds.height.max(1.0).round() as u32,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_state_tracks_distinct_bounds_per_viewport() {
        let mut states = ViewportRenderStateResource::default();
        let first = ViewportId(1_000_001);
        let second = ViewportId(1_000_002);

        states.upsert_state(ViewportRenderStateEntry {
            viewport_id: first,
            tool_surface_id: Some(ToolSurfaceInstanceId::try_from_raw(1).unwrap()),
            bounds: UiRect::new(0.0, 0.0, 320.0, 240.0),
            render_state: EditorViewportRenderState::default(),
        });
        states.upsert_state(ViewportRenderStateEntry {
            viewport_id: second,
            tool_surface_id: Some(ToolSurfaceInstanceId::try_from_raw(2).unwrap()),
            bounds: UiRect::new(320.0, 0.0, 480.0, 240.0),
            render_state: EditorViewportRenderState::default(),
        });

        assert_eq!(
            expression_dimensions_for_bounds(states.state_for(first).unwrap().bounds),
            editor_viewport::ExpressionDimensions::new(320, 240),
        );
        assert_eq!(
            expression_dimensions_for_bounds(states.state_for(second).unwrap().bounds),
            editor_viewport::ExpressionDimensions::new(480, 240),
        );
    }

    #[test]
    fn render_state_commands_apply_to_one_viewport() {
        let mut states = ViewportRenderStateResource::default();
        let first = ViewportId(2);
        let second = ViewportId(3);
        states.upsert_state(ViewportRenderStateEntry {
            viewport_id: first,
            tool_surface_id: Some(ToolSurfaceInstanceId::try_from_raw(1).unwrap()),
            bounds: UiRect::new(0.0, 0.0, 320.0, 240.0),
            render_state: EditorViewportRenderState::default(),
        });
        states.upsert_state(ViewportRenderStateEntry {
            viewport_id: second,
            tool_surface_id: Some(ToolSurfaceInstanceId::try_from_raw(2).unwrap()),
            bounds: UiRect::new(320.0, 0.0, 480.0, 240.0),
            render_state: EditorViewportRenderState::default(),
        });

        assert!(
            states.apply_command(ViewportRenderStateCommand::SetDebugStage {
                viewport_id: second,
                debug_stage: EditorViewportDebugStage::PrimitiveAvailability,
            })
        );
        assert!(
            states.apply_command(ViewportRenderStateCommand::SetRootBackgroundOpaque {
                viewport_id: second,
                enabled: true,
            })
        );

        assert_eq!(
            states.state_for(first).unwrap().render_state.debug_stage,
            EditorViewportDebugStage::Scene,
        );
        assert_eq!(
            states.state_for(second).unwrap().render_state.debug_stage,
            EditorViewportDebugStage::PrimitiveAvailability,
        );
        assert!(
            states
                .state_for(second)
                .unwrap()
                .render_state
                .root_background_opaque
        );
    }
}
