---
title: "ECS Crate"
description: "Documentation for ECS crate."
status: active
owner: ecs
layer: domain
canonical: true
last_reviewed: 2026-09-06
related_designs:
  - ../../design/accepted/execution-fabric-and-product-jobs-design.md
related_roadmaps:
  - ../../workspace/sdf-first-execution-roadmap.md
---

# ECS Crate

`ecs` is the ECS runtime foundation in the `domain` layer.

## Quick Overview

- `World`: entities/components/resources, event channels, component indexes
- Query runtime: `Query`, `QueryState`, `QueryOrphaned`
- Runtime and scheduling bridge: `Runtime`, `IntoSystem`, set ordering (`in_set`, `before`, `after`)
- System params: `Res`, `ResMut`, `ResView`, `Commands`, `BroadcastReader`, `BroadcastWriter`
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
let entity = world.spawn(Position { x: 1.0, y: 2.0 });
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

For the SDF-first open-world substrate, ECS owns live runtime state, system
interfaces, deferred command visibility, and the runtime query contracts that
feed query snapshot products. The scheduler owns deterministic planning and
barriers; engine runtime owns execution; product-family domains own product
truth.

Near-term ECS work should preserve serial equivalence while adding explicit
product-publication and query-snapshot behavior at scheduler barriers. ECS now
exposes generic barrier handlers keyed by scheduler `BarrierKind`; handlers are
runtime hooks, not product-domain knowledge. ECS must not become a global
product registry or renderer truth source.

ECS also exposes `query_snapshot_source_generation` plus explicit `QueryAccess`
builder methods for component/resource access sets. These helpers compute
deterministic source generations from existing component and resource change
tracking while keeping `domain/ecs` product-agnostic.
