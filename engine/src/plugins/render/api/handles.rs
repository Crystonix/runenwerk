use crate::plugins::render::RenderPassId;
use runen_gpu::GpuBufferHandle;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PassHandle {
    id: RenderPassId,
}

impl PassHandle {
    pub const fn new(id: RenderPassId) -> Self {
        Self { id }
    }

    pub fn id(&self) -> &RenderPassId {
        &self.id
    }
}

/// Render-owned relationship between two logical GPU buffers.
///
/// The individual resources retain kind-safe RunenGPU handles; this type owns
/// only the render-specific ping-pong relationship and its authoring label.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RenderDoubleBuffer {
    name: String,
    a: GpuBufferHandle,
    b: GpuBufferHandle,
}

impl RenderDoubleBuffer {
    pub(crate) fn new(name: String, a: GpuBufferHandle, b: GpuBufferHandle) -> Self {
        Self { name, a, b }
    }

    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    pub fn a(&self) -> &GpuBufferHandle {
        &self.a
    }

    pub fn b(&self) -> &GpuBufferHandle {
        &self.b
    }
}
