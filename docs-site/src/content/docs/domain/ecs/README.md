---
title: "RunenECS"
description: "Documentation for the standalone RunenECS package."
status: active
owner: runen-ecs
layer: domain
canonical: true
last_reviewed: 2026-09-10
related_designs:
  - ../../design/accepted/runenecs-extraction-boundary-design.md
  - ../../design/accepted/runenecs-boundary-repair-execution-plan.md
---

# RunenECS

The `runen-ecs` package is a deterministic, engine-agnostic entity-component-
system framework. Its Rust crate name is `runen_ecs`; the derive macros are
provided by the companion `runen-ecs-macros` package.

## Quick Overview

- `World`: entities, components, resources, and optional component indexes
- `Query`, `QueryState`, and ECS-native `Added`, `Changed`, and `RemovedQuery`
- `Runtime`, schedule labels, system sets, explicit semantic ordering, and validation
- `Res`, `ResMut`, built-in exclusive `WorldMut`, and derived `SystemParam`
- deferred `Commands` and `BatchCommands` with ECS-owned visibility boundaries
- optional standalone reflection APIs

```rust
use runen_ecs::prelude::*;

#[derive(Debug, Copy, Clone, PartialEq, runen_ecs::Component)]
struct Position { x: f32, y: f32 }

let mut world = World::new();
let entity = world.spawn(Position { x: 1.0, y: 2.0 }).unwrap();
world.require_mut::<Position>(entity).unwrap().x += 1.0;
assert_eq!(world.require::<Position>(entity).unwrap().x, 2.0);
```

## Ownership boundary

RunenECS owns live ECS state, generic schedule semantics, deterministic serial
execution, ECS access facts, structured ECS runtime errors, and deferred-command
visibility. Application lifecycle, rendering, networking, replay, product
publication, and frame policy remain host concerns.

The workspace also carries `domain/ecs_conformance`, a downstream fixture that
uses the published crate identities through a renamed dependency key. It keeps
derive resolution, `WorldMut`, runtime execution, and sealed compile-fail
boundaries checked outside the package implementation.

The `ChangeCursor` returned by `World::current_change_tick` is an ECS-local
monotonic observation position. It crosses the inner counter boundary through
an explicit epoch and panics before the absolute two-word position is reused;
it is not an application, network, or persistence revision.

## Documentation

- [Overview](./00-overview.md)
- [Usage guide](./usage-guide.md)
- [Advanced guide](./advanced-guide.md)
- [Architecture](./architecture.md)
- [Feature map](./features.md)
