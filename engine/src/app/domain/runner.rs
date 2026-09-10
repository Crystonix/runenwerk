use crate::prelude::Time;
use crate::runtime::fixed_time::FixedTimeConfig;
use engine_sim::SimulationTick;
use runen_ecs::World;

// Owner: Engine Runtime - App Runner
pub trait AppRunner: Send {
    fn next_frame(&mut self, completed_frames: usize, world: &World) -> bool;

    fn before_frame(&mut self, _world: &mut World) {}
}

#[derive(Debug, Clone)]
pub struct FixedFramesRunner {
    frames_remaining: usize,
}

impl FixedFramesRunner {
    pub fn new(frame_count: usize) -> Self {
        Self {
            frames_remaining: frame_count,
        }
    }
}

impl AppRunner for FixedFramesRunner {
    fn next_frame(&mut self, _completed_frames: usize, _world: &World) -> bool {
        if self.frames_remaining == 0 {
            return false;
        }
        self.frames_remaining -= 1;
        true
    }
}

#[derive(Debug, Clone)]
pub struct FixedTicksRunner {
    target_ticks: u64,
}

impl FixedTicksRunner {
    pub fn new(target_ticks: u64) -> Self {
        Self { target_ticks }
    }
}

impl AppRunner for FixedTicksRunner {
    fn next_frame(&mut self, _completed_frames: usize, world: &World) -> bool {
        world
            .resource::<SimulationTick>()
            .map(|tick| tick.0 < self.target_ticks)
            .unwrap_or(false)
    }

    fn before_frame(&mut self, world: &mut World) {
        let fixed_step_seconds = world
            .resource::<FixedTimeConfig>()
            .map(|config| config.step_seconds)
            .unwrap_or(1.0 / 60.0);
        if let Ok(time) = world.resource_mut::<Time>() {
            time.delta_seconds = fixed_step_seconds;
        }
    }
}
