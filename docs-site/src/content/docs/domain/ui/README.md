---
title: UI Domain
description: Documentation index for Runenwerk UI substrate and surface semantics.
status: active
owner: ui
layer: domain
canonical: true
last_reviewed: 2026-09-10
---

# UI Domain

`domain/ui/*` owns reusable, engine-agnostic UI substrate contracts, authored
UI definition/formation contracts, retained runtime layers, and renderer-facing
UI frame products.

Runenwerk UI is source-backed, formation-driven, interaction-contract-first,
and currently retained-UI-backed:

```text
Authored UI / editor definitions
  -> validation / normalization
  -> formed interaction contracts
  -> formed retained UI product
  -> ui_runtime enforcement
  -> render/product-surface output
```

Renderer output is derived product data. It is not UI authority.

## Canonical Framework Architecture

Start with:

- [Runenwerk UI Framework Architecture](../../architecture/ui-framework-architecture.md)

This is the top-down architecture spine for source-backed UI, authoring
frontends, interaction formation, UiProgram, runtime/evaluator artifacts, story
proof, host adapters, and render/SDF targets.

This page remains the UI-domain landing page and current docs router.

## Source Of Truth Order

1. Accepted ADRs win.
2. Canonical architecture spine explains the full UI framework.
3. Active UI design docs define target slices.
4. `docs/domain/ui/architecture.md` records current code truth.
5. `docs/domain/ui/roadmap.md` records execution sequencing.
6. Narrow polish docs are supporting evidence only.

## Current UI Truth

- [UI Definition Usage](./ui-definition-usage.md)
- [UI Composition Usage](./ui-composition-usage.md)
- [Adaptive Composition Usage](./ui-adaptive-composition-usage.md)
- [Current Architecture](./architecture.md)
- [Roadmap](./roadmap.md)
- [Story Acceptance and Review Checklist](./story-acceptance-and-review-checklist.md)

## Story Workflow

- [Runenwerk UI Story Driven Golden Workflow Design](../../design/active/runenwerk-ui-story-driven-golden-workflow-design.md)
- [Runenwerk UI Platform Capability Roadmap](../../design/active/runenwerk-ui-platform-capability-roadmap.md)
- [Story Acceptance and Review Checklist](./story-acceptance-and-review-checklist.md)

## Formation / Source Model

- [UI Definition Formation Framework Design](../../design/implemented/ui-definition-formation-foundation-design.md)
- [Editor Self-Authoring and UI Workspace Design](../../design/implemented/editor-self-authoring-and-final-ui-design.md)

## Structural Composition

- [App-Neutral UI Composition Design](../../design/accepted/app-neutral-ui-composition-design.md)
- [ADR 0013: App-Neutral UI Composition Clean Cutover](../../adr/accepted/0013-app-neutral-ui-composition-clean-cutover.md)
- [UI Composition Usage](./ui-composition-usage.md)
- [Adaptive Composition Usage](./ui-adaptive-composition-usage.md)

`domain/ui/ui_composition` is the app-neutral structural authority for saved
composition definitions, ratified mutable structure, typed transactions,
structural history, explicit promotion, content liveness vocabulary,
deterministic linked persistence bundles, atomic generation activation, and
headless fixture contracts. It does not own adaptive projection, app-extension
meaning, storage-root policy, native windows, product content, or editor
semantics.

`domain/ui/ui_adaptive_composition` derives transient projection, reflow,
hit-test, preview, drag/resize-session, proposal, accessibility, and explicit
promotion-delta products from immutable composition snapshots. It cannot
mutate or persist canonical composition state.

Editor and Draw static structure now project through `ui_composition`. Legacy
editor workspace structure is read-only compatibility input until the governed
docking-runtime gate. `ui_surface` remains a temporary mapped compatibility
boundary; neither legacy area is a parallel target authority.

## Interaction / Runtime

- [Editor UI Runtime V2 and Interaction Formation Design](../../design/implemented/editor-ui-runtime-v2-and-interaction-formation-design.md)
- [ADR 0009: UI Interaction Formation V2](../../adr/accepted/0009-ui-interaction-formation-v2.md)
- [Editor UI Popup, Adornment, And Drop Preview Contract](../../design/active/editor-ui-popup-adornment-drop-preview-contract.md)

## Legacy Shell / Workspace / Tool Surfaces

- [Editor UI Workspace Tool Surface Architecture](../../design/active/editor-ui-workspace-tool-surface-architecture.md)
- [Editor Workspace Document Mode Panel Architecture](../../design/implemented/editor-workspace-document-mode-panel-architecture.md)

## Deferred Execution Targets

- [UI Model Multiple Execution Strategies Design](../../design/deferred/ui-model-multiple-execution-strategies-design.md)

## Interaction V2 Migration Spine

ADR 0009 makes Interaction V2 the shared guardrail for popup stack, scroll
ownership, focus, menu sizing, chrome slots, docking/drop-zone, status overflow,
and viewport input arbitration.

Every retained UI migration slice should flow through:

```text
definition vocabulary
  -> validation rule
  -> FormedInteractionModel record
  -> retained UI formation adapter
  -> ui_runtime enforcement
  -> editor/app guard
```

Narrow shell polish docs are supporting evidence. They do not own long-term UI
policy and cannot promote alternate execution targets.

The current retained UI slice catalog is:

- `IV2-menu-stack`
- `IV2-scroll-ownership`
- `IV2-menu-sizing`
- `IV2-chrome-slots`
- `IV2-dock-drop-zones`
- `IV2-status-and-viewport-arbitration`

Each slice is defined in
[Editor UI Runtime V2 and Interaction Formation Design](../../design/implemented/editor-ui-runtime-v2-and-interaction-formation-design.md)
and must be consumed as a contract by downstream retained UI work.

As of 2026-05-15, code-bearing retained slices have landed for
`IV2-menu-stack`, `IV2-scroll-ownership`, `IV2-menu-sizing`,
`IV2-chrome-slots`, `IV2-dock-drop-zones`, and
`IV2-status-and-viewport-arbitration`. The owning implementation entry points
are
`domain/ui/ui_definition/src/interaction.rs`,
`domain/ui/ui_runtime/src/input/pointer.rs::dispatch_pointer_event`,
`domain/ui/ui_runtime/src/input/hit_test.rs`,
`domain/ui/ui_runtime/src/layout/engine.rs::layout_popup`,
`domain/ui/ui_runtime/src/runtime/ui_runtime.rs::dispatch_keyboard_event`, and
the toolbar/tab-stack/dock-drop/status adapters in
`domain/editor/editor_shell/src/composition/`. WR-024 shell polish must still
cite the named Interaction V2 slice it consumes.

## Scope Boundary

`domain/ui` owns substrate/runtime contracts (`ui_tree`, `ui_runtime`,
`ui_widgets`), app-neutral structural composition (`ui_composition`), transient
adaptive mechanism (`ui_adaptive_composition`), temporary generic surface
compatibility contracts (`ui_surface`), and generic UI definition formation.
It does not own editor-shell product wording or app extensions, app runtime
wiring, provider behavior, app IO, native-window lifecycle, renderer execution
policy, or concrete command execution.

Authored UI definitions must not persist runtime `WidgetId`, ECS entity ids,
app IO, provider state, provider behavior, or command execution. Editor command
semantics stay in editor/app owners and enter UI products only through explicit
route slots and ratified command paths.

<!-- BEGIN RUNENWERK:UI_COMPONENT_PLATFORM:domain-ui-note -->
## UI Component Platform activation note

The active Component Platform roadmap is `PT-UI-COMPONENT-PLATFORM`: reusable, story-proven `ControlPackage` and surface maturity after `PM-UI-STORY-004`. The platform introduces reusable kernels for control packages, authoring, story proof, catalog/discovery, input/gesture/device, state/binding/host intent, theme/token styling, accessibility/focus, layout/container/virtualization, render/surface output, overlay/popup/layering, text, Surface2D, SpatialCanvas, NodeCanvas, PortGraphCanvas, ProgressionTreeView, TrackSurface/Timeline, transitions/effects, and adoption gates.
<!-- END RUNENWERK:UI_COMPONENT_PLATFORM:domain-ui-note -->
