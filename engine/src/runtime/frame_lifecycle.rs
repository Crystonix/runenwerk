use crate::runtime::fixed_step_executor::run_fixed_update_frame;
use crate::runtime::schedules::{
    FrameEnd, PreUpdate, RenderPrepare, RenderSubmit, Startup, Update,
};
use crate::runtime::window::WindowState;
use anyhow::Result;
use ecs::{Runtime, World};

/// Applies builtin runtime run-state before startup/frame execution.
///
/// This does not install resources. Builtin resources are installed by
/// `App::install_builtin_resources` during app construction.
pub(crate) fn prepare_world_for_run(world: &mut World, title: &str, headless: bool) {
    if let Ok(window) = world.resource_mut::<WindowState>() {
        window.set_headless(headless);
        window.redraw_requested = false;
        window.close_requested = false;
        window.title = title.to_string();
    }
}

/// Runs `Startup` at most once for a runtime state.
pub(crate) fn run_startup_if_needed(
    world: &mut World,
    scheduler: &mut Runtime,
    startup_ran: &mut bool,
) -> Result<()> {
    if *startup_ran {
        return Ok(());
    }

    scheduler.run_schedule::<Startup>(world)?;
    *startup_ran = true;
    Ok(())
}

/// Runs one runtime frame using the canonical execution-fabric phase order.
///
/// Each schedule below maps to a scheduler `ExecutionPhase`. Batch 1 keeps the
/// execution serial and delegates wave/barrier semantics to the ECS runtime:
///
/// 1. `PreUpdate`
/// 2. fixed-step loop (`FixedUpdate` zero or more times)
/// 3. `Update`
/// 4. `RenderPrepare`
/// 5. `RenderSubmit`
/// 6. `FrameEnd`
pub(crate) fn run_frame(world: &mut World, scheduler: &mut Runtime) -> Result<()> {
    scheduler.run_schedule::<PreUpdate>(world)?;
    run_fixed_update_frame(world, scheduler)?;
    scheduler.run_schedule::<Update>(world)?;
    scheduler.run_schedule::<RenderPrepare>(world)?;
    scheduler.run_schedule::<RenderSubmit>(world)?;
    scheduler.run_schedule::<FrameEnd>(world)?;
    world.advance_change_frame();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugins::time::domain::Time;
    use crate::runtime::fixed_time::{
        CatchupBudget, FixedTimeConfig, FixedTimeState, SimulationTick,
    };
    use anyhow::anyhow;

    fn test_world() -> World {
        let mut world = World::new();
        let mut time = Time::default();
        time.delta_seconds = 0.0;
        world.insert_resource(time);
        world.insert_resource(FixedTimeConfig::default());
        world.insert_resource(CatchupBudget::default());
        world.insert_resource(FixedTimeState::default());
        world.insert_resource(SimulationTick(0));
        world
    }

    #[test]
    fn successful_frame_end_advances_change_frame_once() {
        let mut world = test_world();
        let mut runtime = Runtime::new();

        run_frame(&mut world, &mut runtime).expect("frame should succeed");

        assert_eq!(world.current_frame_index(), 1);
    }

    #[test]
    fn change_frame_advance_is_skipped_when_frame_end_schedule_fails() {
        fn fail_frame_end() -> anyhow::Result<()> {
            Err(anyhow!("frame end failure"))
        }

        let mut world = test_world();
        let mut runtime = Runtime::new();
        runtime.add_systems::<FrameEnd, _, _>(&mut world, fail_frame_end);

        let err = run_frame(&mut world, &mut runtime).expect_err("frame should fail");
        assert!(format!("{err:#}").contains("frame end failure"));
        assert_eq!(world.current_frame_index(), 0);
    }
}
