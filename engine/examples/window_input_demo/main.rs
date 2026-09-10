use anyhow::Result;
use engine::plugins::default_plugins;
use engine::plugins::input::domain::action;
use engine::prelude::*;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Component, runen_ecs::Resource)]
struct Player;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Component, runen_ecs::Resource)]
struct Position {
    x: i32,
    y: i32,
}

struct WindowInputDemoPlugin;

impl Plugin for WindowInputDemoPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(default_plugins());
        app.add_systems(Startup, setup);
        app.add_systems(
            Update,
            update_demo.after(CoreSet::Input).after(CoreSet::Time),
        );
    }
}

fn setup(mut commands: Commands) {
    commands.spawn((Player, Position { x: 0, y: 0 }));
}

fn update_demo(
    input: Res<InputState>,
    time: Res<Time>,
    mut window: ResMut<WindowState>,
    mut query: Query<&mut Position>,
) {
    let position = query.single().expect("demo should have one player");
    if input.action_down(action::WORLD_MOVE_LEFT) {
        position.x -= 1;
    }
    if input.action_down(action::WORLD_MOVE_RIGHT) {
        position.x += 1;
    }
    if input.action_down(action::WORLD_MOVE_UP) {
        position.y -= 1;
    }
    if input.action_down(action::WORLD_MOVE_DOWN) {
        position.y += 1;
    }

    if input.action_pressed(action::SYSTEM_TOGGLE_PAUSE_MENU) {
        window.request_close();
    }

    window.set_title(format!(
        "Window Input Demo | pos=({}, {}) dt={:.4}",
        position.x, position.y, time.delta_seconds
    ));
}

fn main() -> Result<()> {
    let mut app = App::new();
    app.set_title("Window Input Demo");
    app.add_plugin(WindowInputDemoPlugin);
    app.run()
}
