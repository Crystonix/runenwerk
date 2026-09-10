use crate::runtime::system::IntoSystemSetKey;
use ecs::{ScheduleLabel, SystemSetKey};

/// Core runtime schedules.
///
/// Frame contract (see `runtime::frame_lifecycle::run_frame`):
/// `PreUpdate` -> `FixedUpdate` (0..N) -> `Update` -> `RenderPrepare` -> `RenderSubmit` -> `FrameEnd`.

#[derive(Debug, Copy, Clone, Default)]
pub struct Startup;

impl ScheduleLabel for Startup {
    fn name() -> &'static str {
        "Startup"
    }
}

#[derive(Debug, Copy, Clone, Default)]
pub struct Update;

impl ScheduleLabel for Update {
    fn name() -> &'static str {
        "Update"
    }
}

#[derive(Debug, Copy, Clone, Default)]
pub struct PreUpdate;

impl ScheduleLabel for PreUpdate {
    fn name() -> &'static str {
        "PreUpdate"
    }
}

#[derive(Debug, Copy, Clone, Default)]
pub struct FixedUpdate;

impl ScheduleLabel for FixedUpdate {
    fn name() -> &'static str {
        "FixedUpdate"
    }
}

#[derive(Debug, Copy, Clone, Default)]
pub struct RenderPrepare;

impl ScheduleLabel for RenderPrepare {
    fn name() -> &'static str {
        "RenderPrepare"
    }
}

#[derive(Debug, Copy, Clone, Default)]
pub struct RenderSubmit;

impl ScheduleLabel for RenderSubmit {
    fn name() -> &'static str {
        "RenderSubmit"
    }
}

#[derive(Debug, Copy, Clone, Default)]
pub struct FrameEnd;

impl ScheduleLabel for FrameEnd {
    fn name() -> &'static str {
        "FrameEnd"
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum CoreSet {
    Input,
    Time,
    Scene,
    Simulation,
    FrameEnd,
}

impl IntoSystemSetKey for CoreSet {
    fn system_set_key(&self) -> SystemSetKey {
        match self {
            Self::Input => SystemSetKey::of::<CoreSet>("CoreSet::Input"),
            Self::Time => SystemSetKey::of::<CoreSet>("CoreSet::Time"),
            Self::Scene => SystemSetKey::of::<CoreSet>("CoreSet::Scene"),
            Self::Simulation => SystemSetKey::of::<CoreSet>("CoreSet::Simulation"),
            Self::FrameEnd => SystemSetKey::of::<CoreSet>("CoreSet::FrameEnd"),
        }
    }
}
