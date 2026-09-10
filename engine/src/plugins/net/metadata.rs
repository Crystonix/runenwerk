use engine_net::replication::{
    NetComponentMetadata, NetEntityMetadata, NetResourceMetadata, ReplicatedComponentDescriptor,
    ReplicatedEntityDescriptor, ReplicatedResourceDescriptor, ReplicationRegistry,
};
use runen_ecs::World;

#[derive(Debug, Clone, Default, runen_ecs::Component, runen_ecs::Resource)]
pub struct NetworkReplicationMetadata {
    registry: ReplicationRegistry,
}

impl NetworkReplicationMetadata {
    pub fn registry(&self) -> &ReplicationRegistry {
        &self.registry
    }

    pub fn registry_mut(&mut self) -> &mut ReplicationRegistry {
        &mut self.registry
    }

    pub fn register_component_descriptor(&mut self, descriptor: ReplicatedComponentDescriptor) {
        self.registry.register_component_descriptor(descriptor);
    }

    pub fn register_entity_descriptor(&mut self, descriptor: ReplicatedEntityDescriptor) {
        self.registry.register_entity_descriptor(descriptor);
    }

    pub fn register_resource_descriptor(&mut self, descriptor: ReplicatedResourceDescriptor) {
        self.registry.register_resource_descriptor(descriptor);
    }
}

pub fn register_component_metadata<T: NetComponentMetadata>(world: &mut World) {
    if let Ok(metadata) = world.resource_mut::<NetworkReplicationMetadata>() {
        metadata.registry_mut().register_component::<T>();
    }
}

pub fn register_entity_metadata<T: NetEntityMetadata>(world: &mut World) {
    if let Ok(metadata) = world.resource_mut::<NetworkReplicationMetadata>() {
        metadata.registry_mut().register_entity::<T>();
    }
}

pub fn register_resource_metadata<T: NetResourceMetadata>(world: &mut World) {
    if let Ok(metadata) = world.resource_mut::<NetworkReplicationMetadata>() {
        metadata.registry_mut().register_resource::<T>();
    }
}

pub fn replication_registry(world: &World) -> Option<&ReplicationRegistry> {
    world
        .resource::<NetworkReplicationMetadata>()
        .ok()
        .map(|metadata| metadata.registry())
}
