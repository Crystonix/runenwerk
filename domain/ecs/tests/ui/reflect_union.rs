use ecs::Reflect;

#[derive(Reflect)]
union Unsupported {
    value: u32,
}

fn main() {}
