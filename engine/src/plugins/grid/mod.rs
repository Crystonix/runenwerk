use crate::app::App;
use crate::plugin::Plugin;
use crate::runtime::{CoreSet, Res, ResMut, SystemConfigExt, Update};
use crate::state::GameplayRuntimeConfig;

pub struct GridPlugin;

#[derive(Debug, Copy, Clone, Default, runen_ecs::Component, runen_ecs::Resource)]
pub struct GridRuntimeConfig {
    pub chunk_size: f32,
    pub chunk_load_radius: u32,
    pub infinite_world: bool,
}

impl Plugin for GridPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GameplayRuntimeConfig>();
        app.init_resource::<GridRuntimeConfig>();
        app.add_systems(Update, grid_prepare_system.in_set(CoreSet::Scene));
    }
}

fn grid_prepare_system(gameplay: Res<GameplayRuntimeConfig>, mut grid: ResMut<GridRuntimeConfig>) {
    *grid = GridRuntimeConfig {
        chunk_size: gameplay.chunk_size,
        chunk_load_radius: gameplay.chunk_load_radius,
        infinite_world: gameplay.infinite_world,
    };
}

#[cfg(test)]
mod tests {
    use super::{GameplayRuntimeConfig, GridPlugin, GridRuntimeConfig};
    use crate::prelude::*;

    #[test]
    fn grid_plugin_publishes_runtime_config_from_scene_state() {
        let mut app = App::headless();
        app.insert_resource(GameplayRuntimeConfig {
            chunk_size: 48.0,
            chunk_load_radius: 5,
            infinite_world: false,
        });
        app.add_plugin(GridPlugin);
        let app = app.run_for_frames(1).expect("grid plugin should run");

        let grid = app.world().resource::<GridRuntimeConfig>().unwrap();
        assert_eq!(grid.chunk_size, 48.0);
        assert_eq!(grid.chunk_load_radius, 5);
        assert!(!grid.infinite_world);
    }
}
