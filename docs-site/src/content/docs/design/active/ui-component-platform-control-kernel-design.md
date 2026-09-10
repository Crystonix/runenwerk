---
title: UI Component Platform Control Kernel Design
description: Defines the reusable ControlPackage and control-kernel contract for all story-proven Runenwerk UI controls and surfaces.
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

# UI Component Platform Control Kernel Design

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

Every reusable control or surface package must expose one consistent package contract before it can be accepted by PT-UI-COMPONENT-PLATFORM. The kernel is the shared shape for identity, schema, state, binding, theme, accessibility, diagnostics, stories, render evidence, budgets, and host intent proposals.

## Feature List

- control package id
- control kind id
- display name, description, category, and tags
- target profiles and compatibility flags
- property schema and state schema
- event payload schema and host intent route ids
- layout, interaction, visual/render kernel ids
- accessibility, binding, and theme/token requirements
- diagnostic, fixture, and story ids
- capability requirements
- migration and deprecation status
- mount eligibility and budget evidence requirements

## Ergonomics Gate

A new control author can copy the package shape and know where to declare props, state, events, fixtures, diagnostics, stories, accessibility facts, token requirements, and host routes.

## Out Of Scope

- product authoring UX
- renderer-owned UI semantics
- game HUD behavior
- world-space/entity-attached UI
- app/editor/game mutation

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
