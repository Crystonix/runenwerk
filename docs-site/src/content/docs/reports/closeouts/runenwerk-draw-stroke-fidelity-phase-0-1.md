---
title: Runenwerk Draw Stroke Fidelity Phase 0/1
description: Historical repair-plan evidence for the completed Draw input-ordering and preview/final-parity work.
status: completed
owner: drawing
layer: reports
canonical: false
last_reviewed: 2026-09-10
related_docs:
  - ../../apps/runenwerk-draw/README.md
  - ../../apps/runenwerk-draw/roadmap.md
  - ../../design/implemented/runenwerk-draw-stroke-fidelity-phase-1-preview-final-parity.md
  - ../../design/deferred/runenwerk-draw-paper-response-phase-6a.md
  - ../../domain/drawing/README.md
  - ../../adapters/native-tablet-input/README.md
---

# Runenwerk Draw Stroke Fidelity Phase 0/1

## Historical status

This completed repair plan is retained as historical evidence for the input-ordering and preview/final-parity work. Current behavior is owned by code/tests and the durable Phase 1 architecture is retained under implemented designs. Paper Response Phase 6A is deferred separately. The planning language below is point-in-time rationale and does not authorize active work.

## Summary

This document plans the behavior-safe stroke fidelity repair that must happen
before Paper Response Phase 6A resumes.

The current visible problem has two separate causes:

- the Winit mouse fallback can combine frame-level button transitions and
  cursor samples in an order that creates early stroke kinks;
- active immediate preview is drawn through `UiPrimitive::Stroke`, while
  preview and committed ink products are formed by deterministic CPU tile
  rasterization in `domain/drawing`.

Paper Response Phase 6A is paused until Phase 0 input ordering and Phase 1
preview/final visual parity are stable. CPU tile formation remains drawing
truth. `ProductPublication` and `QuerySnapshotPublication` barriers must remain
unchanged.

Recommended implementation order:

1. Phase 0: fix Winit fallback sample ordering.
2. Phase 1: improve preview/final visual parity without changing product or
   query barriers.
3. Phase 2: add deterministic domain-owned stroke reconstruction/smoothing if
   straight segment rasterization still limits hand feel after Phase 0/1.

## Investigation Findings

`apps/runenwerk_draw/src/runtime/systems.rs::route_draw_input_system` currently
builds fallback mouse events from frame aggregate state. It reads
`InputState::mouse_position`, `InputState::mouse_motion_samples`, button
pressed/released flags, and dispatches separate `PointerEventKind::Down`,
`Move`, and `Up` events into `RunenwerkDrawApp::dispatch_input`.

`apps/runenwerk_draw/src/runtime/systems.rs::pointer_motion_packet` uses
`samples.split_last()`: the last `MouseMotionSample` becomes the current move
event position and earlier samples become coalesced samples.

`engine/src/plugins/input/state.rs::InputState::handle_cursor_moved` updates
the single frame-level `mouse_position` and appends to
`mouse_motion_samples`. `engine/src/plugins/input/state.rs::InputState::handle_mouse_input`
records button state transitions but does not record the cursor position or
motion-sample index at the moment of the transition.

`apps/runenwerk_draw/src/runtime/systems.rs::coalesce_pointer_move_events`
coalesces native tablet move bursts after explicit native events have already
preserved event order. This path is different from the Winit fallback aggregate
path.

`apps/runenwerk_draw/src/app/input.rs::DrawingToolInputEvent::to_stroke_samples`
appends coalesced samples first and then the current sample. That behavior is
correct for ordered packets, but it faithfully preserves bad ordering if an
upstream packet contains pre-contact or out-of-window samples after a down.

`apps/runenwerk_draw/src/app/state.rs::RunenwerkDrawApp::commit_preview_stroke`
commits preview strokes through `DrawingTransaction` and
`DrawingCommand::{BeginStroke, AppendStrokeSample, CommitStroke}`. It then
invalidates dirty tiles through the drawing-owned invalidation path.

`apps/runenwerk_draw/src/app/state.rs::RunenwerkDrawApp::immediate_stroke_projection`
projects the current or just-released preview stroke when
`preview_stroke_visible` is true.

`apps/runenwerk_draw/src/app/presentation.rs::push_immediate_stroke` renders
that projection as `UiPrimitive::Stroke`. This is a UI primitive, not a formed
ink product and not drawing truth.

`apps/runenwerk_draw/src/app/presentation.rs::build_workspace_frame_with_ink_surface_refs_and_stroke`
projects committed product surfaces and only projects preview product surfaces
when there is no immediate stroke overlay. This avoids double-composition but
also means the active visible stroke shape is usually the UI stroke primitive.

`apps/runenwerk_draw/src/runtime/ink.rs::process_drawing_preview_ink_jobs`
forms preview tile catch-up products asynchronously. `apps/runenwerk_draw/src/runtime/ink.rs::publish_drawing_ink_products`
forms committed products. `apps/runenwerk_draw/src/runtime/ink.rs::publish_drawing_ink_query_snapshots`
accepts query snapshots and calls
`RunenwerkDrawApp::clear_preview_after_committed_acceptance`.

`domain/drawing/src/tile/formation.rs::DrawingTileFormationPolicy::preview`
and `domain/drawing/src/tile/formation.rs::DrawingTileFormationPolicy::final_quality`
use different output profiles: preview products are lower-resolution than final
products. Both still route through deterministic CPU tile formation.

`domain/drawing/src/tile/formation.rs::rasterize_stroke` and
`domain/drawing/src/tile/formation.rs::append_segment_dabs` rasterize the
stored stroke samples with straight segment dab placement. There is no
domain-owned higher-order stroke reconstruction or smoothing policy yet.

`domain/drawing/tests/ink_tile.rs::preview_tile_formation_uses_committed_raster_payload_behavior`
proves preview and committed formation share raster payload behavior for the
same policy.

`domain/drawing/tests/ink_tile.rs::sparse_fast_segment_deposits_ink_between_input_samples`
proves sparse segments deposit ink between input samples, but it does not prove
smooth reconstruction.

`domain/drawing/tests/ink_tile.rs::preview_and_final_tile_identity_include_quality_and_cache_identity`
proves preview and final quality profiles are distinct product/cache identity
inputs.

`apps/runenwerk_draw/tests/app_shell.rs::coalesced_pointer_samples_append_ordered_preview_samples_before_current_sample`,
`apps/runenwerk_draw/tests/app_shell.rs::window_touch_history_routes_as_ordered_preview_samples`,
and `apps/runenwerk_draw/tests/app_shell.rs::native_tablet_move_burst_routes_as_one_coalesced_preview_update`
cover ordered coalesced input after packets are already in a coherent order.
They do not cover the Winit fallback case where same-frame mouse movement and
button transitions are aggregated before routing.

`apps/runenwerk_draw/tests/app_shell.rs::released_preview_products_stay_visible_until_committed_replacement`
and
`apps/runenwerk_draw/tests/app_shell.rs::committed_ink_products_publish_snapshot_and_become_visible_only_after_barriers`
encode the current lifecycle: after release, the immediate stroke primitive
remains visible until committed products are accepted by product and query
barriers, after which the primitive disappears and product surfaces become
visible.

`docs-site/src/content/docs/apps/runenwerk-draw/README.md` documents that
pointer feedback uses `UiPrimitive::Stroke`, while preview and committed ink
tile products are formed asynchronously and published through product/query
barriers.

`docs-site/src/content/docs/apps/runenwerk-draw/roadmap.md` documents the
current rendering foundation as `UiPrimitive::Stroke` immediate feedback plus
deterministic CPU preview/final tile products.

`docs-site/src/content/docs/design/active/runenwerk-draw-paper-response-phase-6a.md`
plans paper response as a visible drawing-quality slice. That work remains
valid, but it should wait until stroke fidelity no longer changes unexpectedly
between input, active preview, release, and accepted products.

## Root Cause Analysis

Root cause 1 is input ordering in the Winit fallback path. The engine input
state records cursor movement samples and button transitions as frame aggregate
facts, not as one ordered pointer event stream. A frame can therefore contain a
left-button press and motion samples without enough information to know which
motion samples happened before contact. Draw then routes a down at the current
frame cursor position and a move whose coalesced samples may include positions
that should not follow that down. The result is an early kink or backtrack that
the app correctly preserves as stroke truth.

Root cause 2 is preview/final representation mismatch. Active preview is a
screen-space UI stroke primitive with one width and a polyline point list.
Preview and committed products are domain CPU tile products formed
asynchronously with dab rasterization and different preview/final quality
profiles. When the product/query barriers accept committed output, the UI
stroke disappears and product surfaces become visible. If the two
representations do not match closely, the user sees the stroke change after
commit.

Root cause 3 is the absence of a domain-owned stroke reconstruction policy.
Even after ordering is correct and preview/final projection is aligned, raw
point-to-point segment rasterization can still feel rough during fast or sparse
input. Any smoothing that changes final ink pixels must belong to
`domain/drawing`, not to app presentation.

## Phase 0: Winit Fallback Sample Ordering

Phase 0 must run before visual parity work. A stable preview cannot be built on
bad sample order.

Goal:

- ensure fallback mouse input never turns pre-contact or out-of-window cursor
  samples into post-contact coalesced stroke samples;
- preserve ordered native tablet and touch behavior;
- keep `DrawingToolInputEvent::to_stroke_samples` semantics unchanged for
  already ordered packets.

Preferred implementation shape:

- add focused coverage for a same-frame Winit mouse press plus motion samples;
- fix the Winit fallback route in
  `apps/runenwerk_draw/src/runtime/systems.rs::route_draw_input_system` only if
  it can be made correct from existing `InputState` facts;
- otherwise, move the root fix to
  `engine/src/plugins/input/state.rs::InputState` by recording event-ordered
  pointer/button transition facts with transition-local cursor positions.

The long-term-correct contract is an event-ordered input stream or enough
transition metadata to reconstruct one. `ui_input` packet normalization should
carry ordering facts; it should not guess them after the source order is lost.

Phase 0 should not:

- change stroke smoothing;
- change preview/final product policy;
- change product publication or query snapshot barriers;
- silently drop valid post-contact samples from native tablet or touch input.

Acceptance criteria:

- same-frame fallback mouse down plus motion does not create a preview sample
  sequence that backtracks behind the down position;
- same-frame fallback mouse up plus motion does not duplicate or reorder the
  final sample;
- native tablet coalesced move tests and touch history tests still pass;
- pointer down/move/up still commits exactly one stroke.

## Phase 1: Preview/Final Visual Parity

Phase 1 should make active preview and accepted product output visually agree
without changing drawing truth or product barriers.

Policy:

- CPU tile formation remains canonical drawing output.
- `UiPrimitive::Stroke` may remain only as an ultra-low-latency overlay.
- `UiPrimitive::Stroke` must not become the authoritative preview shape.
- App presentation must not own final stroke smoothing semantics.
- Product/query barriers must remain unchanged.

Recommended direction:

- keep immediate `UiPrimitive::Stroke` for first-frame responsiveness;
- prefer CPU preview tile products for formed, stable preview regions as soon
  as they are available;
- restrict the immediate overlay to the part of the stroke that has not yet
  been represented by current preview products, or prove an equivalent policy
  that avoids visible shape replacement;
- keep released preview stable until committed products are accepted, but make
  that stable preview use the closest available domain-formed representation
  rather than a full UI polyline whenever possible.

The current implementation deliberately suppresses preview product surfaces
while an immediate stroke exists to avoid double-composition. Phase 1 should
replace that binary choice with an explicit preview visibility policy:

- product surfaces represent domain-formed preview output;
- immediate overlay represents only low-latency unformed samples;
- committed product surfaces replace preview surfaces only through accepted
  product/query lifecycle.

Phase 1 should not force preview products to become final-quality products
unless profiling and tests show the quality mismatch is the remaining visible
issue. Preview and final quality are currently part of product identity and
cache identity, so any quality-policy change must be explicit and tested.

Acceptance criteria:

- active strokes keep first-frame feedback;
- preview tile catch-up products can become visible without double-drawing the
  same stroke section;
- released preview does not continue changing through stale preview jobs;
- accepted committed products still become visible only after product and query
  barriers;
- users should not see a surprising shape change when the immediate overlay
  disappears after accepted products arrive.

## Phase 2: Domain Stroke Reconstruction/Smoothing

Phase 2 is not part of the first repair unless Phase 0 and Phase 1 prove that
straight segment rasterization is still the dominant fidelity problem.

If needed, smoothing must be deterministic and domain-owned. The app may ask
for a low-latency projection of the same reconstructed path, but it must not
invent independent smoothing that differs from committed CPU tile formation.

Possible domain-owned model:

- a `StrokeReconstructionPolicy` or equivalent formation policy component;
- deterministic sample filtering/resampling before dab placement;
- pressure, tilt, width, opacity, and flow interpolation rules tied to the same
  reconstructed path;
- formation/cache identity that includes the reconstruction policy and version.

Acceptance criteria:

- same input stroke plus same policy produces identical products;
- changed reconstruction policy changes determinism and cache identity;
- tile invalidation bounds still cover every affected pixel;
- app preview overlay, if it uses reconstructed geometry, uses the same
  domain-owned reconstruction output or a documented low-latency approximation
  that is replaced by domain output.

## File-by-File Plan

`engine/src/plugins/input/state.rs`

- Phase 0 may need event-ordered pointer facts or button-transition metadata if
  the app cannot reconstruct correct order from current frame aggregates.
- If changed, keep this as input-state semantics only, not engine render work.

`apps/runenwerk_draw/src/runtime/systems.rs`

- Phase 0 should first inspect and test
  `route_draw_input_system`, `pointer_motion_packet`, and
  `coalesce_pointer_move_events`.
- Winit fallback should avoid routing pre-contact motion as post-contact
  coalesced stroke data.
- Native tablet and touch routing should remain ordered and unchanged except for
  additional regression coverage.

`apps/runenwerk_draw/src/app/input.rs`

- Keep `DrawingToolInputEvent` as normalized app input facts.
- Keep `DrawingToolInputEvent::to_stroke_samples` coalesced-first behavior for
  ordered packets.
- Do not use this layer to guess chronological order after the source has lost
  it.

`apps/runenwerk_draw/src/app/state.rs`

- Phase 1 should review `immediate_stroke_projection`,
  `clear_preview_after_committed_acceptance`, `next_preview_tile_job_snapshot`,
  `preview_tile_job_is_current`, and `rebuild_last_frame`.
- Preview stroke state and product lifecycle coordination can stay in
  `RunenwerkDrawApp` until moving it is proven lower risk.

`apps/runenwerk_draw/src/app/presentation.rs`

- Phase 1 should replace the binary "immediate stroke hides preview products"
  policy with a parity-oriented policy.
- This file may project the chosen app/domain products and low-latency overlay,
  but it must not own final smoothing semantics.

`apps/runenwerk_draw/src/runtime/ink.rs`

- Preserve `process_drawing_preview_ink_jobs`,
  `publish_drawing_ink_products`, and
  `publish_drawing_ink_query_snapshots` barrier behavior.
- Phase 1 may adjust when preview products are considered visible, but accepted
  committed visibility still depends on product and query snapshot acceptance.

`apps/runenwerk_draw/src/runtime/ink_jobs.rs`

- Keep preview and committed CPU job execution as the domain tile formation
  bridge.
- Do not add renderer-only stroke fidelity behavior here.

`domain/drawing/src/tile/formation.rs`

- Phase 0 and Phase 1 should avoid formation changes unless tests prove they
  are necessary.
- Phase 2 belongs here if deterministic stroke reconstruction/smoothing becomes
  required.

`domain/drawing/tests/ink_tile.rs`

- Add Phase 2 tests only when domain reconstruction/smoothing is implemented.
- Existing formation determinism and preview/final identity tests are the
  baseline to preserve.

`apps/runenwerk_draw/tests/app_shell.rs`

- Add Phase 0 Winit fallback input-order regression tests.
- Add Phase 1 preview/product parity lifecycle tests.
- Preserve existing product/query barrier assertions.

`docs-site/src/content/docs/apps/runenwerk-draw/README.md`

- Update only after implementation changes behavior or closes the known gap.
- It should continue to state that CPU tile products are drawing truth.

`docs-site/src/content/docs/apps/runenwerk-draw/roadmap.md`

- Update after the repair is implemented and validated to record stroke
  fidelity as the gate before Paper Response Phase 6A.

`docs-site/src/content/docs/design/active/runenwerk-draw-paper-response-phase-6a.md`

- Keep the paper response design, but treat it as paused until stroke fidelity
  is stable.

## Test Plan

Phase 0 tests:

- fallback mouse press and same-frame motion route in chronological order;
- fallback mouse press does not append pre-contact motion behind the down
  sample;
- fallback mouse release and same-frame motion do not duplicate or reorder the
  final sample;
- native tablet coalesced move bursts still route as ordered preview samples;
- touch history still routes as ordered preview samples;
- pointer down/move/up still commits exactly one stroke transaction.

Phase 1 tests:

- active preview still has immediate first-frame feedback;
- preview products can be visible for formed regions without double-drawing the
  immediate overlay;
- preview tile job stale checks still reject stale generations;
- released preview remains stable and does not continue changing through
  preview tile jobs;
- committed products become visible only after product and query snapshot
  acceptance;
- accepted committed output removes the immediate overlay without a surprising
  representation swap.

Phase 2 tests, only if implemented:

- reconstructed stroke formation is deterministic;
- reconstruction policy participates in formation and cache identity;
- sparse or noisy sample inputs produce smoother output without losing
  pressure/order semantics;
- tile invalidation bounds cover reconstructed geometry.

Validation commands for eventual implementation:

```text
cargo test -p runenwerk_draw --test app_shell
cargo test -p runenwerk_draw
cargo test -p drawing --test ink_tile
cargo check --workspace
task docs:validate
git diff --check
```

## Non-Goals

- Paper Response Phase 6A implementation.
- Watercolor or wet media simulation.
- New tools, lasso, transform, fill, eraser-mode changes, radial menu, or
  offhand behavior.
- Engine render semantics changes.
- Renderer-owned stroke smoothing.
- App-presentation-owned final stroke smoothing.
- GPU authority over drawing truth.
- Product publication lifecycle changes.
- Query snapshot lifecycle changes.
- CPU tile authority changes.
- Package IO, export, layer UI, or Workbench integration.

## Risks And Mitigations

Risk: fixing Winit fallback in the app may only mask an engine input-order
contract gap.

Mitigation: write the regression test around the user-visible route and move
the implementation to `InputState` if current app-level facts cannot prove
chronological order.

Risk: making preview products visible during active drawing can double-draw the
same stroke section.

Mitigation: make the preview visibility policy explicit: formed product
surfaces own formed regions, while immediate overlay owns only unformed
low-latency samples.

Risk: final-quality products still look different from preview products because
preview products use a lower-resolution profile.

Mitigation: first align representation semantics. Only change preview/final
quality policy if tests and visual proof show resolution profile mismatch is
the remaining issue.

Risk: adding smoothing in app presentation gives active preview a different
shape than committed ink.

Mitigation: defer smoothing to Phase 2 and implement it in `domain/drawing`,
with app preview consuming the same reconstructed semantics or an explicitly
temporary low-latency overlay.

Risk: product/query barrier changes hide the existing lifecycle guarantees.

Mitigation: preserve the existing app-shell tests that require committed ink to
become visible only after both product publication and query snapshot
acceptance.

## Implementation Prompts For Later

Phase 0 prompt:

```text
Implement Runenwerk Draw Stroke Fidelity Phase 0: Winit fallback sample
ordering. Preserve current stroke behavior except for fixing chronological
ordering. Start with failing tests in apps/runenwerk_draw/tests/app_shell.rs
for same-frame fallback mouse press/motion and release/motion ordering. If
apps/runenwerk_draw/src/runtime/systems.rs cannot reconstruct correct order
from current InputState facts, add event-ordered pointer transition metadata in
engine/src/plugins/input/state.rs. Do not change preview/final product
lifecycle, CPU tile formation, smoothing, or renderer behavior.
```

Phase 1 prompt:

```text
Implement Runenwerk Draw Stroke Fidelity Phase 1: preview/final visual parity.
Keep ProductPublication and QuerySnapshotPublication barriers unchanged. Keep
CPU tile products as drawing truth. Let UiPrimitive::Stroke remain only an
ultra-low-latency overlay while domain-formed preview products represent formed
preview regions. Do not add smoothing in app presentation. Add focused
app_shell tests proving first-frame feedback, no preview/product
double-composition, stable released preview, and committed visibility only
after product/query acceptance.
```

Phase 2 prompt:

```text
Plan and then implement domain-owned deterministic stroke reconstruction if
Phase 0 and Phase 1 leave straight segment rasterization as the remaining
stroke fidelity bottleneck. Keep reconstruction in domain/drawing CPU tile
formation, include the policy in determinism/cache identity, and keep app
presentation from owning final smoothing semantics.
```
