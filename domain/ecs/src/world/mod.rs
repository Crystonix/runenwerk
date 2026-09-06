mod capability;
pub mod change_extraction;
mod change_tracking;
mod component_indexes;
mod entity_handles;
pub mod messaging;
mod reflection;
mod runtime;
mod state;

pub mod component;
pub mod entity;
pub mod ownership;
pub mod resource;
pub mod spatial;

pub use change_extraction::{
    ChangeExtractionFilter, ChangeExtractionWindow, ComponentStructuralDelta,
    ResourceStructuralDelta, StructuralDeltaBatch, StructuralDeltaRef,
};
pub use change_tracking::{
    ComponentChangeKind, ComponentChangeRecord, ComponentTypeKey, ResourceChangeKind,
    ResourceChangeRecord, ResourceTypeKey,
};
pub use entity_handles::{EntityMut, EntityRef, Mut};
pub use messaging::{
    BroadcastDiagnosticsSnapshot, BroadcastKey, BroadcastLifetime, BroadcastObserverNotification,
    BroadcastObserverTrigger, BroadcastOverflowPolicy, BroadcastStreamConfig, BroadcastStreamStats,
    BroadcastTracingPolicy, EntityDespawnedEvent, EntitySpawnedEvent, MessagingDiagnosticsSnapshot,
    MessagingFinalizationCounters, TickBufferConfig, TickBufferDiagnosticsSnapshot, TickBufferKey,
    TickBufferMeta, TickBufferProvenance, TickBufferPushError, TickBufferRecord,
    TickBufferRecordRef, TickBufferStats, WorkQueueConfig, WorkQueueDiagnosticsSnapshot,
    WorkQueueEnqueueError, WorkQueueKey, WorkQueueStats,
};
pub use ownership::{
    OwnerId, OwnerRole, OwnerState, OwnershipTarget, OwnershipTransferRecord, ResourceOwnerKey,
    ResourceOwnershipDescriptor,
};
pub use state::World;

pub(crate) use capability::{
    MessagingCapability, QueryCapability, ResourceCapability, ResourceMutationCapability,
    WorldAuthority,
};
