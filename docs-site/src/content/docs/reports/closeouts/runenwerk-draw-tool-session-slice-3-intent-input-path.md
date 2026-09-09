---
title: Runenwerk Draw Tool Session Slice 3 Intent Input Path
description: Historical implementation-plan evidence for the completed inert ToolSession control-input slice.
status: completed
owner: drawing
layer: reports
canonical: false
last_reviewed: 2026-09-10
related_docs:
  - ../../design/implemented/runenwerk-draw-tool-session-architecture-slice.md
  - ./runenwerk-draw-tool-session-slice-2-session-control.md
  - ../../design/active/runenwerk-draw-pen-first-radial-tablet-ux-design.md
  - ../../apps/runenwerk-draw/README.md
  - ../../adapters/native-tablet-input/README.md
  - ../../domain/drawing/README.md
---

# Runenwerk Draw Tool Session Slice 3 Intent Input Path

## Historical status

This completed slice is retained as historical implementation-plan evidence. The durable ToolSession boundary is owned by the implemented ToolSession architecture design and current behavior is owned by code and tests. The planning language below is preserved as point-in-time rationale and does not authorize active work.

## Summary

This document plans the docs and implementation path for ToolSession slice 3
without changing runtime behavior.

Slice 3 should add an app-owned normalized control-input fact path for
non-pointer control facts and neutral inert intent plumbing. It must keep no
default key bindings, keep text input ignored/no-op for Draw tool control, keep
cancel and radial request intents as explicit synthetic/request plumbing only,
and defer `OffhandInputObserved` plus tool switching until there is a real
app-owned input or tool identity model.

This is still an app-layer architecture slice. It must not add visible
cancellation, radial UI, offhand UI behavior, tool switching behavior, or
pointer stroke behavior changes.

## Investigation Findings

`domain/ui/ui_input/src/event.rs::UiInputEvent` currently has `Pointer`,
`Keyboard`, and `Text` variants.

`domain/ui/ui_input/src/event.rs::KeyboardEvent` carries `Key`, `KeyState`,
and `Modifiers`. `domain/ui/ui_input/src/keyboard.rs::Modifiers` carries
`shift`, `ctrl`, `alt`, and `meta` flags.

`domain/ui/ui_input/src/event.rs::PointerEvent` also carries `Modifiers`.
`apps/runenwerk_draw/src/runtime/systems.rs::route_draw_input_system` builds
pointer modifiers from runtime input, currently with `shift` populated and the
other modifier flags left false.

`apps/runenwerk_draw/src/app/input.rs::DrawingToolInputEvent` carries
pointer/stylus facts: route kind, pointer kind, screen and canvas positions,
source kind, tool kind, device id, timestamp, pressure, tilt, twist, eraser
state, barrel buttons, low-latency classification, coalesced sample count,
predicted sample count, and coalesced samples. It does not copy
`PointerEvent::modifiers`.

`apps/runenwerk_draw/src/app/state.rs::RunenwerkDrawApp::dispatch_input`
currently handles only `UiInputEvent::Pointer`; non-pointer input returns
`false`.

`apps/runenwerk_draw/src/app/tool_session.rs::DrawingToolSession` owns
app-local session metadata after slice 2: `DrawingToolSessionId`,
`DrawingToolSessionPhase`, and `DrawingToolSessionAnchor`. That metadata is
interaction state only and must not participate in drawing document, command,
product, or cache identity.

`apps/runenwerk_draw/src/app/input.rs::DrawingToolRouteKind` remains
transitional stroke compatibility routing. It must not gain cancellation,
radial, offhand, or tool-switch semantics.

`apps/runenwerk_draw/src/runtime/systems.rs::coalesce_pointer_move_events`
uses pointer modifiers only to decide whether pointer moves can be coalesced.
There is no current Draw keyboard or text dispatch path in that runtime
system.

`docs-site/src/content/docs/adapters/native-tablet-input/README.md` documents
`native_tablet_input` as a packet normalization boundary. It preserves tablet
facts such as device id, tool kind, pressure, tilt, twist, tangential pressure,
hover, eraser, barrel buttons, calibration, raw timestamps, coalesced samples,
and predicted samples, but it must not own drawing semantics.

`docs-site/src/content/docs/design/active/runenwerk-draw-pen-first-radial-tablet-ux-design.md`
requires explicit offhand input for radial menus, forbids pen-contact hold
delays, and keeps radial menus as app/UI command surfaces rather than drawing
document truth.

## Problem Statement

The current input path has a clean split for pointer drawing facts, but Draw
does not yet have an app-owned normalized control-input fact path. Keyboard and
text events exist in `ui_input`, but `RunenwerkDrawApp::dispatch_input`
currently ignores them. That is behavior-preserving today, but it means future
cancel, radial, and tool-switch work has no correct place to enter the
ToolSession boundary.

Adding cancel, radial, offhand, or tool-switch semantics to
`DrawingToolRouteKind` would repeat the original routing problem. Route kind is
stroke compatibility routing. Future non-pointer control facts need a separate
app-owned control input path that can produce inert ToolSession intents without
mutating document truth or changing stroke behavior.

The first real slice should prove that path exists while staying neutral: raw
keyboard input is observed as generic control input, text input remains ignored
for Draw tool control, and request intents are emitted only by explicit
synthetic/request constructors.

## Input Boundary Review

`UiInputEvent` remains the platform-neutral UI input envelope owned by
`domain/ui/ui_input`.

`DrawingToolInputEvent` remains the app-owned pointer/stylus input fact object.
Do not add control, cancel, radial, offhand, or tool-switch semantics to it.

`DrawingToolControlInputEvent` should become the app-owned non-pointer control
input fact object for ToolSession. It should normalize keyboard/control facts
for Draw without pretending all keyboard input is offhand input.

`native_tablet_input` remains packet normalization only. It should not emit
Draw cancel, radial, offhand, or tool-switch semantics.

`RunenwerkDrawApp` remains the coordinator that receives `UiInputEvent`,
normalizes app facts, calls `DrawingToolSession`, and applies emitted intents.
It must continue to own document mutation, preview state, dirty invalidation,
preview jobs, product lifecycle, and frame rebuilds.

## Proposed Intent Input Path

Add an app-owned control input fact type separate from `DrawingToolInputEvent`:

```rust
pub struct DrawingToolControlInputEvent {
    pub source: DrawingToolControlInputSource,
    pub request: DrawingToolControlRequest,
}

pub enum DrawingToolControlInputSource {
    Keyboard {
        key: Key,
        state: KeyState,
        modifiers: Modifiers,
    },
    Synthetic,
}

pub enum DrawingToolControlRequest {
    Observe,
    Cancel,
    RadialMenu,
}
```

The exact names can be adjusted during implementation, but the ownership and
behavior are fixed:

- keyboard input normalizes to `Observe`;
- text input does not produce a control event in this slice;
- cancel and radial requests are explicit synthetic/request plumbing only;
- no default key binding maps raw keyboard input to cancel or radial requests.

Add `DrawingToolSession::handle_control_input(...) -> DrawingToolSessionOutcome`.
It should emit only inert intents and must not change pointer route handling.

Add inert `DrawingToolIntent` variants:

```rust
ControlInputObserved { input: DrawingToolControlInputEvent }
RequestCancel {
    input: DrawingToolControlInputEvent,
    active_session_id: Option<DrawingToolSessionId>,
}
RequestRadialMenu {
    input: DrawingToolControlInputEvent,
    anchor: Option<DrawingToolSessionAnchor>,
}
```

Do not add `OffhandInputObserved` in slice 3. Keyboard input is neutral control
input, not automatically offhand input.

Do not add `ToolSwitchRequested` in slice 3. Tool switching is deferred until
there is a real app-owned tool identity or catalog.

## Cancellation Readiness

`RequestCancel` is plumbing only. It should carry the explicit control input
request and the current active session id if one exists.

No app behavior should consume `RequestCancel` to cancel a stroke in this
slice. `RunenwerkDrawApp::apply_tool_intent` should treat it as a no-op.

Future cancellation behavior needs a separate slice that decides:

- which input policy maps to cancel;
- whether an active preview stroke is discarded or preserved;
- how pending preview tile jobs and preview products are cleared;
- how immediate stroke projection changes;
- what diagnostics, if any, are recorded;
- how last-good committed products are preserved.

## Offhand Input Readiness

Slice 3 should not use the word "offhand" for raw keyboard input. Keyboard
input is neutral control input until the app has a profile, device, shortcut,
or express-key policy that can distinguish offhand input from ordinary
keyboard/control input.

`OffhandInputObserved` is deferred. A future slice may introduce it after the
app has a normalized way to identify offhand source policy.

Barrel buttons remain pointer/stylus packet facts. They are not assigned
default radial or offhand behavior in slice 3.

## Radial Request Readiness

`RequestRadialMenu` is plumbing only. It should carry the explicit control
input request and the current session anchor if one exists.

No radial UI should open in this slice. No command palette, radial surface,
selection highlight, radial cancel zone, radial slider, or tool switch behavior
should be added.

No pen-contact hold delay may be introduced. Pen contact must continue drawing
immediately through the existing pointer path.

## Tool Switch Readiness

Tool switching remains deferred.

Do not add `ToolSwitchRequested` until `runenwerk_draw` has a concrete
app-owned tool identity or catalog. Without that model, a tool switch intent
would either be fake abstraction or would leak future tool semantics into
temporary routing.

Future tool switching should use app-owned tool identity and may later produce
app-only actions or domain `DrawingCommand` transactions depending on the tool.

## DrawingToolSession State Changes

Slice 3 should not add new phase variants unless the implementation needs
inert metadata only. The existing slice-2 `Idle` and `Active` phase model is
enough for this path.

`DrawingToolSession` should inspect current `DrawingToolSessionPhase` when
building request intents:

- `RequestCancel` should include `active_session_id()`;
- `RequestRadialMenu` should include `active_anchor().cloned()`;
- `ControlInputObserved` should not change phase.

Pointer `handle_input` behavior must remain equivalent to slice 2.

## RunenwerkDrawApp Coordination Plan

`RunenwerkDrawApp::dispatch_input` should keep the current pointer branch
unchanged.

Add a non-pointer branch:

- `UiInputEvent::Keyboard` normalizes to `DrawingToolControlInputEvent` with
  `Observe`;
- `UiInputEvent::Text` returns `false` and does not call ToolSession;
- generated control outcomes are passed to `apply_tool_intent`;
- `ControlInputObserved`, `RequestCancel`, and `RequestRadialMenu` are no-ops
  in `apply_tool_intent`;
- the dispatch return value for raw keyboard/text remains `false` in this
  slice because there are no default key bindings and no visible behavior.

Explicit synthetic/request constructors for cancel and radial should be used
by tests and later policy code. They should not be called from default runtime
keyboard input in slice 3.

## File-by-File Plan

`apps/runenwerk_draw/src/app/input.rs`

Add the app-owned normalized control-input fact types. Keep
`DrawingToolInputEvent`, `DrawingToolInputSample`, `DrawingPreviewStroke`,
`DrawingToolRouteKind`, `from_pointer_with_capture`, and `to_stroke_samples`
unchanged.

`apps/runenwerk_draw/src/app/tool_session.rs`

Add `DrawingToolSession::handle_control_input` and the inert intent variants.
Keep pointer route-to-intent mapping unchanged. Use existing slice-2 session
metadata when constructing request intents.

`apps/runenwerk_draw/src/app/state.rs`

Add non-pointer dispatch coordination for keyboard/text while preserving
current behavior. Apply new inert intents as no-ops. Do not move preview
stroke state, dirty tracking, commit behavior, preview jobs, product
publication, query snapshots, GPU validation, or frame projection.

`apps/runenwerk_draw/src/app/mod.rs`

Export new control-input/session intent types only as needed by app code and
tests. Avoid broadening public surface unnecessarily.

`apps/runenwerk_draw/src/runtime/systems.rs`

No behavior change is required. The runtime may keep routing pointer/touch and
native tablet events as it does today. Do not add default keyboard shortcuts in
this slice.

`apps/runenwerk_draw/tests/app_shell.rs`

Add focused behavior-preservation tests for non-pointer dispatch and existing
stroke behavior. Keep product/query lifecycle tests unchanged.

## Behavior Preservation Requirements

Slice 3 must preserve:

- pointer down/move/up stroke behavior;
- coalesced sample ordering;
- captured outside-canvas movement/up behavior;
- hover, scroll, enter, leave, and ignored pointer behavior;
- raw keyboard input as non-mutating neutral control observation;
- text input as ignored/no-op for Draw tool control;
- no runtime Escape/radial/offhand defaults;
- preview generation;
- dirty-start sample tracking;
- preview tile jobs and stale-job checks;
- committed product publication;
- query snapshot publication;
- GPU validation and promotion behavior;
- domain command, product, and cache identity independence from ToolSession and
  control-input ids.

## Test Plan

Do not add tests for this docs task.

The eventual implementation should add focused tests for:

- keyboard normalization into `ControlInputObserved`;
- text input remains non-mutating/no-op;
- explicit synthetic cancel request emits `RequestCancel`;
- explicit synthetic radial request emits `RequestRadialMenu`;
- no runtime Escape/radial/offhand defaults exist in this slice;
- non-pointer dispatch remains non-mutating;
- pointer stroke behavior remains unchanged;
- `DrawingToolRouteKind` remains stroke compatibility only;
- session/control ids do not affect domain command identity, product identity,
  or cache identity.

Validation commands for the eventual implementation:

```text
cargo test -p runenwerk_draw --test app_shell
cargo test -p runenwerk_draw
cargo test -p drawing --test ink_tile
cargo check --workspace
task docs:validate
git diff --check
```

## Non-Goals

- visible radial menu;
- actual cancellation behavior;
- actual offhand UI behavior;
- actual tool switching;
- default key bindings;
- `OffhandInputObserved`;
- `ToolSwitchRequested`;
- lasso;
- transform;
- fill;
- eraser mode changes;
- paper response;
- watercolor;
- export/package IO;
- layer UI;
- Workbench integration;
- engine render changes;
- `domain/drawing` authority changes;
- native tablet semantic routing;
- product lifecycle changes;
- GPU validation changes.

## Risks And Mitigations

Risk: keyboard input is mistaken for offhand input.

Mitigation: use `ControlInputObserved` and keep `OffhandInputObserved`
deferred until source policy exists.

Risk: inert cancel/radial requests accidentally become behavior.

Mitigation: keep `RequestCancel` and `RequestRadialMenu` as explicit
synthetic/request plumbing and no-op app intents in slice 3.

Risk: default key bindings change user-visible behavior.

Mitigation: add no default key bindings. Raw keyboard input only creates
neutral control observation, and text input remains ignored/no-op.

Risk: control input pollutes pointer route semantics.

Mitigation: keep `DrawingToolControlInputEvent` separate from
`DrawingToolInputEvent` and keep `DrawingToolRouteKind` stroke-only.

Risk: request metadata leaks into drawing truth.

Mitigation: keep session/control ids out of `DrawingCommand`,
`DrawingTransaction`, product identity, cache identity, and renderer state.

## Implementation Prompt For Later

```text
Implement ToolSession slice 3: inert normalized control-input path.

Scope:
- apps/runenwerk_draw only.
- No domain/drawing changes.
- No engine render changes.
- No visible cancellation behavior.
- No radial UI.
- No offhand UI behavior.
- No tool switching behavior.
- No default key bindings.
- Preserve pointer stroke behavior exactly.

Use:
- docs-site/src/content/docs/design/active/runenwerk-draw-tool-session-slice-3-intent-input-path.md

Implementation requirements:
- Add `DrawingToolControlInputEvent`, `DrawingToolControlInputSource`, and
  `DrawingToolControlRequest` as app-owned control input facts.
- Keep `DrawingToolInputEvent` pointer/stylus-only.
- Keep `DrawingToolRouteKind` stroke compatibility only.
- Add `DrawingToolSession::handle_control_input`.
- Add inert intents:
  - `ControlInputObserved { input }`
  - `RequestCancel { input, active_session_id }`
  - `RequestRadialMenu { input, anchor }`
- Raw keyboard input maps only to `ControlInputObserved`.
- Text input remains ignored/no-op for Draw tool control.
- `RequestCancel` and `RequestRadialMenu` are explicit synthetic/request
  plumbing only.
- Do not add `OffhandInputObserved` or `ToolSwitchRequested`.
- Do not alter preview stroke state, dirty tracking, preview jobs, product
  publication, query snapshots, GPU validation, or frame projection.

Tests:
- keyboard normalization into `ControlInputObserved`;
- text input remains non-mutating/no-op;
- explicit synthetic cancel request emits `RequestCancel`;
- explicit synthetic radial request emits `RequestRadialMenu`;
- no runtime Escape/radial/offhand defaults exist;
- pointer stroke behavior remains unchanged.

Validation:
- cargo test -p runenwerk_draw --test app_shell
- cargo test -p runenwerk_draw
- cargo test -p drawing --test ink_tile
- cargo check --workspace
- task docs:validate
- git diff --check
```
