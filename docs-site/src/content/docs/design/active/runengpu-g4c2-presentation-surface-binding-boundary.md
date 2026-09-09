---
title: RunenGPU G4C2 Presentation-Surface Binding Boundary
description: Narrow G4C2 correction keeping acquired presentation surfaces out of pre-G7 shader-resource realization.
status: active
owner: gpu
layer: framework/gpu
canonical: true
last_reviewed: 2026-08-11
related_docs:
  - ./runengpu-g4b-contracts-g4c-delivery-design.md
  - ./runengpu-g4-context-program-realization-design.md
  - ../accepted/runenrender-decomposition-design.md
  - ../../adr/accepted/0015-separate-gpu-execution-from-rendering.md
  - ../../workspace/specs/pt-runengpu-g4c-wgpu-realization-cutover.ron
  - ../../workspace/specs/pt-runengpu-g4c1-resource-realization.ron
  - ../../workspace/specs/pt-runengpu-g4c2-program-binding-realization.ron
---

# RunenGPU G4C2 Presentation-Surface Binding Boundary

## Status and scope

Issue `#230` discovered this boundary during the mandatory pre-G4C2 exact-current-main
readiness review after accepted G4C1. This document narrows G4C2 only where current
RenderFlow presentation-surface handling conflicts with the already accepted G4C
resource and surface ownership model.

It does not implement G4C2, G7, a new surface abstraction, or a new bridge. The canonical
G4C2 phase specification remains authoritative for program, layout, and typed bind-group
realization. This correction makes one previously implicit consequence explicit:
**an acquired presentation surface is not an accepted G4C1 shader resource.**

## Exact current-source conflict

Accepted current `main` at discovery is:

```text
3bcf32689450435b2ff4b3a0728b81fd529a8ad0
```

The relevant source facts are simultaneously true:

1. `engine/src/plugins/render/backend/surface.rs::build_surface_config` configures
   acquired presentation textures for `RENDER_ATTACHMENT | COPY_SRC | COPY_DST` and
   does not request `TEXTURE_BINDING` or `STORAGE_BINDING`.
2. `engine/src/plugins/render/graph/prepared_validation.rs` currently admits
   `PreparedTargetBinding::SurfaceColor` for a sampled-texture alias use.
3. `engine/src/plugins/render/renderer/render_flow/bindings.rs` has a separate raw
   `SurfaceTextureView(TextureView)` bind-group path beside accepted
   `GpuRealizedTextureView` resources.
4. current resource lowering exposes `surface.color` as imported texture-shaped
   RenderFlow state and can resolve it to the acquired frame texture.

The path is therefore not merely a dormant type variant. Preparation may admit a
sampled use that the current presentation texture configuration does not authorize, and
G4C2 cutover would need a raw acquired-surface exception to preserve it.

That exception is rejected.

## Ownership decision

Before accepted G7:

```text
presentation SurfaceColor
    may remain a presentation/render attachment
    existing copy behavior remains outside this correction
    is not a G4C1 logical resource
    is not a G4C2 sampled bind-group resource
    is not a G4C2 storage bind-group resource

G4C2 typed bind groups
    consume accepted G4C1 resource handles
    never consume a raw acquired-surface TextureView

G7
    owns any future explicit surface-resource capability
    owns surface affinity/generation and usage negotiation
    may later decide whether sampled presentation surfaces are supported
```

This does not decide that surface sampling is undesirable forever. It decides only that
G4C2 cannot acquire G7 ownership or manufacture an exception around the accepted G4C1
resource contract.

## WGPU capability consequence

WGPU texture usage is part of correctness, not a performance hint. A texture bound as a
sampled texture requires texture-binding usage; a storage texture requires
storage-binding usage. Surface texture usages are constrained by the capabilities of the
surface/adapter pair rather than being universally available.

Therefore a future G7 decision to support sampled presentation surfaces must:

1. inspect the applicable surface capability facts;
2. require the intended texture usage to be supported;
3. configure the surface with that usage before acquisition;
4. bind the resulting surface lifetime, affinity, and generation explicitly;
5. define failure/fallback policy without pretending the surface is an ordinary G4C1
   owned or imported resource.

The current fixed surface configuration's broader copy-usage portability is also a G7
surface-capability concern. This correction does not change those usages because doing so
would widen #230 beyond the G4C2 blocker.

## Required G4C2 source transition

The first accepted G4C2 implementation must make current source agree with this boundary.
At minimum:

- a sampled or storage shader binding that resolves to `SurfaceColor` rejects
  structurally before G4C2 bind-group realization;
- the raw `SurfaceTextureView(TextureView)` path is not retained as a G4C2 bind-group
  resource exception;
- no G4C1 surface logical handle, realization registry entry, or imported-resource
  surrogate is introduced;
- G4C2 bind groups use accepted typed G4C1 resources directly;
- tests that merely preserve the currently invalid sampled-surface admission are updated
  to prove structural rejection instead;
- attachment, present, and separately owned copy behavior remain outside this migration
  unless another owning issue explicitly changes them.

If an exact-current-main product consumer proves that sampled presentation surfaces are
required for correctness before G7, G4C2 must stop rather than add a raw-surface terminal.
That evidence requires a separately reviewed surface-capability ownership decision.

## Bridge consequence

The accepted G4C bridge ladder does not change:

```text
G4C1  CurrentRenderResourceBridge
          ↓ deleted/superseded by G4C2
G4C2  CurrentRenderPipelineBridge
          ↓ deleted/superseded by G4C3
G4C3  CurrentRenderExecutionBridge
          ↓ deleted by G5
```

`CurrentRenderPipelineBridge` may carry only residual accepted G4C1 resource terminals
plus the G4C2 objects required by exact uncut G4C3/G5 consumers. It must not gain a raw
presentation-surface terminal for shader binding. The separately bounded
`CurrentRenderDeviceQueue` operation loan likewise cannot become a surface escape hatch.

## Non-goals

This correction does not authorize:

- adding `TEXTURE_BINDING` or `STORAGE_BINDING` to surface configuration;
- implementing G7 surface capability negotiation;
- moving presentation surfaces into G4C1 ownership;
- a second object-reference bridge;
- public WGPU surface or texture-view access;
- changing RunenRender image-formation semantics;
- changing G4C3 or G5 ownership;
- broad surface, renderer, phase-spec, or documentation cleanup.

## Acceptance

Before G4C2 may activate:

- this boundary is merged and accepted-main verified;
- `#213` records the accepted correction and re-resolves its base from then-current
  `main`;
- the exact-current-main census still proves no conflicting source change;
- G4C2 implementation explicitly owns retirement of the sampled/storage `SurfaceColor`
  bind-group path;
- `#214` remains blocked.
