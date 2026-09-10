---
title: Feature Map
description: Current capability map for the ecs crate.
status: active
owner: ecs
layer: domain
canonical: true
last_reviewed: 2026-09-10
---

- ✅ Core-supported
- ⚠ Partial / constrained support
- ❌ Missing / outside the current public contract

| Area | Capability | Status | Notes |
| --- | --- | --- | --- |
| **Derives** | `Component` | ✅ | Stable derive and runtime registration flow. |
|  | `Resource` | ✅ | Stable derive and resource lifecycle APIs. |
|  | `Bundle` | ✅ | Supported for spawn/insert/remove composition. |
|  | `StatefulComponent` | ✅ | Generation/version state is available and explicit-change APIs are implemented. |
|  | `Event` derive (`#[derive(Event)]`) | ❌ | Generic event-channel transport is not part of the current RunenECS public contract. |
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
|  | Generic event reader/writer params | ❌ | No current `BroadcastReader` / `BroadcastWriter` compatibility surface is retained. |
| **Commands / Runtime** | `Commands` queue + `apply` | ✅ | Deferred commands are collected and flushed at ECS deferred-apply boundaries. |
|  | `DeferredCommand<T>` | ✅ | Typed deferred command trait is available. |
|  | `BatchCommands` | ✅ | Ordered batched mutations are implemented. |
|  | Schedule-failure command isolation | ✅ | Commands from failed runs are discarded, not replayed on later runs. |
|  | Conditional command DSL (`ConditionalCommands`) | ❌ | No dedicated conditional command primitive. |
| **Scheduling** | Schedule labels and system sets | ✅ | Owned by RunenECS; explicit `before` / `after` relations define semantic ordering. |
|  | Access conflict facts | ✅ | Read/write incompatibilities are reported independently of semantic ordering. |
|  | Deterministic serial reference execution | ✅ | Systems execute deterministically; access conflicts do not create ordering edges. |
|  | Deferred-apply boundaries | ✅ | Deferred structural mutations become visible only after an ECS deferred-apply boundary. |
|  | Execution stages | ✅ | Current plan/report grouping for execution and diagnostics; stage identity is not host lifecycle or publication identity. |
| **Events / Reactivity** | Generic world event channels | ❌ | The retired broadcast/channel model is not part of current RunenECS. |
|  | Generic channel configuration / observers / drain helpers | ❌ | No compatibility surface for the retired C6 messaging APIs is retained. |
| **Indexes** | Component secondary indexes | ✅ | Named indexes keyed by `(component type, key type, name)`. |
|  | Multiple named indexes per component | ✅ | Supported via `ensure_component_index_named` / `find_*_by_index_named`. |
| **Telemetry** | Feature-gated telemetry (`reset`, `snapshot`) | ✅ | Runtime/query/schedule/command instrumentation is available behind `telemetry`. |
|  | Dedicated query profiler APIs | ❌ | No separate high-level profiler subsystem yet. |

## Notes on Scope

Current ECS priorities are core runtime correctness, deterministic scheduling/deferred-visibility semantics, and maintainable module boundaries (`world`, `commands`, `query`, `system`). Access compatibility is scheduling metadata; it does not imply semantic execution order. Current execution-stage indices are planning/diagnostic facts and must not be used as application publication identity.

Generic event/channel transport, editor-facing reflection policy, network replication derives, and history/undo primitives are not part of the current RunenECS public contract.

## Related Runtime Audit

For the current ECS + multiplayer runtime capability audit and prioritized sequencing,
see:

- [../../net/ecs-runtime-feature-inventory.md](../../net/ecs-runtime-feature-inventory.md)
- [../../net/ecs-runtime-gap-summary.md](../../net/ecs-runtime-gap-summary.md)
- [../../net/ecs-runtime-prioritized-roadmap.md](../../net/ecs-runtime-prioritized-roadmap.md)
