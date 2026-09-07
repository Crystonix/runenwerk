use super::descriptors::ProceduralPassDescriptor;
use super::lowering::{ProceduralPassLowering, ProceduralUniformBinding, lower_procedural_pass};
use super::validation::validate_procedural_pass;
use crate::plugins::render::api::{PassParamBinding, RenderFlow, RenderFlowAuthoringError};
use crate::plugins::render::{
    DrawIndexedIndirectArgs, DrawIndirectArgs, GpuParams, IndirectDrawArgsBuffer,
    RenderIndirectDrawArgsKind,
};
use runen_gpu::{GpuBindingKey, GpuBufferHandle, GpuWorkResourceId};

#[derive(Debug)]
pub struct ProceduralPassBuilder {
    flow: RenderFlow,
    descriptor: ProceduralPassDescriptor,
    uniform_bindings: Vec<ProceduralUniformBinding>,
    draw_source: ProceduralDrawSource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProceduralDrawSource {
    Direct,
    Indirect {
        args_buffer: GpuWorkResourceId,
        args_kind: RenderIndirectDrawArgsKind,
        args_element_count: u64,
        args_element_size: u64,
        byte_offset: u64,
    },
}

impl ProceduralPassBuilder {
    pub(crate) fn new(
        flow: RenderFlow,
        descriptor: ProceduralPassDescriptor,
    ) -> Result<Self, RenderFlowAuthoringError> {
        validate_procedural_pass(&descriptor)?;
        Ok(Self {
            flow,
            descriptor,
            uniform_bindings: Vec::new(),
            draw_source: ProceduralDrawSource::Direct,
        })
    }

    pub fn uniform_from_state<S, U, F>(
        mut self,
        binding: GpuBindingKey,
        projection: F,
    ) -> Result<Self, RenderFlowAuthoringError>
    where
        S: ecs::Resource + Send + Sync + 'static,
        U: GpuParams + Send + Sync + 'static,
        F: Fn(&S) -> U + Send + Sync + 'static,
    {
        let uniform = self
            .flow
            .allocate_uniform_resource::<U>(self.descriptor.label.as_str())?;
        self.uniform_bindings.push(ProceduralUniformBinding {
            key: binding,
            projection: PassParamBinding::uniform_state(uniform.diagnostic_identity(), projection),
        });
        Ok(self)
    }

    pub fn uniform_from_state_with_surface<S, U, F>(
        mut self,
        binding: GpuBindingKey,
        projection: F,
    ) -> Result<Self, RenderFlowAuthoringError>
    where
        S: ecs::Resource + Send + Sync + 'static,
        U: GpuParams + Send + Sync + 'static,
        F: Fn(&S, (u32, u32)) -> U + Send + Sync + 'static,
    {
        let uniform = self
            .flow
            .allocate_uniform_resource::<U>(self.descriptor.label.as_str())?;
        self.uniform_bindings.push(ProceduralUniformBinding {
            key: binding,
            projection: PassParamBinding::uniform_state_with_surface(
                uniform.diagnostic_identity(),
                projection,
            ),
        });
        Ok(self)
    }

    pub fn uniform_from_state_to<S, U, F>(
        mut self,
        binding: GpuBindingKey,
        handle: GpuBufferHandle,
        projection: F,
    ) -> Self
    where
        S: ecs::Resource + Send + Sync + 'static,
        U: GpuParams + Send + Sync + 'static,
        F: Fn(&S) -> U + Send + Sync + 'static,
    {
        self.uniform_bindings.push(ProceduralUniformBinding {
            key: binding,
            projection: PassParamBinding::uniform_state(handle.diagnostic_identity(), projection),
        });
        self
    }

    pub fn uniform_from_state_with_surface_to<S, U, F>(
        mut self,
        binding: GpuBindingKey,
        handle: GpuBufferHandle,
        projection: F,
    ) -> Self
    where
        S: ecs::Resource + Send + Sync + 'static,
        U: GpuParams + Send + Sync + 'static,
        F: Fn(&S, (u32, u32)) -> U + Send + Sync + 'static,
    {
        self.uniform_bindings.push(ProceduralUniformBinding {
            key: binding,
            projection: PassParamBinding::uniform_state_with_surface(
                handle.diagnostic_identity(),
                projection,
            ),
        });
        self
    }

    pub fn draw_indirect(
        self,
        args_buffer: GpuBufferHandle,
    ) -> Result<Self, RenderFlowAuthoringError> {
        self.draw_indirect_with_offset_typed::<DrawIndirectArgs>(args_buffer, 0)
    }

    pub fn draw_indexed_indirect(
        self,
        args_buffer: GpuBufferHandle,
    ) -> Result<Self, RenderFlowAuthoringError> {
        self.draw_indirect_with_offset_typed::<DrawIndexedIndirectArgs>(args_buffer, 0)
    }

    pub fn draw_indirect_with_offset(
        self,
        args_buffer: GpuBufferHandle,
        byte_offset: u64,
    ) -> Result<Self, RenderFlowAuthoringError> {
        self.draw_indirect_with_offset_typed::<DrawIndirectArgs>(args_buffer, byte_offset)
    }

    fn draw_indirect_with_offset_typed<T: IndirectDrawArgsBuffer + 'static>(
        mut self,
        args_buffer: GpuBufferHandle,
        byte_offset: u64,
    ) -> Result<Self, RenderFlowAuthoringError> {
        let args_element_count = self.flow.indirect_buffer_element_count::<T>(&args_buffer)?;
        self.draw_source = ProceduralDrawSource::Indirect {
            args_buffer: args_buffer.diagnostic_identity(),
            args_kind: T::ARGS_KIND,
            args_element_count,
            args_element_size: T::BYTE_SIZE,
            byte_offset,
        };
        Ok(self)
    }

    pub fn finish(self) -> Result<RenderFlow, RenderFlowAuthoringError> {
        lower_procedural_pass(
            self.flow,
            self.descriptor,
            ProceduralPassLowering {
                uniform_bindings: self.uniform_bindings,
                draw_source: self.draw_source,
            },
        )
    }
}
