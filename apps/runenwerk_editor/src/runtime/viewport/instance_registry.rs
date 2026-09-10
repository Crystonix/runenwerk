//! File: apps/runenwerk_editor/src/runtime/viewport/instance_registry.rs
//! Purpose: Explicit runtime viewport instance ownership.

use std::collections::{BTreeMap, BTreeSet};

use editor_shell::{EditorCompositionRuntime, PanelInstanceId, ToolSurfaceInstanceId};
use editor_viewport::ViewportId;
use ui_composition::MountedUnitId;

use crate::runtime::viewport::MAIN_VIEWPORT_ID;
use crate::shell::tool_suites::SCENE_VIEWPORT_SURFACE_KEY;

const FIRST_ALLOCATED_VIEWPORT_ID: u64 = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ViewportInstanceRecord {
    pub mounted_unit_id: MountedUnitId,
    pub viewport_id: ViewportId,
    pub tool_surface_id: ToolSurfaceInstanceId,
    pub panel_instance_id: PanelInstanceId,
}

#[derive(Debug, Clone, runen_ecs::Component, runen_ecs::Resource)]
pub struct ViewportInstanceRegistryResource {
    next_viewport_id: u64,
    records_by_mounted_unit: BTreeMap<MountedUnitId, ViewportInstanceRecord>,
    mounted_unit_by_tool_surface: BTreeMap<ToolSurfaceInstanceId, MountedUnitId>,
    mounted_unit_by_viewport: BTreeMap<ViewportId, MountedUnitId>,
}

impl Default for ViewportInstanceRegistryResource {
    fn default() -> Self {
        Self {
            next_viewport_id: FIRST_ALLOCATED_VIEWPORT_ID,
            records_by_mounted_unit: BTreeMap::new(),
            mounted_unit_by_tool_surface: BTreeMap::new(),
            mounted_unit_by_viewport: BTreeMap::new(),
        }
    }
}

impl ViewportInstanceRegistryResource {
    pub fn sync_from_composition(&mut self, runtime: &EditorCompositionRuntime) {
        let mut active_mounted_units = BTreeSet::new();
        for extension in runtime.extension().mounted_units() {
            if extension.stable_content_key != SCENE_VIEWPORT_SURFACE_KEY {
                continue;
            }
            let Ok(tool_surface_id) =
                ToolSurfaceInstanceId::try_from_raw(extension.compatibility_surface_raw)
            else {
                continue;
            };
            let Ok(panel_instance_id) = PanelInstanceId::try_from_raw(extension.panel_instance_raw)
            else {
                continue;
            };
            active_mounted_units.insert(extension.mounted_unit_id);
            if let Some(viewport_raw) = extension.viewport_instance_raw {
                self.restore_mounted_instance(
                    extension.mounted_unit_id,
                    tool_surface_id,
                    panel_instance_id,
                    ViewportId(viewport_raw),
                );
            } else {
                self.ensure_for_mounted_unit(
                    extension.mounted_unit_id,
                    tool_surface_id,
                    panel_instance_id,
                );
            }
        }

        self.records_by_mounted_unit
            .retain(|mounted_unit_id, record| {
                let keep = active_mounted_units.contains(mounted_unit_id);
                if !keep {
                    self.mounted_unit_by_viewport.remove(&record.viewport_id);
                    self.mounted_unit_by_tool_surface
                        .remove(&record.tool_surface_id);
                }
                keep
            });
    }

    pub fn ensure_for_mounted_unit(
        &mut self,
        mounted_unit_id: MountedUnitId,
        tool_surface_id: ToolSurfaceInstanceId,
        panel_instance_id: PanelInstanceId,
    ) -> ViewportInstanceRecord {
        if let Some(record) = self.records_by_mounted_unit.get_mut(&mounted_unit_id) {
            record.panel_instance_id = panel_instance_id;
            record.tool_surface_id = tool_surface_id;
            return *record;
        }

        let viewport_id = self.allocate_viewport_id();
        let record = ViewportInstanceRecord {
            mounted_unit_id,
            viewport_id,
            tool_surface_id,
            panel_instance_id,
        };
        self.records_by_mounted_unit.insert(mounted_unit_id, record);
        self.mounted_unit_by_tool_surface
            .insert(tool_surface_id, mounted_unit_id);
        self.mounted_unit_by_viewport
            .insert(viewport_id, mounted_unit_id);
        record
    }

    pub fn restore_mounted_instance(
        &mut self,
        mounted_unit_id: MountedUnitId,
        tool_surface_id: ToolSurfaceInstanceId,
        panel_instance_id: PanelInstanceId,
        viewport_id: ViewportId,
    ) -> ViewportInstanceRecord {
        if let Some(record) = self.records_by_mounted_unit.get_mut(&mounted_unit_id)
            && record.viewport_id == viewport_id
        {
            record.panel_instance_id = panel_instance_id;
            self.next_viewport_id = self.next_viewport_id.max(viewport_id.0.saturating_add(1));
            return *record;
        }
        if let Some(previous_mounted_unit) = self.mounted_unit_by_viewport.get(&viewport_id)
            && *previous_mounted_unit != mounted_unit_id
            && let Some(previous) = self.records_by_mounted_unit.remove(previous_mounted_unit)
        {
            self.mounted_unit_by_tool_surface
                .remove(&previous.tool_surface_id);
        }
        if let Some(previous_record) = self.records_by_mounted_unit.get(&mounted_unit_id) {
            self.mounted_unit_by_viewport
                .remove(&previous_record.viewport_id);
            self.mounted_unit_by_tool_surface
                .remove(&previous_record.tool_surface_id);
        }

        self.next_viewport_id = self.next_viewport_id.max(viewport_id.0.saturating_add(1));
        let record = ViewportInstanceRecord {
            mounted_unit_id,
            viewport_id,
            tool_surface_id,
            panel_instance_id,
        };
        self.records_by_mounted_unit.insert(mounted_unit_id, record);
        self.mounted_unit_by_tool_surface
            .insert(tool_surface_id, mounted_unit_id);
        self.mounted_unit_by_viewport
            .insert(viewport_id, mounted_unit_id);
        record
    }

    pub fn duplicate_mounted_instance(
        &mut self,
        source_mounted_unit_id: MountedUnitId,
        target_mounted_unit_id: MountedUnitId,
        target_tool_surface_id: ToolSurfaceInstanceId,
        target_panel_instance_id: PanelInstanceId,
    ) -> Option<ViewportInstanceRecord> {
        self.records_by_mounted_unit.get(&source_mounted_unit_id)?;
        Some(self.ensure_for_mounted_unit(
            target_mounted_unit_id,
            target_tool_surface_id,
            target_panel_instance_id,
        ))
    }

    pub fn close_mounted_instance(
        &mut self,
        mounted_unit_id: MountedUnitId,
    ) -> Option<ViewportInstanceRecord> {
        let record = self.records_by_mounted_unit.remove(&mounted_unit_id)?;
        self.mounted_unit_by_viewport.remove(&record.viewport_id);
        self.mounted_unit_by_tool_surface
            .remove(&record.tool_surface_id);
        Some(record)
    }

    pub fn viewport_for_tool_surface(
        &self,
        tool_surface_id: ToolSurfaceInstanceId,
    ) -> Option<ViewportId> {
        let mounted_unit_id = self.mounted_unit_by_tool_surface.get(&tool_surface_id)?;
        self.records_by_mounted_unit
            .get(mounted_unit_id)
            .map(|record| record.viewport_id)
    }

    pub fn record_for_viewport(&self, viewport_id: ViewportId) -> Option<ViewportInstanceRecord> {
        let mounted_unit_id = self.mounted_unit_by_viewport.get(&viewport_id)?;
        self.records_by_mounted_unit.get(mounted_unit_id).copied()
    }

    pub fn records(&self) -> impl Iterator<Item = ViewportInstanceRecord> + '_ {
        self.records_by_mounted_unit.values().copied()
    }

    pub fn is_empty(&self) -> bool {
        self.records_by_mounted_unit.is_empty()
    }

    #[cfg(test)]
    pub fn ensure_for_tool_surface(
        &mut self,
        tool_surface_id: ToolSurfaceInstanceId,
        panel_instance_id: PanelInstanceId,
    ) -> ViewportInstanceRecord {
        self.ensure_for_mounted_unit(
            MountedUnitId::try_from_raw(tool_surface_id.raw()).unwrap(),
            tool_surface_id,
            panel_instance_id,
        )
    }

    #[cfg(test)]
    pub fn restore_instance(
        &mut self,
        tool_surface_id: ToolSurfaceInstanceId,
        panel_instance_id: PanelInstanceId,
        viewport_id: ViewportId,
    ) -> ViewportInstanceRecord {
        self.restore_mounted_instance(
            MountedUnitId::try_from_raw(tool_surface_id.raw()).unwrap(),
            tool_surface_id,
            panel_instance_id,
            viewport_id,
        )
    }

    #[cfg(test)]
    pub fn duplicate_instance(
        &mut self,
        source_tool_surface_id: ToolSurfaceInstanceId,
        target_tool_surface_id: ToolSurfaceInstanceId,
        target_panel_instance_id: PanelInstanceId,
    ) -> Option<ViewportInstanceRecord> {
        self.duplicate_mounted_instance(
            MountedUnitId::try_from_raw(source_tool_surface_id.raw()).unwrap(),
            MountedUnitId::try_from_raw(target_tool_surface_id.raw()).unwrap(),
            target_tool_surface_id,
            target_panel_instance_id,
        )
    }

    #[cfg(test)]
    pub fn close_instance(
        &mut self,
        tool_surface_id: ToolSurfaceInstanceId,
    ) -> Option<ViewportInstanceRecord> {
        let mounted_unit_id = self
            .mounted_unit_by_tool_surface
            .get(&tool_surface_id)
            .copied()?;
        self.close_mounted_instance(mounted_unit_id)
    }

    #[cfg(test)]
    pub fn sync_from_workspace_state(&mut self, workspace: &editor_shell::WorkspaceState) {
        let profile_id = editor_shell::SCENE_WORKSPACE_PROFILE_ID;
        let runtime = editor_shell::import_legacy_workspace(profile_id, workspace).unwrap();
        self.sync_from_composition(&runtime);
    }

    fn allocate_viewport_id(&mut self) -> ViewportId {
        let mut viewport_id = ViewportId(self.next_viewport_id.max(FIRST_ALLOCATED_VIEWPORT_ID));
        while viewport_id == MAIN_VIEWPORT_ID
            || self.mounted_unit_by_viewport.contains_key(&viewport_id)
        {
            viewport_id = ViewportId(viewport_id.0.saturating_add(1));
        }
        self.next_viewport_id = viewport_id.0.saturating_add(1);
        viewport_id
    }
}

#[cfg(test)]
pub(crate) fn is_viewport_tool_surface(surface: &editor_shell::ToolSurfaceState) -> bool {
    surface.stable_surface_key().as_str() == SCENE_VIEWPORT_SURFACE_KEY
}

#[cfg(test)]
mod tests {
    use super::*;
    use editor_shell::{
        WorkspaceId, WorkspaceIdentityAllocator, WorkspaceMutation, WorkspaceState,
        reduce_workspace,
    };

    fn ids() -> (
        ToolSurfaceInstanceId,
        ToolSurfaceInstanceId,
        PanelInstanceId,
        PanelInstanceId,
    ) {
        (
            ToolSurfaceInstanceId::try_from_raw(11).unwrap(),
            ToolSurfaceInstanceId::try_from_raw(12).unwrap(),
            PanelInstanceId::try_from_raw(21).unwrap(),
            PanelInstanceId::try_from_raw(22).unwrap(),
        )
    }

    #[test]
    fn allocates_stable_viewport_identity_per_tool_surface() {
        let (surface_a, surface_b, panel_a, panel_b) = ids();
        let mut registry = ViewportInstanceRegistryResource::default();

        let first = registry.ensure_for_tool_surface(surface_a, panel_a);
        let again = registry.ensure_for_tool_surface(surface_a, panel_b);
        let second = registry.ensure_for_tool_surface(surface_b, panel_b);

        assert_eq!(first.viewport_id, again.viewport_id);
        assert_eq!(again.panel_instance_id, panel_b);
        assert_ne!(first.viewport_id, second.viewport_id);
        assert_ne!(first.viewport_id, MAIN_VIEWPORT_ID);
    }

    #[test]
    fn restore_reuses_supplied_viewport_identity() {
        let (surface_a, _, panel_a, _) = ids();
        let mut registry = ViewportInstanceRegistryResource::default();
        let restored = registry.restore_instance(surface_a, panel_a, ViewportId(77));

        assert_eq!(restored.viewport_id, ViewportId(77));
        assert_eq!(
            registry.viewport_for_tool_surface(surface_a),
            Some(ViewportId(77))
        );
    }

    #[test]
    fn sync_retains_only_mounted_viewport_surfaces() {
        let mut allocator = WorkspaceIdentityAllocator::new();
        let workspace_id = WorkspaceId::try_from_raw(1).unwrap();
        let workspace = WorkspaceState::bootstrap_current_layout(workspace_id, &mut allocator);
        let mut registry = ViewportInstanceRegistryResource::default();

        registry.sync_from_workspace_state(&workspace);

        assert!(registry.records().any(|record| {
            workspace
                .tool_surface(record.tool_surface_id)
                .is_some_and(is_viewport_tool_surface)
        }));
    }

    #[test]
    fn sync_restores_persisted_viewport_identity() {
        let mut allocator = WorkspaceIdentityAllocator::new();
        let workspace_id = WorkspaceId::try_from_raw(1).unwrap();
        let workspace = WorkspaceState::bootstrap_current_layout(workspace_id, &mut allocator);
        let viewport_surface = workspace
            .tool_surfaces()
            .find(|surface| is_viewport_tool_surface(surface))
            .map(|surface| surface.id)
            .expect("bootstrap workspace should contain a viewport surface");
        let workspace = reduce_workspace(
            &workspace,
            WorkspaceMutation::SetToolSurfaceViewportInstanceId {
                tool_surface_id: viewport_surface,
                viewport_instance_id: Some(ViewportId(77)),
            },
        )
        .expect("restore id mutation should be valid");
        let mut registry = ViewportInstanceRegistryResource::default();

        registry.sync_from_workspace_state(&workspace);

        assert_eq!(
            registry.viewport_for_tool_surface(viewport_surface),
            Some(ViewportId(77))
        );
    }
}
