use crate::runtime::fixed_step_executor::run_fixed_update_frame;
use crate::runtime::publication::run_schedule_with_publication;
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

    run_schedule_with_publication::<Startup>(world, scheduler)?;
    *startup_ran = true;
    Ok(())
}

/// Runs one runtime frame using the canonical Engine-owned lifecycle order:
///
/// 1. `PreUpdate`
/// 2. fixed-step loop (`FixedUpdate` zero or more times)
/// 3. `Update`
/// 4. `RenderPrepare`
/// 5. `RenderSubmit`
/// 6. `FrameEnd`
///
/// RunenECS executes each generic schedule and reports ECS-neutral deferred-apply
/// boundaries after deferred commands are applied. Engine publication policy is
/// dispatched at those boundaries.
pub(crate) fn run_frame(world: &mut World, scheduler: &mut Runtime) -> Result<()> {
    run_schedule_with_publication::<PreUpdate>(world, scheduler)?;
    run_fixed_update_frame(world, scheduler)?;
    run_schedule_with_publication::<Update>(world, scheduler)?;
    run_schedule_with_publication::<RenderPrepare>(world, scheduler)?;
    run_schedule_with_publication::<RenderSubmit>(world, scheduler)?;
    run_schedule_with_publication::<FrameEnd>(world, scheduler)?;
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
    fn frame_end_failure_is_propagated() {
        fn fail_frame_end() -> anyhow::Result<()> {
            Err(anyhow!("frame end failure"))
        }

        let mut world = test_world();
        let mut runtime = Runtime::new();
        runtime.add_systems::<FrameEnd, _, _>(&mut world, fail_frame_end);

        let err = run_frame(&mut world, &mut runtime).expect_err("frame should fail");
        assert!(format!("{err:#}").contains("frame end failure"));
    }
}
