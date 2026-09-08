use runen_spatial::{ChunkCoord3, ChunkId, WorldId};

use crate::plugins::render::scene::RenderSceneStore;
use crate::plugins::ui::render_scene::UiSurfaceRenderSceneAdapter;
use crate::plugins::ui::{UiMountRequest, UiMountRequestsResource, UiMountSource};
use crate::plugins::world::adapters::render_scene::WorldChunkRenderSceneAdapter;
use crate::plugins::world::chunks::{ChunkLifecycleState, WorldChunkRuntimeMapResource};

fn chunk_id(x: i64) -> ChunkId {
    ChunkId::new(WorldId::new(11), ChunkCoord3 { x, y: 0, z: 0 })
}

fn mount(source: &mut UiMountRequestsResource, screen: &str) -> ui_surface::SurfaceInstanceId {
    source.record_mount_request(UiMountRequest::new(screen), UiMountSource::AppMountUi);
    source
        .mounted_sessions()
        .last()
        .expect("accepted mount request should create a mounted session")
        .surface_instance_id()
}

#[test]
fn world_and_ui_share_one_renderer_identity_and_scene_authority() {
    let first_chunk = chunk_id(1);
    let second_chunk = chunk_id(2);
    let mut world_source = WorldChunkRuntimeMapResource::default();
    world_source.ensure_chunk(first_chunk).lifecycle = ChunkLifecycleState::Ready;
    world_source.ensure_chunk(second_chunk).lifecycle = ChunkLifecycleState::Rebuilding;

    let mut ui_source = UiMountRequestsResource::default();
    let surface = mount(&mut ui_source, "r1.shared-authority");

    let mut scene = RenderSceneStore::new();
    let mut world_adapter = WorldChunkRenderSceneAdapter::new();
    let mut ui_adapter = UiSurfaceRenderSceneAdapter::new();

    world_adapter
        .synchronize(&world_source, &mut scene)
        .expect("world adapter should publish participating chunks");
    ui_adapter
        .synchronize(&ui_source, &mut scene)
        .expect("UI adapter should publish mounted surface");

    let first_object = world_adapter
        .object_id_for_chunk(first_chunk)
        .expect("first world chunk should have renderer identity");
    let second_object = world_adapter
        .object_id_for_chunk(second_chunk)
        .expect("second world chunk should have renderer identity");
    let ui_object = ui_adapter
        .object_id_for_surface(surface)
        .expect("mounted UI surface should have renderer identity");

    assert_ne!(first_object, second_object);
    assert_ne!(first_object, ui_object);
    assert_ne!(second_object, ui_object);

    let retained = scene.snapshot();
    assert_eq!(retained.len(), 3);
    assert!(retained.contains(first_object));
    assert!(retained.contains(second_object));
    assert!(retained.contains(ui_object));

    let revision_before_allocation = scene.revision();
    let reserved_only = scene
        .allocate_object_id()
        .expect("renderer allocator should issue another unique identity");
    assert_eq!(scene.revision(), revision_before_allocation);
    assert_ne!(reserved_only, first_object);
    assert_ne!(reserved_only, second_object);
    assert_ne!(reserved_only, ui_object);

    world_source.by_chunk_id.clear();
    let retirement = world_adapter
        .synchronize(&world_source, &mut scene)
        .expect("world retirement should commit atomically");

    assert_eq!(retirement.change_set().removed().map(<[_]>::len), Some(2));
    assert!(!retirement.snapshot().contains(first_object));
    assert!(!retirement.snapshot().contains(second_object));
    assert!(retirement.snapshot().contains(ui_object));
    assert!(retained.contains(first_object));
    assert!(retained.contains(second_object));
    assert!(retained.contains(ui_object));

    ui_source.unmount_surface(surface);
    ui_adapter
        .synchronize(&ui_source, &mut scene)
        .expect("UI retirement should not depend on world adapter state");
    assert!(scene.snapshot().is_empty());
}
