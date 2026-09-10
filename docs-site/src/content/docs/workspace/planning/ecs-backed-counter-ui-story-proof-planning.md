---
title: ECS-backed Counter UI Story Proof Planning
description: Implementation-planning contract for the first real UI framework proof after the UI Framework App Integration Direction Review.
status: active
owner: ui
layer: workspace
canonical: false
last_reviewed: 2026-08-04
related_docs:
  - ../../design/active/ui-framework-app-integration-direction-review.md
  - ../../design/active/runenwerk-ui-story-driven-golden-workflow-design.md
  - ../../design/active/ui-runtime-rendering-pipeline-roadmap.md
  - ../../design/active/ui-program-architecture.md
  - ../../design/active/editor-ui-runtime-v2-and-interaction-formation-design.md
  - ../../adr/accepted/0009-ui-interaction-formation-v2.md
  - ../../domain/ui/roadmap.md
  - ../../reports/investigations/typed-app-program-ui-proof-001-superseded-planning-archive.md
---

# ECS-backed Counter UI Story Proof Planning

## Status

Planning status: active.

Implementation authorization: conditional on an owning GitHub issue and pull request under the repository executor contract in [`AGENTS.md`](../../../../../../AGENTS.md).

This document is the implementation-planning contract for the first real Runenwerk UI framework proof. It supports a later implementation branch only if the owning issue and pull request accept this scope and the implementation stays within the exact files, dependencies, tests, and stop conditions below.

This planning change remains docs-only.

## Purpose

Implement the first real UI framework proof:

```text
ECS-backed Counter UI Story Proof
```

The proof must demonstrate the intended framework loop:

```text
App/ECS host state
  -> UI screen/component source
  -> ui_definition validation/normalization
  -> FormedInteractionModel / UiProgram route-event facts
  -> runtime artifact/output
  -> UI input/event proposal
  -> typed app action
  -> app/ECS-owned mutation
  -> next UI output
  -> UiStory-style report evidence
```

The proof exists to validate framework usage, not to build a product app, game UI, editor feature, SpatialCanvas component, shared plugin framework, or general app-program runtime.

## Current source facts used by this contract

Current workspace facts:

```text
Cargo.toml already contains UI crates through ui_story.
No ui_app_integration crate exists yet.
```

Current UI app-facing substrate facts:

```text
ui_definition owns AuthoredUiTemplate and NormalizedUiTemplate.
ui_definition owns UiNodeDefinition variants including Column, Label, Button, and route slots.
ui_definition owns UiRouteSlotRef and related slot refs.
ui_program owns UiEventPacket, RouteId, RouteSchemaVersion, RouteCapability, payload, source-control, phase, source-map, and diagnostics.
ui_program_lowering exposes form_ui_program_report_from_node_with_registry_snapshot(...).
ui_controls exposes runenwerk_control_package() and ControlPackageRegistry/ControlPackageRegistrySnapshot.
ui_story is V2 story/proof contract surface and must not regress to old flat-stage APIs.
ui_testing already depends on ui_program, ui_compiler, ui_evaluator, ui_binding, ui_hosts, ui_accessibility, ui_geometry, ui_input, ui_state, and related proof/evaluation crates.
engine::App exposes plugin, system, resource, headless App construction, and runtime host methods, but its world is private.
```

Implications:

```text
1. Do not make engine depend on ui_app_integration in the first proof.
2. Do not make ui_definition, ui_program, or ui_runtime depend on engine/ECS.
3. The first public AppUiExt API is deferred because an external integration crate cannot safely assume engine internals.
4. The first proof should use ecs directly as the host-state proof substrate and reserve engine::App ergonomics for a later adapter slice.
5. The new integration layer may depend on lower UI crates and ecs, and may use ui_testing/ui_story as dev/test support.
```

## Authority

Primary authority:

```text
docs-site/src/content/docs/design/active/ui-framework-app-integration-direction-review.md
```

Supporting authority:

```text
docs-site/src/content/docs/domain/ui/roadmap.md
docs-site/src/content/docs/design/active/runenwerk-ui-story-driven-golden-workflow-design.md
docs-site/src/content/docs/design/active/ui-runtime-rendering-pipeline-roadmap.md
docs-site/src/content/docs/design/active/ui-program-architecture.md
docs-site/src/content/docs/adr/accepted/0009-ui-interaction-formation-v2.md
```

Historical pressure evidence:

```text
docs-site/src/content/docs/reports/investigations/typed-app-program-ui-proof-001-superseded-planning-archive.md
```

## Current planning relationship

```text
PT-UI-FRAMEWORK-APP-INTEGRATION-001 — completed direction decision through PR #70
PT-UI-FRAMEWORK-APP-INTEGRATION-002 — this implementation-planning contract
```

## Core direction to preserve

The framework direction is:

```text
ECS/App/Plugin-hosted app authoring
+ ui_definition-backed UI source
+ FormedInteractionModel / UiProgram contracts
+ ui_runtime / ui_evaluator runtime output
+ UiStory proof and mount eligibility
+ host/app-owned mutation
```

The first implementation proves the ECS-hosted part directly. Public App/Plugin extension ergonomics are deferred until after dependency direction and owner boundaries are proven.

This proof must not regress to any rejected direction:

```text
raw ECS-owned UI source of truth
manual app_program public framework
external-template-only first step
SpatialCanvas as app integration
callback-first generic UI mutation
renderer-owned UI semantics
```

## Selected implementation strategy

Selected strategy:

```text
C-internal first, then B-public later.
```

Meaning:

```text
Create a small UI-owned app-integration crate now.
Use it to prove an ECS-backed Counter UiStory-style proof through the existing UI source/program/runtime/proof stack.
Keep all registration/build APIs internal or proof-local for this first proof.
Do not add public engine::App extension methods in this first implementation.
After this proof validates boundaries, add an AppUiExt-style public ergonomics slice separately.
```

## New crate

Add a new workspace crate:

```text
domain/ui/ui_app_integration
```

Crate name:

```text
ui_app_integration
```

Crate role:

```text
A UI-owned bridge between ECS-hosted app state/action proof fixtures and the UI source/program/story proof pipeline.
```

The crate must remain small and proof-driven. It must not become a generic app framework, AppRecipe, PluginSuite, or engine subsystem.

## Workspace updates allowed

Allowed root workspace changes:

```text
Cargo.toml
```

Required root `members` addition:

```text
"domain/ui/ui_app_integration",
```

Recommended placement:

```text
after "domain/ui/ui_story"
```

Required `[workspace.dependencies]` addition:

```text
ui_app_integration = { path = "domain/ui/ui_app_integration" }
```

Recommended placement:

```text
after ui_story
```

## New crate Cargo contract

Create:

```text
domain/ui/ui_app_integration/Cargo.toml
```

Package:

```toml
[package]
name = "ui_app_integration"
version = "0.1.0"
edition = "2024"
publish = false
```

Allowed production dependencies:

```toml
[dependencies]
serde = { version = "1.0.228", features = ["derive"] }
ecs.workspace = true
ui_binding.workspace = true
ui_controls.workspace = true
ui_definition.workspace = true
ui_hosts.workspace = true
ui_program.workspace = true
ui_program_lowering.workspace = true
ui_schema.workspace = true
```

Allowed dev-dependencies:

```toml
[dev-dependencies]
ui_testing.workspace = true
ui_story.workspace = true
ui_evaluator.workspace = true
ui_compiler.workspace = true
ui_artifacts.workspace = true
```

Forbidden production dependencies:

```text
engine
editor_* crates
game/app crates
ui_testing
ui_story
ui_runtime
renderer backend crates
net crates
material_graph
procgen
foundation/meta
foundation/commands
```

Rationale:

```text
The bridge may depend on ecs to prove ECS-owned state/mutation without dragging engine::App or engine runtime internals into the UI crate.
The bridge may depend on ui_definition, ui_program, ui_program_lowering, ui_controls, ui_binding, ui_hosts, and ui_schema because it forms UI source/program/action bridge facts.
The bridge may use ui_testing/ui_story only in tests because they execute proof, not framework ownership.
```

## Exact implementation files allowed

Allowed implementation files:

```text
Cargo.toml
domain/ui/ui_app_integration/Cargo.toml
domain/ui/ui_app_integration/src/lib.rs
domain/ui/ui_app_integration/src/ids.rs
domain/ui/ui_app_integration/src/action.rs
domain/ui/ui_app_integration/src/screen.rs
domain/ui/ui_app_integration/src/source.rs
domain/ui/ui_app_integration/src/bridge.rs
domain/ui/ui_app_integration/src/host.rs
domain/ui/ui_app_integration/src/report.rs
domain/ui/ui_app_integration/src/proof.rs
domain/ui/ui_app_integration/tests/counter_ui_story_proof.rs
domain/ui/ui_app_integration/tests/counter_ui_story_fail_closed.rs
```

Allowed only if compile wiring proves necessary:

```text
domain/ui/ui_definition/src/lib.rs
domain/ui/ui_program/src/lib.rs
domain/ui/ui_hosts/src/lib.rs
domain/ui/ui_binding/src/lib.rs
```

Any change outside these files requires returning to planning unless it is a mechanical Cargo lock/update equivalent required by the chosen toolchain.

## Public API rule

The first implementation must not add public engine::App extension methods.

Forbidden in this first implementation:

```text
AppUiExt as stable public API
add_ui_action
add_ui_screen
add_ui_screen_router
engine::App impl block changes
engine prelude exports for UI app integration
```

Allowed internal/proof-local API shape:

```text
UiAppIntegrationProof
UiAppIntegrationProofBuilder
UiAppScreenRegistry
UiAppScreenDescriptor
UiAppScreenId
UiAppActionRegistry
UiAppActionDescriptor
UiAppActionId
UiAppRouteBinding
UiAppRouteResolution
UiAppRouteResolutionReport
UiAppHostSnapshot
UiAppMutationReport
UiAppIntegrationReport
```

These names are accepted as internal/proof-layer vocabulary. They do not become long-term user-facing API merely by existing in this proof crate.

## Module contracts

### `src/lib.rs`

Responsibilities:

```text
crate boundary docs
module declarations
minimal exports required by tests
no public AppUiExt API
```

Required module declarations:

```rust
pub mod action;
pub mod bridge;
pub mod host;
pub mod ids;
pub mod proof;
pub mod report;
pub mod screen;
pub mod source;
```

### `src/ids.rs`

Owns stable typed ID wrappers:

```text
UiAppScreenId
UiAppActionId
UiAppRouteBindingId
UiAppProofId
```

Rules:

```text
IDs must be non-empty.
Route/action IDs used for UI route facts must be namespaced.
Visible labels must not be durable identity.
Prefer constructor validation mirroring ui_program RouteId namespacing rules where route-facing.
```

### `src/action.rs`

Owns app-action registry/proof descriptors, not app mutation:

```text
UiAppActionDescriptor
UiAppActionRegistry
UiAppActionRegistryError
UiAppActionCapabilityRequirement
```

Rules:

```text
No callbacks.
No reducers.
No direct Counter mutation.
No engine command execution.
Map action descriptors to stable route/action facts only.
```

### `src/screen.rs`

Owns screen registry/proof descriptors:

```text
UiAppScreenDescriptor
UiAppScreenRegistry
UiAppScreenRegistryError
UiAppScreenRoute
UiAppScreenSelection
```

Rules:

```text
No renderer/window ownership.
No ECS entity UI source of truth.
A screen descriptor points to source-builder/proof source functions or produced source records.
```

### `src/source.rs`

Owns minimal code-authored source helper for the proof:

```text
UiAppSourceBuilder
UiAppSourceBuildReport
UiAppSourceNodeRef
UiAppSourceRouteRef
```

Required output:

```text
ui_definition::UiNodeDefinition or ui_definition::AuthoredUiTemplate
```

For the counter proof, it must produce equivalent source for:

```text
Counter screen:
  Column
    Label bound/rendered with count text
    Button with stable increment route slot

Win screen:
  Column
    Label "You win!"
    Button with stable reset route slot
```

Rules:

```text
No direct UiProgram graph construction from app code.
No raw ECS UI entity authoring.
No runtime widget-only source.
Source builder must preserve source/provenance facts sufficient for report assertions.
```

### `src/bridge.rs`

Owns route proposal to typed action resolution:

```text
UiAppRouteBinding
UiAppRouteBridge
UiAppRouteResolution
UiAppRouteResolutionStatus
UiAppRouteResolutionDiagnostic
UiAppResolvedAction
```

Required behavior:

```text
known route + valid schema + required capability -> resolved action
unknown route -> reject
wrong schema -> reject
missing capability -> reject
invalid payload -> reject
route diagnostic rejection -> reject
rejected route must not request mutation
```

Rules:

```text
Consume ui_program::UiEventPacket for route/event facts.
Do not invent a parallel event packet.
Do not store raw private payloads in report output.
Do not use visible button text as action identity.
```

### `src/host.rs`

Owns proof-local ECS host snapshot and mutation evidence:

```text
UiAppHostSnapshot
UiAppHostMutation
UiAppHostMutationStatus
UiAppHostMutationReport
```

For the counter proof, tests may define local fixture types:

```text
Counter
CounterAction
CounterScreen
```

Required behavior:

```text
Counter starts at 0.
Resolved Increment mutates 0 -> 1.
Resolved increments reach 5.
Count >= 5 selects Win screen.
Resolved Reset mutates to 0.
Rejected actions do not mutate Counter.
```

Rules:

```text
Use ecs Resource/World/Res/ResMut or direct ecs-backed proof host types.
Do not depend on engine::App in production.
Do not use wall-clock time or unseeded randomness.
Do not use scheduler order as proof truth.
```

### `src/report.rs`

Owns deterministic report structures:

```text
UiAppIntegrationReport
UiAppIntegrationStepReport
UiAppSourceReport
UiAppFormationReportSummary
UiAppRuntimeReportSummary
UiAppActionReport
UiAppMutationReport
UiAppProofDiagnostic
```

Required report facts:

```text
source identity
screen identity
control/action source-map reference
formation diagnostics
UiProgram route/event diagnostics
compiler/runtime artifact diagnostics
runtime view/output facts
input event facts
route proposal facts
host/app action resolution facts
Counter before/after snapshot facts
mutation owner facts
next-output facts
pass/fail summary
```

Rules:

```text
Stable ordering.
Distinct diagnostic namespaces.
Safe bounded payload summaries.
No raw private payloads.
No label-as-ID.
No mutation reported for rejected action.
```

### `src/proof.rs`

Owns proof orchestration:

```text
UiAppIntegrationProof
UiAppIntegrationProofBuilder
UiAppIntegrationProofRun
```

Required behavior:

```text
build source
validate/lower source through ui_program_lowering
use runenwerk_control_package registry snapshot
produce route/event packet for activation proof
resolve route/event through bridge
apply ECS-owned mutation only after successful resolution
rebuild/re-evaluate next source/output evidence
produce UiAppIntegrationReport
```

Rules:

```text
Do not call app reducer directly without route/event resolution.
Do not fake success if formation/runtime/report stages have diagnostics that should fail the proof.
Do not depend on renderer backend/windowing.
```

## Counter proof tests

### `tests/counter_ui_story_proof.rs`

Must assert positive path:

```text
initial Counter = 0
Counter screen output includes count text and increment route evidence
activation emits or constructs UiEventPacket from route evidence
bridge resolves increment action
ECS-backed mutation changes count 0 -> 1
repeat to count 5
Win screen selected
reset route resolves
ECS-backed mutation resets count to 0
Counter screen selected again
report pass/fail summary is pass
report contains source, formation, route, resolution, mutation, and next-output facts
```

### `tests/counter_ui_story_fail_closed.rs`

Must assert negative path:

```text
unknown route rejected
wrong schema version rejected
missing capability rejected
invalid payload rejected
disabled/unavailable route emits no resolved action
missing host/binding data reports diagnostics
rejected action does not mutate Counter
runtime input outside target emits no app action
report fails if mandatory source/formation/route/mutation/next-output stage is missing
no callback/direct mutation bypass exists
```

Implementation may combine these into one test file only if the test names remain distinct and the planning closeout explains why.

## UI source requirements

Counter source must lower through `ui_definition` and `ui_program_lowering`.

Required source facts:

```text
Authored or source-equivalent root node identity
Counter label node identity
Increment button node identity
Increment route slot identity
Win label node identity
Reset button node identity
Reset route slot identity
source/provenance entries for action controls
```

Allowed simplification:

```text
The first proof may use code-authored source helpers instead of external .ron story manifests.
```

Forbidden simplification:

```text
The first proof must not manually construct final UiProgram graph rows as the normal source path.
```

## Route/action mapping requirements

Required stable route IDs:

```text
counter.increment
counter.reset
```

Required action IDs:

```text
counter.increment
counter.reset
```

Required schema versions:

```text
RouteSchemaVersion::new(1)
```

Required capabilities:

```text
counter.action.increment
counter.action.reset
```

Allowed payload:

```text
unit/null payload with explicit schema reference
```

Rules:

```text
Route IDs must use ui_program::RouteId.
Schema versions must use ui_program::RouteSchemaVersion.
Capabilities must use ui_program::RouteCapability.
Route packets must use ui_program::UiEventPacket.
```

## Host/ECS requirements

The proof must use ECS-backed state in one of these accepted forms:

```text
ecs::World with Counter as Resource
or
minimal proof host using ecs Resource/World APIs
```

Forbidden for the first implementation:

```text
engine::App production dependency
engine::App extension API
engine scheduler/runtime changes
engine prelude changes
engine world field access or privacy changes
```

Rationale:

```text
engine::App currently exposes resource/system methods, but external crates cannot rely on private world internals. A public App integration API should be added only after this proof defines the safe bridge boundary.
```

## Story/proof requirement

The first implementation may not be able to emit the final future `UiStoryRunReport` shape directly if current ui_story V2 contracts need extension.

Accepted first proof report name:

```text
UiAppIntegrationReport
```

Required relationship to story proof:

```text
The report must be structured as a story-compatible proof envelope.
The implementation must document which fields are direct UiStory V2 facts and which are integration-layer proof facts.
The report must not claim full production mount eligibility unless ui_story mount eligibility is actually exercised.
```

Allowed dev/test usage:

```text
ui_testing and ui_story may be used in tests to compare/evaluate proof facts.
```

Forbidden claim:

```text
Do not claim full UiStory production readiness if the proof produces only UiAppIntegrationReport.
```

## Validation commands

Required before implementation PR merge:

```bash
cargo validate
git diff --check
CI=true pnpm --dir docs-site build
cargo test -p ui_app_integration
cargo test -p ui_app_integration --test counter_ui_story_proof
cargo test -p ui_app_integration --test counter_ui_story_fail_closed
cargo test -p ui_program event
cargo test -p ui_program_lowering
cargo test -p ui_definition
cargo test -p ui_controls control_package
```

If exact filters differ after implementation, the implementation PR must explain the substitution and show equivalent coverage.

Docs-only planning PR validation uses the same repository baseline:

```bash
cargo validate
git diff --check
CI=true pnpm --dir docs-site build
```

## Non-owned responsibilities

This proof must not own:

```text
editor shell commands
product/editor/game mutation beyond the local Counter fixture
renderer backend resources
windowing or OS event loop
networking or multiplayer
async effect lifecycle
asset IO or file IO
hot reload
localization system
UI Designer persistence
SpatialCanvas item semantics
NodeCanvas / PortGraphCanvas semantics
AppRecipe / PluginSuite / shared plugin framework
foundation/meta
foundation/commands execution behavior
```

## Stop conditions

Stop and return to design if implementation requires:

```text
raw ECS entities as durable UI source
callback-first mutation from generic UI controls
bypassing ui_definition validation/normalization
bypassing UiProgram route/event facts
bypassing story-compatible proof reports
new public App extension names without accepted API review
engine core depending on ui_app_integration
ui_definition depending on engine/ECS
ui_program depending on engine/ECS
ui_runtime executing app mutation directly
ui_hosts executing effects directly
spatial canvas or component-platform scope changes
reopening PR #69 as implementation foundation
app_program crate resurrection as framework foundation
renderer/windowing work
network/multiplayer work
async effects
hot reload
```

## Closeout evidence required

Implementation closeout must record:

```text
changed files and ownership check
new crate boundary check
production dependency check for ui_app_integration
proof that ui_testing/ui_story are dev/test-only if used
positive counter proof summary
negative fail-closed proof summary
report structures produced
validation commands and results
proof that no non-owned files/crates changed
proof that no public AppUiExt-style API was added
proof that no engine core/prelude/world privacy changes were made
principle compliance review
module decomposition review
follow-up decision: AppUiExt ergonomics slice, story-report integration slice, or hold
```

## Future follow-up after this proof

If this proof succeeds, the next slice may be:

```text
PT-UI-FRAMEWORK-APP-INTEGRATION-003 — AppUiExt Ergonomics Proof
```

That future slice may evaluate public methods like:

```text
add_ui_action
add_ui_screen
add_ui_screen_router
```

It must be a separate design/implementation contract.

## Final implementation decision

Proceed next with:

```text
Create ui_app_integration as a small UI-owned proof bridge.
Implement the ECS-backed Counter UiStory-compatible proof through internal/proof-local bridge APIs.
Do not add public AppUiExt ergonomics yet.
Do not merge app_program or SpatialCanvas into this proof.
```
