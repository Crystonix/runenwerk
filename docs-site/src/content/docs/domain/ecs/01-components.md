---
title: Components
description: Engine-agnostic guide to defining and using ecs components in the domain layer.
status: active
owner: ecs
layer: domain
canonical: true
last_reviewed: 2026-04-27
---

# ECS Components

Components represent per-entity state in the ECS. They are small, focused, and engine-agnostic. Components should encapsulate only data and avoid behavior outside domain rules.

## Purpose

- Represent per-entity or per-instance state.
- Enable systems to query and modify entities efficiently.
- Provide type-safe guarantees for domain logic.

## Key Concepts

- **Component** – A struct representing data for an entity.
- **Tag / Marker Component** – Zero-sized components used to mark entities.
- **Resource** – Singleton data attached to the world, not per entity.
- **Bundle** – A group of components that can be added/removed together.

## Implementation / API

Components derive `runen_ecs::Component`. Resources derive `runen_ecs::Resource`. Bundles derive `runen_ecs::Bundle`.

### Basic Component

Example of a per-entity component:

```rust
#[derive(Debug, Copy, Clone, PartialEq, runen_ecs::Component)]
struct Position {
    x: f32,
    y: f32,
}
```
### Tag / Marker Component

Zero-sized markers for entities:
```rust
#[derive(Debug, Copy, Clone, PartialEq, runen_ecs::Component)]
struct Player;
```
### Resource

Singleton world data:
```rust
#[derive(Debug, PartialEq, Eq, runen_ecs::Resource)]
struct Frame(u64);
```
### Bundle

Grouping multiple components:
```rust
#[derive(Debug, PartialEq, runen_ecs::Bundle)]
struct CombatBundle {
health: Health,
name: Name,
}
```
## Invariants & Rules

- Components should be **small** and **focused**.
- Prefer **immutable fields** where possible.
- Avoid engine-specific data, references, or handles.
- Resources should be used for global/shared state.
- Systems must declare explicit read/write access when querying components.

## Usage Examples

### Example 1: Spawning an Entity
```rust
let mut world = World::new();
let entity = world.spawn(Position { x: 1.0, y: 2.0 });
assert!(world.contains(entity));
```
### Example 2: Using Bundles
```rust
let entity = world.spawn(CombatBundle {
health: Health(10),
name: Name("hero".to_string()),
});
let removed: CombatBundle = world.remove(entity).unwrap();
assert_eq!(removed.health, Health(10));
```
### Example 3: Tag Components
```rust
world.spawn((Position { x: 0.0, y: 0.0 }, Player));
```
### Example 4: Resources
```rust
world.insert_resource(Frame(0));
world.resource_mut::<Frame>().unwrap().0 += 1;
```
## Design Guidelines

- Keep components **independent of engine specifics**.
- Prefer **plain data types** and avoid logic in components.
- Group related components using **Bundles**.
- Use **Resources** for singleton/shared domain state.

## Integration Notes

- Engine adapters consume components to implement runtime, rendering, networking, or tooling.
- Components should remain engine-agnostic for cross-platform reuse.
- Bundles, resources, and marker components can be leveraged for editor tooling.

## Future Considerations

- History-aware components for undo/redo and editor integration.
- Reactive components that notify systems of changes automatically.
- Transient components for ephemeral objects or gizmos.

## References & Links

- [usage-guide.md](usage-guide.md) – Basic ECS API usage.
- [advanced-guide.md](advanced-guide.md) – Deferred commands, scheduling, and runtime integration.
- [architecture.md](architecture.md) – Internal invariants and unsafe boundaries.
- [features.md](features.md) – ECS feature map.
