use std::time::Instant;

#[derive(runen_ecs::Component, runen_ecs::Resource)]
pub struct Time {
    last_frame: Instant,
    pub delta_seconds: f32,
}

impl Time {
    pub fn new() -> Self {
        Self {
            last_frame: Instant::now(),
            delta_seconds: 1.0 / 60.0,
        }
    }

    pub fn tick(&mut self) {
        let now = Instant::now();
        self.delta_seconds = (now - self.last_frame).as_secs_f32().min(0.25);
        self.last_frame = now;
    }
}

impl Default for Time {
    fn default() -> Self {
        Self::new()
    }
}
