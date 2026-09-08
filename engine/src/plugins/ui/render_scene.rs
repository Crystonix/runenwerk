use std::collections::{BTreeMap, BTreeSet};

use thiserror::Error;
use ui_surface::SurfaceInstanceId;

use crate::plugins::render::scene::{
    RenderObjectId, RenderObjectIdAllocationError, RenderSceneCommit, RenderSceneCommitError,
    RenderSceneStore, RenderSceneUpdate,
};

use super::UiMountRequestsResource;

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub(crate) enum UiSurfaceRenderSceneAdapterError {
    #[error("renderer object ID allocation failed: {0}")]
    ObjectIdAllocation(#[from] RenderObjectIdAllocationError),
    #[error("renderer scene commit failed: {0}")]
    SceneCommit(#[from] RenderSceneCommitError),
}

#[derive(Debug, Default)]
pub(crate) struct UiSurfaceRenderSceneAdapter {
    object_ids: BTreeMap<SurfaceInstanceId, RenderObjectId>,
}

impl UiSurfaceRenderSceneAdapter {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn object_id_for_surface(
        &self,
        surface_instance_id: SurfaceInstanceId,
    ) -> Option<RenderObjectId> {
        self.object_ids.get(&surface_instance_id).copied()
    }

    pub(crate) fn len(&self) -> usize {
        self.object_ids.len()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.object_ids.is_empty()
    }

    pub(crate) fn synchronize(
        &mut self,
        source: &UiMountRequestsResource,
        scene: &mut RenderSceneStore,
    ) -> Result<RenderSceneCommit, UiSurfaceRenderSceneAdapterError> {
        let mounted = source
            .mounted_sessions()
            .iter()
            .map(|session| session.surface_instance_id())
            .collect::<BTreeSet<_>>();

        let retiring = self
            .object_ids
            .iter()
            .filter(|(surface_instance_id, _)| !mounted.contains(surface_instance_id))
            .map(|(surface_instance_id, object_id)| (*surface_instance_id, *object_id))
            .collect::<Vec<_>>();

        let mut allocated = Vec::new();
        for surface_instance_id in mounted {
            if self.object_ids.contains_key(&surface_instance_id) {
                continue;
            }
            allocated.push((surface_instance_id, scene.allocate_object_id()?));
        }

        let mut update = RenderSceneUpdate::new();
        for (_, object_id) in &allocated {
            update.insert(*object_id);
        }
        for (_, object_id) in &retiring {
            update.remove(*object_id);
        }

        let commit = scene.commit(update)?;

        for (surface_instance_id, _) in retiring {
            self.object_ids.remove(&surface_instance_id);
        }
        for (surface_instance_id, object_id) in allocated {
            self.object_ids.insert(surface_instance_id, object_id);
        }

        Ok(commit)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugins::ui::{UiMountRequest, UiMountSource};

    fn mount(source: &mut UiMountRequestsResource, screen: &str) -> SurfaceInstanceId {
        source.record_mount_request(UiMountRequest::new(screen), UiMountSource::AppMountUi);
        source
            .mounted_sessions()
            .last()
            .expect("accepted mount request should create a mounted session")
            .surface_instance_id()
    }

    #[test]
    fn mounted_surfaces_are_inserted_and_unmounted_surfaces_are_removed() {
        let mut source = UiMountRequestsResource::default();
        let first = mount(&mut source, "hud.main");
        let second = mount(&mut source, "hud.overlay");

        let mut scene = RenderSceneStore::new();
        let mut adapter = UiSurfaceRenderSceneAdapter::new();
        let insert = adapter
            .synchronize(&source, &mut scene)
            .expect("mounted surfaces should synchronize");
        let first_object = adapter
            .object_id_for_surface(first)
            .expect("first surface should have renderer identity");
        let second_object = adapter
            .object_id_for_surface(second)
            .expect("second surface should have renderer identity");

        assert_eq!(adapter.len(), 2);
        assert_ne!(first_object, second_object);
        assert_eq!(insert.change_set().inserted(), Some(&[first_object, second_object][..]));

        source.unmount_surface(first);
        let removal = adapter
            .synchronize(&source, &mut scene)
            .expect("unmounted surface should be removed from renderer scene");

        assert_eq!(adapter.len(), 1);
        assert_eq!(adapter.object_id_for_surface(first), None);
        assert_eq!(adapter.object_id_for_surface(second), Some(second_object));
        assert_eq!(removal.change_set().removed(), Some(&[first_object][..]));
        assert!(!removal.snapshot().contains(first_object));
        assert!(removal.snapshot().contains(second_object));
    }

    #[test]
    fn failed_scene_commit_does_not_mutate_adapter_mapping() {
        let mut source = UiMountRequestsResource::default();
        let surface = mount(&mut source, "hud.failure-proof");

        let mut scene = RenderSceneStore::new();
        let mut adapter = UiSurfaceRenderSceneAdapter::new();
        adapter
            .synchronize(&source, &mut scene)
            .expect("initial synchronization should succeed");
        let object_id = adapter
            .object_id_for_surface(surface)
            .expect("surface should have renderer identity");

        let mut external_remove = RenderSceneUpdate::new();
        external_remove.remove(object_id);
        scene
            .commit(external_remove)
            .expect("test setup should remove the mapped renderer object");

        source.unmount_surface(surface);
        let revision_before = scene.revision();
        let result = adapter.synchronize(&source, &mut scene);

        assert!(matches!(
            result,
            Err(UiSurfaceRenderSceneAdapterError::SceneCommit(
                RenderSceneCommitError::ObjectMissing { object_id: missing }
            )) if missing == object_id
        ));
        assert_eq!(adapter.object_id_for_surface(surface), Some(object_id));
        assert_eq!(scene.revision(), revision_before);
    }

    #[test]
    fn empty_source_keeps_adapter_empty() {
        let source = UiMountRequestsResource::default();
        let mut scene = RenderSceneStore::new();
        let mut adapter = UiSurfaceRenderSceneAdapter::new();

        let commit = adapter
            .synchronize(&source, &mut scene)
            .expect("empty source should synchronize as no-op");

        assert!(adapter.is_empty());
        assert!(commit.change_set().is_empty_incremental());
    }
}
