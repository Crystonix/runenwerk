---
title: Sparse SDF Terrain Runtime Integration
description: Deferred design for a shader-bound sparse SDF terrain runtime that consumes renderer-owned SDF GPU resources without making renderer code own SDF product truth.
status: deferred
owner: engine
layer: engine-runtime / renderer / sdf-products
canonical: true
last_reviewed: 2026-09-10
related_designs:
  - ../accepted/sdf-world-rendering-and-raymarch-acceleration-design.md
  - ../accepted/sdf-product-renderer-and-gpu-residency-design.md
  - ../accepted/renderer-scale-residency-and-gpu-driven-visibility-design.md
  - ../accepted/render-production-readiness-and-inspection-design.md
  - ../accepted/renderer-gpu-evidence-and-procedural-visuals-design.md
  - ../accepted/runenrender-decomposition-design.md
related_roadmaps:
  - ../../engine/roadmaps/fully-featured-renderer-roadmap.md
---

# Sparse SDF Terrain Runtime Integration

## Deferred status

This shader-bound sparse-SDF terrain runtime remains a valid follow-on target but is not active work. Reactivate only through a dedicated accepted RunenRender/SDF integration issue after the current RunenRender sequence establishes the relevant rendering boundary and a fresh source/product census proves the sparse residency, ABI, product-ownership, and runtime-proof assumptions still hold. The existing analytic terrain example does not activate this design.

## Decision

This design records a renderer follow-up target for shader-bound sparse SDF
terrain runtime integration. The completed historical `PT-RENDER-SDF` work
remains valid and closed: it proved derived residency, raymarch acceleration
inspection, runtime evidence aggregation, and diagnostics. The remaining gap is
that there is not yet a production-oriented terrain render path where WGSL
traversal consumes the sparse SDF page table, brick atlas, distance mip, and
candidate-list resources produced by renderer residency.

The source truth stays in `domain/world_sdf::SdfChunkPayload`, product
publication, and source generations. The renderer owns only derived GPU
resources, bind plans, pass execution, timing, and diagnostics.

`PT-RENDER-SDF`, `PM-RENDER-SDF-RUNTIME-*`, and `WR-103` are retained as
historical decomposition/provenance vocabulary. They are not live work
authority. Any implementation requires an owning GitHub issue, sequencing
through the canonical renderer/workspace roadmaps when relevant, and a reviewed
pull request with exact-head validation evidence.

## Current State

`engine/examples/procedural_sky_sdf_terrain` is a useful visual demo. It renders
terrain with analytic shader functions and fullscreen raymarching, but its WGSL
does not traverse renderer-owned sparse SDF residency resources. It remains an
analytic visual example and must not be used to satisfy the sparse runtime
proof.

The existing renderer SDF code exposes deterministic inspection evidence:

- `RenderSdfResidencyResource::derive_from_sources` derives residency DTOs
  from source payloads.
- `inspect_sdf_raymarch_acceleration` derives distance-mip, safe-step,
  candidate-list, and diagnostic evidence.
- `inspect_render_sdf_production_evidence` aggregates residency, raymarch,
  timing, benchmark, and visual evidence for closeout.

Those surfaces are necessary but not sufficient for the runtime gap. The
missing contract is the shader-bound data flow from source payloads into GPU
bindings consumed by a terrain raymarch pass.

## Runtime Contract

The intended data flow is:

```text
SdfChunkPayload
  -> RenderSdfResidencyResource::derive_from_sources
  -> inspect_sdf_raymarch_acceleration / acceleration resource
  -> RenderSdfTerrainRuntimeBindPlan
  -> shader-bound fullscreen terrain raymarch pass
  -> RenderSdfTerrainRuntimeFrameInspection
```

Follow-on implementation should introduce renderer-facing types with these
responsibilities:

- `RenderSdfTerrainRuntimeConfig`: per-runtime limits, budgets, scale bands,
  camera-relative framing policy, and diagnostic toggles.
- `RenderSdfTerrainRuntimeBindPlan`: renderer-owned binding plan for page
  table, brick atlas, distance mip, candidate lists, generations, limits, and
  camera-relative origin data.
- `RenderSdfTerrainRuntimeFrameInspection`: per-frame evidence that the shader
  path used sparse resources, not analytic-only terrain math.
- `RenderSdfTerrainRuntimeDiagnostic`: fail-closed diagnostics for missing
  residency, stale generations, unsupported limits, candidate explosion,
  unsafe overstep, over-budget residency, and empty candidate coverage.

The first proof should use deterministic synthetic `SdfChunkPayload` terrain
products. That keeps the renderer integration unblocked while the final
open-world terrain product pipeline remains separate product-family scope.

## Rendering Rules

- Use one fullscreen terrain raymarch pass per prepared view.
- Never multiply terrain raymarch work by chunk, entity, source, or payload
  count.
- WGSL must consume sparse SDF resource bindings for traversal. Analytic-only
  terrain functions, hardcoded heightfields, or CPU-only inspection DTOs cannot
  satisfy the runtime proof.
- Page table, brick atlas, distance mip, and candidate-list bindings must be
  part of the shader-visible contract.
- Camera-relative world framing is required so endless-world coordinates do
  not depend on fragile absolute `f32` positions.

## Proposed Decomposition

The historical milestone split remains useful architecture decomposition:

- `PM-RENDER-SDF-RUNTIME-001`: governance and implementation-contract boundary.
- `PM-RENDER-SDF-RUNTIME-002`: GPU ABI and runtime bind-plan implementation,
  keeping backend handles inside renderer boundaries and product truth outside.
- `PM-RENDER-SDF-RUNTIME-003`: dedicated sparse terrain runtime example and WGSL
  traversal proof.
- `PM-RENDER-SDF-RUNTIME-004`: evidence commands, focused tests, benchmarks,
  docs, and runtime-proven closeout.

An owning GitHub issue may adopt, refine, split, or reject this decomposition
when work is activated. Naming these phases does not create active milestones.

## Non-Goals

- Do not reopen the completed historical sparse-SDF renderer work or rewrite its
  closeout claims.
- Do not make `procedural_sky_sdf_terrain` the production sparse runtime proof.
- Do not treat historical `WR-103` as implementation authority.
- Do not make renderer code authoritative for SDF payload truth, collision
  truth, query policy, product fallback legality, or open-world generation.
- Do not claim final no-gap conformance outside the accepted RunenRender R8 -> RX authority.

## Evidence Expectations

Follow-on implementation evidence must prove:

- deterministic GPU-layout derivation from source generations;
- fail-closed missing and stale residency diagnostics;
- bounded candidate-list behavior;
- conservative distance-mip stepping with unsafe-overstep diagnostics;
- camera-relative far-origin behavior;
- shader-bound sparse resource consumption rather than analytic-only terrain;
- runtime budget and residency pressure reporting;
- docs that distinguish renderer runtime proof from real open-world terrain
  product integration.

Broad acceptance validation at one unchanged reviewed head includes focused
crate tests plus `cargo validate`, `git diff --check`, and the documentation
build. Repository-owned exact-head CI and Documentation Build are the delivery
acceptance evidence.
