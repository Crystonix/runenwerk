//! File: apps/runenwerk_editor/src/runtime/viewport/layout_map.rs
//! Purpose: Runtime viewport host widget -> viewport-id layout mapping.

use std::collections::BTreeMap;

use editor_shell::{StructuralWidgetRoutingContext, WidgetId};
use editor_viewport::ViewportId;
use ui_math::UiRect;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ViewportLayoutEntry {
    pub viewport_id: ViewportId,
    pub host_widget_id: WidgetId,
    pub structural_context: StructuralWidgetRoutingContext,
    pub bounds: UiRect,
}

#[derive(Debug, Clone, runen_ecs::Component, runen_ecs::Resource, Default)]
pub struct ViewportLayoutMapResource {
    entries_by_viewport: BTreeMap<ViewportId, ViewportLayoutEntry>,
    entries_by_structural_context: BTreeMap<StructuralWidgetRoutingContext, ViewportLayoutEntry>,
    viewport_by_structural_context: BTreeMap<StructuralWidgetRoutingContext, ViewportId>,
}

impl ViewportLayoutMapResource {
    pub fn clear(&mut self) {
        self.entries_by_viewport.clear();
        self.entries_by_structural_context.clear();
        self.viewport_by_structural_context.clear();
    }

    pub fn upsert_entry(&mut self, entry: ViewportLayoutEntry) {
        if let Some(previous) = self
            .entries_by_structural_context
            .get(&entry.structural_context)
            && previous.viewport_id != entry.viewport_id
            && self
                .entries_by_viewport
                .get(&previous.viewport_id)
                .is_some_and(|current| current.structural_context == previous.structural_context)
        {
            self.entries_by_viewport.remove(&previous.viewport_id);
        }
        self.viewport_by_structural_context
            .insert(entry.structural_context, entry.viewport_id);
        self.entries_by_structural_context
            .insert(entry.structural_context, entry);
        self.entries_by_viewport.insert(entry.viewport_id, entry);
    }

    pub fn entry_for_viewport(&self, viewport_id: ViewportId) -> Option<&ViewportLayoutEntry> {
        self.entries_by_viewport.get(&viewport_id)
    }

    pub fn viewport_for_structural_context(
        &self,
        structural_context: StructuralWidgetRoutingContext,
    ) -> Option<ViewportId> {
        self.viewport_by_structural_context
            .get(&structural_context)
            .copied()
    }

    pub fn entries(&self) -> impl Iterator<Item = &ViewportLayoutEntry> {
        self.entries_by_structural_context.values()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use editor_shell::{PanelInstanceId, TabStackId, ToolSurfaceInstanceId};

    fn structural_context(panel: u64, stack: u64, surface: u64) -> StructuralWidgetRoutingContext {
        StructuralWidgetRoutingContext {
            mounted_unit_id: None,
            panel_instance_id: PanelInstanceId::try_from_raw(panel).unwrap(),
            active_tool_surface: Some(ToolSurfaceInstanceId::try_from_raw(surface).unwrap()),
            tab_stack_id: TabStackId::try_from_raw(stack).unwrap(),
        }
    }

    #[test]
    fn layout_entries_are_authoritative_per_structural_context() {
        let mut layout = ViewportLayoutMapResource::default();
        let first_context = structural_context(1, 10, 100);
        let second_context = structural_context(2, 20, 200);

        layout.upsert_entry(ViewportLayoutEntry {
            viewport_id: ViewportId(7),
            host_widget_id: WidgetId(101),
            structural_context: first_context,
            bounds: UiRect::new(0.0, 0.0, 100.0, 100.0),
        });
        layout.upsert_entry(ViewportLayoutEntry {
            viewport_id: ViewportId(7),
            host_widget_id: WidgetId(202),
            structural_context: second_context,
            bounds: UiRect::new(100.0, 0.0, 100.0, 100.0),
        });

        assert_eq!(layout.entries().count(), 2);
        assert_eq!(
            layout.viewport_for_structural_context(first_context),
            Some(ViewportId(7))
        );
        assert_eq!(
            layout.viewport_for_structural_context(second_context),
            Some(ViewportId(7))
        );
    }
}
