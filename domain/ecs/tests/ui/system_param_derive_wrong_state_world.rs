#[derive(ecs::SystemParam)]
struct WrongOrder<'w, 's> {
    query: ecs::Query<'s, 'w, &'static Marker>,
}

#[derive(ecs::Component)]
struct Marker;

fn main() {}
