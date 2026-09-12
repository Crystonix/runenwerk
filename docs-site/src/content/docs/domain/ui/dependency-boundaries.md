---
title: UI Dependency Boundaries
description: Dependency direction and enforcement policy for Runenwerk UI crates.
status: active
owner: ui
layer: domain
canonical: true
last_reviewed: 2026-09-12
---

# UI Dependency Boundaries

This document defines how UI crates may depend on each other.

It exists to prevent accidental architecture drift while the UI system evolves from retained/editor-specific proof into a long-term domain program family.

## Boundary model

UI crates are grouped by responsibility:

```text
foundation
definition
definition_adapter
program
render
proof
composition
surface
retained
testing
```

Each crate has:

```text
layer
owned responsibility
allowed dependencies
temporary exceptions
forbidden dependencies
```

The machine-readable map is:

```text
domain/ui/ui-crate-ownership.toml
```

The checker is:

```text
tools/checks/check_ui_layer_dependencies.py
```

## Enforcement principle

The checker is intentionally conservative.

It should catch obvious forbidden crossings, not replace design review. A green check means "no known forbidden dependency crossing was detected." It does not mean the architecture is complete.

## Hard forbidden dependencies

These should not be allowed without an accepted design change:

```text
domain/ui/* -> apps/*
domain/ui/* -> editor app crate internals
ui_story -> runenwerk_editor
ui_story -> editor_* crates
ui_program -> ui_definition
ui_program -> ui_story
ui_compiler -> ui_story
ui_artifacts -> ui_story
ui_runtime_view -> ui_story
ui_composition -> any other production UI crate
ui_adaptive_composition -> app/editor/engine/ui_surface crates
ui_surface -> game/editor/world concrete semantic crates
```

## Transitional dependencies

Some current dependencies may exist for migration reasons, especially around retained UI and first-button proof.

A transitional exception must include:

```text
why it exists
owner
target removal phase
replacement owner
validation that prevents it from growing
```

No exception is permanent by default.

## Domain source-of-truth boundaries

### Authored UI definition

Owned by:

```text
ui_definition
```

May not own long-term retained runtime or render proof.

### UI program

Owned by:

```text
ui_program
```

May not load authored assets directly. Lowering from definitions belongs to:

```text
ui_program_lowering
```

### Compiler and artifact

Owned by:

```text
ui_compiler
ui_artifacts
```

They may produce diagnostics and artifact manifests, but they must not depend on story/gallery/app orchestration.

### Runtime view

Owned by:

```text
ui_runtime_view
```

It should consume runtime artifacts and expose read models/facts. Control-specific reports should move to control-owned adapters once the component platform expands.

### Render proof

Owned by:

```text
ui_render_primitives
ui_headless_render_data
ui_static_mount
```

These crates produce renderer-neutral proof facts. They do not own editor/game host semantics.

### Story proof

Owned by:

```text
ui_story
```

It aggregates proof evidence and determines verdict/mount eligibility. It does not own the semantic meaning of compiler/runtime/render stages.

### Structural composition

Owned by:

```text
ui_composition
```

`ui_composition` is a leaf app-neutral domain authority. It depends only on
approved foundation primitives and serialization support; it may not depend on
`ui_program`, retained runtime crates, `ui_surface`, editor/Draw/game crates,
apps, engine, adapters, windowing, or provider/session owners.

`ui_adaptive_composition` may depend only on `ui_composition`, `ui_input`, and
`ui_math`. It derives transient state from immutable snapshots and emits typed
proposals. It may not hold mutable composition state, execute transactions,
persist canonical definitions, redefine structural identity, or depend on
editor/Draw/game/app/engine/windowing/provider/session owners.

`ui_testing` may depend on `ui_composition` and
`ui_adaptive_composition` to execute headless structural and adaptive fixtures.
Browser, terminal, dashboard, mobile, and game fixture names are profiles only
and must not bring product imports or behavior into any of these crates.

The enforced direction is:

```text
ui_composition + ui_input + ui_math
  <- ui_adaptive_composition
  <- ui_testing
```

Only an app/host owner may inspect a proposal, apply policy, form a typed
`ui_composition` transaction, and submit it against the current revision.

### App orchestration

Owned by:

```text
apps/runenwerk_editor
future game/editor/world-space tracks
```

Apps may wire many domain crates together. Domain crates must not depend upward on apps.

## Required validation commands

Run after any dependency or ownership-map change:

```sh
python3 tools/checks/check_ui_layer_dependencies.py --root .
cargo check --workspace
cargo test -p ui_story
cargo test -p runenwerk_editor story
cargo validate
git diff --check
```

<!-- BEGIN RUNENWERK:UI_COMPONENT_PLATFORM:domain-ui-note -->
## UI Component Platform activation note

The active Component Platform roadmap is `PT-UI-COMPONENT-PLATFORM`: reusable, story-proven `ControlPackage` and surface maturity after `PM-UI-STORY-004`. The platform introduces reusable kernels for control packages, authoring, story proof, catalog/discovery, input/gesture/device, state/binding/host intent, theme/token styling, accessibility/focus, layout/container/virtualization, render/surface output, overlay/popup/layering, text, Surface2D, SpatialCanvas, NodeCanvas, PortGraphCanvas, ProgressionTreeView, TrackSurface/Timeline, transitions/effects, and adoption gates.
<!-- END RUNENWERK:UI_COMPONENT_PLATFORM:domain-ui-note -->
