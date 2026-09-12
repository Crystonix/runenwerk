---
title: Render Product Surface Foundation Bundle Design
description: No-compromise render update bundle for dynamic product surfaces, target aliases, prepared render views, history retention, inspection, and proof workloads.
status: implemented
owner: engine
layer: engine-runtime
canonical: true
last_reviewed: 2026-09-12
related_designs:
  - ../accepted/editor-native-multi-window-presentation-design.md
  - ../accepted/render-fragment-data-driven-maturity-design.md
  - ./viewport-dynamic-product-target-allocation-design.md
  - ./workspace-viewport-expression-upgrade-design.md
  - ../active/engine-game-runtime-editor-ecs-scripting-hot-reload-design.md
related_roadmaps:
  - ../../engine/plugins/render/docs/roadmap.md
  - ../../engine/roadmaps/render-final-architecture-migration.md
  - ../../apps/runenwerk-editor/viewport-expression-implementation-roadmap.md
related:
  - ../../engine/reference/plugins/render/architecture.md
  - ../../engine/reference/plugins/render/render-target-architecture.md
---

# Render Product Surface Foundation Bundle Design

## Status

Implemented foundation bundle with follow-up hardening tracked in the render roadmap.

This bundle replaces piecemeal render feature work with one coherent foundation: product surfaces can be declared, prepared, allocated, rendered into, sampled by UI, retained across frames, inspected, and proven with real workloads. The bundle is intentionally larger than the editor viewport V5 target-allocation phase because allocation alone would still leave producer execution dependent on another bridge.

## Goal

Make render product surfaces a first-class engine capability:

```text
RenderFlow authoring
  -> compiled target aliases and binding plans
  -> prepared render views / flow invocations
  -> dynamic target requests and history requests
  -> renderer-owned target and history caches
  -> pass execution into dynamic or flow-owned targets
  -> UI sampling / debug inspection
  -> app-level product presentation
```

The end state must support editor viewports, future asset/material/field previews, debug texture viewers, SDF/raymarch flows, boids/particle-style workloads, and temporal/history resources without introducing editor-specific render APIs.

## No-Compromise Outcomes

- Dynamic texture targets are both writable render attachments and sampleable resources where their descriptor allows it.
- The same compiled flow can execute for multiple prepared render views or product jobs without cloning the static flow registry.
- Target identity is explicit: flow-owned target, dynamic target key, or surface target. It is never hidden in string-suffixed static labels.
- Prepared render frames carry all render-relevant product target, view, history, uniform, and dispatch data needed for submission.
- Render submission performs no live ECS extraction to discover targets, views, uniforms, dispatch, or product bindings.
- Renderer caches own backend objects and expose inspection metadata without leaking `wgpu` handles into domain/app/UI data.
- UI viewport embeds can sample dynamic targets through backend-neutral binding sources.
- History and retention policy are explicit, deterministic, and invalidated by view/target signature changes.
- The editor viewport cutover lands as V5+V6 behavior: targets are allocated and producers write into them through the final render path.

## Bundle Scope

This bundle includes:

- R4 binding model closeout for real graphics/resource-heavy workflows.
- Dynamic texture target descriptors, prepared-frame requests, and renderer cache.
- Dynamic target aliasing so passes can write to runtime-selected product targets.
- Prepared render views and per-view or per-job flow invocations.
- Persistent/history resource retention and invalidation enough for viewport/product surfaces and future temporal workflows.
- UI dynamic texture sampling through `ViewportSurfaceBindingSource`.
- Render inspection for dynamic targets, flow invocations, target alias resolution, and history retention.
- End-to-end editor viewport proof with multiple independent viewports.
- Boids and SDF/raymarch proof workloads on the same public render API.

This bundle does not implement every future product producer. It creates the render foundation those producers need.

## Explicit Non-Goals

- No editor-only concepts in `engine/src/plugins/render`.
- No one-`RenderFlow`-per-viewport workaround.
- No static label generation such as `editor.viewport.42.scene_color` as the final identity model.
- No backend texture handles in `domain/ui/ui_render_data`.
- No general render-graph scheduler rewrite.
- No full OS/window multi-swapchain productization unless a concrete app task requires it. This bundle supports offscreen prepared render views; native multi-window presentation can follow on top.

## Core Concepts

### Product Surface

A product surface is a renderable texture target that represents a consumer-facing product such as scene color, picking ids, overlay, material preview, field slice, SDF preview, debug output, or a history texture.

Product semantics belong in app/domain code. The render engine only sees target descriptors and prepared flow invocations.

### Dynamic Target Key

A dynamic target key is an opaque engine-runtime address:

```rust
pub struct RenderDynamicTextureTargetKey {
    pub namespace: String,
    pub target_id: String,
}
```

The key is not semantic truth. App/domain code owns semantic keys such as `(ViewportId, ExpressionProductId, ViewportSurfacePresentationSlot)` and translates them into dynamic target keys before render prepare.

### Target Alias

A target alias is a static flow authoring placeholder that is resolved per prepared render view or per flow invocation:

```text
flow authoring target alias: "viewport.scene_color"
prepared invocation binding: "viewport.scene_color" -> dynamic target key "editor.viewport.17.scene_color.primary"
```

Target aliases let one compiled flow render many product jobs without mutating the flow registry or compiling one flow per viewport.

### Prepared Render View

A prepared render view is an execution scope with dimensions, scale policy, target alias bindings, history signature, and per-view prepared data. It may represent the main swapchain view or an offscreen product view.

This extends the existing `engine/src/plugins/render/frame/view.rs::PreparedViewFrame` direction instead of inventing viewport-specific renderer state.

### Prepared Flow Invocation

A prepared flow invocation binds a compiled flow to one prepared render view and carries the flow inputs for that invocation:

- projected uniform bytes;
- per-invocation uniform overrides for product/view-local data prepared by the caller;
- projected dispatch workgroups;
- target alias bindings;
- history/resource signatures;
- feature contribution status for that invocation.

This replaces the current assumption that `engine/src/plugins/render/frame/packet.rs::PreparedFlowInputs` is one global input set per flow.
Product invocations that produce offscreen textures must execute before a main-surface presentation invocation samples those products, so resize-time target allocation cannot expose blank newly allocated textures to UI composite.

## Engine Contract

### Resource And Target Descriptors

Owning modules:

```text
engine/src/plugins/render/resource/descriptors.rs
engine/src/plugins/render/resource/dynamic_target.rs
engine/src/plugins/render/resource/lifetime.rs
engine/src/plugins/render/resource/usages.rs
```

Required changes:

- add explicit texture dimensions, format policy, usage flags, sample mode, and retention policy for dynamic targets;
- audit flow-owned color, depth, storage, sampled, and history descriptors so runtime allocation is not forced to infer all texture details from swapchain size and format;
- keep convenience defaults for common flow-owned targets, but make the underlying runtime descriptor explicit;
- validate unsupported format/usage/sample combinations before renderer allocation.

### Authoring And Compilation

Owning modules:

```text
engine/src/plugins/render/api/flow.rs
engine/src/plugins/render/api/passes.rs
engine/src/plugins/render/graph/execution_plan.rs
engine/src/plugins/render/graph/validation.rs
```

Required changes:

- finish binding model parity for sampled textures, storage textures, vertex buffers, instance buffers, index buffers, indirect buffers, and storage access;
- add target alias declaration APIs for dynamic attachment slots;
- compile target aliases into explicit execution metadata;
- reject unresolved target aliases unless a flow declares them as optional diagnostic outputs;
- validate that fullscreen/graphics passes write only compatible target categories.

The public API should keep the normal static target path simple while making advanced dynamic products discoverable.

### Prepared Frame Boundary

Owning modules:

```text
engine/src/plugins/render/frame/packet.rs
engine/src/plugins/render/frame/view.rs
engine/src/plugins/render/runtime/frame_prepare.rs
engine/src/plugins/render/runtime/frame_submit.rs
```

Required changes:

- extend `PreparedRenderFrame` with dynamic target request snapshots;
- replace per-flow-only inputs with prepared flow invocations keyed by flow id and prepared view id or invocation id;
- carry target alias bindings in the prepared packet;
- carry invocation-local uniform bytes in the prepared packet rather than deriving product camera/target data during submit;
- carry history signatures and invalidation causes;
- keep submit free of live ECS extraction.

### Renderer Execution

Owning modules:

```text
engine/src/plugins/render/renderer/mod.rs
engine/src/plugins/render/renderer/dynamic_targets.rs
engine/src/plugins/render/renderer/render_flow/runtime_resources.rs
engine/src/plugins/render/renderer/render_flow/runtime_resources/resolve.rs
engine/src/plugins/render/renderer/render_flow/execute.rs
engine/src/plugins/render/renderer/render_flow/execute_passes.rs
engine/src/plugins/render/renderer/render_flow/bindings.rs
```

Required changes:

- add a renderer-owned dynamic target cache;
- add a shared resolution path for flow-owned, surface, dynamic, and history targets;
- execute compiled flows for each prepared flow invocation;
- resolve target aliases to dynamic or surface targets before pass encoding;
- bind invocation-scoped uniform buffers so per-view product uniforms cannot be overwritten by later invocations before the GPU executes the command buffer;
- include resolved resource identity in bind-group cache keys so invocation-local uniforms and dynamic targets with the same generation never alias another prepared invocation's bindings;
- report invocation-scoped uniform buffers through runtime inspection after prepared invocations have encoded;
- reallocate only resources whose descriptor signature changed;
- preserve previous valid dynamic/history targets when a new request is invalid and the retention policy allows reuse-last-good;
- fail loudly when a pass requests an unbound required dynamic target.

### UI Sampling

Owning modules:

```text
domain/ui/ui_render_data/src/primitives/viewport_surface_embed.rs
engine/src/plugins/render/renderer/setup.rs
```

Required changes:

- make `ViewportSurfaceBindingSource::DynamicTexture` the only viewport surface binding source;
- remove the old flow-resource viewport embed compatibility path instead of preserving it as a parallel bridge;
- key UI viewport embed bind groups by full binding source;
- reject normal UI sampling for non-sampleable dynamic target descriptors such as picking id targets.

### Inspection

Owning modules:

```text
engine/src/plugins/render/inspect/resource_inspector.rs
engine/src/plugins/render/inspect/texture_view.rs
engine/src/plugins/render/inspect/graph_dump.rs
engine/src/plugins/render/inspect/timings.rs
```

Required changes:

- expose dynamic target key, label, dimensions, format, usage, sample mode, retention policy, generation, stale/valid state, and last invalidation reason;
- expose prepared flow invocations and their view/target alias bindings;
- expose history resources and view signatures;
- make dynamic product textures viewable through debug texture tooling when their format is displayable.

## Implementation Phases

### RB0 - Freeze Bridge Growth

Change:

- `engine/tests/render_cutoff_guard.rs`
- `apps/runenwerk_editor/tests/viewport_architecture_guards.rs`
- `docs-site/src/content/docs/engine/plugins/render/docs/roadmap.md`

Exit criteria:

- guards reject new static viewport product ids and flow-per-viewport workarounds;
- docs mark the product-surface bundle as the active render update.

### RB1 - Binding Model Closeout

Change:

- `engine/src/plugins/render/api/bindings.rs`
- `engine/src/plugins/render/api/passes.rs`
- `engine/src/plugins/render/graph/execution_plan.rs`
- `engine/src/plugins/render/graph/validation.rs`
- `engine/src/plugins/render/renderer/render_flow/bindings.rs`
- `engine/src/plugins/render/renderer/render_flow/execute_passes.rs`

Exit criteria:

- graphics, fullscreen, compute, copy, present, and UI composite pass binding contracts are coherent;
- sampled/storage texture, vertex, instance, index, indirect, and storage access validation is covered;
- unsupported runtime paths fail loudly with actionable diagnostics.

Validation:

```text
cargo test -p engine --test render_flow_v2
cargo test -p engine --test render_resource_model
```

### RB2 - Texture Descriptor And Target Policy

Change:

- `engine/src/plugins/render/resource/descriptors.rs`
- `engine/src/plugins/render/resource/dynamic_target.rs`
- `engine/src/plugins/render/resource/usages.rs`
- `engine/src/plugins/render/renderer/render_flow/runtime_resources/realize.rs`

Exit criteria:

- target dimensions, format, usage, sample mode, and retention are explicit;
- flow-owned targets can keep ergonomic defaults while exposing a real descriptor signature;
- renderer allocation no longer assumes every non-surface target is surface-sized and surface-formatted.

### RB3 - Prepared Render Views And Flow Invocations

Change:

- `engine/src/plugins/render/frame/view.rs`
- `engine/src/plugins/render/frame/packet.rs`
- `engine/src/plugins/render/runtime/frame_prepare.rs`
- `engine/src/plugins/render/runtime/frame_submit.rs`

Exit criteria:

- prepared frame carries offscreen render views as first-class execution scopes;
- prepared frame carries per-invocation flow inputs instead of one mutable input set per flow;
- submit consumes the prepared packet only.

### RB4 - Dynamic Target Cache And Target Alias Resolution

Change:

- `engine/src/plugins/render/api/flow.rs`
- `engine/src/plugins/render/api/passes.rs`
- `engine/src/plugins/render/resource/dynamic_target.rs`
- `engine/src/plugins/render/runtime/dynamic_targets.rs`
- `engine/src/plugins/render/renderer/dynamic_targets.rs`
- `engine/src/plugins/render/renderer/render_flow/runtime_resources/resolve.rs`

Exit criteria:

- passes can write to target aliases resolved to dynamic targets in prepared flow invocations;
- dynamic targets can be sampled by later passes where descriptors allow sampling;
- invalid or missing required alias bindings fail loudly;
- resize/reallocate is scoped to the changed target key.

### RB5 - History And Retention

Change:

- `engine/src/plugins/render/resource/lifetime.rs`
- `engine/src/plugins/render/resource/transient.rs`
- `engine/src/plugins/render/renderer/dynamic_targets.rs`
- `engine/src/plugins/render/renderer/render_flow/runtime_resources.rs`

Exit criteria:

- history targets and dynamic targets share a deterministic retention/invalidation model;
- view signature changes invalidate only affected history/product targets;
- retained targets expose stale/valid state for reuse-last-good workflows.

### RB6 - UI Dynamic Sampling And Debug Texture Viewing

Change:

- `domain/ui/ui_render_data/src/primitives/viewport_surface_embed.rs`
- `engine/src/plugins/render/renderer/setup.rs`
- `engine/src/plugins/render/inspect/texture_view.rs`

Exit criteria:

- UI viewport embeds can bind dynamic texture sources;
- non-sampleable targets cannot be silently sampled;
- displayable dynamic targets can be inspected through render debug tooling.

### RB7 - Editor Viewport End-To-End Cutover

Change:

- `apps/runenwerk_editor/src/runtime/viewport/product_targets.rs`
- `apps/runenwerk_editor/src/runtime/viewport/render_jobs.rs`
- `apps/runenwerk_editor/src/runtime/viewport/producer_scene.rs`
- `apps/runenwerk_editor/src/runtime/viewport/surface_set.rs`
- `apps/runenwerk_editor/src/runtime/viewport/presentation_resolver.rs`
- `assets/shaders/editor_viewport_scene_product.wgsl`

Exit criteria:

- editor viewport V5 and V6 land together as a visible cutover;
- each viewport job renders scene color, picking ids, and overlay into its own dynamic targets;
- UI presentation samples the owning viewport's dynamic scene color target;
- shader-side multi-rectangle viewport containment is removed.

Validation:

```text
cargo test -p runenwerk_editor viewport
cargo test -p runenwerk_editor --test startup_render_smoke
cargo test -p runenwerk_editor --test viewport_architecture_guards
RUNENWERK_ENABLE_GPU_SMOKE=1 RUNENWERK_ENABLE_MACOS_MAIN_THREAD_GPU_SMOKE=1 cargo test -p runenwerk_editor --test viewport_gpu_truth_smoke -- --ignored
```

### RB8 - Feature Proofs

Status: implemented. Boids now proves compute, storage, graphics, instance draw-buffer binding, history copy, and present through the public render API. SDF now proves compute preparation, fullscreen compose, invocation-scoped history copy, and present through the public render API. Dynamic target alias execution is closed through prepared render views and flow invocations.

Change:

- `engine/examples/boids_render_flow/`
- `engine/examples/sdf_render_flow/`
- `docs-site/src/content/docs/engine/examples/boids-render-flow/README.md`
- `docs-site/src/content/docs/engine/examples/sdf-render-flow/README.md`

Exit criteria:

- boids proves compute + storage + graphics + draw buffers + present without custom executors;
- SDF proves compute/fullscreen + dynamic or flow-owned targets + history/copy/present without custom executors;
- examples use the public render API and reveal no special-case editor dependencies.

### RB9 - Inspection, Docs, And Cleanup

Status: implemented for the foundation bundle. Render reference docs describe dynamic target descriptors, target aliases, prepared render views, flow invocations, UI sampling boundaries, dynamic-only viewport embeds, and history retention. `engine::plugins::render::inspect::inspect_prepared_render_frame` exposes prepared views, dynamic target descriptors, flow invocations, target alias bindings, and history signatures. Follow-up roadmap work is limited to broader feature maturity and API polish, not migration bridges.

Change:

- `engine/src/plugins/render/inspect/*`
- `docs-site/src/content/docs/engine/reference/plugins/render/*`
- `docs-site/src/content/docs/engine/plugins/render/docs/roadmap.md`
- `docs-site/src/content/docs/engine/roadmaps/render-final-architecture-migration.md`

Exit criteria:

- render docs describe dynamic targets, target aliases, prepared render views, flow invocations, UI sampling, and history retention;
- migration-only flow-resource viewport bindings are removed or guarded;
- inspection output is useful enough to debug target/view/flow invocation problems without reading renderer internals.

## Final Acceptance Criteria

The render product surface bundle is complete only when:

- one compiled flow can execute against multiple prepared offscreen render views without cloning the flow registry;
- dynamic targets are writable attachments and sampleable resources according to their descriptors;
- renderer-owned dynamic and history caches expose deterministic generation and invalidation metadata;
- `PreparedRenderFrame` contains all target/view/invocation data needed by submit;
- UI embeds can sample dynamic targets through backend-neutral binding sources;
- editor split viewports render independent scene color, picking, and overlay products without static shared viewport resources;
- boids and SDF proof workloads run through builtin compiled execution only;
- architecture guards prevent reintroducing flow-per-viewport, shared static viewport product ids, or submit-time target extraction.

## Relationship To Existing Roadmaps

This bundle pulls these render roadmap items forward together:

- R4 binding model expansion;
- R-DT dynamic texture target allocation;
- the relevant part of render final architecture Phase 8 for prepared render views and view/history invalidation;
- R8 persistent/history resource support needed by product surfaces;
- R9 inspection/tooling for target and invocation debugging;
- R6 and R7 proof workloads after the foundation is in place.

This does not make every later render feature complete. Full native multi-window presentation is specified separately in `docs-site/src/content/docs/design/accepted/editor-native-multi-window-presentation-design.md`, and full fragment/data-driven maturity is specified in `docs-site/src/content/docs/design/accepted/render-fragment-data-driven-maturity-design.md`.
