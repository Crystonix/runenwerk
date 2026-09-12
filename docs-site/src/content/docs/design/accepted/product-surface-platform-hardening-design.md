---
title: Product Surface Platform Hardening Design
description: Accepted design for PM-RENDER-PG-005 shared product-surface hardening across viewport, preview, field/debug, drawing, and future producer families.
status: accepted
owner: engine
layer: engine-runtime / render product surfaces
canonical: true
last_reviewed: 2026-09-12
related_designs:
  - ./render-product-graph-platform-design.md
  - ./render-contract-ergonomics-design.md
  - ./render-execution-graph-compiler-maturity-design.md
  - ./feature-owned-render-contributions-design.md
  - ../implemented/render-product-surface-foundation-bundle-design.md
  - ../implemented/field-visualizer-product-workflow-design.md
  - ../active/material-lab-and-material-preview-design.md
  - ../active/drawing-authoring-and-comic-layout-platform-design.md
related_roadmaps:
  - ../../engine/roadmaps/fully-featured-renderer-roadmap.md
---

# Product Surface Platform Hardening Design

## Status

This is the accepted design contract for `PM-RENDER-PG-005`.

It accepts the shared product-surface platform hardening direction before
implementation work starts. It does not authorize product code changes by
itself, does not mark `PM-RENDER-PG-005` complete, and does not assign
`completion_quality`. Implementation requires an owning GitHub issue, focused
validation, pull-request review/evidence, and sequencing through the canonical
renderer/workspace roadmaps where relevant.

## Goal

Harden one shared render product-surface path for current and near-future
producer families:

```text
owning app/domain product decision
  -> producer-owned product-surface manifest
  -> return-only render request batch
  -> explicit ECS publication by the producer
  -> RenderPrepare snapshots targets, uploads, views, invocations, UI bindings, history, diagnostics
  -> Render Execution Graph Compiler preflight
  -> backend runtime executes derived GPU state
  -> UI/debug tooling samples or inspects declared surfaces
```

The path must serve editor viewport products, material previews, texture/debug
previews, field visualizer products, drawing product tiles, and future preview
products without adding a renderer-owned semantic shortcut for any of them.

The renderer remains an execution and presentation layer. Product truth,
selection, freshness, fallback legality, authority, rebuild policy, dependency
truth, source lineage, and residency intent stay with Product Jobs, domains,
apps, and owning producers.

## Current Baseline

The foundation is already in place:

- `engine/src/plugins/render/frame/product_surface.rs` provides return-only
  request batches for dynamic target descriptors, prepared views, and prepared
  flow invocations.
- `apps/runenwerk_editor/src/runtime/viewport/render_jobs.rs` and
  `apps/runenwerk_editor/src/runtime/systems/material_preview.rs` use the
  shared helper path while publishing ECS resources explicitly.
- `engine/src/plugins/render/frame/packet.rs` exposes typed
  `PreparedRenderFrameRequestDiagnostic` values for duplicate prepared views and
  invocations.
- `engine/src/plugins/render/graph/prepared_validation.rs` validates prepared
  alias bindings, dynamic target descriptors, history signatures, uniforms,
  dispatches, and capability compatibility before backend encoding.
- `domain/ui/ui_render_data/src/primitives/product_surface.rs` and
  `domain/ui/ui_render_data/src/primitives/viewport_surface_embed.rs` expose
  dynamic texture UI binding primitives without backend handles.
- Drawing and texture-preview paths already use product-surface UI primitives
  and dynamic target/upload registries, but they are not yet described by the
  same producer manifest and diagnostic vocabulary as flow-backed viewport and
  material preview producers.
- Field visualizer work owns product selection and presentation settings in the
  viewport workflow, but the renderer-facing hardening still needs a shared
  surface contract for unavailable, stale, rejected, fallback, non-sampleable,
  and debug states.

The gap is not a missing renderer policy layer. The gap is that producer
families still describe product surfaces through adjacent but different shapes:
dynamic target requests, texture uploads, prepared views, flow invocations,
viewport embeds, product-surface UI nodes, shell diagnostics, and product
availability states. PM-005 makes those shapes inspectable and contractually
consistent without making the renderer decide product meaning.

## Ownership

DDD bounded context owners:

- `engine/src/plugins/render` owns backend-neutral render product-surface
  request contracts, validation helpers, prepared-frame inspection, renderer
  preflight diagnostics, dynamic target/upload snapshots, and UI binding
  compatibility checks.
- `domain/ui/ui_render_data` owns renderer-neutral UI product-surface and
  viewport-surface primitives.
- `apps/runenwerk_editor` owns editor viewport, material preview, texture/debug
  preview, and field visualizer producer publication.
- `apps/runenwerk_draw` and `domain/drawing` own drawing document, tile,
  preview/final product truth, and drawing product-surface publication.
- Product domains own product descriptors, lineage, freshness, fallback
  legality, rebuild policy, and product diagnostics.

Team Topologies ownership:

- Complicated-subsystem render owner provides the shared contract and
  validation.
- Stream-aligned app/domain teams translate their product decisions into the
  shared product-surface manifest and publish explicit ECS requests.

No new `foundation` or `domain/render_contracts` crate is required for this
milestone. Create one only if a later accepted design needs product-surface
request DTOs outside the engine/app/runtime boundary.

## Vocabulary

PM-005 uses these terms consistently:

- `product-surface producer`: app/domain/runtime code that owns a product
  decision and publishes renderer-facing target, upload, view, invocation, UI,
  history, and diagnostic data.
- `producer manifest`: the producer-owned, backend-neutral description of all
  render product-surface requests it will publish for one frame or product
  generation.
- `surface request batch`: return-only render helper output containing dynamic
  target descriptors, texture uploads, prepared views, prepared flow
  invocations, UI binding intents, history signatures, and producer
  diagnostics.
- `flow-backed surface`: a product surface rendered by a prepared flow
  invocation into a dynamic target.
- `upload-backed surface`: a product surface populated by a texture upload or
  CPU/product payload and then sampled by UI or debug tooling.
- `viewport surface embed`: viewport-local UI primitive that samples a selected
  viewport product slot.
- `product-surface UI primitive`: generic UI primitive that samples a dynamic
  product surface outside viewport slot semantics.
- `surface status`: producer-owned availability/freshness/fallback/rejected
  state carried for diagnostics and UI, not a renderer decision.

## Locked Decisions

PM-005 accepts these decisions:

- Keep helpers return-only. Shared helpers may build manifests and request
  batches, but they must not publish into ECS resources.
- Producer systems must explicitly publish into
  `RenderDynamicTextureTargetRequestRegistryResource`,
  `RenderDynamicTextureUploadRegistryResource`,
  `PreparedRenderFrameRequestResource`, and UI binding registries.
- The shared contract must cover both flow-backed and upload-backed product
  surfaces. Product-surface hardening is not limited to prepared flow
  invocations.
- Dynamic texture target keys remain opaque renderer addresses. Semantic product
  identity remains app/domain-owned.
- UI bindings must reference declared dynamic texture targets or uploads through
  backend-neutral binding sources. They must not carry WGPU handles.
- Non-sampleable, missing, stale, fallback, rejected, or unavailable surfaces
  must be represented as typed producer diagnostics and UI/product status. The
  renderer may validate sampleability and binding legality, but it must not
  decide whether the product is semantically acceptable.
- The compiler/preflight layer from PM-004 remains the execution validator.
  PM-005 adds producer-surface manifest consistency and inspection around it.
- Existing viewport and material preview producers remain valid; PM-005
  hardens the shared contract and migrates additional producer families only
  through explicit tests and bounded write scopes.
- `WR-003` remains contextual support evidence only. It is historical
  decomposition/provenance, not current implementation authority.

## Public Contract Shape

Implementation should extend the existing render product-surface authoring
surface instead of creating a parallel API:

```text
engine/src/plugins/render/frame/product_surface.rs
engine/src/plugins/render/frame/packet.rs
engine/src/plugins/render/resource/dynamic_target.rs
engine/src/plugins/render/resource/dynamic_upload.rs
engine/src/plugins/render/inspect/prepared_frame.rs
engine/src/plugins/render/graph/prepared_validation.rs
domain/ui/ui_render_data/src/primitives/product_surface.rs
domain/ui/ui_render_data/src/primitives/viewport_surface_embed.rs
```

Expected concepts:

- a producer-scoped product-surface manifest or request batch that can carry:
  - dynamic target descriptors;
  - dynamic texture upload descriptors;
  - prepared views;
  - prepared flow invocation requests;
  - viewport-surface binding intents;
  - product-surface UI binding intents;
  - history signatures;
  - producer diagnostics and product-surface status.
- typed diagnostics for manifest consistency, including:
  - missing target descriptor for a flow-backed or UI-sampled surface;
  - missing upload descriptor for an upload-backed surface;
  - duplicate producer surface key;
  - non-sampleable target bound to UI;
  - stale/fallback/rejected/unavailable surface status carried by producer;
  - conflicting history signature for one target key;
  - UI binding without a declared dynamic source.
- inspection DTOs that expose producer id, product family, target key, upload
  key, prepared view id, invocation id, UI binding source, history signature,
  surface status, and diagnostics.

Names may follow local Rust conventions during implementation, but the contract
must stay typed, discoverable from the render public API, and backend-neutral.

## Producer Families

PM-005 should harden current producer families in this priority order:

1. Editor viewport products: scene color, picking ids, overlay, depth/debug
   products, and viewport surface embeds.
2. Material preview products: flow-backed material preview scene surfaces and
   provider product-surface nodes.
3. Texture/debug previews: upload-backed texture and volume preview surfaces
   that already use product-surface UI primitives.
4. Field visualizer products: viewport-owned field/debug presentation that
   carries product availability, selected component/slice/ramp/debug mode, and
   diagnostics without becoming a parallel viewer path.
5. Drawing product surfaces: preview/final ink tile product surfaces that use
   dynamic uploads and product-surface primitives while drawing semantics remain
   in `domain/drawing` and `apps/runenwerk_draw`.
6. Future preview products: new producers must start from the same manifest
   path and may not introduce renderer-private binding shortcuts.

The owning implementation issue may stage these migrations if needed, but
closeout for PM-005 must prove the shared contract across more than the two
PM-002 producers.

## Diagnostics Contract

Diagnostics must be producer-scoped and tool-friendly. Each diagnostic should
include:

- producer id;
- product family or producer class;
- dynamic target key or upload key when relevant;
- prepared view id or invocation id when relevant;
- UI binding source when relevant;
- request kind;
- severity;
- stable diagnostic kind;
- surface status when the producer is reporting stale, fallback, rejected,
  unavailable, or preserved-last-good state;
- human-readable message.

Diagnostics must distinguish:

- renderer execution invalidity, which blocks prepare/preflight/submit;
- producer-reported product status, which informs UI and tooling but remains
  app/domain policy;
- UI sampling invalidity, such as non-sampleable target bound to a product
  surface primitive.

Diagnostics must not centralize product freshness, authority, fallback
legality, rebuild, or residency policy in the renderer.

## UI Binding Contract

UI surface binding is part of PM-005 hardening:

- `ViewportSurfaceBindingSource::DynamicTexture` remains the viewport surface
  embed binding source.
- `ProductSurfaceTextureBindingSource::DynamicTexture` remains the generic
  product-surface primitive binding source.
- Both binding shapes must be traceable to declared producer manifests and
  dynamic target/upload requests.
- UI binding identity should include namespace, target id, and enough producer
  or product-surface identity to avoid cross-product bind group aliasing.
- Non-sampleable targets such as picking ids may be rendered and inspected, but
  normal UI sampling must reject them or expose a producer-owned diagnostic
  state.

PM-005 does not add native multi-window surface ownership. Surface-scoped
swapchains and native presentation remain PM-RENDER-PG-006.

## Relationship To PM-004 Compiler Preflight

PM-005 does not replace PM-004 compiler/preflight validation.

PM-004 validates compiled execution compatibility for prepared frames. PM-005
validates that product-surface producers produce a coherent manifest and that
UI/debug surfaces can be traced back to declared render requests and producer
status.

If both layers can report the same issue, prefer:

- PM-005 producer diagnostics for missing publication, stale/fallback/rejected
  status, or UI binding without a manifest entry;
- PM-004 preflight diagnostics for alias binding, descriptor compatibility,
  sampleability, dispatch/uniform, history, and backend capability errors known
  at compiled execution time.

## Architecture Governance Result

Architecture governance review for this design resolves as:

- DDD owner: `engine/src/plugins/render` for the shared contract, with
  stream-aligned producer ownership in editor, drawing, and product domains.
- Dependency direction: unchanged. Domains/apps publish backend-neutral
  requests and statuses; the renderer consumes and validates them.
- ADR need: no new ADR while PM-005 remains an engine/app contract hardening
  slice and does not introduce a cross-domain ABI, persisted plugin contract, or
  source-of-truth change.
- ATAM-lite tradeoff: one shared manifest adds explicit DTO surface area, but it
  removes hidden divergence between viewport, preview, field/debug, drawing,
  and future producer paths. That tradeoff is accepted.
- Strangler migration: add manifest/diagnostic inspection beside existing
  producers, migrate producer families incrementally, keep existing guards, and
  remove duplicated publication paths only after tests prove equivalence.
- Fitness functions: manifest helper tests, duplicate/conflict diagnostics,
  UI binding traceability tests, non-sampleable UI rejection tests,
  upload-backed product-surface tests, field/debug status tests, drawing product
  surface tests, prepared-frame inspection tests, and workflow validation.

## Implementation Gates

Before code changes:

1. Confirm this accepted design and the canonical renderer/workspace roadmaps
   still support the intended bounded PM-005 slice.
2. Create or select an owning GitHub issue. The issue owns activation, current
   scope, dependencies, and the accepted base.
3. Deliver through a pull request. The PR owns the reviewed feature head,
   complete diff, validation evidence, and acceptance state.
4. Implement one bounded product-surface hardening slice, validate it at one
   unchanged reviewed head, and close it before activating PM-006 work.

Historical `PT-*`, `PM-*`, and `WR-*` labels may remain as decomposition or
provenance vocabulary; they do not grant current implementation authority.

## Validation Required For Implementation

Focused implementation validation must include:

```text
cargo test -p engine render_dynamic_targets
cargo test -p engine render_runtime_inspect
cargo test -p engine render_flow
cargo test -p runenwerk_editor viewport
cargo test -p runenwerk_editor material_preview
cargo test -p runenwerk_editor texture_preview
cargo test -p runenwerk_draw product_surface
```

The owning implementation issue may adjust exact test filters to existing module
names, but it must prove:

- helper-built manifests for flow-backed and upload-backed surfaces;
- diagnostics for duplicate, missing, stale, rejected, fallback, unavailable,
  and non-sampleable surfaces;
- viewport and material preview still use the shared helper path;
- texture/debug previews bind declared dynamic product surfaces;
- field visualizer status remains viewport-owned and does not create a parallel
  renderer viewer path;
- drawing product-surface tiles stay drawing-owned while using the shared render
  product-surface contract;
- prepared-frame and product-surface inspection exposes producer, target,
  upload, view, invocation, UI binding, history, and diagnostics;
- no product truth or product policy moves into renderer helper code.

Repository validation:

```text
cargo validate
git diff --check
CI=true pnpm --dir docs-site build
```

Repository-owned exact-head CI and Documentation Build are the acceptance
evidence for the reviewed revision.

## Non-Goals

PM-005 must not:

- move product truth, freshness, fallback legality, authority, rebuild policy,
  product dependency truth, or residency policy into the renderer;
- implement native multi-window or surface-scoped swapchain presentation from
  PM-RENDER-PG-006;
- implement render fragments, fragment assets, hot reload, or last-good
  fragment promotion from PM-RENDER-PG-007;
- implement production readiness budgets, capture/replay policy, final examples,
  or release inspection from PM-RENDER-PG-008;
- implement SDF brick/page-table, clipmap, raymarch acceleration, mesh/material
  truth, drawing semantics, or product-family source ownership;
- expose WGPU handles or renderer-private backend state to apps, domains, UI,
  or product producers;
- claim every future product family is implemented merely because the shared
  product-surface contract exists.

## Acceptance Bar

This design is accepted when:

- product-surface hardening covers flow-backed and upload-backed surfaces;
- viewport, material preview, texture/debug preview, field/debug, drawing, and
  future producers are mapped to the same contract vocabulary;
- helper APIs remain return-only and producer publication remains explicit;
- diagnostics distinguish renderer execution invalidity from producer-owned
  product status;
- UI bindings are backend-neutral and traceable to declared producer requests;
- implementation is activated by an owning GitHub issue and delivered through
  a reviewed, exact-head-validated pull request.
