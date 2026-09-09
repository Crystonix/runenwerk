---
title: Runenwerk Draw Tool Session Slice 2 Session Control
description: Historical implementation-plan evidence for the completed DrawingToolSession session-control slice.
status: completed
owner: drawing
layer: reports
canonical: false
last_reviewed: 2026-09-10
related_docs:
  - ../../design/implemented/runenwerk-draw-tool-session-architecture-slice.md
  - ../../design/active/runenwerk-draw-pen-first-radial-tablet-ux-design.md
  - ../../apps/runenwerk-draw/README.md
  - ../../adapters/native-tablet-input/README.md
  - ../../domain/drawing/README.md
---

# Runenwerk Draw Tool Session Slice 2 Session Control

## Historical status

This completed slice is retained as historical implementation-plan evidence. The durable ToolSession boundary is owned by the implemented ToolSession architecture design and current behavior is owned by code and tests. The planning language below is preserved as point-in-time rationale and does not authorize active work.

## Summary

This document plans the second behavior-preserving `DrawingToolSession` slice.
The first slice introduced the app-owned ToolSession wrapper between
`DrawingToolInputEvent` and `RunenwerkDrawApp` intent handling. Slice 2 should
make the session boundary carry real interaction control state without moving
preview stroke data or adding visible tool behavior.

Recommended slice 2 scope:

- add explicit session identity and phase metadata inside
  `DrawingToolSession`;
- track the initiating pointer/device/tool facts for the active session;
- keep current route-to-intent mapping behavior unchanged;
- keep preview stroke samples, preview generation, dirty-start tracking,
  preview jobs, product publication, query snapshots, GPU validation, and
  document mutation in their current owners;
- defer cancel, offhand, radial, lasso, transform, fill, and eraser behavior
  until there is a real normalized input or tool feature slice.

This is still an app-layer architecture slice, not a feature slice.

## Investigation Findings

`apps/runenwerk_draw/src/app/tool_session.rs::DrawingToolSession` currently
owns only `active_kind: Option<DrawingToolSessionKind>`. Its
`DrawingToolSession::handle_input` method maps
`DrawingToolRouteKind::{BeginPreviewStroke, UpdatePreviewStroke,
EndPreviewStroke, Hover, Scroll, Ignored}` into `DrawingToolIntent` values.

`apps/runenwerk_draw/src/app/tool_session.rs::DrawingToolSessionKind` currently
has one variant, `InkStroke`. The implementation comment says this is a
gesture session kind, not a brush category or domain stroke type.

`apps/runenwerk_draw/src/app/tool_session.rs::DrawingToolIntent` currently has
named-field variants for `BeginPreviewStroke`, `UpdatePreviewStroke`,
`FinishPreviewStroke`, `Hover`, `Scroll`, and `Ignore`. The only payload today
is the normalized `DrawingToolInputEvent`.

`apps/runenwerk_draw/src/app/tool_session.rs::DrawingToolSessionOutcome`
currently carries `intent` and `handled`. It preserves the existing
`RunenwerkDrawApp::dispatch_input` handled/ignored return contract.

`apps/runenwerk_draw/src/app/input.rs::DrawingToolInputEvent` remains the
normalized app input fact object. It captures route kind, pointer kind, screen
position, optional canvas position, source/tool kind, device id, timestamp,
pressure, tilt, twist, eraser state, barrel buttons, latency class, and
coalesced/predicted sample counts.

`apps/runenwerk_draw/src/app/input.rs::DrawingToolInputEvent` does not
currently copy `ui_input::PointerEvent::modifiers` into the app input fact.
`domain/ui/ui_input/src/event.rs::PointerEvent` has `modifiers`, and
`domain/ui/ui_input/src/event.rs::UiInputEvent` already has `Keyboard` and
`Text` variants, but `apps/runenwerk_draw/src/app/state.rs::RunenwerkDrawApp::dispatch_input`
currently returns `false` for non-pointer events.

`apps/runenwerk_draw/src/app/input.rs::DrawingToolRouteKind` remains
transitional compatibility routing. It is still stroke-specific:
`BeginPreviewStroke`, `UpdatePreviewStroke`, and `EndPreviewStroke` are preview
stroke lifecycle routes, not generic tool-session semantics.

`apps/runenwerk_draw/src/app/state.rs::RunenwerkDrawApp` still owns
`preview_stroke`, `preview_stroke_visible`, `preview_generation`,
`preview_dirty_start_sample_index`, `pending_preview_job`,
`next_preview_stroke_id`, and `next_preview_sequence`.

`apps/runenwerk_draw/src/app/state.rs::RunenwerkDrawApp::dispatch_input`
calculates capture from the app-owned preview stroke, builds
`DrawingToolInputEvent::from_pointer_with_capture`, sends it through
`DrawingToolSession::handle_input`, applies the emitted intent, rebuilds the
frame, stores the routed input, and returns the outcome handled flag.

`apps/runenwerk_draw/src/app/state.rs::RunenwerkDrawApp::apply_tool_intent`
still owns preview stroke mutation. It starts preview strokes, appends samples,
finishes strokes, commits through `RunenwerkDrawApp::commit_preview_stroke`,
and leaves hover/scroll/ignored intents as no-ops.

`apps/runenwerk_draw/src/app/state.rs::RunenwerkDrawApp::next_preview_tile_job_snapshot`
and `RunenwerkDrawApp::preview_tile_job_is_current` still own preview tile job
snapshot and stale-job checks.

`apps/runenwerk_draw/tests/app_shell.rs` covers the existing stroke path:
pointer down starts preview, coalesced samples are ordered before current
samples, outside-canvas capture continues, pointer up commits one stroke,
non-mutating hover/scroll/enter/leave/ignored input does not mutate document or
preview state, preview jobs stay asynchronous, and product publication/query
snapshot barriers preserve visibility behavior.

`docs-site/src/content/docs/design/active/runenwerk-draw-tool-session-architecture-slice.md`
describes slice 1 as a compatibility wrapper and records the long-term
pipeline:

```text
PointerEvent / tablet packet
  -> DrawingToolInputEvent
  -> DrawingToolSession
  -> DrawingToolIntent
  -> DrawingCommand transaction or app-only navigation/selection action
  -> dirty tile invalidation / product lifecycle
```

`docs-site/src/content/docs/design/active/runenwerk-draw-pen-first-radial-tablet-ux-design.md`
requires explicit offhand input for radial menus. It says pen contact draws
immediately, no hidden pen-contact delay may be added, barrel buttons are
detected but unassigned by default, and radial menus must be driven by offhand
input/session state rather than preview-stroke routing.

`docs-site/src/content/docs/adapters/native-tablet-input/README.md` keeps
`native_tablet_input` as a packet normalization boundary. It maps native
tablet facts into platform-neutral pointer events and must not own drawing
semantics, stroke ratification, brush smoothing, canvas state, package IO, or
render/tile formation.

## Problem Statement

The first ToolSession slice created the correct app-owned boundary, but the
session currently has too little state to support future interaction control.
It can say that an ink stroke is active, but it cannot identify the active
session, describe how it started, connect later offhand/cancel input to a
specific gesture, or expose a phase model that future radial/modal behavior can
extend.

Adding cancellation, offhand input, radial entry points, tool switching, lasso,
transform, fill, or eraser behavior directly to `DrawingToolRouteKind` would
repeat the original architectural problem. Those are semantic session outcomes
and app actions. The next slice should therefore make
`DrawingToolSession` capable of owning session control metadata while leaving
current stroke behavior untouched.

The key constraint is that session control readiness must not become visible
feature behavior. There should be no new cancel path, radial UI, offhand
shortcut, tool switch, or tool type behavior in slice 2.

## Proposed Boundary Changes

`DrawingToolSession` should start owning interaction control metadata:

- active session identity;
- current session phase;
- current semantic session kind;
- initiating pointer/device/tool facts for the active session.

`DrawingToolSession` should not own:

- `DrawingDocument`;
- preview stroke sample storage;
- preview dirty tracking;
- preview tile job snapshots;
- product publication;
- query snapshot publication;
- GPU validation;
- render behavior.

`RunenwerkDrawApp` should continue applying emitted intents. It should ignore
new session-control metadata for now except through tests and introspection
accessors. This keeps the slice behavior-preserving.

`DrawingToolInputEvent` should remain pointer/stylus fact normalization. It
should not become an offhand/radial command model. If future offhand input
needs a normalized app fact object, add a separate app-owned input type rather
than forcing keyboard, express-key, or radial semantics into pointer facts.

`DrawingToolRouteKind` should remain unchanged in slice 2. The route mapping
is still the compatibility bridge from current pointer facts into ToolSession.

## Session State Model

Recommended slice 2 model:

```rust
pub struct DrawingToolSession {
    next_session_id: u64,
    phase: DrawingToolSessionPhase,
}

pub struct DrawingToolSessionId(pub u64);

pub enum DrawingToolSessionPhase {
    Idle,
    Active {
        id: DrawingToolSessionId,
        kind: DrawingToolSessionKind,
        anchor: DrawingToolSessionAnchor,
    },
}

pub struct DrawingToolSessionAnchor {
    pub source_kind: PointerSourceKind,
    pub tool_kind: PointerToolKind,
    pub device_id: Option<PointerDeviceId>,
    pub started_screen_position: UiPoint,
    pub started_canvas_position: Option<CanvasCoordinate>,
}
```

The implementation should use existing `DrawingToolInputEvent` fields to build
`DrawingToolSessionAnchor`. It should not query native tablet APIs or domain
drawing state.

`DrawingToolSessionPhase::Idle` and `DrawingToolSessionPhase::Active` are
enough for slice 2. Do not add `SuspendedForModal`, `PendingCancel`,
`RadialMenu`, or `ToolSwitching` phases until an actual input path or feature
uses them. Those names are useful future directions, but adding unused phases
now would create fake architecture.

`DrawingToolSessionKind::InkStroke` remains the only implemented semantic
session kind. Future `Lasso`, `Transform`, `Fill`, or mask-eraser session kinds
should be added only when their behavior and authority boundaries are planned.

The current `active_kind()` accessor may stay for compatibility, but it should
be derived from the phase. Add focused accessors only if tests need them:

- `phase()`;
- `active_session_id()`;
- `active_anchor()`.

## Intent Model Additions

Do not add emitted placeholder intents in slice 2.

The following should remain deferred:

- `RequestCancel`;
- `RequestRadialMenu`;
- `OffhandInputObserved`;
- `ToolSwitchRequested`;
- lasso/transform/fill/eraser intents.

Those intents would be unobservable placeholders unless slice 2 also adds a
real normalized input path for offhand/cancel/radial commands, which is outside
this behavior-preserving slice.

Keep the current `DrawingToolIntent` variants and route mapping equivalent:

- `BeginPreviewStroke { input }`;
- `UpdatePreviewStroke { input }`;
- `FinishPreviewStroke { input }`;
- `Hover { input }`;
- `Scroll { input }`;
- `Ignore { input }`.

If slice 2 needs to expose session identity, prefer session accessors on
`DrawingToolSession` over adding unused fields to every intent. This keeps the
app apply path stable while still making session control state real.

## Offhand Input Readiness

Current repo truth:

- `domain/ui/ui_input/src/event.rs::UiInputEvent` already has `Keyboard` and
  `Text` variants.
- `apps/runenwerk_draw/src/app/state.rs::RunenwerkDrawApp::dispatch_input`
  currently handles only `UiInputEvent::Pointer`.
- `apps/runenwerk_draw/src/app/input.rs::DrawingToolInputEvent` captures
  pointer packet `barrel_buttons`, but it does not capture
  `PointerEvent::modifiers`.
- `docs-site/src/content/docs/design/active/runenwerk-draw-pen-first-radial-tablet-ux-design.md`
  says explicit offhand keyboard or tablet express input should open radial
  menus, while pen contact continues drawing immediately.

Slice 2 should prepare for offhand input by making active session metadata
addressable. It should not route keyboard, express-key, or radial commands yet.

Future offhand work should introduce an app-owned normalized input fact type,
for example `DrawingToolControlInputEvent` or `DrawingOffhandInputEvent`, after
the source input policy is known. That future type can target the active
`DrawingToolSessionId` or open an app-only radial session without mutating
`DrawingDocument`.

Do not put offhand semantics into `native_tablet_input`. Native adapters remain
packet normalization only.

## Cancellation Readiness

Current repo truth:

- Pointer up commits an active preview stroke through
  `RunenwerkDrawApp::apply_tool_intent` and
  `RunenwerkDrawApp::commit_preview_stroke`.
- There is no cancel route, cancel intent, cancel phase, or cancel source.
- `RunenwerkDrawApp` preserves released preview visibility until product
  publication/query snapshot lifecycle replaces it.

Slice 2 should prepare cancellation by adding active session identity and
anchor metadata. This lets a future cancel input refer to "the active ink
stroke session" instead of guessing from preview state.

Slice 2 must not change current commit/cancel semantics. There is no current
cancel behavior to preserve, so do not add hidden cancel behavior. Pointer up
must still finish and commit exactly as it does now.

Future cancellation should be a separate behavior slice with explicit rules:

- what input requests cancel;
- whether cancel discards preview samples, hides immediate preview, or records
  diagnostics;
- how pending preview jobs and preview products are cleared;
- whether any document transaction is emitted;
- how last-good visible products are preserved.

## Radial Menu Readiness

Current repo truth:

- The radial UX design requires explicit offhand input to open radial menus.
- It forbids pen-contact hold delays.
- It says radial menus are app/UI command surfaces, not drawing document truth.
- There is no current radial UI or radial ToolSession behavior in
  `apps/runenwerk_draw`.

Slice 2 should not add radial menu UI or radial intents. It should make the
active session identifiable so a future radial entry point can decide whether
it is opening during idle drawing, during an active ink stroke, or as an
app-only modal interaction.

Future radial work should add an explicit app action path. It must not mutate
`DrawingDocument` unless the selected radial entry later produces a planned
domain command transaction.

## File-by-File Plan

`apps/runenwerk_draw/src/app/tool_session.rs`

Implement the slice 2 state model here. Add `DrawingToolSessionId`,
`DrawingToolSessionPhase`, and `DrawingToolSessionAnchor`. Replace
`active_kind: Option<DrawingToolSessionKind>` with a phase model and derive
`active_kind()` from that phase. Keep `handle_input` mapping behavior
equivalent to slice 1.

`apps/runenwerk_draw/src/app/input.rs`

No behavior change in slice 2. Keep `DrawingToolInputEvent`,
`DrawingToolRouteKind`, `from_pointer_with_capture`, and `to_stroke_samples`
unchanged. Do not add offhand/radial semantics here. Only add imports or small
documentation if the implementation needs them.

`apps/runenwerk_draw/src/app/state.rs`

No ownership migration in slice 2. Keep `RunenwerkDrawApp` as preview stroke,
dirty tracking, transaction, preview job, product lifecycle, and frame rebuild
coordinator. `dispatch_input` should keep routing through
`DrawingToolSession::handle_input` and applying the emitted intent exactly as
it does now.

`apps/runenwerk_draw/src/app/mod.rs`

Export any new session-control types only if tests or app callers need them.
Avoid widening the public surface unnecessarily.

`apps/runenwerk_draw/tests/app_shell.rs`

Add behavior-preservation tests only if existing coverage misses a regression.
Current app-shell tests already cover the main stroke path, coalesced ordering,
outside-canvas capture, ignored inputs, preview jobs, and product barriers.

`apps/runenwerk_draw/src/runtime/ink.rs`

No changes. Preview jobs, product publication, query snapshots, cache behavior,
and GPU validation remain unrelated to slice 2.

`docs-site/src/content/docs/design/active/runenwerk-draw-tool-session-slice-2-session-control.md`

This planning document owns the slice 2 contract.

`docs-site/src/content/docs/design/active/README.md`

Link this document under Drawing / Apps.

## Migration Strategy

1. Add the session-control types in `tool_session.rs`.
2. Initialize `DrawingToolSession` with `phase: Idle` and
   `next_session_id: 1`.
3. On `BeginPreviewStroke`, allocate a new session id, store
   `Active { kind: InkStroke, anchor }`, and emit the same
   `BeginPreviewStroke { input }` intent as slice 1.
4. On `UpdatePreviewStroke`, keep the active phase and emit the same
   `UpdatePreviewStroke { input }` intent. Do not alter sample conversion.
5. On `FinishPreviewStroke`, emit the same `FinishPreviewStroke { input }`
   intent and return to `Idle`.
6. On hover, scroll, and ignored input, emit the same intents and leave the
   active phase unchanged unless current slice-1 behavior already clears it.
7. Keep `RunenwerkDrawApp::apply_tool_intent` unchanged except for compile
   adjustments caused by type exports.
8. Add direct unit tests for session id allocation, phase transitions, and
   route-to-intent equivalence.
9. Run the full validation list before considering the slice closed.

If preserving exact slice-1 direct `handle_input` behavior conflicts with a
cleaner state invariant, prefer preserving app-observable behavior and document
the invariant as a future cleanup. Current app-generated update/end routes are
already gated by `RunenwerkDrawApp` capture state.

## Behavior Preservation Requirements

Slice 2 must preserve:

- pointer down inside canvas starts preview stroke;
- pointer move while captured appends samples;
- pointer up commits exactly one stroke transaction;
- coalesced samples remain ordered before current sample;
- captured outside-canvas movement/up behavior remains unchanged;
- hover, scroll, enter, leave, and ignored inputs do not mutate document or
  preview state;
- `RunenwerkDrawApp::dispatch_input` handled/ignored return semantics;
- `DrawingToolInputEvent::from_pointer_with_capture`;
- `DrawingToolInputEvent::to_stroke_samples`;
- preview generation and dirty-start sample tracking;
- preview tile job snapshots and stale-job checks;
- committed product publication and query snapshot lifecycle;
- GPU validation and promotion behavior.

Slice 2 must not introduce visible cancellation, radial, offhand, tool switch,
lasso, transform, fill, or eraser behavior.

## Test Plan

Do not add tests for this planning task.

The eventual slice 2 implementation should keep the existing coverage and add
focused tests for:

- `DrawingToolSession` starts in `Idle`;
- `BeginPreviewStroke` allocates a stable active `DrawingToolSessionId`;
- `UpdatePreviewStroke` preserves the active session id and anchor;
- `FinishPreviewStroke` returns the session to `Idle`;
- hover, scroll, and ignored inputs do not create a session;
- route-to-intent mapping remains equivalent to slice 1;
- `RunenwerkDrawApp::dispatch_input` handled/ignored return semantics remain
  unchanged;
- pointer down/move/up still commits exactly one stroke;
- coalesced samples preserve sequence order;
- captured outside-canvas movement remains accepted while capture is active;
- preview tile job and committed product publication/query snapshot tests
  still pass.

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

- visible cancellation behavior;
- radial menu UI;
- offhand keyboard or express-key routing;
- tool switching behavior;
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
- preview stroke state migration into `DrawingToolSession`;
- preview tile job changes;
- committed ink product lifecycle changes;
- query snapshot lifecycle changes;
- GPU validation changes.

## Risks And Mitigations

Risk: session metadata becomes ceremonial.

Mitigation: only add metadata derived from real current input and test phase/id
transitions directly. Do not add un-emitted cancel/radial/offhand placeholder
intents.

Risk: session state starts competing with `RunenwerkDrawApp` preview state.

Mitigation: keep ToolSession responsible only for interaction control metadata.
Keep preview stroke samples, dirty tracking, commit, and product lifecycle in
`RunenwerkDrawApp`.

Risk: direct `handle_input` invariants diverge from app-observable behavior.

Mitigation: preserve the current route-to-intent mapping and add unit tests for
session phase transitions. Treat impossible app routes, such as update without
capture, as compatibility behavior unless a later cleanup proves they should
be rejected.

Risk: offhand/radial readiness gets mixed into pointer input facts.

Mitigation: keep `DrawingToolInputEvent` pointer/stylus-specific. Add a
separate app-owned offhand/control input fact type in a later slice if needed.

Risk: cancellation readiness accidentally changes stroke commit behavior.

Mitigation: do not add cancel routes or intent handling in slice 2. Pointer up
continues to commit exactly one stroke through the current app path.

Risk: future phases are over-modeled.

Mitigation: implement only `Idle` and `Active` in slice 2. Document
`SuspendedForModal`, `PendingCancel`, and radial phases as future behavior
work, not current code.

## Implementation Prompt For Later

```text
Implement ToolSession slice 2 for behavior-preserving session control metadata.

Scope:
- apps/runenwerk_draw only.
- No engine render changes.
- No domain/drawing authority changes.
- No visible cancellation behavior.
- No radial menu UI.
- No offhand keyboard or express-key routing.
- No lasso, transform, fill, eraser-mode changes, paper response, watercolor,
  export/package IO, layer UI, or Workbench integration.
- Preserve current ink stroke behavior exactly.

Use:
- docs-site/src/content/docs/design/active/runenwerk-draw-tool-session-slice-2-session-control.md

Implementation requirements:
- Add `DrawingToolSessionId`, `DrawingToolSessionPhase`, and
  `DrawingToolSessionAnchor` in `apps/runenwerk_draw/src/app/tool_session.rs`.
- Replace `active_kind: Option<DrawingToolSessionKind>` with a phase model.
- Keep `DrawingToolIntent` route mapping equivalent to slice 1.
- Keep `DrawingToolRouteKind` as transitional compatibility routing.
- Keep preview stroke samples, dirty tracking, commit, preview jobs, product
  publication, query snapshots, GPU validation, and frame rebuilds in
  `RunenwerkDrawApp` and existing runtime owners.
- Do not alter `DrawingToolInputEvent::from_pointer_with_capture` or
  `DrawingToolInputEvent::to_stroke_samples`.

Tests:
- Add narrow ToolSession unit tests for idle/active phase, session id
  allocation, anchor preservation, finish returning to idle, ignored inputs not
  creating sessions, and route-to-intent equivalence.
- Keep existing app-shell behavior tests passing.

Validation:
- cargo test -p runenwerk_draw --test app_shell
- cargo test -p runenwerk_draw
- cargo test -p drawing --test ink_tile
- cargo check --workspace
- task docs:validate
- git diff --check
```
