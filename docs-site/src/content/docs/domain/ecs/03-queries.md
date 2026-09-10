---
title: Queries
description: Engine-agnostic guide for ecs queries.
status: active
owner: ecs
layer: domain
canonical: true
last_reviewed: 2026-09-10
---

# ECS Queries

Queries retrieve entities/components through typed access rules and filters.

## Purpose

- Select entity subsets by component composition.
- Support read and mutable access with ECS-owned access validation and conflict diagnostics.
- Express change-driven logic with `Added<T>` and `Changed<T>`.
- Observe recent component removals through `QueryOrphaned<T>`.

## Key Concepts

- `Query<Q, F>`: in-system typed query param.
- `QueryState<Q, F>`: detached reusable query state.
- Filters: `With<T>`, `Without<T>`, `Added<T>`, `Changed<T>`.
- `QueryOrphaned<T>` / `QueryOrphanedState<T>`: current removed-component observation window.

## API Notes

Detached query state:

```rust
let query = world.query_state::<(&mut Position, &Velocity), ()>();
for (pos, vel) in query.iter(&mut world) {
    pos.x += vel.x;
    pos.y += vel.y;
}
```

Removed-component observation:

```rust
fn process_removed(mut orphaned: QueryOrphaned<Velocity>) {
    for removed in orphaned.iter() {
        let entity = removed.entity();
        let tick = removed.tick();
        let _ = (entity, tick);
    }
}
```

## Invariants

- Queries do not observe runtime-deferred structural changes until the applicable ECS deferred-apply boundary has completed.
- `Added<T>` / `Changed<T>` use ECS-local change state and are not application frame/tick semantics.
- `QueryOrphaned<T>` reports removals in the current ECS removal-observation window; that window is not an Engine lifecycle or product-publication identity.
- Mutable query shapes must not alias the same component mutably.
- Query access facts may diagnose concurrency incompatibility but do not establish semantic `before` / `after` order.
