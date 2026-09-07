use crate::plugins::render::api::ids::RenderFeatureId;
use crate::plugins::render::api::{
    ComputeDispatchBinding, ComputeDispatchDescriptor, PassParamBinding, RenderFlow,
    RenderFlowAuthoringError, RenderShaderBinding, RenderShaderBindingResource,
};
use crate::plugins::render::graph::RenderShaderReference;
use crate::plugins::render::{
    DrawIndexedIndirectArgs, DrawIndirectArgs, GpuParams, IndirectDrawArgsBuffer,
    RenderDrawDescriptor, RenderIndirectDrawResource, RenderPassId, RenderPassKind, RenderPassNode,
    RenderPassShapeIntent, RenderPassViewScope, RenderRasterState, RenderVertexBufferLayout,
    ShaderHandle,
};
use runen_gpu::{
    GpuBindingKey, GpuBufferHandle, GpuStorageBufferAccess, GpuStorageTextureAccess,
    GpuWorkResourceId,
};

#[derive(Debug)]
pub struct ComputePassBuilder {
    flow: RenderFlow,
    pass: RenderPassNode,
}

impl ComputePassBuilder {
    pub(crate) fn new(mut flow: RenderFlow, label: String) -> Self {
        let pass = new_pass(&mut flow, label, RenderPassKind::Compute);
        Self { flow, pass }
    }

    pub fn shader(mut self, shader: ShaderHandle) -> Self {
        self.pass.shader = Some(RenderShaderReference::RegistryHandle(shader));
        self
    }

    pub fn shader_asset(mut self, path: impl Into<String>) -> Self {
        self.pass.shader = Some(RenderShaderReference::AssetPath(path.into()));
        self
    }

    pub fn for_feature(mut self, feature_id: RenderFeatureId) -> Self {
        self.pass.feature_id = Some(feature_id);
        self
    }

    pub fn main_surface_only(mut self) -> Self {
        self.pass.view_scope = RenderPassViewScope::MainSurfaceOnly;
        self
    }

    pub fn offscreen_products_only(mut self) -> Self {
        self.pass.view_scope = RenderPassViewScope::OffscreenProductsOnly;
        self
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
        let uniform_id = allocate_uniform_id::<U>(&mut self.flow, &self.pass)?;
        add_uniform_state_binding::<S, U, F>(&mut self.pass, binding, uniform_id, projection);
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
        add_uniform_state_binding::<S, U, F>(
            &mut self.pass,
            binding,
            handle.diagnostic_identity(),
            projection,
        );
        self
    }

    pub fn bind_storage(mut self, binding: GpuBindingKey, handle: GpuBufferHandle) -> Self {
        let id = handle.diagnostic_identity();
        add_shader_binding(
            &mut self.pass,
            binding,
            RenderShaderBindingResource::StorageBuffer {
                resource: id,
                access: GpuStorageBufferAccess::ReadWrite,
            },
        );
        push_unique_resource(&mut self.pass.storage_reads, id);
        push_unique_resource(&mut self.pass.storage_writes, id);
        self
    }

    pub fn write_texture(
        mut self,
        binding: GpuBindingKey,
        resource_label: impl Into<String>,
    ) -> Self {
        let id = require_resource_id(&self.flow, resource_label.into().as_str());
        add_shader_binding(
            &mut self.pass,
            binding,
            RenderShaderBindingResource::StorageTexture {
                resource: id,
                access: GpuStorageTextureAccess::WriteOnly,
            },
        );
        push_unique_resource(&mut self.pass.write_textures, id);
        self
    }

    pub fn bind_ping_pong_storage(
        mut self,
        a_binding: GpuBindingKey,
        b_binding: GpuBindingKey,
        label: impl Into<String>,
    ) -> Self {
        let label = label.into();
        let (a_id, b_id) = require_ping_pong_storage(&self.flow, label.as_str());
        for (binding, id) in [(a_binding, a_id), (b_binding, b_id)] {
            add_shader_binding(
                &mut self.pass,
                binding,
                RenderShaderBindingResource::StorageBuffer {
                    resource: id,
                    access: GpuStorageBufferAccess::ReadWrite,
                },
            );
            push_unique_resource(&mut self.pass.storage_reads, id);
            push_unique_resource(&mut self.pass.storage_writes, id);
        }
        self
    }

    pub fn dispatch(mut self, xyz: [u32; 3]) -> Self {
        self.pass.compute_dispatch = Some(ComputeDispatchDescriptor::Fixed(xyz));
        self
    }

    pub fn dispatch_from_state<S>(mut self, projection: fn(&S) -> [u32; 3]) -> Self
    where
        S: ecs::Resource + Send + Sync + 'static,
    {
        self.pass.compute_dispatch = Some(ComputeDispatchDescriptor::State(
            ComputeDispatchBinding::state(projection),
        ));
        self
    }

    pub fn reads_current(
        self,
        a_binding: GpuBindingKey,
        b_binding: GpuBindingKey,
        label: impl Into<String>,
    ) -> Self {
        self.bind_ping_pong_storage(a_binding, b_binding, label)
    }

    pub fn writes_next(
        self,
        a_binding: GpuBindingKey,
        b_binding: GpuBindingKey,
        label: impl Into<String>,
    ) -> Self {
        self.bind_ping_pong_storage(a_binding, b_binding, label)
    }

    pub fn order_after(mut self, pass_label: impl Into<String>) -> Self {
        add_non_data_order_by_label(&self.flow, &mut self.pass, pass_label.into().as_str());
        self
    }

    pub fn finish(self) -> RenderFlow {
        self.flow.push_pass(self.pass)
    }
}

#[derive(Debug)]
pub struct FullscreenPassBuilder {
    flow: RenderFlow,
    pass: RenderPassNode,
}

impl FullscreenPassBuilder {
    pub(crate) fn new(mut flow: RenderFlow, label: String) -> Self {
        let pass = new_pass(&mut flow, label, RenderPassKind::Fullscreen);
        Self { flow, pass }
    }

    pub fn shader(mut self, shader: ShaderHandle) -> Self {
        self.pass.shader = Some(RenderShaderReference::RegistryHandle(shader));
        self
    }

    pub fn shader_asset(mut self, path: impl Into<String>) -> Self {
        self.pass.shader = Some(RenderShaderReference::AssetPath(path.into()));
        self
    }

    pub fn material_scene_shader_asset(mut self, fallback_asset: impl Into<String>) -> Self {
        self.pass.shader = Some(RenderShaderReference::MaterialSceneBundle {
            fallback_asset: fallback_asset.into(),
        });
        self
    }

    pub fn for_feature(mut self, feature_id: RenderFeatureId) -> Self {
        self.pass.feature_id = Some(feature_id);
        self
    }

    pub fn main_surface_only(mut self) -> Self {
        self.pass.view_scope = RenderPassViewScope::MainSurfaceOnly;
        self
    }

    pub fn offscreen_products_only(mut self) -> Self {
        self.pass.view_scope = RenderPassViewScope::OffscreenProductsOnly;
        self
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
        let uniform_id = allocate_uniform_id::<U>(&mut self.flow, &self.pass)?;
        add_uniform_state_binding::<S, U, F>(&mut self.pass, binding, uniform_id, projection);
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
        let uniform_id = allocate_uniform_id::<U>(&mut self.flow, &self.pass)?;
        add_uniform_state_with_surface_binding::<S, U, F>(
            &mut self.pass,
            binding,
            uniform_id,
            projection,
        );
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
        add_uniform_state_binding::<S, U, F>(
            &mut self.pass,
            binding,
            handle.diagnostic_identity(),
            projection,
        );
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
        add_uniform_state_with_surface_binding::<S, U, F>(
            &mut self.pass,
            binding,
            handle.diagnostic_identity(),
            projection,
        );
        self
    }

    pub fn bind_storage(mut self, binding: GpuBindingKey, handle: GpuBufferHandle) -> Self {
        let id = handle.diagnostic_identity();
        add_shader_binding(
            &mut self.pass,
            binding,
            RenderShaderBindingResource::StorageBuffer {
                resource: id,
                access: GpuStorageBufferAccess::ReadOnly,
            },
        );
        push_unique_resource(&mut self.pass.storage_reads, id);
        self
    }

    pub fn sample_texture(
        mut self,
        texture_binding: GpuBindingKey,
        sampler_binding: GpuBindingKey,
        resource_label: impl Into<String>,
    ) -> Self {
        let id = require_resource_id(&self.flow, resource_label.into().as_str());
        add_shader_binding(
            &mut self.pass,
            texture_binding,
            RenderShaderBindingResource::SampledTexture(id),
        );
        add_shader_binding(
            &mut self.pass,
            sampler_binding,
            RenderShaderBindingResource::Sampler,
        );
        push_unique_resource(&mut self.pass.sampled_textures, id);
        self
    }

    pub fn write_texture(
        mut self,
        binding: GpuBindingKey,
        resource_label: impl Into<String>,
    ) -> Self {
        let id = require_resource_id(&self.flow, resource_label.into().as_str());
        add_shader_binding(
            &mut self.pass,
            binding,
            RenderShaderBindingResource::StorageTexture {
                resource: id,
                access: GpuStorageTextureAccess::WriteOnly,
            },
        );
        push_unique_resource(&mut self.pass.write_textures, id);
        self
    }

    pub fn bind_ping_pong_storage(
        mut self,
        a_binding: GpuBindingKey,
        b_binding: GpuBindingKey,
        label: impl Into<String>,
    ) -> Self {
        let label = label.into();
        let (a_id, b_id) = require_ping_pong_storage(&self.flow, label.as_str());
        for (binding, id) in [(a_binding, a_id), (b_binding, b_id)] {
            add_shader_binding(
                &mut self.pass,
                binding,
                RenderShaderBindingResource::StorageBuffer {
                    resource: id,
                    access: GpuStorageBufferAccess::ReadOnly,
                },
            );
            push_unique_resource(&mut self.pass.storage_reads, id);
        }
        self
    }

    pub fn write_surface_color(mut self) -> Result<Self, RenderFlowAuthoringError> {
        let id = self.flow.ensure_surface_color_resource()?;
        push_unique_resource(&mut self.pass.color_outputs, id);
        Ok(self)
    }

    /// Writes this pass into a raster color attachment.
    ///
    /// Validation accepts flow-owned color targets and the imported surface color.
    pub fn write_color_target(mut self, resource_label: impl Into<String>) -> Self {
        let id = require_resource_id(&self.flow, resource_label.into().as_str());
        push_unique_resource(&mut self.pass.color_outputs, id);
        self
    }

    pub fn write_target_alias(self, target_alias: impl Into<String>) -> Self {
        self.write_color_target(target_alias)
    }

    pub fn clear_color(mut self, color: [f32; 4]) -> Self {
        self.pass.clear_color = Some(color);
        self
    }

    pub fn order_after(mut self, pass_label: impl Into<String>) -> Self {
        add_non_data_order_by_label(&self.flow, &mut self.pass, pass_label.into().as_str());
        self
    }

    pub fn finish(self) -> RenderFlow {
        self.flow.push_pass(self.pass)
    }
}

#[derive(Debug)]
pub struct GraphicsPassBuilder {
    flow: RenderFlow,
    pass: RenderPassNode,
}

impl GraphicsPassBuilder {
    pub(crate) fn new(mut flow: RenderFlow, label: String) -> Self {
        let pass = new_pass(&mut flow, label, RenderPassKind::Graphics);
        Self { flow, pass }
    }

    pub fn shader(mut self, shader: ShaderHandle) -> Self {
        self.pass.shader = Some(RenderShaderReference::RegistryHandle(shader));
        self
    }

    pub fn shader_asset(mut self, path: impl Into<String>) -> Self {
        self.pass.shader = Some(RenderShaderReference::AssetPath(path.into()));
        self
    }

    pub fn material_scene_shader_asset(mut self, fallback_asset: impl Into<String>) -> Self {
        self.pass.shader = Some(RenderShaderReference::MaterialSceneBundle {
            fallback_asset: fallback_asset.into(),
        });
        self
    }

    pub fn for_feature(mut self, feature_id: RenderFeatureId) -> Self {
        self.pass.feature_id = Some(feature_id);
        self
    }

    pub fn main_surface_only(mut self) -> Self {
        self.pass.view_scope = RenderPassViewScope::MainSurfaceOnly;
        self
    }

    pub fn offscreen_products_only(mut self) -> Self {
        self.pass.view_scope = RenderPassViewScope::OffscreenProductsOnly;
        self
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
        let uniform_id = allocate_uniform_id::<U>(&mut self.flow, &self.pass)?;
        add_uniform_state_binding::<S, U, F>(&mut self.pass, binding, uniform_id, projection);
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
        let uniform_id = allocate_uniform_id::<U>(&mut self.flow, &self.pass)?;
        add_uniform_state_with_surface_binding::<S, U, F>(
            &mut self.pass,
            binding,
            uniform_id,
            projection,
        );
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
        add_uniform_state_binding::<S, U, F>(
            &mut self.pass,
            binding,
            handle.diagnostic_identity(),
            projection,
        );
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
        add_uniform_state_with_surface_binding::<S, U, F>(
            &mut self.pass,
            binding,
            handle.diagnostic_identity(),
            projection,
        );
        self
    }

    pub fn bind_storage(mut self, binding: GpuBindingKey, handle: GpuBufferHandle) -> Self {
        let id = handle.diagnostic_identity();
        add_shader_binding(
            &mut self.pass,
            binding,
            RenderShaderBindingResource::StorageBuffer {
                resource: id,
                access: GpuStorageBufferAccess::ReadOnly,
            },
        );
        push_unique_resource(&mut self.pass.storage_reads, id);
        self
    }

    pub fn bind_ping_pong_storage(
        mut self,
        a_binding: GpuBindingKey,
        b_binding: GpuBindingKey,
        label: impl Into<String>,
    ) -> Self {
        let label = label.into();
        let (a_id, b_id) = require_ping_pong_storage(&self.flow, label.as_str());
        for (binding, id) in [(a_binding, a_id), (b_binding, b_id)] {
            add_shader_binding(
                &mut self.pass,
                binding,
                RenderShaderBindingResource::StorageBuffer {
                    resource: id,
                    access: GpuStorageBufferAccess::ReadOnly,
                },
            );
            push_unique_resource(&mut self.pass.storage_reads, id);
        }
        self
    }

    pub fn sample_texture(
        mut self,
        texture_binding: GpuBindingKey,
        sampler_binding: GpuBindingKey,
        resource_label: impl Into<String>,
    ) -> Self {
        let id = require_resource_id(&self.flow, resource_label.into().as_str());
        add_shader_binding(
            &mut self.pass,
            texture_binding,
            RenderShaderBindingResource::SampledTexture(id),
        );
        add_shader_binding(
            &mut self.pass,
            sampler_binding,
            RenderShaderBindingResource::Sampler,
        );
        push_unique_resource(&mut self.pass.sampled_textures, id);
        self
    }

    pub fn write_texture(
        mut self,
        binding: GpuBindingKey,
        resource_label: impl Into<String>,
    ) -> Self {
        let id = require_resource_id(&self.flow, resource_label.into().as_str());
        add_shader_binding(
            &mut self.pass,
            binding,
            RenderShaderBindingResource::StorageTexture {
                resource: id,
                access: GpuStorageTextureAccess::WriteOnly,
            },
        );
        push_unique_resource(&mut self.pass.write_textures, id);
        self
    }

    pub fn write_surface_color(mut self) -> Result<Self, RenderFlowAuthoringError> {
        let id = self.flow.ensure_surface_color_resource()?;
        push_unique_resource(&mut self.pass.color_outputs, id);
        Ok(self)
    }

    /// Writes this pass into a raster color attachment.
    ///
    /// Validation accepts flow-owned color targets and the imported surface color.
    pub fn write_color_target(mut self, resource_label: impl Into<String>) -> Self {
        let id = require_resource_id(&self.flow, resource_label.into().as_str());
        push_unique_resource(&mut self.pass.color_outputs, id);
        self
    }

    pub fn write_target_alias(self, target_alias: impl Into<String>) -> Self {
        self.write_color_target(target_alias)
    }

    pub fn depth_target(mut self, resource_label: impl Into<String>) -> Self {
        let id = require_resource_id(&self.flow, resource_label.into().as_str());
        self.pass.depth_target = Some(id);
        self
    }

    pub fn vertex_buffer(self, handle: GpuBufferHandle, layout: RenderVertexBufferLayout) -> Self {
        self.push_vertex_buffer_resource(handle.diagnostic_identity(), layout)
    }

    pub fn index_buffer(self, handle: GpuBufferHandle) -> Self {
        self.push_index_buffer_resource(handle.diagnostic_identity())
    }

    pub fn instance_buffer(
        self,
        handle: GpuBufferHandle,
        layout: RenderVertexBufferLayout,
    ) -> Self {
        self.push_instance_buffer_resource(handle.diagnostic_identity(), layout)
    }

    pub fn indirect_buffer(mut self, handle: GpuBufferHandle) -> Self {
        let id = handle.diagnostic_identity();
        push_unique_resource(&mut self.pass.indirect_buffers, id);
        self
    }

    pub fn clear_color(mut self, color: [f32; 4]) -> Self {
        self.pass.clear_color = Some(color);
        self
    }

    pub fn raster_state(mut self, state: RenderRasterState) -> Self {
        self.pass.raster_state = state;
        self
    }

    pub fn draw(mut self, vertex_count: u32, instance_count: u32) -> Self {
        self.pass.draw = Some(RenderDrawDescriptor::new(vertex_count, instance_count));
        self
    }

    pub fn draw_indirect(
        mut self,
        args_buffer: GpuBufferHandle,
        vertex_count: u32,
        instance_count: u32,
    ) -> Result<Self, RenderFlowAuthoringError> {
        let id = args_buffer.diagnostic_identity();
        let element_count = self
            .flow
            .indirect_buffer_element_count::<DrawIndirectArgs>(&args_buffer)?;
        push_unique_resource(&mut self.pass.indirect_buffers, id);
        self.pass.draw = Some(RenderDrawDescriptor::indirect(
            vertex_count,
            instance_count,
            RenderIndirectDrawResource::new(
                id,
                DrawIndirectArgs::ARGS_KIND,
                element_count,
                DrawIndirectArgs::BYTE_SIZE,
                0,
            ),
        ));
        Ok(self)
    }

    pub fn draw_indexed_indirect(
        mut self,
        args_buffer: GpuBufferHandle,
        index_count: u32,
        instance_count: u32,
    ) -> Result<Self, RenderFlowAuthoringError> {
        let id = args_buffer.diagnostic_identity();
        let element_count = self
            .flow
            .indirect_buffer_element_count::<DrawIndexedIndirectArgs>(&args_buffer)?;
        push_unique_resource(&mut self.pass.indirect_buffers, id);
        self.pass.draw = Some(RenderDrawDescriptor::indirect(
            index_count,
            instance_count,
            RenderIndirectDrawResource::new(
                id,
                DrawIndexedIndirectArgs::ARGS_KIND,
                element_count,
                DrawIndexedIndirectArgs::BYTE_SIZE,
                0,
            ),
        ));
        Ok(self)
    }

    pub fn draw_with_offsets(
        mut self,
        vertex_count: u32,
        instance_count: u32,
        first_vertex: u32,
        first_instance: u32,
    ) -> Self {
        self.pass.draw = Some(RenderDrawDescriptor::with_offsets(
            vertex_count,
            instance_count,
            first_vertex,
            first_instance,
        ));
        self
    }

    pub fn draw_indirect_with_offsets(
        mut self,
        args_buffer: GpuBufferHandle,
        vertex_count: u32,
        instance_count: u32,
        first_vertex: u32,
        first_instance: u32,
        byte_offset: u64,
    ) -> Result<Self, RenderFlowAuthoringError> {
        let id = args_buffer.diagnostic_identity();
        let element_count = self
            .flow
            .indirect_buffer_element_count::<DrawIndirectArgs>(&args_buffer)?;
        push_unique_resource(&mut self.pass.indirect_buffers, id);
        self.pass.draw = Some(RenderDrawDescriptor::indirect_with_offsets(
            vertex_count,
            instance_count,
            first_vertex,
            first_instance,
            RenderIndirectDrawResource::new(
                id,
                DrawIndirectArgs::ARGS_KIND,
                element_count,
                DrawIndirectArgs::BYTE_SIZE,
                byte_offset,
            ),
        ));
        Ok(self)
    }

    pub fn allow_instanced_fullscreen(
        mut self,
        max_instances: u32,
        reason: impl Into<String>,
    ) -> Self {
        self.pass.shape_intent = RenderPassShapeIntent::AdvancedInstancedFullscreen {
            max_instances,
            reason: reason.into(),
        };
        self
    }

    pub fn order_after(mut self, pass_label: impl Into<String>) -> Self {
        add_non_data_order_by_label(&self.flow, &mut self.pass, pass_label.into().as_str());
        self
    }

    pub fn finish(self) -> RenderFlow {
        self.flow.push_pass(self.pass)
    }

    pub(crate) fn push_vertex_buffer_resource(
        mut self,
        id: GpuWorkResourceId,
        layout: RenderVertexBufferLayout,
    ) -> Self {
        push_unique_resource(&mut self.pass.vertex_buffers, id);
        self.pass.vertex_buffer_layouts.push(layout);
        self
    }

    pub(crate) fn push_index_buffer_resource(mut self, id: GpuWorkResourceId) -> Self {
        push_unique_resource(&mut self.pass.index_buffers, id);
        self
    }

    pub(crate) fn push_instance_buffer_resource(
        mut self,
        id: GpuWorkResourceId,
        layout: RenderVertexBufferLayout,
    ) -> Self {
        push_unique_resource(&mut self.pass.instance_buffers, id);
        self.pass.instance_buffer_layouts.push(layout);
        self
    }

    pub(crate) fn push_uniform_binding(
        mut self,
        binding_key: GpuBindingKey,
        binding: PassParamBinding,
    ) -> Self {
        let uniform_id = *binding.uniform_id();
        add_shader_binding(
            &mut self.pass,
            binding_key,
            RenderShaderBindingResource::UniformBuffer(uniform_id),
        );
        self.pass.uniform_bindings.push(binding);
        self
    }

    pub(crate) fn draw_indirect_resource(
        mut self,
        indirect: RenderIndirectDrawResource,
        vertex_count: u32,
        instance_count: u32,
    ) -> Self {
        push_unique_resource(&mut self.pass.indirect_buffers, indirect.args_buffer);
        self.pass.draw = Some(RenderDrawDescriptor::indirect(
            vertex_count,
            instance_count,
            indirect,
        ));
        self
    }
}

#[derive(Debug)]
pub struct CopyPassBuilder {
    flow: RenderFlow,
    pass: RenderPassNode,
}

impl CopyPassBuilder {
    pub(crate) fn new(mut flow: RenderFlow, label: String) -> Self {
        let pass = new_pass(&mut flow, label, RenderPassKind::Copy);
        Self { flow, pass }
    }

    pub fn main_surface_only(mut self) -> Self {
        self.pass.view_scope = RenderPassViewScope::MainSurfaceOnly;
        self
    }

    pub fn offscreen_products_only(mut self) -> Self {
        self.pass.view_scope = RenderPassViewScope::OffscreenProductsOnly;
        self
    }

    pub fn source(mut self, resource_label: impl Into<String>) -> Self {
        let id = require_resource_id(&self.flow, resource_label.into().as_str());
        self.pass.copy_source = Some(id);
        self
    }

    pub fn destination(mut self, resource_label: impl Into<String>) -> Self {
        let id = require_resource_id(&self.flow, resource_label.into().as_str());
        self.pass.copy_destination = Some(id);
        self
    }

    pub fn order_after(mut self, pass_label: impl Into<String>) -> Self {
        add_non_data_order_by_label(&self.flow, &mut self.pass, pass_label.into().as_str());
        self
    }

    pub fn finish(self) -> RenderFlow {
        self.flow.push_pass(self.pass)
    }
}

#[derive(Debug)]
pub struct PresentPassBuilder {
    flow: RenderFlow,
    pass: RenderPassNode,
}

impl PresentPassBuilder {
    pub(crate) fn new(
        mut flow: RenderFlow,
        label: String,
    ) -> Result<Self, RenderFlowAuthoringError> {
        flow.ensure_surface_color_resource()?;
        let pass = new_pass(&mut flow, label, RenderPassKind::Present);
        Ok(Self { flow, pass })
    }

    pub fn main_surface_only(mut self) -> Self {
        self.pass.view_scope = RenderPassViewScope::MainSurfaceOnly;
        self
    }

    pub fn source(mut self, resource_label: impl Into<String>) -> Self {
        let id = require_resource_id(&self.flow, resource_label.into().as_str());
        self.pass.present_source = Some(id);
        self
    }

    pub fn surface_color(mut self) -> Result<Self, RenderFlowAuthoringError> {
        let id = self.flow.ensure_surface_color_resource()?;
        self.pass.present_source = Some(id);
        Ok(self)
    }

    pub fn order_after(mut self, pass_label: impl Into<String>) -> Self {
        add_non_data_order_by_label(&self.flow, &mut self.pass, pass_label.into().as_str());
        self
    }

    pub fn finish(self) -> RenderFlow {
        self.flow.push_pass(self.pass)
    }
}

#[derive(Debug)]
pub struct BuiltinUiCompositePassBuilder {
    flow: RenderFlow,
    pass: RenderPassNode,
}

impl BuiltinUiCompositePassBuilder {
    pub(crate) fn new(
        mut flow: RenderFlow,
        label: String,
    ) -> Result<Self, RenderFlowAuthoringError> {
        let color_output = flow.ensure_surface_color_resource()?;
        let mut pass = new_pass(&mut flow, label, RenderPassKind::BuiltinUiComposite);
        push_unique_resource(&mut pass.color_outputs, color_output);
        Ok(Self { flow, pass })
    }

    pub fn main_surface_only(mut self) -> Self {
        self.pass.view_scope = RenderPassViewScope::MainSurfaceOnly;
        self
    }

    pub fn order_after(mut self, pass_label: impl Into<String>) -> Self {
        add_non_data_order_by_label(&self.flow, &mut self.pass, pass_label.into().as_str());
        self
    }

    pub fn finish(self) -> RenderFlow {
        self.flow.push_pass(self.pass)
    }
}

fn new_pass(flow: &mut RenderFlow, label: String, kind: RenderPassKind) -> RenderPassNode {
    let (pass_id, pass_label) = flow.allocate_pass(label);
    RenderPassNode::new(pass_id, pass_label, kind)
}

fn allocate_uniform_id<U>(
    flow: &mut RenderFlow,
    pass: &RenderPassNode,
) -> Result<GpuWorkResourceId, RenderFlowAuthoringError>
where
    U: GpuParams + 'static,
{
    Ok(*flow
        .allocate_uniform_resource::<U>(pass.label.as_str())?
        .diagnostic_identity_ref())
}

fn add_shader_binding(
    pass: &mut RenderPassNode,
    key: GpuBindingKey,
    resource: RenderShaderBindingResource,
) {
    pass.shader_bindings
        .push(RenderShaderBinding::new(key, resource));
}

fn add_uniform_state_binding<S, U, F>(
    pass: &mut RenderPassNode,
    binding: GpuBindingKey,
    uniform_id: GpuWorkResourceId,
    projection: F,
) where
    S: ecs::Resource + Send + Sync + 'static,
    U: GpuParams + Send + Sync + 'static,
    F: Fn(&S) -> U + Send + Sync + 'static,
{
    add_shader_binding(
        pass,
        binding,
        RenderShaderBindingResource::UniformBuffer(uniform_id),
    );
    pass.uniform_bindings
        .push(PassParamBinding::uniform_state(uniform_id, projection));
}

fn add_uniform_state_with_surface_binding<S, U, F>(
    pass: &mut RenderPassNode,
    binding: GpuBindingKey,
    uniform_id: GpuWorkResourceId,
    projection: F,
) where
    S: ecs::Resource + Send + Sync + 'static,
    U: GpuParams + Send + Sync + 'static,
    F: Fn(&S, (u32, u32)) -> U + Send + Sync + 'static,
{
    add_shader_binding(
        pass,
        binding,
        RenderShaderBindingResource::UniformBuffer(uniform_id),
    );
    pass.uniform_bindings
        .push(PassParamBinding::uniform_state_with_surface(
            uniform_id, projection,
        ));
}

fn add_non_data_order_by_label(flow: &RenderFlow, pass: &mut RenderPassNode, pass_label: &str) {
    let dependency = require_pass_id(flow, pass_label);
    push_unique_pass_dependency(&mut pass.non_data_order_after, dependency);
}

fn require_ping_pong_storage(
    flow: &RenderFlow,
    label: &str,
) -> (GpuWorkResourceId, GpuWorkResourceId) {
    flow.ping_pong_storage_ids(label).unwrap_or_else(|| {
        panic!(
            "ping-pong storage '{}' is not registered in flow '{}'",
            label,
            flow.label()
        )
    })
}

fn require_resource_id(flow: &RenderFlow, label: &str) -> GpuWorkResourceId {
    flow.resolve_resource_id(label).unwrap_or_else(|| {
        panic!(
            "resource label '{}' is not registered in flow '{}'",
            label,
            flow.label()
        )
    })
}

fn require_pass_id(flow: &RenderFlow, label: &str) -> RenderPassId {
    flow.resolve_pass_id(label).unwrap_or_else(|| {
        panic!(
            "pass label '{}' is not registered in flow '{}'",
            label,
            flow.label()
        )
    })
}

fn push_unique_resource(resources: &mut Vec<GpuWorkResourceId>, id: GpuWorkResourceId) {
    if resources.iter().all(|existing| *existing != id) {
        resources.push(id);
    }
}

fn push_unique_pass_dependency(dependencies: &mut Vec<RenderPassId>, id: RenderPassId) {
    if dependencies.iter().all(|existing| *existing != id) {
        dependencies.push(id);
    }
}
