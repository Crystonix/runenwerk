// Owner: ecs World - World State and Construction
use super::change_tracking::{
    ComponentChangeRecord, ComponentMeta, RemovedComponentRecord, ResourceChangeRecord,
    ResourceMeta,
};
use super::component_indexes::{ComponentIndexKey, ComponentIndexStorage};
use super::messaging::broadcast::{
    BroadcastObserver, BroadcastObserverNotification, BroadcastStreamStorage,
};
use super::messaging::finalization::MessagingFinalizationCounters;
use super::messaging::tick_buffer::TickBufferStorage;
use super::messaging::work_queue::WorkQueueStorage;
use super::ownership::OwnershipRegistry;
use crate::entity::{Entity, EntityAllocator, WorldScopeId};
use crate::indexing::SpatialIndexStorage;
use crate::storage::{ArchetypeRegistry, EntityLocationMap};
use std::any::{Any, TypeId};
use std::cell::RefCell;
use std::collections::{BTreeSet, HashMap};

pub struct World {
    pub(super) allocator: EntityAllocator,
    pub(super) alive_entities: BTreeSet<Entity>,

    pub(super) component_type_registry: HashMap<TypeId, ComponentMeta>,
    pub(super) reflected_component_types:
        HashMap<TypeId, crate::reflect::ReflectedComponentRegistration>,
    pub(super) reflected_component_order: Vec<TypeId>,
    pub(super) reflected_resource_types:
        HashMap<TypeId, crate::reflect::ReflectedResourceRegistration>,
    pub(super) reflected_resource_order: Vec<TypeId>,
    pub(super) type_registry: crate::reflect::TypeRegistry,

    pub(super) next_component_id: u32,
    pub(super) next_resource_id: u32,
    pub(super) resources: HashMap<TypeId, Box<dyn Any>>,
    pub(super) resource_type_registry: HashMap<TypeId, ResourceMeta>,

    pub(super) broadcast_streams: HashMap<TypeId, BroadcastStreamStorage>,
    pub(super) broadcast_observers: HashMap<String, BroadcastObserver>,
    pub(super) broadcast_observer_notifications: Vec<BroadcastObserverNotification>,
    pub(super) work_queues: HashMap<TypeId, WorkQueueStorage>,
    pub(super) tick_buffers: HashMap<TypeId, TickBufferStorage>,
    pub(super) next_broadcast_key: u64,
    pub(super) next_work_queue_key: u64,
    pub(super) next_tick_buffer_key: u64,
    pub(super) current_buffer_tick: u64,
    pub(super) finalized_buffer_tick: Option<u64>,
    pub(super) current_frame_index: u64,
    pub(super) messaging_finalization_counters: MessagingFinalizationCounters,
    pub(super) ownership: OwnershipRegistry,

    pub(super) component_indexes:
        RefCell<HashMap<ComponentIndexKey, Box<dyn ComponentIndexStorage>>>,
    pub(super) spatial_indexes: HashMap<String, Box<dyn SpatialIndexStorage>>,

    pub(super) archetype_registry: ArchetypeRegistry,
    pub(super) entity_locations: EntityLocationMap,

    pub(super) change_tick: u64,
    pub(super) component_change_ticks: HashMap<TypeId, u64>,
    pub(super) resource_change_ticks: HashMap<TypeId, u64>,
    pub(super) component_change_log: Vec<ComponentChangeRecord>,
    pub(super) removed_component_records: HashMap<TypeId, Vec<RemovedComponentRecord>>,
    pub(super) resource_change_log: Vec<ResourceChangeRecord>,
}

impl World {
    pub fn new() -> Self {
        Self {
            allocator: EntityAllocator::new(),
            alive_entities: BTreeSet::new(),

            component_type_registry: HashMap::new(),
            reflected_component_types: HashMap::new(),
            reflected_component_order: Vec::new(),
            reflected_resource_types: HashMap::new(),
            reflected_resource_order: Vec::new(),
            type_registry: crate::reflect::TypeRegistry::new(),

            next_component_id: 0,
            next_resource_id: 0,
            resources: HashMap::new(),
            resource_type_registry: HashMap::new(),

            broadcast_streams: HashMap::new(),
            broadcast_observers: HashMap::new(),
            broadcast_observer_notifications: Vec::new(),
            work_queues: HashMap::new(),
            tick_buffers: HashMap::new(),
            next_broadcast_key: 0,
            next_work_queue_key: 0,
            next_tick_buffer_key: 0,
            current_buffer_tick: 0,
            finalized_buffer_tick: None,
            current_frame_index: 0,
            messaging_finalization_counters: MessagingFinalizationCounters::default(),
            ownership: OwnershipRegistry::default(),

            component_indexes: RefCell::new(HashMap::new()),
            spatial_indexes: HashMap::new(),

            archetype_registry: ArchetypeRegistry::new(),
            entity_locations: Default::default(),

            change_tick: 0,
            component_change_ticks: HashMap::new(),
            resource_change_ticks: HashMap::new(),
            component_change_log: Vec::new(),
            removed_component_records: HashMap::new(),
            resource_change_log: Vec::new(),
        }
    }

    pub(crate) const fn scope_id(&self) -> WorldScopeId {
        self.allocator.scope_id()
    }
}

impl Default for World {
    fn default() -> Self {
        Self::new()
    }
}
