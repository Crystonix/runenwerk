---
title: UI Component Platform Authoring Kit Design
description: Phase 2 design for ergonomic ControlPackage authoring helpers that preserve the Phase 1 ControlPackage and ControlKernel contract boundaries.
status: active
owner: ui
layer: domain
canonical: true
last_reviewed: 2026-06-25
related_designs:
  - ./ui-component-platform-control-kernel-design.md
  - ./runenwerk-ui-platform-capability-roadmap.md
  - ./runenwerk-ui-story-driven-golden-workflow-design.md
  - ../implemented/editor-ui-runtime-v2-and-interaction-formation-design.md
related_docs:
  - ../../domain/ui/architecture.md
  - ../../domain/ui/roadmap.md
  - ../../workspace/planning/roadmap.md
---

# UI Component Platform Authoring Kit Design

## Status

This is the Phase 2 planning and acceptance design for `feature/ui-component-platform-002-authoring-kit`.

It follows Phase 1, which established the reusable `ControlPackage` and `ControlKernel` contract. This document may authorize a later bounded implementation pass only after Phase 1 is merged and local validation is green. It does not authorize runtime widget behavior, story runner behavior, Gallery previews, Designer UX, Workbench behavior, canvas surfaces, text editing, transitions, or runtime mount eligibility.

## Problem

Phase 1 made the reusable control contract explicit, but hand-authoring a valid `ControlPackage` still requires repeated boilerplate across:

- package IDs;
- control kind IDs;
- property, state, and event payload schemas;
- five kernel IDs;
- fixtures;
- diagnostics;
- migrations;
- stories;
- route requirements;
- binding, theme, accessibility, render, and budget requirements;
- explicit descriptor-only compatibility and non-mount eligibility.

That repetition is useful as a raw contract, but it is too error-prone as the long-term authoring path for new base controls and future reusable surfaces.

## Decision

Add a thin authoring kit for `ui_controls` that constructs ordinary Phase 1 descriptors through a smaller, typed authoring API.

The authoring kit must produce the same public contract objects as the manual path:

```text
ControlPackageDescriptor
ControlKindDescriptor
ControlModuleDescriptor
ControlKernelSet
ControlSchemaDescriptor
ControlFixtureDescriptor
ControlDiagnosticDescriptor
ControlMigrationHook
ControlStoryDescriptor
ControlRouteRequirement
ControlRenderEvidenceRequirement
ControlBudgetEvidenceRequirement
```

The authoring kit is a construction convenience only. Validation remains owned by `ControlPackageDescriptor::validate_contract`, registry insertion remains fail-closed through `ControlPackageRegistry::register`, and artifact export remains read-only through `UiControlPackageArtifact`.

## Ownership

Owning crate:

```text
domain/ui/ui_controls
```

Candidate module boundary:

```text
domain/ui/ui_controls/src/authoring.rs
```

Use a single `authoring.rs` first. Split into `domain/ui/ui_controls/src/authoring/` only when the module has multiple stable responsibilities that would otherwise blur together.

## Proposed API Shape

The implementation should prefer narrow builders/helpers over a broad generic framework.

Candidate public concepts:

```text
ControlPackageAuthoringSpec
ControlKindAuthoringSpec
ControlModuleAuthoringBuilder
ControlPackageAuthoringBuilder
ControlKernelAuthoring
ControlSchemaAuthoring
ControlEvidenceAuthoring
```

The final names may differ after inspecting nearby module conventions, but the responsibilities should remain:

- derive stable, namespaced IDs from a package namespace and control suffix;
- require explicit display name, description, target profile, route capability, and schema inputs;
- attach exactly five kernel roles: layout, interaction, visual, accessibility, inspection;
- attach descriptor-only compatibility defaults;
- attach explicit non-mount eligibility by default;
- attach fixture, diagnostic, migration, story, render evidence, and budget evidence placeholders;
- return ordinary Phase 1 descriptors;
- leave validation to existing validation APIs.

## Required Defaults

The authoring kit may provide defaults only when they preserve Phase 1 doctrine:

- descriptor-only compatibility is allowed;
- non-mount eligibility is required by default;
- story/render/budget evidence requirements may be created as requirements, not proof;
- package/control IDs must remain stable and namespaced;
- diagnostics must remain explicit and stable;
- route requirements must remain host-owned intent/capability metadata;
- migration version starts at `ControlPackageVersion::new(1)` unless explicitly specified.

## Non-Goals

Do not implement:

- runtime widget behavior;
- story runner behavior;
- Gallery previews;
- Designer UX;
- Workbench behavior;
- Surface2D;
- SpatialCanvas;
- NodeCanvas;
- PortGraphCanvas;
- ProgressionTreeView;
- TrackSurface;
- Timeline;
- CurveEditor;
- transitions;
- text editing;
- runtime mount eligibility;
- app/editor/game mutation;
- consumer-specific behavior inside `ui_controls`;
- macros or code generation unless a later design proves simple builders are insufficient.

## Boundary Rules

- The authoring kit must not bypass `ControlPackageDescriptor::validate_contract`.
- The authoring kit must not widen registry insertion behavior.
- The authoring kit must not create hidden global registries.
- The authoring kit must not move Gallery, Workbench, or UI Designer semantics into `ui_controls`.
- The authoring kit must not turn evidence requirements into evidence proof.
- The authoring kit must not make any control runtime-mount eligible.
- The manual descriptor path must remain valid and public.

## Acceptance Criteria

Phase 2 is complete only when:

- the authoring API exists in `domain/ui/ui_controls`;
- `runenwerk_control_package()` can either remain manual or be migrated to the authoring kit with no semantic contract change;
- tests prove the authoring kit produces a valid package equivalent to the intended Phase 1 base package contract;
- invalid authoring output still fails closed through existing validation;
- descriptor-only compatibility remains explicit;
- non-mount eligibility remains explicit;
- registry insertion still validates before insertion;
- no consumer-specific product behavior enters `ui_controls`.

## Test Plan

Required focused tests:

```text
cargo test -p ui_controls control_authoring
cargo test -p ui_controls control_package
cargo test -p ui_controls control_registry
cargo test -p ui_controls control_kernel
cargo test -p ui_artifacts control_package
cargo test -p ui_program route
```

Required static checks:

```text
cargo fmt --all --check
cargo check -p ui_controls
git diff --check
```

Recommended test cases:

- authoring kit builds one valid base control module;
- authoring kit builds the full Runenwerk base controls package or an equivalent focused package fixture;
- missing schema still reports `MissingSchema`;
- missing kernel still reports `MissingKernel`;
- missing diagnostic/fixture/story still fails closed;
- authoring output remains not mount eligible;
- registry rejects invalid authoring output.

## Phase 2 Implementation Gate

Before writing Rust code, confirm:

- Phase 1 is merged or the Phase 2 branch is intentionally based on the completed Phase 1 branch;
- this design is still current;
- planning records name Phase 2 as active;
- no new stop condition has been triggered;
- local validation for Phase 1 passed.

## Handoff

Start implementation only after this design and planning state are accepted. The first implementation pass should add the smallest coherent authoring module and focused tests. Do not add story proof, catalog/discovery, input/gesture/device, text, canvas, timeline, transition, Gallery, Designer, or Workbench behavior in Phase 2.
