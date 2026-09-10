---
title: Typed App Program And UI Proof Design
description: Complete design draft for typed app-program model/action/reducer/effect/replay architecture with UI headless counter proof as the first proving slice.
status: active
owner: ui
layer: design
canonical: false
last_reviewed: 2026-08-04
related_docs:
  - ../../reports/investigations/typed-app-program-current-state-investigation.md
  - ../../guidelines/domain-program-architecture-pattern.md
  - ./runenwerk-domain-workbench-north-star.md
  - ./ui-program-architecture.md
  - ./ui-program-architecture-owner-map.md
  - ./runenwerk-typed-app-composition-plugin-framework-design.md
---

# Typed App Program And UI Proof Design

## Purpose

Define the long-term typed app-program architecture slice needed for ergonomic, inspectable, replayable app behavior without placing behavior inside `ui_definition`, `ui_controls`, renderer output, ECS storage, or a premature shared plugin framework.

This design uses UI as the first proving domain because `UiProgram`, `ui_hosts`, `ui_binding`, `ui_state`, `ui_evaluator`, and `ui_testing` already exist. It does not claim that UI permanently owns generic app semantics.

## Decision

Adopt this architecture direction:

```text
Typed App Program
  AppModelSnapshot
  AppAction
  RouteActionMap
  AppViewProjection
  AppReducer
  AppEffectPlan
  AppReplayTrace
  AppProgramReport

UiProgram
  UI semantic program, route/event packet, control/binding/state/output facts

Host
  host profile, host route compatibility, inert host/domain command or effect proposal
```

First implementation proof after this design is accepted:

```text
Headless Counter App Proof
```

The first proof must use existing UI program infrastructure and headless proof infrastructure as much as possible. It must not create a generic app/plugin framework, `foundation/meta`, product editor behavior, game behavior, or renderer-owned truth.

## Non-Authorization

This design does not authorize implementation by itself.

It also does not authorize:

```text
product code changes
crate creation
foundation/meta
shared plugin framework extraction
AppRecipe / PluginSuite implementation
Phase 17 SpatialCanvas implementation
editor command execution in domain/ui
game command execution in domain/ui
callback-first UI behavior
ECS-owned app/domain semantics
renderer-owned product truth
```

Implementation requires an owning GitHub issue and pull request under the repository executor contract in [`AGENTS.md`](../../../../../../AGENTS.md), naming exact files/crates, validation commands, evidence expectations, stop conditions, and module decomposition.

## Design Goals

1. Make small UI apps ergonomic without hidden callbacks.
2. Keep `ui_definition` and `ui_controls` behavior-free.
3. Reuse existing `UiProgram`, `UiEventPacket`, `ui_hosts`, `ui_binding`, `ui_evaluator`, and `ui_testing` infrastructure.
4. Preserve source-map, diagnostic, inspection, and replay evidence.
5. Keep app/domain mutation outside UI.
6. Support headless proof first.
7. Leave room for editor, game HUD, world-space UI, material/procgen, and visual UI designer consumers.
8. Block shared extraction until UI plus a non-UI proof expose repeated domain-neutral primitives.

## Design Non-Goals

```text
No callback architecture.
No hidden mutable state inside controls.
No global app store.
No universal event enum.
No universal graph runtime.
No generic plugin framework now.
No `foundation/meta`.
No material/procgen implementation.
No editor/workbench product integration in the first proof.
No game runtime or world-space runtime integration in the first proof.
```

## Architecture Spine

The target spine is:

```text
AppModelSnapshot
  -> AppViewProjection
  -> UiProgram / UiRuntimeArtifact / UiOutput
  -> UiEventPacket
  -> RouteActionMap
  -> AppAction
  -> AppReducer
  -> AppEffectPlan
  -> Host compatibility / host policy
  -> AppModelSnapshot revision N+1
  -> AppReplayTrace / AppProgramReport
```

For the first proof, `UiProgram` may be built directly from a proof builder or through existing lowering where reliable. The proof must not require visual editor integration or native windowing.

## Ownership Model

### Structural Ownership

| Structure | Owner For First Proof | Long-Term Owner Decision |
| --- | --- | --- |
| `UiProgram` graph/event/output semantics | `domain/ui` existing owners | Fixed UI owner. |
| `AppModelSnapshot` | proof-local app-program module/design-owned | Not final shared owner yet. |
| `AppAction` | proof-local app-program module/design-owned | Candidate cross-domain structure after second proof. |
| `RouteActionMap` | proof-local app-program module/design-owned, consumes `UiEventPacket` | Candidate shared structure later. |
| `AppReducer` | proof-local typed reducer contract | Domain/app-owned reducer semantics long-term. |
| `AppEffectPlan` | proof-local inert plan | May later reference `foundation/commands`; must stay inert. |
| `AppReplayTrace` | proof-local report contract | Candidate domain-neutral proof primitive after second proof. |
| Headless proof harness | `ui_testing` for first proof | Testing/proof only, not app semantics owner. |
| Host route compatibility | `ui_hosts` existing owner | Keep as UI host boundary. |

### Meaning Ownership

| Meaning | Owner | Rule |
| --- | --- | --- |
| Counter count/win semantics | proof-local counter app | Not UI/platform semantics. |
| Button/control semantics | `ui_controls` / `ui_program` | Controls declare event contracts, not app mutation. |
| Editor workspace mutation | `editor_shell` / app owner | UI can propose, not execute. |
| Game HUD/gameplay mutation | game/runtime owner | UI can project/emit, not execute. |
| World-space anchors/projection | world-space / viewport owners | App program only sees host compatibility later. |
| Material/procgen graph semantics | material/procgen domains | Later second-domain proof only. |

## Vocabulary

### Required New Vocabulary

```text
AppProgramId
AppProgramVersion
AppModelId
AppModelVersion
AppModelSnapshot
AppModelRevision
AppActionId
AppActionVersion
AppActionPayload
AppActionCapability
RouteActionMap
RouteActionMapping
RouteActionResolution
AppReducer
AppReducerInput
AppReducerOutcome
AppEffectPlan
AppEffect
AppReplayScenario
AppReplayStep
AppReplayTrace
AppProgramReport
AppViewProjection
AppViewProjectionReport
```

### Existing Vocabulary To Reuse

```text
UiProgram
UiProgramId
UiProgramVersion
UiEventPacket
RouteId
RouteSchemaVersion
RouteCapability
UiSchemaRef
UiSchemaValue
UiRuntimeArtifact
UiOutput
HostRouteMap
HostRouteMapping
HostCommand
DomainCommand
HostKind
HostDataSnapshot
UiEvaluationContext
UiStateModel
```

### Forbidden Durable Vocabulary For First Proof

```text
Callback
ClosureRoute
UniversalAction
GlobalStore
ServiceLocator
PluginRegistry
FoundationMeta
UniversalGraph
EcsAppModel
RendererState
EditorCommandExecutor
GameCommandExecutor
SpatialCanvasAction
```

These words may appear in rejected alternatives or stop conditions only. They must not become public API in the first proof.

## Data Model

### `AppProgramId`

Stable identity for a typed app-program proof.

Rules:

```text
non-empty
trimmed
namespaced
versioned separately
not a UI route id
not a host command id
```

Example:

```text
runenwerk.proofs.counter_app
```

### `AppModelSnapshot`

An immutable or cloneable snapshot of app-owned state at a revision.

Minimum fields:

```text
model_id
model_version
revision
values
diagnostics
```

For first proof, values may be schema-backed key/value records:

```text
counter.count -> integer
counter.screen -> "counter" | "win"
```

The first proof must not use `UiStateModel` as the app model. `UiStateModel` remains UI state.

### `AppAction`

A typed app event after route resolution.

Minimum fields:

```text
action_id
action_version
payload
required_capabilities
source_route
source_control
source_map
```

Examples:

```text
counter.increment
counter.reset
```

### `RouteActionMap`

Deterministic mapping from UI route events to app actions.

Input:

```text
UiEventPacket
HostRouteMap or compatible host route evidence
App route-action declarations
```

Output:

```text
RouteActionResolution
```

Resolution variants:

```text
Accepted(AppAction)
RejectedUnknownRoute
RejectedSchemaVersion
RejectedPayloadShape
RejectedMissingCapability
RejectedDisabledByHost
RejectedDiagnostics
```

All rejections must be reported and fail closed.

### `AppReducer`

Pure transformation from model/action to next model/effect plan.

Input:

```text
before_model
action
host_capabilities or compatibility facts
```

Output:

```text
AppReducerOutcome
```

Outcome fields:

```text
before_model
action
after_model
effect_plan
diagnostics
accepted
```

Reducer must not perform IO, renderer mutation, ECS mutation, or host command execution.

### `AppEffectPlan`

Inert description of requested host/domain effects.

First proof may use:

```text
NoEffect
```

But the structure must leave room for:

```text
HostCommandProposal
DomainCommandProposal
FocusRequest
NavigationRequest
AsyncRequest
```

Effect plans are proposals. They are not execution.

### `AppViewProjection`

Projection from app model to UI inputs.

For first proof:

```text
AppModelSnapshot
  -> UiProgram or UiProgram formation inputs
  -> HostDataSnapshot values
```

Projection report fields:

```text
source_model_revision
selected_screen
host_data_snapshots
program_id or artifact id
output_summary
diagnostics
```

### `AppReplayTrace`

Report for deterministic proof execution.

Required fields:

```text
scenario_id
program_id
initial_model
steps
final_model
diagnostics
passed
```

Each step records:

```text
step_index
input_event_packet
route_resolution
reducer_outcome
effect_plan
view_projection_report
ui_output_summary
```

## First Proof: Headless Counter App

### Scenario

The counter proof starts at:

```text
count = 0
screen = counter
```

It renders:

```text
Clicked 0 / 5
Click me
```

Each accepted `counter.increment` action increments `count`.

When `count >= 5`, projection switches to:

```text
You win!
Reset
```

`counter.reset` returns to:

```text
count = 0
screen = counter
```

### Required Steps

```text
1. render initial counter screen
2. emit/resolve increment event
3. reducer count 0 -> 1
4. render count 1 / 5
5. repeat through count 5
6. render win screen
7. emit/resolve reset event
8. reducer count 5 -> 0
9. render counter screen
```

### Required Negative Cases

```text
unknown route rejected
wrong route schema version rejected
missing required capability rejected
invalid payload rejected
route with diagnostics rejected
reducer diagnostic fails report
projection diagnostic fails report
```

### Required Reports

```text
CounterAppReplayReport
RouteActionResolutionReport
CounterReducerTraceReport
CounterViewProjectionReport
CounterEffectPlanReport
UiProgramFormationReport or direct program-build report
UiCompilerReport
UiEvaluation output diagnostics
HeadlessFixture-style reproducibility evidence
```

## Feature Support Matrix

| Feature | First Proof | Later UI Proof | Later Cross-Domain Proof |
| --- | --- | --- | --- |
| App model snapshot | Required | Required | Required |
| App action identity/version | Required | Required | Required |
| Route-action mapping | Required | Required | Maybe domain-specific event mapping |
| Reducer trace | Required | Required | Required |
| Effect plan | Required as no-op/inert | Host/domain proposals | Domain-owned effect proposals |
| View projection | Required to UI | Required to UI/editor | Domain-specific projection |
| Headless replay | Required | Required | Required |
| Host compatibility | Required for headless | Editor/game/world later | Domain-specific host later |
| Source maps | Required where available | Required | Required |
| Editor integration | Not allowed | Future | Not relevant |
| Game HUD integration | Not allowed | Future pressure | Not relevant |
| World-space UI | Not allowed | Future pressure | Not relevant |
| Material/procgen proof | Not allowed | Not UI proof | Required before shared extraction |
| Shared plugin framework | Blocked | Blocked | Consider only after second proof |

## Ergonomics Target

Future ergonomic API may look like:

```rust
app_program("runenwerk.proofs.counter_app")
    .model(CounterModel { count: 0 })
    .view(counter_view)
    .route("counter.increment", CounterAction::Increment)
    .route("counter.reset", CounterAction::Reset)
    .reducer(counter_reducer)
    .scenario(counter_replay_scenario)
```

But this is an authoring convenience only. The durable records must be explicit:

```text
AppModelSnapshot
AppAction
RouteActionMap
AppReducerOutcome
AppEffectPlan
AppViewProjectionReport
AppReplayTrace
```

Callbacks are rejected as the durable contract. If later introduced, they must lower to typed actions/effects and be inspectable.

## Relationship To Existing Crates

### `ui_program`

Reuse:

```text
UiProgram
UiEventPacket
RouteId
RouteSchemaVersion
RouteCapability
InteractionGraph
BindingGraph
StateGraph
source maps
diagnostics
```

Do not add app model/reducer ownership here in the first proof.

### `ui_hosts`

Reuse host route compatibility and route resolution concepts.

Potential additions after design/implementation authorization:

```text
host compatibility facts for app action/effect plans
headless accepted route/action capabilities
```

Do not make `ui_hosts` execute effects.

### `ui_binding`

Reuse `HostDataSnapshot` and binding reports for model-to-UI projection.

Do not make `ui_binding` own app selectors or reducer semantics unless a later design proves it.

### `ui_state`

Reuse only as UI state during evaluation.

Do not make it generic app model state.

### `ui_testing`

Use for proof harnesses and replay fixtures.

Do not make it production semantic owner.

### `foundation/commands`

May be referenced by later effect plan design because it already provides inert command descriptors/proposals.

Do not make foundation route, validate domain meaning, or execute effects.

### `editor_shell`

May consume future app effects or host/domain command proposals later.

Do not include editor integration in first proof.

### `material_graph` / `procgen`

Use as second-domain pressure only. Do not implement now.

## Module Decomposition For First Implementation Proof

Exact file placement remains bounded by implementation planning. If implemented inside existing UI proof/test crates, expected decomposition should resemble:

```text
app_program/
  ids.rs
  model.rs
  action.rs
  route_action.rs
  reducer.rs
  effect.rs
  projection.rs
  replay.rs
  report.rs
  counter_fixture.rs
```

If implemented in existing crates without a new crate, acceptable candidates are:

```text
domain/ui/ui_testing/src/app_program/
domain/ui/ui_hosts/src/app_action_compatibility.rs
domain/ui/ui_runtime_view/src/app_projection_helpers.rs
```

However, final files must be decided by the implementation planning contract. This design does not authorize them.

Forbidden decomposition:

```text
one monolithic app_program.rs with model/action/reducer/effect/replay mixed together
callback execution inside ui_definition
callback execution inside ui_controls
app reducer inside ui_program graph model
app reducer inside ui_state
host effect execution inside ui_hosts
shared foundation app framework
```

## Validation Plan For Future Implementation

Minimum validation after code exists:

```bash
cargo validate
git diff --check
CI=true pnpm --dir docs-site build
cargo test -p ui_testing counter_app
cargo test -p ui_hosts route
cargo test -p ui_program event
cargo test -p ui_binding host_data
cargo test -p ui_evaluator
```

Exact focused commands must be narrowed during implementation planning based on changed crates.

Docs-only design validation before merge uses the same repository baseline:

```bash
cargo validate
git diff --check
CI=true pnpm --dir docs-site build
```

## Principle Compliance

### KISS

The first proof uses one small app and one host: headless counter. It avoids editor/game/world-space integration and avoids full app recipe/plugin extraction.

### DRY

Reuse `UiProgram`, `UiEventPacket`, `ui_hosts`, `ui_binding`, `ui_evaluator`, and `ui_testing`. Do not duplicate route/event/program infrastructure.

### YAGNI

Do not implement actors, async workflows, editor integration, game HUD, world-space UI, material/procgen, or plugin suites in the first proof.

### SOLID

Each contract has one owner: app model/action/reducer/effect/replay are separate from UI program/control/binding/host/evaluator contracts.

### Separation Of Concerns

UI emits semantic events and output facts. App reducers mutate app model snapshots. Hosts execute or reject effects outside pure UI crates.

### Avoid Premature Optimization

No runtime hot-path optimization until proof shows cost. Runtime artifacts remain derived, not source truth.

### Law Of Demeter

The app proof should communicate through explicit packets, maps, reports, and snapshots. It must not reach into renderer, editor, game, or ECS internals.

## Future-Use Pressure

### Editor / Workbench

Future editor integration should map app actions/effects to editor-owned command proposals and reducer-style mutations. UI must not execute workspace mutations.

### Game HUD

Future game HUD integration should project game-owned state into UI and emit game-owned command proposals. UI must not own health, inventory, gameplay rules, or ECS mutation.

### World-Space UI

Future world-space UI requires anchors, camera/projection consumption, entity association, visibility/culling/occlusion, and input policy. Typed app program may provide route/action/effect/replay structure only. World-space semantics remain external-owner.

### UI Designer / App Builder

Future visual builder can author model/action/view/projection/route/effect declarations. The visual builder UX is Designer/Workbench product work, not app-program core.

### Material / Procgen

Material and procgen are the likely second-domain proofs. They must prove reuse of structural app-program concepts without importing UI semantics.

### Actors / State Machines

Actors/state machines remain optional workflow machinery for complex long-lived flows:

```text
modal flows
async loading
tool sessions
drag/drop sessions
wizards
remote previews
```

They are not the default app model. The default remains MVU/reducer-style model/action/update/effect.

## Shared Extraction Rule

No shared app-program framework may be extracted from the first UI proof alone.

Extraction requires:

```text
UI proof passes
at least one non-UI proof passes
same domain-neutral primitive appears in both
primitive does not own domain meaning
accepted extraction design names exact owner/files
validation and migration implications are recorded
```

Potential extractable primitives later:

```text
stable app/action IDs
model/action schema references
replay trace shape
capability compatibility reports
effect proposal envelopes
```

Non-extractable without separate design:

```text
UI controls
material node semantics
procgen node semantics
editor commands
game commands
world-space anchors
renderer pass semantics
universal graph interpretation
universal compiler/evaluator behavior
```

## Stop Conditions

Stop and return to design if any implementation requires:

```text
new crate without exact owner decision
modifying active Phase 17 SpatialCanvas scope
moving app reducer into ui_definition
moving app reducer into ui_controls
moving generic app model into ui_state
executing host effects in ui_hosts
executing domain commands in foundation/commands
using callbacks as durable route contracts
using ECS as static app model source of truth
using renderer output as product truth
implementing AppRecipe/PluginSuite before second-domain proof
```

## Design Acceptance Checklist

Before implementation authorization, record:

```text
exact first-proof owner
exact files/crates
exact allowed dependencies
exact non-owned crates
route/action/effect vocabulary
projection strategy
reducer trace structure
negative-case requirements
validation commands
report names
closeout evidence requirements
rollback plan
```

## Recommended Next Action

After this design is reviewed and accepted, open a narrow implementation-planning contract for:

```text
Typed App Program UI Proof 001 — Headless Counter App Proof
```

The first implementation must be proof-first, not framework-first.

## Summary

Typed App Program is the right long-term direction only if it remains layered:

```text
UI owns UiProgram semantics.
App owns model/action/reducer/effect semantics.
Host owns execution policy.
Domains own meaning.
Platform owns structure.
Shared extraction waits for at least two proofs.
```
