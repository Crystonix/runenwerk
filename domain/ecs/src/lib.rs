extern crate self as ecs;

mod bundle;
mod commands;
mod component;
mod entity;
mod errors;
mod indexing;
pub mod prelude;
pub mod query;
pub mod reflect;
mod storage;
pub mod system;
pub mod telemetry;
mod world;
pub use bundle::Bundle;
#[doc(hidden)]
pub use bundle::{BundleComponentDescriptor, BundleComponents};
pub use commands::{BatchCommands, Commands, DeferredCommand};
pub use component::{Component, ComponentState, Resource, StatefulComponent};
pub use ecs_macros::{Bundle, Component, Reflect, Resource, StatefulComponent, SystemParam};
pub use entity::{Entity, EntityAllocator};
pub use errors::{
    CommandError, EntityAllocationError, EntityError, QueryError, ResourceError, SpatialIndexError,
};
pub use indexing::{DEFAULT_SPATIAL_INDEX_NAME, SpatialHashConfig, SpatialHashIndex, SpatialIndex};
pub use query::{
    Added, Changed, Orphaned, Query, QueryAccess, QueryOrphaned, QueryOrphanedState, QueryState,
    QueryTypeAccess, With, Without, query_snapshot_source_generation,
};
pub use reflect::{
    EnumInfo, EnumVariantInfo, FieldInfo, Reflect, ReflectShape, ReflectValueMut, ReflectValueRef,
    StructInfo, StructValueMut, StructValueRef, TypeInfo, TypeRegistry,
};
pub use system::{
    BroadcastReader, BroadcastReaderState, BroadcastWriter, ConfiguredSystem, IntoSystem,
    IntoSystemConfigs, IntoSystemSetKey, ParamSlotDescriptor, ParamSlotId, ParamSlotMetadata, Res,
    ResMut, ResView, Runtime, RuntimePlanBarrierReport, RuntimePlanConflictReport,
    RuntimePlanDiagnosticReport, RuntimePlanPhaseReport, RuntimePlanReport, RuntimePlanStageReport,
    RuntimePlanSystemReport, RuntimePlanWaveReport, SystemConfigExt, SystemId, SystemParam,
    SystemParamContext, SystemParamError, TickBufferDrainer, TickBufferReader, TickBufferWriter,
    WorkQueueDrainer, WorkQueueReader, WorkQueueWriter,
};
pub use world::{
    BroadcastDiagnosticsSnapshot, BroadcastKey, BroadcastLifetime, BroadcastObserverNotification,
    BroadcastObserverTrigger, BroadcastOverflowPolicy, BroadcastStreamConfig, BroadcastStreamStats,
    BroadcastTracingPolicy, ChangeExtractionFilter, ChangeExtractionWindow, ComponentChangeKind,
    ComponentChangeRecord, ComponentStructuralDelta, ComponentTypeKey, EntityDespawnedEvent,
    EntityMut, EntityRef, EntitySpawnedEvent, MessagingDiagnosticsSnapshot,
    MessagingFinalizationCounters, Mut, OwnerId, OwnerRole, OwnerState, OwnershipTarget,
    OwnershipTransferRecord, ResourceChangeKind, ResourceChangeRecord, ResourceOwnerKey,
    ResourceOwnershipDescriptor, ResourceStructuralDelta, ResourceTypeKey, StructuralDeltaBatch,
    StructuralDeltaRef, TickBufferConfig, TickBufferDiagnosticsSnapshot, TickBufferKey,
    TickBufferMeta, TickBufferProvenance, TickBufferPushError, TickBufferRecord,
    TickBufferRecordRef, TickBufferStats, WorkQueueConfig, WorkQueueDiagnosticsSnapshot,
    WorkQueueEnqueueError, WorkQueueKey, WorkQueueStats, World,
};
