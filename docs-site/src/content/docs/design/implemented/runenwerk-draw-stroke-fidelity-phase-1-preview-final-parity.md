---
title: Runenwerk Draw Stroke Fidelity Phase 1 Preview Final Parity
description: Implemented product-based preview/final stroke-visualization architecture for Runenwerk Draw.
status: implemented
owner: drawing
layer: app
canonical: true
last_reviewed: 2026-09-10
related_docs:
  - ../../reports/closeouts/runenwerk-draw-stroke-fidelity-phase-0-1.md
  - ../../apps/runenwerk-draw/README.md
  - ../../apps/runenwerk-draw/roadmap.md
  - ../../domain/drawing/README.md
---

# Runenwerk Draw Stroke Fidelity Phase 1 Preview Final Parity

## Implementation status

This architecture is implemented and has been checked against current Draw presentation/state code and the #492 Draw disposition audit. Domain-formed preview products are the primary formed stroke representation; `UiPrimitive::Stroke` survives only as a bounded, non-authoritative tail for samples not yet covered by preview products and disappears when formed coverage catches up. Pointer-up/released preview visibility remains bridged until committed replacement passes the existing product/query acceptance barriers.

The proposed `ActiveStrokePreviewSession` name and other type-level decomposition below were directional rather than required API spellings. Current code and tests own exact behavior. Sections describing the “current” lifecycle or investigation findings record the design-time baseline and are retained as rationale. Paper Response Phase 6A remains separately deferred and is not activated by this promotion.

## Summary

Stroke Fidelity Phase 1 plans the long-term stroke visualization architecture for eliminating the split between app-local immediate stroke polylines and domain-formed CPU ink products.

The previous sample-count watermark plus immediate tail overlay approach is no longer the main solution. Sample-count or range metadata may still exist as scheduling and progress metadata inside the active preview pipeline, but it must not define visual truth. Active preview should be represented primarily by domain-formed preview ink products, and committed output should replace provisional preview products through the existing product/query barriers.

The sections below retain the original implementation-planning contract. Current code/tests own behavior; this design does not alter `ProductPublication` or `QuerySnapshotPublication` semantics, implement domain smoothing, or activate Paper Response Phase 6A.

## Investigation Findings

- `apps/runenwerk_draw/src/app/presentation.rs::push_immediate_stroke` renders active or released preview samples as `UiPrimitive::Stroke`, which creates an app-local stroke geometry path separate from domain tile formation.
- `apps/runenwerk_draw/src/app/presentation.rs::build_workspace_frame_with_ink_surface_refs_and_stroke` and `build_workspace_frame_with_ink_refs_and_stroke` are the presentation paths that decide how committed products, preview products, and immediate stroke projection are composed.
- `apps/runenwerk_draw/src/app/state.rs::RunenwerkDrawApp::commit_preview_stroke` commits preview strokes through `DrawingCommand::{BeginStroke, AppendStrokeSample, CommitStroke}` inside a `DrawingTransaction`, then transitions preview state after pointer-up.
- `apps/runenwerk_draw/src/app/state.rs::RunenwerkDrawApp::freeze_preview_after_commit` and `clear_preview_after_committed_acceptance` currently define the released-preview-to-committed-product bridge.
- `apps/runenwerk_draw/src/app/state.rs` owns preview stroke samples, dirty-start tracking, stale preview job checks, and released-preview coordination today.
- `apps/runenwerk_draw/src/app/ink/preview.rs` owns formed preview tile products, preview diagnostics, and the latest dirty preview tile count.
- `apps/runenwerk_draw/src/runtime/ink_jobs.rs::DrawingInkTileJobBatch::from_snapshot` forms preview and committed tile products through domain tile formation.
- `apps/runenwerk_draw/src/runtime/ink.rs::process_drawing_preview_ink_jobs` records preview products back into app ink state, while `publish_drawing_ink_products` and `publish_drawing_ink_query_snapshots` own the committed product/query lifecycle.
- `domain/drawing/src/tile/formation.rs` owns deterministic CPU ink tile formation. Phase 1 should converge presentation toward this domain output instead of refining an app-local polyline path.
- `apps/runenwerk_draw/tests/app_shell.rs` covers preview job snapshots, compatible lagging preview output handling, dirty-start tracking, committed product publication, and query snapshot publication. Phase 1 tests should preserve those barriers while changing the intended projection lifecycle.

## Problem Statement

Runenwerk Draw currently has two visible stroke paths:

1. App-local immediate preview: `apps/runenwerk_draw/src/app/presentation.rs::push_immediate_stroke` projects raw preview samples as `UiPrimitive::Stroke`.
2. Domain output: `domain/drawing/src/tile/formation.rs` forms preview and final ink as deterministic CPU tile products.

That split causes visible stroke changes after commit and product/query acceptance. It also makes long strokes expensive if the app keeps projecting an ever-growing immediate polyline every frame.

The durable fix is not a watermark/tail hack. Phase 1 should move active and released preview visibility toward domain-formed preview products, while later phases extract and version the shared domain stroke visualization contract.

## Corrected Architecture

The long-term stroke visualization and ink formation contract should be owned by `domain/drawing`:

```text
Raw StrokeSample stream
  -> StrokeSampleTimeline
  -> StrokeReconstructedPath
  -> BrushDabStream
  -> Preview/Final InkTileProducts
```

The exact type names may change during implementation, but ownership must not:

- `domain/drawing` owns stroke geometry, reconstruction, brush dab placement, deterministic tile formation, and later paper response composition.
- `apps/runenwerk_draw` owns interaction/session state, active preview product lifecycle, provisional visibility, and product-surface projection.
- `engine/render` must not own stroke smoothing or drawing truth.
- App presentation must not own final stroke geometry truth.

## Current Lifecycle

Current active drawing behavior is structurally safe but visually split:

1. Pointer input becomes `DrawingToolInputEvent` and is routed through `DrawingToolSession`.
2. `RunenwerkDrawApp` appends preview samples and schedules preview tile work.
3. App presentation can project the full active or released preview sample list as `UiPrimitive::Stroke`.
4. Preview tile jobs form products asynchronously through domain tile formation.
5. Pointer-up commits a `DrawingTransaction`.
6. Preview/released state bridges visibility until committed products and query snapshots are accepted.
7. Committed products become visible through `ProductPublication` and `QuerySnapshotPublication`.

The problem is representation parity: the app-local polyline and domain tile products are not the same visual contract.

## Target Lifecycle

Target Phase 1 lifecycle:

1. Ordered samples enter an app-owned active preview session.
2. The app computes dirty tile ids or dirty regions for new sample ranges.
3. Preview tile jobs form domain preview products for affected tiles.
4. Accumulated preview products are projected as the primary active stroke representation.
5. `UiPrimitive::Stroke`, if retained, is only a short-lived non-authoritative ghost for the first frame or a not-yet-formed micro-region.
6. Pointer-up freezes current preview products as provisional visible products.
7. Final committed products replace provisional products only after the existing `ProductPublication` and `QuerySnapshotPublication` acceptance path.

This keeps CPU tile products as drawing truth while preserving first-frame responsiveness.

## Product-Based Active Preview

Active drawing preview should be represented primarily by domain-formed preview ink products.

Phase 1 should introduce or formalize an app-owned `ActiveStrokePreviewSession` concept that tracks:

- active stroke id;
- ordered sample count and sample ranges;
- dirty tile ids or dirty regions;
- preview product generation;
- provisional visible preview products;
- frozen preview products after pointer-up;
- replacement state for final accepted products.

Sample-count and range tracking may exist inside this session as scheduling metadata. It must not define a second visual truth or authorize a full accumulated app-local tail polyline.

## Pointer-Up Provisional Preview Flow

Pointer-up should not clear preview products and fall back to a UI polyline. The target flow is:

```text
pointer-up
  -> commit DrawingTransaction
  -> freeze current preview products as provisional visible products
  -> stop active preview intake / ignore stale preview jobs
  -> keep provisional products visible
  -> form/publish final committed products
  -> replace provisional products after existing ProductPublication + QuerySnapshotPublication acceptance
```

The replacement must be atomic at the product/query lifecycle level: provisional products stay visible until accepted final products can replace them.

## UiPrimitive Stroke Demotion

`UiPrimitive::Stroke` may remain only as an optional ultra-low-latency ghost:

- no longer the main active stroke representation;
- non-authoritative;
- short-lived;
- not the full accumulated stroke;
- used only for the first frame or a not-yet-formed micro-region;
- eventually generated from the same domain stroke reconstruction contract if retained.

The app should not refine `UiPrimitive::Stroke` into a parallel stroke renderer. Any real stroke smoothing, reconstruction, or dab semantics belong in `domain/drawing`.

## Incremental Formation And Performance

Long strokes must not require projecting the full accumulated sample list every frame. The intended incremental preview formation flow is:

```text
ordered samples enter ActiveStrokePreviewSession
  -> dirty tile ids/regions computed from new sample ranges
  -> preview tile jobs update affected tiles
  -> accumulated preview products stay visible as product surfaces
  -> renderer uploads only changed dynamic texture targets
```

This keeps the app's responsibility focused on scheduling, provisional visibility, and projection of product surfaces. Domain formation remains responsible for the pixels.

## Product/Query Barrier Preservation

Phase 1 must not change publication authority:

- `ProductPublication` still controls committed product visibility.
- `QuerySnapshotPublication` still controls query snapshot acceptance.
- Dirty tiles still clear only after the accepted product/query lifecycle.
- Failed generations preserve last-good committed visibility.
- Preview and provisional products remain app-visible proof/provisional surfaces, never document truth.

The change is presentation and preview lifecycle, not publication authority.

## Phase Split

Recommended order:

1. **Phase 1A: Product-Based Preview Lifecycle**
   Introduce active/provisional preview product visibility, freeze preview products after pointer-up, and stop relying on a full accumulated `UiPrimitive::Stroke`.
2. **Phase 1B: Domain Contract Extraction**
   Extract or name the domain-owned `StrokeSampleTimeline`, `StrokeReconstructedPath`, and `BrushDabStream` contracts if current formation code needs clearer boundaries for shared preview/final semantics.
3. **Phase 1C: Optional Low-Latency Ghost**
   Keep or reintroduce a tiny first-frame ghost only if it is generated from the same domain stroke reconstruction contract or is explicitly documented as temporary non-authoritative latency UI.
4. **Phase 2: Reconstruction/Smoothing**
   Add higher-order smoothing/reconstruction as a deterministic, versioned domain policy.
5. **Phase 6A: Paper Response**
   Resume paper response only after product-based preview/final parity is stable.

## File-by-File Plan

`apps/runenwerk_draw/src/app/ink/preview.rs`

- Introduce or formalize `ActiveStrokePreviewSession` state.
- Track ordered sample ranges, dirty regions, preview product generations, provisional products, and final replacement state.
- Reject incompatible or stopped-session preview jobs while allowing compatible
  lagging active preview output to advance formed sample coverage.
- Keep dirty-start sample tracking.
- Freeze preview products after pointer-up instead of falling back to released full polyline projection.

`apps/runenwerk_draw/src/app/presentation.rs`

- Project active and provisional preview products as primary stroke surfaces.
- Demote `push_immediate_stroke` to optional non-authoritative ghost projection.
- Ensure no full accumulated `UiPrimitive::Stroke` is projected once preview products exist.
- Keep presentation limited to visibility/projection decisions, not stroke geometry truth.

`apps/runenwerk_draw/src/app/state.rs`

- Keep `RunenwerkDrawApp` as coordinator for input, preview state, transaction commit, dirty invalidation, preview jobs, product lifecycle, and frame rebuilds.
- Adjust pointer-up coordination to freeze provisional preview products while final products are forming.
- Preserve `commit_preview_stroke` transaction behavior.

`apps/runenwerk_draw/src/runtime/ink_jobs.rs`

- Preserve the domain tile formation entry point.
- Carry enough preview generation/sample-range metadata to reject incompatible
  jobs, apply compatible lagging output only as forward progress, and update
  only affected preview products.
- Do not introduce app-local stroke reconstruction here.

`apps/runenwerk_draw/src/runtime/ink.rs`

- Preserve preview job processing, product publication, and query snapshot publication barriers.
- Record active/provisional preview products without promoting them to document truth.
- Clear provisional products only after accepted final replacement.

`apps/runenwerk_draw/tests/app_shell.rs`

- Update lifecycle expectations to product-based active preview.
- Preserve existing tests for stale preview jobs, dirty-start tracking, `ProductPublication`, and `QuerySnapshotPublication`.

`domain/drawing/src/tile/formation.rs`

- Remains the deterministic CPU tile formation authority.
- Phase 1B may extract named timeline/reconstruction/dab contracts around existing logic if needed.
- Phase 2 owns higher-order smoothing/reconstruction policy.

## Test Plan

Eventual implementation should add or update focused tests for:

- active preview products are visible during drawing;
- pointer-up preserves preview products until final accepted products replace them;
- no full accumulated `UiPrimitive::Stroke` is projected once preview products exist;
- final accepted products replace provisional products without a geometry jump beyond preview/final quality or profile differences;
- long strokes do not rebuild or project an unbounded UI polyline every frame;
- `ProductPublication` and `QuerySnapshotPublication` behavior remains unchanged;
- incompatible or stopped-session preview jobs cannot replace current
  provisional preview products;
- compatible lagging active preview jobs can advance product coverage but must
  not regress formed sample coverage;
- domain preview and final formation share stroke reconstruction and brush dab semantics;
- existing pointer down/move/up stroke behavior remains unchanged.

## Non-Goals

- No paper response.
- No watercolor or wet simulation.
- No GPU-only preview truth.
- No renderer-owned stroke smoothing.
- No app-local smoothing.
- No product/query lifecycle rewrite.
- No new tools.
- No radial, offhand, or cancellation behavior.
- No layer UI, export/package IO, or Workbench integration.

## Risks And Mitigations

- Risk: provisional products become a second source of document truth.
  Mitigation: keep them app-visible only and replace them only after existing product/query acceptance.
- Risk: app presentation starts owning stroke geometry.
  Mitigation: presentation may choose which product surfaces or ghosts to show, but geometry, reconstruction, and dab placement stay in `domain/drawing`.
- Risk: first-frame responsiveness regresses if `UiPrimitive::Stroke` is removed too early.
  Mitigation: keep a tiny optional ghost for first-frame latency only, explicitly non-authoritative and not a full accumulated stroke.
- Risk: stale preview products appear after pointer-up.
  Mitigation: keep preview generation/sample-range checks, apply compatible
  lagging output only while the same preview stroke is active, and ignore jobs
  after active preview intake stops.
- Risk: long strokes still rebuild too much presentation state.
  Mitigation: project accumulated preview products as surfaces and upload only changed dynamic texture targets.

## Implementation Prompt For Later

Implement Stroke Fidelity Phase 1A: product-based active preview lifecycle and provisional preview visibility for Runenwerk Draw.

Scope:

- `apps/runenwerk_draw` primary scope.
- No engine render changes unless implementation proves a product-surface projection gap.
- No domain smoothing/reconstruction policy yet.
- No paper response, watercolor, new tools, radial/offhand/cancel behavior, or product/query lifecycle rewrite.

Requirements:

- Represent active preview primarily with domain-formed preview ink products.
- Introduce or formalize `ActiveStrokePreviewSession` with active stroke id, ordered sample ranges, dirty regions, preview generation, provisional products, frozen pointer-up products, and final replacement state.
- On pointer-up, commit the `DrawingTransaction`, freeze current preview products, stop active preview intake, ignore stale preview jobs, keep provisional products visible, and replace them only after existing `ProductPublication` plus `QuerySnapshotPublication` acceptance.
- Demote `UiPrimitive::Stroke` to an optional short-lived non-authoritative ghost, not a full accumulated stroke.
- Preserve dirty-start sample tracking, incompatible preview-job rejection,
  compatible lagging preview forward progress, committed publication, query
  snapshot publication, CPU tile formation truth, and GPU validation behavior.
- Add focused app-shell and domain tests for active preview product visibility, pointer-up provisional preservation, no unbounded UI polyline projection, unchanged product/query barriers, and shared preview/final domain formation semantics.

Validation:

```text
cargo test -p runenwerk_draw --test app_shell
cargo test -p runenwerk_draw
cargo test -p drawing --test ink_tile
cargo check --workspace
task docs:validate
git diff --check
```
