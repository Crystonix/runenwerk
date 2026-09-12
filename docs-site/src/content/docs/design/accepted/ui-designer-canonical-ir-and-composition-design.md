---
title: UI Designer Canonical IR And Composition Design
description: Accepted design for PM-UI-DESIGN-002 canonical UI IR, validation, deterministic composition, and round-trip authoring boundaries.
status: accepted
owner: editor
layer: domain/ui-definition
canonical: true
last_reviewed: 2026-05-22
related_adrs:
  - ../../adr/accepted/0001-use-domain-owned-commands.md
  - ../../adr/accepted/0004-separate-description-from-execution.md
  - ../../adr/accepted/0005-projections-are-derived-state.md
  - ../../adr/superseded/0012-capability-workbench-clean-break.md
related_designs:
  - ./ui-designer-interface-lab-platform-design.md
  - ../implemented/ui-definition-formation-foundation-design.md
  - ../implemented/editor-self-authoring-and-final-ui-design.md
  - ../active/editor-ui-workspace-tool-surface-architecture.md
related_roadmaps:
  - ../../domain/ui/roadmap.md
---

# UI Designer Canonical IR And Composition Design

## Status

This is the accepted design contract for `PM-UI-DESIGN-002`.

It accepts the canonical UI definition and composition boundary needed before
later UI Designer implementation milestones can write product code. It does not
mark any implementation milestone complete, does not create runtime behavior,
does not create schemas, and does not authorize code changes by itself. Later
implementation requires an owning GitHub issue, canonical roadmap sequencing
where relevant, and pull-request-owned review and exact-head validation.

## Goal

Visual and textual UI/interface authoring must converge through one typed,
inspectable, versioned definition pipeline:

```text
authored UI/interface document
  -> schema/version gate
  -> migration dry-run
  -> Canonical UI IR
  -> deterministic composition
  -> validation and diagnostics
  -> target projection planning
```

The Canonical UI IR is source/IR for UI/interface definitions only. It is not
editor command execution, gameplay state, renderer state, material truth, scene
truth, asset truth, network state, save-game state, or project truth.

## Architecture Governance Result

Architecture governance accepts the existing owner for this milestone:

- `domain/ui/ui_definition` owns generic authored UI definition vocabulary,
  schema/version checks, migration reports, source maps, normalization,
  Canonical UI IR, deterministic composition contracts, generic validation, and
  round-trip definition evidence.
- `domain/editor/editor_definition` owns editor/workbench-specific authored
  extensions: workspace profiles, suites, panels, menus, shortcuts,
  command-binding descriptors, panel registries, tool-surface registries, and
  editor binding metadata.
- Future game-runtime UI target extensions must use shared `domain/ui` and
  `domain/ui/ui_definition` contracts without depending on
  `domain/editor/editor_shell`.
- `apps/runenwerk_editor` may own Designer/Lab surfaces, fixture loading,
  preview orchestration, project IO, and command bridges, but it must not own
  canonical UI/interface truth.

No new ADR is required for PM-002 because this design preserves the current
Clean Architecture dependency direction and keeps canonical definition ownership
inside existing domain crates. A future ADR or accepted design update is
required before extracting a standalone `domain/ui_definition` crate, creating a
new game-runtime UI owner crate, or changing dependency direction between UI,
editor, engine, and app layers.

## Accepted Pipeline

### Authored Document Gate

Every persisted or imported UI/interface document must declare:

- schema version;
- definition category or target family;
- stable authored ids;
- source-map-capable node paths;
- target-profile declarations or target-profile compatibility metadata;
- explicit extension ownership for editor/workbench or game-runtime vocabulary.

Unsupported schema versions, unknown required fields, malformed stable ids, and
invalid target-profile declarations fail before composition or activation.

### Migration Dry-Run

Migration is a dry-run before activation. It reports:

- original schema version;
- target schema version;
- changed fields;
- incompatible fields;
- preserved compatible unknown fields where possible;
- source locations for failures.

The migration step may produce a reviewable textual diff. It must not mutate the
active definition set by itself.

### Canonical UI IR

The Canonical UI IR is the normalized, execution-neutral representation used by
both visual and textual editing. It must preserve:

- stable authored ids;
- source-map paths;
- layout tree structure;
- component and widget references;
- token, theme, mode, state, and skin references;
- binding descriptors to read-only view-model contracts;
- intent descriptors that are validated before activation;
- target-profile compatibility declarations;
- diagnostic provenance.

The IR must not contain retained-runtime `WidgetId` values, ECS entities,
renderer handles, app provider instances, concrete shell sessions, or executable
command callbacks.

### Deterministic Composition

Composition must use this deterministic strength order, lowest to highest:

1. base UI library;
2. shared component recipes;
3. target-profile defaults;
4. project, game, and editor overrides;
5. skin and theme packages;
6. platform overrides;
7. accessibility modes;
8. localization and text expansion modes;
9. user preferences;
10. preview overrides;
11. host/runtime policy overlays.

Each composed value must retain provenance for the winning source and all losing
sources that were overridden or rejected.

## Conflict And Diagnostic Contract

Composition conflicts must be inspectable. A conflict diagnostic includes:

- stable diagnostic code;
- severity;
- source location;
- owning domain;
- affected target profile;
- affected host, suite, or surface when applicable;
- winning source;
- losing source;
- activation impact;
- suggested fix.

The bounded PM-002 taxonomy covers schema/version, id/reference,
layout/composition, token/theme reference, target-profile compatibility, and
migration/compatibility diagnostics. Later PM-UI-DESIGN milestones extend the
taxonomy for target projection, visual layout editing, styling, recipes,
binding, preview, accessibility, performance, and production evidence.

## Round-Trip Authoring

Visual edits are accepted only when they can be expressed as Canonical UI IR and
serialized as deterministic textual diffs.

The round-trip contract is:

```text
textual definition
  -> Canonical UI IR
  -> visual edit operation
  -> Canonical UI IR diff
  -> deterministic textual definition
```

An edit that cannot preserve stable ids, source-map provenance, deterministic
formatting, and reviewable diff output remains preview-only and cannot activate.

## Implementation Sequence

PM-002 is design-only. Later code-bearing work must follow this sequence:

1. Confirm the downstream PM slice remains supported by this accepted design
   and the canonical UI/workspace roadmap.
2. Create or select an owning GitHub issue. The issue owns activation, current
   scope, dependencies, and the accepted base.
3. Implement only the bounded issue scope.
4. Add focused tests and current repository validation.
5. Deliver through a pull request; the PR owns the reviewed feature head,
   complete diff, validation evidence, and acceptance state.

Historical `PT-*`, `PM-*`, and `WR-*` labels may remain as decomposition or
provenance vocabulary; they do not grant current implementation authority.

No implementation issue may treat PM-002 as permission to move editor,
gameplay, render, material, scene, asset, simulation, save-game, network, or
project semantics into the Designer.

## Required Fitness Functions

Later implementation slices must add focused checks for:

- schema/version failures before composition;
- unsupported migration failures before activation;
- deterministic composition order;
- conflict diagnostics with winning and losing source provenance;
- stable id preservation through visual edit round trips;
- reviewable textual diffs after visual edits;
- app/provider/runtime layers consuming projection output without becoming
  source truth.

For this design-only slice, validation is documentation and repository
validation.

## Non-Goals

PM-002 does not:

- implement Canonical UI IR code;
- add a new crate;
- change persisted schemas;
- create Designer/Lab app surfaces;
- implement target projection profiles from PM-UI-DESIGN-003;
- implement visual layout editing from PM-UI-DESIGN-004;
- implement theme/token resolution from PM-UI-DESIGN-005;
- implement component recipe libraries from PM-UI-DESIGN-006;
- implement binding or intent activation from PM-UI-DESIGN-007;
- implement live preview fixtures from PM-UI-DESIGN-008;
- implement persistence activation from PM-UI-DESIGN-009;
- claim production readiness from PM-UI-DESIGN-010.

## Acceptance Bar

PM-002 is accepted when:

- this document is in `design/accepted` with `status: accepted`;
- the accepted design is discoverable from the canonical UI roadmap and related
  design navigation;
- architecture governance records that existing `domain/ui/ui_definition` and
  `domain/editor/editor_definition` ownership is sufficient for PM-002;
- accepted delivery evidence is owned by its reviewed pull request;
- current repository validation passes at the accepted revision.
