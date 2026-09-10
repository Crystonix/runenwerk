#[derive(Debug, Clone, Default, runen_ecs::Reflect)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone, Default, runen_ecs::Component, runen_ecs::Reflect)]
pub struct Position {
    pub value: Vec2,
    pub speed: f32,
    pub label: String,
}
