pub use engine_sim::SimulationTick;

/// Fixed-step configuration and per-frame state resources.
///
/// Execution semantics are implemented in `runtime::fixed_step_executor`.

#[derive(Debug, Copy, Clone, PartialEq, runen_ecs::Component, runen_ecs::Resource)]
pub struct FixedTimeConfig {
    pub step_seconds: f32,
}

impl Default for FixedTimeConfig {
    fn default() -> Self {
        Self {
            step_seconds: 1.0 / 60.0,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, runen_ecs::Component, runen_ecs::Resource)]
pub struct CatchupBudget {
    pub max_steps_per_frame: u32,
}

impl Default for CatchupBudget {
    fn default() -> Self {
        Self {
            max_steps_per_frame: 4,
        }
    }
}

#[derive(Debug, Copy, Clone, Default, PartialEq, runen_ecs::Component, runen_ecs::Resource)]
pub struct FixedTimeState {
    pub accumulator_seconds: f32,
    pub steps_ran_last_frame: u32,
    pub saturated_frames: u64,
}

impl FixedTimeState {
    pub fn bounded_render_substeps(self, budget: CatchupBudget, region_max_substeps: u32) -> u32 {
        self.steps_ran_last_frame
            .min(budget.max_steps_per_frame)
            .min(region_max_substeps)
    }
}
