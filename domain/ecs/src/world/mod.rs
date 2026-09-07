mod capability;
pub mod change_extraction;
mod change_tracking;
mod component_indexes;
mod entity_handles;
mod reflection;
mod runtime;
mod state;

pub mod component;
pub mod entity;
pub mod ownership;
pub mod resource;

pub use change_extraction::{
    ChangeExtractionFilter, ChangeExtractionWindow, ComponentStructuralDelta,
    ResourceStructuralDelta, StructuralDeltaBatch, StructuralDeltaRef,
};
pub use change_tracking::{
    ComponentChangeKind, ComponentChangeRecord, ComponentTypeKey, ResourceChangeKind,
    ResourceChangeRecord, ResourceTypeKey,
};
pub use entity_handles::{EntityMut, EntityRef, Mut};
pub use ownership::{
    OwnerId, OwnerRole, OwnerState, OwnershipTarget, OwnershipTransferRecord, ResourceOwnerKey,
    ResourceOwnershipDescriptor,
};
pub use state::World;

pub(crate) use capability::{
    QueryCapability, ResourceCapability, ResourceMutationCapability, WorldAuthority,
};
