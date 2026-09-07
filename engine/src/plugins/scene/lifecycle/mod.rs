mod overlay_update;
mod setup;
mod transitions;
mod world_update;

use self::overlay_update::scene_overlay_update_system;
use self::setup::scene_setup_system;
use self::transitions::scene_transition_system;
use self::world_update::world_scene_update_system;
use crate::app::App;
use crate::runtime::{CoreSet, FixedUpdate, PreUpdate, Startup, SystemConfigExt, Update};

pub(crate) fn install_scene_runtime_systems(app: &mut App) {
    app.add_systems(Startup, scene_setup_system);
    app.add_systems(PreUpdate, scene_transition_system.in_set(CoreSet::Scene));
    app.add_systems(
        FixedUpdate,
        world_scene_update_system.in_set(CoreSet::Scene),
    );
    app.add_systems(Update, scene_overlay_update_system.in_set(CoreSet::Scene));
}
