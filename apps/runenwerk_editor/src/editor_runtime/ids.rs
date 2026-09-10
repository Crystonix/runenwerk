use std::any::TypeId;
use std::collections::HashMap;

use editor_core::{ComponentTypeId, EditorMutationError, EntityId, ResourceTypeId};

type RemovedComponentKey = (EntityId, ComponentTypeId);
type RemovedComponentStore = HashMap<RemovedComponentKey, Box<dyn RemovedComponentValue>>;

trait RemovedComponentValue {
    fn restore(
        self: Box<Self>,
        world: &mut runen_ecs::World,
        entity: runen_ecs::Entity,
    ) -> Result<(), EditorMutationError>;
}

struct RemovedComponent<T> {
    value: T,
}

impl<T> RemovedComponentValue for RemovedComponent<T>
where
    T: runen_ecs::Component + 'static,
{
    fn restore(
        self: Box<Self>,
        world: &mut runen_ecs::World,
        entity: runen_ecs::Entity,
    ) -> Result<(), EditorMutationError> {
        world.insert(entity, self.value).map_err(|_| {
            EditorMutationError::runtime_rejected("failed to restore removed component")
        })
    }
}

struct SceneEntityRecord {
    ecs_entity: runen_ecs::Entity,
}

struct ComponentRegistration {
    rust_type_id: TypeId,
    display_name: String,
    insert_default: fn(&mut runen_ecs::World, runen_ecs::Entity) -> Result<(), EditorMutationError>,
    remove_and_capture: fn(
        &mut runen_ecs::World,
        runen_ecs::Entity,
        &mut RemovedComponentStore,
        RemovedComponentKey,
    ) -> Result<(), EditorMutationError>,
}

#[derive(Default)]
pub struct EditorRuntimeIdRegistry {
    next_entity_id: u64,
    entities: HashMap<EntityId, SceneEntityRecord>,
    component_types: HashMap<ComponentTypeId, ComponentRegistration>,
    resource_type_ids: HashMap<ResourceTypeId, TypeId>,
    removed_components: RemovedComponentStore,
}

impl EditorRuntimeIdRegistry {
    pub fn new() -> Self {
        Self {
            next_entity_id: 1,
            ..Self::default()
        }
    }

    pub fn allocate_entity_id(&mut self) -> EntityId {
        let id = EntityId(self.next_entity_id);
        self.next_entity_id += 1;
        id
    }

    pub fn register_entity(&mut self, editor_id: EntityId, ecs_entity: runen_ecs::Entity) {
        self.next_entity_id = self.next_entity_id.max(editor_id.0.saturating_add(1));
        self.entities
            .insert(editor_id, SceneEntityRecord { ecs_entity });
    }

    pub fn unregister_entity(&mut self, editor_id: EntityId) -> Option<runen_ecs::Entity> {
        self.entities
            .remove(&editor_id)
            .map(|record| record.ecs_entity)
    }

    pub fn resolve_entity(&self, entity_id: EntityId) -> Option<runen_ecs::Entity> {
        self.entities
            .get(&entity_id)
            .map(|record| record.ecs_entity)
    }

    pub fn entity_ids(&self) -> impl Iterator<Item = EntityId> + '_ {
        self.entities.keys().copied()
    }

    pub fn clear_scene_entities(&mut self) {
        self.entities.clear();
        self.removed_components.clear();
        self.next_entity_id = 1;
    }

    pub fn clear_removed_component_cache(&mut self) {
        self.removed_components.clear();
    }

    pub fn register_component_type<T>(&mut self, editor_id: ComponentTypeId)
    where
        T: runen_ecs::Component + runen_ecs::Reflect + Default + 'static,
    {
        self.component_types.insert(
            editor_id,
            ComponentRegistration {
                rust_type_id: TypeId::of::<T>(),
                display_name: short_type_name::<T>(),
                insert_default: insert_default_component::<T>,
                remove_and_capture: remove_and_capture_component::<T>,
            },
        );
    }

    pub fn component_type_ids(&self) -> impl Iterator<Item = ComponentTypeId> + '_ {
        self.component_types.keys().copied()
    }

    pub fn resolve_component_rust_type_id(
        &self,
        component_type: ComponentTypeId,
    ) -> Option<TypeId> {
        self.component_types
            .get(&component_type)
            .map(|registration| registration.rust_type_id)
    }

    pub fn component_display_name(&self, component_type: ComponentTypeId) -> Option<&str> {
        self.component_types
            .get(&component_type)
            .map(|registration| registration.display_name.as_str())
    }

    pub fn add_default_component(
        &self,
        world: &mut runen_ecs::World,
        entity: runen_ecs::Entity,
        component_type: ComponentTypeId,
    ) -> Result<(), EditorMutationError> {
        let registration = self.component_types.get(&component_type).ok_or(
            EditorMutationError::runtime_rejected(
                "component type is not registered in editor runtime",
            ),
        )?;

        (registration.insert_default)(world, entity)
    }

    pub fn remove_component_and_capture(
        &mut self,
        world: &mut runen_ecs::World,
        editor_entity: EntityId,
        ecs_entity: runen_ecs::Entity,
        component_type: ComponentTypeId,
    ) -> Result<(), EditorMutationError> {
        let registration = self.component_types.get(&component_type).ok_or(
            EditorMutationError::runtime_rejected(
                "component type is not registered in editor runtime",
            ),
        )?;

        (registration.remove_and_capture)(
            world,
            ecs_entity,
            &mut self.removed_components,
            (editor_entity, component_type),
        )
    }

    pub fn restore_removed_component(
        &mut self,
        world: &mut runen_ecs::World,
        editor_entity: EntityId,
        ecs_entity: runen_ecs::Entity,
        component_type: ComponentTypeId,
    ) -> Result<(), EditorMutationError> {
        let key = (editor_entity, component_type);

        if let Some(removed) = self.removed_components.remove(&key) {
            return removed.restore(world, ecs_entity);
        }

        self.add_default_component(world, ecs_entity, component_type)
    }

    pub fn register_resource_type<T: 'static>(&mut self, editor_id: ResourceTypeId) {
        self.resource_type_ids.insert(editor_id, TypeId::of::<T>());
    }

    pub fn resource_type_ids(&self) -> impl Iterator<Item = ResourceTypeId> + '_ {
        self.resource_type_ids.keys().copied()
    }

    pub fn resolve_resource_rust_type_id(&self, resource_type: ResourceTypeId) -> Option<TypeId> {
        self.resource_type_ids.get(&resource_type).copied()
    }
}

fn short_type_name<T>() -> String {
    let full = std::any::type_name::<T>();
    full.rsplit("::").next().unwrap_or(full).to_string()
}

fn insert_default_component<T>(
    world: &mut runen_ecs::World,
    entity: runen_ecs::Entity,
) -> Result<(), EditorMutationError>
where
    T: runen_ecs::Component + Default + 'static,
{
    world
        .insert(entity, T::default())
        .map_err(|_| EditorMutationError::runtime_rejected("failed to insert default component"))
}

fn remove_and_capture_component<T>(
    world: &mut runen_ecs::World,
    entity: runen_ecs::Entity,
    store: &mut HashMap<RemovedComponentKey, Box<dyn RemovedComponentValue>>,
    key: RemovedComponentKey,
) -> Result<(), EditorMutationError>
where
    T: runen_ecs::Component + 'static,
{
    let value = world
        .remove::<T>(entity)
        .map_err(|_| EditorMutationError::runtime_rejected("failed to remove component"))?;

    store.insert(key, Box::new(RemovedComponent::<T> { value }));
    Ok(())
}
