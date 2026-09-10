use std::collections::BTreeMap;

use editor_viewport::ViewportId;

/// File: apps/runenwerk_editor/src/runtime/viewport/surface_set.rs
/// Purpose: Explicit per-viewport-owned presentation surface bundles.
///
/// Governing rule:
/// Viewports consume typed expression products, resolve them through
/// viewport-scoped presentation state, and embed viewport-owned
/// presentation surfaces into the shell.
pub use editor_viewport::ViewportSurfaceSlot;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewportSurfaceHandleSource {
    DynamicTexture,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ViewportSurfaceHandle {
    pub source: ViewportSurfaceHandleSource,
    pub namespace: String,
    pub target_id: String,
    pub ui_sampleable: bool,
}

impl ViewportSurfaceHandle {
    pub fn dynamic_texture(
        namespace: impl Into<String>,
        target_id: impl Into<String>,
        ui_sampleable: bool,
    ) -> Self {
        Self {
            source: ViewportSurfaceHandleSource::DynamicTexture,
            namespace: namespace.into(),
            target_id: target_id.into(),
            ui_sampleable,
        }
    }

    pub fn is_ui_sampleable(&self) -> bool {
        self.ui_sampleable
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ViewportSurfaceSet {
    viewport_id: ViewportId,
    slots: BTreeMap<ViewportSurfaceSlot, ViewportSurfaceHandle>,
}

impl ViewportSurfaceSet {
    pub fn new(viewport_id: ViewportId) -> Self {
        Self {
            viewport_id,
            slots: BTreeMap::new(),
        }
    }

    pub fn viewport_id(&self) -> ViewportId {
        self.viewport_id
    }

    pub fn insert(
        &mut self,
        slot: ViewportSurfaceSlot,
        handle: ViewportSurfaceHandle,
    ) -> Option<ViewportSurfaceHandle> {
        self.slots.insert(slot, handle)
    }

    pub fn get(&self, slot: ViewportSurfaceSlot) -> Option<ViewportSurfaceHandle> {
        self.slots.get(&slot).cloned()
    }

    pub fn contains(&self, slot: ViewportSurfaceSlot) -> bool {
        self.slots.contains_key(&slot)
    }

    pub fn clear(&mut self) {
        self.slots.clear();
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, runen_ecs::Component, runen_ecs::Resource)]
pub struct ViewportSurfaceSetResource {
    sets: BTreeMap<ViewportId, ViewportSurfaceSet>,
}

impl ViewportSurfaceSetResource {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn ensure_viewport(&mut self, viewport_id: ViewportId) -> &mut ViewportSurfaceSet {
        self.sets
            .entry(viewport_id)
            .or_insert_with(|| ViewportSurfaceSet::new(viewport_id))
    }

    pub fn set_surface(
        &mut self,
        viewport_id: ViewportId,
        slot: ViewportSurfaceSlot,
        handle: ViewportSurfaceHandle,
    ) -> Option<ViewportSurfaceHandle> {
        self.ensure_viewport(viewport_id).insert(slot, handle)
    }

    pub fn surface(
        &self,
        viewport_id: ViewportId,
        slot: ViewportSurfaceSlot,
    ) -> Option<ViewportSurfaceHandle> {
        self.sets.get(&viewport_id).and_then(|set| set.get(slot))
    }

    pub fn surface_set(&self, viewport_id: ViewportId) -> Option<&ViewportSurfaceSet> {
        self.sets.get(&viewport_id)
    }

    pub fn viewport_ids(&self) -> impl Iterator<Item = ViewportId> + '_ {
        self.sets.keys().copied()
    }

    pub fn surface_sets(&self) -> impl Iterator<Item = &ViewportSurfaceSet> {
        self.sets.values()
    }

    pub fn retain_viewports(&mut self, mut keep: impl FnMut(ViewportId) -> bool) {
        self.sets.retain(|viewport_id, _| keep(*viewport_id));
    }

    pub fn clear_viewport_surfaces(&mut self, viewport_id: ViewportId) {
        if let Some(set) = self.sets.get_mut(&viewport_id) {
            set.clear();
        }
    }
}

pub fn ensure_editor_main_surface_set(
    viewport_surface_sets: &mut ViewportSurfaceSetResource,
    viewport_id: ViewportId,
) {
    viewport_surface_sets.ensure_viewport(viewport_id);
}
