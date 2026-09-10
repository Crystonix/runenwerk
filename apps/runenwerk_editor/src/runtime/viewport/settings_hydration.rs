//! File: apps/runenwerk_editor/src/runtime/viewport/settings_hydration.rs
//! Purpose: Track one-shot hydration of persisted viewport runtime settings.

use editor_shell::ToolSurfaceInstanceId;
use editor_viewport::{ExpressionProductId, ViewportFieldVisualizerSettings, ViewportId};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ViewportRuntimeSettingsHydrationKey {
    pub tool_surface_id: ToolSurfaceInstanceId,
    pub viewport_id: ViewportId,
    pub selected_primary_product_id: Option<ExpressionProductId>,
    pub field_visualizer_settings: ViewportFieldVisualizerSettings,
}

impl ViewportRuntimeSettingsHydrationKey {
    pub const fn new(
        tool_surface_id: ToolSurfaceInstanceId,
        viewport_id: ViewportId,
        selected_primary_product_id: Option<ExpressionProductId>,
        field_visualizer_settings: ViewportFieldVisualizerSettings,
    ) -> Self {
        Self {
            tool_surface_id,
            viewport_id,
            selected_primary_product_id,
            field_visualizer_settings,
        }
    }
}

#[derive(Debug, Clone, Default, runen_ecs::Component, runen_ecs::Resource)]
pub struct ViewportRuntimeSettingsHydrationResource {
    restored_keys: Vec<ViewportRuntimeSettingsHydrationKey>,
}

impl ViewportRuntimeSettingsHydrationResource {
    pub fn should_hydrate(&mut self, key: ViewportRuntimeSettingsHydrationKey) -> bool {
        if self.restored_keys.contains(&key) {
            return false;
        }
        self.restored_keys.push(key);
        true
    }

    pub fn retain_viewports(&mut self, mut keep: impl FnMut(ViewportId) -> bool) {
        self.restored_keys.retain(|key| keep(key.viewport_id));
    }

    pub fn is_empty(&self) -> bool {
        self.restored_keys.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hydration_resource_tracks_keys_once_per_viewport_settings_identity() {
        let surface_id = ToolSurfaceInstanceId::try_from_raw(1).unwrap();
        let key = ViewportRuntimeSettingsHydrationKey::new(
            surface_id,
            ViewportId(2),
            Some(ExpressionProductId(6)),
            ViewportFieldVisualizerSettings::default(),
        );
        let mut hydration = ViewportRuntimeSettingsHydrationResource::default();

        assert!(hydration.should_hydrate(key));
        assert!(!hydration.should_hydrate(key));
    }

    #[test]
    fn hydration_resource_prunes_removed_viewports() {
        let surface_id = ToolSurfaceInstanceId::try_from_raw(1).unwrap();
        let mut hydration = ViewportRuntimeSettingsHydrationResource::default();
        hydration.should_hydrate(ViewportRuntimeSettingsHydrationKey::new(
            surface_id,
            ViewportId(2),
            None,
            ViewportFieldVisualizerSettings::default(),
        ));

        hydration.retain_viewports(|viewport_id| viewport_id != ViewportId(2));

        assert!(hydration.is_empty());
    }
}
