use engine::plugins::render::GpuUniform;

pub(crate) const DEFAULT_MOVE_SPEED: f32 = 13.0;
pub(crate) const DEFAULT_SPRINT_MULTIPLIER: f32 = 2.4;
pub(crate) const DEFAULT_LOOK_SENSITIVITY: f32 = 0.0022;
const MAX_PITCH_RADIANS: f32 = 1.54;

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct FreeFlightInput {
    pub forward_axis: f32,
    pub right_axis: f32,
    pub up_axis: f32,
    pub speed_scale: f32,
    pub mouse_delta: (f32, f32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TerrainViewMode {
    Lit,
    Height,
    Normals,
    Steps,
}

impl TerrainViewMode {
    pub(crate) fn next(self) -> Self {
        match self {
            Self::Lit => Self::Height,
            Self::Height => Self::Normals,
            Self::Normals => Self::Steps,
            Self::Steps => Self::Lit,
        }
    }

    pub(crate) fn as_u32(self) -> u32 {
        match self {
            Self::Lit => 0,
            Self::Height => 1,
            Self::Normals => 2,
            Self::Steps => 3,
        }
    }

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Lit => "lit",
            Self::Height => "height",
            Self::Normals => "normals",
            Self::Steps => "steps",
        }
    }
}

#[derive(Debug, Clone, Copy, GpuUniform)]
pub(crate) struct TerrainComposeParams {
    pub time_data: [f32; 4],
    pub surface: [f32; 4],
    pub camera_position: [f32; 4],
    pub camera_orientation: [f32; 4],
    pub terrain: [f32; 4],
    pub sky: [f32; 4],
    pub view_data: [u32; 4],
    pub colors_a: [f32; 4],
    pub colors_b: [f32; 4],
    pub colors_c: [f32; 4],
    pub colors_d: [f32; 4],
}

#[derive(Debug, Clone, runen_ecs::Resource)]
pub(crate) struct ProceduralSkyTerrainState {
    time_seconds: f32,
    camera_position: [f32; 3],
    camera_yaw_radians: f32,
    camera_pitch_radians: f32,
    move_speed: f32,
    look_sensitivity: f32,
    fov_radians: f32,
    view_mode: TerrainViewMode,
}

impl Default for ProceduralSkyTerrainState {
    fn default() -> Self {
        Self {
            time_seconds: 0.0,
            camera_position: [0.0, 6.2, 26.0],
            camera_yaw_radians: std::f32::consts::PI,
            camera_pitch_radians: -0.22,
            move_speed: DEFAULT_MOVE_SPEED,
            look_sensitivity: DEFAULT_LOOK_SENSITIVITY,
            fov_radians: 52.0_f32.to_radians(),
            view_mode: TerrainViewMode::Lit,
        }
    }
}

impl ProceduralSkyTerrainState {
    pub(crate) fn cycle_view_mode(&mut self) {
        self.view_mode = self.view_mode.next();
    }

    pub(crate) fn view_mode_label(&self) -> &'static str {
        self.view_mode.label()
    }

    pub(crate) fn apply_free_flight_input(&mut self, delta_seconds: f32, input: FreeFlightInput) {
        let safe_delta = delta_seconds.clamp(0.0, 1.0 / 15.0).max(1.0 / 240.0);
        self.time_seconds += safe_delta;

        self.camera_yaw_radians = (self.camera_yaw_radians
            - input.mouse_delta.0 * self.look_sensitivity)
            .rem_euclid(std::f32::consts::TAU);
        self.camera_pitch_radians = (self.camera_pitch_radians
            - input.mouse_delta.1 * self.look_sensitivity)
            .clamp(-MAX_PITCH_RADIANS, MAX_PITCH_RADIANS);

        let (forward, right) = self.camera_basis();
        let mut move_dir = [0.0_f32, 0.0_f32, 0.0_f32];
        move_dir[0] += forward[0] * input.forward_axis + right[0] * input.right_axis;
        move_dir[1] += forward[1] * input.forward_axis + input.up_axis;
        move_dir[2] += forward[2] * input.forward_axis + right[2] * input.right_axis;

        let len_sq =
            move_dir[0] * move_dir[0] + move_dir[1] * move_dir[1] + move_dir[2] * move_dir[2];
        if len_sq > 1.0e-6 {
            let inv_len = len_sq.sqrt().recip();
            let speed_boost = if input.speed_scale > 0.0 {
                input.speed_scale
            } else {
                1.0
            };
            let speed = self.move_speed * speed_boost * safe_delta;
            self.camera_position[0] += move_dir[0] * inv_len * speed;
            self.camera_position[1] += move_dir[1] * inv_len * speed;
            self.camera_position[2] += move_dir[2] * inv_len * speed;
        }
    }

    pub(crate) fn compose_params(&self, surface: (u32, u32)) -> TerrainComposeParams {
        let width = surface.0.max(1) as f32;
        let height = surface.1.max(1) as f32;
        let pulse = 0.5 + 0.5 * (self.time_seconds * 0.21).sin();

        TerrainComposeParams {
            time_data: [
                self.time_seconds,
                pulse,
                (self.time_seconds * 0.13).sin(),
                (self.time_seconds * 0.07).cos(),
            ],
            surface: [width, height, 1.0 / width, 1.0 / height],
            camera_position: [
                self.camera_position[0],
                self.camera_position[1],
                self.camera_position[2],
                self.fov_radians,
            ],
            camera_orientation: [self.camera_yaw_radians, self.camera_pitch_radians, 0.0, 0.0],
            terrain: [
                -1.8,  // base height
                6.4,   // height scale
                0.085, // base frequency
                2.6,   // detail gain
            ],
            sky: [
                0.75, // turbidity
                0.52, // sun elevation weight
                1.35, // cloud scale
                0.18, // cloud drift speed
            ],
            view_data: [self.view_mode.as_u32(), 0, 0, 0],
            colors_a: [0.06, 0.17, 0.38, 1.0], // sky top
            colors_b: [0.95, 0.60, 0.34, 1.0], // sky horizon
            colors_c: [0.25, 0.21, 0.16, 1.0], // terrain lowland
            colors_d: [0.24, 0.50, 0.24, 1.0], // terrain highland
        }
    }

    fn camera_basis(&self) -> ([f32; 3], [f32; 3]) {
        let (sin_yaw, cos_yaw) = self.camera_yaw_radians.sin_cos();
        let (sin_pitch, cos_pitch) = self.camera_pitch_radians.sin_cos();

        let forward = [cos_pitch * sin_yaw, sin_pitch, cos_pitch * cos_yaw];
        // Match shader basis: right = normalize(cross(forward, world_up)).
        let right = [-cos_yaw, 0.0, sin_yaw];
        (forward, right)
    }
}

#[cfg(test)]
mod tests {
    use super::{FreeFlightInput, ProceduralSkyTerrainState};

    #[test]
    fn advance_updates_time_without_motion() {
        let mut state = ProceduralSkyTerrainState::default();
        let before = state.compose_params((1280, 720));
        state.apply_free_flight_input(1.0 / 60.0, FreeFlightInput::default());
        let after = state.compose_params((1280, 720));

        assert!(after.time_data[0] > before.time_data[0]);
        assert_eq!(after.camera_position[0], before.camera_position[0]);
        assert_eq!(after.camera_position[1], before.camera_position[1]);
        assert_eq!(after.camera_position[2], before.camera_position[2]);
    }

    #[test]
    fn free_flight_motion_changes_position_and_orientation() {
        let mut state = ProceduralSkyTerrainState::default();
        let before = state.compose_params((1280, 720));

        state.apply_free_flight_input(
            1.0 / 60.0,
            FreeFlightInput {
                forward_axis: 1.0,
                right_axis: 0.5,
                up_axis: 1.0,
                speed_scale: 1.0,
                mouse_delta: (12.0, -8.0),
            },
        );

        let after = state.compose_params((1280, 720));
        assert_ne!(after.camera_position[0], before.camera_position[0]);
        assert_ne!(after.camera_position[1], before.camera_position[1]);
        assert_ne!(after.camera_position[2], before.camera_position[2]);
        assert_ne!(after.camera_orientation[0], before.camera_orientation[0]);
        assert_ne!(after.camera_orientation[1], before.camera_orientation[1]);
    }

    #[test]
    fn compose_params_include_surface_metrics() {
        let state = ProceduralSkyTerrainState::default();
        let params = state.compose_params((1920, 1080));
        assert_eq!(params.surface[0], 1920.0);
        assert_eq!(params.surface[1], 1080.0);
        assert!((params.surface[2] - (1.0 / 1920.0)).abs() < 1.0e-6);
        assert!((params.surface[3] - (1.0 / 1080.0)).abs() < 1.0e-6);
    }

    #[test]
    fn cycle_view_mode_updates_uniform_mode() {
        let mut state = ProceduralSkyTerrainState::default();
        assert_eq!(state.compose_params((1, 1)).view_data[0], 0);
        state.cycle_view_mode();
        assert_eq!(state.compose_params((1, 1)).view_data[0], 1);
        state.cycle_view_mode();
        assert_eq!(state.compose_params((1, 1)).view_data[0], 2);
        state.cycle_view_mode();
        assert_eq!(state.compose_params((1, 1)).view_data[0], 3);
        state.cycle_view_mode();
        assert_eq!(state.compose_params((1, 1)).view_data[0], 0);
    }
}
