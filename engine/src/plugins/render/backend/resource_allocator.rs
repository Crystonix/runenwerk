use crate::plugins::render::RenderPassId;
use runen_gpu::GpuWorkResourceId;
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextureResourceEntry {
    pub id: GpuWorkResourceId,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BufferResourceEntry {
    pub id: GpuWorkResourceId,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransientResourceClaim {
    pub id: GpuWorkResourceId,
    pub owner_pass: RenderPassId,
}

#[derive(Debug, Clone, Default, ecs::Component, ecs::Resource)]
pub struct BackendResourceAllocatorResource {
    textures: BTreeMap<GpuWorkResourceId, TextureResourceEntry>,
    buffers: BTreeMap<GpuWorkResourceId, BufferResourceEntry>,
    transients: BTreeMap<GpuWorkResourceId, TransientResourceClaim>,
}

impl BackendResourceAllocatorResource {
    pub fn upsert_texture(&mut self, id: GpuWorkResourceId, label: impl Into<String>) {
        self.textures.insert(
            id,
            TextureResourceEntry {
                id,
                label: label.into(),
            },
        );
    }

    pub fn upsert_buffer(&mut self, id: GpuWorkResourceId, label: impl Into<String>) {
        self.buffers.insert(
            id,
            BufferResourceEntry {
                id,
                label: label.into(),
            },
        );
    }

    pub fn claim_transient(&mut self, id: GpuWorkResourceId, owner_pass: RenderPassId) {
        self.transients
            .insert(id, TransientResourceClaim { id, owner_pass });
    }

    pub fn release_transient(&mut self, id: GpuWorkResourceId) -> bool {
        self.transients.remove(&id).is_some()
    }

    pub fn remove_texture(&mut self, id: GpuWorkResourceId) -> bool {
        self.textures.remove(&id).is_some()
    }

    pub fn remove_buffer(&mut self, id: GpuWorkResourceId) -> bool {
        self.buffers.remove(&id).is_some()
    }

    pub fn texture_entry(&self, id: GpuWorkResourceId) -> Option<&TextureResourceEntry> {
        self.textures.get(&id)
    }

    pub fn buffer_entry(&self, id: GpuWorkResourceId) -> Option<&BufferResourceEntry> {
        self.buffers.get(&id)
    }

    pub fn transient_claim(&self, id: GpuWorkResourceId) -> Option<&TransientResourceClaim> {
        self.transients.get(&id)
    }

    pub fn texture_entries(&self) -> Vec<TextureResourceEntry> {
        self.textures.values().cloned().collect()
    }

    pub fn buffer_entries(&self) -> Vec<BufferResourceEntry> {
        self.buffers.values().cloned().collect()
    }

    pub fn transient_claims(&self) -> Vec<TransientResourceClaim> {
        self.transients.values().cloned().collect()
    }
}
