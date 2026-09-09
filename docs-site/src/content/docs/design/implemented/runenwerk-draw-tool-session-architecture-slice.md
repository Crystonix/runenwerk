---
title: Runenwerk Draw Tool Session Architecture Slice
description: Implemented DrawingToolSession boundary between normalized Draw input and drawing commands or app actions.
status: implemented
owner: drawing
layer: app
canonical: true
last_reviewed: 2026-09-10
related_docs:
  - ../../apps/runenwerk-draw/README.md
  - ../active/runenwerk-draw-pen-first-radial-tablet-ux-design.md
  - ../../domain/drawing/README.md
  - ../../adapters/native-tablet-input/README.md
---

# Runenwerk Draw Tool Session Architecture Slice

## Implementation status

This boundary is implemented and has been checked against the current Draw app. `apps/runenwerk_draw/src/app/tool_session.rs` owns `DrawingToolSession`, session identity/phase/anchor state, `DrawingToolIntent`, and the normalized control-input/request path, while `RunenwerkDrawApp` remains the app coordinator and `domain/drawing` remains the owner of document mutation and drawing truth.

The older proposal and implementation-plan sections below are retained as design rationale. Current code and the Draw app documentation own exact API spelling and behavior. Later ToolSession slices added control state and inert control-input plumbing without changing this ownership boundary.

## Summary

This document plans the first behavior-preserving ToolSession boundary after
inspecting the current drawing app code and docs.

The target app-layer pipeline is:

```text
PointerEvent / tablet packet
  -> DrawingToolInputEvent
  -> DrawingToolSession
  -> DrawingToolIntent
  -> DrawingCommand transaction or app-only navigation/selection action
  -> dirty tile invalidation / product lifecycle
```

This is an architecture boundary slice, not a feature slice. It does not add
lasso, transform, fill, eraser mode changes, radial menus, offhand input,
cancellation behavior, rendering changes, or drawing-domain authority changes.

## Investigation Findings

The current repository already has domain-owned drawing mutation contracts.
`domain/drawing/src/history/operation.rs::DrawingCommand` defines the command
vocabulary, including `BeginStroke`, `AppendStrokeSample`, and `CommitStroke`.
`domain/drawing/src/history/operation.rs::DrawingTransaction` groups commands
and applies them through `DrawingTransaction::apply_to`.

The drawing app already commits preview strokes through those domain contracts.
`apps/runenwerk_draw/src/app/state.rs::RunenwerkDrawApp::commit_preview_stroke`
builds a `DrawingTransaction` from `DrawingCommand::{BeginStroke,
AppendStrokeSample, CommitStroke}` and applies it to `DrawingDocument`.

`DrawingInkRuntimeState` is already split. The aggregate lives in
`apps/runenwerk_draw/src/app/ink/mod.rs::DrawingInkRuntimeState`, and that
module composes `cache`, `gpu_validation`, `journal`, `preview`, `publication`,
and `visibility` submodules under `apps/runenwerk_draw/src/app/ink/`.

Tool input normalization is app-owned today.
`apps/runenwerk_draw/src/app/input.rs::DrawingToolInputEvent` stores normalized
pointer/stylus facts for the drawing app.
`DrawingToolInputEvent::from_pointer_with_capture` converts `ui_input`
pointer facts into app input facts, including captured outside-canvas mapping.

Pointer-to-stroke-sample conversion is currently also in the app input module.
`apps/runenwerk_draw/src/app/input.rs::DrawingToolInputEvent::to_stroke_samples`
preserves coalesced sample order before appending the current sample.

`DrawingToolRouteKind` is currently stroke-specific compatibility routing.
`apps/runenwerk_draw/src/app/input.rs::DrawingToolRouteKind` has
`BeginPreviewStroke`, `UpdatePreviewStroke`, and `EndPreviewStroke` variants.
`apps/runenwerk_draw/src/app/state.rs::RunenwerkDrawApp::apply_routed_input`
matches those route kinds directly to mutate preview stroke state.

Preview stroke state currently lives in `RunenwerkDrawApp`.
`apps/runenwerk_draw/src/app/state.rs::RunenwerkDrawApp` owns
`preview_stroke`, `preview_stroke_visible`, `preview_generation`,
`preview_dirty_start_sample_index`, `pending_preview_job`,
`next_preview_stroke_id`, and `next_preview_sequence`.

Preview tile job state is also coordinated by `RunenwerkDrawApp`.
`RunenwerkDrawApp::next_preview_tile_job_snapshot` builds the snapshot consumed
by preview jobs, and `RunenwerkDrawApp::preview_tile_job_is_current` rejects
stale preview job output.

Preview job processing and committed product publication stay in runtime ink
systems. `apps/runenwerk_draw/src/runtime/ink.rs::process_drawing_preview_ink_jobs`
submits and drains preview jobs.
`apps/runenwerk_draw/src/runtime/ink.rs::publish_drawing_ink_products` owns
committed ink product publication, and
`apps/runenwerk_draw/src/runtime/ink.rs::publish_drawing_ink_query_snapshots`
owns query snapshot publication.

`native_tablet_input` remains a packet normalization boundary.
`docs-site/src/content/docs/adapters/native-tablet-input/README.md` states that
the adapter maps native tablet facts into platform-neutral `ui_input` pointer
events and must not own drawing semantics, stroke ratification, brush
smoothing, canvas state, package IO, or render/tile formation.

The app README was stale relative to current code. It still referenced
`apps/runenwerk_draw/src/app/ink.rs` and described the `DrawingInkRuntimeState`
split as future work even though the split now exists under
`apps/runenwerk_draw/src/app/ink/`.

## Problem Statement

The current app has one stroke-oriented contact lifecycle and it works for
present ink behavior. The architectural problem is that `DrawingToolRouteKind`
is doing more than neutral routing: it names preview-stroke lifecycle events
and `RunenwerkDrawApp::apply_routed_input` treats those route names as tool
semantics.

That model should not expand into the long-term semantic model for lasso,
transform, fill, eraser mode changes, radial menus, offhand input,
cancellation, or navigation. Those tools need a session/intent boundary that
can express semantic outcomes without adding every future behavior to
`DrawingToolRouteKind`.

## Proposed Boundary

`DrawingToolInputEvent` remains the normalized app-owned input fact object. It
should continue to own extracted pointer kind, screen and canvas positions,
source/tool kind, device id, pressure, tilt, twist, eraser flag, barrel
buttons, latency class, coalesced samples, and predicted sample counts.

`DrawingToolSession` belongs in `apps/runenwerk_draw` because it is app-owned
interaction/session state, not domain drawing truth. It should own active
gesture/tool session state and translate input facts into semantic
`DrawingToolIntent` values.

`DrawingToolIntent` should represent app/tool outcomes. In the first slice,
the intents mirror the current preview stroke begin/update/finish behavior.
Later slices can add app-only navigation/selection actions or document-mutation
intents without expanding `DrawingToolRouteKind`.

`RunenwerkDrawApp` remains the coordinator for this slice. It continues to own
preview stroke state, transaction application, dirty tile invalidation, preview
job state, ink product lifecycle coordination, and frame rebuilds.

`domain/drawing` remains the owner of drawing truth, `DrawingCommand`,
`DrawingTransaction`, ratification, deterministic CPU tile formation,
invalidation helpers, and product descriptor helpers.

`native_tablet_input` remains packet normalization only. It must not own draw
tool sessions, stroke commit policy, brush behavior, document mutation, or
render products.

`DrawingToolRouteKind` may remain temporarily as compatibility routing. It
must not grow into the long-term semantic model for future tools.

## First-Slice Decision

Choose option A: keep preview stroke state in `RunenwerkDrawApp` for the first
slice, and let `DrawingToolSession` produce intents only.

This is the lowest-risk behavior-preserving path because current preview state
is entangled with preview generation, dirty-start tracking, preview tile job
snapshots, transaction commit, dirty tile invalidation, frame rebuilds, and
product lifecycle coordination.

Option B, moving preview stroke state into `DrawingToolSession` immediately, is
too broad for this slice. It would turn a boundary introduction into a preview
state migration.

Option C remains available later: split preview stroke state after the
session/intent boundary is proven behavior-preserving by tests.

## Proposed Types

The future implementation should add
`apps/runenwerk_draw/src/app/tool_session.rs`.

Planned first-slice types:

```rust
pub struct DrawingToolSession;

pub enum DrawingToolSessionKind {
    InkStroke,
}

pub enum DrawingToolIntent {
    BeginPreviewStroke { input: DrawingToolInputEvent },
    UpdatePreviewStroke { input: DrawingToolInputEvent },
    FinishPreviewStroke { input: DrawingToolInputEvent },
    Hover { input: DrawingToolInputEvent },
    Scroll { input: DrawingToolInputEvent },
    Ignore { input: DrawingToolInputEvent },
}

pub struct DrawingToolSessionOutcome {
    pub intent: DrawingToolIntent,
    pub handled: bool,
}
```

`DrawingToolSessionKind::InkStroke` is the first semantic session kind. It is
not a brush category and not a domain stroke type.

Do not add lasso, transform, fill, eraser, radial menu, offhand input,
cancellation, or navigation behavior in this slice.

## File-by-File Plan

`apps/runenwerk_draw/src/app/tool_session.rs` should be added in the future
implementation. It should hold the app-owned session and intent types plus the
compatibility mapping from current route kinds into intents.

`apps/runenwerk_draw/src/app/input.rs` should keep `DrawingToolInputEvent`,
`DrawingToolInputSample`, `DrawingPreviewStroke`, and pointer-to-stroke-sample
conversion unchanged for this slice. `DrawingToolRouteKind` should remain
temporary compatibility routing and should not gain future tool semantics.

`apps/runenwerk_draw/src/app/state.rs` should route normalized input through
`DrawingToolSession` and apply the emitted `DrawingToolIntent`. It should keep
preview stroke state, preview generation, dirty-start tracking, transaction
commit, dirty invalidation, preview job snapshots, stale job checks, and frame
rebuilds.

`apps/runenwerk_draw/src/app/mod.rs` should add the `tool_session` module and
export only the session types needed by app code or tests.

`apps/runenwerk_draw/src/runtime/ink.rs` should not change behavior in this
slice. Preview jobs, committed product publication, query snapshot publication,
cache behavior, and journals remain as they are.

`apps/runenwerk_draw/tests/app_shell.rs` should not change for this docs task.
The later implementation should add tests only if existing coverage does not
catch a wrapper regression.

`docs-site/src/content/docs/design/implemented/runenwerk-draw-tool-session-architecture-slice.md`
is this design document.

`docs-site/src/content/docs/design/implemented/README.md` should link this document
under App / Tooling.

`docs-site/src/content/docs/apps/runenwerk-draw/README.md` should describe the
current `DrawingInkRuntimeState` split instead of a future split plan.

## Migration Strategy

The future implementation should proceed as a compatibility wrapper:

1. Add `DrawingToolSession`, `DrawingToolSessionKind`, `DrawingToolIntent`, and
   `DrawingToolSessionOutcome`.
2. Keep `DrawingToolInputEvent::from_pointer_with_capture` unchanged.
3. Keep `DrawingToolInputEvent::to_stroke_samples` unchanged.
4. Convert the current `DrawingToolRouteKind` values into equivalent
   `DrawingToolIntent` values.
5. Apply intents in `RunenwerkDrawApp` by reusing the existing preview stroke
   and commit methods.
6. Preserve `RunenwerkDrawApp::dispatch_input` handled/ignored return behavior
   and routed input history.
7. Leave preview tile jobs, committed ink formation, product publication, query
   snapshot publication, cache, and GPU validation untouched.

## Behavior Preservation Requirements

The future implementation must preserve:

- pointer down inside the canvas starts a preview stroke;
- pointer move while captured appends samples;
- pointer up commits exactly one stroke transaction;
- coalesced samples are appended before the current sample in sequence order;
- outside-canvas move/up while captured uses the current unbounded mapping;
- hover, scroll, enter, leave, and ignored inputs do not mutate
  `DrawingDocument`;
- `preview_generation` increments through `RunenwerkDrawApp::mark_preview_dirty`;
- `preview_dirty_start_sample_index` preserves the earliest dirty sample for
  pending preview tile work;
- `next_preview_tile_job_snapshot` and `preview_tile_job_is_current` behavior;
- dirty tile invalidation happens after successful `DrawingTransaction`
  application;
- committed product publication and query snapshot publication continue to gate
  visible committed ink;
- GPU validation and promotion remain unrelated.

## Test Plan

Do not add tests for this docs task.

The future implementation should preserve or add coverage for:

- pointer down/move/up stroke behavior;
- coalesced sample ordering;
- captured outside-canvas movement;
- hover/scroll/ignored inputs;
- preview generation;
- dirty-start sample tracking;
- preview tile jobs;
- committed product publication;
- query snapshot publication;
- dirty tile invalidation after successful transaction.

Validation commands for the future implementation:

```text
cargo test -p runenwerk_draw --test app_shell
cargo test -p runenwerk_draw
cargo test -p drawing --test ink_tile
cargo check --workspace
task docs:validate
git diff --check
```

## Docs Plan

This design is indexed from
`docs-site/src/content/docs/design/implemented/README.md`.

The Runenwerk Draw app README should be corrected only where current code
evidence shows stale wording:

- replace `apps/runenwerk_draw/src/app/ink.rs` with the current
  `apps/runenwerk_draw/src/app/ink/mod.rs` aggregate and focused modules;
- replace the future `DrawingInkRuntimeState` split plan with a current split
  architecture summary.

The pen-first radial tablet UX design already describes the intended
ToolSession boundary and does not need to duplicate this implementation
planning detail.

## Non-Goals

- lasso;
- transform;
- fill;
- eraser mode changes;
- radial menu;
- offhand input;
- cancellation behavior;
- paper response;
- watercolor;
- export/package IO;
- layer UI;
- Workbench integration;
- engine render changes;
- `domain/drawing` authority changes;
- native tablet semantic routing;
- preview tile job changes;
- committed ink product lifecycle changes;
- GPU validation changes.

## Risks And Mitigations

Risk: handled/ignored input behavior changes.
Mitigation: preserve the current `RunenwerkDrawApp::dispatch_input` return
behavior through `DrawingToolSessionOutcome::handled`.

Risk: coalesced sample ordering changes.
Mitigation: keep `DrawingToolInputEvent::to_stroke_samples` unchanged in the
first slice.

Risk: outside-canvas capture regresses.
Mitigation: keep `DrawingToolInputEvent::from_pointer_with_capture` and the
existing `capture_active` calculation unchanged.

Risk: preview dirty tracking changes.
Mitigation: keep `preview_generation`, `preview_dirty_start_sample_index`, and
`RunenwerkDrawApp::mark_preview_dirty` in `RunenwerkDrawApp`.

Risk: product lifecycle changes accidentally.
Mitigation: do not change `apps/runenwerk_draw/src/runtime/ink.rs` behavior in
this slice.

Risk: `DrawingToolRouteKind` keeps growing.
Mitigation: document it as transitional compatibility routing and express
future semantics through `DrawingToolIntent`.

## Implementation Prompt For Later

```text
Implement the first behavior-preserving DrawingToolSession wrapper in
runenwerk_draw.

Use docs-site/src/content/docs/design/implemented/runenwerk-draw-tool-session-architecture-slice.md
as the architecture contract.

Scope:
- Add apps/runenwerk_draw/src/app/tool_session.rs.
- Add DrawingToolSession, DrawingToolSessionKind, DrawingToolIntent, and
  DrawingToolSessionOutcome.
- Keep DrawingToolInputEvent as the normalized app input fact object.
- Keep preview_stroke state, preview generation, dirty-start tracking,
  transaction commit, dirty invalidation, preview tile jobs, product
  publication, query snapshot publication, cache, and GPU validation behavior
  unchanged.
- Keep DrawingCommand and DrawingTransaction domain-owned.
- Treat DrawingToolRouteKind as temporary compatibility routing.
- Do not add lasso, transform, fill, eraser mode changes, radial menu, offhand
  input, cancellation, paper response, watercolor, export/package IO, layer UI,
  Workbench integration, engine render changes, or domain/drawing authority
  changes.

Validation:
- cargo test -p runenwerk_draw --test app_shell
- cargo test -p runenwerk_draw
- cargo test -p drawing --test ink_tile
- cargo check --workspace
- task docs:validate
- git diff --check
```
