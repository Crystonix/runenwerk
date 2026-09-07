use crate::plugins::render::{
    RenderBlendMode, RenderCullMode, RenderDepthPolicy, RenderPrimitiveTopology, RenderRasterState,
    RenderVertexBufferLayout, ShaderHandle,
};
use runen_gpu::{GpuBufferHandle, GpuWorkResourceId};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProceduralShader {
    AssetPath(String),
    RegistryHandle(ShaderHandle),
}

impl ProceduralShader {
    pub fn asset(path: impl Into<String>) -> Self {
        Self::AssetPath(path.into())
    }

    pub const fn handle(handle: ShaderHandle) -> Self {
        Self::RegistryHandle(handle)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProceduralBufferBinding {
    pub resource_id: GpuWorkResourceId,
    pub layout: RenderVertexBufferLayout,
}

impl ProceduralBufferBinding {
    pub fn storage(handle: GpuBufferHandle, layout: RenderVertexBufferLayout) -> Self {
        Self {
            resource_id: handle.diagnostic_identity(),
            layout,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProceduralIndexBuffer {
    pub resource_id: GpuWorkResourceId,
}

impl ProceduralIndexBuffer {
    pub fn storage(handle: GpuBufferHandle) -> Self {
        Self {
            resource_id: handle.diagnostic_identity(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProceduralSdf2dImpostorDescriptor {
    pub shader_payload: ProceduralSdf2dPayload,
}

impl ProceduralSdf2dImpostorDescriptor {
    pub const fn local_2d() -> Self {
        Self {
            shader_payload: ProceduralSdf2dPayload::Local2d,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProceduralSdf2dPayload {
    Local2d,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProceduralVisualDescriptor {
    MeshSprite {
        vertex_buffer: ProceduralBufferBinding,
        vertex_count: u32,
    },
    QuadSprite {
        vertex_count: u32,
    },
    LocalSdf2dImpostor {
        sdf: ProceduralSdf2dImpostorDescriptor,
        vertex_count: u32,
    },
}

impl ProceduralVisualDescriptor {
    pub fn mesh_sprite(vertex_buffer: ProceduralBufferBinding, vertex_count: u32) -> Self {
        Self::MeshSprite {
            vertex_buffer,
            vertex_count,
        }
    }

    pub const fn quad_sprite() -> Self {
        Self::QuadSprite { vertex_count: 6 }
    }

    pub const fn local_sdf_2d_impostor(sdf: ProceduralSdf2dImpostorDescriptor) -> Self {
        Self::LocalSdf2dImpostor {
            sdf,
            vertex_count: 6,
        }
    }

    pub const fn vertex_count(&self) -> u32 {
        match self {
            Self::MeshSprite { vertex_count, .. }
            | Self::QuadSprite { vertex_count }
            | Self::LocalSdf2dImpostor { vertex_count, .. } => *vertex_count,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProceduralTargetDescriptor {
    pub color_target: String,
    pub depth_target: Option<String>,
    pub clear_color: Option<[f32; 4]>,
}

impl ProceduralTargetDescriptor {
    pub fn color(color_target: impl Into<String>) -> Self {
        Self {
            color_target: color_target.into(),
            depth_target: None,
            clear_color: None,
        }
    }

    pub fn depth_target(mut self, depth_target: impl Into<String>) -> Self {
        self.depth_target = Some(depth_target.into());
        self
    }

    pub const fn clear_color(mut self, clear_color: [f32; 4]) -> Self {
        self.clear_color = Some(clear_color);
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ProceduralRenderPolicy {
    pub primitive_topology: RenderPrimitiveTopology,
    pub blend_mode: RenderBlendMode,
    pub depth_policy: RenderDepthPolicy,
    pub cull_mode: RenderCullMode,
}

impl Default for ProceduralRenderPolicy {
    fn default() -> Self {
        Self {
            primitive_topology: RenderPrimitiveTopology::TriangleList,
            blend_mode: RenderBlendMode::Alpha,
            depth_policy: RenderDepthPolicy::Default,
            cull_mode: RenderCullMode::None,
        }
    }
}

impl ProceduralRenderPolicy {
    pub const fn primitive_topology(mut self, value: RenderPrimitiveTopology) -> Self {
        self.primitive_topology = value;
        self
    }

    pub const fn blend_mode(mut self, value: RenderBlendMode) -> Self {
        self.blend_mode = value;
        self
    }

    pub const fn depth_policy(mut self, value: RenderDepthPolicy) -> Self {
        self.depth_policy = value;
        self
    }

    pub const fn cull_mode(mut self, value: RenderCullMode) -> Self {
        self.cull_mode = value;
        self
    }
}

impl From<ProceduralRenderPolicy> for RenderRasterState {
    fn from(value: ProceduralRenderPolicy) -> Self {
        Self {
            primitive_topology: value.primitive_topology,
            blend_mode: value.blend_mode,
            cull_mode: value.cull_mode,
            depth_policy: value.depth_policy,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProceduralPassDescriptor {
    pub label: String,
    pub shader: Option<ProceduralShader>,
    pub visual: ProceduralVisualDescriptor,
    pub instance_buffer: ProceduralBufferBinding,
    pub instance_count: u32,
    pub index_buffer: Option<ProceduralIndexBuffer>,
    pub target: Option<ProceduralTargetDescriptor>,
    pub policy: ProceduralRenderPolicy,
    pub non_data_order_after: Vec<String>,
}

impl ProceduralPassDescriptor {
    pub fn new(
        label: impl Into<String>,
        visual: ProceduralVisualDescriptor,
        instance_buffer: ProceduralBufferBinding,
        instance_count: u32,
    ) -> Self {
        Self {
            label: label.into(),
            shader: None,
            visual,
            instance_buffer,
            instance_count,
            index_buffer: None,
            target: None,
            policy: ProceduralRenderPolicy::default(),
            non_data_order_after: Vec::new(),
        }
    }

    pub fn mesh_sprites(
        label: impl Into<String>,
        vertex_buffer: ProceduralBufferBinding,
        vertex_count: u32,
        instance_buffer: ProceduralBufferBinding,
        instance_count: u32,
    ) -> Self {
        Self::new(
            label,
            ProceduralVisualDescriptor::mesh_sprite(vertex_buffer, vertex_count),
            instance_buffer,
            instance_count,
        )
    }

    pub fn quad_sprites(
        label: impl Into<String>,
        instance_buffer: ProceduralBufferBinding,
        instance_count: u32,
    ) -> Self {
        Self::new(
            label,
            ProceduralVisualDescriptor::quad_sprite(),
            instance_buffer,
            instance_count,
        )
    }

    pub fn local_sdf_2d_impostors(
        label: impl Into<String>,
        instance_buffer: ProceduralBufferBinding,
        instance_count: u32,
    ) -> Self {
        Self::new(
            label,
            ProceduralVisualDescriptor::local_sdf_2d_impostor(
                ProceduralSdf2dImpostorDescriptor::local_2d(),
            ),
            instance_buffer,
            instance_count,
        )
    }

    pub fn shader_asset(mut self, path: impl Into<String>) -> Self {
        self.shader = Some(ProceduralShader::asset(path));
        self
    }

    pub fn shader(mut self, shader: ShaderHandle) -> Self {
        self.shader = Some(ProceduralShader::handle(shader));
        self
    }

    pub fn index_buffer(mut self, index_buffer: ProceduralIndexBuffer) -> Self {
        self.index_buffer = Some(index_buffer);
        self
    }

    pub fn write_color_target(mut self, color_target: impl Into<String>) -> Self {
        self.target = Some(ProceduralTargetDescriptor::color(color_target));
        self
    }

    pub fn target(mut self, target: ProceduralTargetDescriptor) -> Self {
        self.target = Some(target);
        self
    }

    pub fn policy(mut self, policy: ProceduralRenderPolicy) -> Self {
        self.policy = policy;
        self
    }

    pub fn order_after(mut self, pass_label: impl Into<String>) -> Self {
        self.non_data_order_after.push(pass_label.into());
        self
    }
}
