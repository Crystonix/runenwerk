// Owner: ecs World - Change Tracking Types
use crate::entity::Entity;
use std::any::TypeId;

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct ComponentTypeKey(pub u32);

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct ResourceTypeKey(pub u32);

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ComponentChangeKind {
    Added,
    Modified,
    Removed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComponentChangeRecord {
    pub tick: u64,
    pub frame: u64,
    pub entity: Entity,
    pub component_type: TypeId,
    pub component_key: ComponentTypeKey,
    pub component_name: &'static str,
    pub kind: ComponentChangeKind,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(super) struct RemovedComponentRecord {
    pub(super) tick: u64,
    pub(super) entity: Entity,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ResourceChangeKind {
    Inserted,
    Modified,
    Removed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceChangeRecord {
    pub tick: u64,
    pub frame: u64,
    pub resource_type: TypeId,
    pub resource_key: ResourceTypeKey,
    pub resource_name: &'static str,
    pub kind: ResourceChangeKind,
}

#[derive(Debug)]
pub(super) struct ComponentMeta {
    pub(super) id: ComponentTypeKey,
    pub(super) name: &'static str,
}

#[derive(Debug)]
pub(super) struct ResourceMeta {
    pub(super) id: ResourceTypeKey,
    pub(super) name: &'static str,
}

impl super::World {
    /// Advance the frame index used exclusively by component/resource change records.
    ///
    /// Runenwerk calls this only after the complete host `FrameEnd` schedule succeeds.
    pub fn advance_change_frame(&mut self) {
        self.current_frame_index = self.current_frame_index.saturating_add(1);
    }
}
