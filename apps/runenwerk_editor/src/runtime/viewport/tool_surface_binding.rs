//! File: apps/runenwerk_editor/src/runtime/viewport/tool_surface_binding.rs
//! Purpose: Runtime-owned structural tool-surface -> viewport binding records.

use std::collections::BTreeMap;

use editor_shell::{
    PanelInstanceId, StructuralCommandTarget, StructuralWidgetRoutingContext, TabStackId,
    ToolSurfaceInstanceId, WidgetId,
};
use editor_viewport::ViewportId;
use ui_math::{UiPoint, UiRect};

use crate::runtime::viewport::{ViewportInstanceRegistryResource, ViewportLayoutMapResource};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ToolSurfaceRuntimeBindingRecord {
    pub tool_surface_id: ToolSurfaceInstanceId,
    pub panel_instance_id: PanelInstanceId,
    pub tab_stack_id: TabStackId,
    pub viewport_id: ViewportId,
    pub host_widget_id: WidgetId,
    pub bounds: UiRect,
    pub generation: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ToolSurfaceRuntimeRebind {
    pub tool_surface_id: ToolSurfaceInstanceId,
    pub from_viewport_id: ViewportId,
    pub to_viewport_id: ViewportId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolSurfaceRuntimeBindingResolveError {
    MissingStructuralToolSurface {
        panel_instance_id: PanelInstanceId,
        tab_stack_id: TabStackId,
    },
    MissingRuntimeBinding {
        tool_surface_id: ToolSurfaceInstanceId,
        panel_instance_id: PanelInstanceId,
        tab_stack_id: TabStackId,
    },
    StaleOrReplacedRuntimeBinding {
        tool_surface_id: ToolSurfaceInstanceId,
        requested_viewport_id: ViewportId,
        bound_viewport_id: ViewportId,
    },
    StructuralBindingMismatch {
        tool_surface_id: ToolSurfaceInstanceId,
        expected_panel_instance_id: PanelInstanceId,
        expected_tab_stack_id: TabStackId,
        bound_panel_instance_id: PanelInstanceId,
        bound_tab_stack_id: TabStackId,
    },
}

impl std::fmt::Display for ToolSurfaceRuntimeBindingResolveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingStructuralToolSurface {
                panel_instance_id,
                tab_stack_id,
            } => write!(
                f,
                "missing structural tool-surface target for panel={:?} tab_stack={:?}",
                panel_instance_id, tab_stack_id
            ),
            Self::MissingRuntimeBinding {
                tool_surface_id,
                panel_instance_id,
                tab_stack_id,
            } => write!(
                f,
                "missing runtime binding for tool_surface={:?} panel={:?} tab_stack={:?}",
                tool_surface_id, panel_instance_id, tab_stack_id
            ),
            Self::StaleOrReplacedRuntimeBinding {
                tool_surface_id,
                requested_viewport_id,
                bound_viewport_id,
            } => write!(
                f,
                "stale/replaced runtime binding for tool_surface={:?}: requested viewport={:?}, bound viewport={:?}",
                tool_surface_id, requested_viewport_id, bound_viewport_id
            ),
            Self::StructuralBindingMismatch {
                tool_surface_id,
                expected_panel_instance_id,
                expected_tab_stack_id,
                bound_panel_instance_id,
                bound_tab_stack_id,
            } => write!(
                f,
                "structural binding mismatch for tool_surface={:?}: expected panel={:?} tab_stack={:?}, bound panel={:?} tab_stack={:?}",
                tool_surface_id,
                expected_panel_instance_id,
                expected_tab_stack_id,
                bound_panel_instance_id,
                bound_tab_stack_id
            ),
        }
    }
}

impl std::error::Error for ToolSurfaceRuntimeBindingResolveError {}

#[derive(Debug, Clone, runen_ecs::Component, runen_ecs::Resource, Default)]
pub struct ToolSurfaceRuntimeBindingRegistryResource {
    generation: u64,
    bindings_by_tool_surface: BTreeMap<ToolSurfaceInstanceId, ToolSurfaceRuntimeBindingRecord>,
    latest_rebind_by_tool_surface: BTreeMap<ToolSurfaceInstanceId, ToolSurfaceRuntimeRebind>,
}

impl ToolSurfaceRuntimeBindingRegistryResource {
    pub fn generation(&self) -> u64 {
        self.generation
    }

    pub fn binding_for_tool_surface(
        &self,
        tool_surface_id: ToolSurfaceInstanceId,
    ) -> Option<ToolSurfaceRuntimeBindingRecord> {
        self.bindings_by_tool_surface.get(&tool_surface_id).copied()
    }

    pub fn latest_rebind_for_tool_surface(
        &self,
        tool_surface_id: ToolSurfaceInstanceId,
    ) -> Option<ToolSurfaceRuntimeRebind> {
        self.latest_rebind_by_tool_surface
            .get(&tool_surface_id)
            .copied()
    }

    pub fn latest_rebinds(&self) -> impl Iterator<Item = ToolSurfaceRuntimeRebind> + '_ {
        self.latest_rebind_by_tool_surface.values().copied()
    }

    pub fn bindings(&self) -> impl Iterator<Item = ToolSurfaceRuntimeBindingRecord> + '_ {
        self.bindings_by_tool_surface.values().copied()
    }

    pub fn binding_for_host_widget(
        &self,
        host_widget_id: WidgetId,
    ) -> Option<ToolSurfaceRuntimeBindingRecord> {
        self.bindings_by_tool_surface
            .values()
            .find(|binding| binding.host_widget_id == host_widget_id)
            .copied()
    }

    pub fn binding_containing_cursor(
        &self,
        cursor: UiPoint,
    ) -> Option<ToolSurfaceRuntimeBindingRecord> {
        self.bindings_by_tool_surface
            .values()
            .filter(|binding| binding.bounds.contains(cursor))
            .min_by(|left, right| {
                let left_area = left.bounds.width * left.bounds.height;
                let right_area = right.bounds.width * right.bounds.height;
                left_area.total_cmp(&right_area)
            })
            .copied()
    }

    pub fn upsert_binding(&mut self, mut binding: ToolSurfaceRuntimeBindingRecord) {
        if binding.generation == 0 {
            binding.generation = self.generation.max(1);
        }
        self.bindings_by_tool_surface
            .insert(binding.tool_surface_id, binding);
    }

    pub fn rebuild_from_layout_map(&mut self, layout_map: &ViewportLayoutMapResource) {
        self.rebuild_from_layout_entries(layout_map, |entry| Some(entry.viewport_id));
    }

    pub fn rebuild_from_layout_map_with_instances(
        &mut self,
        layout_map: &ViewportLayoutMapResource,
        viewport_instances: &ViewportInstanceRegistryResource,
    ) {
        self.rebuild_from_layout_entries(layout_map, |entry| {
            entry
                .structural_context
                .active_tool_surface
                .and_then(|tool_surface_id| {
                    viewport_instances.viewport_for_tool_surface(tool_surface_id)
                })
        });
    }

    fn rebuild_from_layout_entries(
        &mut self,
        layout_map: &ViewportLayoutMapResource,
        mut viewport_id_for_entry: impl FnMut(
            &crate::runtime::viewport::ViewportLayoutEntry,
        ) -> Option<ViewportId>,
    ) {
        self.generation = self.generation.saturating_add(1);
        let previous = self.bindings_by_tool_surface.clone();
        self.bindings_by_tool_surface.clear();
        self.latest_rebind_by_tool_surface.clear();

        for entry in layout_map.entries() {
            let Some(tool_surface_id) = entry.structural_context.active_tool_surface else {
                continue;
            };
            let Some(viewport_id) = viewport_id_for_entry(entry) else {
                continue;
            };

            let binding = ToolSurfaceRuntimeBindingRecord {
                tool_surface_id,
                panel_instance_id: entry.structural_context.panel_instance_id,
                tab_stack_id: entry.structural_context.tab_stack_id,
                viewport_id,
                host_widget_id: entry.host_widget_id,
                bounds: entry.bounds,
                generation: self.generation,
            };

            if let Some(previous_binding) = previous.get(&tool_surface_id)
                && previous_binding.viewport_id != binding.viewport_id
            {
                self.latest_rebind_by_tool_surface.insert(
                    tool_surface_id,
                    ToolSurfaceRuntimeRebind {
                        tool_surface_id,
                        from_viewport_id: previous_binding.viewport_id,
                        to_viewport_id: binding.viewport_id,
                    },
                );
            }

            self.bindings_by_tool_surface
                .insert(tool_surface_id, binding);
        }
    }

    pub fn resolve_structural_context(
        &self,
        context: StructuralWidgetRoutingContext,
    ) -> Option<ToolSurfaceRuntimeBindingRecord> {
        let tool_surface_id = context.active_tool_surface?;
        let binding = self.binding_for_tool_surface(tool_surface_id)?;
        if binding.panel_instance_id == context.panel_instance_id
            && binding.tab_stack_id == context.tab_stack_id
        {
            Some(binding)
        } else {
            None
        }
    }

    pub fn resolve_command_target(
        &self,
        target: StructuralCommandTarget,
        requested_viewport_id: ViewportId,
    ) -> Result<ToolSurfaceRuntimeBindingRecord, ToolSurfaceRuntimeBindingResolveError> {
        let Some(tool_surface_id) = target.active_tool_surface else {
            return Err(
                ToolSurfaceRuntimeBindingResolveError::MissingStructuralToolSurface {
                    panel_instance_id: target.panel_instance_id,
                    tab_stack_id: target.tab_stack_id,
                },
            );
        };

        let binding = self.binding_for_tool_surface(tool_surface_id).ok_or(
            ToolSurfaceRuntimeBindingResolveError::MissingRuntimeBinding {
                tool_surface_id,
                panel_instance_id: target.panel_instance_id,
                tab_stack_id: target.tab_stack_id,
            },
        )?;

        if binding.panel_instance_id != target.panel_instance_id
            || binding.tab_stack_id != target.tab_stack_id
        {
            return Err(
                ToolSurfaceRuntimeBindingResolveError::StructuralBindingMismatch {
                    tool_surface_id,
                    expected_panel_instance_id: target.panel_instance_id,
                    expected_tab_stack_id: target.tab_stack_id,
                    bound_panel_instance_id: binding.panel_instance_id,
                    bound_tab_stack_id: binding.tab_stack_id,
                },
            );
        }

        if binding.viewport_id != requested_viewport_id {
            return Err(
                ToolSurfaceRuntimeBindingResolveError::StaleOrReplacedRuntimeBinding {
                    tool_surface_id,
                    requested_viewport_id,
                    bound_viewport_id: binding.viewport_id,
                },
            );
        }

        Ok(binding)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use editor_shell::{PanelInstanceId, TabStackId, ToolSurfaceInstanceId, WidgetId};
    use editor_viewport::ViewportId;
    use ui_math::UiRect;

    use crate::runtime::viewport::{ViewportLayoutEntry, ViewportLayoutMapResource};

    fn seeded_entry(
        viewport_id: ViewportId,
        panel: u64,
        stack: u64,
        surface: Option<u64>,
    ) -> ViewportLayoutEntry {
        seeded_entry_with_widget(viewport_id, panel, stack, surface, 100 + viewport_id.0)
    }

    fn seeded_entry_with_widget(
        viewport_id: ViewportId,
        panel: u64,
        stack: u64,
        surface: Option<u64>,
        widget_id: u64,
    ) -> ViewportLayoutEntry {
        ViewportLayoutEntry {
            viewport_id,
            host_widget_id: WidgetId(widget_id),
            structural_context: StructuralWidgetRoutingContext {
                mounted_unit_id: None,
                panel_instance_id: PanelInstanceId::try_from_raw(panel).unwrap(),
                active_tool_surface: surface
                    .map(|id| ToolSurfaceInstanceId::try_from_raw(id).unwrap()),
                tab_stack_id: TabStackId::try_from_raw(stack).unwrap(),
            },
            bounds: UiRect::new(10.0, 20.0, 300.0, 200.0),
        }
    }

    #[test]
    fn rebuild_registers_tool_surface_bindings_from_layout_map() {
        let mut layout = ViewportLayoutMapResource::default();
        layout.upsert_entry(seeded_entry(ViewportId(1), 11, 21, Some(31)));
        let mut registry = ToolSurfaceRuntimeBindingRegistryResource::default();

        registry.rebuild_from_layout_map(&layout);

        let binding = registry
            .binding_for_tool_surface(ToolSurfaceInstanceId::try_from_raw(31).unwrap())
            .expect("tool surface binding should exist");
        assert_eq!(binding.viewport_id, ViewportId(1));
        assert_eq!(
            binding.panel_instance_id,
            PanelInstanceId::try_from_raw(11).unwrap()
        );
        assert_eq!(binding.tab_stack_id, TabStackId::try_from_raw(21).unwrap());
    }

    #[test]
    fn rebuild_keeps_multiple_structural_surfaces_for_shared_viewport() {
        let mut layout = ViewportLayoutMapResource::default();
        layout.upsert_entry(seeded_entry_with_widget(
            ViewportId(1),
            11,
            21,
            Some(31),
            101,
        ));
        layout.upsert_entry(seeded_entry_with_widget(
            ViewportId(1),
            12,
            22,
            Some(32),
            202,
        ));
        let mut registry = ToolSurfaceRuntimeBindingRegistryResource::default();

        registry.rebuild_from_layout_map(&layout);

        let first = registry
            .binding_for_tool_surface(ToolSurfaceInstanceId::try_from_raw(31).unwrap())
            .expect("first shared-viewport surface should remain bound");
        let second = registry
            .binding_for_tool_surface(ToolSurfaceInstanceId::try_from_raw(32).unwrap())
            .expect("second shared-viewport surface should remain bound");
        assert_eq!(first.viewport_id, ViewportId(1));
        assert_eq!(second.viewport_id, ViewportId(1));
        assert_eq!(first.host_widget_id, WidgetId(101));
        assert_eq!(second.host_widget_id, WidgetId(202));
    }

    #[test]
    fn rebuild_tracks_rebind_when_viewport_changes_for_same_tool_surface() {
        let mut layout = ViewportLayoutMapResource::default();
        layout.upsert_entry(seeded_entry(ViewportId(1), 11, 21, Some(31)));
        let mut registry = ToolSurfaceRuntimeBindingRegistryResource::default();
        registry.rebuild_from_layout_map(&layout);

        layout.clear();
        layout.upsert_entry(seeded_entry(ViewportId(2), 11, 21, Some(31)));
        registry.rebuild_from_layout_map(&layout);

        let rebind = registry
            .latest_rebind_for_tool_surface(ToolSurfaceInstanceId::try_from_raw(31).unwrap())
            .expect("rebind should be tracked");
        assert_eq!(rebind.from_viewport_id, ViewportId(1));
        assert_eq!(rebind.to_viewport_id, ViewportId(2));
    }

    #[test]
    fn resolve_command_target_fails_when_runtime_binding_missing() {
        let registry = ToolSurfaceRuntimeBindingRegistryResource::default();
        let target = StructuralCommandTarget {
            mounted_unit_id: None,
            panel_instance_id: PanelInstanceId::try_from_raw(11).unwrap(),
            active_tool_surface: Some(ToolSurfaceInstanceId::try_from_raw(31).unwrap()),
            tab_stack_id: TabStackId::try_from_raw(21).unwrap(),
        };

        let error = registry
            .resolve_command_target(target, ViewportId(1))
            .expect_err("missing binding should fail");
        assert!(matches!(
            error,
            ToolSurfaceRuntimeBindingResolveError::MissingRuntimeBinding {
                tool_surface_id, ..
            } if tool_surface_id == ToolSurfaceInstanceId::try_from_raw(31).unwrap()
        ));
    }

    #[test]
    fn resolve_command_target_fails_when_requested_viewport_is_stale() {
        let mut registry = ToolSurfaceRuntimeBindingRegistryResource::default();
        registry.upsert_binding(ToolSurfaceRuntimeBindingRecord {
            tool_surface_id: ToolSurfaceInstanceId::try_from_raw(31).unwrap(),
            panel_instance_id: PanelInstanceId::try_from_raw(11).unwrap(),
            tab_stack_id: TabStackId::try_from_raw(21).unwrap(),
            viewport_id: ViewportId(2),
            host_widget_id: WidgetId(42),
            bounds: UiRect::new(0.0, 0.0, 100.0, 50.0),
            generation: 1,
        });
        let target = StructuralCommandTarget {
            mounted_unit_id: None,
            panel_instance_id: PanelInstanceId::try_from_raw(11).unwrap(),
            active_tool_surface: Some(ToolSurfaceInstanceId::try_from_raw(31).unwrap()),
            tab_stack_id: TabStackId::try_from_raw(21).unwrap(),
        };

        let error = registry
            .resolve_command_target(target, ViewportId(1))
            .expect_err("stale viewport request should fail");
        assert!(matches!(
            error,
            ToolSurfaceRuntimeBindingResolveError::StaleOrReplacedRuntimeBinding {
                requested_viewport_id,
                bound_viewport_id,
                ..
            } if requested_viewport_id == ViewportId(1) && bound_viewport_id == ViewportId(2)
        ));
    }

    #[test]
    fn resolve_command_target_fails_when_structural_identity_mismatch() {
        let mut registry = ToolSurfaceRuntimeBindingRegistryResource::default();
        registry.upsert_binding(ToolSurfaceRuntimeBindingRecord {
            tool_surface_id: ToolSurfaceInstanceId::try_from_raw(31).unwrap(),
            panel_instance_id: PanelInstanceId::try_from_raw(22).unwrap(),
            tab_stack_id: TabStackId::try_from_raw(44).unwrap(),
            viewport_id: ViewportId(2),
            host_widget_id: WidgetId(42),
            bounds: UiRect::new(0.0, 0.0, 100.0, 50.0),
            generation: 1,
        });
        let target = StructuralCommandTarget {
            mounted_unit_id: None,
            panel_instance_id: PanelInstanceId::try_from_raw(11).unwrap(),
            active_tool_surface: Some(ToolSurfaceInstanceId::try_from_raw(31).unwrap()),
            tab_stack_id: TabStackId::try_from_raw(21).unwrap(),
        };

        let error = registry
            .resolve_command_target(target, ViewportId(2))
            .expect_err("structural mismatch should fail");
        assert!(matches!(
            error,
            ToolSurfaceRuntimeBindingResolveError::StructuralBindingMismatch {
                expected_panel_instance_id,
                expected_tab_stack_id,
                bound_panel_instance_id,
                bound_tab_stack_id,
                ..
            } if expected_panel_instance_id == PanelInstanceId::try_from_raw(11).unwrap()
                && expected_tab_stack_id == TabStackId::try_from_raw(21).unwrap()
                && bound_panel_instance_id == PanelInstanceId::try_from_raw(22).unwrap()
                && bound_tab_stack_id == TabStackId::try_from_raw(44).unwrap()
        ));
    }

    #[test]
    fn rebuild_updates_widget_host_without_reported_rebind_when_viewport_is_unchanged() {
        let mut layout = ViewportLayoutMapResource::default();
        layout.upsert_entry(seeded_entry_with_widget(
            ViewportId(1),
            11,
            21,
            Some(31),
            101,
        ));
        let mut registry = ToolSurfaceRuntimeBindingRegistryResource::default();
        registry.rebuild_from_layout_map(&layout);

        layout.clear();
        layout.upsert_entry(seeded_entry_with_widget(
            ViewportId(1),
            11,
            21,
            Some(31),
            202,
        ));
        registry.rebuild_from_layout_map(&layout);

        let binding = registry
            .binding_for_tool_surface(ToolSurfaceInstanceId::try_from_raw(31).unwrap())
            .expect("binding should exist");
        assert_eq!(binding.viewport_id, ViewportId(1));
        assert_eq!(binding.host_widget_id, WidgetId(202));
        assert!(
            registry
                .latest_rebind_for_tool_surface(ToolSurfaceInstanceId::try_from_raw(31).unwrap())
                .is_none(),
            "changing widget host without viewport remap must not produce runtime rebind",
        );
    }
}
