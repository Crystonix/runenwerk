---
title: UI Component Platform Executable Interaction Story Design
description: Completed Tier 5 proof design reference for replay/live parity, proof-host sessions, static frame evidence, and no-bypass reusable UI interaction validation.
status: active
owner: ui
layer: domain
canonical: true
last_reviewed: 2026-09-12
related_docs:
  - ../../workspace/planning/roadmap.md
  - ../../workspace/planning/decision-register.md
  - ./ui-component-platform-generic-interaction-design.md
  - ../../reports/closeouts/pt-ui-component-platform-012a-executable-interaction-story-implementation-scope-closeout.md
  - ./ui-component-platform-overlay-popup-layering-design.md
  - ./ui-component-platform-story-proof-envelope-design.md
  - ./ui-component-platform-input-gesture-device-design.md
  - ./editor-ui-runtime-v2-and-interaction-formation-design.md
---

# UI Component Platform Executable Interaction Story Design

## Status

Lifecycle state: `completed`.

Planning ID: `PT-UI-COMPONENT-PLATFORM-012A`.

Accepted by user review on 2026-06-29. Implemented and merged through PR #43 on 2026-06-30 at merge commit `c8b73dfa95fc335fd2b33c9137cac03a0f35060f`. User start condition for Phase 13 reports PR #43 was validated and merged.

This document is the completed Tier 5 proof design reference for reusable UI interaction work. The exact completed owner files, validation gate, evidence expectation, and stop conditions are retained in the [historical Phase 12A implementation-scope closeout](../../reports/closeouts/pt-ui-component-platform-012a-executable-interaction-story-implementation-scope-closeout.md).

Product-facing UI Gallery exposure remains separate future work under `PT-UI-GALLERY-001`. Phase 13 overlay/popup/layering must consume this proof standard and must not create a parallel executable story path.

## Decision summary

Reusable UI interaction work should not close on static proof alone when the claim is live interaction. The long-term standard is an **Executable UI Interaction Story**: one story definition that can run deterministic replay mode and live proof-host mode while preserving the same normalized input stream, runtime interaction formation path, report shape, visual proof shape, static frame artifact, and no-bypass boundary assertions.

The key invariant is:

```text
Replay mode and live mode may differ only by input source.
Everything after NormalizedInputSample must be shared.
```

This keeps the proof stronger than a static marker frame and safer than a fake live demo. Gallery, Workbench, UI Designer, and later product hosts may display or drive proof evidence in future accepted plans, but they must not redefine reusable control semantics.

## Completed proof tier

Phase 12 provided Tier 1 through Tier 3 evidence:

```text
Tier 1 — Contract proof
  descriptors, package validation, catalog, inspection

Tier 2 — Deterministic replay proof
  scripted normalized input -> runtime report/outcomes

Tier 3 — Static visual proof
  visual proof -> UiFrame -> static mount validation
```

Phase 12A completed Tier 4 and Tier 5 proof-host core evidence:

```text
Tier 4 — Live proof host core
  UiInputEvent / NormalizedInputSample input -> same runtime session -> visible proof updates

Tier 5 — Executable interaction story
  shared story definition, replay mode, live proof-host mode, semantic replay/live parity,
  static frame artifact, and no-bypass boundary assertions
```

Default rule for later reusable interaction phases:

```text
If a phase claims reusable interaction behavior, require Tier 5 unless the owning GitHub issue explicitly accepts a contract-only or static-only scope.
```

## Owner map

### `ui_controls`

Owns reusable control interaction declarations, package-backed descriptors, and catalog/inspection summaries.

Must not own story execution, live input sessions, raw host event collection, gallery presentation, product commands, or product state mutation.

### `ui_input`

Owns normalized input fact vocabulary, including pointer, keyboard, focus, semantic, and text-intent facts.

Must not own reusable control semantics, story execution, product behavior, or gallery rendering.

### `ui_runtime`

Owns mounted interaction fixtures, `InteractionStorySession` execution mechanics, incremental `apply_sample` behavior, deterministic replay over the same session path, input log capture, `InteractionFormationReport` updates, `InteractionVisualProof` formation, and `InteractionProofRenderFrame` projection.

Must not own story registry/discovery authority, gallery/product event loops, product command routing, product state mutation, overlay/layer creation in Phase 12A, or full text editing transactions.

### `ui_story`

Owns or remains authority for executable story identity, workflow profile, evidence envelope, reports, diagnostics, and mount verdict expectations.

It must not absorb reusable control semantics from `ui_controls` or product behavior from apps/editors/games.

### Proof host / editor adapter

The merged PR #43 proof-host adapter is narrow. It adapts existing `UiInputEvent` values to normalized samples, feeds an `InteractionStorySession`, and exposes proof/frame/report/static-mount evidence for tests and future display.

It must not execute product commands, mutate editor state, register product-facing gallery surfaces, implement overlays, or implement text editing.

### `ui_static_mount`

Owns static `UiFrame` validation. It must not own live interaction, story execution, or control semantics.

### Product/editor/app layers

May later own mapping semantic outcomes to commands, product state changes, overlay and popup product behavior, text editing transactions, selection, command routing, undo/redo, persistence, and app-specific policy.

They are consumers of reusable interaction outcomes, not owners of reusable interaction semantics.

## Target architecture

```text
ui_controls
  ControlInteractionDescriptor
  package/catalog/inspection interaction facts

ui_input
  NormalizedInputSample
  pointer/key/focus/text-intent facts

ui_story
  executable story identity, workflow/evidence/report envelope, diagnostics

ui_runtime
  InteractionStorySession
  mounted fixture resolution
  deterministic replay and live incremental apply
  InteractionFormationReport
  InteractionVisualProof
  InteractionProofRenderFrame

proof host adapter
  UiInputEvent -> NormalizedInputSample
  live session drive
  visible proof/report/static-mount access

ui_static_mount
  UiFrame validation

product/editor/app
  later command/state behavior
```

## Replay workflow

```text
Executable UI Interaction Story
  -> replay script
  -> NormalizedInputSample stream
  -> InteractionStorySession::apply_sample
  -> InteractionFormationReport
  -> InteractionVisualProof
  -> InteractionProofRenderFrame
  -> UiStaticMountReport::from_frame
```

Replay mode proves deterministic regression behavior and can run in tests.

## Live proof-host workflow

```text
Executable UI Interaction Story
  -> proof host adapter
  -> UiInputEvent / raw host input seam
  -> NormalizedInputSample
  -> InteractionStorySession::apply_sample
  -> current InteractionFormationReport
  -> current InteractionVisualProof
  -> current InteractionProofRenderFrame
```

The proof host may expose proof evidence. It must not mutate reusable interaction state directly.

## Semantic replay/live parity workflow

```text
Live input log
  -> deterministic replay mode
  -> replayed-live-log report
  -> compare with original live report
```

Compare:

- target resolution;
- focus resolution;
- state transitions;
- runtime facts;
- runtime events;
- semantic outcomes;
- suppressed events;
- no-target events;
- observed markers;
- final current states;
- boundary assertions.

Do not compare:

- wall-clock timing;
- raw OS event ids;
- pixel snapshots;
- exact primitive count after every step;
- text wrapping;
- animation interpolation.

## Durable public concepts

Completed durable public/runtime names include:

```text
BASE_CONTROLS_GENERIC_INTERACTION_PROOF_ID
BASE_CONTROLS_EXECUTABLE_INTERACTION_STORY_ID
base_controls_generic_interaction_fixture
base_controls_generic_interaction_positive_script
base_controls_generic_interaction_negative_scripts
base_controls_generic_interaction_proof_frame
base_controls_executable_interaction_story_session
base_controls_executable_interaction_expected_evidence
InteractionStoryExecutionMode
InteractionStorySession
InteractionStoryRunReport
InteractionStoryStepEvidence
InteractionReplayLiveParityReport
BaseControlsInteractionProofHost
```

Phase-shaped public aliases and compatibility shims are not part of the completed proof path.

## Completed story content

The executable generic interaction story mounts:

```text
Button
Disabled Button
Inert Button
ActionPrompt
InspectorField
Read-only InspectorField
ListView
TreeView
TableView
Label
```

Required interaction evidence includes:

```text
hover Button
press Button
release Button inside
press Button then release outside
focus Button
keyboard activate Button
focus ActionPrompt and activate
focus List and navigate
focus Tree and navigate
focus Table and navigate
send text-intent to InspectorField
send text-intent to Read-only InspectorField
attempt text-intent on non-text control
press Disabled Button
click outside all controls
```

Required visible evidence includes:

```text
hovered
pressed
captured
focused
focus-visible
activation-requested
action-intent
list-active-item-intent
tree-node-intent
table-cell-or-row-intent
text-intent-probe
read-only-text-intent-probe
disabled
suppressed
no-target
```

## Completion criteria

Phase 12A is complete because planning evidence records that:

- an executable generic interaction story exists;
- deterministic replay mode runs canonical scripts;
- live proof-host mode accepts input through the same normalized runtime path;
- live mode records normalized input logs;
- recorded live input logs replay deterministically;
- replay/live semantic parity passes by user-reported validation;
- static `UiFrame` mount validation passes by user-reported validation;
- host command, product mutation, overlay, and text-edit transaction counters remain zero;
- product-facing Gallery exposure remains outside PR #43 and future work under `PT-UI-GALLERY-001`.

## Future extension seams

Later accepted plans may consume this standard for:

- multi-pointer, touch, pen, pressure, click count, long press, wheel/scroll;
- drag start, drag update, drag cancel, drop intent, drop target resolution;
- focus scopes, roving focus, modal focus trap, surface focus, focus restoration;
- shortcut intent and later product command routing;
- editable text, caret, selection, IME, clipboard, undo/redo, text buffer mutation;
- dropdowns, popups, tooltips, modals, outside-click dismissal, overlay stacking;
- scroll containers, viewport clipping, virtual lists;
- tabs, splits, docking, floating panels, multi-window, surface routing;
- accessibility roles, names, states, keyboard navigation contracts, accessibility proof;
- hover/press/focus animations as separate animation evidence;
- recorded input logs, golden semantic reports, static frame artifacts, CI comparison.

## Retained stop conditions

Stop and redesign if later work tries to use this design to add:

- story execution moving into `ui_controls`;
- a parallel story framework that bypasses `ui_story`;
- live proof-host state changes that do not pass through normalized input and runtime interaction formation;
- activation outcomes executing product commands;
- product/app/editor/game state mutation;
- overlay/popup/layering behavior inside Phase 12A artifacts;
- full text editing behavior;
- broad plugin framework extraction;
- `foundation/meta`;
- generic plugin primitives;
- pixel-perfect replay/live parity.

## Relationship to Phase 13

`PT-UI-COMPONENT-PLATFORM-013` is the active overlay/popup/layering design intake. It must consume this executable interaction story standard rather than bypassing it.

Phase 13 may define overlay declarations, anchor/placement/layer/focus/dismissal evidence, and overlay story proof. It must not implement product-facing UI Gallery, UI Designer, authored UI editing, product command execution, product mutation, full text editing, dynamic plugin loading, `foundation/meta`, or shared plugin primitives.
