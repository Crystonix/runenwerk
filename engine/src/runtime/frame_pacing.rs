use std::time::{Duration, Instant};

pub const DEFAULT_ANIMATION_FPS: u32 = 60;
const MIN_ANIMATION_FPS: u32 = 1;
const MAX_ANIMATION_FPS: u32 = 240;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FramePacingMode {
    ContinuousCapped { target_fps: u32 },
    OnDemand,
}

impl FramePacingMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ContinuousCapped { .. } => "continuous_capped",
            Self::OnDemand => "on_demand",
        }
    }

    pub fn target_fps(self) -> Option<u32> {
        match self {
            Self::ContinuousCapped { target_fps } => Some(clamp_target_fps(target_fps)),
            Self::OnDemand => None,
        }
    }

    pub fn frame_interval(self) -> Option<Duration> {
        self.target_fps()
            .map(|target_fps| Duration::from_secs_f64(1.0 / f64::from(target_fps)))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, runen_ecs::Component, runen_ecs::Resource)]
pub struct FramePacingPolicyResource {
    pub mode: FramePacingMode,
}

impl Default for FramePacingPolicyResource {
    fn default() -> Self {
        Self::continuous_capped(DEFAULT_ANIMATION_FPS)
    }
}

impl FramePacingPolicyResource {
    pub fn continuous_capped(target_fps: u32) -> Self {
        Self {
            mode: FramePacingMode::ContinuousCapped {
                target_fps: clamp_target_fps(target_fps),
            },
        }
    }

    pub fn on_demand() -> Self {
        Self {
            mode: FramePacingMode::OnDemand,
        }
    }

    pub fn target_frame_interval(self) -> Option<Duration> {
        self.mode.frame_interval()
    }
}

#[derive(Debug, Clone, PartialEq, runen_ecs::Component, runen_ecs::Resource)]
pub struct FramePacingRuntimeStateResource {
    pub mode: FramePacingMode,
    pub last_frame_interval_ms: f32,
    pub next_frame_delay_ms: Option<f32>,
    pub redraw_requested: bool,
}

impl Default for FramePacingRuntimeStateResource {
    fn default() -> Self {
        Self {
            mode: FramePacingPolicyResource::default().mode,
            last_frame_interval_ms: 0.0,
            next_frame_delay_ms: None,
            redraw_requested: false,
        }
    }
}

impl FramePacingRuntimeStateResource {
    pub fn observe_policy(&mut self, policy: FramePacingPolicyResource) {
        self.mode = policy.mode;
    }

    pub fn observe_frame_interval(&mut self, interval: Duration) {
        self.last_frame_interval_ms = interval.as_secs_f32() * 1000.0;
    }

    pub fn observe_next_deadline(&mut self, now: Instant, deadline: Option<Instant>) {
        self.next_frame_delay_ms = deadline.map(|value| {
            value
                .saturating_duration_since(now)
                .as_secs_f32()
                .mul_add(1000.0, 0.0)
        });
    }

    pub fn observe_redraw_requested(&mut self, requested: bool) {
        self.redraw_requested = requested;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FramePacingDecision {
    pub request_redraw: bool,
    pub next_deadline: Option<Instant>,
}

pub fn next_continuous_frame_deadline(
    policy: FramePacingPolicyResource,
    last_frame_at: Option<Instant>,
    now: Instant,
) -> Option<Instant> {
    let interval = policy.target_frame_interval()?;
    Some(last_frame_at.map_or(now, |last_frame| last_frame + interval))
}

pub fn decide_frame_pacing(
    policy: FramePacingPolicyResource,
    last_frame_at: Option<Instant>,
    now: Instant,
) -> FramePacingDecision {
    match next_continuous_frame_deadline(policy, last_frame_at, now) {
        Some(deadline) if now >= deadline => FramePacingDecision {
            request_redraw: true,
            next_deadline: Some(now + policy.target_frame_interval().unwrap_or_default()),
        },
        Some(deadline) => FramePacingDecision {
            request_redraw: false,
            next_deadline: Some(deadline),
        },
        None => FramePacingDecision {
            request_redraw: false,
            next_deadline: None,
        },
    }
}

fn clamp_target_fps(target_fps: u32) -> u32 {
    target_fps.clamp(MIN_ANIMATION_FPS, MAX_ANIMATION_FPS)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_pacing_default_is_sixty_fps_continuous() {
        let policy = FramePacingPolicyResource::default();

        assert_eq!(
            policy.mode,
            FramePacingMode::ContinuousCapped { target_fps: 60 }
        );
        assert_eq!(policy.mode.target_fps(), Some(60));
        assert_eq!(
            policy.target_frame_interval(),
            Some(Duration::from_secs_f64(1.0 / 60.0))
        );
    }

    #[test]
    fn frame_pacing_on_demand_has_no_continuous_deadline() {
        let now = Instant::now();
        let policy = FramePacingPolicyResource::on_demand();

        let decision = decide_frame_pacing(policy, Some(now), now);

        assert_eq!(
            decision,
            FramePacingDecision {
                request_redraw: false,
                next_deadline: None
            }
        );
    }

    #[test]
    fn frame_pacing_continuous_deadline_is_stable_and_nonzero() {
        let now = Instant::now();
        let last_frame = now - Duration::from_millis(1);
        let policy = FramePacingPolicyResource::continuous_capped(60);

        let decision = decide_frame_pacing(policy, Some(last_frame), now);

        assert!(!decision.request_redraw);
        assert!(decision.next_deadline.expect("deadline") > now);
    }

    #[test]
    fn frame_pacing_continuous_requests_redraw_after_deadline() {
        let now = Instant::now();
        let last_frame = now - Duration::from_millis(20);
        let policy = FramePacingPolicyResource::continuous_capped(60);

        let decision = decide_frame_pacing(policy, Some(last_frame), now);

        assert!(decision.request_redraw);
        assert!(decision.next_deadline.expect("next deadline") > now);
    }
}
