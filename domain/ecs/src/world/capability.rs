//! Invocation-scoped projections owned by the `World` implementation.
//!
//! These capabilities are deliberately not general world handles. The only
//! raw `World` address is held by the invocation authority in `system::extract`;
//! this module immediately projects that authority to the concrete storage and
//! bookkeeping domains used by a parameter. The projections are valid only
//! while the invocation's structural freeze is active.

use super::World;
use super::change_tracking::{
    ComponentChangeKind, ComponentChangeRecord, ComponentMeta, RemovedComponentRecord,
    ResourceChangeKind, ResourceChangeRecord, ResourceMeta,
};
use super::component_indexes::{ComponentIndexKey, ComponentIndexStorage};
use crate::component::Component;
use crate::entity::{Entity, WorldScopeId};
use crate::errors::ResourceError;
use crate::storage::{ArchetypeExecutionBinding, ArchetypeRegistry, EntityLocationMap};
use std::any::{TypeId, type_name};
use std::cell::RefCell;
use std::collections::{BTreeSet, HashMap};
use std::marker::PhantomData;
use std::ptr::NonNull;
use std::time::Instant;

/// The sole invocation-scoped authority from which narrow capabilities are
/// projected. It is never stored in a user-facing parameter value.
#[derive(Copy, Clone)]
pub(crate) struct WorldAuthority<'world> {
    world: NonNull<World>,
    _marker: PhantomData<&'world mut World>,
}

impl<'world> WorldAuthority<'world> {
    pub(crate) fn new(world: &'world mut World) -> Self {
        Self {
            world: NonNull::from(world),
            _marker: PhantomData,
        }
    }

    pub(crate) fn query(self) -> QueryCapability<'world> {
        // Safety: the authority was constructed from the live invocation World;
        // the bridge immediately projects only owned query fields.
        unsafe { QueryCapability::from_world_ptr(self.world) }
    }

    pub(crate) unsafe fn world_mut(mut self) -> &'world mut World {
        unsafe { self.world.as_mut() }
    }

    pub(crate) fn resource<T: crate::component::Resource>(
        self,
    ) -> Result<ResourceCapability<'world, T>, ResourceError> {
        // Safety: the authority lifetime is the invocation lifetime and the
        // resource bridge retains only the stable boxed payload address.
        unsafe { World::resource_capability_from_ptr(self.world, false) }
    }

    pub(crate) fn resource_mut<T: crate::component::Resource>(
        self,
    ) -> Result<ResourceCapability<'world, T>, ResourceError> {
        // Safety: access validation rejects overlapping resource borrows before
        // this projection is manufactured.
        unsafe { World::resource_capability_from_ptr(self.world, true) }
    }
}

/// Narrow query-domain authority. It contains pointers only to the storage,
/// location, and change-bookkeeping fields needed by supported query forms.
#[doc(hidden)]
pub struct QueryCapability<'world> {
    world_scope: WorldScopeId,
    alive_entities: NonNull<BTreeSet<Entity>>,
    archetype_registry: NonNull<ArchetypeRegistry>,
    entity_locations: NonNull<EntityLocationMap>,
    component_type_registry: NonNull<HashMap<TypeId, ComponentMeta>>,
    component_indexes: NonNull<RefCell<HashMap<ComponentIndexKey, Box<dyn ComponentIndexStorage>>>>,
    change_tick: NonNull<u64>,
    component_change_ticks: NonNull<HashMap<TypeId, u64>>,
    component_change_log: NonNull<Vec<ComponentChangeRecord>>,
    removed_component_records: NonNull<HashMap<TypeId, Vec<RemovedComponentRecord>>>,
    _marker: PhantomData<&'world World>,
}

impl<'world> Copy for QueryCapability<'world> {}
impl<'world> Clone for QueryCapability<'world> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<'world> QueryCapability<'world> {
    pub(super) fn from_world(world: &'world World) -> Self {
        // Safety: this is the one World-owned projection point. All fields are
        // part of `world` and remain at stable addresses while the invocation's
        // structural freeze is active; no capability stores an archetype row or
        // movable map entry address.
        Self {
            world_scope: world.scope_id(),
            alive_entities: NonNull::from(&world.alive_entities),
            archetype_registry: NonNull::from(&world.archetype_registry),
            entity_locations: NonNull::from(&world.entity_locations),
            component_type_registry: NonNull::from(&world.component_type_registry),
            component_indexes: NonNull::from(&world.component_indexes),
            change_tick: NonNull::from(&world.change_tick),
            component_change_ticks: NonNull::from(&world.component_change_ticks),
            component_change_log: NonNull::from(&world.component_change_log),
            removed_component_records: NonNull::from(&world.removed_component_records),
            _marker: PhantomData,
        }
    }

    pub(super) fn from_world_mut(world: &'world mut World) -> Self {
        Self {
            world_scope: world.scope_id(),
            alive_entities: NonNull::from(&mut world.alive_entities),
            archetype_registry: NonNull::from(&mut world.archetype_registry),
            entity_locations: NonNull::from(&mut world.entity_locations),
            component_type_registry: NonNull::from(&mut world.component_type_registry),
            component_indexes: NonNull::from(&mut world.component_indexes),
            change_tick: NonNull::from(&mut world.change_tick),
            component_change_ticks: NonNull::from(&mut world.component_change_ticks),
            component_change_log: NonNull::from(&mut world.component_change_log),
            removed_component_records: NonNull::from(&mut world.removed_component_records),
            _marker: PhantomData,
        }
    }

    pub(super) unsafe fn from_world_ptr(world: NonNull<World>) -> Self {
        let world_ptr = world.as_ptr();
        Self {
            world_scope: unsafe { (*world_ptr).scope_id() },
            alive_entities: unsafe {
                NonNull::new_unchecked(std::ptr::addr_of_mut!((*world_ptr).alive_entities))
            },
            archetype_registry: unsafe {
                NonNull::new_unchecked(std::ptr::addr_of_mut!((*world_ptr).archetype_registry))
            },
            entity_locations: unsafe {
                NonNull::new_unchecked(std::ptr::addr_of_mut!((*world_ptr).entity_locations))
            },
            component_type_registry: unsafe {
                NonNull::new_unchecked(std::ptr::addr_of_mut!((*world_ptr).component_type_registry))
            },
            component_indexes: unsafe {
                NonNull::new_unchecked(std::ptr::addr_of_mut!((*world_ptr).component_indexes))
            },
            change_tick: unsafe {
                NonNull::new_unchecked(std::ptr::addr_of_mut!((*world_ptr).change_tick))
            },
            component_change_ticks: unsafe {
                NonNull::new_unchecked(std::ptr::addr_of_mut!((*world_ptr).component_change_ticks))
            },
            component_change_log: unsafe {
                NonNull::new_unchecked(std::ptr::addr_of_mut!((*world_ptr).component_change_log))
            },
            removed_component_records: unsafe {
                NonNull::new_unchecked(std::ptr::addr_of_mut!(
                    (*world_ptr).removed_component_records
                ))
            },
            _marker: PhantomData,
        }
    }

    pub(crate) fn current_change_tick(self) -> u64 {
        unsafe { *self.change_tick.as_ptr() }
    }

    pub(crate) fn world_scope(self) -> WorldScopeId {
        self.world_scope
    }

    pub(crate) fn matching_entities_into(
        self,
        required_present: &[TypeId],
        excluded: &[TypeId],
        out: &mut Vec<Entity>,
    ) {
        let start = Instant::now();
        unsafe {
            self.archetype_registry.as_ref().collect_matching_entities(
                required_present,
                excluded,
                out,
            )
        };
        let count = out.len() as u64;
        crate::telemetry::record_query_matching(start.elapsed().as_nanos() as u64, count, count);
    }

    pub(crate) fn matching_archetype_bindings_into(
        self,
        required_present: &[TypeId],
        excluded: &[TypeId],
        out: &mut Vec<ArchetypeExecutionBinding>,
    ) -> bool {
        unsafe {
            self.archetype_registry.as_ref().collect_matching_bindings(
                required_present,
                excluded,
                out,
            )
        }
    }

    pub(crate) fn archetype_entity_at(self, archetype_index: usize, row: usize) -> Option<Entity> {
        unsafe {
            self.archetype_registry
                .as_ref()
                .entity_at(archetype_index, row)
        }
    }

    pub(crate) fn entity_matches_component_constraints(
        self,
        entity: Entity,
        required_present: &[TypeId],
        excluded: &[TypeId],
    ) -> bool {
        self.contains(entity)
            && required_present
                .iter()
                .all(|type_id| self.has_component_by_type_id(entity, *type_id))
            && excluded
                .iter()
                .all(|type_id| !self.has_component_by_type_id(entity, *type_id))
    }

    pub(crate) fn contains(self, entity: Entity) -> bool {
        unsafe { self.alive_entities.as_ref().contains(&entity) }
    }

    pub(crate) fn has_component_by_type_id(self, entity: Entity, type_id: TypeId) -> bool {
        let locations = unsafe { self.entity_locations.as_ref() };
        let Some(location) = locations.get(entity) else {
            return false;
        };
        unsafe {
            self.archetype_registry
                .as_ref()
                .component_types(location.archetype_id)
        }
        .is_some_and(|types| types.binary_search(&type_id).is_ok())
    }

    pub(crate) fn component<T: Component>(self, entity: Entity) -> Option<&'world T> {
        let locations = unsafe { self.entity_locations.as_ref() };
        let ptr = unsafe {
            self.archetype_registry
                .as_ref()
                .component_ptr::<T>(entity, locations)
        }?;
        // Safety: the storage registry verified the typed column and row. The
        // boxed payload allocation is stable across registry/container moves.
        Some(unsafe { &*ptr })
    }

    /// # Safety
    /// The query access contract must contain the exclusive component borrow and
    /// structural mutation must remain frozen for `'world`.
    pub(crate) unsafe fn component_mut<T: Component>(
        mut self,
        entity: Entity,
    ) -> Option<&'world mut T> {
        let locations = unsafe { self.entity_locations.as_ref() };
        let registry = unsafe { self.archetype_registry.as_mut() };
        let ptr = registry.component_mut_ptr::<T>(entity, locations)?;
        Some(unsafe { &mut *ptr })
    }

    pub(crate) fn component_metadata<T: Component>(self, entity: Entity) -> Option<(u64, u64)> {
        let locations = unsafe { self.entity_locations.as_ref() };
        let metadata = unsafe {
            self.archetype_registry
                .as_ref()
                .component_metadata::<T>(entity, locations)
        }?;
        Some((metadata.added_tick, metadata.changed_tick))
    }

    pub(crate) fn mark_component_modified_by_id(
        mut self,
        entity: Entity,
        component_type: TypeId,
        component_name: &'static str,
    ) {
        let tick = self.record_component_change(
            entity,
            component_type,
            component_name,
            ComponentChangeKind::Modified,
        );
        let locations = unsafe { self.entity_locations.as_ref() };
        let _ = unsafe {
            self.archetype_registry
                .as_mut()
                .mark_component_changed_by_id(entity, component_type, tick, locations)
        };
    }

    pub(crate) fn mark_component_modified<T: Component>(self, entity: Entity) {
        if self.component::<T>(entity).is_some() {
            self.mark_component_modified_by_id(entity, TypeId::of::<T>(), T::component_name());
        }
    }

    fn record_component_change(
        mut self,
        entity: Entity,
        component_type: TypeId,
        component_name: &'static str,
        kind: ComponentChangeKind,
    ) -> u64 {
        let tick = unsafe {
            let tick = self.change_tick.as_mut();
            *tick = tick.saturating_add(1);
            *tick
        };
        unsafe {
            self.component_change_ticks
                .as_mut()
                .insert(component_type, tick)
        };
        let component_key = unsafe { self.component_type_registry.as_ref().get(&component_type) }
            .map(|meta| meta.id)
            .unwrap_or_default();
        unsafe {
            self.component_change_log
                .as_mut()
                .push(ComponentChangeRecord {
                    tick,
                    entity,
                    component_type,
                    component_key,
                    component_name,
                    kind,
                })
        };
        if matches!(kind, ComponentChangeKind::Removed) {
            unsafe {
                self.removed_component_records
                    .as_mut()
                    .entry(component_type)
                    .or_default()
                    .push(RemovedComponentRecord { tick, entity })
            };
        }
        self.mark_component_indexes_dirty(component_type);
        tick
    }

    fn mark_component_indexes_dirty(self, component_type: TypeId) {
        let mut indexes = unsafe { self.component_indexes.as_ref().borrow_mut() };
        for (index_key, index) in indexes.iter_mut() {
            if index_key.component_type == component_type {
                index.mark_dirty();
            }
        }
    }

    pub(crate) fn component_changed_for_entity_since<T: Component>(
        self,
        entity: Entity,
        tick: u64,
    ) -> bool {
        let start = Instant::now();
        let changed = self
            .component_metadata::<T>(entity)
            .is_some_and(|(_, changed_tick)| changed_tick > tick);
        crate::telemetry::record_changed_check(start.elapsed().as_nanos() as u64);
        changed
    }

    pub(crate) fn component_added_for_entity_since<T: Component>(
        self,
        entity: Entity,
        tick: u64,
    ) -> bool {
        let start = Instant::now();
        let added = self
            .component_metadata::<T>(entity)
            .is_some_and(|(added_tick, _)| added_tick > tick);
        crate::telemetry::record_added_check(start.elapsed().as_nanos() as u64);
        added
    }

    pub(crate) fn removed_component_records_current_window(
        self,
        component_type: TypeId,
        out: &mut Vec<(Entity, u64)>,
    ) {
        out.clear();
        let records = unsafe { self.removed_component_records.as_ref() };
        if let Some(records) = records.get(&component_type) {
            out.extend(records.iter().map(|record| (record.entity, record.tick)));
        }
    }
}

/// Stable typed resource payload plus the separate narrow change recorder used
/// by `ResMut`. No resource parameter retains a world or registry-entry pointer.
#[doc(hidden)]
pub struct ResourceCapability<'world, T> {
    value: NonNull<T>,
    mutation: Option<ResourceMutationCapability<'world>>,
    _marker: PhantomData<&'world T>,
}

impl<'world, T> Copy for ResourceCapability<'world, T> {}
impl<'world, T> Clone for ResourceCapability<'world, T> {
    fn clone(&self) -> Self {
        *self
    }
}

#[derive(Copy, Clone)]
pub(crate) struct ResourceMutationCapability<'world> {
    next_resource_id: NonNull<u32>,
    resource_type_registry: NonNull<HashMap<TypeId, ResourceMeta>>,
    change_tick: NonNull<u64>,
    resource_change_ticks: NonNull<HashMap<TypeId, u64>>,
    resource_change_log: NonNull<Vec<ResourceChangeRecord>>,
    _marker: PhantomData<&'world mut World>,
}

impl<'world> ResourceMutationCapability<'world> {
    pub(crate) fn mark_modified<T: 'static>(mut self) {
        let type_id = TypeId::of::<T>();
        let name = type_name::<T>();
        let registry = unsafe { self.resource_type_registry.as_mut() };
        let key = registry
            .entry(type_id)
            .or_insert_with(|| {
                let id = unsafe { *self.next_resource_id.as_ptr() };
                unsafe { *self.next_resource_id.as_mut() = id.saturating_add(1) };
                ResourceMeta {
                    id: super::change_tracking::ResourceTypeKey(id),
                    name,
                }
            })
            .id;
        let tick = unsafe {
            let tick = self.change_tick.as_mut();
            *tick = tick.saturating_add(1);
            *tick
        };
        unsafe { self.resource_change_ticks.as_mut().insert(type_id, tick) };
        unsafe {
            self.resource_change_log
                .as_mut()
                .push(ResourceChangeRecord {
                    tick,
                    resource_type: type_id,
                    resource_key: key,
                    resource_name: name,
                    kind: ResourceChangeKind::Modified,
                })
        };
    }
}

impl World {
    /// Central World-owned bridge for ordinary query extraction.
    pub(crate) fn query_capability(&self) -> QueryCapability<'_> {
        QueryCapability::from_world(self)
    }

    pub(crate) fn query_capability_mut(&mut self) -> QueryCapability<'_> {
        QueryCapability::from_world_mut(self)
    }

    pub(crate) unsafe fn resource_capability_from_ptr<'world, T: crate::component::Resource>(
        world: NonNull<World>,
        mutable: bool,
    ) -> Result<ResourceCapability<'world, T>, ResourceError> {
        let world_ptr = world.as_ptr();
        let type_id = TypeId::of::<T>();
        if mutable {
            let value = unsafe {
                (*world_ptr)
                    .resources
                    .get_mut(&type_id)
                    .and_then(|resource| resource.downcast_mut::<T>())
            }
            .ok_or(ResourceError::Missing {
                resource: type_name::<T>(),
            })?;
            let mutation = ResourceMutationCapability {
                next_resource_id: unsafe {
                    NonNull::new_unchecked(std::ptr::addr_of_mut!((*world_ptr).next_resource_id))
                },
                resource_type_registry: unsafe {
                    NonNull::new_unchecked(std::ptr::addr_of_mut!(
                        (*world_ptr).resource_type_registry
                    ))
                },
                change_tick: unsafe {
                    NonNull::new_unchecked(std::ptr::addr_of_mut!((*world_ptr).change_tick))
                },
                resource_change_ticks: unsafe {
                    NonNull::new_unchecked(std::ptr::addr_of_mut!(
                        (*world_ptr).resource_change_ticks
                    ))
                },
                resource_change_log: unsafe {
                    NonNull::new_unchecked(std::ptr::addr_of_mut!((*world_ptr).resource_change_log))
                },
                _marker: PhantomData,
            };
            Ok(ResourceCapability {
                value: NonNull::from(&mut *value),
                mutation: Some(mutation),
                _marker: PhantomData,
            })
        } else {
            let value = unsafe {
                (*world_ptr)
                    .resources
                    .get(&type_id)
                    .and_then(|resource| resource.downcast_ref::<T>())
            }
            .ok_or(ResourceError::Missing {
                resource: type_name::<T>(),
            })?;
            Ok(ResourceCapability {
                value: NonNull::from(value),
                mutation: None,
                _marker: PhantomData,
            })
        }
    }
}

impl<'world, T> ResourceCapability<'world, T> {
    pub(crate) fn value(self) -> NonNull<T> {
        self.value
    }

    pub(crate) fn mutation(self) -> Option<ResourceMutationCapability<'world>> {
        self.mutation
    }
}
