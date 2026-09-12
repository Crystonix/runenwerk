---
title: Fully Featured Renderer Roadmap
description: Canonical long-term roadmap for the engine renderer and editor viewport/product integration path.
status: active
owner: engine
layer: engine-runtime / editor-product-integration
canonical: true
last_reviewed: 2026-09-12
related_designs:
  - ../../design/accepted/sdf-first-field-world-platform-design.md
  - ../../design/accepted/sdf-product-renderer-and-gpu-residency-design.md
  - ../../design/implemented/render-product-surface-foundation-bundle-design.md
  - ../../design/implemented/editor-rendered-world-and-multi-entity-viewport-design.md
  - ../../design/implemented/field-visualizer-product-workflow-design.md
  - ../../design/active/material-lab-and-material-preview-design.md
  - ../../design/accepted/render-contract-ergonomics-design.md
  - ../../design/accepted/feature-owned-render-contributions-design.md
  - ../../design/accepted/render-execution-graph-compiler-maturity-design.md
  - ../../design/accepted/render-fragment-data-driven-maturity-design.md
  - ../../design/accepted/render-production-readiness-and-inspection-design.md
  - ../../design/accepted/render-product-graph-platform-design.md
related_roadmaps:
  - ./render-final-architecture-migration.md
  - ../plugins/render/docs/roadmap.md
  - ../../apps/runenwerk-editor/viewport-expression-implementation-roadmap.md
  - ../../workspace/roadmap-items.yaml
related_reports:
  - ../../reports/closeouts/wr-018-rendered-world-v1/closeout.md
  - ../../reports/closeouts/wr-020-source-backed-asset-core-contracts/closeout.md
---

# Fully Featured Renderer Roadmap

## Status

Active canonical landing page for the long-term engine renderer plus editor
viewport/product integration path.

This document does not create one giant renderer implementation ticket. The
workspace roadmap keeps existing WR rows as implementation units. This roadmap
connects those rows to the larger renderer capability program so future work is
sequenced instead of rediscovered.

## North Star

Runenwerk's renderer should become a compiled, multi-view, product-surface,
SDF-first, material-capable, inspectable, and data-driven renderer.

The renderer must support:

- editor scene, picking, overlay, field, material, debug, and preview product
  surfaces;
- sparse SDF/world rendering through prepared product selection and derived GPU
  residency;
- material, mesh, lighting, postprocess, temporal, particle, VFX, atmosphere,
  water, and animation producers as product consumers;
- authored render fragments and hot reload without bypassing `RenderFlow`
  validation or compiled execution;
- production inspection, diagnostics, performance budgets, and last-good
  fallback behavior.

## Non-Negotiables

- Domain and app layers own world, asset, material, prefab, field, editor, and
  gameplay truth.
- The renderer consumes prepared products, prepared render views, target alias
  bindings, feature contributions, and GPU residency requests.
- Renderer-owned GPU buffers, textures, pipelines, bind groups, page tables,
  atlases, history resources, and caches are derived state only.
- Runtime submission must not perform live ECS extraction to discover scene,
  product, target, uniform, or fallback data.
- Product surfaces are the integration path for editor viewport products,
  material previews, field visualizers, debug outputs, and future product
  previews.
- Missing, stale, ghost, fallback, over-budget, and failed-preserved product
  states must be visible to diagnostics instead of silently promoted.

## Current Baseline

- `WR-018` completed the editor rendered-world V1 packet: authored SDF
  primitive entities render and pick through one app-owned viewport scene
  packet.
- `WR-020` completed domain-owned source/catalog/import/asset identity
  contracts needed by future material, field, prefab, and render-fragment
  workflows.
- The render product surface foundation is implemented: dynamic product
  targets, target aliases, prepared views/invocations, history signatures, UI
  sampling, and inspection now exist as engine capabilities.
- The bounded render fragment foundation is implemented: fragment packages,
  typed fragment ids/namespaces, validation, merge provenance, last-good reload
  state, and fragment inspection merge into normal `RenderFlow` compilation.
- The bounded production readiness inspection surface is implemented: readiness
  reports aggregate existing prepared-frame, product-surface, graph/preflight,
  fragment, capture, timing, and budget DTOs; replay manifests fail closed
  before evidence is treated as valid.
- The final render architecture migration defines the prepared/compiled/executed
  cut: ECS prepares packets, graph compilation shapes execution, and renderer
  runtime owns backend artifacts.
- The viewport expression roadmap defines the editor-side product routing path:
  viewport instances, viewport-local render jobs, product target registry,
  surface bindings, and UI embeds.

## Milestone Ladder

### FR-0 - Documentation Reconciliation

Make this roadmap discoverable from workspace, engine, render plugin, viewport,
and rendered-world docs. Keep roadmap rows and generated decision tables aligned.

Exit criteria: a reader can find the renderer plan from workspace roadmap,
engine docs, render plugin docs, and editor rendered-world docs.

### FR-1 - Render Contract Hardening And API Ergonomics

Continue renderer contract work through bounded WR rows: prepared product
selection, derived residency, target aliases, history signatures, diagnostics,
and public API ergonomics. Keep render as a contract-following consumer.

Exit criteria: product selection, residency, target, history, and flow authoring
contracts are easy to use and guarded by focused tests.

### FR-2 - Editor Scene And Viewport Producer Expansion

Extend the WR-018 viewport packet path without changing ownership. Add
storage-buffer scene packets, depth outputs, richer product outputs, and
producer health only when a selected field/material/prefab workflow needs them.

Exit criteria: editor scene, picking, overlay, depth, field, material preview,
and debug outputs route through viewport product surfaces with no parallel
viewer path.

### FR-3 - SDF And World Renderer

Build the serious SDF/world renderer on prepared product selection and derived
GPU residency. Support sparse SDF bricks, page tables, clipmaps, analytic SDF
instances, cluster fields, aggregate fields, generation-aware invalidation, and
diagnostics.

Exit criteria: SDF/world rendering consumes product lineage and residency
requests, renders through product surfaces, and exposes fallback, stale,
over-budget, memory pressure, and unsafe-overstep diagnostics.

### FR-4 - Mesh, Material, And Lighting Path

Connect material graph preview products and future mesh/material contributions
to renderer material handoff without moving material truth into renderer code.
Support material preview targets, lighting inputs, debug views, and pipeline
specialization only as prepared renderer data.

Exit criteria: Material Lab products can preview through viewport/product
targets, and mesh/material paths share renderer contracts with SDF and debug
products.

### FR-5 - Temporal, Postprocess, And History Workflows

Harden history resources for temporal AA, postprocess chains, accumulated debug
views, persistent render caches, and history-aware product previews.

Exit criteria: history targets are view/invocation-scoped, invalidated by
signature changes, inspectable, and protected from stale cross-view reuse.

### FR-6 - Product Producers For VFX, Animation, Atmosphere, And Water

Add particles, VFX, animation/deformation, atmosphere, water, vegetation, and
related world-process rendering only after owning product contracts exist.

Exit criteria: each producer emits prepared render contributions and residency
requests while its domain keeps semantic truth and mutation policy.

### FR-7 - Render Fragments, Hot Reload, And Authoring

`WR-010` implements the bounded render-fragment design: typed fragment ids and
namespaces, validation, merge into `RenderFlow`, registry reload state,
last-good fallback, inspection, and a fragment compositor example. Later
asset/editor catalog integration remains outside this renderer slice.

Exit criteria: fragment-driven flows run through normal compiled execution;
invalid fragments cannot affect active rendering; provenance and reload
diagnostics are inspectable.

### FR-8 - Production Readiness

`PM-RENDER-PG-008` is gated by the accepted render production readiness and
inspection design. `WR-045` implements the bounded readiness-report, budget,
fail-closed replay-manifest, public API documentation, and example proof slice
without moving product truth or product policy into renderer code.

Exit criteria: complex renderer behavior can be understood from inspection
tools, examples, roadmap docs, and tests without reading internal backend code
first.

## Roadmap Mapping

- `WR-003` remains support-only context for FR-1 and shared renderer enabling
  work; bounded production-track implementation rows carry PM-RENDER-PG slices.
- `WR-041` implements the bounded `PM-RENDER-PG-002` ergonomics slice for
  return-only render product-surface request helpers, typed prepared-frame
  request diagnostics, and editor viewport plus material preview producer
  migration.
- `PM-RENDER-PG-003` is gated by the accepted feature-owned render
  contributions design. Its implementation must add the typed collector
  registry beside the current central path and keep render fragments, hot
  reload, and compiler maturity out of the slice.
- `PM-RENDER-PG-004` is gated by the accepted render execution graph compiler
  maturity design. Its implementation must mature static flow validation and
  prepared-frame execution preflight for resources, pass order, aliases,
  history, lifetimes, and backend capabilities without implementing fragment
  assets or hot reload.
- `PM-RENDER-PG-005` is gated by the accepted product-surface platform
  hardening design. Its implementation must make flow-backed and upload-backed
  product-surface producers share one manifest, diagnostics, UI binding,
  dynamic target, prepared view, invocation, history, and inspection contract
  without moving product truth or product policy into the renderer.
- `WR-019` implements viewport-owned Field Visualizer product routing and
  presentation settings as part of FR-2.
- `WR-026` implements editor asset adapters that feed source-backed renderer
  consumers; it does not move asset truth into renderer code.
- `WR-021` implements Material Lab and material preview products as part of
  FR-4.
- `WR-022` implements prefab runtime/product integration only after asset and
  renderer handoff contracts are ready.
- `WR-010` implements FR-7 render fragments and data-driven maturity.
- `WR-014` and later product-family rows can feed FR-6 only after their owning
  domain product contracts are accepted.

## Production Track Mapping

`PT-RENDER-PG` is the production planning layer for this roadmap. It guides
sequencing but does not replace the WR roadmap as the legal execution graph.

- `FR-0` maps to `PM-RENDER-PG-001` for doctrine, docs reconciliation, and
  production-track alignment.
- `FR-1` maps to `PM-RENDER-PG-002` for render contract hardening and API
  ergonomics.
- `FR-2` maps to `PM-RENDER-PG-005` for shared product-surface hardening.
- `FR-3` remains deferred to product-family designs and future renderer WR
  rows; `PT-RENDER-PG` defines the handoff boundary but does not implement SDF
  brick/page-table, clipmap, or raymarch acceleration work by itself.
- `FR-4` maps to `PM-RENDER-PG-005` plus material/product roadmap rows; material
  truth stays outside the renderer.
- `FR-5` maps to `PM-RENDER-PG-004` and `PM-RENDER-PG-005` for history,
  lifetime, and product-surface validation.
- `FR-6` remains deferred to product-family designs plus
  `PM-RENDER-PG-005`; atmosphere, water, VFX, animation, and vegetation must
  first have owning product contracts.
- `FR-7` maps to `PM-RENDER-PG-007` for render fragments, hot reload,
  validation, provenance, and last-good behavior.
- `FR-8` maps to `PM-RENDER-PG-008` for production diagnostics, inspection,
  examples, budgets, and closeout evidence.

## Validation Expectations

Renderer roadmap work should keep these checks current as relevant:

```text
cargo test -p engine
cargo test -p runenwerk_editor viewport
task docs:validate
task roadmap:render
task roadmap:validate
task roadmap:check
task puml:validate
task links:check
```

No milestone is complete if it creates a renderer-owned shortcut around product
selection, source/catalog truth, app-owned viewport workflow, or domain-owned
world/material/prefab policy.
