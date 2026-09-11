---
title: Usage Guide
description: Engine-agnostic guide for ecs usage.
status: active
owner: runen-ecs
layer: domain
canonical: true
last_reviewed: 2026-09-10
---

# ECS Usage Guide

Audience: normal ECS users and gameplay/system authors using the public API.

For advanced scheduling, deferred-command, and index topics, see [advanced-guide.md](advanced-guide.md).
For internals and invariants, see [architecture.md](architecture.md).

## 1. Import the API

```rust
use runen_ecs::prelude::*;
```

## 2. Define Components, Tags, Resources

Per-entity data derives `runen_ecs::Component`; world-singleton data derives `runen_ecs::Resource`.

```rust
use runen_ecs::prelude::*;

#[derive(Debug, Copy, Clone, PartialEq, runen_ecs::Component)]
struct Position {
    x: f32,
    y: f32,
}

#[derive(Debug, Copy, Clone, PartialEq, runen_ecs::Component)]
struct Velocity {
    x: f32,
    y: f32,
}

#[derive(Debug, Copy, Clone, PartialEq, runen_ecs::Component)]
struct Player;

#[derive(Debug, PartialEq, Eq, runen_ecs::Resource)]
struct Frame(u64);
```

- Components: per-entity data (`Position`, `Velocity`)
- Tags: zero-sized components (`Player`)
- Resources: singleton components on `World` (`Frame`)

## 3. World Lifecycle

```rust
use runen_ecs::prelude::*;

#[derive(Debug, Copy, Clone, PartialEq, runen_ecs::Component)]
struct Position {
    x: f32,
    y: f32,
}

let mut world = World::new();
let entity = world.spawn(Position { x: 1.0, y: 2.0 });
assert!(world.contains(entity));
world.despawn(entity).unwrap();
assert!(!world.contains(entity));
```

## 4. Bundles

`World::spawn`, `World::insert`, `World::remove`, and `Commands` use `Bundle`.

Tuple bundles are supported directly:

```rust
use runen_ecs::prelude::*;

#[derive(Debug, Copy, Clone, PartialEq, runen_ecs::Component)]
struct Position {
    x: f32,
    y: f32,
}

#[derive(Debug, Copy, Clone, PartialEq, runen_ecs::Component)]
struct Velocity {
    x: f32,
    y: f32,
}

let mut world = World::new();
let entity = world.spawn((Position { x: 0.0, y: 0.0 }, Velocity { x: 1.0, y: 1.0 }));
let removed: (Position, Velocity) = world.remove(entity).unwrap();
assert_eq!(removed.0.x, 0.0);
```

Custom bundle structs are available via derive:

```rust
use runen_ecs::prelude::*;

#[derive(Debug, PartialEq, Eq, runen_ecs::Component)]
struct Health(i32);

#[derive(Debug, Clone, PartialEq, Eq, runen_ecs::Component)]
struct Name(String);

#[derive(Debug, PartialEq, runen_ecs::Bundle)]
struct CombatBundle {
    health: Health,
    name: Name,
}

let mut world = World::new();
let entity = world.spawn(CombatBundle {
    health: Health(10),
    name: Name("hero".to_string()),
});

let removed: CombatBundle = world.remove(entity).unwrap();
assert_eq!(removed.health, Health(10));
```

## 5. Entity Access

```rust
use runen_ecs::prelude::*;

#[derive(Debug, Copy, Clone, PartialEq, runen_ecs::Component)]
struct Position {
    x: f32,
    y: f32,
}

let mut world = World::new();
let entity = world.spawn(Position { x: 1.0, y: 2.0 });

assert_eq!(world.require::<Position>(entity).unwrap().x, 1.0);
world.require_mut::<Position>(entity).unwrap().x += 1.0;

let entity_ref = world.entity(entity).unwrap();
assert!(entity_ref.contains::<Position>());

let mut entity_mut = world.entity_mut(entity).unwrap();
entity_mut.require_mut::<Position>().unwrap().y += 1.0;
```

## 6. Queries

Detached reusable query state:

```rust
use runen_ecs::prelude::*;

#[derive(Debug, Copy, Clone, PartialEq, runen_ecs::Component)]
struct Position {
    x: f32,
    y: f32,
}

#[derive(Debug, Copy, Clone, PartialEq, runen_ecs::Component)]
struct Velocity {
    x: f32,
    y: f32,
}

let mut world = World::new();
world.spawn((Position { x: 0.0, y: 0.0 }, Velocity { x: 1.0, y: 2.0 }));

let query = world.query_state::<(&mut Position, &Velocity), ()>();
for (position, velocity) in query.iter(&mut world) {
    position.x += velocity.x;
    position.y += velocity.y;
}
```

Common forms:

- `Query<&T>`
- `Query<&mut T>`
- `Query<(Entity, &T)>`
- `Query<(Entity, &mut T)>`
- `Query<(&A, &B)>`
- `Query<(&mut A, &B)>`
- `Query<(&mut A, &mut B)>`
- `Query<Option<&T>>`
- `Query<Option<&mut T>>`
- `Query<(Entity, Option<&T>)>`

## 7. Query Filters

```rust
use runen_ecs::prelude::*;

fn players(_query: Query<&Position, With<Player>>) {}
fn active_players(_query: Query<&Position, (With<Player>, Without<Disabled>)>) {}
fn changed_players(_query: Query<&Position, Changed<Position>>) {}
fn added_health(_query: Query<(Entity, &Health), Added<Health>>) {}

# #[derive(runen_ecs::Component)] struct Position;
# #[derive(runen_ecs::Component)] struct Player;
# #[derive(runen_ecs::Component)] struct Disabled;
# #[derive(runen_ecs::Component)] struct Health;
```

`Changed<T>` and `Added<T>` are stateful per `QueryState`/`Query` instance.

## 8. Resources

```rust
use runen_ecs::prelude::*;

#[derive(Debug, PartialEq, Eq, runen_ecs::Resource)]
struct Frame(u64);

let mut world = World::new();
world.insert_resource(Frame(1));
assert!(world.has_resource::<Frame>());

world.resource_mut::<Frame>().unwrap().0 += 1;
assert_eq!(world.resource::<Frame>().unwrap().0, 2);
assert_eq!(world.remove_resource::<Frame>(), Some(Frame(2)));
```

## 9. Commands

```rust
use runen_ecs::prelude::*;

#[derive(Debug, Copy, Clone, PartialEq, runen_ecs::Component)]
struct Position {
    x: f32,
    y: f32,
}

#[derive(Debug, Copy, Clone, PartialEq, runen_ecs::Component)]
struct Velocity {
    x: f32,
    y: f32,
}

let mut world = World::new();
let entity = world.spawn(Position { x: 1.0, y: 1.0 });

let mut commands = world.commands();
commands.insert(entity, Velocity { x: 0.5, y: 0.0 });
commands.spawn(Position { x: 3.0, y: 4.0 });
commands.apply(&mut world).unwrap();
```

## 10. Runtime Basics

`ScheduleLabel` is owned by `runen-ecs` and is available through the prelude.

```rust
use runen_ecs::prelude::*;

#[derive(Copy, Clone)]
struct Update;
impl ScheduleLabel for Update {
    fn name() -> &'static str {
        "Update"
    }
}

#[derive(Debug, PartialEq, Eq, runen_ecs::Resource)]
struct Frame(u64);

fn tick(mut frame: ResMut<Frame>) {
    frame.0 += 1;
}

let mut world = World::new();
world.insert_resource(Frame(0));

let mut runtime = Runtime::new();
runtime.add_systems::<Update, _, _>(&mut world, tick);
runtime.run_schedule::<Update>(&mut world).unwrap();
assert_eq!(world.resource::<Frame>().unwrap().0, 1);
```

## 11. Ordering and Deferred Visibility

Use system sets and explicit `before` / `after` relations when one system must observe deferred structural changes from another.

```rust
use runen_ecs::prelude::*;

#[derive(Copy, Clone)]
struct Update;
impl ScheduleLabel for Update {}

#[derive(Copy, Clone)]
struct Produce;
impl SystemSet for Produce {}

#[derive(Copy, Clone)]
struct Observe;
impl SystemSet for Observe {}

fn produce(mut commands: Commands) {
    commands.spawn(());
}

fn observe() {}

let mut world = World::new();
let mut runtime = Runtime::new();
runtime.add_systems::<Update, _, _>(&mut world, produce.in_set(Produce));
runtime.add_systems::<Update, _, _>(&mut world, observe.in_set(Observe).after(Produce));
runtime.run_schedule::<Update>(&mut world).unwrap();
```

The explicit set edge establishes semantic precedence. Deferred commands from the earlier ordered work are applied at an ECS deferred-apply boundary before dependent later work runs. Read/write access conflicts are reported independently and do not create ordering edges or extra visibility boundaries. Current execution-plan stage indices are diagnostic/planning facts rather than application lifecycle or publication identity.

Generic broadcast/event channels are not part of the current RunenECS public contract; see [04-events.md](04-events.md).

## 12. Change Tracking Basics

```rust
use runen_ecs::prelude::*;

#[derive(Debug, Copy, Clone, PartialEq, runen_ecs::Component)]
struct Position {
    x: f32,
    y: f32,
}

let mut world = World::new();
let tick = world.current_change_tick();
let entity = world.spawn(Position { x: 0.0, y: 0.0 });
world.require_mut::<Position>(entity).unwrap().x = 1.0;

assert!(world.component_changed_since::<Position>(tick));
assert!(world.current_change_tick() > tick);
```

## 13. Secondary Indexes Basics

```rust
use runen_ecs::prelude::*;

#[derive(Debug, Clone, PartialEq, Eq, runen_ecs::Component)]
struct Name(String);

let mut world = World::new();
world.ensure_component_index::<Name, String>(|name| name.0.clone());

let hero = world.spawn(Name("hero".to_string()));
assert_eq!(
    world.find_entity_by_index::<Name, String>(&"hero".to_string()),
    Some(hero)
);
```

## 14. Error Handling Basics

Common error types:

- `EntityError`: invalid entity or missing component during entity operations
- `ResourceError`: missing resource access
- `QueryError`: query cardinality errors (for example `single()`)
- `CommandError`: deferred command apply failure
- `ScheduleValidationError`: invalid schedule structure such as an explicit ordering cycle

```rust
use runen_ecs::prelude::*;

#[derive(Debug, PartialEq, Eq, runen_ecs::Resource)]
struct Frame(u64);

let world = World::new();
let missing = world.resource::<Frame>();
assert!(missing.is_err());
```

## 15. Benchmark entry point

The package contains a semantic Criterion baseline covering insertion, query
iteration, archetype transitions, serial schedule execution, and deferred
command application:

```text
cargo bench -p runen-ecs --bench semantic_baseline --locked
```

The benchmark is a measurement aid, not a hard performance guarantee. Its
provenance is maintained in the standalone repository's
`benchmarks/semantic_baseline.md`; Runenwerk does not carry benchmark artifacts
or a second benchmark authority.
