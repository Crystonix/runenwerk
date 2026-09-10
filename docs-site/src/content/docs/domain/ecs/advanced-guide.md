---
title: Advanced Guide
description: Engine-agnostic advanced guide for RunenECS.
status: active
owner: runen-ecs
layer: domain
canonical: true
last_reviewed: 2026-09-10
---

# RunenECS Advanced Guide

For day-to-day usage, start with [usage-guide.md](usage-guide.md). Internal
invariants are documented in [architecture.md](architecture.md).

## Deferred commands and visibility

`Commands` queues structural mutations for the next ECS deferred-apply
boundary. Queues are collected in deterministic serial execution order and
are discarded if the system or command application fails. A system that must
observe a queued mutation in the same schedule run should be placed after the
producer with an explicit system-set ordering relation.

Access incompatibility is separate from semantic ordering and does not create
an extra visibility boundary.

## Ordering

`in_set`, `before`, and `after` express semantic precedence. `ScheduleLabel` and
`SystemSet` are generic ECS identities. Ordering cycles are rejected during
schedule validation; otherwise unordered systems use deterministic registration
order in the serial reference executor.

The runtime exposes `DeferredApplyBoundary` for a host callback that needs to
run after a successful flush. Its index is only ECS deferred-apply progress,
not an application frame, render phase, network tick, or publication identity.

## Secondary indexes

Named indexes can accelerate typed component lookup:

```rust
use runen_ecs::prelude::*;

#[derive(Debug, Clone, PartialEq, Eq, runen_ecs::Component)]
struct Name(String);

let mut world = World::new();
world.ensure_component_index::<Name, String>(|name| name.0.clone());
let entity = world.spawn(Name("hero".to_owned())).unwrap();
assert_eq!(world.find_entity_by_index::<Name, String>(&"hero".to_owned()), Some(entity));
```

Indexes are ECS-owned caches and are dirtied by component changes. They do not
define persistence or network identity.

## Change observation

`Added<T>` and `Changed<T>` are query filters backed by ECS-local change state.
`World::current_change_tick` returns a `ChangeCursor` with an epoch and inner
tick. The cursor remains ordered across the inner counter boundary and the
runtime fails before reusing the absolute two-word position. It is not a host
frame or application revision.

## System parameters and safety

Use built-in `Res`, `ResMut`, `Query`, `Commands`, and `WorldMut`, or derive a
composed parameter with `#[derive(SystemParam)]`. `WorldMut` is the supported
exclusive whole-world parameter and cannot be combined with sibling world
borrows in the same system.

The low-level `SystemParam` trait is doc-hidden and manual downstream
implementations are unsupported. Its unsafe contract covers lifetime-independent
cached state, complete access declarations, and invocation-scoped extraction;
the maintained Miri suite exercises the retained query and extraction paths.

## Runtime errors

Callers can match `RuntimeError` for ECS-owned setup, schedule validation,
parameter, command, system, boundary, and invariant failures. User failures
remain boxed causes inside the system or boundary variants. RunenECS does not
make `anyhow` part of its dependency or public error contract.

## Out of scope

RunenECS does not provide generic event channels, process-global telemetry,
unbounded change journals, application lifecycle phases, rendering, replay,
networking, or product-publication policy.
