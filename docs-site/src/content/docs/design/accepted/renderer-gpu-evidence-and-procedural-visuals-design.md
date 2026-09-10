---
title: Renderer GPU Evidence And Procedural Visuals Platform
description: Active design for renderer GPU pass evidence, render-flow contract guards, hybrid procedural sprite APIs, and a canonical boids runtime proof.
status: accepted
owner: engine
layer: engine
canonical: true
last_reviewed: 2026-09-10
related_designs:
  - ./render-product-graph-platform-design.md
  - ./render-production-readiness-and-inspection-design.md
  - ./renderer-scale-residency-and-gpu-driven-visibility-design.md
  - ./sdf-world-rendering-and-raymarch-acceleration-design.md
  - ./renderer-mesh-material-lighting-and-asset-handoff-design.md
  - ./renderer-temporal-reconstruction-and-dynamic-resolution-design.md
  - ./renderer-hardware-ray-query-and-hybrid-tracing-design.md
  - ./renderer-product-visual-producers-platform-design.md
  - ./runenrender-decomposition-design.md
  - ../active/runenrender-internal-decomposition-execution-plan.md
---

# Renderer GPU Evidence And Procedural Visuals Platform

This design extends the completed Render Product Graph Platform without reopening it. The completed product-graph work made render requests, dynamic targets, fragments, timing, and inspection contract-driven. This track adds the next missing production layer: GPU pass-cost evidence, stricter render-flow shape guards, discoverable procedural visual APIs, and a corrected boids example that proves the renderer can execute many procedural instances without multiplying fullscreen work.

The renderer still does not own product truth. Product selection, freshness, authority, fallback legality, rebuild policy, residency policy, gameplay particle semantics, field/VFX emission truth, and authored model meaning stay in their owning domains and producers. The renderer owns execution contracts, validation diagnostics, derived GPU resources, command encoding, pass timing, examples, and inspection DTOs.

## Problem

The boids render-flow example exposed a contract gap. The upgraded renderer correctly honors graphics pass instance counts, but the example used a fullscreen triangle shader that loops over every boid in the fragment stage and then drew that fullscreen pass once per boid. The result is multiplied fullscreen work, not a real per-boid sprite rendering model.

The deeper issue is not boids alone. The renderer lacks durable runtime evidence and guardrails for these cases:

- CPU encode/submit timing is visible, but GPU pass execution cost is not first-class.
- Render-flow validation does not make dangerous pass-shape combinations obvious enough, such as fullscreen-style rendering multiplied by large instance counts.
- Procedural instance rendering has low discoverability; users can bind raw buffers and issue draws, but there is no canonical public path for mesh sprites, local SDF impostors, instance buffers, and explicit render state policy.
- The canonical boids example does not yet demonstrate the intended long-term model: compute simulation in storage buffers, rendering as bounded per-instance local geometry or SDF impostors, and runtime GPU evidence for smoothness and pass cost.

## Goals

1. Add renderer-visible GPU timing DTOs that separate CPU encode/submit timing from GPU pass execution timing.
2. Add capability-gated timestamp query support with explicit unsupported diagnostics when a backend cannot provide GPU timestamps.
3. Add render-flow validation and preflight diagnostics for dangerous pass shapes, especially fullscreen-style graphics multiplied by instance count.
4. Add discoverable procedural instance APIs for mesh/quad sprites, local 2D SDF impostors, shared storage-backed instance buffers, and explicit blend/depth/cull/primitive policy.
5. Rewrite boids as the canonical hybrid procedural visual example: storage-backed compute simulation plus per-boid local mesh/SDF sprite rendering.
6. Produce runtime evidence, docs, benchmarks, and examples that support a `runtime_proven` production quality target.

## Focused Track Stack

This track is the GPU evidence and procedural-visual foundation. It does not
try to cover every renderer feature in one production track. The complete
renderer program is split into focused tracks:

- `PT-RENDER-GPU`: GPU timings, pass-shape guards, procedural mesh/SDF sprite
  APIs, and the corrected boids proof.
- `PT-RENDER-SCALE`: finite working sets, residency, GPU-driven visibility,
  LOD, culling, indirect drawing, and millions-scale evidence.
- `PT-RENDER-SDF`: sparse SDF bricks, page tables, clipmaps, distance mips,
  candidate lists, and conservative raymarch acceleration.
- `PT-RENDER-MESH-MATERIAL`: mesh, material, lighting, shader specialization,
  pipeline cache policy, last-good shader fallback, and asset handoff.
- `PT-RENDER-TEMPORAL`: TAA, TAAU, dynamic resolution, history validity, and
  FSR-style optional adapters.
- `PT-RENDER-RT`: optional hardware ray-query and hybrid tracing paths with
  mandatory fallback.
- `PT-RENDER-PRODUCT-VISUALS`: product-owned particles, VFX, vegetation, water,
  atmosphere, weather, trails, decals, and animation render producers.
- RunenRender `R8 -> RX`: current cross-track operational conformance,
  residual-gap audit, and extraction-readiness authority under ADR 0021.

Completed `PT-RENDER-PG` remains the product-graph foundation. These tracks
extend it without reopening its completed milestones.

## Non-Goals

- Do not move product truth, selection, freshness, authority, fallback legality, rebuild policy, or residency policy into the renderer.
- Do not make the renderer own gameplay particles, field/VFX emitters, simulation truth, authored model semantics, or material truth.
- Do not hide expensive work by lowering boid count, disabling validation, disabling diagnostics, or weakening prepared-frame preflight.
- Do not add a renderer-owned gameplay/VFX product system in this track. Product-domain emitters may link to this renderer platform later.
- Do not reopen completed `PT-RENDER-PG` milestones; this track builds on their accepted contracts.
- Do not use this track to implement SDF brick/page-table residency, clipmaps,
  raymarch acceleration, temporal reconstruction, hardware ray tracing,
  material truth/lowering, product visual emitters, or the current RunenRender
  R8 -> RX conformance/audit sequence. Those are governed by the focused tracks
  and accepted RunenRender authority above.

## Ownership

Renderer-owned:

- GPU timing capability detection and unsupported diagnostics.
- Timestamp query allocation, resolve, readback staging, and DTO projection.
- Render-flow pass-shape validation and submit/preflight diagnostics.
- Procedural rendering APIs and derived GPU resources.
- Example render flows, renderer docs, benchmarks, and runtime evidence.

Producer/domain-owned:

- Product truth and product selection.
- Gameplay, field, VFX, particle, material, model, and editor semantics.
- Freshness, authority, fallback legality, rebuild policy, and residency intent.
- Any user-facing policy that decides whether a visual product should exist.

## Production Sequence

### PM-RENDER-GPU-001: Doctrine And Accepted Design

Lock the doctrine and acceptance gates for GPU evidence plus procedural visuals. This milestone is design-only. It accepts the doctrine, links the production track, and records the WR intake proposals. Implementation remains blocked until the linked implementation rows are promoted through their own gates.

### PM-RENDER-GPU-002: GPU Pass Timing Foundation

Add capability-gated GPU pass timing. Timestamp queries are required when supported. Unsupported backends must expose explicit diagnostics rather than silent absence. Public DTOs must distinguish CPU encode/submit timing from GPU execution timing and keep backend handles private.

### PM-RENDER-GPU-003: Render-Flow Pass-Shape And Instance Guards

Validate render-flow shapes that can create accidental multiplied work. Fullscreen-style rendering combined with large instance counts must produce typed diagnostics or require explicit opt-in evidence. The guard belongs in render validation/preflight, not in the boids example.

### PM-RENDER-GPU-004: Hybrid Procedural Instance Rendering API

Add a public procedural instance path for mesh/quad sprites and local 2D SDF impostors. The API must support shared storage-backed instance buffers, explicit blend/depth/cull/primitive policy, and validation diagnostics. It must not infer product semantics.

### PM-RENDER-GPU-005: Canonical Boids Rewrite

Rewrite boids to use the hybrid procedural instance path. Compute simulation remains storage-backed. Rendering becomes bounded local sprite/impostor work per boid. The example must not issue fullscreen-per-boid work and must not keep history copies unless a real trail/history effect consumes them.

### PM-RENDER-GPU-006: Production Evidence

Harden docs, examples, benchmarks, runtime inspection, and closeout evidence. Completion targets `runtime_proven`; final operational conformance remains unavailable until the RunenRender R8 -> RX sequence proves the required no-gap evidence.

## Contract Targets

### GPU Timing DTOs

The public timing model should expose:

- CPU preflight, flow encode, command submit, diagnostics/report, shader poll, and pacing timing.
- GPU pass timing by surface, frame, flow, pass, and pass kind.
- Timing source and capability state: supported, unsupported, unavailable this frame, or readback pending.
- Unsupported diagnostics that explain which capability is missing.

GPU timing DTOs must not expose mutable backend handles.

### Pass-Shape Guards

Render-flow validation should diagnose dangerous combinations before runtime stutter:

- Fullscreen-style vertex generation multiplied by high instance counts.
- Fragment work that iterates over storage-backed instance collections while also instancing the fullscreen draw.
- Missing explicit procedural primitive policy when a pass appears to encode procedural sprite semantics.
- Compute/graphics shape mismatches already covered by fragment validation.

The guard should support explicit advanced opt-in only when the author records intent and validation evidence. Silent fallback to primary, implicit product policy, or boids-specific special cases are not acceptable.

### Procedural Instance API

The API should make normal usage obvious:

- Mesh/quad sprite descriptors.
- Local 2D SDF impostor descriptors.
- Instance buffer declarations for storage-backed and vertex-backed paths.
- Explicit blend, depth, cull, primitive topology, and target policy.
- Validation errors for missing buffers, incompatible layouts, unsupported backend capabilities, and ambiguous pass shapes.

The API can derive GPU resources from declared inputs, but it cannot decide which product is authoritative or fresh.

### Boids Runtime Proof

The boids example should prove:

- Compute simulation writes storage-backed boid state.
- Render pass draws local per-boid geometry or SDF impostors.
- No fullscreen pass is multiplied by boid count.
- GPU pass timing identifies compute, render, and present costs.
- Runtime inspection can show timing, pass shape, instance count, and unsupported timing diagnostics without backend handles.

## Validation Requirements

Future implementation rows must add focused tests and evidence for:

- GPU timing capability detection and unsupported diagnostics.
- Timestamp query readback and timing DTO projection when supported.
- Pass-shape guard diagnostics for fullscreen-plus-instance hazards.
- Procedural sprite/SDF API validation and inspection.
- Boids render-flow shape no longer issuing multiplied fullscreen work.
- Runtime GPU evidence for boids smoothness and pass cost.

## Acceptance Decisions

The doctrine can move toward acceptance with these decisions:

1. Fullscreen-style graphics combined with large instance counts is rejected by
   default. Any advanced opt-in must be explicit render-flow policy with typed
   author intent, pass identity, bounded instance evidence, and inspection
   output. The opt-in cannot be inferred from boids, shader names, topology
   alone, or product fallback policy.
2. Backends without timestamp-query support can still participate in
   `runtime_proven` closeout only when they expose typed unsupported timing
   diagnostics, CPU encode/submit timing, pass-shape evidence, and runtime
   inspection proving that missing GPU timing is capability-driven rather than
   silently absent. Hardware with timestamp support must provide GPU pass timing
   evidence before the timing row can close.
3. The first public SDF impostor API is local 2D sprite/impostor oriented. 3D
   SDF impostor hooks stay out of scope until the sparse SDF and raymarch tracks
   define accepted brick, page-table, residency, and acceleration contracts.
4. Boids is the mandatory canonical runtime proof for this track. Additional
   examples may support documentation and coverage, but they are not mandatory
   before doctrine acceptance.
5. Initial boids production evidence measures stable bounded work instead of
   claiming universal frame-rate thresholds. The production evidence row must
   record scene size, backend capability, CPU timing, GPU timing or unsupported
   diagnostics, pass shape, instance count, and benchmark command.

## Acceptance Gate Notes

`PM-RENDER-GPU-001` remains design-only. The design-first implementation
contract for `WR-082` lives at
`docs-site/src/content/docs/reports/implementation-plans/wr-082-renderer-gpu-evidence-and-procedural-visuals-doctrine-acceptance/plan.md`.

No ADR is required while this doctrine preserves the accepted renderer/product
boundary: the renderer owns execution contracts, derived GPU resources,
backend-neutral timing evidence, validation diagnostics, and inspection DTOs.
Product producers retain product truth, selection, freshness, authority,
fallback legality, rebuild policy, residency intent, gameplay/VFX semantics,
material truth, and model truth.

Create an ADR or stop before implementation if later work changes dependency
direction, makes renderer state authoritative for product policy, introduces a
persisted cross-domain ABI, or makes GPU timing/procedural visual APIs mandatory
outside the renderer-owned boundary.
