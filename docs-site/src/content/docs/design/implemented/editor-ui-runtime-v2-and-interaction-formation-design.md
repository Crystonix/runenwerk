---
title: Editor UI Runtime V2 And Interaction Formation Design
description: Implemented design for execution-neutral Interaction V2 contracts formed before retained UI runtime execution.
status: implemented
owner: ui
layer: domain
canonical: true
last_reviewed: 2026-09-12
related:
  - ./ui-definition-formation-foundation-design.md
  - ./editor-self-authoring-and-final-ui-design.md
  - ../deferred/ui-model-multiple-execution-strategies-design.md
  - ../../adr/accepted/0009-ui-interaction-formation-v2.md
  - ../../domain/ui/roadmap.md
  - ../../apps/runenwerk-editor/roadmap.md
related_reports:
  - ../../reports/closeouts/wr-025-interaction-v2-doctrine-repair/closeout.md
---

# Editor UI Runtime V2 And Interaction Formation Design

## Status

Implemented. [ADR 0009](../../adr/accepted/0009-ui-interaction-formation-v2.md) remains the durable decision authority. WR-025 established and repaired the retained Interaction V2 contract spine, and all named retained slices have code-bearing contracts plus behavior evidence.

This document records the implemented boundary rather than an active migration plan. The former shell-polish record is historical evidence and the popup/adornment/drop-preview plan is superseded; neither is current interaction authority. Compiled-reactive UI, ECS-driven UI, and other alternate execution targets remain separately deferred and are not activated by this implementation status.

## Implemented Formation Boundary

```text
AuthoredUiTemplate / editor definition
  -> validation
  -> normalization / formation
  -> FormedInteractionModel
  -> formed retained UI product
  -> ui_runtime enforcement
  -> render/product-surface output
```

`FormedInteractionModel` is an execution-neutral interaction contract, not a second runtime. Renderer/product output remains derived state rather than UI authority.

## Ownership

`domain/ui/ui_definition` owns generic authored interaction vocabulary, validation, normalization, and formed interaction records that do not depend on editor commands or concrete app sessions.

`domain/ui/ui_runtime` owns retained runtime input routing, focus, scroll ownership, layout measurement, clipping, hit testing, popup behavior, and frame output for formed retained UI products.

`domain/editor/editor_definition` owns editor-specific authored descriptors. `domain/editor/editor_shell` adapts editor shell/composition state into formed interaction contracts. `apps/runenwerk_editor` owns app/runtime integration, concrete command execution, and viewport input fallback policy without becoming generic UI authority.

## Implemented Contract Families

The implemented `FormedInteractionModel` carries the retained Interaction V2 families used by current code:

- **menu stack/scopes** — stable popup scope, anchor, parent scope, dismissal, and focus-return behavior;
- **scroll ownership** — explicit owner/axis/boundary policy so UI ownership is distinct from whether scroll offset mutated;
- **menu sizing** — formed item-width and overflow behavior for clamped retained menus;
- **chrome slots** — structural close, active-indicator, label, command-area, and drag-region roles;
- **dock/drop zones** — candidate/active/invalid states, scope, side, priority, and preview-only semantics;
- **status/viewport arbitration** — explicit status overflow, metric priority, and UI-before-viewport input fallback policy.

These correspond to the stable WR-025 slice names `IV2-menu-stack`, `IV2-scroll-ownership`, `IV2-menu-sizing`, `IV2-chrome-slots`, `IV2-dock-drop-zones`, and `IV2-status-and-viewport-arbitration`.

## Current Code Anchors

Current implementation evidence includes:

- `domain/ui/ui_definition/src/interaction.rs::FormedInteractionModel` and its formed records;
- `domain/ui/ui_definition/src/validate.rs` interaction validation;
- `domain/ui/ui_definition/src/form.rs::form_retained_ui`;
- `domain/ui/ui_runtime/src/input/pointer.rs` for pointer dismissal and scroll ownership;
- `domain/ui/ui_runtime/src/input/hit_test.rs` for popup/chrome/drop-zone precedence;
- `domain/ui/ui_runtime/src/layout/engine.rs::layout_popup` for menu sizing;
- `domain/ui/ui_runtime/src/runtime/ui_runtime.rs::dispatch_keyboard_event` for keyboard dismissal behavior;
- `domain/editor/editor_shell/src/composition/` adapters for toolbar/menu, tab/chrome, dock/drop, and viewport status formation;
- `apps/runenwerk_editor/src/runtime/systems/input_bridge.rs` and viewport architecture guards for fail-closed scene fallback.

The exact implementation may continue to evolve; code and tests own current behavior while ADR 0009 and this implemented design own the durable decision and bounded contract shape.

## Guardrails

Any later retained UI change that extends these interaction areas must still identify:

```text
owning authored vocabulary
-> formed interaction record
-> retained formation adapter
-> ui_runtime enforcement point
-> shell/app boundary
-> regression guard
```

Do not recreate popup, scroll, docking, chrome, or viewport-input policy independently in app/shell code when the generic interaction contract owns it.

Compatibility-only paths must remain explicitly bounded and must not become a second interaction authority.

## Deferred Execution Targets

Compiled-reactive UI and ECS-driven UI are not implemented by Interaction V2. A future accepted design or ADR may add another execution target only if it consumes the same normalized authored UI and formed interaction semantics without replacing source identity, command ownership, or derived-renderer boundaries.

```text
NormalizedUiTemplate
  -> FormedInteractionModel
  -> retained product today
  -> possible additional accepted execution target later
```

## Non-Goals

- No alternate UI runtime activation.
- No external UI-framework adoption decision.
- No movement of editor command semantics into `domain/ui`.
- No rewrite of authored UI source identity, provider ownership, or command ratification boundaries.
- No claim that historical shell-polish or superseded popup documents are independent interaction authorities.

## Completion Evidence

The behavior-level doctrine repair and completion evidence is recorded in [WR-025 Interaction V2 Doctrine Repair Closeout](../../reports/closeouts/wr-025-interaction-v2-doctrine-repair/closeout.md). Current source/tests remain the authority for exact behavior.

<!-- BEGIN RUNENWERK:UI_COMPONENT_PLATFORM:interaction-consumption -->
## Component Platform interaction consumption

`PT-UI-COMPONENT-PLATFORM` consumes Interaction V2 by making generic interaction deterministic, replayable, inspectable, and story-proven across reusable controls and surfaces.

Required proof vocabulary includes hover, pressed, focus, keyboard activation, pointer capture, cancelled click, disabled non-activation, wheel ownership, scroll boundary consumption, popup outside dismiss, Escape dismiss, focus return, route/capability decisions, host intent proposals, interaction trace replay, and post-interaction frame proof.
<!-- END RUNENWERK:UI_COMPONENT_PLATFORM:interaction-consumption -->
