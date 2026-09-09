use crate::app::App;
use crate::plugin::Plugin;
use crate::runtime::{
    CoreSet, FixedUpdate, IntoSystemSetKey, RenderPrepare, Res, ResMut, SystemConfigExt,
};
use ecs::SystemSetKey;
use engine_sim::{AuthorityRole, SimulationProfileConfig};

use super::build::integration::{
    integrate_completed_build_outputs_system, sync_world_runtime_debug_metrics_system,
};
use super::build::jobs::{dispatch_world_build_jobs_system, sync_world_build_debug_metrics_system};
use super::chunks::lifecycle::advance_chunk_lifecycle_system;
use super::chunks::render_cache_bridge::{
    WorldRenderCacheInvalidationQueueResource, flush_world_render_cache_invalidations_system,
};
use super::debug::metrics::WorldDebugMetricsResource;
use super::prepare::contributions::prepare_world_feature_contributions_system;
use super::streaming::replication::{
    WorldReplicationExtractionCursor, rebuild_world_replication_state_system,
};
use super::{
    adapters::resources::{
        BuildGraphResource, BuildQueueResource, CaveLightingScopeResource, CavePortalGraphResource,
        CaveSectorResource, CollisionQueryServiceResource, OperationLogResource,
        PartitionConfigResource, RegionInvalidationJournalResource, ReplicationStateResource,
        SdfChunkStoreResource, WorldQuantizationScaleResource,
    },
    chunks::{DirtyChunkMapResource, lifecycle::WorldChunkRuntimeMapResource},
    queries::nav::WorldNavSummaryResource,
};
use world_ops::WorldRevision;

#[derive(Debug, Copy, Clone, PartialEq, Eq, ecs::Component, ecs::Resource, Default)]
pub enum WorldRuntimeMode {
    Writable,
    #[default]
    ReadOnly,
}

#[derive(Debug, Clone, ecs::Component, ecs::Resource)]
pub struct WorldRuntimeConfig {
    pub mode: WorldRuntimeMode,
}

impl Default for WorldRuntimeConfig {
    fn default() -> Self {
        Self {
            mode: WorldRuntimeMode::ReadOnly,
        }
    }
}

#[derive(Debug, Copy, Clone, Default, ecs::Component, ecs::Resource)]
pub struct WorldAuthorityState {
    pub world_revision: WorldRevision,
}

#[derive(Debug, Copy, Clone, Default, ecs::Component, ecs::Resource)]
pub struct WorldRuntimeState {
    pub integrated_build_outputs: u64,
    pub dropped_stale_build_outputs: u64,
}

pub struct WorldPlugin;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum WorldRuntimeSet {
    ModeSync,
    Lifecycle,
    BuildDispatch,
    BuildMetrics,
    BuildIntegrate,
    RenderCacheSync,
    ReplicationState,
}

impl IntoSystemSetKey for WorldRuntimeSet {
    fn system_set_key(&self) -> SystemSetKey {
        match self {
            Self::ModeSync => SystemSetKey::of::<WorldRuntimeSet>("WorldRuntimeSet::ModeSync"),
            Self::Lifecycle => SystemSetKey::of::<WorldRuntimeSet>("WorldRuntimeSet::Lifecycle"),
            Self::BuildDispatch => {
                SystemSetKey::of::<WorldRuntimeSet>("WorldRuntimeSet::BuildDispatch")
            }
            Self::BuildMetrics => {
                SystemSetKey::of::<WorldRuntimeSet>("WorldRuntimeSet::BuildMetrics")
            }
            Self::BuildIntegrate => {
                SystemSetKey::of::<WorldRuntimeSet>("WorldRuntimeSet::BuildIntegrate")
            }
            Self::RenderCacheSync => {
                SystemSetKey::of::<WorldRuntimeSet>("WorldRuntimeSet::RenderCacheSync")
            }
            Self::ReplicationState => {
                SystemSetKey::of::<WorldRuntimeSet>("WorldRuntimeSet::ReplicationState")
            }
        }
    }
}

pub fn world_runtime_mode_for_authority(authority: AuthorityRole) -> WorldRuntimeMode {
    match authority {
        AuthorityRole::Client => WorldRuntimeMode::ReadOnly,
        AuthorityRole::Local | AuthorityRole::Server | AuthorityRole::Peer => {
            WorldRuntimeMode::Writable
        }
    }
}

fn sync_world_runtime_mode_system(
    simulation_profile: Res<SimulationProfileConfig>,
    mut runtime_config: ResMut<WorldRuntimeConfig>,
) {
    runtime_config.mode = world_runtime_mode_for_authority(simulation_profile.authority);
}

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WorldRuntimeConfig>();
        app.init_resource::<WorldRuntimeState>();
        app.init_resource::<WorldAuthorityState>();
        app.init_resource::<PartitionConfigResource>();
        app.init_resource::<WorldQuantizationScaleResource>();
        app.init_resource::<WorldChunkRuntimeMapResource>();
        app.init_resource::<DirtyChunkMapResource>();
        app.init_resource::<WorldRenderCacheInvalidationQueueResource>();
        app.init_resource::<SdfChunkStoreResource>();
        app.init_resource::<OperationLogResource>();
        app.init_resource::<RegionInvalidationJournalResource>();
        app.init_resource::<BuildGraphResource>();
        app.init_resource::<BuildQueueResource>();
        app.init_resource::<super::build::jobs::WorldBuildJobRuntimeResource>();
        app.init_resource::<super::build::integration::WorldCompletedBuildQueueResource>();
        app.init_resource::<CollisionQueryServiceResource>();
        app.init_resource::<WorldNavSummaryResource>();
        app.init_resource::<ReplicationStateResource>();
        app.init_resource::<WorldReplicationExtractionCursor>();
        app.init_resource::<CaveSectorResource>();
        app.init_resource::<CavePortalGraphResource>();
        app.init_resource::<CaveLightingScopeResource>();
        app.init_resource::<WorldDebugMetricsResource>();

        let authority = app
            .world()
            .resource::<SimulationProfileConfig>()
            .map(|config| config.authority)
            .ok();
        if let (Some(authority), Ok(runtime_config)) = (
            authority,
            app.world_mut().resource_mut::<WorldRuntimeConfig>(),
        ) {
            runtime_config.mode = world_runtime_mode_for_authority(authority);
        }

        app.add_systems(
            FixedUpdate,
            sync_world_runtime_mode_system
                .in_set(WorldRuntimeSet::ModeSync)
                .in_set(CoreSet::Simulation)
                .before(WorldRuntimeSet::Lifecycle),
        );
        app.add_systems(
            FixedUpdate,
            advance_chunk_lifecycle_system
                .in_set(WorldRuntimeSet::Lifecycle)
                .in_set(CoreSet::Simulation)
                .after(WorldRuntimeSet::ModeSync),
        );
        app.add_systems(
            FixedUpdate,
            dispatch_world_build_jobs_system
                .in_set(WorldRuntimeSet::BuildDispatch)
                .in_set(CoreSet::Simulation)
                .after(WorldRuntimeSet::Lifecycle),
        );
        app.add_systems(
            FixedUpdate,
            sync_world_build_debug_metrics_system
                .in_set(WorldRuntimeSet::BuildMetrics)
                .in_set(CoreSet::Simulation)
                .after(WorldRuntimeSet::BuildDispatch),
        );
        app.add_systems(
            FixedUpdate,
            integrate_completed_build_outputs_system
                .in_set(WorldRuntimeSet::BuildIntegrate)
                .in_set(CoreSet::Simulation)
                .after(WorldRuntimeSet::BuildMetrics),
        );
        app.add_systems(
            FixedUpdate,
            sync_world_runtime_debug_metrics_system
                .in_set(CoreSet::Simulation)
                .after(WorldRuntimeSet::BuildIntegrate)
                .before(WorldRuntimeSet::RenderCacheSync),
        );
        app.add_systems(
            FixedUpdate,
            flush_world_render_cache_invalidations_system
                .in_set(WorldRuntimeSet::RenderCacheSync)
                .in_set(CoreSet::Simulation)
                .after(WorldRuntimeSet::BuildIntegrate),
        );
        app.add_systems(
            FixedUpdate,
            rebuild_world_replication_state_system
                .in_set(WorldRuntimeSet::ReplicationState)
                .in_set(CoreSet::Simulation)
                .after(WorldRuntimeSet::RenderCacheSync),
        );
        app.add_systems(RenderPrepare, prepare_world_feature_contributions_system);
    }
}
