---
title: Systems
description: Engine-agnostic guide to defining and using ecs systems in the domain layer.
status: active
owner: ecs
layer: domain
canonical: true
last_reviewed: 2026-09-10
---

# ECS Systems

Systems are functions or processes that operate over components and resources. They define ECS behavior by reading and writing data through explicit system parameters.

## Purpose

- Encapsulate domain logic applied to entities and resources.
- Enable deterministic execution of gameplay or simulation rules.
- Separate computation from data storage (components/resources).

## Key Concepts

- **System** – A function that queries components/resources and performs updates.
- **System Param** – A typed input describing required ECS access.
- **Query** – Filters and retrieves entities with matching components.
- **Command Queue** – Deferred structural mutations collected per system run.
- **System Set** – A semantic grouping used by explicit ordering constraints.
- **Deferred Apply Boundary** – The ECS-owned point at which queued structural mutations become visible.
- **WorldMut** – Built-in exclusive access to the complete world for a system that needs coordinated ECS operations.

## Implementation / API

Systems are added to a `Runtime` under an ECS-owned `ScheduleLabel`. System parameters provide the read/write access facts used for validation and conflict diagnostics.

### System with Query

```rust
#[derive(Debug, Copy, Clone, PartialEq, runen_ecs::Component)]
struct Position { x: f32, y: f32 }

#[derive(Debug, Copy, Clone, PartialEq, runen_ecs::Component)]
struct Velocity { x: f32, y: f32 }

fn movement_system(mut query: Query<(&mut Position, &Velocity)>) {
    for (pos, vel) in query.iter() {
        pos.x += vel.x;
        pos.y += vel.y;
    }
}
```

### Deferred Commands

Systems can queue structural changes safely:

```rust
fn spawn_entity(mut commands: Commands) {
    commands.spawn(Position { x: 0.0, y: 0.0 });
}
```

### Set Ordering

Runtime execution can be ordered explicitly. `ScheduleLabel` and `SystemSet` are available from the ECS prelude:

```rust
use runen_ecs::prelude::*;

#[derive(Copy, Clone)]
struct Update;
impl ScheduleLabel for Update {}

#[derive(Copy, Clone)]
struct Gameplay;
impl SystemSet for Gameplay {}

#[derive(Copy, Clone)]
struct PostGameplay;
impl SystemSet for PostGameplay {}

runtime.add_systems::<Update, _, _>(&mut world, tick.in_set(Gameplay));
runtime.add_systems::<Update, _, _>(
    &mut world,
    spawn_entity.in_set(PostGameplay).after(Gameplay),
);
```

The explicit `after(Gameplay)` edge establishes semantic precedence. Deferred commands produced by earlier ordered work are applied at an ECS deferred-apply boundary before dependent later work executes. Without such an ordering edge, systems remain semantically unordered even when their access facts conflict.

## Invariants & Rules

- System parameters declare ECS **access facts**. Conflicting access may constrain future parallel admission, but it does not invent an A-before-B semantic order.
- Explicit `before` / `after` set relations define semantic ordering and are cycle-validated.
- The serial reference executor uses deterministic registration order for otherwise unordered systems.
- Structural changes are **deferred** and become visible only after an ECS deferred-apply boundary.
- Systems that execute before the same deferred-apply boundary do not observe one another's deferred structural mutations.
- `WorldMut` is an ECS-owned exclusive parameter; it cannot be combined with sibling world borrows in one system.
- Avoid hidden side effects outside system parameters when deterministic behavior matters.
- Runtime errors preserve ECS-owned categories while user failures remain causes.

## Usage Examples

### Movement System

```rust
fn movement_system(mut query: Query<(&mut Position, &Velocity)>) {
    for (pos, vel) in query.iter() {
        pos.x += vel.x;
        pos.y += vel.y;
    }
}
```

### Spawn System

```rust
fn spawn_entity(mut commands: Commands) {
    commands.spawn(Position { x: 0.0, y: 0.0 });
}
```

## Design Guidelines

- Keep reusable system logic independent of Engine lifecycle policy.
- Let system parameters describe component/resource access.
- Use explicit system-set ordering only when the behavior actually requires semantic order or deferred visibility.
- Do not use access conflicts as substitute ordering edges.
- Keep product publication, render phases, replay/network lifecycle, and other host policy outside RunenECS.

## References & Links

- [usage-guide.md](usage-guide.md) – Normal ECS usage.
- [advanced-guide.md](advanced-guide.md) – Deferred commands, scheduling, and runtime integration.
- [architecture.md](architecture.md) – Internal scheduling and runtime invariants.
- [05-commands.md](05-commands.md) – Deferred command behavior.
