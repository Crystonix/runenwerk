#[derive(ecs::SystemParam)]
struct WrongOrder<'w, 's> {
    query: ecs::Query<'s, 'w, ecs::Entity>,
}

fn main() {}
