use ecs::{Component, ComponentChangeKind, Resource, ResourceChangeKind, World};

#[derive(Debug, Copy, Clone, PartialEq, Eq, Component)]
struct A(u32);

#[derive(Debug, Copy, Clone, PartialEq, Eq, Resource)]
struct R(u32);

#[test]
fn local_change_cursor_observes_component_and_resource_mutations() {
    let mut world = World::new();
    let start = world.current_change_tick();

    let entity = world.spawn(A(1)).expect("spawn should succeed");
    let after_component = world.current_change_tick();
    assert!(after_component > start);

    world.insert_resource(R(1));
    let after_resource = world.current_change_tick();
    assert!(after_resource > after_component);

    let component_key = world
        .component_type_key::<A>()
        .expect("component type key should exist");
    let component_changes = world.component_changes_since(start);
    assert_eq!(component_changes.len(), 1);
    assert_eq!(component_changes[0].entity, entity);
    assert_eq!(component_changes[0].component_key, component_key);
    assert_eq!(component_changes[0].kind, ComponentChangeKind::Added);
    assert!(component_changes[0].tick > start);

    let resource_key = world
        .resource_type_key::<R>()
        .expect("resource type key should exist");
    let resource_changes = world.resource_changes_since(start);
    assert_eq!(resource_changes.len(), 1);
    assert_eq!(resource_changes[0].resource_key, resource_key);
    assert_eq!(resource_changes[0].kind, ResourceChangeKind::Inserted);
    assert!(resource_changes[0].tick > after_component);

    assert!(world.component_changes_since(after_component).is_empty());
    assert!(world.resource_changes_since(after_resource).is_empty());
}

#[test]
fn local_change_cursor_reports_only_mutations_after_the_cursor() {
    let mut world = World::new();
    let entity = world.spawn(A(1)).expect("spawn should succeed");
    world.insert_resource(R(1));
    let cursor = world.current_change_tick();

    world
        .remove::<A>(entity)
        .expect("component removal should succeed");
    assert_eq!(world.remove_resource::<R>(), Some(R(1)));

    let component_changes = world.component_changes_since(cursor);
    assert_eq!(component_changes.len(), 1);
    assert_eq!(component_changes[0].entity, entity);
    assert_eq!(component_changes[0].kind, ComponentChangeKind::Removed);
    assert!(component_changes[0].tick > cursor);

    let resource_changes = world.resource_changes_since(cursor);
    assert_eq!(resource_changes.len(), 1);
    assert_eq!(resource_changes[0].kind, ResourceChangeKind::Removed);
    assert!(resource_changes[0].tick > component_changes[0].tick);
}
