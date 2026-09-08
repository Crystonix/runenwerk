// Owner: ecs World Component - Access, Mutation, Change Tracking, and Matching APIs
use crate::component::{Component, ComponentState, StatefulComponent};
use crate::entity::Entity;
use crate::errors::EntityError;
use crate::telemetry;
use crate::world::World;
use crate::world::change_tracking::ComponentTypeKey;
use crate::world::entity_handles::Mut;
use std::any::{TypeId, type_name};
use std::time::Instant;

impl World {
    pub fn get<T: Component>(&self, entity: Entity) -> Option<&T> {
        if !self.contains(entity) {
            return None;
        }
        self.archetype_component::<T>(entity)
    }

    pub fn get_mut<T: Component>(&mut self, entity: Entity) -> Option<Mut<'_, T>> {
        if !self.contains(entity) {
            return None;
        }
        self.mark_component_modified_by_id(entity, TypeId::of::<T>(), T::component_name());
        let value = self.archetype_component_mut_untracked::<T>(entity)?;
        Some(Mut { value })
    }

    pub fn component_state<T: StatefulComponent>(&self, entity: Entity) -> Option<ComponentState> {
        if !self.contains(entity) {
            return None;
        }
        let (generation, version) = self.archetype_registry.component_state_by_id(
            entity,
            TypeId::of::<T>(),
            &self.entity_locations,
        )?;
        Some(ComponentState {
            generation,
            version,
        })
    }

    pub fn mark_stateful_changed<T: StatefulComponent>(&mut self, entity: Entity) -> bool {
        if !self.has_component_by_type_id(entity, TypeId::of::<T>()) {
            return false;
        }
        self.mark_component_modified_by_id(entity, TypeId::of::<T>(), T::component_name());
        self.archetype_registry
            .mark_component_stateful_changed_by_id(
                entity,
                TypeId::of::<T>(),
                self.change_tick,
                &self.entity_locations,
            )
    }

    pub fn require<T: Component>(&self, entity: Entity) -> Result<&T, EntityError> {
        self.ensure_entity_exists(entity)?;
        self.archetype_component::<T>(entity)
            .ok_or(EntityError::MissingComponent {
                entity,
                component: type_name::<T>(),
            })
    }

    pub fn require_mut<T: Component>(&mut self, entity: Entity) -> Result<Mut<'_, T>, EntityError> {
        self.ensure_entity_exists(entity)?;
        self.mark_component_modified_by_id(entity, TypeId::of::<T>(), T::component_name());
        let value = self.archetype_component_mut_untracked::<T>(entity).ok_or(
            EntityError::MissingComponent {
                entity,
                component: type_name::<T>(),
            },
        )?;
        Ok(Mut { value })
    }

    pub(crate) fn __commit_insert_component<T: Component>(&mut self, entity: Entity, component: T) {
        debug_assert!(self.contains(entity));
        let kind = if self.contains_component::<T>(entity) {
            crate::world::change_tracking::ComponentChangeKind::Modified
        } else {
            crate::world::change_tracking::ComponentChangeKind::Added
        };
        let component_type = TypeId::of::<T>();
        let commit_tick = self.change_tick.saturating_add(1);

        let inserted = if matches!(
            kind,
            crate::world::change_tracking::ComponentChangeKind::Added
        ) {
            self.archetype_registry.add_component::<T>(
                entity,
                component,
                commit_tick,
                &mut self.entity_locations,
            )
        } else {
            self.archetype_registry.update_component::<T>(
                entity,
                component,
                commit_tick,
                &self.entity_locations,
            )
        };

        assert!(
            inserted,
            "preflighted archetype component insert/update must succeed"
        );
        self.record_component_change(entity, component_type, T::component_name(), kind);
    }

    pub(crate) fn __commit_remove_component<T: Component>(&mut self, entity: Entity) -> T {
        debug_assert!(self.contains(entity));
        let value = self
            .archetype_registry
            .remove_component::<T>(entity, &mut self.entity_locations)
            .expect("preflighted archetype component removal must succeed");
        self.record_component_change(
            entity,
            TypeId::of::<T>(),
            T::component_name(),
            crate::world::change_tracking::ComponentChangeKind::Removed,
        );
        value
    }

    pub fn component_changed_since<T: Component>(&self, tick: u64) -> bool {
        self.component_change_ticks
            .get(&TypeId::of::<T>())
            .is_some_and(|changed| *changed > tick)
    }

    pub fn component_changes_since(
        &self,
        tick: u64,
    ) -> Vec<crate::world::change_tracking::ComponentChangeRecord> {
        self.component_change_log
            .iter()
            .filter(|change| change.tick > tick)
            .cloned()
            .collect()
    }

    pub fn component_type_key<T: Component>(&self) -> Option<ComponentTypeKey> {
        self.component_type_registry
            .get(&TypeId::of::<T>())
            .map(|meta| meta.id)
    }

    pub fn component_type_key_by_id(&self, type_id: TypeId) -> Option<ComponentTypeKey> {
        self.component_type_registry
            .get(&type_id)
            .map(|meta| meta.id)
    }

    pub fn component_type_name_by_key(&self, key: ComponentTypeKey) -> Option<&'static str> {
        self.component_type_registry
            .values()
            .find(|meta| meta.id == key)
            .map(|meta| meta.name)
    }

    pub(crate) fn has_component_by_type_id(&self, entity: Entity, type_id: TypeId) -> bool {
        let Some(location) = self.entity_locations.get(entity) else {
            return false;
        };

        self.archetype_registry
            .component_types(location.archetype_id)
            .is_some_and(|component_types| component_types.binary_search(&type_id).is_ok())
    }

    pub(crate) fn contains_component<T: Component>(&self, entity: Entity) -> bool {
        self.has_component_by_type_id(entity, TypeId::of::<T>())
    }

    pub(crate) fn mark_component_modified_by_id(
        &mut self,
        entity: Entity,
        component_type: TypeId,
        component_name: &'static str,
    ) {
        self.record_component_change(
            entity,
            component_type,
            component_name,
            crate::world::change_tracking::ComponentChangeKind::Modified,
        );

        let _ = self.archetype_registry.mark_component_changed_by_id(
            entity,
            component_type,
            self.change_tick,
            &self.entity_locations,
        );
    }

    fn mark_component_type_changed_by_id(&mut self, type_id: TypeId) {
        self.change_tick = self.change_tick.saturating_add(1);
        self.component_change_ticks
            .insert(type_id, self.change_tick);
        self.mark_component_indexes_dirty(type_id);
    }

    pub(crate) fn record_component_change(
        &mut self,
        entity: Entity,
        component_type: TypeId,
        component_name: &'static str,
        kind: crate::world::change_tracking::ComponentChangeKind,
    ) {
        self.mark_component_type_changed_by_id(component_type);
        let component_key = self
            .component_type_registry
            .get(&component_type)
            .map(|meta| meta.id)
            .unwrap_or_default();

        self.component_change_log
            .push(crate::world::change_tracking::ComponentChangeRecord {
                tick: self.change_tick,
                entity,
                component_type,
                component_key,
                component_name,
                kind,
            });

        if matches!(
            kind,
            crate::world::change_tracking::ComponentChangeKind::Removed
        ) {
            self.removed_component_records
                .entry(component_type)
                .or_default()
                .push(crate::world::change_tracking::RemovedComponentRecord {
                    tick: self.change_tick,
                    entity,
                });
        }
    }

    pub(crate) fn begin_stage_command_flush(&mut self) {
        self.removed_component_records.clear();
    }

    pub(crate) fn archetype_component<T: Component>(&self, entity: Entity) -> Option<&T> {
        let ptr = self
            .archetype_registry
            .component_ptr::<T>(entity, &self.entity_locations)?;
        Some(unsafe { &*ptr })
    }

    pub(crate) fn archetype_component_mut_untracked<T: Component>(
        &mut self,
        entity: Entity,
    ) -> Option<&mut T> {
        let ptr = self
            .archetype_registry
            .component_mut_ptr::<T>(entity, &self.entity_locations)?;
        Some(unsafe { &mut *ptr })
    }

    pub(crate) fn archetype_component_metadata<T: Component>(
        &self,
        entity: Entity,
    ) -> Option<(u64, u64)> {
        let metadata = self
            .archetype_registry
            .component_metadata::<T>(entity, &self.entity_locations)?;
        Some((metadata.added_tick, metadata.changed_tick))
    }

    pub(crate) fn mark_component_indexes_dirty(&mut self, component_type: TypeId) {
        let mut indexes = self.component_indexes.borrow_mut();
        for (index_key, index) in indexes.iter_mut() {
            if index_key.component_type == component_type {
                index.mark_dirty();
            }
        }
    }

    pub(crate) fn matching_entities_into(
        &self,
        required_present: &[TypeId],
        excluded: &[TypeId],
        out: &mut Vec<Entity>,
    ) {
        let start = Instant::now();
        let _ = self
            .archetype_registry
            .collect_matching_entities(required_present, excluded, out);
        let count = out.len() as u64;
        telemetry::record_query_matching(start.elapsed().as_nanos() as u64, count, count);
    }
}
