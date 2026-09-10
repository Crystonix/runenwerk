---
title: Editor Rendered World And Multi Entity Viewport Design
description: Implemented design for the SDF-first editor rendered world, deterministic multi-entity viewport scene packets, and picking/render alignment.
status: implemented
owner: apps/runenwerk_editor
layer: app-runtime / engine-render
canonical: true
last_reviewed: 2026-09-10
related_designs:
  - ./render-product-surface-foundation-bundle-design.md
  - ../active/workspace-viewport-expression-upgrade-design.md
  - ../accepted/sdf-first-production-capability-map.md
related_roadmaps:
  - ../../engine/roadmaps/fully-featured-renderer-roadmap.md
  - ../../apps/runenwerk-editor/roadmap.md
  - ../../apps/runenwerk-editor/viewport-expression-implementation-roadmap.md
  - ../../engine/plugins/render/docs/roadmap.md
related_reports:
  - ../../reports/closeouts/wr-018-rendered-world-v1/closeout.md
---

# Editor Rendered World And Multi Entity Viewport Design

## Status

Implemented. WR-018 closed the bounded rendered-world V1 architecture: authored editor entities with `scene::LocalTransform` plus `runenwerk_editor::editor_runtime::EditorPrimitive` are extracted by the editor app into one deterministic viewport scene packet, then consumed by scene-color and picking products. The renderer does not own editor world truth.

The implementation has evolved since the original V1 closeout. Production extraction now routes through `extract_viewport_scene_render_packet_with_material_slots`; the original `extract_viewport_scene_render_packet` name remains as a test helper. Later material work added packet-local material-slot data without changing the ownership rule or turning the renderer into scene authority.

This document records the durable implemented boundary. Historical WR-018 phase state and exact delivery evidence remain in the closeout and Git history rather than as active planning text here.

## Implemented Contract

```text
RunenwerkEditorRuntime scene reality
  -> app-owned deterministic extraction
  -> EditorViewportSceneRenderPacket
  -> target-local prepared render inputs
  -> scene color / picking products
```

The packet carries the bounded editor primitive representation used by the viewport, including stable editor entity identity, transforms and primitive parameters, selected/hovered state, packet-local pick slots, explicit slot-cap overflow evidence, and the later-added material-slot selection used by current rendering.

The default MVP ground plane is authored scene data through `EditorPrimitiveKind::Plane`. Viewport grid rendering is a non-persistent overlay aid. Neither is shader-owned hidden world truth.

## Ownership And Invariants

- `apps/runenwerk_editor` owns extraction from editor scene reality and target-local viewport state.
- `EditorViewportSceneRenderPacket` is the CPU-side scene packet contract used by rendering and picking.
- Scene packet ordering is deterministic by stable editor entity identity.
- Scene-color and GPU-picking products consume the same packet-backed primitive representation.
- CPU entity picking consumes viewport render-state packet semantics and fails closed when that packet is unavailable instead of independently scanning runtime scene state.
- Packet-local picking ids are mapped back to full editor entity identities on the CPU side.
- Scene and overlay product passes clear their targets; transparent misses must not preserve prior-frame contents.
- Viewport render jobs remain per viewport and target-local.
- The renderer consumes prepared products. It does not become authoritative for editor entities, transforms, selection, materials, or world state.

## Current Code Anchors

Current implementation evidence includes:

- `apps/runenwerk_editor/src/runtime/systems/frame_submit.rs::populate_viewport_render_state`;
- `apps/runenwerk_editor/src/runtime/systems/frame_submit.rs::extract_viewport_scene_render_packet_with_material_slots`;
- `apps/runenwerk_editor/src/runtime/resources.rs::EditorViewportSceneRenderPacket`;
- `apps/runenwerk_editor/src/runtime/resources.rs::EditorViewportRenderState::compose_scene_product_uniform`;
- `apps/runenwerk_editor/src/runtime/systems/picking.rs::pick_entity_hit` and viewport picking-context formation;
- `apps/runenwerk_editor/src/runtime/viewport/render_jobs.rs::build_viewport_render_job`;
- `assets/shaders/editor_viewport_scene_product.wgsl`;
- `assets/shaders/editor_viewport_picking_product.wgsl`;
- `assets/shaders/editor_viewport_overlay_product.wgsl`.

The test-only `extract_viewport_scene_render_packet` wrapper remains useful for focused packet tests, but it is not the production extraction entry point anymore.

## V1 Boundary And Later Extensions

WR-018 V1 originally excluded material semantics, terrain, prefab instancing, general mesh scene extraction, renderer-owned ECS extraction, world streaming, and per-invocation storage-buffer scene packets. Later material work extended the existing packet with material-slot selection. That extension is additive implementation evolution, not evidence that WR-018 originally owned material architecture.

Future terrain, field, prefab, mesh, storage-buffer, or world-scale rendering work must preserve the same authority direction: owning domains/apps form semantic products or contributions; RunenRender/render execution consumes them. Such work is governed by its current accepted design and roadmap owner rather than by reopening WR-018.

## Completion Evidence

The canonical delivery evidence is [WR-018 Rendered World V1 Closeout](../../reports/closeouts/wr-018-rendered-world-v1/closeout.md). Code and tests own current behavior; this implemented design owns the durable bounded architecture described above.
