---
title: UI Component Platform Input Gesture And Device Design
description: Phase 5 design for reusable control input, gesture, and device capability declarations without raw device ownership or product input policy.
status: active
owner: ui
layer: domain
canonical: true
last_reviewed: 2026-06-26
related_designs:
  - ./runenwerk-ui-platform-capability-roadmap.md
  - ./runenwerk-ui-story-driven-golden-workflow-design.md
  - ../implemented/editor-ui-runtime-v2-and-interaction-formation-design.md
  - ./ui-component-platform-catalog-discovery-inspection-design.md
related_docs:
  - ../../domain/ui/architecture.md
  - ../../domain/ui/roadmap.md
  - ../../workspace/planning/roadmap.md
---

# UI Component Platform Input Gesture And Device Design

## Status

This is the Phase 5 planning and acceptance design for `PT-UI-COMPONENT-PLATFORM-005`.

It follows Phase 1 `ControlPackage` / `ControlKernel` contracts, Phase 2 authoring ergonomics, Phase 3 story-proof envelopes, and Phase 4 catalog/discovery/inspection. It defines reusable control input, gesture, and device capability declarations. It does not authorize runtime widget behavior, app/editor/game mutation, raw device polling, OS input handling, game input policy, world input policy, drawing semantics, canvas document truth, Gallery previews, Designer UX, Workbench behavior, runtime mount eligibility, text editing implementation, Surface2D, SpatialCanvas, NodeCanvas, PortGraphCanvas, ProgressionTreeView, TrackSurface, Timeline, transitions, renderer behavior, or ECS behavior.

## Existing Authority

`ui_controls` owns reusable control semantics and may declare which normalized input modes, gestures, and device facts a control requires.

Lower UI input substrate crates own normalized input packet vocabulary and runtime input formation. App/editor/game layers own raw device polling, OS/window integration, host routing policy, and command mutation.

Gallery, Workbench, UI Designer, docs, and agents consume input/gesture/device declarations through catalog and inspection facts. They do not define reusable control semantics.

## Problem

Reusable controls currently expose schemas, routes, stories, proof requirements, catalog entries, and inspection facts. They do not yet expose a reusable declaration of the input and gesture capabilities needed to interact with a control.

Without a shared declaration, later phases could duplicate or hardcode input expectations across controls, story fixtures, Gallery previews, Designer pickers, Workbench inspectors, runtime adapters, and product surfaces. That would blur ownership between reusable component semantics and product/device input policy.

## Decision

Add a reusable input/gesture/device declaration layer for the UI Component Platform.

The component layer owns declarations, not runtime packets. It says what a control supports or requires. It does not listen to hardware, normalize OS events, capture pointers at runtime, execute gestures, route host commands, or mutate app/editor/game state.

Correct ownership split:

```text
ui_controls
  owns control input modes, gesture requirements, device capability declarations,
  and derived inspection/catalog facts.

ui_input / existing input substrate
  owns normalized input packet vocabulary, raw sample formation, and runtime input facts.

apps / editor / game hosts
  own OS/window input collection, routing policy, command execution, game input policy,
  world input policy, and mutation.

Gallery / Workbench / UI Designer / docs / agents
  consume declarations; they do not own reusable control semantics.
```

## Proposed Contract Shape

The first implementation should prefer one focused module:

```text
domain/ui/ui_controls/src/input.rs
```

Split later only when stable responsibilities require it.

Candidate public concepts:

```text
ControlInputMode
ControlInputModeSet
ControlGestureRequirement
ControlGestureKind
ControlDeviceRequirement
ControlDeviceKind
ControlPointerRequirement
ControlKeyboardRequirement
ControlWheelRequirement
ControlTextInputRequirement
ControlSemanticActionRequirement
ControlInputCapabilitySummary
ControlInputInspectionFact
```

The final names may differ after inspecting nearby conventions, but the responsibilities should remain:

- declare supported input modes for a control kind;
- declare required gestures without implementing gesture recognition;
- distinguish pointer, wheel, keyboard, semantic action, text input, touch, controller, and stylus/tablet capability facts;
- represent pointer capture and lost-capture requirements as declarations, not runtime capture behavior;
- represent drag, marquee/select rectangle, multi-click, cancel, commit, and rollback as reusable gesture requirements;
- preserve normalized device facts such as hover/contact/pressure/tilt/twist/tangential pressure, eraser, barrel button, coalesced samples, and predicted samples;
- expose summaries that catalog/inspection can consume;
- avoid product-specific command routing and app/editor/game mutation.

## Minimum Phase 5 Scope

The first implementation pass should prove the contract with a small declaration model:

```text
ControlInputDescriptor
ControlInputMode
ControlGestureRequirement
ControlDeviceRequirement
ControlInputCapabilitySummary
```

Minimum input modes:

```text
pointer
wheel
keyboard
semantic-action
text-input
touch-ready
controller
stylus-tablet
```

Minimum gesture kinds:

```text
hover
press
drag
marquee-select
multi-click
cancel
commit
rollback
pointer-capture
lost-capture
```

Minimum device facts:

```text
pressure
tilt
twist
tangential-pressure
eraser
barrel-button
coalesced-samples
predicted-samples
```

## Non-Goals

Do not implement:

- runtime widget behavior;
- app/editor/game mutation;
- raw device polling;
- OS/window input handling;
- game input policy;
- world input policy;
- drawing semantics;
- canvas document truth;
- gesture recognition execution;
- pointer capture execution;
- command routing or host intent mutation;
- Gallery previews;
- Designer UX;
- Workbench behavior;
- runtime mount eligibility;
- text editing implementation;
- Surface2D;
- SpatialCanvas;
- NodeCanvas;
- PortGraphCanvas;
- ProgressionTreeView;
- TrackSurface;
- Timeline;
- transitions;
- renderer behavior;
- ECS behavior.

## Boundary Rules

- Input declarations are package facts, not runtime event handlers.
- Gesture requirements are semantic requirements, not recognizers.
- Device facts describe normalized capabilities, not raw OS packets.
- Host intent and mutation remain outside `domain/ui`.
- Game input policy and world input policy remain outside `PT-UI-COMPONENT-PLATFORM`.
- Catalog/inspection may expose input facts as read-only data.
- Story proof may reference input requirements, but Phase 5 does not run stories.
- Runtime mount eligibility remains future-gated.

## Acceptance Criteria

Phase 5 is implementation-complete only when:

- reusable input/gesture/device declarations exist in `ui_controls`;
- control packages can attach or derive input capability summaries without breaking existing packages;
- declarations distinguish input modes, gestures, and device facts;
- focused tests prove supported modes, gesture requirements, device facts, inspection summaries, and no runtime behavior;
- catalog/inspection can expose input declarations if needed;
- no raw input collection, gesture execution, host mutation, game/world input policy, drawing semantics, text editing implementation, runtime mount, Gallery, Designer, Workbench, renderer, ECS, or canvas behavior is implemented.

## Candidate Implementation Scope

The first implementation pass may touch:

```text
domain/ui/ui_controls/src/input.rs
domain/ui/ui_controls/src/package.rs
domain/ui/ui_controls/src/catalog.rs
domain/ui/ui_controls/tests/control_input_contract.rs
domain/ui/ui_controls/tests/control_catalog_contract.rs
```

Use `catalog.rs` only to expose read-only input declaration summaries. Do not add app/editor/gallery code in Phase 5.

## Test Plan

Required focused tests for the future implementation pass:

```text
cargo test -p ui_controls control_input
cargo test -p ui_controls control_catalog
cargo test -p ui_controls control_package
cargo test -p ui_controls control_registry
cargo test -p ui_controls control_story_proof
cargo test -p ui_controls control_authoring
cargo test -p ui_artifacts control_package
cargo test -p ui_program route
```

Required static checks:

```text
cargo fmt --all --check
cargo check -p ui_controls
git diff --check
```

Recommended test cases:

- input descriptor records supported pointer, wheel, keyboard, semantic-action, and text-input modes;
- gesture requirements distinguish drag, marquee-select, multi-click, cancel, commit, and rollback;
- device requirements distinguish pressure, tilt, twist, eraser, barrel button, coalesced samples, and predicted samples;
- catalog/inspection summaries expose input declarations read-only;
- no declaration makes a control runtime-mount eligible;
- no runtime event handler or host command path is introduced.

## Phase 5 Implementation Gate

Before writing Rust code, confirm:

- this design is accepted;
- Phase 4 remains green on the branch base;
- existing `ui_controls` catalog, story-proof, authoring, package, and validation contracts are still current;
- planning records name Phase 5 implementation as active;
- no new stop condition has been triggered.

## Handoff

Start implementation only after this design and planning state are accepted. The first implementation pass should add the smallest input/gesture/device declaration contract and focused tests. Do not implement runtime input handling, gesture recognition, app/editor/game mutation, drawing semantics, text editing, canvas behavior, Gallery previews, Designer UX, Workbench behavior, renderer behavior, ECS behavior, or runtime mount eligibility in Phase 5.
