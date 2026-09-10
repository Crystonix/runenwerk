//! File: domain/editor/editor_inspector/src/bridge/ecs_bridge.rs
//! Purpose: Stable bridge contracts between editor-owned IDs and ECS runtime IDs.

use std::any::TypeId;
use std::collections::HashMap;

use editor_core::{ComponentTypeId, EntityId, ResourceTypeId};

pub trait EcsInspectorBridge {
    fn resolve_entity(&self, entity_id: EntityId) -> Option<runen_ecs::Entity>;

    fn resolve_component_rust_type_id(&self, component_type: ComponentTypeId) -> Option<TypeId>;

    fn resolve_resource_rust_type_id(&self, resource_type: ResourceTypeId) -> Option<TypeId>;
}

#[derive(Debug, Default, Clone)]
pub struct StaticEcsInspectorBridge {
    entity_ids: HashMap<EntityId, runen_ecs::Entity>,
    component_type_ids: HashMap<ComponentTypeId, TypeId>,
    resource_type_ids: HashMap<ResourceTypeId, TypeId>,
}

impl StaticEcsInspectorBridge {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_entity(mut self, editor_id: EntityId, ecs_entity: runen_ecs::Entity) -> Self {
        self.entity_ids.insert(editor_id, ecs_entity);
        self
    }

    pub fn with_component_type<T: 'static>(mut self, editor_id: ComponentTypeId) -> Self {
        self.component_type_ids.insert(editor_id, TypeId::of::<T>());
        self
    }

    pub fn with_resource_type<T: 'static>(mut self, editor_id: ResourceTypeId) -> Self {
        self.resource_type_ids.insert(editor_id, TypeId::of::<T>());
        self
    }
}

impl EcsInspectorBridge for StaticEcsInspectorBridge {
    fn resolve_entity(&self, entity_id: EntityId) -> Option<runen_ecs::Entity> {
        self.entity_ids.get(&entity_id).copied()
    }

    fn resolve_component_rust_type_id(&self, component_type: ComponentTypeId) -> Option<TypeId> {
        self.component_type_ids.get(&component_type).copied()
    }

    fn resolve_resource_rust_type_id(&self, resource_type: ResourceTypeId) -> Option<TypeId> {
        self.resource_type_ids.get(&resource_type).copied()
    }
}
