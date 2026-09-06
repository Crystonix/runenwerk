---
title: Architecture
description: Internal architecture and invariants for the ecs runtime.
status: active
owner: ecs
layer: domain
canonical: true
last_reviewed: 2026-09-06
---

# ECS Architecture

This document is internal-facing and describes `domain/ecs` runtime internals,
unsafe invariants, and behavior contracts.

For public API usage, see [usage-guide.md](usage-guide.md).
For advanced integration patterns, see [advanced-guide.md](advanced-guide.md).

## 1. Runtime Execution Model

`Runtime` executes registered function systems using cached `SystemParam` state.

- param state is initialized at registration time
- extraction happens each run via raw world pointers
- each system run owns an ephemeral deferred command owner

Supported function system arity is implemented up to 8 parameters.

## 2. Module Boundaries

Current ownership split:

- `world/*`: world state, lifecycle orchestration, world-facing APIs
- `commands/*`: deferred command abstraction, typed-erased queue, batching
- `query/*`: query/filter modeling and execution
- `system/*`: param extraction and scheduler runtime bridge

Boundary intent:

- avoid `world` becoming a dumping ground for subsystem internals
- keep command mechanics in `commands`

## 3. Scheduling and Access Model

`QueryAccess` is transformed into scheduler `SystemAccess` keys:

- component read/write keys
- resource read/write keys
- structural mutation key for deferred commands
- event read/write keys (modeled as resource-like by event type)

Planning combines:

- explicit set dependencies (`in_set`, `before`, `after`)
- access conflict analysis (read/write, write/write)

## 4. Deferred Command Contract

Deterministic ordering contract:

1. systems execute in stage order
2. command queues are collected in system execution order
3. queues are applied at stage end in that order

Failure atomicity contract:

- commands are staged only for successful system runs
- failed schedule runs discard deferred queues instead of replaying later

## 5. Query Engine Internals

`QueryState<Q, F>` stores:

- required and excluded component constraints
- access metadata
- per-query `last_run_tick`
- archetype-row/entity scratch state and optional fast cache

Execution path split:

- archetype-row path for dominant mutable shapes
- entity-list fallback path for remaining supported shapes

`Changed<T>` and `Added<T>` evaluate archetype row ticks against query-local last-seen tick.

## 6. Change Boundary

Two separate mechanisms intentionally coexist:

- query/filter semantics:
  - archetype row `added_tick`/`changed_tick`
  - APIs: `Changed<T>`, `Added<T>`
- reporting/history:
  - world-level change logs and tick maps
  - APIs: `component_changed_since`, `resource_changed_since`,
    `component_changes_since`, `resource_changes_since`

## 7. Event Subsystem Internals

Event channels are keyed by event `TypeId` and carry per-channel configuration, counters,
and pending event storage.

Core mechanics:

- emit path applies capacity/overflow policy
- drain path updates channel counters
- frame-finalization handles `FrameTransient` cleanup
- observer triggers emit on configured boundaries
- `BroadcastReader<T>` param state tracks per-system read cursor (`iter_new`)

## 8. Secondary Index Internals

Secondary component indexes:

- registered by `(component type, key type, name)`
- lazily rebuilt when marked dirty by component churn
- expose `&self` read APIs using interior mutability for cache rebuilds

## 9. Unsafe Boundaries and Required Invariants

Concentrated unsafe sites:

- `query/query_data_impls.rs`
- `query/traits_and_state.rs`
- `system/runtime.rs`
- `system/params.rs`
- `commands/commands.rs`

Required invariants:

- `SystemParam::State` is lifetime-independent
- world pointers used during extraction remain valid for call duration
- mutable query shapes do not alias mutably for same component type
- borrowed command queue forwarding always targets a live owner scope

Unsafe blocks in these files require local invariant comments and focused tests.

## 10. Telemetry Architecture

Feature-gated telemetry (`--features telemetry`) records hot-path cost attribution for:

- query matching/iteration/get/single
- changed/added filter checks
- runtime plan lookup, stage execution, stage-end flush
- scheduler planning/conflict checks and stage counts
- event read/write call and volume counters

Telemetry is observational and must not alter runtime semantics.
