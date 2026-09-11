---
title: Architecture
description: Internal architecture and invariants for the ecs runtime.
status: active
owner: runen-ecs
layer: domain
canonical: true
last_reviewed: 2026-09-10
---

# ECS Architecture

This document records the RunenECS runtime boundary consumed by Runenwerk,
including its unsafe invariants and behavior contracts. The implementation is
owned by the standalone `runen-ecs` repository; these pages are retained as
Runenwerk-facing integration guidance.

For public API usage, see [usage-guide.md](usage-guide.md).
For advanced integration patterns, see [advanced-guide.md](advanced-guide.md).

## 1. Runtime Execution Model

`Runtime` executes registered function systems using cached `SystemParam` state.

- param state is initialized at registration time
- extraction happens each run through `SystemParamContext`
- each system run owns an ephemeral deferred command owner
- systems within a current execution-plan stage execute serially in deterministic registration order
- queued commands flush before the corresponding ECS deferred-apply boundary is reported
- integration callbacks run only after the deferred flush succeeds

Supported function-system and tuple-registration arity is implemented and regression-tested through 16 entries.

## 2. Module Boundaries

Standalone RunenECS ownership split:

- `world/*`: world state, lifecycle orchestration, world-facing APIs
- `commands/*`: deferred command abstraction, typed-erased queue, batching
- `query/*`: query/filter modeling and execution
- `system/*`: system parameters, runtime execution, and public scheduling integration
- `scheduler/*`: crate-private ECS scheduling implementation for labels, access facts, registered systems, plan construction, and validation

The crate-private `scheduler` module is an implementation detail of standalone
RunenECS. It is not the former standalone generic `scheduler` package and does
not define host lifecycle policy.

Boundary intent:

- avoid `world` becoming a dumping ground for subsystem internals
- keep command mechanics in `commands`
- keep reusable ECS schedule semantics inside RunenECS
- keep Engine frame/render/publication/replay/network policy outside RunenECS

## 3. Scheduling and Access Model

`QueryAccess` is transformed into ECS-owned `SystemAccess` facts:

- component reads/writes
- removed-component reads
- resource reads/writes
- structural mutation facts for deferred commands
- exclusive world access

Schedule planning has two deliberately separate concerns.

### Semantic ordering

Semantic precedence is derived only from explicit system-set relations:

- `in_set`
- `before`
- `after`

The ordering graph is cycle-validated. Registration order is the deterministic tie-break/reference execution order for systems that remain otherwise unordered. The planner's internal stages are not public lifecycle or publication identity.

### Access compatibility

Read/write and exclusive-world conflicts are computed independently from the semantic ordering graph. A conflict records that two systems are incompatible for concurrent execution; it does **not** create an ordering edge, stage split, or deferred-command visibility boundary.

This separation is required so that changing access metadata cannot silently change observable ECS semantics.

## 4. Deferred Command Contract

Deterministic ordering contract:

1. systems execute according to validated semantic precedence
2. otherwise unordered systems use deterministic registration order in the serial reference executor
3. command queues are collected in system execution order
4. queued commands are applied in deterministic order at ECS deferred-apply boundaries
5. `DeferredApplyBoundary` is reported only after the corresponding flush succeeds

Visibility contract:

- systems that execute before the same deferred-apply boundary do not observe one another's deferred structural mutations
- explicitly ordered dependent work after the boundary observes mutations applied before that boundary
- access conflicts alone never introduce an extra visibility boundary
- `DeferredApplyBoundary::index()` identifies deferred-apply progress within the schedule run.

Failure atomicity contract:

- commands are staged only for successful system runs
- failed schedule runs discard deferred queues instead of replaying them later
- a failed command flush or deferred-boundary callback stops the schedule and clears pending deferred state

## 5. Query Engine Internals

`QueryState<Q, F>` stores:

- required and excluded component constraints
- access metadata
- per-query `last_run_tick`
- archetype-row/entity scratch state and optional fast cache

Execution path split:

- archetype-row path for dominant mutable shapes
- entity-list fallback path for remaining supported shapes

`Changed<T>` and `Added<T>` evaluate archetype-row ticks against query-local last-seen tick.

## 6. Change Boundary

Query/filter semantics use archetype-row `added_tick` / `changed_tick` values
and query-local observation state through `Changed<T>` and `Added<T>`. World
change positions are exposed only through `ChangeCursor` and lightweight
`*_changed_since` checks; RunenECS does not retain an unbounded change journal.
`ChangeCursor` crosses the inner counter boundary with an epoch and fails before
reusing the absolute two-word position.

## 7. Event / Message Boundary

RunenECS has no current generic event/channel transport subsystem. The retired broadcast stream, reader/writer, observer, work-queue, and tick-buffer facilities are not scheduling primitives and are not retained as compatibility APIs.

If a maintained event/message transport is required, its semantics must be defined by the subsystem that owns that transport rather than by a generic ECS scheduler layer.

## 8. Secondary Index Internals

Secondary component indexes:

- registered by `(component type, key type, name)`
- lazily rebuilt when marked dirty by component churn
- expose `&self` read APIs using interior mutability for cache rebuilds

## 9. Unsafe Boundaries and Required Invariants

Concentrated unsafe sites include:

- `query/query_data_impls.rs`
- `query/traits_and_state.rs`
- `system/runtime.rs`
- `system/params.rs`
- `commands/commands.rs`

Required invariants:

- `SystemParam::State` is lifetime-independent
- world references/pointers used during extraction remain valid for the extraction call
- mutable query shapes do not create mutable aliases for the same component storage
- borrowed command-queue forwarding always targets a live owner scope
- declared `QueryAccess` accurately describes extraction access

Unsafe blocks in these files require local invariant comments and focused tests.

## 10. Runtime error boundary

`RuntimeError` preserves framework-owned setup, schedule-validation,
system-parameter, command, system, boundary-callback, and invariant failures.
User system and boundary failures are boxed causes inside those categories;
the ECS runtime does not require an application error-reporting framework.
