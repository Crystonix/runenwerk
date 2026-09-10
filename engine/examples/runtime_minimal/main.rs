use anyhow::Result;
use engine::prelude::*;

#[derive(Debug, Copy, Clone, PartialEq, Component, runen_ecs::Resource)]
struct Position {
    x: i32,
    y: i32,
}

#[derive(Debug, Copy, Clone, PartialEq, Component, runen_ecs::Resource)]
struct Velocity {
    x: i32,
    y: i32,
}

#[derive(Debug, Default, Component, runen_ecs::Resource)]
struct FrameCounter(u32);

struct RuntimeMinimalPlugin;

impl Plugin for RuntimeMinimalPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FrameCounter>();
        app.add_systems(Startup, setup);
        app.add_systems(Update, movement);
    }
}

fn setup(mut commands: Commands) {
    commands.spawn((Position { x: 0, y: 0 }, Velocity { x: 1, y: 1 }));
}

fn movement(mut query: Query<(&mut Position, &Velocity)>, mut frames: ResMut<FrameCounter>) {
    for (position, velocity) in query.iter() {
        position.x += velocity.x;
        position.y += velocity.y;
    }
    frames.0 += 1;
}

fn main() -> Result<()> {
    let mut app = App::new();
    app.add_plugin(RuntimeMinimalPlugin);
    let app = app.run_for_frames(3)?;

    let frame_count = app.world().resource::<FrameCounter>()?.0;
    let world = app.world();
    let query = world.query_state::<&Position, ()>();
    let positions: Vec<_> = query
        .iter(world)
        .map(|position| (position.x, position.y))
        .collect();

    println!("frames={frame_count} positions={positions:?}");
    Ok(())
}
