use std::any::{Any, TypeId, type_name};
use std::sync::Arc;

pub trait ComputeDispatchProjection: Send + Sync {
    fn state_type_id(&self) -> TypeId;
    fn state_type_name(&self) -> &'static str;
    fn project_dispatch(&self, state: &dyn Any) -> Option<[u32; 3]>;
}

#[derive(Clone)]
pub struct ComputeDispatchBinding {
    projection: Arc<dyn ComputeDispatchProjection>,
}

impl std::fmt::Debug for ComputeDispatchBinding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ComputeDispatchBinding")
            .field("state_type_name", &self.state_type_name())
            .finish()
    }
}

impl ComputeDispatchBinding {
    pub fn state<S>(build: fn(&S) -> [u32; 3]) -> Self
    where
        S: runen_ecs::Resource + Send + Sync + 'static,
    {
        Self {
            projection: Arc::new(ComputeDispatchStateProjection { build }),
        }
    }

    pub fn state_type_id(&self) -> TypeId {
        self.projection.state_type_id()
    }

    pub fn state_type_name(&self) -> &'static str {
        self.projection.state_type_name()
    }

    pub fn project_dispatch(&self, state: &dyn Any) -> Option<[u32; 3]> {
        self.projection.project_dispatch(state)
    }
}

#[derive(Debug, Clone)]
pub enum ComputeDispatchDescriptor {
    Fixed([u32; 3]),
    State(ComputeDispatchBinding),
}

struct ComputeDispatchStateProjection<S>
where
    S: runen_ecs::Resource + 'static,
{
    build: fn(&S) -> [u32; 3],
}

impl<S> ComputeDispatchProjection for ComputeDispatchStateProjection<S>
where
    S: runen_ecs::Resource + Send + Sync + 'static,
{
    fn state_type_id(&self) -> TypeId {
        TypeId::of::<S>()
    }

    fn state_type_name(&self) -> &'static str {
        type_name::<S>()
    }

    fn project_dispatch(&self, state: &dyn Any) -> Option<[u32; 3]> {
        let state = state.downcast_ref::<S>()?;
        Some((self.build)(state))
    }
}
