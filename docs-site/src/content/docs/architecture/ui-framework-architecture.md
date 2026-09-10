---
title: Runenwerk UI Framework Architecture
description: Canonical top-down architecture for Runenwerk's source-backed, backend-neutral, story-proven UI framework, including authoring frontends, UiProgram, runtime/proof flow, host adapters, and SDF/render targets.
status: active
owner: ui
layer: architecture
canonical: true
last_reviewed: 2026-09-10
related_docs:
  - ../domain/ui/README.md
  - ../domain/ui/architecture.md
  - ../domain/ui/roadmap.md
  - ../design/active/ui-framework-app-integration-direction-review.md
  - ../design/active/ui-program-architecture.md
  - ../design/active/ui-program-architecture-owner-map.md
  - ../design/active/runenwerk-ui-story-driven-golden-workflow-design.md
  - ../design/active/ui-runtime-rendering-pipeline-roadmap.md
  - ../design/deferred/game-runtime-ui-projection-and-hud-platform-design.md
  - ../design/deferred/ui-model-multiple-execution-strategies-design.md
  - domain-authoring-platform-overview.md
  - ../adr/accepted/0009-ui-interaction-formation-v2.md
---

# Runenwerk UI Framework Architecture

## Purpose

This is the canonical top-down architecture spine for the Runenwerk UI
framework.

It explains how authored UI source, interaction formation, `UiProgram`,
runtime/evaluator artifacts, story proof, host adapters, and render/projection
targets fit together. It does not replace the current-state UI domain
architecture, active slice designs, planning records, ADRs, or implementation
contracts.

## Architecture statement

Runenwerk UI is a source-backed, backend-neutral, story-proven UI framework.

The framework layer is not "SDF-backed UI" at the semantic layer. SDF is an
important render/projection target, especially for game HUD and world-space
work, but it does not own UI identity, interaction semantics, route/action
mapping, source truth, story truth, or app mutation.

The canonical flow is:

```text
Authoring frontend
  -> ui_definition source IR
  -> NormalizedUiTemplate
  -> FormedInteractionModel
  -> UiProgram
  -> UiCompiler / UiRuntimeArtifact
  -> UiEvaluator / runtime output
  -> UiStoryRunReport
  -> mount eligibility
  -> host adapters and render/projection targets
```

`UiProgram` is the semantic program boundary. Runtime artifacts, evaluator
output, render frames, SDF projections, static mounts, and host-side effects are
derived from that boundary and its source/proof chain.

## Non-goals

This architecture spine does not authorize:

- product Rust code changes;
- public `AppUiExt` APIs;
- a new public app framework surface;
- compiled-reactive or Svelte-like UI implementation;
- ECS-driven UI as an implemented semantic model;
- SDF-owned UI semantics;
- renderer-owned UI semantics;
- app/editor/game mutation from generic UI controls;
- `foundation/meta` or shared plugin framework extraction;
- moving ADR 0009, deferred execution-strategy ideas, or planning truth into
  this file.

## Source-of-truth order

Use this order when UI architecture documents overlap:

1. Accepted ADRs win for accepted decisions.
2. This architecture spine explains the full top-down UI framework.
3. Active UI design docs own slice-level target designs and tradeoffs.
4. `../domain/ui/architecture.md` records current code truth and current
   ownership boundaries.
5. `../domain/ui/roadmap.md` and workspace planning files record execution
   sequencing.
6. Narrow polish docs, reports, spikes, and deferred designs are supporting
   evidence unless promoted by accepted authority.

This file removes the need for every design and planning file to restate the
whole framework architecture.

## Layer model

The UI framework layers are:

| Layer | Owner | Role |
|---|---|---|
| Authoring frontends | UI/app/editor/game/designer owners | Provide ergonomic source authoring, never durable runtime truth by themselves. |
| UI source IR | `ui_definition` | Own `AuthoredUiTemplate`, `UiNodeDefinition`, slots, validation, normalization, and source maps. |
| Interaction formation | `ui_definition` / Interaction V2 contracts | Own `FormedInteractionModel` before retained or alternate execution. |
| Semantic program | `ui_program` | Own `UiProgram`, route/event contracts, graph facts, source maps, schema/capability facts. |
| Runtime artifacts | `ui_compiler`, `ui_artifacts` | Produce derived executable artifacts and manifests from `UiProgram`. |
| Evaluation | `ui_evaluator`, current retained runtime where applicable | Produce deterministic output, event packets, diagnostics, and reports. |
| Story proof | `ui_story`, `ui_testing` | Prove source, program, runtime, interaction, host, render, and mount facts. |
| Host adapters | editor/game/ECS/headless/world-space owners plus UI host contracts | Map event packets to host-owned actions, commands, mutation, effects, and IO. |
| Render/projection targets | render/SDF/static/world-space owners | Consume derived output only. |

## Authoring frontends

Authoring frontends are ergonomic entry points. They are not separate semantic
truth systems.

Each frontend must normalize to `ui_definition` source IR, normally
`AuthoredUiTemplate`, `UiNodeDefinition`, slots, bindings, and source-map
records. Each frontend must then lower through `FormedInteractionModel` and
`UiProgram` before runtime/proof/mount claims.

Supported and intended frontend shapes include:

| Frontend | Intended role | Required lowering rule |
|---|---|---|
| Rust `UiBuilder` screen/component function | Ergonomic code-authored UI for app/plugin/editor/game authors. | Builder output becomes `AuthoredUiTemplate` / `UiNodeDefinition` source records and source maps. |
| RON/authored template | Checked-in or external authored source. | Parsed source becomes `ui_definition` template/node records with validation and normalization. |
| Visual Designer output | Product authoring surface for UI source. | Designer saves or exports source IR, not renderer primitives or app mutation logic. |
| Svelte-like compiler DSL | Future ergonomic compiler frontend. | Deferred until accepted design/ADR; compiled output must still become source/program contracts. |
| Immediate-mode adapter | Convenience adapter for local authoring patterns. | Must capture source records and route/action contracts instead of mutating directly. |
| Reactive/signals adapter | Convenience adapter for signal-driven authoring. | Must lower signal reads/writes into explicit bindings, route/event contracts, and source maps. |
| Game HUD frontend | Game-facing HUD authoring. | Must normalize to UI source/program contracts; game mutation stays in game host owners. |
| World-space UI frontend | Entity/world-attached prompt, label, or interaction authoring. | Must normalize to UI source/program contracts; anchors/projection/lifetime stay host-owned. |

## UI source IR

`ui_definition` owns the source IR:

- `AuthoredUiTemplate`;
- `UiNodeDefinition`;
- route, value, and collection slots;
- authored IDs distinct from runtime IDs, ECS entities, renderer handles, and
  shell session IDs;
- validation and normalization;
- source maps and diagnostics;
- retained compatibility formation where current production paths still need
  retained output.

Source IR is stable authoring truth. It must not store retained `WidgetId`
values, ECS entity IDs, renderer resources, app IO, provider state, concrete
command execution, or product mutation.

## Interaction formation

ADR 0009 is accepted authority for execution-neutral interaction formation.

Execution-neutral means `FormedInteractionModel`, `UiProgram`, route/event
contracts, source maps, and story reports are not owned by retained UI, ECS UI,
SDF UI, renderer code, editor shell, or app mutation code.

The accepted Interaction V2 spine is:

```text
NormalizedUiTemplate
  -> FormedInteractionModel
  -> retained UI formation today
  -> UiProgram / runtime artifacts as program proof matures
```

`FormedInteractionModel` owns generic popup, scroll, focus, menu sizing,
docking/drop-zone, chrome-slot, status overflow, input arbitration, and route
contracts before a concrete execution target consumes them.

## UiProgram semantic program

`UiProgram` is the semantic program boundary.

It is not an authoring frontend, retained tree, ECS entity set, renderer frame,
SDF field, story report, or app mutation layer. It is the durable UI-domain
program that connects source, graph facts, route/event contracts, source maps,
diagnostics, runtime artifacts, host compatibility, and proof.

`UiProgram` owns or coordinates:

- typed graph families such as control, layout, state, style, interaction,
  binding, visual, accessibility, and inspection graphs;
- `RouteId`, `RouteSchemaVersion`, `RouteCapability`, and `UiEventPacket`;
- source-map attachment points;
- package, schema, capability, route, and diagnostic identities;
- explicit package registry snapshots and control package facts;
- binding and host-data contracts without hidden app state access.

## Runtime, evaluator, and artifacts

Runtime and evaluator layers derive facts from `UiProgram` and its compiled
artifact boundary.

`UiCompiler` consumes program, package, schema, host profile, theme, route map,
binding shape, source, and policy inputs. It emits `UiRuntimeArtifact`
manifests and tables.

`UiEvaluator` consumes runtime artifacts plus host snapshots/input and emits:

- backend-neutral UI output such as `UiFrame`;
- `UiEventPacket` and route/action proposals;
- diagnostics;
- inspection reports;
- state and dirty/invalidation reports;
- proof traces.

Retained UI is the current production baseline. Current retained runtime output
and retained compatibility adapters remain valid until bounded proof slices
replace named surfaces under accepted gates.

## Story proof and mount eligibility

`UiStory` is the proof envelope for UI authoring, preview, validation,
inspection, runtime evaluation, rendering evidence, and mount eligibility.

A story proves the full source-to-mount chain:

```text
source load / parse
  -> definition validation / normalization
  -> interaction formation
  -> UiProgram formation
  -> compiler / artifact reports
  -> evaluator / runtime reports
  -> binding and host route reports
  -> layout/style/text/accessibility/interaction evidence
  -> backend-neutral render output
  -> static mount or preview proof
  -> mount eligibility verdict
```

A visible result is not success unless the required upstream story stages pass.
Mounting into editor, game, headless, SDF, or world-space hosts must be derived
from story proof for that host profile.

## Host adapters and mutation ownership

Hosts own mutation and effects.

Generic UI controls must not mutate app, editor, game, domain, ECS, renderer,
network, file, or provider state directly. They emit route/event packets and
derived output. Hosts decide whether those packets are accepted, rejected,
mapped, ratified, queued, or executed.

Host responsibilities include:

- ECS/app resource state and systems;
- editor command routing and project IO;
- game action routing and gameplay mutation;
- headless fixture policy and deterministic proof;
- world-space anchor, projection, lifetime, visibility, and data feed policy;
- route-map and capability policy;
- side effects and persistence.

`ui_app_integration` is currently a renderer-neutral proof bridge, but it is
ECS-host-specific and proof-local. It is not the final host-neutral app
framework and does not expose final public `engine::App` ergonomics.

App authors should eventually get ergonomic `AppUiExt`-style APIs for resources,
systems, screens, routers, and typed UI actions. Public `AppUiExt` remains
deferred to a separate accepted phase after the proof bridge and dependency
direction are validated.

## Render/projection targets, including SDF

Render and projection targets consume derived output only.

Valid targets include:

- backend-neutral `UiFrame`;
- raster renderer output;
- SDF UI/HUD target;
- static mount;
- `SpatialCanvas`;
- world-space projection;
- headless render/proof output.

SDF UI/HUD is a render/projection target. It must consume derived UI output such
as frames, primitives, mount/projection packets, or story-proven render data. It
must not own authored UI source identity, route/action mapping, package
semantics, `UiProgram` truth, story truth, app mutation, or host route policy.

Renderer/SDF targets must not own source identity, route/action mapping, package
semantics, story truth, or app mutation.

## Execution strategies

Execution strategy is below the source/program/proof contracts.

Current status:

- retained UI is the production baseline;
- retained compatibility and strangler adapters remain valid while named
  surfaces migrate through accepted proof gates;
- compiled-reactive or Svelte-like execution is deferred;
- ECS-driven UI execution is deferred;
- SDF/world-space UI is a target/projection consumer, not a semantic owner.

Compiled-reactive/Svelte-like and ECS-driven UI may become execution targets
only after a separate accepted design or ADR promotes them. If promoted, they
must consume `ui_definition`, `FormedInteractionModel`, `UiProgram`, route/event
contracts, source maps, and `UiStory` proof rather than replacing them.

## Current implementation status

Current code already contains substantial pieces of the target pipeline:

- `ui_definition` source IR, validation, normalization, source maps, and
  retained formation;
- Interaction V2 retained-production contracts from ADR 0009;
- `ui_program`, graph families, route/event packet contracts, and source maps;
- `ui_program_lowering`, `ui_compiler`, `ui_artifacts`, `ui_evaluator`, and
  `ui_runtime_view` proof/runtime flow;
- `ui_controls` package-backed reusable control contracts;
- `ui_binding` and `ui_hosts` host/binding contract vocabulary;
- `ui_story`, `ui_testing`, headless/static mount/render proof crates;
- retained UI runtime and renderer-facing `UiFrame` output;
- `ui_app_integration` as the narrow ECS-backed proof bridge.

Current limitations:

- the public host-neutral app framework is not complete;
- public `AppUiExt` ergonomics are deferred;
- `ui_app_integration` is proof-local and ECS-host-specific;
- SDF/game/world-space targets must remain consumers until separate target
  proof promotes their exact contracts;
- compiled-reactive and ECS-driven execution strategies remain deferred.

## Roadmap relationship

This architecture spine is the canonical top-down architecture summary, not
active planning.

Workspace planning owns current focus, milestones, production-track blockers,
and state transitions. Active designs own slice-level tradeoffs. This spine
should be updated only when the canonical top-down model changes, not for every
phase status update.

Current focus is owned by workspace planning. As of the PR #75 closeout, the
ECS-backed Counter UI Story Proof is completed evidence and the active focus is
PR #74 / `PT-UI-RUNTIME-PLATFORM-001` intake review and hardening. That intake
does not authorize Live `UiPlugin` runtime implementation, compiler DSLs, SDF
UI, `SpatialCanvas`, public `AppUiExt` code, generic plugin framework
extraction, or alternate execution strategies.

## Folder and crate ownership map

| Responsibility | Primary owner |
|---|---|
| Architecture spine | `docs-site/src/content/docs/architecture/ui-framework-architecture.md` |
| Current UI code truth | `docs-site/src/content/docs/domain/ui/architecture.md` |
| UI execution sequencing | `docs-site/src/content/docs/domain/ui/roadmap.md` and workspace planning |
| UI source IR | `domain/ui/ui_definition` |
| Interaction formation | `domain/ui/ui_definition` and ADR 0009 contracts |
| Semantic program | `domain/ui/ui_program` |
| Program lowering | `domain/ui/ui_program_lowering` |
| Compiler and artifacts | `domain/ui/ui_compiler`, `domain/ui/ui_artifacts` |
| Evaluation/runtime views | `domain/ui/ui_evaluator`, `domain/ui/ui_runtime_view`, current `ui_runtime` |
| Reusable controls | `domain/ui/ui_controls` |
| Host and binding contracts | `domain/ui/ui_hosts`, `domain/ui/ui_binding` |
| Story/proof | `domain/ui/ui_story`, `domain/ui/ui_testing` |
| Render data | `domain/ui/ui_render_data`, `domain/ui/ui_render_primitives`, `domain/ui/ui_static_mount` |
| ECS-backed proof bridge | `domain/ui/ui_app_integration` |
| Editor host mutation/effects | `domain/editor/*`, `apps/runenwerk_editor` |
| Game/SDF/world-space target proofs | Owning game/runtime/render/world-space designs and future accepted contracts |

## Stop conditions

Stop and redesign if a follow-up tries to:

- make SDF the semantic owner of UI;
- make renderer output, SDF fields, or raster primitives source truth;
- make ECS entities the durable UI semantic model;
- make `ui_app_integration` the final host-neutral framework;
- expose public `AppUiExt` before a separate accepted API phase;
- let generic controls mutate host/app/editor/game/domain state directly;
- bypass `ui_definition`, `FormedInteractionModel`, `UiProgram`, or `UiStory`;
- promote compiled-reactive or ECS-driven execution from the deferred design
  without a separate accepted design or ADR;
- move app/editor/game semantics into `domain/ui`;
- move route/action mapping, source identity, package semantics, story truth,
  or mutation into renderer/SDF targets;
- turn root docs into long architecture manuals;
- duplicate the whole framework architecture in active design or planning files.

## Diagram

Diagram source:
[ui-framework-architecture.puml](./diagrams/ui-framework-architecture.puml).

## Related docs

- [UI Domain](../domain/ui/README.md)
- [UI Domain Current-State Architecture](../domain/ui/architecture.md)
- [UI Substrate and Surface Roadmap](../domain/ui/roadmap.md)
- [UI Framework App Integration Direction Review](../design/active/ui-framework-app-integration-direction-review.md)
- [UI Program Architecture](../design/active/ui-program-architecture.md)
- [UI Program Architecture Owner Map](../design/active/ui-program-architecture-owner-map.md)
- [Runenwerk UI Story Driven Golden Workflow Design](../design/active/runenwerk-ui-story-driven-golden-workflow-design.md)
- [UI Runtime Rendering Pipeline Roadmap](../design/active/ui-runtime-rendering-pipeline-roadmap.md)
- [Game Runtime UI Projection And HUD Platform](../design/deferred/game-runtime-ui-projection-and-hud-platform-design.md)
- [UI Model Multiple Execution Strategies Design](../design/deferred/ui-model-multiple-execution-strategies-design.md)
- [ADR 0009: UI Interaction Formation V2](../adr/accepted/0009-ui-interaction-formation-v2.md)
