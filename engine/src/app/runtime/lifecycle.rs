use crate::app::App;
use crate::app::domain::mode::AppMode;
use crate::app::domain::runner::{FixedFramesRunner, FixedTicksRunner};
use crate::runtime::frame_lifecycle::{
    prepare_world_for_run, run_frame as run_runtime_frame, run_startup_if_needed,
};
use anyhow::Result;

impl App {
    pub fn run(self) -> Result<()> {
        match self.mode {
            AppMode::Windowed => self.run_windowed(),
            AppMode::Headless => {
                let mut app = self;
                app.run_headless()?;
                Ok(())
            }
        }
    }

    pub fn run_for_frames(mut self, frame_count: usize) -> Result<Self> {
        self.set_runner(FixedFramesRunner::new(frame_count));
        self.run_headless()?;
        Ok(self)
    }

    pub fn run_for_ticks(mut self, tick_count: u64) -> Result<Self> {
        self.set_runner(FixedTicksRunner::new(tick_count));
        self.run_headless()?;
        Ok(self)
    }

    pub(crate) fn prepare_for_run(&mut self, headless: bool) -> Result<()> {
        prepare_world_for_run(&mut self.world, &self.title, headless);
        run_startup_if_needed(&mut self.world, &mut self.scheduler, &mut self.startup_ran)
    }

    pub(crate) fn run_frame(&mut self) -> Result<()> {
        run_runtime_frame(&mut self.world, &mut self.scheduler)
    }
}
