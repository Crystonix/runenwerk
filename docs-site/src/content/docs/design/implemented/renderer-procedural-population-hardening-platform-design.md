---
title: Renderer Procedural Population Hardening Platform
description: Implemented design for fail-closed indirect draw contracts, reusable GPU primitive dispatch, graph-level fixed-step catch-up, and reusable procedural camera projection after the procedural population runtime proof.
status: implemented
owner: engine
layer: engine-runtime / renderer / procedural
canonical: true
last_reviewed: 2026-09-10
related_designs:
  - renderer-procedural-population-platform-design.md
  - ../accepted/renderer-gpu-evidence-and-procedural-visuals-design.md
  - ../accepted/renderer-scale-residency-and-gpu-driven-visibility-design.md
  - ../accepted/runenrender-decomposition-design.md
related_reports:
  - ../../reports/closeouts/pt-render-procedural-population-runtime-proven/closeout.md
---

# Renderer Procedural Population Hardening Platform

## Implementation status

This hardening architecture is implemented and backed by the completed runtime-proven hardening track. The design-time gap and production-slice language below is retained as rationale; current code, tests, and closeout evidence own exact implementation state. Where broader renderer acceptance terminology conflicts, ADR 0021 and the accepted RunenRender architecture win. Final operational conformance and residual-audit authority now belongs to the RunenRender R8 -> RX sequence rather than the historical `PT-RENDER-PERFECTION` track.

## Decision

`PT-RENDER-PROCEDURAL-POPULATION-HARDENING` closes the direct technical gaps
left visible by the `PT-RENDER-PROCEDURAL-POPULATION` runtime-proven closeout.
This is a focused hardening track, not a cleanup bucket and not a replacement
for current RunenRender R8 -> RX conformance authority.

The renderer must harden four production contracts before later population
expansion:

- indirect draw submission must be typed, bounds-checked, indexed-aware, and
  fail-closed before runtime submission;
- GPU primitives must lower into reusable renderer-owned shader dispatches,
  including hierarchical prefix scan for arbitrary counts;
- fixed-step catch-up must become graph scheduling with bounded repeated pass
  execution, iteration-scoped uniform projection, and runtime fixed-time source
  reuse rather than example-local timing logic;
- procedural 2D camera projection must become a reusable derived renderer
  contract, so examples can fill the target without letterbox, non-uniform
  stretch, or viewport-dependent simulation truth.

Spatial hash and chunked unbounded populations are intentionally outside this
track. They require a separate intake and design because they add collision
policy, chunk residency, world-coordinate windows, and product ownership
questions that are not direct hardening of the bounded population platform.

Richer boids split/merge behavior is also outside this track. Flock cohorts,
affinity, goals, wander fields, attractors, and semantic behavior evidence are
simulation or product behavior-authoring concerns, not renderer source truth.

## Scope

This track covers:

- indirect draw contract hardening for direct, indexed direct, indirect, and
  indexed indirect draw sources;
- validation of indirect argument buffer type, element count, byte size, byte
  offset alignment, byte offset bounds, and indexed versus non-indexed
  compatibility;
- removal or redesign of indirect APIs whose CPU-side vertex or instance
  offsets cannot affect WGPU indirect submission;
- renderer-owned GPU primitive kernels for counter reset, u32 prefix scan,
  scatter/compaction, and indirect draw argument generation;
- primitive plan lowering into normal render-flow compute passes with stable
  labels, typed diagnostics, and no hidden CPU fallback;
- graph-level fixed-step scheduling for bounded repeated pass execution that
  uses the existing runtime `FixedTimeConfig`, `FixedTimeState`, and
  `CatchupBudget` resources as timing source truth;
- iteration-scoped render-flow uniform projection for passes inside repeated
  regions while existing frame-scoped `uniform_from_state` and
  `uniform_from_state_with_surface` stay source-compatible;
- submitted substep, max substep, accumulated-time, and dropped/deferred-time
  diagnostics proving mouse motion, redraw bursts, cursor movement, and resize
  events do not increase simulation steps per real second;
- procedural 2D camera projection contracts for aspect-correct fill-viewport
  rendering without moving camera truth into prepared views;
- benchmarks, public renderer docs, and closeout evidence sufficient for
  `runtime_proven`.

## Non-Scope

This track does not implement:

- spatial hash grids;
- chunked unbounded population residency;
- world-coordinate streaming windows;
- hash collision policy;
- renderer-owned gameplay or product truth;
- semantic flock identity, affinity groups, goals, or richer boids behavior
  authoring;
- camera source-truth ownership inside `PreparedViewFrame`;
- final no-gap renderer verification.

Those remain separate design or audit work. Current final no-gap renderer
verification belongs to the accepted RunenRender R8 -> RX sequence.

## Ownership

The bounded context owner is `engine/src/plugins/render`.

Renderer owns:

- render-flow draw-source semantics, validation, execution-plan compilation,
  and WGPU submission selection;
- GPU primitive kernels, primitive dispatch lowering, primitive diagnostics,
  and primitive benchmark evidence;
- graph scheduling contracts for repeated pass execution and renderer-visible
  fixed-step evidence derived from runtime fixed-time resources;
- procedural camera descriptors, projection math, projection uniforms, sprite
  sizing contracts, and evidence that renderer projection does not distort world
  scale;
- public renderer usage docs and API reference updates.

Renderer does not own:

- gameplay or world source truth;
- semantic population identity;
- product selection;
- authored simulation meaning;
- camera intent or gameplay camera state supplied by producers;
- streaming authority;
- fallback legality outside renderer capability diagnostics.

`PreparedViewFrame` carries prepared target size, view identity, and history
selection. It must not become the owner of camera intent in this track. Producer
or example state supplies camera intent; renderer code derives projection
uniforms from that intent and surface dimensions.

## Current Gaps

The completed procedural population track left these explicit gaps:

- `engine/src/plugins/render/graph/pass_graph.rs::RenderDrawSource` represents
  indirect drawing, but generic `IndirectDrawArgsBuffer` typing is erased before
  validation and execution, so indexed and non-indexed argument compatibility is
  not yet a fully enforced runtime contract.
- `engine/src/plugins/render/graph/validation.rs::validate_graphics_draw_source`
  checks declaration and 4-byte byte-offset alignment, but does not prove byte
  offset bounds against the typed indirect argument element size.
- `engine/src/plugins/render/api/passes.rs::GraphicsPassBuilder::draw_indirect_with_offsets`
  exposes CPU-side offsets that WGPU indirect submission cannot consume; that
  API shape can mislead users into believing offsets are applied outside the
  indirect argument buffer.
- `engine/src/plugins/render/gpu_primitives` provides reusable descriptors,
  validation, and explicit primitive execution plans, but not renderer-owned
  shader dispatch kernels.
- `engine/examples/boids_render_flow/rendering/state.rs::BoidsRenderState`
  exposes fixed-step evidence for one submitted simulation step, but multi-step
  catch-up is not graph scheduling and must not be added locally to boids.
- The boids example can still tie simulation advancement to render/update
  cadence if it bypasses runtime fixed-time source truth; cursor movement,
  mouse motion, redraw bursts, or resize events must not increase simulation
  steps per real second.
- Boids resize handling needs a reusable renderer procedural camera/projection
  contract. A boids-only draw-parameter patch would leave aspect correctness
  unowned and would not prove equal world x/y scale after projection.

## Architecture Rules

- Direct draw remains the simple public path through `.draw(...)` and
  `.draw_with_offsets(...)`.
- Indirect draw authoring must use explicit indirect methods with typed
  argument-buffer expectations.
- Indexed indirect and non-indexed indirect sources must be distinct validation
  and execution paths.
- Indirect offsets must be expressed by the indirect argument buffer and the
  indirect byte offset. CPU-side vertex, base-vertex, or instance offsets must
  not be accepted on indirect APIs unless a backend-supported contract consumes
  them.
- Primitive dispatch must use renderer-owned shaders and render-flow compute
  passes, not descriptor-only metadata or boids-local shader patches.
- Prefix scan must support arbitrary total counts through a hierarchical plan:
  block scan, block-sum scan, and block-offset propagation.
- Unsupported backend capability for required runtime proof must produce a
  typed diagnostic and fail closed. A CPU fallback is allowed only if a future
  design explicitly names it as an offline/debug path, not as runtime proof.
- Fixed-step catch-up must be graph-level bounded repeated pass execution that
  preserves resource sequencing across substeps.
- Fixed-step source truth comes from existing runtime resources:
  `FixedTimeConfig`, `FixedTimeState`, and `CatchupBudget`. Renderer execution
  consumes the derived submitted substep count and diagnostics; it does not
  become gameplay simulation truth.
- Passes inside repeated regions require iteration-scoped uniform projection.
  Existing `uniform_from_state` and `uniform_from_state_with_surface` remain
  frame-scoped and source-compatible.
- Procedural camera support belongs under renderer procedural infrastructure as
  derived projection machinery. Camera intent remains producer/example state.
- Fill viewport means no letterbox and no non-uniform stretch. The canonical
  boids camera policy is `FillViewport { fixed_axis: Vertical }`: vertical
  world scale stays stable and horizontal visible world span follows viewport
  aspect.
- Boids simulation uses stable world bounds and must not use viewport dimensions
  as simulation truth.
- Richer flock split/merge behavior requires a separate behavior-authoring
  intake with ownership review before implementation.

## Production Slices

- `WR-089`: doctrine and track activation.
- `WR-090`: indirect draw contract hardening.
- `WR-091`: reusable GPU primitive shader dispatch.
- `WR-092`: fixed-step graph catch-up scheduling plus redraw/input-rate
  invariance.
- `WR-101`: procedural 2D camera and view projection contract.
- `WR-093`: evidence, benchmarks, docs, and runtime-proven closeout.

`WR-089` is the activation slice only. It must not absorb product code from
later slices.

## Evidence

Closeout evidence must prove:

- invalid indexed versus non-indexed indirect argument buffers fail graph
  validation before submit;
- indirect byte offsets are alignment-checked and bounds-checked against typed
  argument buffer element sizes;
- indirect APIs do not expose ignored CPU offsets;
- renderer-owned primitive shader dispatch is exercised outside the boids
  example;
- u32 prefix scan supports arbitrary counts through hierarchical dispatch;
- primitive execution has typed diagnostics and no hidden CPU fallback in
  runtime proof paths;
- graph fixed-step catch-up submits `0..N` bounded substeps deterministically
  from runtime fixed-time state;
- mouse motion, cursor movement, redraw bursts, and resize events do not
  increase simulation steps per real second;
- iteration-scoped uniform projection is visible and validated for repeated
  regions;
- ping-pong and other pass resource sequencing remain stable across substeps;
- procedural camera projection keeps equal world x/y scale after projection for
  landscape, portrait, square, and extreme aspect surfaces;
- boids fills the viewport without letterbox, without non-uniform stretch, and
  without viewport-dependent simulation truth;
- behavior-authoring follow-up exists as separate intake and is not folded into
  renderer hardening;
- docs and benchmarks match the public API and runtime behavior.
