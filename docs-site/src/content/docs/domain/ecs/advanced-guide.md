---
title: Advanced Guide
description: Engine-agnostic guide for ecs usage.
status: active
owner: ecs
layer: domain
canonical: true
last_reviewed: 2026-09-10
---

# ECS Advanced Guide

Audience: advanced users, runtime integrators, and users extending ECS behavior.

For normal day-to-day ECS usage, start with [usage-guide.md](usage-guide.md).
For internal implementation invariants, see [architecture.md](architecture.md).

## 1. Deferred Commands and Boundary Visibility

`Commands` are deferred structural mutations.

Runtime rule:

1. each system run gets its own command queue
2. queues are collected in deterministic system execution order
3. queues are applied at an ECS deferred-apply boundary after the current execution-plan ordering level
4. an Engine/runtime integration callback may run only after that flush completes

Systems that execute before the same deferred-apply boundary do not see one another's queued structural changes. If one system queues `spawn` / `insert` / `remove` and another must observe that change in the same schedule run, express semantic ordering that places the observer after the required deferred-apply boundary.

Access incompatibility does **not** create semantic precedence or an additional deferred-apply boundary. Otherwise unordered conflicting systems remain semantically unordered; the reference executor runs them serially in deterministic registration order and reports the conflict independently.

## 2. Runtime Ordering and Configuration

`SystemConfigExt` enables explicit ordering:

- `in_set(...)`
- `before(...)`
- `after(...)`

`ScheduleLabel` and `SystemSet` are RunenECS-owned public contracts and are available through `ecs::prelude::*`.

`Runtime::plan_for::<L>()` returns the compiled execution plan and is useful for inspecting current execution-stage shape in integration tests. `Runtime::plan_report_for::<L>()` provides ECS-neutral reporting for system/order/access diagnostics. Planner stage identity is diagnostic/planning shape; it is not the deferred-boundary, Engine lifecycle, or publication identity.

```rust
use ecs::prelude::*;

#[derive(Copy, Clone)]
struct Update;
impl ScheduleLabel for Update {
    fn name() -> &'static str { "Update" }
}

#[derive(Copy, Clone)]
struct Gameplay;
impl SystemSet for Gameplay {
    fn name() -> &'static str { "Gameplay" }
}

#[derive(Copy, Clone)]
struct PostGameplay;
impl SystemSet for PostGameplay {
    fn name() -> &'static str { "PostGameplay" }
}

fn produce(mut commands: Commands) {
    commands.spawn(());
}

fn observe() {}

let mut world = World::new();
let mut runtime = Runtime::new();
runtime.add_systems::<Update, _, _>(&mut world, produce.in_set(Gameplay));
runtime.add_systems::<Update, _, _>(&mut world, observe.in_set(PostGameplay).after(Gameplay));

let plan = runtime.plan_for::<Update>().unwrap().unwrap();
assert_eq!(plan.stages.len(), 2);
```

Explicit ordering cycles are rejected during schedule validation. Registration order is the deterministic tie-break/reference execution order for systems not separated by semantic ordering.

For host integration that must run after deferred ECS mutation becomes visible, `Runtime::run_schedule_with_deferred_apply_boundary` reports an ECS-neutral `DeferredApplyBoundary`. Its index identifies deferred-apply progress within that schedule run and is deliberately distinct from `ExecutionStage::index`.

## 3. Event and Message Transport Boundary

RunenECS does not currently expose a generic broadcast/event/channel transport API. The former broadcast stream, reader/writer, observer, and drain-helper surface was retired before this scheduling boundary was extracted and is not retained as compatibility API.

Message transport with a real maintained owner belongs with that owner rather than being reconstructed as generic ECS infrastructure. See [04-events.md](04-events.md) for the current boundary.

## 4. Advanced Secondary Index Usage

Beyond basic lookups:

- named indexes: `ensure_component_index_named<T, K>(name, extractor)`
- multi-hit lookups: `find_entities_by_index*`
- direct component lookup: `find_component_by_index*`

Operational note: indexes are lazily rebuilt and dirtied by component churn. Integration code can call lookup helpers from `&World`; rebuild mutation is internal via interior mutability.

## 5. Change Semantics Boundary

Two separate models exist and should not be conflated:

- Query/filter semantics (`Changed<T>`, `Added<T>`): archetype-row ticks, per-query last-seen tick state
- Reporting/introspection (`component_changes_since`, `resource_changes_since`, `*_changed_since`): world-level history views

Guideline: use query filters for gameplay/system behavior and history APIs for diagnostics/reporting.

## 6. Custom `SystemParam` Extension Path

Extension trait: `ecs::SystemParam`.

Required pieces are defined by the current `SystemParam` trait:

- lifetime-independent cached `State`
- `init_state` against `World`
- declared `QueryAccess`
- parameter metadata/slot description
- unsafe extraction through `SystemParamContext`

The safety contract is that extraction must obey the access facts declared by the parameter and must not create aliases beyond those facts. Cached state must remain valid across schedule runs and extraction lifetimes.

## 7. Telemetry Interpretation and Profiling Workflow

Enable telemetry:

```powershell
cargo bench -p ecs --bench phase6 --features telemetry -- --quick
cargo run -p ecs --example phase6_profile --features telemetry --release
```

Use telemetry counters/timers to separate:

- query iteration and filter cost
- schedule planning cost
- per-stage serial execution cost
- deferred command flush cost

Suggested workflow:

1. capture baseline snapshot
2. run targeted workload/benchmark
3. compare query/filter/runtime/flush counters
4. validate no semantic regressions with `cargo test -p ecs`

Do not infer semantic ordering from conflict counts or timing data. Access conflict diagnostics remain separate from the explicit ordering graph.

Related benchmark docs:

- [`benchmark-suite.md`](../../reports/closeouts/ecs-phase6/benchmark-suite.md)
- [`progress-report.md`](../../reports/closeouts/ecs-phase6/progress-report.md)
- [`final-decision-report.md`](../../reports/closeouts/ecs-phase6/final-decision-report.md)
