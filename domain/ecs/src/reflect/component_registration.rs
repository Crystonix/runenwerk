//! File: domain/ecs/src/reflect/component_registration.rs
//! Purpose: ECS-owned reflected component registration metadata and erased value accessors.

use crate::component::Component;
use crate::entity::Entity;
use crate::reflect::{Reflect, ReflectValueMut, ReflectValueRef, TypeInfo};
use crate::world::World;

pub type ComponentValueRefAccessor = for<'a> fn(&'a World, Entity) -> Option<ReflectValueRef<'a>>;
pub type ComponentValueMutAccessor =
    for<'a> fn(&'a mut World, Entity) -> Option<ReflectValueMut<'a>>;

#[derive(Debug, Clone, Copy)]
pub struct ReflectedComponentRegistration {
    pub type_info: TypeInfo,
    pub value_ref: ComponentValueRefAccessor,
    pub value_mut: ComponentValueMutAccessor,
}

impl ReflectedComponentRegistration {
    pub const fn new(
        type_info: TypeInfo,
        value_ref: ComponentValueRefAccessor,
        value_mut: ComponentValueMutAccessor,
    ) -> Self {
        Self {
            type_info,
            value_ref,
            value_mut,
        }
    }
}

pub(crate) fn reflected_component_registration<T>() -> ReflectedComponentRegistration
where
    T: Reflect + Component,
{
    fn value_ref_impl<T>(world: &World, entity: Entity) -> Option<ReflectValueRef<'_>>
    where
        T: Reflect + Component,
    {
        world.get::<T>(entity).map(Reflect::reflect_ref)
    }

    fn value_mut_impl<T>(world: &mut World, entity: Entity) -> Option<ReflectValueMut<'_>>
    where
        T: Reflect + Component,
    {
        let value = world.get_mut::<T>(entity)?;
        Some(Reflect::reflect_mut(value.into_inner()))
    }

    ReflectedComponentRegistration::new(T::type_info(), value_ref_impl::<T>, value_mut_impl::<T>)
}
