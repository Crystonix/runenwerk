---
title: Material Lab And Material Preview Design
description: Active design for roadmap-visible material graph authoring, ratification, preview products, diagnostics, and render handoff.
status: active
owner: domain/material_graph
layer: domain / app-runtime / engine-render
canonical: true
last_reviewed: 2026-09-10
related_designs:
  - ../implemented/editor-rendered-world-and-multi-entity-viewport-design.md
  - ../implemented/render-product-surface-foundation-bundle-design.md
related_roadmaps:
  - ../../apps/runenwerk-editor/roadmap.md
  - ../../workspace/roadmap-items.yaml
---

# Material Lab And Material Preview Design

## Status

Active design, roadmap-first. `WR-021` proved the runtime material-preview
product spine, KTX2 texture residency path, generated shader handoff, and
source-projected Material Lab contracts. `WR-028` owns the remaining
perfectionist work: rich visual graph editing, live texture visibility, and
source-backed SDF primitive material assignment consumed by scene rendering.
Model/mesh material binding is explicitly outside WR-028 and is owned by the
follow-up `WR-029` model/mesh material-binding track.

## Decision

Material Lab uses source-backed material graph documents as truth. Canvas state,
preview products, generated shaders, renderer prepared data, and viewport
targets are projections or derived products, not material source truth.

Scene material assignment is separate source truth owned by
`domain/editor/editor_scene`. Material Lab may project and edit scene material
slots through app workflow adapters, but it must not assign raw artifact ids or
make renderer state canonical.

Owning code paths:

- `domain/material_graph/src`
- `apps/runenwerk_editor/src/shell/providers/material_graph_canvas.rs`
- `apps/runenwerk_editor/src/shell/providers/material_inspector.rs`
- `apps/runenwerk_editor/src/shell/providers/material_preview.rs`
- `apps/runenwerk_editor/src/asset_pipeline`
- `apps/runenwerk_editor/src/runtime/viewport/product_registry.rs`
- `domain/editor/editor_scene/src/model/material.rs`
- `domain/ui/ui_graph_editor/src`

## V1 Contract

Runtime-proven V1 requires:

- `MaterialGraphDocument` source identity;
- ratification diagnostics before preview/render handoff;
- lowering into a renderer-consumable material product;
- material preview as a viewport/product target;
- failed preview preserving the prior valid artifact;
- source lineage in diagnostics and product descriptors.

## Post-Migration Polish

After the Tool Suite Registry / WorkbenchHost migration and the stable-key-native
Tool Suite Registry Inspector closeout, Material Lab is the first product-facing
suite used to stress the completed platform. `ML-A` is a presentation-only
polish slice: it exposes existing Material Lab diagnostics and preview state
through structured, app-neutral presentation DTOs and renders that status in the
Material Graph, Inspector, and Preview providers.

`ML-A` does not change Material Lab workflow behavior, preview publication
semantics, provider-owned graph routing, provider matching, V5 persistence,
renderer/product ownership, or stable-key surface identity. Product lineage,
deeper last-good preview semantics, graph interaction hardening, and richer
texture/material reference diagnostics remain follow-up polish slices.

`ML-B` hardens provider-owned graph interaction parity. Generic
`SurfaceInteraction::GraphCanvasAction` values remain editor-shell-neutral, and
`MaterialGraphCanvasProvider` owns the Material Graph action mapping into
Material Lab commands. The covered path is:
`GraphCanvasAction -> SurfaceInteraction::GraphCanvasAction ->
MaterialGraphCanvasProvider::map_interaction(...) -> Material Lab shell command`.
Committed graph edits and shortcuts are mapped by the provider, provisional
gesture lifecycle events are intentionally ignored as canvas-local state, and
stale projection epochs fail closed in shell command dispatch. ML-B does not
move Material Graph semantics back into `editor_shell`, change preview
publication behavior, change V5 persistence, change provider matching, or add
new tool-surface enum identity. ML-B closeout keeps ML-C preview product /
last-good polish and ML-D texture/material reference diagnostics pending.

`ML-C` starts preview product and last-good polish as current-state surfacing.
The Material Preview provider now exposes active product, material artifact,
shader artifact, scene-shader artifact, viewport product, last publication,
diagnostic count, and last-good preservation labels from existing app runtime
state. This is DTO/rendering polish only: it does not change preview rebuild
timing, publication semantics, prior-valid preservation, graph routing, provider
matching, V5 persistence, renderer ownership, or texture resource resolution.
ML-C closeout keeps preview behavior unchanged. Behavior-changing
preview/product work remains a separate follow-up only if focused tests prove
the existing semantics are insufficient, and ML-D texture/material reference
diagnostics remain pending.

`ML-D` adds texture/material reference diagnostics as a presentation-only polish
slice. Material Lab now surfaces structured resource-binding rows for missing,
ambiguous, incompatible, unsupported, unresolved, and generated texture states
that are already observable from app-owned material source and asset catalog
state. The texture resolver and app workflow remain in `apps/runenwerk_editor`,
`domain/texture` remains the texture contract/product owner, and
`domain/material_graph` remains material source truth. ML-D does not add viewer
handoff, texture graph tooling, texture-resolution behavior changes, preview
workflow changes, V5 persistence changes, or new tool-surface enum identity.
ML-D closeout keeps `resolve_material_resources` as the resolver path with
unchanged behavior, and ML-E workspace/persistence proof remains pending.

`ML-E` proves the polished Material Lab surfaces on the completed stable-key
Workbench platform. The graph canvas, inspector, and preview remain
stable-key-native suite surfaces; V5 persistence writes their stable keys as the
primary identity; the stable-key-only V5 write path carries no Material Lab
legacy enum metadata; and loaded Material Lab workspace requests still resolve
through the material provider family with stable-key-first provider matching.
Existing V5 load compatibility may recover legacy metadata for historical
stable keys, but that bridge is compatibility metadata rather than surface
authority. ML-E does not change preview workflow behavior, publication
semantics, texture resolution, graph routing, provider matching, V5 format, or
tool-surface enum identity. ML-E closeout verified this remains proof-only, and
ML-F closes the post-migration polish track with documentation evidence only.

## Post-Migration Polish Closeout

The Material Lab post-migration polish track is complete through `ML-F`:

- `ML-A` added structured Material Lab diagnostic rows and preview-status
  presentation DTOs while keeping the old provider text lines compatible.
- `ML-B` hardened the provider-owned graph interaction route from generic
  `GraphCanvasAction` input to Material Lab commands without moving Material
  Graph semantics back into `editor_shell`.
- `ML-C` surfaced existing preview product, publication, last-good, shader
  artifact, and viewport-product status without changing preview or publication
  behavior.
- `ML-D` surfaced texture/material resource-binding diagnostics for observable
  missing, ambiguous, incompatible, unsupported, unresolved, and generated
  texture states without changing texture resolution behavior.
- `ML-E` proved the graph canvas, inspector, and preview as stable-key-native
  surfaces through V5 workspace round trip, stable-key-only fixture persistence,
  and provider resolution after load.

The final Material Lab surface state is stable-key-native: graph canvas uses
`runenwerk.material_lab.graph_canvas` with
`ToolSurfaceRoute::ProviderOwnedGraphCanvas`; inspector uses
`runenwerk.material_lab.inspector` with `ToolSurfaceRoute::ProviderOwnedLocal`;
preview uses `runenwerk.material_lab.preview` with
`ToolSurfaceRoute::ProviderOwnedLocal`. Stable keys are the suite surface
identity. Historical `ToolSurfaceKind` material variants remain compatibility
metadata only and are not required by the stable-key-only Material Lab fixture or
provider resolution proof.

The polish track did not change preview workflow behavior, publication or
last-good semantics, texture resource resolution behavior, provider matching,
graph routing architecture, V5 persistence format, renderer/product ownership,
dynamic plugin policy, or enum-backed switch-menu behavior. Texture viewer
handoff, texture graph tooling, richer product lineage behavior, material asset
save/load polish, deeper generated shader/product diagnostics, and broader
Material Lab UX layout polish remain future product work.

Perfectionist V1, owned by `WR-028`, additionally requires:

- a retained graph canvas with node boxes, ports, edges, selection, gestures,
  palette search, typed properties, texture picker, validation overlays,
  diagnostics navigation, undo/redo, shortcuts, and source-owned layout
  persistence;
- live texture visibility for material texture nodes, including Texture2D
  thumbnails and Texture3D slice/mip/channel inspection backed by catalog KTX2
  descriptors and runtime sampling evidence;
- persisted scene material palette slots and assignments for SDF primitives
  through `editor_scene` source state;
- generated scene WGSL selecting material output by the hit SDF primitive's
  material slot, not one global active material;
- GPU proof that material graph edits and texture nodes affect both preview and
  scene pixels.

`WR-029` owns the deferred model/mesh source identity, submesh/material-region
assignment, and any future `renderable_index` renderer ABI extension. WR-028
evidence must not erase those gaps.

`WR-030` is the follow-up renderable-surface prerequisite for WR-029 Phase 4.
Its first accepted direction is a source-backed Mesh Preview product surface for
Material Lab rather than immediate general editor-scene mesh extraction. The
preview surface must consume the same scene-owned model/mesh material
assignments and generated material table as the SDF scene path, while preserving
Material Lab source documents and scene material assignment state as separate
truth.

## Non Goals

- No canvas-only material truth.
- No renderer-specific material graph ownership in domain code.
- No descriptor-only texture previews or status-panel-only graph completion.
- No global active material shader replacement for per-object material binding.
- No prefab material binding until prefab V2 has source/catalog identity.

## Roadmap Visibility

Material Lab must appear explicitly in the editor roadmap and workspace roadmap so it is not mistaken for a missing design. UI Designer remains the already-promoted self-authoring path; Material Lab is a separate material authoring track.

`WR-021` remains completed as `runtime_proven`. `WR-028` is the follow-up repair
row that must close the rich visual graph editor, live texture views, and
SDF primitive scene material binding gaps before that bounded path can claim
`perfectionist_verified`. Model/mesh material binding remains open under
`WR-029`, with `WR-030` providing the Mesh Preview product-surface contract
needed before WR-029 can claim model/mesh pixel proof.

## Tests

Required coverage:

- source-backed material graph round trip;
- ratification blocks invalid graph products;
- failed preview keeps previous valid product;
- material preview target uses viewport product selection;
- provider surfaces fail closed until product handoff is available;
- graph editor interaction evidence for pan/zoom/select, node/edge editing,
  texture picking, overlays, undo/redo, and layout persistence;
- scene material assignment evidence proving SDF primitives consume
  source-backed material slots;
- GPU pixel evidence proving Texture2D and Texture3D/triplanar nodes affect
  visible preview and scene output through real GPU bindings.
