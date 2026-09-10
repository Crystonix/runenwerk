use crate::runtime::NativeWindowId;
use anyhow::{Result, anyhow};
use id_macros::id;
use runen_gpu::{
    GpuSurfaceCapabilities, GpuSurfaceConfiguration, GpuSurfacePresentMode, GpuTextureFormat,
    GpuTextureUsage,
};
use std::collections::BTreeMap;

#[id]
pub struct RenderSurfaceId;

impl RenderSurfaceId {
    pub fn primary() -> Self {
        Self::try_from_raw(1).expect("primary render surface id must be non-zero")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderSurfaceLifecycleState {
    Registered,
    MissingNativeWindow,
    Retired,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderSurfaceRecord {
    pub render_surface_id: RenderSurfaceId,
    pub native_window_id: NativeWindowId,
    pub target_size_px: (u32, u32),
    pub lifecycle_state: RenderSurfaceLifecycleState,
}

impl RenderSurfaceRecord {
    pub fn new(
        render_surface_id: RenderSurfaceId,
        native_window_id: NativeWindowId,
        target_size_px: (u32, u32),
    ) -> Self {
        Self {
            render_surface_id,
            native_window_id,
            target_size_px: (target_size_px.0.max(1), target_size_px.1.max(1)),
            lifecycle_state: RenderSurfaceLifecycleState::Registered,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderSurfaceDiagnostic {
    pub render_surface_id: Option<RenderSurfaceId>,
    pub native_window_id: Option<NativeWindowId>,
    pub message: String,
}

#[derive(Debug, Clone, Default, runen_ecs::Component, runen_ecs::Resource)]
pub struct RenderSurfaceRegistryResource {
    primary_surface_id: Option<RenderSurfaceId>,
    next_surface_raw: u64,
    records: BTreeMap<RenderSurfaceId, RenderSurfaceRecord>,
    surfaces_by_native_window: BTreeMap<NativeWindowId, RenderSurfaceId>,
    diagnostics: Vec<RenderSurfaceDiagnostic>,
}

impl RenderSurfaceRegistryResource {
    pub fn ensure_surface_for_native_window(
        &mut self,
        native_window_id: NativeWindowId,
        target_size_px: (u32, u32),
    ) -> RenderSurfaceId {
        if let Some(surface_id) = self
            .surfaces_by_native_window
            .get(&native_window_id)
            .copied()
        {
            if let Some(record) = self.records.get_mut(&surface_id) {
                record.target_size_px = (target_size_px.0.max(1), target_size_px.1.max(1));
                record.lifecycle_state = RenderSurfaceLifecycleState::Registered;
            }
            return surface_id;
        }

        let render_surface_id = if native_window_id == NativeWindowId::primary() {
            RenderSurfaceId::primary()
        } else {
            self.allocate_secondary_surface_id()
        };
        self.next_surface_raw = self
            .next_surface_raw
            .max(render_surface_id.raw().saturating_add(1));
        self.records.insert(
            render_surface_id,
            RenderSurfaceRecord::new(render_surface_id, native_window_id, target_size_px),
        );
        self.surfaces_by_native_window
            .insert(native_window_id, render_surface_id);
        if native_window_id == NativeWindowId::primary() {
            self.primary_surface_id = Some(render_surface_id);
        }
        render_surface_id
    }

    pub fn primary_surface_id(&self) -> Option<RenderSurfaceId> {
        self.primary_surface_id
    }

    pub fn record(&self, render_surface_id: RenderSurfaceId) -> Option<&RenderSurfaceRecord> {
        self.records.get(&render_surface_id)
    }

    pub fn surface_for_native_window(
        &self,
        native_window_id: NativeWindowId,
    ) -> Option<RenderSurfaceId> {
        self.surfaces_by_native_window
            .get(&native_window_id)
            .copied()
    }

    pub fn records(&self) -> impl Iterator<Item = &RenderSurfaceRecord> {
        self.records.values()
    }

    pub fn retire_surface_for_native_window(
        &mut self,
        native_window_id: NativeWindowId,
    ) -> Option<RenderSurfaceId> {
        let render_surface_id = self.surfaces_by_native_window.remove(&native_window_id)?;
        if let Some(record) = self.records.get_mut(&render_surface_id) {
            record.lifecycle_state = RenderSurfaceLifecycleState::Retired;
        }
        if self.primary_surface_id == Some(render_surface_id) {
            self.primary_surface_id = None;
        }
        Some(render_surface_id)
    }

    pub fn diagnostics(&self) -> &[RenderSurfaceDiagnostic] {
        &self.diagnostics
    }

    pub fn record_diagnostic(&mut self, diagnostic: RenderSurfaceDiagnostic) {
        self.diagnostics.push(diagnostic);
    }

    pub fn clear_diagnostics(&mut self) {
        self.diagnostics.clear();
    }

    fn allocate_secondary_surface_id(&mut self) -> RenderSurfaceId {
        loop {
            let raw = self
                .next_surface_raw
                .max(RenderSurfaceId::primary().raw().saturating_add(1));
            self.next_surface_raw = raw.saturating_add(1);
            if let Ok(id) = RenderSurfaceId::try_from_raw(raw)
                && !self.records.contains_key(&id)
            {
                return id;
            }
        }
    }
}

pub fn build_surface_config(
    width: u32,
    height: u32,
    format: GpuTextureFormat,
    capabilities: &GpuSurfaceCapabilities,
) -> Result<GpuSurfaceConfiguration> {
    let alpha_mode = capabilities
        .alpha_modes()
        .first()
        .copied()
        .ok_or_else(|| anyhow!("render surface reports no supported alpha mode"))?;
    let mut usages = vec![GpuTextureUsage::ColorAttachment];
    for usage in [
        GpuTextureUsage::CopySource,
        GpuTextureUsage::CopyDestination,
    ] {
        if capabilities.supports_usage(usage) {
            usages.push(usage);
        }
    }
    if !capabilities.supports_present_mode(GpuSurfacePresentMode::Fifo) {
        return Err(anyhow!("render surface does not support FIFO presentation"));
    }
    if !capabilities.supports_alpha_mode(alpha_mode) {
        return Err(anyhow!(
            "render surface alpha-mode selection became invalid"
        ));
    }
    Ok(GpuSurfaceConfiguration::new(
        width.max(1),
        height.max(1),
        format,
        usages,
        GpuSurfacePresentMode::Fifo,
        alpha_mode,
        2,
        [format],
    )?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_surface_registry_binds_primary_surface_to_primary_native_window() {
        let mut registry = RenderSurfaceRegistryResource::default();

        let surface_id =
            registry.ensure_surface_for_native_window(NativeWindowId::primary(), (1280, 720));

        assert_eq!(surface_id, RenderSurfaceId::primary());
        assert_eq!(
            registry.primary_surface_id(),
            Some(RenderSurfaceId::primary())
        );
        assert_eq!(
            registry
                .record(surface_id)
                .map(|record| (record.native_window_id, record.target_size_px)),
            Some((NativeWindowId::primary(), (1280, 720)))
        );
    }

    #[test]
    fn render_surface_registry_allocates_distinct_surfaces_per_native_window() {
        let mut registry = RenderSurfaceRegistryResource::default();
        let primary =
            registry.ensure_surface_for_native_window(NativeWindowId::primary(), (1280, 720));
        let secondary_window =
            NativeWindowId::try_from_raw(2).expect("test native window id should be non-zero");

        let secondary = registry.ensure_surface_for_native_window(secondary_window, (640, 480));

        assert_ne!(primary, secondary);
        assert_eq!(
            registry.surface_for_native_window(secondary_window),
            Some(secondary)
        );
        assert_eq!(
            registry
                .record(secondary)
                .map(|record| record.target_size_px),
            Some((640, 480))
        );
    }

    #[test]
    fn render_surface_registry_reserves_primary_id_when_secondary_is_registered_first() {
        let mut registry = RenderSurfaceRegistryResource::default();
        let secondary_window =
            NativeWindowId::try_from_raw(2).expect("test native window id should be non-zero");

        let secondary = registry.ensure_surface_for_native_window(secondary_window, (640, 480));

        assert_ne!(secondary, RenderSurfaceId::primary());
        assert_eq!(registry.primary_surface_id(), None);
        assert_eq!(
            registry.surface_for_native_window(secondary_window),
            Some(secondary)
        );

        let primary =
            registry.ensure_surface_for_native_window(NativeWindowId::primary(), (1280, 720));

        assert_eq!(primary, RenderSurfaceId::primary());
        assert_eq!(
            registry.primary_surface_id(),
            Some(RenderSurfaceId::primary())
        );
        assert_eq!(
            registry.surface_for_native_window(secondary_window),
            Some(secondary)
        );
        assert_eq!(
            registry
                .record(secondary)
                .map(|record| record.native_window_id),
            Some(secondary_window)
        );
    }

    #[test]
    fn retiring_surface_removes_window_lookup_and_preserves_auditable_record() {
        let mut registry = RenderSurfaceRegistryResource::default();
        let secondary_window = NativeWindowId::try_from_raw(2).expect("secondary window id");
        let surface = registry.ensure_surface_for_native_window(secondary_window, (640, 480));

        assert_eq!(
            registry.retire_surface_for_native_window(secondary_window),
            Some(surface)
        );
        assert_eq!(registry.surface_for_native_window(secondary_window), None);
        assert_eq!(
            registry
                .record(surface)
                .map(|record| record.lifecycle_state),
            Some(RenderSurfaceLifecycleState::Retired)
        );
    }
}
