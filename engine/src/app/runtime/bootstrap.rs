use crate::app::App;
use crate::app::domain::mode::AppMode;
use crate::plugins::InputState;
use crate::prelude::Time;
use crate::runtime::platform::PlatformWindowEventQueueResource;
use crate::*;

impl App {
    /// Installs builtin runtime resources required by startup and frame execution.
    ///
    /// Lifecycle contract:
    /// - This runs during `App` construction before any plugin `Startup` systems can run.
    /// - Runners only mutate these resources during run preparation and per-frame execution.
    /// - Plugins may override defaults by inserting resources after app construction.
    pub(crate) fn install_builtin_resources(&mut self) {
        if self.world.resource::<Time>().is_err() {
            self.world.insert_resource(Time::new());
        }
        if self.world.resource::<InputState>().is_err() {
            self.world.insert_resource(InputState::new());
        }
        if self.world.resource::<WindowState>().is_err() {
            let state = match self.mode {
                AppMode::Windowed => WindowState::windowed(self.title.clone()),
                AppMode::Headless => WindowState::headless(self.title.clone()),
            };
            self.world.insert_resource(state);
        }
        if !self.world.has_resource::<FramePacingPolicyResource>() {
            self.world
                .insert_resource(FramePacingPolicyResource::default());
        }
        if !self.world.has_resource::<FramePacingRuntimeStateResource>() {
            self.world
                .insert_resource(FramePacingRuntimeStateResource::default());
        }
        if !self.world.has_resource::<WindowStateRegistryResource>() {
            let registry = self
                .world
                .resource::<WindowState>()
                .ok()
                .map(WindowStateRegistryResource::from_legacy);
            if let Some(registry) = registry {
                self.world.insert_resource(registry);
            }
        }
        if !self
            .world
            .has_resource::<PlatformWindowEventQueueResource>()
        {
            self.world
                .insert_resource(PlatformWindowEventQueueResource::default());
        }
        if !self
            .world
            .has_resource::<NativeWindowHookRegistryResource>()
        {
            self.world
                .insert_resource(NativeWindowHookRegistryResource::default());
        }
        if !self.world.has_resource::<SceneCatalog>() {
            self.world.insert_resource(SceneCatalog::default());
        }
        if !self.world.has_resource::<StartupState>() {
            self.world.insert_resource(StartupState::default());
        }
        if !self.world.has_resource::<SceneRuntimeState>() {
            self.world.insert_resource(SceneRuntimeState::default());
        }
        if !self.world.has_resource::<UiOverlayState>() {
            self.world.insert_resource(UiOverlayState::default());
        }
        if !self.world.has_resource::<GameplayRuntimeConfig>() {
            self.world.insert_resource(GameplayRuntimeConfig::default());
        }
        if !self.world.has_resource::<FixedTimeConfig>() {
            self.world.insert_resource(FixedTimeConfig::default());
        }
        if !self.world.has_resource::<CatchupBudget>() {
            self.world.insert_resource(CatchupBudget::default());
        }
        if !self.world.has_resource::<FixedTimeState>() {
            self.world.insert_resource(FixedTimeState::default());
        }
        if !self.world.has_resource::<SimulationTick>() {
            self.world.insert_resource(SimulationTick::default());
        }
        if !self
            .world
            .has_resource::<ProductPublicationRuntimeResource>()
        {
            self.world
                .insert_resource(ProductPublicationRuntimeResource::default());
        }
        if !self.world.has_resource::<QuerySnapshotRuntimeResource>() {
            self.world
                .insert_resource(QuerySnapshotRuntimeResource::default());
        }
        if !self.world.has_resource::<RuntimeJobExecutorResource>() {
            self.world
                .insert_resource(RuntimeJobExecutorResource::default());
        }
        if !self.world.has_resource::<RuntimeProductCacheResource>() {
            self.world
                .insert_resource(RuntimeProductCacheResource::default());
        }
        if !self.world.has_resource::<SimulationProfileConfig>() {
            self.world
                .insert_resource(SimulationProfileConfig::default());
        }
        self.add_product_publication_handler(publish_staged_product_outcomes);
        self.add_query_snapshot_publication_handler(publish_staged_query_snapshots);
        if !self.world.has_resource::<SimulationSessionId>() {
            self.world.insert_resource(SimulationSessionId::default());
        }
        if !self.world.has_resource::<SimulationSeed>() {
            let seed = SimulationSeed::default();
            self.world.insert_resource(seed);
            self.world.insert_resource(SimulationRng::from_seed(seed));
        } else if !self.world.has_resource::<SimulationRng>() {
            let seed = self
                .world
                .resource::<SimulationSeed>()
                .copied()
                .unwrap_or_default();
            self.world.insert_resource(SimulationRng::from_seed(seed));
        }
    }
}
