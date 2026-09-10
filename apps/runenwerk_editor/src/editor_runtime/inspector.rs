use std::any::TypeId;

use editor_core::{ComponentTypeId, EntityId, ResourceTypeId};
use editor_inspector::EcsInspectorBridge;

use crate::editor_runtime::EditorRuntimeIdRegistry;

pub struct RunenwerkEditorInspectorBridge<'a> {
    ids: &'a EditorRuntimeIdRegistry,
}

impl<'a> RunenwerkEditorInspectorBridge<'a> {
    pub fn new(ids: &'a EditorRuntimeIdRegistry) -> Self {
        Self { ids }
    }
}

impl<'a> EcsInspectorBridge for RunenwerkEditorInspectorBridge<'a> {
    fn resolve_entity(&self, entity_id: EntityId) -> Option<runen_ecs::Entity> {
        self.ids.resolve_entity(entity_id)
    }

    fn resolve_component_rust_type_id(&self, component_type: ComponentTypeId) -> Option<TypeId> {
        self.ids.resolve_component_rust_type_id(component_type)
    }

    fn resolve_resource_rust_type_id(&self, resource_type: ResourceTypeId) -> Option<TypeId> {
        self.ids.resolve_resource_rust_type_id(resource_type)
    }
}
