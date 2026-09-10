---
title: UI Component Platform TrackSurface Timeline And CurveEditor Design
description: Generic time/track surface plus Timeline and CurveEditor specializations without graph semantics.
status: active
owner: ui
layer: domain
canonical: true
last_reviewed: 2026-06-24
related_designs:
  - ./runenwerk-ui-platform-capability-roadmap.md
  - ./runenwerk-ui-story-driven-golden-workflow-design.md
  - ../implemented/editor-ui-runtime-v2-and-interaction-formation-design.md
related_docs:
  - ../../domain/ui/architecture.md
  - ../../domain/ui/roadmap.md
  - ../../workspace/production-milestone-register.md
---

# UI Component Platform TrackSurface Timeline And CurveEditor Design

## Status

This is an active design for the docs-only activation branch `feature/ui-component-platform-000-activation-vocabulary-ergonomics`. It records scope, vocabulary, and acceptance criteria. It does not authorize product code by itself.

## Canonical Vocabulary

- `ControlPackage` - packaged reusable control family with schema, states, interactions, diagnostics, fixtures, stories, accessibility, tokens, render facts, and host routes.
- `control kernel` - shared contract every control package follows.
- `control authoring kit` - templates, naming conventions, checklists, and examples that make new controls easy to add correctly.
- `component story matrix` - story-proven normal, edge, failure, accessibility, interaction, layout, text, mount, and render states for a package.
- `story proof envelope` - `UiStoryRunReport`, evidence records, expected-failure matching, CLI/Gallery report projection, and mount eligibility.
- `catalog/discovery/inspection contract` - searchable and filterable metadata used by Gallery, UI Designer, docs, and Workbench consumers.
- `host intent proposal` - UI output that proposes host action without mutating app/editor/game truth.
- `route/capability decision` - host-owned authorization result for a proposed route or action.
- `state bucket` - named ownership class for transient, preview, committed, focus, hover, drag, animation, host-fed, or package-owned state.
- `Surface2D` - generic 2D coordinate/navigation primitive.
- `SpatialCanvas` - generic positioned-item surface built on `Surface2D`.
- `NodeCanvas` - generic node/link surface built on `SpatialCanvas`.
- `PortGraphCanvas` - editable port/socket graph specialization built on `NodeCanvas`.
- `ProgressionTreeView` - reusable skill/tech/progression tree package built on `NodeCanvas`, without gameplay rule ownership.
- `TrackSurface` - generic time/track surface; `Timeline` and `CurveEditor` specialize it.

## Non-Negotiable Rules

- General kernels come before specializations.
- Story proof comes before mount eligibility.
- Control packages come before product-specific surfaces.
- `Surface2D` must not collapse into Gallery or GraphCanvas.
- `NodeCanvas` must not contain ports or graph-editor commands.
- `PortGraphCanvas` must not own graph/domain truth.
- `ProgressionTreeView` must not own gameplay/progression rules, point spending, persistence, or mutation.
- `TrackSurface` must not inherit graph semantics.
- Host/app/editor/game mutation remains outside `domain/ui` through explicit host intent or command paths.
- UI Story owns proof orchestration only.
- Gallery, Workbench, and UI Designer consume platform contracts; they do not own reusable control semantics.
- Renderer output remains backend-neutral and must not become UI source truth.

## Decision

`TrackSurface` owns time/track mechanics. `Timeline` and `CurveEditor` specialize it; they do not inherit graph semantics.

## Feature List

- time axis
- tracks and lanes
- clips and keyframes
- cursor and range selection
- scrubbing
- time zoom and time pan
- snap grid
- clip drag intent and keyframe drag intent
- large timeline budget
- Timeline ControlPackage
- CurveEditor ControlPackage

## Ergonomics Gate

Timeline and curve-editing users get shared time navigation, selection, dragging, snapping, invalid-time diagnostics, and budgets.

## Out Of Scope

- graph node semantics
- animation runtime mutation
- audio/game timeline product rules

## Validation

Run the branch-level docs and production validation before implementation:

```text
task roadmap:render
task roadmap:validate
task roadmap:check
task docs:validate
task production:validate
cargo fmt --all --check
```
