use crate::plugins::render::RenderResourceDeclaration;
use runen_gpu::GpuWorkResourceId;
use std::any::{TypeId, type_name};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateResourceDeclaration {
    pub type_id: TypeId,
    pub type_name: &'static str,
}

impl StateResourceDeclaration {
    pub fn of<T: ecs::Resource + 'static>() -> Self {
        Self {
            type_id: TypeId::of::<T>(),
            type_name: type_name::<T>(),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct ResourceGraph {
    pub state_resources: Vec<StateResourceDeclaration>,
    pub resources: Vec<RenderResourceDeclaration>,
}

impl ResourceGraph {
    pub fn add_state_resource<T>(&mut self)
    where
        T: ecs::Resource + 'static,
    {
        self.state_resources
            .push(StateResourceDeclaration::of::<T>());
    }

    pub fn add_resource(&mut self, descriptor: RenderResourceDeclaration) {
        self.resources.push(descriptor);
    }

    pub fn uniform_buffer_ids_by_params_type(&self, type_id: TypeId) -> Vec<GpuWorkResourceId> {
        self.resources
            .iter()
            .filter_map(|resource| match resource {
                RenderResourceDeclaration::Uniform(value) if value.params_type_id() == type_id => {
                    Some(*value.id())
                }
                _ => None,
            })
            .collect()
    }

    pub fn has_state_resource(&self, type_id: TypeId) -> bool {
        self.state_resources
            .iter()
            .any(|resource| resource.type_id == type_id)
    }

    pub fn has_uniform_buffer(&self, id: &GpuWorkResourceId) -> bool {
        self.uniform_buffer_params(id).is_some()
    }

    pub fn uniform_buffer_params(&self, id: &GpuWorkResourceId) -> Option<(TypeId, &'static str)> {
        self.resources.iter().find_map(|resource| match resource {
            RenderResourceDeclaration::Uniform(value) if value.id() == id => {
                Some((value.params_type_id(), value.params_type_name()))
            }
            _ => None,
        })
    }
}
