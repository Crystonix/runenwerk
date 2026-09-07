use runen_gpu::{GpuResourceAccessIntent, GpuWorkResourceId};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceUsageKind {
    Sampled,
    Storage,
    ColorTarget,
    DepthTarget,
    Vertex,
    Index,
    Instance,
    Indirect,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceUsage {
    pub resource_id: GpuWorkResourceId,
    pub access: GpuResourceAccessIntent,
    pub kind: ResourceUsageKind,
}

impl ResourceUsage {
    pub fn new(
        resource_id: impl Into<GpuWorkResourceId>,
        access: GpuResourceAccessIntent,
        kind: ResourceUsageKind,
    ) -> Self {
        Self {
            resource_id: resource_id.into(),
            access,
            kind,
        }
    }
}
