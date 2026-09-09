---
title: Systems
description: Engine-agnostic guide to defining and using ecs systems in the domain layer.
status: active
owner: ecs
layer: domain
canonical: true
last_reviewed: 2026-09-09
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
- **Semantic Stage** – A deterministic execution level derived from explicit ordering, not from access incompatibility.

## Implementation / API

Systems are added to a `Runtime` under an ECS-owned `ScheduleLabel`. System parameters provide the read/write access facts used for validation and conflict diagnostics.

### System with Query

```rust
#[derive(Debug, Copy, Clone, PartialEq, ecs::Component)]
struct Position { x: f32, y: f32 }

#[derive(Debug, Copy, Clone, PartialEq, ecs::Component)]
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
use ecs::prelude::*;

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

The explicit `after(Gameplay)` edge puts `PostGameplay` in a later semantic stage. Without such an ordering edge, systems remain semantically unordered even when their access facts conflict.

## Invariants & Rules

- System parameters declare ECS **access facts**. Conflicting access may constrain future parallel admission, but it does not invent an A-before-B semantic order.
- Explicit `before` / `after` set relations define semantic ordering and are cycle-validated.
- Systems within one semantic stage execute serially in deterministic registration order in the reference executor.
- Structural changes are **deferred** and become visible after the semantic stage flush.
- Systems in the same semantic stage do not observe one another's deferred structural mutations.
- Avoid hidden side effects outside system parameters when deterministic behavior matters.
- Use plan reports, history, and telemetry for diagnostics rather than as gameplay authority.

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
