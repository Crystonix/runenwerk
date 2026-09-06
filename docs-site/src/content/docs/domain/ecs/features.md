---
title: Feature Map
description: Current capability map for the ecs crate.
status: active
owner: ecs
layer: domain
canonical: true
last_reviewed: 2026-09-06
---

- ✅ Core-supported
- ⚠ Partial / constrained support
- ❌ Missing

| Area | Capability | Status | Notes |
| --- | --- | --- | --- |
| **Derives** | `Component` | ✅ | Stable derive and runtime registration flow. |
|  | `Resource` | ✅ | Stable derive and resource lifecycle APIs. |
|  | `Bundle` | ✅ | Supported for spawn/insert/remove composition. |
|  | `StatefulComponent` | ✅ | Generation/version state is available and explicit-change APIs are implemented. |
|  | `Event` derive (`#[derive(Event)]`) | ❌ | Event channels are typed but event registration derive is not present. |
| **World Core** | Entities (`spawn`, `despawn`, `entity`, `entity_mut`) | ✅ | Core lifecycle is stable with archetype-backed storage. |
|  | Resources (`insert_resource`, `resource`, `resource_mut`, `remove_resource`) | ✅ | Includes change-log reporting APIs. |
|  | Change logs (`component_changes_since`, `resource_changes_since`) | ✅ | Reporting layer is separate from query filter semantics. |
| **Queries** | `Query<&T>`, `Query<&mut T>`, tuples, optional forms | ✅ | Core query surface is in place. |
|  | Filters `With`, `Without`, `Added`, `Changed` | ✅ | Filter semantics backed by archetype metadata ticks. |
|  | Reusable `QueryState<Q, F>` | ✅ | Detached query state and cache reuse supported. |
|  | `QueryOrphaned<T>` / `QueryOrphanedState<T>` | ✅ | Removed-component stage window is supported. |
|  | Query history / undo-oriented query APIs | ❌ | Not part of current ECS scope. |
| **System Params** | `Res<T>`, `ResMut<T>`, `ResView<T>` | ✅ | `ResView<T>` is a semantic alias for read-only resource access. |
|  | `Commands` param | ✅ | Deferred structural mutation param with runtime scope protection. |
|  | `BroadcastReader<T>`, `BroadcastWriter<T>` | ✅ | Typed read/write event params are implemented. |
|  | `BroadcastReader<T>` param (`iter_all`, `iter_new`, `send`) | ✅ | Per-system cursor state for incremental event reads. |
| **Commands / Runtime** | `Commands` queue + `apply` | ✅ | Deferred commands are collected and flushed at stage boundary. |
|  | `DeferredCommand<T>` | ✅ | Typed deferred command trait is available. |
|  | `BatchCommands` | ✅ | Ordered batched mutations are implemented. |
|  | Stage-failure command isolation | ✅ | Commands from failed runs are discarded, not replayed on later runs. |
|  | Conditional command DSL (`ConditionalCommands`) | ❌ | No dedicated conditional command primitive. |
| **Events / Reactivity** | World events (`publish_broadcast`, `read_broadcast`, `drain_broadcast_admin`, `clear_broadcast_admin`) | ✅ | Typed event channels with world APIs are implemented. |
|  | Channel config (`BroadcastStreamConfig`) | ✅ | Capacity, overflow, lifetime, tracing policy are supported. |
|  | Event observers (`observe_events`, triggers, notifications) | ✅ | `OnEmit`, `OnDrain`, `EndOfFrame` observer triggers are available. |
|  | Drain helpers (`drain_events_map`, `drain_events_filter`) | ✅ | Helper transforms for consumed events are available. |
|  | Event history buffers / replay store | ❌ | No first-class retained event history abstraction. |
| **Indexes** | Component secondary indexes | ✅ | Named indexes keyed by `(component type, key type, name)`. |
|  | Multiple named indexes per component | ✅ | Supported via `ensure_component_index_named` / `find_*_by_index_named`. |
| **Telemetry** | Feature-gated telemetry (`reset`, `snapshot`) | ✅ | Runtime/query/scheduler/event instrumentation is available behind `telemetry`. |
|  | Dedicated query/event profiler APIs | ❌ | No separate high-level profiler subsystem yet. |

## Notes on Scope

Current ECS priorities are core runtime correctness, deterministic scheduling/flush semantics, and maintainable module boundaries (`world`, `commands`, `query`, `system`).

Editor-facing reflection, network replication derives, and history/undo primitives are intentionally out of scope for this phase.

## Related Runtime Audit

For the current ECS + multiplayer runtime capability audit and prioritized sequencing,
see:

- [../../net/ecs-runtime-feature-inventory.md](../../net/ecs-runtime-feature-inventory.md)
- [../../net/ecs-runtime-gap-summary.md](../../net/ecs-runtime-gap-summary.md)
- [../../net/ecs-runtime-prioritized-roadmap.md](../../net/ecs-runtime-prioritized-roadmap.md)
