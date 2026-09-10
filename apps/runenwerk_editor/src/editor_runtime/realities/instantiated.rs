//! File: apps/runenwerk_editor/src/editor_runtime/realities/instantiated.rs
//! Purpose: Read-only instantiated-reality boundary for ECS and runtime id projection.

use editor_core::EntityId;

use crate::editor_runtime::EditorRuntimeIdRegistry;

#[derive(Clone, Copy)]
pub struct InstantiatedSceneReality<'a> {
    world: &'a runen_ecs::World,
    identities: &'a EditorRuntimeIdRegistry,
}

impl<'a> InstantiatedSceneReality<'a> {
    pub fn new(world: &'a runen_ecs::World, identities: &'a EditorRuntimeIdRegistry) -> Self {
        Self { world, identities }
    }

    pub fn world(&self) -> &'a runen_ecs::World {
        self.world
    }

    pub fn identities(&self) -> &'a EditorRuntimeIdRegistry {
        self.identities
    }

    pub fn resolve_entity(&self, entity: EntityId) -> Option<runen_ecs::Entity> {
        self.identities.resolve_entity(entity)
    }
}
