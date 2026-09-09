---
title: "ECS Crate"
description: "Documentation for ECS crate."
status: active
owner: ecs
layer: domain
canonical: true
last_reviewed: 2026-09-09
related_designs:
  - ../../design/accepted/execution-fabric-and-product-jobs-design.md
related_roadmaps:
  - ../../workspace/sdf-first-execution-roadmap.md
---

# ECS Crate

`ecs` is the ECS runtime foundation in the `domain` layer.

## Quick Overview

- `World`: entities/components/resources and component indexes
- Query runtime: `Query`, `QueryState`, `QueryOrphaned`
- ECS scheduling: `Runtime`, `ScheduleLabel`, `SystemSet`, `in_set`, `before`, `after`
- Schedule diagnostics: semantic stages, cycle validation, access conflicts, parameter-slot metadata
- System params: `Res`, `ResMut`, `ResView`, `Commands`
- Deferred mutation primitives: `Commands`, `DeferredCommand`, `BatchCommands`
- Stateful tracking: `StatefulComponent`, `component_state`, `mark_stateful_changed`

## Minimal Getting Started

```rust
use ecs::prelude::*;

#[derive(Debug, Copy, Clone, PartialEq, ecs::Component)]
struct Position {
    x: f32,
    y: f32,
}

let mut world = World::new();
let entity = world.spawn(Position { x: 1.0, y: 2.0 }).unwrap();
world.require_mut::<Position>(entity).unwrap().x += 1.0;
assert_eq!(world.require::<Position>(entity).unwrap().x, 2.0);
```

## Documentation

- Docs hub: [00-overview.md](./00-overview.md)
- Usage guide: [usage-guide.md](./usage-guide.md)
- Advanced guide: [advanced-guide.md](./advanced-guide.md)
- Architecture (internals): [architecture.md](./architecture.md)
- Feature map: [features.md](./features.md)

## SDF-First Execution Ownership

For the SDF-first open-world substrate, RunenECS owns live ECS state, system
interfaces, deterministic system identity, generic schedule labels and system
sets, explicit semantic ordering, ECS access facts, schedule validation,
deterministic serial reference execution, and deferred-command visibility.
Access incompatibility is diagnostic information and does not itself create
semantic `before`/`after` order.

Deferred commands become visible after each ECS semantic stage. `Runtime` can
expose that point as an ECS-neutral `ScheduleBoundary` after the deferred flush.
Runenwerk Engine may use that boundary to apply application policy, but product
publication, query-snapshot publication, rendering, replay/network capture, and
host lifecycle meaning are not RunenECS concepts.

ECS also exposes `query_snapshot_source_generation` plus explicit `QueryAccess`
builder methods for component/resource access sets. These helpers compute
deterministic source generations from existing component and resource change
tracking while keeping `domain/ecs` product-agnostic.
