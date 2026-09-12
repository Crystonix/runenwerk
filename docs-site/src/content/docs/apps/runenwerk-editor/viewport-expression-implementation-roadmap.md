---
title: Viewport Expression Architecture Implementation Roadmap
description: End-to-end no-compromise roadmap for replacing the tactical viewport bridge with per-viewport expression products, render targets, and presentation ownership.
status: active
owner: editor
layer: app
canonical: true
last_reviewed: 2026-09-12
related_designs:
  - ../../design/implemented/workspace-viewport-expression-upgrade-design.md
  - ../../design/implemented/render-product-surface-foundation-bundle-design.md
  - ../../design/implemented/viewport-dynamic-product-target-allocation-design.md
  - ../../design/active/editor-ui-workspace-tool-surface-architecture.md
  - ../../design/implemented/editor-rendered-world-and-multi-entity-viewport-design.md
  - ../../design/implemented/field-visualizer-product-workflow-design.md
related_roadmaps:
  - ../../engine/roadmaps/fully-featured-renderer-roadmap.md
related:
  - ./current-architecture.md
  - ./roadmap.md
  - ./execution-priority-checklist.md
---

# Viewport Expression Architecture Implementation Roadmap

## Goal

Replace the current split-viewport bridge with the final viewport architecture:

```text
Workspace tool surface
  -> ViewportInstanceRegistry
  -> ViewportLayoutMap
  -> ViewportRenderState
  -> ViewportRenderJob
  -> per-viewport expression product targets
  -> ViewportSurfaceBindingRegistry
  -> UI ViewportSurfaceEmbed
```

The final system has no fullscreen viewport masking as its containment model, no shared scene-color product for multiple viewports, no four-viewport shader uniform, no hidden first-observed viewport fallback, and no viewport identity derived implicitly from shell ids as the long-term authority.

Phasing is allowed only to keep patches reviewable. Each phase must move the code toward the final contract and must not add another bridge that becomes a second architecture.

If the render foundation update is active, V5 and V6 should be executed through `docs-site/src/content/docs/design/implemented/render-product-surface-foundation-bundle-design.md` so dynamic targets are allocated, written, sampled, inspected, and proven as one end-to-end render surface capability.

## Doctrine Alignment

This roadmap follows the repository doctrine:

- Domains validate: viewport and expression contracts live in `domain/editor/editor_viewport` when they are engine-agnostic semantic contracts.
- Commands mutate: user-facing viewport configuration changes go through shell/editor commands, not renderer side effects.
- Projections are derived: layout maps, render jobs, surface bindings, and observation frames are rebuilt from authoritative viewport/tool-surface state.
- Runtime composes: `apps/runenwerk_editor` and `engine/src/plugins/render` own executable render target allocation, pass execution, and backend-specific handles.
- Tests protect: every phase adds architecture guards that prevent returning to global viewport products or shader-side panel clipping.

## Current State

Implemented foundation state:

- `apps/runenwerk_editor/src/runtime/viewport/instance_registry.rs::ViewportInstanceRegistryResource` owns explicit viewport instance allocation, lookup, close, duplicate, and persisted restore mapping.
- `domain/editor/editor_shell/src/workspace/state.rs::ToolSurfaceState` carries optional `ViewportId` restore metadata and viewport runtime settings for saved viewport surfaces without making `ViewportId` a structural workspace id.
- `domain/editor/editor_viewport/src/camera.rs::ViewportRuntimeSettings` owns the engine-agnostic camera, debug, root-background, and selected-product settings that persistence and shell commands share.
- `apps/runenwerk_editor/src/runtime/viewport/render_state.rs::ViewportRenderStateResource` records per-viewport bounds and the viewport-local render-state snapshot used for prepared render jobs and picking.
- `apps/runenwerk_editor/src/runtime/viewport/render_state.rs::ViewportRenderStateCommandQueueResource` applies per-viewport camera, debug, and root-background commands after input and before lifecycle/frame submission.
- `apps/runenwerk_editor/src/runtime/systems/input_bridge.rs::dispatch_editor_input_system` routes viewport-local orbit, pan, and zoom input to the owning viewport id.
- `apps/runenwerk_editor/src/runtime/systems/viewport_lifecycle.rs::sync_viewport_instances_system` syncs viewport instances from workspace state before shell frame projection, prunes closed viewport runtime state, and persists viewport-owned settings back to workspace state; frame submit no longer allocates/restores viewport identity or owns live viewport singleton state.
- `apps/runenwerk_editor/src/shell/providers/scene/viewport.rs::SceneViewportProvider::build_frame` no longer inherits an unrelated lone observed viewport for unbound panels.
- `apps/runenwerk_editor/src/runtime/viewport/product_targets.rs::sync_viewport_presentation_products_system` updates product descriptors by viewport id and dimensions.
- `apps/runenwerk_editor/src/runtime/viewport/product_targets.rs::ViewportProductTargetRegistryResource` maps every viewport/product/slot tuple to a dynamic target record.
- `apps/runenwerk_editor/src/runtime/viewport/render_jobs.rs::ViewportRenderJobResource` publishes one prepared view and one prepared flow invocation per viewport without cloning the render flow.
- `domain/ui/ui_render_data/src/primitives/viewport_surface_embed.rs::ViewportSurfaceBindingSource` is dynamic-texture-only; the old flow-resource embed bridge has been removed.
- `apps/runenwerk_editor/src/runtime/systems/frame_submit.rs::extract_viewport_scene_render_packet_with_material_slots` now extracts all editor SDF primitive entities, including material-slot selection, into a stable per-viewport scene packet for the current rendered-world path.
- `assets/shaders/editor_viewport_scene_product.wgsl` renders a viewport-local multi-entity SDF primitive product and no longer contains multi-rectangle containment.
- `assets/shaders/editor_viewport_picking_product.wgsl` consumes the same primitive packet layout for picking-id product output.

Remaining work is no longer migration cleanup. The planned non-viewport surface
workflow follow-up has landed, reusable viewport options now use retained toggle
controls, and the viewport product catalog exposes descriptor rows for field,
atlas, volume slice, brickmap debug, and history color products. Products whose
runtime producers are not implemented are visible but marked unavailable. The
Field Visualizer viewport workflow and presentation controls are implemented;
remaining field-product producer breadth and Material Lab producer maturity are
separately owned work that must continue through the same viewport product
routing rather than descriptor plumbing or parallel viewer paths.

Long-term renderer capability sequencing for product producers, storage-buffer
scene packets, SDF/world rendering, materials, temporal history, fragments, and
inspection is tracked in
[Fully Featured Renderer Roadmap](../../engine/roadmaps/fully-featured-renderer-roadmap.md).

## Final Architecture

### Viewport Instance Ownership

Owning module:

```text
apps/runenwerk_editor/src/runtime/viewport/instance_registry.rs
```

Final responsibilities:

- allocate and retain explicit `ViewportInstanceRecord` values;
- map `ToolSurfaceInstanceId` to `ViewportId`;
- persist or restore viewport instance identity where workspace layout persistence needs it;
- record document/workspace context for the viewport;
- define duplication policy: copy camera and presentation state, then fork future mutations;
- remove viewport instances when their owning tool surface is closed and no durable restored reference remains.

### Viewport Product Contracts

Owning domain module:

```text
domain/editor/editor_viewport/src/
```

Final responsibilities:

- `ViewportId`, `ExpressionProductId`, `ExpressionProductKind`, and `ExpressionProductDescriptor`;
- viewport presentation state and product-selection semantics;
- product source reality, freshness, presentation hints, and product dimensions;
- engine-agnostic contracts for scene color, picking ids, overlay, depth,
  diagnostics, scalar field, vector field, atlas, volume slice, brickmap debug,
  and history color products.

These contracts must stay engine-agnostic. GPU texture handles, render pass labels, and backend resource ids remain runtime/engine concerns.

### Viewport Runtime State

Owning module:

```text
apps/runenwerk_editor/src/runtime/viewport/render_state.rs
```

Final responsibilities:

- viewport bounds in physical pixels;
- render dimensions and scale policy;
- viewport camera state;
- selected debug/presentation mode;
- source reality version;
- visibility/throttling state;
- target allocation status;
- last rendered product revisions.

The singleton `EditorViewportRenderState` in `apps/runenwerk_editor/src/runtime/resources.rs` should be decomposed. Shared defaults can remain as helper functions, but live viewport state must be keyed by `ViewportId`.

### Viewport Render Jobs

Owning module:

```text
apps/runenwerk_editor/src/runtime/viewport/render_jobs.rs
```

Final responsibilities:

- derive one `ViewportRenderJob` per visible viewport;
- copy only the viewport-local data needed for rendering;
- specify target product keys for scene color, picking ids, overlay, and later depth/debug outputs;
- carry viewport-local camera/projection state;
- carry exact render dimensions and source version;
- expose empty/no-op jobs for invalid or hidden viewports instead of silently rendering to a global target.

`submit_editor_frame_system` should not build render jobs directly. It should consume surface bindings and submit the shell frame after viewport systems have already resolved lifecycle, layout, products, and render jobs.

### Dynamic Product Target Allocation

Owning engine/app integration modules:

```text
engine/src/plugins/render/
apps/runenwerk_editor/src/runtime/viewport/producer_scene.rs
apps/runenwerk_editor/src/runtime/viewport/surface_set.rs
```

Final responsibilities:

- allocate or reuse render targets by `(ViewportId, ViewportSurfaceSlot, ExpressionProductId)`;
- resize targets when viewport dimensions change;
- preserve previous valid targets if a producer fails;
- publish concrete surface handles for UI embedding;
- keep resource identity unique per viewport product.

The static ids `editor.viewport.v1.scene_color`, `editor.viewport.v1.picking_ids`, and `editor.viewport.v1.overlay` must be replaced by viewport-scoped target keys or dynamic render resource handles.

### Scene Product Producer

Owning module:

```text
apps/runenwerk_editor/src/runtime/viewport/producer_scene.rs
```

Final responsibilities:

- consume `ViewportRenderJob` records;
- render each visible viewport job into that viewport's scene color target;
- render picking ids into the matching viewport picking target;
- render overlay products into the matching viewport overlay target;
- never render all panels by testing multiple rectangles in one fragment shader;
- expose producer health and product availability per viewport.

The scene producer may stay app-owned while the generic render graph is being extended. It must not leak editor-specific viewport concepts into generic engine APIs.

### UI Embedding And Presentation Resolution

Owning modules:

```text
domain/editor/editor_shell/src/composition/build_viewport_panel.rs::build_viewport_panel
apps/runenwerk_editor/src/runtime/viewport/presentation_resolver.rs::build_surface_binding_registry
```

Final responsibilities:

- shell embeds `(ViewportId, ViewportSurfacePresentationSlot)`;
- presentation resolver maps the viewport's selected expression products to concrete viewport-owned surface handles;
- UI compositor samples the bound surface for that embed only;
- missing products render explicit unavailable/diagnostic presentation rather than falling back to another viewport.

### Viewport-Local Interaction

Owning modules:

```text
apps/runenwerk_editor/src/runtime/systems/input_bridge.rs
apps/runenwerk_editor/src/runtime/systems/picking.rs
apps/runenwerk_editor/src/editor_features/viewport/
```

Final responsibilities:

- pointer routing resolves a `ViewportId` and viewport-local coordinates;
- picking reads the viewport's own picking product or equivalent viewport-local analytic picking result;
- camera orbit/pan/zoom mutates only the selected viewport instance;
- transform tools and selection use viewport-local rays;
- duplicating a viewport copies camera/presentation state and then forks future state.

## Implementation Phases

### V0 - Freeze Tactical Bridge Expansion

Purpose: prevent the current bridge from growing.

Implementation targets:

- `apps/runenwerk_editor/tests/viewport_architecture_guards.rs`
  - add guards that reject adding more `viewport_b`-style uniform fields;
  - add guards that reject new code paths depending on a first observed viewport after projection exists;
  - add guards that reject using static viewport product resource ids in the normal runtime path.
- `docs-site/src/content/docs/apps/runenwerk-editor/current-architecture.md`
  - mark the current multi-rect uniform and static render resources as migration-only.

Exit criteria:

- Architecture guard tests fail if bridge scope increases.
- Documentation clearly labels migration-only code.

Validation:

```text
cargo test -p runenwerk_editor --test viewport_architecture_guards
python3 tools/docs/validate_docs.py
```

### V1 - Explicit Viewport Instance Registry

Purpose: replace implicit viewport identity derivation with explicit viewport instances.

Implementation targets:

- `apps/runenwerk_editor/src/runtime/viewport/instance_registry.rs`
  - add `ViewportInstanceRegistryResource`;
  - add `ViewportInstanceRecord`;
  - add allocation, lookup, duplicate, close, retain, and restore APIs;
  - make `ToolSurfaceInstanceId -> ViewportId` a stored mapping.
- `apps/runenwerk_editor/src/runtime/plugin.rs::EditorAppPlugin::build`
  - register `ViewportInstanceRegistryResource`.
- `apps/runenwerk_editor/src/shell/state.rs::RunenwerkEditorShellState`
  - expose enough structural events or workspace snapshots for the registry to create and prune viewport instances.
- `apps/runenwerk_editor/src/shell/providers/scene/viewport.rs::SceneViewportProvider::build_frame`
  - consume the registry-provided viewport id through the surface/resource context.

Exit criteria:

- Creating a viewport surface creates an explicit viewport instance.
- Duplicating a viewport creates a new viewport instance with copied camera/presentation state.
- Closing a viewport removes its transient instance and product state after the owning surface disappears.
- no derived `ToolSurfaceInstanceId -> ViewportId` helper remains in code or tests.

Validation:

```text
cargo test -p runenwerk_editor viewport_instance
cargo test -p runenwerk_editor viewport
cargo test -p editor_shell
```

### V2 - Viewport Lifecycle Before Shell Projection

Purpose: move viewport lifecycle out of `submit_editor_frame_system`.

Implementation targets:

- `apps/runenwerk_editor/src/runtime/systems/viewport_lifecycle.rs`
  - add a system that syncs viewport instances from workspace state before shell frame projection.
- `apps/runenwerk_editor/src/runtime/viewport/layout_map.rs::ViewportLayoutMapResource`
  - keep layout as derived projection output only.
- `apps/runenwerk_editor/src/runtime/systems/frame_submit.rs::submit_editor_frame_system`
  - remove viewport seeding and lifecycle decisions;
  - leave shell frame building and submission only.
- `apps/runenwerk_editor/src/shell/providers/mod.rs::SurfaceProviderBuildContext`
  - pass already-resolved viewport instance/product context to providers.

Exit criteria:

- Frame submit does not allocate, seed, or infer viewport identity.
- Layout extraction only records already-known viewport ids and bounds.
- Missing viewport instance produces a provider diagnostic or unavailable product frame.

Validation:

```text
cargo test -p runenwerk_editor viewport
cargo test -p runenwerk_editor --test startup_render_smoke
cargo test -p runenwerk_editor --test viewport_architecture_guards
```

### V3 - Per-Viewport Runtime State And Camera

Purpose: make all live viewport state viewport-owned.

Implementation targets:

- `apps/runenwerk_editor/src/runtime/viewport/render_state.rs::ViewportRenderStateResource`
  - preserve viewport-local camera/debug state across shell-frame render-state refreshes;
  - continue adding projection, scale policy, source version, render throttling, and target status as product needs require.
- `apps/runenwerk_editor/src/runtime/viewport/render_state.rs::ViewportRenderStateCommandQueueResource`
  - route per-viewport camera reset/set and debug state commands by `ViewportId`.
- `apps/runenwerk_editor/src/runtime/resources.rs::EditorViewportRenderState`
  - remove live singleton viewport fields after equivalent per-viewport fields exist;
  - keep only shared default helpers if still useful.
- `apps/runenwerk_editor/src/runtime/systems/input_bridge.rs`
  - route camera controls through `ViewportId`.
- `apps/runenwerk_editor/src/editor_features/viewport/`
  - use viewport-local camera state for tool rays and transform previews.

Exit criteria:

- Two visible viewports can have different camera state.
- Resizing one viewport changes only that viewport's render dimensions.
- Viewport debug/presentation options are per viewport where semantically per-viewport.

Validation:

```text
cargo test -p runenwerk_editor viewport_camera
cargo test -p runenwerk_editor viewport
cargo test -p runenwerk_editor --test scene_authoring_workflow_smoke
```

### V4 - Render Job Contract

Purpose: make rendering consume explicit jobs instead of singleton global state.

Implementation targets:

- `apps/runenwerk_editor/src/runtime/viewport/render_jobs.rs`
  - add `ViewportRenderJobResource`;
  - add `ViewportRenderJob`;
  - derive jobs from `ViewportInstanceRegistryResource`, `ViewportRenderStateResource`, and product/presentation state.
- `apps/runenwerk_editor/src/runtime/viewport/product_registry.rs`
  - connect descriptors to job-produced products.
- `apps/runenwerk_editor/src/runtime/systems/frame_submit.rs`
  - stop passing multi-viewport bounds into singleton render state.

Exit criteria:

- Rendering code can iterate visible viewport jobs.
- No render system needs to search the shell UI tree for render dimensions.
- Invalid or hidden viewports generate explicit no-op/unavailable state.

Validation:

```text
cargo test -p runenwerk_editor viewport_render_job
cargo test -p runenwerk_editor viewport
cargo test -p runenwerk_editor --test viewport_architecture_guards
```

### V5 - Dynamic Product Target Allocation

Purpose: remove shared static GPU product resources.

Owning detailed design:

- `docs-site/src/content/docs/design/implemented/viewport-dynamic-product-target-allocation-design.md`

Implementation targets:

- `engine/src/plugins/render/resource/dynamic_target.rs`
  - add backend-neutral dynamic target keys, descriptors, formats, usage flags, sample modes, and retention policy.
- `engine/src/plugins/render/runtime/dynamic_targets.rs`
  - add the request registry copied into each prepared render frame.
- `engine/src/plugins/render/renderer/dynamic_targets.rs`
  - add the renderer-owned dynamic texture target cache.
- `domain/ui/ui_render_data/src/primitives/viewport_surface_embed.rs`
  - change `ViewportSurfaceBinding` to support dynamic texture sources while keeping static flow resources as migration compatibility.
- `apps/runenwerk_editor/src/runtime/viewport/product_targets.rs`
  - add `ViewportProductTargetRegistryResource` and `sync_viewport_product_targets_system`.
- `apps/runenwerk_editor/src/runtime/viewport/surface_set.rs`
  - change `ViewportSurfaceHandle` from static resource labels to dynamic target binding sources.
- `apps/runenwerk_editor/src/runtime/viewport/producer_scene.rs`
  - isolate static resource ids to migration compatibility until V6 removes them or lands in the same branch.
- `apps/runenwerk_editor/src/runtime/viewport/presentation_resolver.rs::build_surface_binding_registry`
  - bind each viewport embed to that viewport's product target.

Exit criteria:

- Two viewports never bind the same scene color resource unless an explicit shared-product mode is selected.
- Product descriptors report dimensions matching their viewport targets.
- Closing a viewport releases or retires its targets.
- Resize reallocates only affected viewport targets.
- Static viewport resource ids cannot be used by normal viewport presentation outside a guarded migration path.

Validation:

```text
cargo test -p runenwerk_editor viewport_render_job
cargo test -p runenwerk_editor viewport_surface
cargo test -p runenwerk_editor --test startup_render_smoke
cargo test -p runenwerk_editor --test viewport_architecture_guards
python3 tools/docs/validate_docs.py
```

### V6 - Per-Viewport Scene, Picking, And Overlay Producers

Purpose: render each viewport from its own render job into its own products.

Implementation targets:

- `apps/runenwerk_editor/src/runtime/viewport/producer_scene.rs`
  - execute scene color per `ViewportRenderJob`;
  - execute picking ids per `ViewportRenderJob`;
  - execute overlay product per `ViewportRenderJob`;
  - publish producer health per viewport/product.
- `assets/shaders/editor_viewport_scene_product.wgsl`
  - remove `viewport_b`, `viewport_c`, `viewport_d`, and reserved multi-rectangle fields;
  - render using local target coordinates for the current job.
- `apps/runenwerk_editor/src/runtime/resources.rs::EditorViewportSceneProductUniform`
  - replace multi-rect fields with one viewport-local target/camera/product uniform.

Exit criteria:

- Five split viewports render correctly.
- Each viewport can show a different camera.
- Picking products are viewport-local.
- The shader no longer chooses between multiple panel rectangles.

Validation:

```text
cargo test -p runenwerk_editor viewport
cargo test -p runenwerk_editor --test viewport_branch_truth_smoke
RUNENWERK_ENABLE_GPU_SMOKE=1 RUNENWERK_ENABLE_MACOS_MAIN_THREAD_GPU_SMOKE=1 cargo test -p runenwerk_editor --test viewport_gpu_truth_smoke -- --ignored
```

### V7 - Presentation Modes And Product Switching

Purpose: prove the expression product model beyond one scene-color path.

Implementation targets:

- `domain/editor/editor_viewport/src/`
  - extend presentation state for primary product, overlays, compare mode, channel/slice/mip selections, and diagnostics visibility.
- `apps/runenwerk_editor/src/shell/providers/scene/viewport.rs`
  - expose product and presentation controls as provider-local actions routed through shell/editor commands.
- `apps/runenwerk_editor/src/runtime/viewport/product_registry.rs`
  - publish depth/debug/overlay descriptors where producers exist.
- `apps/runenwerk_editor/src/runtime/viewport/presentation_resolver.rs`
  - resolve primary, picking, overlay, and diagnostic slots without fallback to another viewport.

Exit criteria:

- Product selection is independent per viewport.
- Overlay selection is independent per viewport.
- Missing/unavailable products render explicit unavailable state.
- Product descriptors are visible in viewport details/statistics.

Validation:

```text
cargo test -p runenwerk_editor viewport_product
cargo test -p runenwerk_editor viewport
```

### V8 - Remove Migration Bridge

Purpose: delete tactical code after final contracts are live.

Implementation targets:

- `apps/runenwerk_editor/src/runtime/resources.rs`
  - remove multi-rect viewport fields and singleton live viewport state.
- `assets/shaders/editor_viewport_scene_product.wgsl`
  - remove rectangle-selection logic and fullscreen discard containment.
- `apps/runenwerk_editor/src/runtime/systems/frame_submit.rs`
  - remove viewport lifecycle/render-state/product sync responsibilities.
- `apps/runenwerk_editor/tests/viewport_architecture_guards.rs`
  - add final guards that fail if static shared products, multi-rect uniforms, or frame-submit lifecycle seeding return.

Exit criteria:

- No normal runtime code uses a shared static viewport scene color target.
- No shader uniform supports multiple panel rectangles.
- No frame-submit system allocates viewport ids or owns viewport lifecycle.
- Multiple viewports, resize, picking, product switching, and close/remount work through explicit viewport instances and products.

Validation:

```text
cargo fmt --all -- --check
cargo test -p runenwerk_editor viewport
cargo test -p runenwerk_editor --test startup_render_smoke
cargo test -p runenwerk_editor --test viewport_architecture_guards
python3 tools/docs/validate_docs.py
./quiet_full_gate.sh
```

## Final Acceptance Criteria

The implementation is complete only when all of these are true:

- `ViewportId` is allocated and retained by an explicit viewport instance registry.
- Every visible viewport has separate scene color, picking ids, and overlay product targets.
- Product descriptors, observations, and surface bindings are keyed by `ViewportId`.
- UI embeds only `(ViewportId, presentation slot)` and never samples a global viewport target accidentally.
- Picking and camera interaction use viewport-local coordinates and viewport-local camera state.
- Closing, splitting, duplicating, resizing, hiding, and restoring viewport surfaces preserve explicit lifecycle semantics.
- Product selection, overlay selection, diagnostics visibility, and camera state are independent per viewport.
- The shader no longer contains `viewport_b`, `viewport_c`, `viewport_d`, or reserved multi-rectangle uniform fields.
- `submit_editor_frame_system` owns shell frame projection and viewport layout extraction only; render-product target allocation, surface binding, and render job publication stay in viewport runtime systems.
- Architecture guards prevent reintroducing shared viewport products, first-observed fallbacks, or shader-side panel containment.

## Future Features After V8

These are not required to close the split-viewport architecture, but the final system should be ready for them:

- field, atlas, volume, brickmap, material, asset, and runtime-debug product producers;
- viewport-local quality, LOD, throttling, visibility, and render-budget policy;
- per-viewport temporal/history products such as previous-frame color, TAA inputs, and retained diagnostic buffers;
- dynamic render target inspection and debug texture viewer surfaces;
- product provenance, freshness, cache lineage, and producer diagnostics in the viewport details UI;
- multi-window, multi-surface, or engine multi-view execution if future editor windows require independent swapchains;
- streamed or collaborative viewport products if sharing/replication needs live preview surfaces.

Those features belong in their owning asset, render, field-world, material, runtime-debug, or networking roadmaps. This viewport roadmap finishes the ownership and execution boundary that lets those features plug in later.

## Non-Goals

These are not part of this roadmap:

- implementing every future field/atlas/volume/brickmap product;
- building the full material graph or asset pipeline;
- making generic engine APIs editor-specific;
- adding another short-term multi-viewport bridge.

The roadmap prepares the viewport architecture for those products by finishing ownership and execution boundaries first.
