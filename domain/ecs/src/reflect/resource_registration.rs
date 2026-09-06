//! File: domain/ecs/src/reflect/resource_registration.rs
//! Purpose: ECS-owned reflected resource registration metadata and erased value accessors.

use crate::component::Resource;
use crate::reflect::{Reflect, ReflectValueMut, ReflectValueRef, TypeInfo};
use crate::world::World;

pub type ResourceValueRefAccessor = for<'a> fn(&'a World) -> Option<ReflectValueRef<'a>>;
pub type ResourceValueMutAccessor = for<'a> fn(&'a mut World) -> Option<ReflectValueMut<'a>>;

#[derive(Debug, Clone, Copy)]
pub struct ReflectedResourceRegistration {
    pub type_info: TypeInfo,
    pub value_ref: ResourceValueRefAccessor,
    pub value_mut: ResourceValueMutAccessor,
}

impl ReflectedResourceRegistration {
    pub const fn new(
        type_info: TypeInfo,
        value_ref: ResourceValueRefAccessor,
        value_mut: ResourceValueMutAccessor,
    ) -> Self {
        Self {
            type_info,
            value_ref,
            value_mut,
        }
    }
}

pub(crate) fn reflected_resource_registration<T>() -> ReflectedResourceRegistration
where
    T: Reflect + Resource,
{
    fn value_ref_impl<T>(world: &World) -> Option<ReflectValueRef<'_>>
    where
        T: Reflect + Resource,
    {
        world.resource::<T>().ok().map(Reflect::reflect_ref)
    }

    fn value_mut_impl<T>(world: &mut World) -> Option<ReflectValueMut<'_>>
    where
        T: Reflect + Resource,
    {
        world
            .resource_mut::<T>()
            .ok()
            .map(|value| Reflect::reflect_mut(value))
    }

    ReflectedResourceRegistration::new(T::type_info(), value_ref_impl::<T>, value_mut_impl::<T>)
}
