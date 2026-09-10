use crate::rendering::{
    DEFAULT_SPRINT_MULTIPLIER, FreeFlightInput, ProceduralSkyTerrainState, build_render_flow,
};
use anyhow::Result;
use engine::SystemConfigExt;
use engine::plugins::input::domain::action;
use engine::plugins::{RenderPlugin, ScenePlugin, default_plugins};
use engine::prelude::{App, CoreSet, InputState, Res, ResMut, Startup, Time, Update, WindowState};
use winit::keyboard::KeyCode;

const ACTION_CYCLE_VIEW_MODE: &str = "terrain.view.cycle";
const ACTION_MOVE_UP: &str = "terrain.move_up";
const ACTION_MOVE_DOWN: &str = "terrain.move_down";

#[derive(Debug, Clone, Copy, runen_ecs::Resource)]
struct FpsTracker {
    fps_ema: f32,
    frame_ms_ema: f32,
}

impl Default for FpsTracker {
    fn default() -> Self {
        Self {
            fps_ema: 0.0,
            frame_ms_ema: 0.0,
        }
    }
}

impl FpsTracker {
    fn observe_frame_delta(&mut self, delta_seconds: f32) {
        let safe_dt = delta_seconds.max(1.0 / 1000.0);
        let fps = (1.0 / safe_dt).clamp(0.0, 2000.0);
        let frame_ms = (safe_dt * 1000.0).clamp(0.0, 1000.0);
        let alpha = 0.12;

        if self.fps_ema <= f32::EPSILON {
            self.fps_ema = fps;
        } else {
            self.fps_ema = self.fps_ema + (fps - self.fps_ema) * alpha;
        }
        if self.frame_ms_ema <= f32::EPSILON {
            self.frame_ms_ema = frame_ms;
        } else {
            self.frame_ms_ema = self.frame_ms_ema + (frame_ms - self.frame_ms_ema) * alpha;
        }
    }
}

pub(crate) fn run() -> Result<()> {
    let mut app = App::new();
    app.set_title("Procedural Sky + SDF Terrain - Public API Example");
    app.add_plugins(default_plugins());
    app.add_plugin(ScenePlugin);
    app.add_plugin(RenderPlugin);
    app.insert_resource(ProceduralSkyTerrainState::default());
    app.insert_resource(FpsTracker::default());
    app.add_systems(Startup, setup_terrain_input_bindings);
    app.add_systems(
        Update,
        update_terrain_view_and_animation_system
            .after(CoreSet::Input)
            .after(CoreSet::Time),
    );
    app.add_render_flow(build_render_flow());
    app.run()
}

fn setup_terrain_input_bindings(mut input: ResMut<InputState>) {
    input.map_key(ACTION_CYCLE_VIEW_MODE, KeyCode::Tab);
    input.map_key(ACTION_MOVE_UP, KeyCode::Space);
    input.map_key(ACTION_MOVE_DOWN, KeyCode::ControlLeft);
    input.map_key(ACTION_MOVE_DOWN, KeyCode::ControlRight);
}

fn update_terrain_view_and_animation_system(
    input: Res<InputState>,
    time: Res<Time>,
    mut state: ResMut<ProceduralSkyTerrainState>,
    mut fps: ResMut<FpsTracker>,
    mut window: ResMut<WindowState>,
) {
    let forward_axis = (if input.action_down(action::WORLD_MOVE_UP) {
        1.0
    } else {
        0.0
    }) - (if input.action_down(action::WORLD_MOVE_DOWN) {
        1.0
    } else {
        0.0
    });
    let right_axis = (if input.action_down(action::WORLD_MOVE_RIGHT) {
        1.0
    } else {
        0.0
    }) - (if input.action_down(action::WORLD_MOVE_LEFT) {
        1.0
    } else {
        0.0
    });
    let up_axis = (if input.action_down(ACTION_MOVE_UP) {
        1.0
    } else {
        0.0
    }) - (if input.action_down(ACTION_MOVE_DOWN) {
        1.0
    } else {
        0.0
    });
    let speed_scale = if input.shift_down() {
        DEFAULT_SPRINT_MULTIPLIER
    } else {
        1.0
    };
    state.apply_free_flight_input(
        time.delta_seconds,
        FreeFlightInput {
            forward_axis,
            right_axis,
            up_axis,
            speed_scale,
            mouse_delta: input.mouse_delta,
        },
    );
    if input.action_pressed(ACTION_CYCLE_VIEW_MODE) {
        state.cycle_view_mode();
    }
    fps.observe_frame_delta(time.delta_seconds);
    window.set_title(format!(
        "Procedural Sky + SDF Terrain | {:.1} fps ({:.2} ms) | WASD/Space/Ctrl + Mouse | Shift sprint | View: {} (Tab)",
        fps.fps_ema,
        fps.frame_ms_ema,
        state.view_mode_label(),
    ));
}
