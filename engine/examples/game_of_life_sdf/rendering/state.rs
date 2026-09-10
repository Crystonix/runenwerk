use engine::plugins::render::{GpuStorage, GpuUniform};
use glam::UVec2;

pub(crate) const DEFAULT_GRID_WIDTH: u32 = 160;
pub(crate) const DEFAULT_GRID_HEIGHT: u32 = 90;
pub(crate) const DEFAULT_GRID_CELL_COUNT: u64 =
    (DEFAULT_GRID_WIDTH as u64) * (DEFAULT_GRID_HEIGHT as u64);
pub(crate) const DEFAULT_TICKS_PER_SECOND: f32 = 12.0;

#[derive(Debug, Clone, Copy, GpuStorage)]
pub(crate) struct GameOfLifeCell {
    pub alive: u32,
}

#[derive(Debug, Clone, Copy, GpuUniform)]
pub(crate) struct GameOfLifeComputeParams {
    pub tick: u32,
    pub seed: u32,
    pub step: u32,
    pub read_from_a: u32,
    pub grid_size: UVec2,
}

#[derive(Debug, Clone, Copy, GpuUniform)]
pub(crate) struct GameOfLifeComposeParams {
    pub grid_size: UVec2,
    pub surface_size: [f32; 2],
    pub alive_mix: f32,
    pub current_is_a: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SimulationPhase {
    Seed,
    Idle,
    Step,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct SimulationClock {
    initialized: bool,
    accumulator_seconds: f32,
    ticks_per_second: f32,
}

impl Default for SimulationClock {
    fn default() -> Self {
        Self {
            initialized: false,
            accumulator_seconds: 0.0,
            ticks_per_second: DEFAULT_TICKS_PER_SECOND,
        }
    }
}

impl SimulationClock {
    pub(crate) fn advance(&mut self, delta_seconds: f32) -> SimulationPhase {
        if !self.initialized {
            self.initialized = true;
            return SimulationPhase::Seed;
        }

        self.accumulator_seconds += delta_seconds.max(0.0);
        let interval = 1.0 / self.ticks_per_second.max(1.0);

        if self.accumulator_seconds >= interval {
            self.accumulator_seconds %= interval;
            SimulationPhase::Step
        } else {
            SimulationPhase::Idle
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct PingPongBuffers {
    current_is_a: bool,
}

impl Default for PingPongBuffers {
    fn default() -> Self {
        Self { current_is_a: true }
    }
}

impl PingPongBuffers {
    pub(crate) fn current_is_a(&self) -> bool {
        self.current_is_a
    }

    pub(crate) fn flip(&mut self) {
        self.current_is_a = !self.current_is_a;
    }

    pub(crate) fn read_from_a(&self, phase: SimulationPhase) -> bool {
        match phase {
            SimulationPhase::Seed => true,
            SimulationPhase::Idle => self.current_is_a,
            SimulationPhase::Step => !self.current_is_a,
        }
    }
}

#[derive(Debug, Clone, runen_ecs::Resource)]
pub(crate) struct GameOfLifeRenderState {
    tick: u32,
    seed: u32,
    grid_size: UVec2,
    alive_mix: f32,
    phase: SimulationPhase,
    clock: SimulationClock,
    buffers: PingPongBuffers,
}

impl Default for GameOfLifeRenderState {
    fn default() -> Self {
        Self {
            tick: 0,
            seed: 0xC0FF_EE11,
            grid_size: UVec2::new(DEFAULT_GRID_WIDTH, DEFAULT_GRID_HEIGHT),
            alive_mix: 1.0,
            phase: SimulationPhase::Seed,
            clock: SimulationClock::default(),
            buffers: PingPongBuffers::default(),
        }
    }
}

impl GameOfLifeRenderState {
    pub(crate) fn advance_by_frame_delta(&mut self, delta_seconds: f32) {
        self.phase = self.clock.advance(delta_seconds);
        if self.phase == SimulationPhase::Step {
            self.tick = self.tick.saturating_add(1);
            self.buffers.flip();
        }
    }

    pub(crate) fn compute_params(&self) -> GameOfLifeComputeParams {
        GameOfLifeComputeParams {
            tick: self.tick,
            seed: self.seed,
            step: u32::from(self.phase == SimulationPhase::Step),
            read_from_a: u32::from(self.buffers.read_from_a(self.phase)),
            grid_size: self.grid_size,
        }
    }

    pub(crate) fn compose_params(&self, surface: (u32, u32)) -> GameOfLifeComposeParams {
        GameOfLifeComposeParams {
            grid_size: self.grid_size,
            surface_size: [surface.0 as f32, surface.1 as f32],
            alive_mix: self.alive_mix,
            current_is_a: u32::from(self.buffers.current_is_a()),
        }
    }

    pub(crate) fn dispatch_workgroups(&self) -> [u32; 3] {
        [
            self.grid_size.x.div_ceil(8),
            self.grid_size.y.div_ceil(8),
            1,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::{DEFAULT_TICKS_PER_SECOND, GameOfLifeRenderState};

    #[test]
    fn first_update_keeps_seed_phase() {
        let mut state = GameOfLifeRenderState::default();
        state.advance_by_frame_delta(1.0);
        let params = state.compute_params();
        assert_eq!(params.step, 0);
        assert_eq!(params.tick, 0);
        assert_eq!(params.read_from_a, 1);
    }

    #[test]
    fn stepping_flips_ping_pong_after_interval() {
        let mut state = GameOfLifeRenderState::default();
        state.advance_by_frame_delta(0.0);
        state.advance_by_frame_delta(1.0 / DEFAULT_TICKS_PER_SECOND);

        let params = state.compute_params();
        assert_eq!(params.tick, 1);
        assert_eq!(params.step, 1);
        assert_eq!(params.read_from_a, 1);
        assert_eq!(state.compose_params((1, 1)).current_is_a, 0);
    }

    #[test]
    fn idle_phase_keeps_current_buffer_read() {
        let mut state = GameOfLifeRenderState::default();
        state.advance_by_frame_delta(0.0);
        state.advance_by_frame_delta(0.0001);
        let params = state.compute_params();
        assert_eq!(params.step, 0);
        assert_eq!(params.read_from_a, 1);
    }

    #[test]
    fn large_delta_steps_once() {
        let mut state = GameOfLifeRenderState::default();
        state.advance_by_frame_delta(0.0);
        state.advance_by_frame_delta(10.0);

        let params = state.compute_params();
        assert_eq!(params.tick, 1);
        assert_eq!(params.step, 1);
    }
}
