---
title: Runenwerk Viewport Architecture Design
description: Implemented viewport expression product, presentation, and render-target ownership design.
status: implemented
owner: workspace
layer: workspace
canonical: true
last_reviewed: 2026-09-12
related_roadmaps:
  - ../../apps/runenwerk-editor/viewport-expression-implementation-roadmap.md
---

# Viewport Expression Upgrade Design

## Status
Implemented foundation. Product maturity and additional producers remain separately owned follow-up work and do not keep this foundation design active.

## Purpose
Document the long-term panel-owned viewport presentation architecture built around typed expression products. The fullscreen scene pass with shader-side viewport masking has been removed from the normal render path.

This design is written for the endgame architecture, not as a temporary bridge. Implementation breadth may be phased, but the ownership model, contracts, and architectural boundaries defined here are intended to remain valid long term.

## No-Compromise Update

As of 2026-05-08, the editor viewport foundation uses viewport-owned dynamic targets, prepared render views, prepared flow invocations, dynamic-only UI sampling, persisted viewport runtime settings, viewport-local camera/debug/root commands, and viewport-local render-state snapshots.

The foundation removed these migration-only traits:

- multiple viewport rectangles inside one scene-product uniform;
- static scene color, picking id, and overlay resource ids shared by all viewports;
- viewport render-product synchronization inside frame submission;
- fullscreen shader-side viewport containment as the primary correctness boundary.

Remaining work is product maturity: richer non-viewport editor surfaces, more product producers beyond the current editor viewport, boids, and SDF proofs, and broader history/workflow polish.

The implementation roadmap that executes this design is `docs-site/src/content/docs/apps/runenwerk-editor/viewport-expression-implementation-roadmap.md`.

---

## Problem Summary

The legacy editor viewport path behaved like this:

1. Render a fullscreen scene pass.
2. Attempt to restrict visibility in the shader using a viewport rectangle uniform.
3. Composite UI afterward.

This causes structural issues:

- scene content is produced in screen space instead of being owned by the viewport panel
- ordering is fragile because UI simply overlays the scene pass
- viewport containment depends on coordinate-space agreement between shell layout, shell scaling, surface pixels, and fragment coordinates
- picking and interaction risk inheriting fullscreen-space assumptions
- multiple viewports are awkward
- non-scene outputs such as field textures, atlas previews, and brickmap views do not fit naturally
- the viewport is tightly coupled to one rendering mode instead of being a general presentation consumer

This is not the correct long-term architecture and should not be evolved further as a foundation.

---

## Design Intent

This upgrade must establish the architecture that Runenwerk can carry forward without repeated conceptual refactors.

The implementation may start small, but the architecture must already assume:

- multiple viewport panels
- multiple expression products
- non-scene products
- panel-owned presentation surfaces
- viewport-local interaction
- explicit separation between producer, expression product, and viewport presentation
- comparative, layered, and debug-oriented viewing behavior
- eventual retained expression product invalidation/caching without changing ownership

The first implementation is allowed to exercise only a small subset of this architecture, but it must not violate it.

---

## Core Architectural Rules

These rules are mandatory from the first implementation.

### 1. The viewport is not a scene renderer
The viewport is a **presentation consumer**.

It does not own world rendering policy and it does not define product semantics. It presents typed expression products produced elsewhere.

### 2. The viewport does not own “the scene texture”
Each viewport owns a **viewport surface set** and **presentation state**, not a one-off hardcoded scene-color surface.

### 3. Producers do not render “to the panel”
Producers publish **typed expression products**.
Viewport presentation resolves those products into panel-owned surfaces for display.

### 4. UI does not clip fullscreen rendering
The shell embeds viewport-owned presentation surfaces.
Containment is achieved by ownership and composition, not by fullscreen rendering followed by shader masking.

### 5. Interaction is viewport-local
Picking and interaction must use viewport-local coordinates and viewport-owned picking products or equivalent viewport-local picking expressions.

### 6. Producer / product / presentation are separate
This separation is not optional and must exist from the first implementation.

- producer creates product
- product is identified and described
- viewport presentation selects and displays product

### 7. Phase 1 may be narrow in breadth, not in architecture
It is acceptable if phase 1 only has one viewport and one main scene-color-like product.
It is not acceptable if phase 1 hardcodes an architecture equivalent to “viewport == scene color renderer output.”

### 8. The viewport must be a serious tooling surface
The architecture must support, by design, not by later exception:
- comparative viewing
- layered viewing
- debug and diagnostic products
- atlas/field/volume/brick products
- future product provenance visibility

---

## Architectural Positioning in the Nine Layers

### Layer 1 — Runtime Simulation
Owns live world and simulation state.
Examples:
- ECS scene state
- water simulation state
- cellular automata state
- wind field state
- influence field state

Viewport relevance:
- producers derive products from runtime/simulation realities
- viewport presentation consumes outputs from these domains but does not own them

### Layer 2 — Mutation / Ratification
Owns accepted changes to viewport configuration, panel configuration, and authored graph-backed display settings.
Examples:
- selected displayed product
- viewport presentation mode
- overlay configuration
- viewport camera mode
- slice/channel selection

Viewport relevance:
- presentation state changes should be explicit editor/tooling mutations
- the viewport should not change products implicitly through hidden renderer state

### Layer 3 — Retention / Recovery
Owns reconstructability, viewport configuration history, and future expression cache lineage.
Examples:
- viewport layout history
- selected output history
- retained expression metadata
- future cache invalidation lineage

Viewport relevance:
- the design must allow viewport configuration and future retained expression metadata without redefining ownership later

### Layer 4 — Observation
Owns observed editor-facing forms.
Examples:
- available viewport products
- selected product metadata
- producer health
- invalidation state
- diagnostics and artifact browser state

Viewport relevance:
- viewport UI and tooling consume observation-oriented state, not renderer-private internals

### Layer 5 — Authority / Partition
Owns which domains may produce, expose, or consume specific products.
Examples:
- local-only debug outputs
- partition-local field products
- shared preview outputs

Viewport relevance:
- viewport presentation is a consumer, not an authority owner
- the architecture must preserve who owns and may expose products

### Layer 6 — Asset / Content
Owns authored graphs and content definitions.
Examples:
- scene document
- material graph
- field graph
- simulation graph
- atlas metadata
- brickmap authored configuration

Viewport relevance:
- the viewport architecture must be able to consume products originating from content/graph-backed systems later without redesign
- viewport should not depend directly on material graph as its general abstraction

### Layer 7 — Expression
Owns typed consumer-facing products derived from authored, formed, or simulated realities.
This is the central layer for the viewport upgrade.
Examples:
- scene color expression
- picking expression
- overlay expression
- scalar field image
- vector field visualization
- atlas preview expression
- brickmap slice/projection expression
- diagnostics and provenance-facing products

Viewport relevance:
- viewport presentation is fundamentally a consumer of expression products
- offscreen viewport surfaces and product display belong structurally here and in Layer 9, not in fullscreen shell hacks

### Layer 8 — Sharing / Replication
Owns shared or remote display products where needed.
Examples:
- collaborative viewport preview
- streamed diagnostics view

Viewport relevance:
- not required in initial breadth, but the architecture must not block future shared products

### Layer 9 — Editor / Tooling
Owns panel behavior, interactions, and tooling-side presentation.
Examples:
- viewport panel
- split view
- overlay toggles
- artifact browser
- channel/slice/mip controls
- annotations and probes

Viewport relevance:
- panel owns presentation state
- panel embeds the viewport-owned presentation surface
- panel interaction is viewport-local

---

## Scope Boundary

### Architecturally mandatory now
These are required in the first implementation because they are part of the architecture, not optional refinements.

- offscreen per-viewport render target ownership
- panel-owned presentation instead of fullscreen masking
- viewport-local picking / id surface or equivalent viewport-local picking product
- viewport presentation state
- support for switching displayed output through presentation state
- typed product handle/model
- explicit separation between producer, expression product, and viewport presentation
- support for at least one future-proof presentation mode abstraction beyond a hardcoded scene color path

### Breadth that may remain small initially
These may be exercised with a narrow implementation at first, but the architecture must already allow them naturally.

- multiple viewport outputs
- multiple viewport panels
- non-scene products
- basic product registry / descriptor model
- a small baseline of comparative/overlay/view modes

### Not required to be broad in the first implementation
These should not block the immediate upgrade as long as the architecture remains compatible with them.

- broad set of rendering pipelines
- full deferred / forward+ / raymarch orchestration redesign
- full material graph integration
- full compute graph integration
- full retained expression graph caching strategy
- all specialized atlas / field / brickmap viewers
- rich comparative mode catalog

---

## Core Design Decision

The viewport becomes a **panel-owned presentation surface architecture for typed expression products**.

Not:
- fullscreen scene pass with shader-side viewport masking
- one-off scene-color texture architecture
- direct dependency on material graph
- direct dependency on renderer-private global surfaces

Instead:
- each viewport panel has a stable viewport identity
- each viewport panel owns a viewport surface set
- producers publish typed expression products
- viewport presentation state selects which product(s) are displayed
- the shell embeds the resulting viewport-owned presentation surface inside panel bounds

This is the architecture now. Future work extends breadth, not ownership.

---

## High-Level Model

### 1. Producers
Producers generate typed expression products.

Examples:
- scene render producer
- picking producer
- overlay producer
- atlas preview producer
- field visualization producer
- brickmap visualization producer
- volume slice producer
- diagnostics/provenance producer

A producer must not be modeled as “rendering to the panel.”
It creates products.

### 2. Expression Products
Expression products are the consumer-facing outputs created by producers.

Each product has:
- stable id
- semantic kind
- descriptor
- dimensions and format
- producer identity
- lifetime / retention metadata
- presentation hints
- optional channel/layer/slice metadata

Even if only one product exists initially, it must still be modeled as a product, not a special viewport exception.

### 3. Product Registry / Product Source View
A product lookup/registry concept must exist from the first implementation, even if minimal.

This may begin as a small local registry/source, but it must already support:
- lookup by product id
- descriptor access
- selected product resolution for a viewport
- future exposure of product provenance and status

### 4. Viewport Surface Set
Each viewport owns a set of offscreen surfaces used for display and interaction.

Initial required surfaces:
- color surface
- picking/id surface

Near-term surfaces:
- depth surface
- overlay/composition surface

Future optional surfaces:
- normals
- motion vectors
- simulation debug surfaces
- temporary composition surfaces
- ROI/progressive evaluation scratch targets

The important architectural point is that this is already a **surface set**, not a one-off scene color texture.

### 5. Viewport Presentation State
Each viewport owns configuration that describes what it is presenting and how.

Examples:
- selected primary product id
- selected overlay product ids
- presentation mode
- channel mode
- slice mode
- mip level
- comparison mode
- zoom/pan state
- ROI state
- probe/annotation visibility

This is mandatory from the first implementation.

### 6. Viewport Panel Composition
The shell/editor panel embeds the viewport presentation surface as a panel-local UI primitive.

The viewport is therefore contained by ownership and composition, not by fullscreen shader clipping.

---

## Key Contracts

These contracts should be introduced from the first implementation.

### ViewportId
Stable identifier for a viewport instance.

`ViewportId` is specialized viewport/tool identity. It must coexist with broader workspace hosting identities such as panel hosts, panel instances, tab stacks, and generic tool-surface instances without being conflated with them.

### ExpressionProductId
Stable identifier for a presentable expression product.

### ExpressionProductKind
Describes the semantic kind of output.

Initial kinds should already allow future breadth, even if not all are implemented immediately.
Examples:
- SceneColor2D
- Depth2D
- PickingIds2D
- Overlay2D
- ScalarField2D
- VectorField2D
- Atlas2D
- VolumeSlice2D
- VolumeProjection2D
- BrickmapDebug2D
- Diagnostics2D

### ExpressionProductDescriptor
Metadata describing the output.

Minimum required fields:
- id
- kind
- dimensions
- format
- producer label
- source reality class
- freshness/version marker where available
- presentation hints
- optional channel/layer/slice metadata

### ViewportSurfaceSet
Per-viewport-owned surface bundle.

Minimum required responsibility:
- own the panel-local surfaces used for presentation and interaction
- resize with viewport changes
- remain distinct from fullscreen render targets

### ViewportPresentationState
Per-viewport presentation configuration.

Minimum required responsibility:
- select displayed product id
- control panel-local display mode
- remain owned by viewport/tooling side rather than hidden renderer state

### ArtifactObservationFrame
Observed model exposed to tooling/UI.

This should be understood as a viewport-facing observation frame over available expression products and their status, not as direct exposure of renderer-private authority state.

Minimum useful fields:
- available products
- selected product(s)
- availability state
- producer status
- dimensions/format summary
- freshness / stale state where known

---

## Viewport Capability Baseline

The design should treat these as baseline viewport capabilities for a serious tool, even if implementation breadth ramps up over time.

- multiple independent viewports
- per-viewport toolbar/actions
- viewport-local zoom/pan/framing
- viewport-local picking
- saved viewport layouts/presets
- selected product switching
- per-viewport diagnostics visibility
- stable redraw behavior under resize and hidden/visible transitions

---

## Comparative and Overlay Viewing

The architecture must support comparative and layered viewing, not just one-product display.

Required fit:
- A/B compare
- wipe/slider compare
- difference mode
- overlay stack
- split view
- quad/comparative view
- linked multi-view comparisons

These may arrive incrementally, but the presentation state and surface model must allow them naturally.

---

## Layered Presentation Model

Viewport presentation should be able to combine several display layers.

Examples:
- base image/product layer
- labels/id layer
- points/gizmo/annotation layer
- vector overlay layer
- measurement/probe layer
- selection/highlight overlay

This is especially important for non-scene and debug-oriented workflows.

---

## ROI and Progressive Evaluation

The viewport architecture must allow restricted and progressive evaluation for expensive products.

Required fit:
- region of interest rendering/evaluation
- progressive refinement for expensive products
- paused/frozen viewport mode
- future playback/preview caching hooks
- ability to update only selected layers/products where valid

These do not all need broad first implementation, but the product and surface design must not prevent them.

---

## Product Provenance and Pipeline Visibility

The viewport should eventually expose enough information for advanced debugging and tooling.

Required fit:
- which producer created the product
- upstream dependencies
- freshness/stale state
- invalidation reason
- last build time or cost class where available

The design should therefore preserve product identity and descriptors rather than hiding them behind ad hoc renderer state.

---

## Overview / Navigator View

The architecture should support overview/navigator-style viewports.

Examples:
- optional overview/minimap viewport
- current viewport extent indicator
- linked navigation between overview and active viewport
- large-world and streaming-friendly overview modes

This is especially useful for large worlds and non-scene artifact viewing.

---

## Rendering Control and Throttling Policy

The design should explicitly allow per-viewport rendering control.

Required fit:
- auto-render vs manual refresh modes
- hidden/background viewport throttling
- redraw suppression during bulk changes where appropriate
- per-viewport refresh policy
- paused viewport mode

This matters for expensive producers and multiple viewport panels.

---

## Channel / Layer / Slice / Component Inspection

The architecture must support inspection modes beyond a single final image.

Required fit:
- channel selection
- layer selection
- slice selection
- mip selection
- scalar/vector component selection
- label/id inspection mode

This is critical for fields, atlases, volumes, and other non-scene products.

---

## Annotations and Measurement Overlays

The architecture should support panel-local annotations and review tooling.

Required fit:
- viewport annotations
- markers/bookmarks
- measurement overlays
- pixel/value probes
- temporary review markup

These should be modeled as layered presentation/observation concerns, not as renderer-specific hacks.

---

## Display Product Taxonomy

The design should maintain a coherent product taxonomy.

Suggested groups:
- scene products
- picking/selection products
- scalar field products
- vector field products
- atlas/packing products
- volume/brick/sparse products
- annotation/overlay products
- diagnostic/provenance products

This helps future additions remain consistent.

---

## Relationship to Material Graphs, Shader Graphs, and Simulation Graphs

The viewport system should not depend directly on the material graph.

Correct relationship:
- material graph is authored content
- simulation/field graphs are authored content where applicable
- shader and compute lowering products are formed execution artifacts
- producers create expression products from those systems
- viewport presentation consumes expression products

This allows the viewport architecture to remain stable while supporting:
- scene color
- material preview
- scalar fields
- vector fields
- atlas previews
- brickmap views
- volume slices

---

## Current Approach vs Target Approach

### Current
- fullscreen scene pass
- shader uniform contains viewport rectangle
- shader discards outside viewport
- shell UI composites afterward

### Target
- per-viewport offscreen surface set
- producers create typed products
- viewport presentation resolves chosen product(s) into viewport-owned surfaces
- shell embeds viewport-owned surface inside panel bounds
- interaction uses viewport-local coordinates and viewport-owned picking product

---

## Final Runtime Shape

The final runtime flow is:

```text
Workspace tool surface
  -> explicit ViewportInstanceRecord
  -> projected viewport layout entry
  -> per-viewport render state
  -> per-viewport render job
  -> per-viewport product targets
  -> presentation surface binding
  -> UI ViewportSurfaceEmbed
```

Each step has one owner.

- Viewport instance ownership lives in the editor app runtime viewport subsystem.
- Engine-agnostic product semantics live in `domain/editor/editor_viewport`.
- Concrete target allocation and render execution live in the app/engine runtime boundary.
- Shell composition embeds a resolved viewport surface; it does not own product rendering.

No step may infer correctness from "there is only one viewport" once shell projection artifacts exist.

### Required final modules

- `apps/runenwerk_editor/src/runtime/viewport/instance_registry.rs`
  - explicit `ViewportId` allocation, restore, duplication, close, and tool-surface mapping.
- `apps/runenwerk_editor/src/runtime/viewport/render_state.rs`
  - per-viewport bounds, dimensions, camera, presentation/debug state, render freshness, and target status.
- `apps/runenwerk_editor/src/runtime/viewport/render_jobs.rs`
  - one render job per visible viewport.
- `apps/runenwerk_editor/src/runtime/viewport/producer_scene.rs`
  - scene color, picking ids, overlay, and later depth/debug products rendered per viewport job.
- `apps/runenwerk_editor/src/runtime/viewport/surface_set.rs`
  - per-viewport concrete surface handles, not shared static labels.
- `apps/runenwerk_editor/src/runtime/viewport/presentation_resolver.rs`
  - selected products resolved to viewport-owned surfaces without fallback to another viewport.

### Final rejection tests

The final system should have architecture guards that fail if:

- `EditorViewportSceneProductUniform` contains `viewport_b`, `viewport_c`, `viewport_d`, or reserved multi-rectangle fields;
- normal runtime flow uses static `editor.viewport.v1.scene_color` as the shared target for multiple viewports;
- `submit_editor_frame_system` allocates or seeds viewport identity;
- a provider uses a lone observed viewport as the runtime identity for an unbound split/replacement surface;
- picking reads global screen-space state instead of viewport-local state.

---

## Rendering and Ordering Model

### Current ordering problem
The scene pass is effectively global, and the shell UI overlays it.
This is why content can appear behind panels and only peek through gaps.

### New ordering model
For each viewport:
1. Determine panel bounds.
2. Allocate or resize the viewport surface set.
3. Resolve selected expression product(s) into viewport-owned surface(s).
4. Embed the resolved viewport surface in the viewport panel.
5. Composite shell/UI around the already-contained viewport content.

### Consequence
No fullscreen masking is needed as the primary containment mechanism.
Scissor or shader clipping may still exist inside specific producers, but they are not the architectural boundary.

---

## Multi-Viewport and Multi-Product Assumptions

The architecture must assume multiple viewports and multiple products from the first implementation.

Even if only one viewport is initially created and only one scene-color-like product is initially exercised, the design must already support:
- more than one viewport id
- more than one product id
- more than one presentation mode
- more than one product category

This is what prevents immediate redesign pressure.

---

## Picking and Interaction

Picking must become viewport-local.

### Required model
- viewport-local coordinates
- viewport-owned picking/id surface or equivalent picking expression
- interaction resolution against the selected viewport instance rather than fullscreen screen coordinates

This is architecturally mandatory now, even if the first implementation keeps the picking path simple.

---

## Implementation Strategy

This strategy phases breadth, not architecture.

### Step 1 — Establish the real viewport ownership model
Mandatory outcome:
- viewport id exists
- viewport surface set exists
- viewport presentation state exists
- typed product handle/model exists
- shell embeds viewport-owned surface in the panel
- fullscreen viewport masking path is no longer the architectural center

This step may initially exercise only one main product.

### Step 2 — Make interaction viewport-local
Mandatory outcome:
- viewport-local picking/id product exists or equivalent picking path exists
- fullscreen-space interaction assumptions are removed

### Step 3 — Formalize product lookup / registry behavior
Mandatory outcome:
- products are not hardcoded as special renderer outputs
- viewport presentation selects product by id
- basic descriptor access exists
- basic product observation state exists

### Step 4 — Expand breadth
Examples:
- multiple viewport panels
- multiple products
- alternate debug products
- overlays and split view
- non-scene products
- basic provenance visibility

This expands the architecture already in place rather than changing it.

---

## What This Design Intentionally Rejects

- fullscreen scene pass as the long-term viewport architecture
- shader-side masking as the primary containment mechanism
- one-off “scene color viewport” architecture
- direct coupling of viewport to material graph as the general abstraction
- direct coupling of viewport to renderer-private global surfaces
- postponing producer/product/presentation separation until later
- postponing viewport-local interaction until later
- treating comparative/layered/debug display as an afterthought rather than a first-class fit

---

## Minimum Abstraction Rule

The first implementation must already enforce this separation:

- producer creates product
- product is identified and described
- viewport presentation chooses what to display
- panel embeds the chosen viewport-owned presentation surface

If any first implementation bypasses this and effectively hardcodes “viewport == scene color renderer output,” it should be considered architecturally incorrect even if it appears to solve the immediate bug.

---

## Expected Outcome

After the first correct implementation:
- viewport content renders inside the viewport panel correctly
- the architecture no longer depends on fullscreen masking
- picking becomes viewport-local
- the viewport is already structurally prepared for multiple products and multiple panels
- future scene, debug, atlas, field, and brickmap outputs can fit without redesigning viewport ownership
- later comparative, layered, and diagnostics-rich viewing can expand without ownership refactors

---

## Short Version

The long-term Runenwerk viewport must be:

**a panel-owned presentation architecture for typed expression products, with viewport-local interaction, explicit producer/product/presentation separation, comparative and layered viewing support, and nine-layer alignment from the first implementation.**
