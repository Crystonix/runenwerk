use ecs::{ChangeExtractionFilter, Component, Resource, ResourceTypeKey, World};

#[derive(Debug, Copy, Clone, PartialEq, Eq, Component)]
struct A(u32);

#[derive(Debug, Copy, Clone, PartialEq, Eq, Component)]
struct B(u32);

#[derive(Debug, Copy, Clone, PartialEq, Eq, Resource)]
struct R(u32);

#[test]
fn extraction_orders_component_and_resource_deltas_by_stable_keys() {
    let mut world = World::new();
    let first = world.spawn(A(1)).expect("spawn should succeed");
    let second = world.spawn(A(2)).expect("spawn should succeed");
    world.insert_resource(R(1));

    world
        .insert(first, B(9))
        .expect("component insert should succeed");
    world
        .insert(second, B(5))
        .expect("component insert should succeed");
    world
        .remove::<B>(first)
        .expect("component remove should succeed");
    world.insert_resource(R(2));

    let batch = world.extract_structural_deltas_for_tick_window(
        0,
        world.current_change_tick(),
        ChangeExtractionFilter::default(),
    );

    assert!(!batch.component_deltas.is_empty());
    assert!(!batch.resource_deltas.is_empty());

    for pair in batch.component_deltas.windows(2) {
        let left = &pair[0];
        let right = &pair[1];
        assert!(
            (left.entity, left.component_key, left.tick)
                <= (right.entity, right.component_key, right.tick),
            "component deltas must be ordered by (entity, component_key, tick)"
        );
    }

    for pair in batch.resource_deltas.windows(2) {
        let left = &pair[0];
        let right = &pair[1];
        assert!(
            (left.resource_key, left.tick) <= (right.resource_key, right.tick),
            "resource deltas must be ordered by (resource_key, tick)"
        );
    }
}

#[test]
fn extraction_supports_resource_key_filtering() {
    let mut world = World::new();
    world.insert_resource(R(1));
    world.insert_resource(R(2));

    let resource_key = world
        .resource_type_key::<R>()
        .expect("resource type key should exist");

    let batch = world.extract_structural_deltas_for_tick_window(
        0,
        world.current_change_tick(),
        ChangeExtractionFilter {
            component_key_filter: None,
            resource_key_filter: Some(&move |key: ResourceTypeKey| key == resource_key),
        },
    );

    assert!(batch.component_deltas.is_empty());
    assert!(
        batch
            .resource_deltas
            .iter()
            .all(|delta| delta.resource_key == resource_key)
    );
}
