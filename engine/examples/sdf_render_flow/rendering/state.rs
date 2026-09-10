use engine::plugins::render::{GpuStorage, GpuUniform};

pub(crate) const DEFAULT_ORBIT_SPEED: f32 = 0.38;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SdfViewMode {
    Lit,
    Depth,
    Normals,
    Steps,
}

impl SdfViewMode {
    pub(crate) fn next(self) -> Self {
        match self {
            Self::Lit => Self::Depth,
            Self::Depth => Self::Normals,
            Self::Normals => Self::Steps,
            Self::Steps => Self::Lit,
        }
    }

    pub(crate) fn as_u32(self) -> u32 {
        match self {
            Self::Lit => 0,
            Self::Depth => 1,
            Self::Normals => 2,
            Self::Steps => 3,
        }
    }

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Lit => "lit",
            Self::Depth => "depth",
            Self::Normals => "normals",
            Self::Steps => "steps",
        }
    }
}

#[derive(Debug, Clone, Copy, GpuUniform)]
pub(crate) struct SdfPrepareParams {
    pub frame: [u32; 4],
    pub time: [f32; 4],
}

#[derive(Debug, Clone, Copy, GpuStorage)]
pub(crate) struct SdfHistoryProbe {
    pub value: [f32; 4],
}

#[derive(Debug, Clone, Copy, GpuUniform)]
pub(crate) struct SdfComposeParams {
    pub time_data: [f32; 4],
    pub surface: [f32; 4],
    pub camera: [f32; 4],
    pub view_data: [u32; 4],
    pub fog: [f32; 4],
    pub color_a: [f32; 4],
    pub color_b: [f32; 4],
}

#[derive(Debug, Clone, runen_ecs::Resource)]
pub(crate) struct Sdf3dRenderState {
    time_seconds: f32,
    orbit_yaw_radians: f32,
    orbit_pitch_radians: f32,
    orbit_distance: f32,
    orbit_speed: f32,
    fov_radians: f32,
    view_mode: SdfViewMode,
}

impl Default for Sdf3dRenderState {
    fn default() -> Self {
        Self {
            time_seconds: 0.0,
            orbit_yaw_radians: 0.15,
            orbit_pitch_radians: 0.32,
            orbit_distance: 5.2,
            orbit_speed: DEFAULT_ORBIT_SPEED,
            fov_radians: 57.0_f32.to_radians(),
            view_mode: SdfViewMode::Lit,
        }
    }
}

impl Sdf3dRenderState {
    pub(crate) fn cycle_view_mode(&mut self) {
        self.view_mode = self.view_mode.next();
    }

    pub(crate) fn view_mode_label(&self) -> &'static str {
        self.view_mode.label()
    }

    pub(crate) fn advance_by_frame_delta(&mut self, delta_seconds: f32) {
        let safe_delta = delta_seconds.clamp(0.0, 1.0 / 15.0).max(1.0 / 240.0);
        self.time_seconds += safe_delta;
        self.orbit_yaw_radians = (self.orbit_yaw_radians + safe_delta * self.orbit_speed)
            .rem_euclid(std::f32::consts::TAU);
    }

    pub(crate) fn prepare_params(&self) -> SdfPrepareParams {
        SdfPrepareParams {
            frame: [self.view_mode.as_u32(), 0, 0, 0],
            time: [
                self.time_seconds,
                self.orbit_yaw_radians,
                self.orbit_pitch_radians,
                self.orbit_distance,
            ],
        }
    }

    pub(crate) fn compose_params(&self, surface: (u32, u32)) -> SdfComposeParams {
        let width = surface.0.max(1) as f32;
        let height = surface.1.max(1) as f32;
        let pulse = 0.5 + 0.5 * (self.time_seconds * 0.82).sin();

        SdfComposeParams {
            time_data: [
                self.time_seconds,
                pulse,
                (self.time_seconds * 0.35).sin(),
                (self.time_seconds * 0.27).cos(),
            ],
            surface: [width, height, 1.0 / width, 1.0 / height],
            camera: [
                self.orbit_yaw_radians,
                self.orbit_pitch_radians,
                self.orbit_distance,
                self.fov_radians,
            ],
            view_data: [self.view_mode.as_u32(), 0, 0, 0],
            fog: [0.045, 2.4, 22.0, 0.0],
            color_a: [0.10, 0.52, 0.94, 1.0],
            color_b: [0.92, 0.34, 0.48, 1.0],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Sdf3dRenderState;

    #[test]
    fn advance_updates_time_and_orbit() {
        let mut state = Sdf3dRenderState::default();
        let before = state.compose_params((1280, 720));
        state.advance_by_frame_delta(1.0 / 60.0);
        let after = state.compose_params((1280, 720));

        assert!(after.time_data[0] > before.time_data[0]);
        assert!(after.camera[0] > before.camera[0]);
    }

    #[test]
    fn compose_params_include_surface_metrics() {
        let state = Sdf3dRenderState::default();
        let params = state.compose_params((1920, 1080));
        assert_eq!(params.surface[0], 1920.0);
        assert_eq!(params.surface[1], 1080.0);
        assert!((params.surface[2] - (1.0 / 1920.0)).abs() < 1.0e-6);
        assert!((params.surface[3] - (1.0 / 1080.0)).abs() < 1.0e-6);
    }

    #[test]
    fn prepare_params_preserve_view_and_history_seed_data() {
        let state = Sdf3dRenderState::default();
        let params = state.prepare_params();

        assert_eq!(params.frame[0], 0);
        assert_eq!(params.time[0], 0.0);
        assert_eq!(params.time[3], 5.2);
    }

    #[test]
    fn cycle_view_mode_updates_uniform_mode() {
        let mut state = Sdf3dRenderState::default();
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
