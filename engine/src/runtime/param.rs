use ecs::{QueryAccess, SystemParam, SystemParamContext, SystemParamError, World};
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};
use std::ptr::NonNull;

pub use ecs::{Commands, Query, Res, ResMut};

/// The sole whole-world system parameter. Registration rejects every sibling
/// immediate borrow before this capability is extracted.
pub struct WorldMut<'world> {
    world: NonNull<World>,
    _marker: PhantomData<&'world mut World>,
}

impl<'world> Deref for WorldMut<'world> {
    type Target = World;
    fn deref(&self) -> &World {
        unsafe { self.world.as_ref() }
    }
}

impl<'world> DerefMut for WorldMut<'world> {
    fn deref_mut(&mut self) -> &mut World {
        unsafe { self.world.as_mut() }
    }
}

unsafe impl<'param> SystemParam for WorldMut<'param> {
    type State = ();
    type Item<'world, 'state> = WorldMut<'world>;
    fn init_state(_: &mut World) -> Result<Self::State, SystemParamError> {
        Ok(())
    }
    fn access(_: &Self::State) -> QueryAccess {
        QueryAccess::exclusive_world()
    }
    fn slot_descriptor() -> scheduler::system::ParamSlotDescriptor {
        scheduler::system::ParamSlotDescriptor::leaf(
            "world_mut",
            "WorldMut",
            std::any::type_name::<Self>(),
        )
    }
    unsafe fn extract<'world, 'state>(
        _: &'state mut Self::State,
        context: SystemParamContext<'world>,
    ) -> Result<Self::Item<'world, 'state>, SystemParamError> {
        let world = unsafe { context.world_mut() };
        Ok(WorldMut {
            world: NonNull::from(world),
            _marker: PhantomData,
        })
    }
}
