use std::collections::{BTreeMap, BTreeSet};

use runen_spatial::ChunkId;
use thiserror::Error;

use crate::plugins::render::scene::{
    RenderObjectId, RenderObjectIdAllocationError, RenderSceneCommit, RenderSceneCommitError,
    RenderSceneStore, RenderSceneUpdate,
};
use crate::plugins::world::chunks::{ChunkLifecycleState, WorldChunkRuntimeMapResource};

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WorldChunkRenderSceneAdapterError {
    #[error("renderer object ID allocation failed: {0}")]
    ObjectIdAllocation(#[from] RenderObjectIdAllocationError),
    #[error("renderer scene commit failed: {0}")]
    SceneCommit(#[from] RenderSceneCommitError),
}

#[derive(Debug, Default)]
pub(crate) struct WorldChunkRenderSceneAdapter {
    object_ids: BTreeMap<ChunkId, RenderObjectId>,
}

impl WorldChunkRenderSceneAdapter {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn object_id_for_chunk(&self, chunk_id: ChunkId) -> Option<RenderObjectId> {
        self.object_ids.get(&chunk_id).copied()
    }

    pub(crate) fn len(&self) -> usize {
        self.object_ids.len()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.object_ids.is_empty()
    }

    pub(crate) fn synchronize(
        &mut self,
        source: &WorldChunkRuntimeMapResource,
        scene: &mut RenderSceneStore,
    ) -> Result<RenderSceneCommit, WorldChunkRenderSceneAdapterError> {
        let participating = source
            .by_chunk_id
            .values()
            .filter(|record| participates_in_render_scene(record.lifecycle))
            .map(|record| record.chunk_id)
            .collect::<BTreeSet<_>>();

        let retiring = self
            .object_ids
            .iter()
            .filter(|(chunk_id, _)| !participating.contains(chunk_id))
            .map(|(chunk_id, object_id)| (*chunk_id, *object_id))
            .collect::<Vec<_>>();

        let mut allocated = Vec::new();
        for chunk_id in participating {
            if self.object_ids.contains_key(&chunk_id) {
                continue;
            }
            allocated.push((chunk_id, scene.allocate_object_id()?));
        }

        let mut update = RenderSceneUpdate::new();
        for (_, object_id) in &allocated {
            update.insert(*object_id);
        }
        for (_, object_id) in &retiring {
            update.remove(*object_id);
        }

        let commit = scene.commit(update)?;

        for (chunk_id, _) in retiring {
            self.object_ids.remove(&chunk_id);
        }
        for (chunk_id, object_id) in allocated {
            self.object_ids.insert(chunk_id, object_id);
        }

        Ok(commit)
    }
}

fn participates_in_render_scene(lifecycle: ChunkLifecycleState) -> bool {
    matches!(
        lifecycle,
        ChunkLifecycleState::Ready
            | ChunkLifecycleState::Resident
            | ChunkLifecycleState::Rebuilding
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use runen_spatial::{ChunkCoord3, WorldId};

    fn chunk_id(x: i64) -> ChunkId {
        ChunkId::new(WorldId::new(7), ChunkCoord3 { x, y: 0, z: 0 })
    }

    fn set_lifecycle(
        source: &mut WorldChunkRuntimeMapResource,
        chunk_id: ChunkId,
        lifecycle: ChunkLifecycleState,
    ) {
        source.ensure_chunk(chunk_id).lifecycle = lifecycle;
    }

    #[test]
    fn participating_chunks_are_inserted_and_retired_atomically() {
        let first = chunk_id(1);
        let second = chunk_id(2);
        let mut source = WorldChunkRuntimeMapResource::default();
        set_lifecycle(&mut source, first, ChunkLifecycleState::Ready);
        set_lifecycle(&mut source, second, ChunkLifecycleState::Resident);

        let mut scene = RenderSceneStore::new();
        let mut adapter = WorldChunkRenderSceneAdapter::new();

        let insert = adapter
            .synchronize(&source, &mut scene)
            .expect("participating chunks should synchronize");
        let first_object = adapter
            .object_id_for_chunk(first)
            .expect("first chunk should have renderer identity");
        let second_object = adapter
            .object_id_for_chunk(second)
            .expect("second chunk should have renderer identity");

        assert_eq!(adapter.len(), 2);
        assert_ne!(first_object, second_object);
        assert_eq!(insert.change_set().inserted(), Some(&[first_object, second_object][..]));
        assert!(insert.snapshot().contains(first_object));
        assert!(insert.snapshot().contains(second_object));

        let retained = insert.snapshot().clone();
        source.by_chunk_id.clear();
        let retirement = adapter
            .synchronize(&source, &mut scene)
            .expect("retiring chunks should synchronize atomically");

        assert!(adapter.is_empty());
        assert_eq!(retirement.change_set().removed().map(<[_]>::len), Some(2));
        assert!(retirement.snapshot().is_empty());
        assert!(retained.contains(first_object));
        assert!(retained.contains(second_object));
    }

    #[test]
    fn failed_scene_commit_does_not_mutate_adapter_mapping() {
        let chunk = chunk_id(9);
        let mut source = WorldChunkRuntimeMapResource::default();
        set_lifecycle(&mut source, chunk, ChunkLifecycleState::Ready);

        let mut scene = RenderSceneStore::new();
        let mut adapter = WorldChunkRenderSceneAdapter::new();
        adapter
            .synchronize(&source, &mut scene)
            .expect("initial synchronization should succeed");
        let object_id = adapter
            .object_id_for_chunk(chunk)
            .expect("chunk should have renderer identity");

        let mut external_remove = RenderSceneUpdate::new();
        external_remove.remove(object_id);
        scene
            .commit(external_remove)
            .expect("test setup should remove the mapped renderer object");

        source.by_chunk_id.clear();
        let revision_before = scene.revision();
        let result = adapter.synchronize(&source, &mut scene);

        assert!(matches!(
            result,
            Err(WorldChunkRenderSceneAdapterError::SceneCommit(
                RenderSceneCommitError::ObjectMissing { object_id: missing }
            )) if missing == object_id
        ));
        assert_eq!(adapter.object_id_for_chunk(chunk), Some(object_id));
        assert_eq!(scene.revision(), revision_before);
    }
}
