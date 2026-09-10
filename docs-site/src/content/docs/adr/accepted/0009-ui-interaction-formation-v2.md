---
title: UI Interaction Formation V2
description: Decision to add execution-neutral interaction formation before retained UI execution.
status: accepted
owner: ui
layer: domain
canonical: true
last_reviewed: 2026-09-10
related:
  - ../../design/implemented/editor-ui-runtime-v2-and-interaction-formation-design.md
  - ../../design/implemented/ui-definition-formation-foundation-design.md
  - ../../design/deferred/ui-model-multiple-execution-strategies-design.md
---

# ADR: UI Interaction Formation V2

## Status

Accepted.

## Context

Runenwerk already has a compiler-inspired UI definition pipeline: authored
templates validate, normalize, and form retained UI products. The retained UI
runtime remains the accepted production path.

The recurring editor UI failures are not caused by the lack of a compiled UI
runtime. They come from missing shared interaction contracts in the accepted
retained UI path:

- popup and submenu stack ownership;
- scroll ownership and wheel-input boundary behavior;
- menu sizing and item stretch policy;
- focus scopes, dismissal, and escape handling;
- docking and drop-zone precedence;
- tab and workspace chrome anatomy;
- viewport input arbitration when UI overlays are active.

Local retained-UI patches solve individual symptoms but keep the interaction
rules distributed across editor shell providers, runtime layout behavior, and
app viewport input glue.

## Decision

Introduce execution-neutral interaction formation between normalized UI
definitions and retained UI products.

The first target remains retained UI:

```text
AuthoredUiTemplate
  -> validate
  -> normalize
  -> FormedInteractionModel
  -> FormedRetainedUiProduct
  -> ui_runtime enforcement
  -> render/product-surface output
```

`FormedInteractionModel` owns generic interaction contracts. It does not own
editor commands, provider state, runtime authority, app viewport state, or ECS
entities. It may later feed additional execution targets, but the default editor
path stays retained UI until a separate accepted ADR changes that default.
Renderer and product-surface output are derived products, not UI authority.

The initial Interaction V2 contract set is:

- popup stack and menu scope formation;
- scroll ownership and boundary-consumption policy;
- focus scope and dismissal policy;
- menu sizing and item fill policy;
- docking/drop-zone priority and preview policy;
- structural chrome slots for close affordances, active indicators, labels, and
  drag regions;
- status/metadata overflow policy for compact editor surfaces.

## Ownership Rules

`domain/ui/ui_definition` owns generic authored interaction vocabulary,
validation, normalization, source maps, and formed interaction outputs. It must
not mention editor commands, app runtime state, concrete `WidgetId` values, or
ECS entities.

`domain/ui/ui_runtime` owns retained runtime enforcement: layout, clipping, hit
testing, focus, scroll ownership, input ownership, and frame output for formed
retained UI products.

`domain/editor/editor_definition` owns editor-specific definitions that refer to
generic UI interaction contracts: menu descriptors, workspace chrome, command
bindings, panel registries, and tool-surface descriptors.

`domain/editor/editor_shell` owns shell composition and Strangler adapters that
map existing shell state into formed interaction contracts while migration is in
progress.

`apps/runenwerk_editor` owns app/runtime integration, viewport input
arbitration, file/project IO, fixture loading, concrete provider state, and
command execution.

## Migration Policy

Adopt Interaction V2 through a Strangler migration. Existing retained UI
composition remains live while popup, scroll, chrome, docking, and status
surfaces migrate to formed interaction contracts one slice at a time.

Each migrated slice must keep existing authored/source identity and command
ratification boundaries intact. Compatibility adapters are allowed only as
explicit migration code; they must not become a second interaction
architecture.

Each slice must define the same migration spine before implementation:

```text
definition vocabulary
  -> validation rule
  -> FormedInteractionModel record
  -> retained UI formation adapter
  -> ui_runtime enforcement
  -> editor/app guard
```

The old retained path may remain during migration only behind named
compatibility adapters. Those adapters must identify the target formed contract
and the guard that prevents the old behavior from expanding.

The canonical retained UI migration slices are:

- `IV2-menu-stack` for popup stack, menu scope, submenu anchors,
  outside-dismiss, escape, and focus return;
- `IV2-scroll-ownership` for nearest scroll owner, axis ownership, boundary
  consumption, scrollbar capture, and viewport fallback rejection;
- `IV2-menu-sizing` for intrinsic menu measurement, item fill, popup clamp, and
  scroll fallback;
- `IV2-chrome-slots` for close affordance, active indicator, label, command,
  and drag slots in tab and workspace chrome;
- `IV2-dock-drop-zones` for tab reorder, split insertion, floating-host,
  invalid-target, and preview-only drop-zone policy;
- `IV2-status-and-viewport-arbitration` for compact status overflow, essential
  metric priority, and viewport pointer/wheel arbitration.

These names do not add runtime behavior by themselves. They define the stable
contract references that downstream retained UI work must consume or cite as
explicit compatibility targets.

## Dependent Work Policy

Editor shell polish is downstream implementation evidence, not the durable
owner of interaction policy. Work such as popup contrast, scrollable menus,
Switch Type submenu anchoring, tab close placement, active indicators, docking
previews, or viewport statistics may proceed only as:

- a retained UI slice consuming an already defined Interaction V2 contract; or
- explicit compatibility evidence that records the old behavior while the
  target contract is being formed.

It must not introduce a second popup policy, app-local viewport arbitration
truth, compiled-reactive UI, ECS-driven UI, an external UI framework, or
renderer-owned UI semantics.

## Non-Decisions

This ADR does not:

- accept a compiled-reactive UI runtime;
- accept an ECS-driven UI runtime;
- accept an external UI framework replacement;
- move editor command semantics into `domain/ui`;
- make Interaction V2 a product implementation phase by itself.

## Rejected Alternatives

Promote compiled-reactive UI now. A compiled target would still need the same
scroll, focus, popup, layout, docking, and viewport arbitration rules. Promoting
it before those contracts exist would move ambiguity into a new runtime.

Adopt an external UI framework now. Slint, Iced, egui, GPUI, and similar
frameworks do not directly provide Runenwerk's source-backed editor
definitions, self-authoring, viewport product embedding, shell command
ratification, and docking semantics. A replacement evaluation may be useful
later, but it is not the shortest long-term fix for the current contract gaps.

Keep patching retained UI locally. That has already produced repeated
regressions because each surface re-solves popup, scroll, chrome, and docking
behavior differently.

## Consequences

Runtime work can proceed through a Strangler migration: new formed interaction
contracts are introduced and individual menus, scroll surfaces, chrome controls,
and docking zones migrate without rewriting the entire editor shell.

`domain/ui` gains the generic interaction vocabulary and runtime enforcement,
while `domain/editor` and `apps/runenwerk_editor` keep editor semantics,
provider state, viewport integration, and command execution.

Compiled-reactive UI and ECS-driven UI remain deferred. If they are promoted
later, they must consume normalized definitions and formed interaction contracts
instead of replacing authored UI identity, source maps, command ratification, or
retained UI's current production role.

Validation must cover the interaction contracts directly: definition validation,
runtime input/layout ownership, editor shell migration behavior, and app
viewport input arbitration. Roadmap rows should treat this ADR as the durable
decision gate for Interaction V2 and not as permission to introduce alternate UI
execution targets or renderer-owned UI semantics.

<!-- BEGIN RUNENWERK:UI_COMPONENT_PLATFORM:adr-consumption -->
## Component Platform consumption note

`PT-UI-COMPONENT-PLATFORM` consumes this ADR without changing its decision. The component platform makes the accepted interaction/canvas boundaries reusable and story-proven through `ControlPackage`s, `Surface2D`, `SpatialCanvas`, `NodeCanvas`, `PortGraphCanvas`, `ProgressionTreeView`, `TrackSurface`, Gallery proof, Workbench adoption, and UI Designer consumption. This ADR remains the authority for the underlying boundary; Component Platform docs define reusable package contracts and proof requirements.
<!-- END RUNENWERK:UI_COMPONENT_PLATFORM:adr-consumption -->
