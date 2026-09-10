use runen_spatial::*;
use std::ops::{Deref, DerefMut};
use world_ops::{
    BuildGraph, BuildQueue, DirtyChunkMap, OperationLog, RegionInvalidationJournal,
    ReplicationState, WorldQuantizationScale,
};
use world_sdf::{
    CaveLightingScope, CavePortalGraph, CaveSectorStore, CollisionQueryService, SdfChunkStore,
};

macro_rules! resource_wrapper {
    ($name:ident, $inner:ty) => {
        #[derive(Debug, Clone, Default, runen_ecs::Resource)]
        pub struct $name(pub $inner);

        impl Deref for $name {
            type Target = $inner;

            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }

        impl DerefMut for $name {
            fn deref_mut(&mut self) -> &mut Self::Target {
                &mut self.0
            }
        }
    };
}

resource_wrapper!(PartitionConfigResource, GridPartitionConfig);
resource_wrapper!(WorldQuantizationScaleResource, WorldQuantizationScale);
resource_wrapper!(DirtyChunkMapResource, DirtyChunkMap);
resource_wrapper!(SdfChunkStoreResource, SdfChunkStore);
resource_wrapper!(OperationLogResource, OperationLog);
resource_wrapper!(RegionInvalidationJournalResource, RegionInvalidationJournal);
resource_wrapper!(BuildGraphResource, BuildGraph);
resource_wrapper!(BuildQueueResource, BuildQueue);
resource_wrapper!(ReplicationStateResource, ReplicationState);
resource_wrapper!(CaveSectorResource, CaveSectorStore);
resource_wrapper!(CavePortalGraphResource, CavePortalGraph);
resource_wrapper!(CaveLightingScopeResource, CaveLightingScope);

#[derive(Debug, Copy, Clone, Default, runen_ecs::Resource)]
pub struct CollisionQueryServiceResource(pub CollisionQueryService);

impl Deref for CollisionQueryServiceResource {
    type Target = CollisionQueryService;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for CollisionQueryServiceResource {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
