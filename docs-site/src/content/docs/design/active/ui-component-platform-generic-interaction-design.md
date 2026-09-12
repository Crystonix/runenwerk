---
title: UI Component Platform Generic Interaction Design
description: Completed Phase 12 reference for reusable generic interaction semantics across ui_controls, ui_input, ui_runtime, host-owned policy, visible proof, and later text-editing readiness.
status: active
owner: ui
layer: domain
canonical: true
last_reviewed: 2026-09-12
related_docs:
  - ../../workspace/planning/roadmap.md
  - ./ui-component-platform-input-gesture-device-design.md
  - ./ui-component-platform-executable-interaction-story-design.md
  - ../../reports/closeouts/pt-ui-component-platform-012a-executable-interaction-story-implementation-scope-closeout.md
  - ./ui-component-platform-overlay-popup-layering-design.md
  - ../implemented/editor-ui-runtime-v2-and-interaction-formation-design.md
  - ./ui-component-platform-base-control-packages-design.md
---

# UI Component Platform Generic Interaction Design

## Status

This is the completed Phase 12 design reference for `PT-UI-COMPONENT-PLATFORM-012`.

Lifecycle state: `completed`.

PR #43 merged the implementation into `main` on 2026-06-30 at merge commit `c8b73dfa95fc335fd2b33c9137cac03a0f35060f`. User start condition for Phase 13 reports PR #43 was validated and merged.

This design remains the owner-boundary reference for package-backed declarations, normalized input replay, renderer-neutral visible proof, negative proof cases, durable base-controls proof fixtures, and no-bypass assertions. It does not authorize overlays, popups, dropdowns, tooltips, modal-like behavior, full text editing, product command execution, product/editor/game mutation, UI Gallery exposure, UI Designer work, shared plugin framework extraction, generic plugin primitives, or `foundation/meta`.

## Decision summary

Reusable controls may declare generic interaction support. They must not collect OS/window input, execute host commands, mutate product/editor/game state, create overlays, or own full text editing.

The reusable interaction path is:

```text
ui_controls package descriptors
  -> ui_input normalized facts
  -> ui_runtime descriptor-backed interaction formation
  -> InteractionVisualProof
  -> InteractionProofRenderFrame / UiFrame
  -> ui_static_mount validation
```

Phase labels may appear in planning and history. Current public APIs, stable ids, reusable fixture helpers, proof-host types, and active implementation-scope files must use durable domain names such as `base_controls_*` and `BaseControlsInteractionProofHost`.

## Owner boundaries

### `ui_controls`

Owns:

- `ControlInteractionDescriptor`;
- `ControlInteractionRequirement`;
- `ControlInteractionState` and `ControlInteractionStateSet`;
- `ControlInteractionTrigger`;
- `ControlInteractionOutcome`;
- `ControlInteractionSupportSummary`;
- catalog and inspection projections.

Does not own:

- OS/window input collection;
- runtime session state;
- host commands;
- product mutation;
- overlays;
- full text editing.

### `ui_input`

Owns normalized input facts:

```text
NormalizedInputSample
NormalizedInputFact
PointerInputFact
KeyboardInputFact
FocusInputFact
SemanticInputFact
TextIntentFact
```

Facts are data. They do not decide Button/List/Tree/Table/Inspector semantics.

### `ui_runtime`

Owns descriptor-backed interaction formation, replay/report evidence, visible proof formation, and durable base-controls proof fixtures:

```text
MountedInteractionFixture
MountedInteractionPlacement
InteractionReplayScript
InteractionReplayStep
InteractionFormationReport
InteractionVisualProof
InteractionProofFrame
InteractionProofRenderFrame
BASE_CONTROLS_GENERIC_INTERACTION_PROOF_ID
base_controls_generic_interaction_fixture
base_controls_generic_interaction_positive_script
base_controls_generic_interaction_negative_scripts
base_controls_generic_interaction_proof_frame
```

`ui_runtime` may emit reusable facts, events, outcomes, suppression evidence, no-target evidence, and boundary counters. It must not execute product behavior.

### `ui_static_mount`

Owns static frame validation through `UiStaticMountReport::from_frame`.

Current static proof tests use durable base-controls file names:

```text
domain/ui/ui_static_mount/tests/base_controls_generic_interaction_static_mount.rs
domain/ui/ui_static_mount/tests/base_controls_executable_interaction_story_static_mount.rs
```

### Hosts, apps, editor, and game layers

Own OS/window input collection, routing policy, command execution, app/editor/game mutation, persistence policy, product selection, product editing, and product-specific behavior.

## Base-control proof matrix

| Control | Interaction role | Proof expectation |
| --- | --- | --- |
| Label | inert/read-only display | Does not produce activation, mutation, or focus behavior unless explicitly declared later. |
| Button | hover, press, capture, focus, keyboard activation | Pointer release-inside and Enter/Space produce reusable activation intent only. |
| ActionPrompt | focus and action intent | Keyboard activation produces action intent without product behavior. |
| InspectorField | focus and text-intent probe | Text intent is visible as probe evidence only. |
| ColorPicker | focus and open intent seam | Picker overlays and value mutation remain future host-owned work. |
| ListView | active-item intent | Keyboard navigation produces reusable intent, not product selection mutation. |
| TreeView | node intent | Navigation produces reusable intent, not tree data mutation. |
| TableView | cell/row intent | Navigation produces reusable intent, not product data edits. |
| Disabled fixture | suppression | Input emits suppressed evidence. |
| Read-only fixture | read-only text-intent probe | Text intent is observed without edit transactions. |

## Interaction state semantics

Canonical reusable visible states include:

```text
enabled
disabled
read-only
hovered
pressed
active
focused
focus-visible
captured
suppressed
```

Important rules:

- `pressed` and `captured` may be observed evidence without being final current state after release.
- `disabled` suppresses activation and mutable interaction outcomes.
- `read-only` may observe text intent but does not edit text.
- `focused` does not imply `active`.
- `active` does not imply product mutation.
- `suppressed` means input was understood and intentionally ignored.

## Deterministic replay proof

The deterministic replay proof covers:

```text
pointer move over Button
pointer press Button
pointer release inside Button
pointer press then release outside Button
focus Button
keyboard activate Button
focus ActionPrompt and activate
focus List and navigate
focus Tree and navigate
focus Table and navigate
text-intent probe on InspectorField
text-intent probe on read-only InspectorField
text intent against non-text control
press Disabled Button
input outside all controls
```

Expected report evidence includes target resolution, focus resolution, state transitions, runtime facts, runtime events, semantic outcomes, suppression, no-target rows, and boundary counters.

Boundary counters must remain:

```text
host_commands_executed: 0
product_mutations: 0
overlay_events: 0
text_edit_transactions: 0
```

## Visible proof path

`InteractionVisualProof` is the semantic visible proof model. It exposes:

```text
main view:
  mounted controls and visible markers

inspector view:
  selected widget, control kind, declared requirements, observed reusable states,
  current interaction state set

report/event view:
  replay steps, target/focus resolution, state transitions, runtime facts/events,
  semantic outcomes, suppressed/no-target evidence, and boundary assertions
```

`interaction_visual_proof_to_frame` projects that proof into deterministic `UiFrame` render data. `UiStaticMountReport::from_frame` validates the frame without creating product UI, overlays, popups, or a broad gallery framework.

## Text-intent probe

This phase proves text-intent seams only. It does not introduce:

- caret geometry;
- selection ranges;
- text insertion/deletion transactions;
- IME/composition behavior;
- clipboard behavior;
- undo/redo integration;
- mutable text buffer ownership;
- text layout or scrolling integration.

## Negative proof scenarios

Required negative scenarios:

- disabled control suppression;
- keyboard activation without focus;
- pointer release outside after press;
- input outside all controls;
- missing focus target;
- disabled focus target;
- non-focusable focus target;
- focus target that does not declare focus;
- text intent against non-text-probe control;
- activation outcomes do not execute host commands;
- list/tree/table navigation does not mutate product data;
- overlay, popup, dropdown, tooltip, and layering behavior remain absent;
- full text editing remains absent.

## Implemented write scope in PR #43

Primary implementation files include:

```text
domain/ui/ui_controls/src/interaction.rs
domain/ui/ui_controls/src/package/descriptor.rs
domain/ui/ui_controls/src/package/validation.rs
domain/ui/ui_controls/src/authoring/mod.rs
domain/ui/ui_controls/src/base_control/compiler.rs
domain/ui/ui_controls/src/base_control/lowering/interaction.rs
domain/ui/ui_controls/src/base_control/lowering/inspection.rs
domain/ui/ui_controls/src/catalog/entry.rs
domain/ui/ui_input/src/facts.rs
domain/ui/ui_runtime/src/input/generic_interaction.rs
domain/ui/ui_runtime/src/input/generic_interaction_fixture.rs
domain/ui/ui_runtime/src/input/generic_interaction_visual_frame.rs
domain/ui/ui_runtime/src/input/mod.rs
domain/ui/ui_runtime/tests/interaction_replay_report.rs
domain/ui/ui_static_mount/src/lib.rs
domain/ui/ui_static_mount/tests/base_controls_generic_interaction_static_mount.rs
```

The implementation did not rewrite Phase 11 base controls. Additive interaction declarations attach through the existing contribution/lowering path and preserve the Phase 11 ownership model.

## Validation gate used for completion evidence

Completion evidence is recorded in planning from merged PR #43 plus the user validation report. The focused validation gate for the implementation was:

```text
cargo fmt --all --check
cargo check -p ui_controls
cargo check -p ui_input
cargo check -p ui_runtime
cargo check -p ui_static_mount
cargo test -p ui_controls control_interaction
cargo test -p ui_input input
cargo test -p ui_runtime executable_interaction_story
cargo test -p ui_runtime --test interaction_replay_report
cargo test -p ui_static_mount base_controls
python tools/docs/validate_docs.py
git diff --check
```

## Stop conditions retained for later work

Stop and redesign if later changes require:

- app/editor/game command behavior in reusable control code;
- OS/window input collection in `ui_controls`;
- product state changes in reusable control code;
- backend renderer behavior;
- overlay, popup, dropdown, tooltip, or layering behavior inside Phase 12 artifacts;
- full text editing behavior;
- broad shared plugin framework extraction;
- `foundation/meta`;
- generic plugin primitives;
- Phase 11 base-control rewrites;
- UI Gallery product exposure inside generic interaction proof.

## Relationship to next work

`PT-UI-COMPONENT-PLATFORM-013` is the active overlay/popup/layering design intake. It must consume the Phase 12 focus, keyboard, pointer capture, normalized input, replay/report, static frame, and no-bypass substrate rather than bypassing it.

Full text editing remains later and must consume the Phase 12 text-intent seam without treating text-intent evidence as editable text-buffer ownership.
