#![allow(non_camel_case_types)]

#[derive(ecs::Resource)]
struct Counter;

#[derive(ecs::Resource)]
struct Marker<const N: usize>;

#[derive(ecs::SystemParam)]
struct WorldGroup<'w> {
    counter: ecs::Res<'w, Counter>,
}

#[derive(ecs::SystemParam)]
struct QueryGroup<'w, 's> {
    query: ecs::Query<'w, 's, ecs::Entity>,
}

#[derive(ecs::SystemParam)]
struct NestedQueryGroup<'w, 's> {
    inner: QueryGroup<'w, 's>,
}

#[derive(ecs::SystemParam)]
struct GenericConstGroup<'w, T: ecs::Resource, const N: usize> {
    value: ecs::Res<'w, T>,
    marker: ecs::Res<'w, Marker<N>>,
}

#[derive(ecs::SystemParam)]
struct GeneratedNameCollision<'w, world: ecs::Resource> {
    value: ecs::Res<'w, world>,
}

fn assert_param<P: ecs::SystemParam>() {}

fn main() {
    assert_param::<WorldGroup<'static>>();
    assert_param::<QueryGroup<'static, 'static>>();
    assert_param::<NestedQueryGroup<'static, 'static>>();
    assert_param::<GenericConstGroup<'static, Counter, 3>>();
    assert_param::<GeneratedNameCollision<'static, Counter>>();
}
