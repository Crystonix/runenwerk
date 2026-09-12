---
title: UI Designer Component Surface And Widget Recipe Library Design
description: Accepted design for PM-UI-DESIGN-006 reusable component, surface, and widget recipe contracts over shared UI primitives and target profiles.
status: accepted
owner: editor
layer: domain/ui-definition
canonical: true
last_reviewed: 2026-05-22
related_adrs:
  - ../../adr/accepted/0004-separate-description-from-execution.md
  - ../../adr/accepted/0005-projections-are-derived-state.md
  - ../../adr/superseded/0012-capability-workbench-clean-break.md
related_designs:
  - ./ui-designer-canonical-ir-and-composition-design.md
  - ./ui-designer-target-projection-profiles-design.md
  - ./ui-designer-visual-layout-and-interface-composition-design.md
  - ./ui-designer-theme-tokens-modes-skins-and-state-variants-design.md
  - ./ui-designer-interface-lab-platform-design.md
  - ../active/editor-tool-suite-registry-and-workbench-host-design.md
related_roadmaps:
  - ../../domain/ui/roadmap.md
---

# UI Designer Component Surface And Widget Recipe Library Design

## Status

This is the accepted implementation design for `PM-UI-DESIGN-006`.

It defines the ownership and contract shape for reusable component, surface, and
widget recipes. It does not implement code or authorize product changes by
itself. Implementation requires an owning GitHub issue, canonical roadmap
sequencing where relevant, and pull-request-owned review and exact-head
validation.

## Goal

The UI Designer needs a reusable recipe library above Canonical UI IR and theme
tokens:

```text
shared UI primitives
  -> component recipes
  -> widget recipes
  -> surface recipes
  -> target-profile compatibility
  -> composed Canonical UI IR
```

Recipes must describe UI structure, slots, required token families, state
variants, accessibility metadata, layout behavior, focus/navigation behavior,
and target-profile compatibility without owning editor, gameplay, renderer,
material, project, provider, or runtime truth.

## Architecture Governance Result

Architecture governance accepts this implementation direction:

- `domain/ui/ui_definition` owns runtime-neutral component, widget, and surface
  recipe ids, recipe declarations, slot contracts, recipe composition,
  target-profile compatibility checks, and recipe diagnostics.
- `domain/ui/ui_theme` owns token graph resolution and typed token ids consumed
  by recipes. It does not own component recipe composition.
- `domain/editor/editor_definition` owns editor/workbench-specific recipe
  packages and adapters into generic recipe contracts.
- Future game-runtime UI recipe packages must depend on shared `domain/ui`
  contracts and game-owned target extensions, not editor shell or app session
  state.
- `apps/runenwerk_editor` may host recipe browsers, Designer/Lab previews, and
  project IO, but it must not own generic recipe truth.

No new ADR is required for the first PM-006 design because the direction
preserves description-versus-execution and derived-projection ADRs. A future ADR
or accepted design update is required before app state, renderer handles,
provider sessions, editor shell state, or game-runtime state become recipe
source truth.

## Recipe Contract

Every generic recipe declaration includes:

- stable recipe id;
- recipe kind: component, widget, or surface;
- human-readable label and category;
- target-profile compatibility;
- Canonical UI IR node template;
- named slots with accepted child recipe kinds;
- required token families and optional default token ids;
- supported state variants;
- accessibility role, label strategy, and required semantics;
- layout behavior and sizing constraints;
- focus, navigation, and interaction affordance descriptors;
- source package id and optional source location;
- diagnostic provenance.

Recipes may reference token ids and state ids, but they do not resolve concrete
theme values by themselves. Concrete styling remains a derived result of
`PM-UI-DESIGN-005` token resolution and target-profile projection.

## Recipe Composition

Recipe composition is deterministic:

1. resolve the requested target profile;
2. validate recipe id, kind, slot, and target-profile compatibility;
3. expand recipe templates into Canonical UI IR nodes with stable authored ids;
4. attach token, state, accessibility, focus, and navigation references;
5. validate required slots and required semantic metadata;
6. report unsupported target-profile features before activation.

Composition output is derived state. It can feed visual editing, preview, and
target projection, but the authored recipe declaration and authored UI document
remain source truth.

## Target Profiles

The editor/workbench target profile consumes generic recipes through
editor/workbench adapters for panels, menus, inspectors, graph canvases, data
tables, diagnostics surfaces, and provider-hosted tool surfaces. These adapters
may reference `domain/editor/editor_definition` and `domain/editor/editor_shell`
contracts but must not move generic recipe invariants into those crates.

The game-runtime target profile consumes generic recipes for HUDs, hotbars,
inventory, dialogue, quest, minimap, player status, world-space UI, safe-area
layouts, platform prompts, and accessibility modes. Game-runtime extensions must
not depend on editor shell ownership.

## Diagnostics

Recipe diagnostics include:

- stable diagnostic code;
- severity;
- source location when available;
- recipe id and slot path;
- target profile;
- owning domain;
- source package;
- winning and losing source provenance when composition conflicts;
- activation impact;
- suggested fix.

The first taxonomy covers duplicate recipe ids, unknown recipe references,
unsupported target profiles, invalid slot children, missing required slots,
missing required token families, missing accessibility semantics, incompatible
state variants, invalid focus/navigation descriptors, and preview-only
composition attempts.

## Implementation Activation

This accepted design does not select or activate an implementation slice.

The next legal implementation action is to create or select one bounded owning
GitHub issue. That issue should cover only the first generic
`domain/ui/ui_definition` recipe-contract slice and should not implement
app-hosted recipe browsers, game-runtime package loading, renderer lowering,
binding activation, preview matrices, persistence activation, or production
readiness.

Historical `PT-*`, `PM-*`, and `WR-*` labels may remain as decomposition or
provenance vocabulary; they do not grant current implementation authority.

## Required Fitness Functions

The first implementation slice must add focused validation for:

- stable recipe id preservation;
- deterministic recipe expansion into Canonical UI IR;
- slot compatibility diagnostics;
- required token-family diagnostics;
- accessibility metadata diagnostics;
- target-profile compatibility diagnostics;
- preview-only activation rejection;
- editor/workbench and game-runtime target-profile examples without sharing
  domain semantics.

## Non-Goals

PM-006 design acceptance does not:

- implement app-hosted component, widget, or surface recipe UI;
- implement editor-specific recipe package storage;
- implement game-runtime recipe package loading;
- implement renderer material lowering or GPU policy;
- implement view-model or intent binding from PM-UI-DESIGN-007;
- implement live preview fixture matrices from PM-UI-DESIGN-008;
- implement persistence activation from PM-UI-DESIGN-009;
- implement production readiness from PM-UI-DESIGN-010;
- move editor, gameplay, render, material, scene, asset, simulation, save-game,
  network, project, provider, or app truth into generic recipes.

## Acceptance Bar

PM-006 remains an accepted design when:

- this accepted design exists and is discoverable from current UI design
  navigation;
- the canonical UI roadmap carries any durable sequence that remains relevant;
- implementation is activated only by an owning GitHub issue;
- delivery evidence belongs to the reviewed pull request;
- current repository validation passes at the accepted revision.
