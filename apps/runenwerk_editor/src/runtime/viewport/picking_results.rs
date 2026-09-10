//! File: apps/runenwerk_editor/src/runtime/viewport/picking_results.rs
//! Purpose: Canonical viewport-keyed picking backend ownership for editor runtime.

use std::collections::BTreeMap;

use editor_viewport::ViewportId;
use engine::plugins::render::EditorPickingHit;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ViewportPickingResult {
    pub cursor_px: (f32, f32),
    pub viewport_bounds_px: (f32, f32, f32, f32),
    pub hit: EditorPickingHit,
    pub revision: u64,
}

impl Default for ViewportPickingResult {
    fn default() -> Self {
        Self {
            cursor_px: (0.0, 0.0),
            viewport_bounds_px: (0.0, 0.0, 0.0, 0.0),
            hit: EditorPickingHit::none(),
            revision: 0,
        }
    }
}

#[derive(Debug, Clone, runen_ecs::Component, runen_ecs::Resource, Default)]
pub struct ViewportPickingResultsResource {
    results_by_viewport: BTreeMap<ViewportId, ViewportPickingResult>,
    global_revision: u64,
}

impl ViewportPickingResultsResource {
    pub fn result_for(&self, viewport_id: ViewportId) -> Option<&ViewportPickingResult> {
        self.results_by_viewport.get(&viewport_id)
    }

    pub fn set_viewport_result(
        &mut self,
        viewport_id: ViewportId,
        cursor_px: (f32, f32),
        viewport_bounds_px: (f32, f32, f32, f32),
        hit: EditorPickingHit,
    ) {
        self.global_revision = self.global_revision.saturating_add(1);
        self.results_by_viewport.insert(
            viewport_id,
            ViewportPickingResult {
                cursor_px,
                viewport_bounds_px,
                hit,
                revision: self.global_revision,
            },
        );
    }

    pub fn clear_viewport_hit(
        &mut self,
        viewport_id: ViewportId,
        cursor_px: (f32, f32),
        viewport_bounds_px: (f32, f32, f32, f32),
    ) {
        self.set_viewport_result(
            viewport_id,
            cursor_px,
            viewport_bounds_px,
            EditorPickingHit::none(),
        );
    }

    pub fn clear_all_hits(&mut self, cursor_px: (f32, f32)) {
        let viewport_ids = self.results_by_viewport.keys().copied().collect::<Vec<_>>();
        for viewport_id in viewport_ids {
            let viewport_bounds_px = self
                .results_by_viewport
                .get(&viewport_id)
                .map(|value| value.viewport_bounds_px)
                .unwrap_or((0.0, 0.0, 0.0, 0.0));
            self.clear_viewport_hit(viewport_id, cursor_px, viewport_bounds_px);
        }
    }

    pub fn viewport_ids(&self) -> impl Iterator<Item = ViewportId> + '_ {
        self.results_by_viewport.keys().copied()
    }

    pub fn retain_viewports(&mut self, mut keep: impl FnMut(ViewportId) -> bool) {
        self.results_by_viewport
            .retain(|viewport_id, _| keep(*viewport_id));
    }

    pub fn global_revision(&self) -> u64 {
        self.global_revision
    }

    pub fn is_empty(&self) -> bool {
        self.results_by_viewport.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use engine::plugins::render::EditorPickingTarget;

    #[test]
    fn defaults_to_empty_results_before_bootstrap() {
        assert!(ViewportPickingResultsResource::default().is_empty());
    }

    #[test]
    fn viewport_results_are_independent() {
        let mut resource = ViewportPickingResultsResource::default();
        let first = ViewportId(1);
        let second = ViewportId(2);
        resource.set_viewport_result(
            first,
            (10.0, 20.0),
            (0.0, 0.0, 100.0, 100.0),
            EditorPickingHit {
                target: EditorPickingTarget::Entity(7),
                distance: 1.0,
            },
        );
        resource.set_viewport_result(
            second,
            (30.0, 40.0),
            (100.0, 0.0, 100.0, 100.0),
            EditorPickingHit {
                target: EditorPickingTarget::Grid,
                distance: 2.0,
            },
        );

        assert_eq!(
            resource.result_for(first).map(|value| value.hit.target),
            Some(EditorPickingTarget::Entity(7))
        );
        assert_eq!(
            resource.result_for(second).map(|value| value.hit.target),
            Some(EditorPickingTarget::Grid)
        );
    }
}
