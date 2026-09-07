use crate::plugins::render::api::ids::RenderFeatureId;
use crate::plugins::render::api::{
    ComputeDispatchDescriptor, PassParamBinding, RenderShaderBinding,
};
use crate::plugins::render::{GpuParams, GpuStorage, RenderPassId, ShaderHandle};
use runen_gpu::{GpuSpecializationValue, GpuWorkResourceId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RenderPassKind {
    Compute,
    Fullscreen,
    BuiltinUiComposite,
    Graphics,
    Copy,
    Present,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderPassViewScope {
    AllViews,
    MainSurfaceOnly,
    OffscreenProductsOnly,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum RenderPassShapeIntent {
    #[default]
    Default,
    AdvancedInstancedFullscreen {
        max_instances: u32,
        reason: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenderShaderReference {
    AssetPath(String),
    MaterialSceneBundle { fallback_asset: String },
    RegistryHandle(ShaderHandle),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RenderShaderConstant {
    pub name: String,
    pub value: GpuSpecializationValue,
}

impl RenderShaderConstant {
    pub fn new(name: impl Into<String>, value: impl Into<GpuSpecializationValue>) -> Self {
        Self {
            name: name.into(),
            value: value.into(),
        }
    }

    pub fn u32(name: impl Into<String>, value: u32) -> Self {
        Self::new(name, value)
    }

    pub fn i32(name: impl Into<String>, value: i32) -> Self {
        Self::new(name, value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RenderVertexStepMode {
    Vertex,
    Instance,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RenderPrimitiveTopology {
    TriangleList,
    TriangleStrip,
    LineList,
    LineStrip,
    PointList,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RenderBlendMode {
    Alpha,
    Replace,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RenderCullMode {
    None,
    Front,
    Back,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RenderDepthPolicy {
    Default,
    Disabled,
    ReadOnly,
    ReadWrite,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RenderRasterState {
    pub primitive_topology: RenderPrimitiveTopology,
    pub blend_mode: RenderBlendMode,
    pub cull_mode: RenderCullMode,
    pub depth_policy: RenderDepthPolicy,
}

impl Default for RenderRasterState {
    fn default() -> Self {
        Self {
            primitive_topology: RenderPrimitiveTopology::TriangleList,
            blend_mode: RenderBlendMode::Alpha,
            cull_mode: RenderCullMode::None,
            depth_policy: RenderDepthPolicy::Default,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RenderVertexFormat {
    Float32,
    Float32x2,
    Float32x3,
    Float32x4,
    Uint32,
    Uint32x2,
    Uint32x3,
    Uint32x4,
    Sint32,
    Sint32x2,
    Sint32x3,
    Sint32x4,
}

impl RenderVertexFormat {
    pub const fn size_bytes(self) -> u64 {
        match self {
            Self::Float32 | Self::Uint32 | Self::Sint32 => 4,
            Self::Float32x2 | Self::Uint32x2 | Self::Sint32x2 => 8,
            Self::Float32x3 | Self::Uint32x3 | Self::Sint32x3 => 12,
            Self::Float32x4 | Self::Uint32x4 | Self::Sint32x4 => 16,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RenderVertexAttribute {
    pub shader_location: u32,
    pub offset: u64,
    pub format: RenderVertexFormat,
}

impl RenderVertexAttribute {
    pub const fn new(shader_location: u32, offset: u64, format: RenderVertexFormat) -> Self {
        Self {
            shader_location,
            offset,
            format,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RenderVertexBufferLayout {
    pub slot: u32,
    pub array_stride: u64,
    pub step_mode: RenderVertexStepMode,
    pub attributes: Vec<RenderVertexAttribute>,
}

impl RenderVertexBufferLayout {
    pub fn vertex(slot: u32, array_stride: u64) -> Self {
        Self {
            slot,
            array_stride,
            step_mode: RenderVertexStepMode::Vertex,
            attributes: Vec::new(),
        }
    }

    pub fn instance(slot: u32, array_stride: u64) -> Self {
        Self {
            slot,
            array_stride,
            step_mode: RenderVertexStepMode::Instance,
            attributes: Vec::new(),
        }
    }

    pub fn attribute(
        mut self,
        shader_location: u32,
        offset: u64,
        format: RenderVertexFormat,
    ) -> Self {
        self.attributes
            .push(RenderVertexAttribute::new(shader_location, offset, format));
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RenderDrawSource {
    Direct,
    Indirect {
        args_buffer: GpuWorkResourceId,
        args_kind: RenderIndirectDrawArgsKind,
        args_element_count: u64,
        args_element_size: u64,
        byte_offset: u64,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RenderIndirectDrawResource {
    pub args_buffer: GpuWorkResourceId,
    pub args_kind: RenderIndirectDrawArgsKind,
    pub args_element_count: u64,
    pub args_element_size: u64,
    pub byte_offset: u64,
}

impl RenderIndirectDrawResource {
    pub const fn new(
        args_buffer: GpuWorkResourceId,
        args_kind: RenderIndirectDrawArgsKind,
        args_element_count: u64,
        args_element_size: u64,
        byte_offset: u64,
    ) -> Self {
        Self {
            args_buffer,
            args_kind,
            args_element_count,
            args_element_size,
            byte_offset,
        }
    }
}

impl RenderDrawSource {
    pub const fn indirect(
        args_buffer: GpuWorkResourceId,
        args_kind: RenderIndirectDrawArgsKind,
        args_element_count: u64,
        args_element_size: u64,
        byte_offset: u64,
    ) -> Self {
        Self::Indirect {
            args_buffer,
            args_kind,
            args_element_count,
            args_element_size,
            byte_offset,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RenderIndirectDrawArgsKind {
    Draw,
    DrawIndexed,
}

impl RenderIndirectDrawArgsKind {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Draw => "DrawIndirectArgs",
            Self::DrawIndexed => "DrawIndexedIndirectArgs",
        }
    }
}

pub trait IndirectDrawArgsBuffer: GpuParams {
    const ARGS_KIND: RenderIndirectDrawArgsKind;
    const BYTE_SIZE: u64;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RenderFixedStepRegionId(u64);

impl RenderFixedStepRegionId {
    pub const fn new(raw: u64) -> Self {
        Self(raw)
    }

    pub const fn raw(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RenderFixedStepRegionMembership {
    pub region_id: RenderFixedStepRegionId,
    pub region_label: String,
    pub max_substeps: u32,
    pub iteration_uniform: GpuWorkResourceId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, GpuStorage)]
pub struct DrawIndirectArgs {
    pub vertex_count: u32,
    pub instance_count: u32,
    pub first_vertex: u32,
    pub first_instance: u32,
}

impl DrawIndirectArgs {
    pub const BYTE_SIZE: u64 = 16;

    pub const fn new(
        vertex_count: u32,
        instance_count: u32,
        first_vertex: u32,
        first_instance: u32,
    ) -> Self {
        Self {
            vertex_count,
            instance_count,
            first_vertex,
            first_instance,
        }
    }
}

impl IndirectDrawArgsBuffer for DrawIndirectArgs {
    const ARGS_KIND: RenderIndirectDrawArgsKind = RenderIndirectDrawArgsKind::Draw;
    const BYTE_SIZE: u64 = Self::BYTE_SIZE;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, GpuStorage)]
pub struct DrawIndexedIndirectArgs {
    pub index_count: u32,
    pub instance_count: u32,
    pub first_index: u32,
    pub base_vertex: i32,
    pub first_instance: u32,
}

impl DrawIndexedIndirectArgs {
    pub const BYTE_SIZE: u64 = 20;

    pub const fn new(
        index_count: u32,
        instance_count: u32,
        first_index: u32,
        base_vertex: i32,
        first_instance: u32,
    ) -> Self {
        Self {
            index_count,
            instance_count,
            first_index,
            base_vertex,
            first_instance,
        }
    }
}

impl IndirectDrawArgsBuffer for DrawIndexedIndirectArgs {
    const ARGS_KIND: RenderIndirectDrawArgsKind = RenderIndirectDrawArgsKind::DrawIndexed;
    const BYTE_SIZE: u64 = Self::BYTE_SIZE;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RenderDrawDescriptor {
    pub vertex_count: u32,
    pub instance_count: u32,
    pub first_vertex: u32,
    pub first_instance: u32,
    pub source: RenderDrawSource,
}

impl RenderDrawDescriptor {
    pub const fn new(vertex_count: u32, instance_count: u32) -> Self {
        Self {
            vertex_count,
            instance_count,
            first_vertex: 0,
            first_instance: 0,
            source: RenderDrawSource::Direct,
        }
    }

    pub const fn with_offsets(
        vertex_count: u32,
        instance_count: u32,
        first_vertex: u32,
        first_instance: u32,
    ) -> Self {
        Self {
            vertex_count,
            instance_count,
            first_vertex,
            first_instance,
            source: RenderDrawSource::Direct,
        }
    }

    pub const fn indirect(
        vertex_count: u32,
        instance_count: u32,
        indirect: RenderIndirectDrawResource,
    ) -> Self {
        Self {
            vertex_count,
            instance_count,
            first_vertex: 0,
            first_instance: 0,
            source: RenderDrawSource::Indirect {
                args_buffer: indirect.args_buffer,
                args_kind: indirect.args_kind,
                args_element_count: indirect.args_element_count,
                args_element_size: indirect.args_element_size,
                byte_offset: indirect.byte_offset,
            },
        }
    }

    pub const fn indirect_with_offsets(
        vertex_count: u32,
        instance_count: u32,
        first_vertex: u32,
        first_instance: u32,
        indirect: RenderIndirectDrawResource,
    ) -> Self {
        Self {
            vertex_count,
            instance_count,
            first_vertex,
            first_instance,
            source: RenderDrawSource::Indirect {
                args_buffer: indirect.args_buffer,
                args_kind: indirect.args_kind,
                args_element_count: indirect.args_element_count,
                args_element_size: indirect.args_element_size,
                byte_offset: indirect.byte_offset,
            },
        }
    }
}

#[derive(Debug, Clone)]
pub struct RenderPassNode {
    pub id: RenderPassId,
    pub label: String,
    pub kind: RenderPassKind,
    pub view_scope: RenderPassViewScope,
    pub feature_id: Option<RenderFeatureId>,
    pub shape_intent: RenderPassShapeIntent,
    pub shader: Option<RenderShaderReference>,
    pub shader_constants: Vec<RenderShaderConstant>,
    /// Explicit shader-visible binding identity. `GpuBindingKey` is authoritative;
    /// the resource/hazard vectors below must not be used to reconstruct slots.
    pub shader_bindings: Vec<RenderShaderBinding>,
    /// Render-owned storage hazard semantics. Generic access and hazard truth
    /// is derived immediately by the RunenGPU work adapter.
    pub storage_reads: Vec<GpuWorkResourceId>,
    pub storage_writes: Vec<GpuWorkResourceId>,
    /// Ordered raster color outputs. Attachment access is owned by the lowered
    /// `GpuRenderOperation`, not by a parallel renderer access list.
    pub color_outputs: Vec<GpuWorkResourceId>,
    pub copy_source: Option<GpuWorkResourceId>,
    pub copy_destination: Option<GpuWorkResourceId>,
    pub present_source: Option<GpuWorkResourceId>,
    /// Render-only non-data ordering requests. The adapter lowers these to
    /// `GpuExplicitOrder`; G3 rejects any request that duplicates typed hazard
    /// order.
    pub non_data_order_after: Vec<RenderPassId>,
    pub workgroup_size: Option<[u32; 3]>,
    pub clear_color: Option<[f32; 4]>,
    pub compute_dispatch: Option<ComputeDispatchDescriptor>,
    pub sampled_textures: Vec<GpuWorkResourceId>,
    pub write_textures: Vec<GpuWorkResourceId>,
    pub vertex_buffers: Vec<GpuWorkResourceId>,
    pub vertex_buffer_layouts: Vec<RenderVertexBufferLayout>,
    pub index_buffers: Vec<GpuWorkResourceId>,
    pub instance_buffers: Vec<GpuWorkResourceId>,
    pub instance_buffer_layouts: Vec<RenderVertexBufferLayout>,
    pub indirect_buffers: Vec<GpuWorkResourceId>,
    pub depth_target: Option<GpuWorkResourceId>,
    pub raster_state: RenderRasterState,
    pub draw: Option<RenderDrawDescriptor>,
    pub uniform_bindings: Vec<PassParamBinding>,
    pub fixed_step_region: Option<RenderFixedStepRegionMembership>,
    pub fixed_step_iteration_uniforms: Vec<GpuWorkResourceId>,
}

impl RenderPassNode {
    pub fn new(
        id: impl Into<RenderPassId>,
        label: impl Into<String>,
        kind: RenderPassKind,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            kind,
            view_scope: RenderPassViewScope::AllViews,
            feature_id: None,
            shape_intent: RenderPassShapeIntent::Default,
            shader: None,
            shader_constants: Vec::new(),
            shader_bindings: Vec::new(),
            storage_reads: Vec::new(),
            storage_writes: Vec::new(),
            color_outputs: Vec::new(),
            copy_source: None,
            copy_destination: None,
            present_source: None,
            non_data_order_after: Vec::new(),
            workgroup_size: None,
            clear_color: None,
            compute_dispatch: None,
            sampled_textures: Vec::new(),
            write_textures: Vec::new(),
            vertex_buffers: Vec::new(),
            vertex_buffer_layouts: Vec::new(),
            index_buffers: Vec::new(),
            instance_buffers: Vec::new(),
            instance_buffer_layouts: Vec::new(),
            indirect_buffers: Vec::new(),
            depth_target: None,
            raster_state: RenderRasterState::default(),
            draw: None,
            uniform_bindings: Vec::new(),
            fixed_step_region: None,
            fixed_step_iteration_uniforms: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct PassGraph {
    pub passes: Vec<RenderPassNode>,
}

impl PassGraph {
    pub fn add_pass(&mut self, pass: RenderPassNode) {
        self.passes.push(pass);
    }
}
