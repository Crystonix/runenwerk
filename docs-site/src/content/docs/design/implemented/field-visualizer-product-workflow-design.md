---
title: Field Visualizer Product Workflow Design
description: Implemented design for inspecting scalar, vector, atlas, volume, brickmap, and history field products through viewport products.
status: implemented
owner: apps/runenwerk_editor
layer: app-runtime / editor-ui
canonical: true
last_reviewed: 2026-09-12
related_designs:
  - ./editor-rendered-world-and-multi-entity-viewport-design.md
  - ./render-product-surface-foundation-bundle-design.md
  - ../accepted/sdf-first-field-world-platform-design.md
related_roadmaps:
  - ../../apps/runenwerk-editor/roadmap.md
---

# Field Visualizer Product Workflow Design

## Status

Implemented for the viewport-owned Field Visualizer workflow completed by WR-019. Field-product producer algorithms, renderer-owned field truth, Material Lab, prefab/runtime, terrain, and broader product maturity remain separately owned work; they do not keep this consumer workflow design active.

## Decision

The field visualizer is a viewport product workflow, not a separate viewer.

Owning code paths:

- `domain/editor/editor_viewport/src/expression/field_visualizer.rs`
- `apps/runenwerk_editor/src/runtime/viewport/product_registry.rs`
- `apps/runenwerk_editor/src/runtime/viewport/product_targets.rs`
- `apps/runenwerk_editor/src/runtime/viewport/presentation_resolver.rs`
- `apps/runenwerk_editor/src/shell/providers/scene/viewport.rs`
- `apps/runenwerk_editor/src/shell/providers/field_product_viewer.rs`
- `apps/runenwerk_editor/src/shell/dispatch/viewport.rs`
- `domain/editor/editor_shell/src/composition/build_viewport_panel.rs::viewport_options_popup`
- `domain/editor/editor_shell/src/workspace/persisted.rs`

## Product Kinds

The visualizer must route through existing product descriptors for:

- scalar fields;
- vector fields;
- atlas layers;
- volume slices;
- brickmap debug views;
- temporal/history color products.

Each product exposes availability, producer health, dimensions, format, freshness, and source reality through `editor_viewport::ExpressionProductDescriptor`.

## Controls

V1 controls belong in viewport-owned presentation settings and round-trip
through persisted tool-surface viewport settings:

- `component`: `Auto`, `X`, `Y`, `Z`, `W`, `Magnitude`;
- `slice_index`: `u32`, default `0`;
- `color_ramp`: `Grayscale`, `Heat`, `DivergingSigned`;
- `debug_mode`: `Values`, `Availability`, `Freshness`;
- unavailable-product diagnostics.

Controls must not duplicate product target identity. Product descriptors and
target records keep stable product identity; field visualizer settings
parameterize presentation and producer behavior for the product that the
viewport already presents. The Field Product Viewer surface remains
diagnostics/status-only and does not select viewport products directly.

## Non Goals

- No parallel texture viewer path for fields.
- No new renderer-owned field source.
- No product publication outside the product-job and query-snapshot boundaries.

## Tests

Required coverage:

- product selection maps to the correct `ViewportSurfacePresentationSlot`;
- unavailable products stay visible with diagnostics;
- visualizer presentation settings persist per viewport/tool surface;
- viewport option controls route through provider actions, shell dispatch, and
  runtime presentation state rather than local-only UI state;
- changing component, slice, ramp, or debug mode updates presentation settings
  without changing product target identity;
- field products use the same dynamic target registry as scene color, picking ids, and overlay.
