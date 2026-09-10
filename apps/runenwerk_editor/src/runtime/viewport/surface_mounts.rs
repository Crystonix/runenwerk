//! File: apps/runenwerk_editor/src/runtime/viewport/surface_mounts.rs
//! Purpose: Runtime resources for ui_surface definitions and mounted surface lifecycle.

use editor_shell::{
    EditorCompositionRuntime, ToolSurfaceStableKey, editor_surface_definitions,
    tool_surface_definition_id, tool_surface_kind_for_stable_key,
};
use ui_surface::{
    MountedSurfaceInstance, MountedSurfaceRegistry, SurfaceDefinition, SurfaceDefinitionId,
    SurfaceDefinitionRegistry, SurfaceInstanceId,
};

#[derive(Debug, Clone, runen_ecs::Component, runen_ecs::Resource)]
pub struct SurfaceDefinitionRegistryResource {
    registry: SurfaceDefinitionRegistry,
}

impl Default for SurfaceDefinitionRegistryResource {
    fn default() -> Self {
        let mut registry = SurfaceDefinitionRegistry::default();
        for definition in editor_surface_definitions() {
            registry.register(definition);
        }
        Self { registry }
    }
}

impl SurfaceDefinitionRegistryResource {
    pub fn definition(&self, definition_id: SurfaceDefinitionId) -> Option<SurfaceDefinition> {
        self.registry.definition(definition_id)
    }

    pub fn definitions(&self) -> impl Iterator<Item = SurfaceDefinition> + '_ {
        self.registry.definitions()
    }

    pub fn is_empty(&self) -> bool {
        self.registry.is_empty()
    }
}

#[derive(Debug, Clone, runen_ecs::Component, runen_ecs::Resource, Default)]
pub struct MountedSurfaceRegistryResource {
    registry: MountedSurfaceRegistry,
}

impl MountedSurfaceRegistryResource {
    pub fn generation(&self) -> u64 {
        self.registry.generation()
    }

    pub fn mounted_surface(
        &self,
        surface_instance_id: SurfaceInstanceId,
    ) -> Option<MountedSurfaceInstance> {
        self.registry.mounted_surface(surface_instance_id)
    }

    pub fn mounted_surfaces(&self) -> impl Iterator<Item = MountedSurfaceInstance> + '_ {
        self.registry.mounted_surfaces()
    }

    pub fn is_empty(&self) -> bool {
        self.registry.is_empty()
    }

    pub fn sync_from_composition(&mut self, runtime: &EditorCompositionRuntime) {
        let mounts = runtime
            .extension()
            .mounted_units()
            .iter()
            .filter_map(|record| {
                let stable_key =
                    ToolSurfaceStableKey::new(record.stable_content_key.clone()).ok()?;
                let kind = tool_surface_kind_for_stable_key(&stable_key)?;
                Some(MountedSurfaceInstance::new(
                    SurfaceInstanceId::new(record.compatibility_surface_raw),
                    tool_surface_definition_id(kind),
                    ui_surface::SurfaceHostInstanceId::new(record.panel_instance_raw),
                ))
            });
        self.registry.rebuild(mounts);
    }

    #[cfg(test)]
    pub fn sync_from_workspace_state(&mut self, workspace: &editor_shell::WorkspaceState) {
        let runtime = editor_shell::import_legacy_workspace(
            editor_shell::SCENE_WORKSPACE_PROFILE_ID,
            workspace,
        )
        .unwrap();
        self.sync_from_composition(&runtime);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use editor_shell::{
        VIEWPORT_SURFACE_DEFINITION_ID, WorkspaceIdentityAllocator, WorkspaceState,
    };

    #[test]
    fn definition_registry_seeds_editor_surface_definitions() {
        let definitions = SurfaceDefinitionRegistryResource::default()
            .definitions()
            .collect::<Vec<_>>();

        assert!(!definitions.is_empty());
        assert!(
            definitions
                .iter()
                .any(|definition| definition.id == VIEWPORT_SURFACE_DEFINITION_ID)
        );
    }

    #[test]
    fn mounted_registry_syncs_from_workspace_mounts() {
        let mut allocator = WorkspaceIdentityAllocator::new();
        let workspace_id = allocator.allocate_workspace_id();
        let workspace_state =
            WorkspaceState::bootstrap_current_layout(workspace_id, &mut allocator);

        let mut mounts = MountedSurfaceRegistryResource::default();
        mounts.sync_from_workspace_state(&workspace_state);

        assert_eq!(mounts.generation(), 1);
        assert_eq!(mounts.mounted_surfaces().count(), 5);
    }
}
