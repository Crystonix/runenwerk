---
title: Renderer Procedural Population Platform
description: Implemented design for reusable GPU procedural population infrastructure, bounded grid acceleration, first-class draw sources, and production boids evidence.
status: implemented
owner: engine
layer: engine-runtime / renderer / procedural
canonical: true
last_reviewed: 2026-09-10
related_designs:
  - ../accepted/renderer-gpu-evidence-and-procedural-visuals-design.md
  - ../accepted/renderer-scale-residency-and-gpu-driven-visibility-design.md
  - ../accepted/render-product-graph-platform-design.md
  - ../accepted/render-production-readiness-and-inspection-design.md
  - ../accepted/runenrender-decomposition-design.md
---

# Renderer Procedural Population Platform

## Implementation status

This procedural population architecture is implemented and backed by the completed `WR-083` through `WR-088` runtime-proven track. It remains a durable renderer capability contract, subordinate to ADR 0021 and the accepted RunenRender architecture where broader renderer ownership or lifecycle terminology conflicts. Current final operational conformance and residual-audit authority belongs to the RunenRender R8 -> RX sequence, not the historical `PT-RENDER-PERFECTION` track.

## Decision

Large procedural populations are renderer-owned derived execution data, not
one-off example shader patches. The renderer provides reusable authoring,
graph, primitive, population, diagnostic, benchmark, and evidence contracts for
GPU-resident populations whose submitted work remains bounded and inspectable.

This design extends the completed GPU/procedural and scale tracks. It does not
reopen those tracks and it does not claim final operational conformance.
The production target was `runtime_proven`; current no-gap verification is owned
by the accepted RunenRender R8 -> RX sequence.

## Scope

This track covers:

- procedural-owned pass authoring that lowers to graphics internally;
- first-class direct versus indirect draw-source descriptors;
- reusable GPU u32 scan, counter reset, scatter/compaction, and indirect draw
  argument contracts;
- bounded uniform-grid population support for fixed-radius local neighbor
  queries;
- a production boids example with fixed-step evidence, stable visual heading,
  aspect-correct impostors, bounded work, and explicit unsupported diagnostics;
- public API, usage-guide, evidence, and benchmark updates.

It does not define unbounded world truth, streaming policy, authored simulation
semantics, product freshness, fallback legality, or final renderer audit
criteria.

## Ownership

The bounded context owner is `engine/src/plugins/render`.

Renderer owns:

- render-flow graph draw-source semantics and validation;
- procedural pass authoring and internal graphics lowering;
- derived GPU primitive buffers and primitive diagnostics;
- population grid build/simulation/draw scheduling contracts;
- example runtime evidence, benchmark runners, public renderer docs, and
  inspection data.

Domain, product, gameplay, world, asset, editor, and streaming owners retain
source truth, semantic population identity, product selection, authority,
freshness, fallback legality, and residency intent.

## Invariants

- Procedural APIs must not expose `GraphicsPassBuilder` as their extension
  surface.
- Existing `.draw(...)` remains the simple direct draw path.
- Indirect drawing is expressed as a typed draw source, not only as a buffer
  sidecar.
- Unsupported indirect/readback/storage/timing capability states produce typed
  diagnostics rather than success-shaped no-ops.
- Canonical population paths size buffers by total boid or cell count. Fixed
  bucket overflow without explicit diagnostics is not acceptable.
- The first boids production path uses one fixed simulation step per submitted
  frame and reports that evidence. Multi-step catch-up requires a later graph
  scheduling feature.
- The canonical boids shader must not keep an O(n^2) production neighbor loop
  or a render-stage storage loop over all boids.
- Resize must preserve impostor aspect by deriving clip offsets from surface
  dimensions.
- Spatial hash and chunked unbounded populations are later milestones after
  bounded-grid evidence passes.

## Production Slices

`WR-083` is doctrine and track activation only. Implementation is split into
bounded rows:

- `WR-084`: procedural builder plus first-class indirect draw contract.
- `WR-085`: reusable GPU prefix scan and compaction primitives.
- `WR-086`: bounded uniform-grid procedural population support.
- `WR-087`: boids production upgrade.
- `WR-088`: evidence, benchmarks, docs, and closeout.

## Evidence

Acceptance evidence must prove:

- no render-stage storage loop over all boids;
- no production O(n^2) neighbor loop;
- no aspect skew on resize;
- no silent grid overflow;
- explicit unsupported diagnostics;
- bounded submitted work;
- stable production evidence from the boids example.

Benchmark and artifact placement follows the repository benchmark conventions:
runner code stays in code locations such as `benches/` or `examples/`, raw
outputs stay in dedicated artifact folders, and prose reports stay in docs
benchmark or closeout folders.
