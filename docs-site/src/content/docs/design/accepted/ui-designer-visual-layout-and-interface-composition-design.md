---
title: UI Designer Visual Layout And Interface Composition Design
description: Accepted design for PM-UI-DESIGN-004 visual layout editing over Canonical UI IR and deterministic target-profile composition.
status: accepted
owner: editor
layer: domain/ui-definition
canonical: true
last_reviewed: 2026-05-22
related_adrs:
  - ../../adr/accepted/0004-separate-description-from-execution.md
  - ../../adr/accepted/0005-projections-are-derived-state.md
related_designs:
  - ./ui-designer-canonical-ir-and-composition-design.md
  - ./ui-designer-target-projection-profiles-design.md
  - ./ui-designer-interface-lab-platform-design.md
  - ../implemented/ui-definition-formation-foundation-design.md
  - ../implemented/editor-self-authoring-and-final-ui-design.md
related_roadmaps:
  - ../../domain/ui/roadmap.md
---

# UI Designer Visual Layout And Interface Composition Design

## Status

This is the accepted implementation design for `PM-UI-DESIGN-004`.

It defines the first implementation direction for visual layout editing over
Canonical UI IR. It does not implement code, does not mark PM-004 complete, and
does not authorize product changes by itself. Implementation requires an owning
GitHub issue, canonical roadmap sequencing where relevant, and
pull-request-owned review and exact-head validation.

## Goal

Visual layout editing must produce deterministic Canonical UI IR changes:

```text
Canonical UI IR
  -> visual layout edit operation
  -> source-mapped Canonical UI IR diff
  -> deterministic textual definition patch
  -> target-profile composition validation
```

The Designer may preview speculative visual edits, but activation requires a
valid Canonical UI IR diff and a deterministic textual definition patch.

## Architecture Governance Result

Architecture governance accepts this implementation direction:

- `domain/ui/ui_definition` owns generic visual layout edit operations,
  stable-id preservation, deterministic formatting requirements, source-mapped
  diagnostics, and Canonical UI IR diff contracts.
- `domain/editor/editor_definition` owns editor/workbench-specific visual layout
  extensions and compatibility metadata.
- `apps/runenwerk_editor` may later host concrete Designer/Lab UI, preview, and
  file IO, but it must call domain contracts instead of owning canonical
  UI/interface truth.
- Projection, runtime, and provider layers consume valid projection output and
  cannot activate preview-only edits.

No new ADR is required for this PM-004 design because it preserves the accepted
description-versus-execution and derived-projection decisions. A future ADR or
accepted design update is required before visual editing can mutate domain
truth directly or before preview/runtime layers become source truth.

## Visual Edit Operation Contract

Visual edit operations are command-like definition edits, not runtime widget
mutations. Each operation declares:

- operation id;
- source document id;
- target authored node path;
- expected stable authored id;
- target profile;
- edit kind;
- old value or structural precondition;
- new value or structural patch;
- source location when available;
- preview-only flag when the edit cannot be serialized deterministically.

The first bounded implementation slice may focus on core layout-tree operations:

- insert node;
- remove node;
- move node;
- reorder sibling;
- change stack axis;
- change split ratio;
- wrap selection in container;
- unwrap container;
- replace template reference.

Every operation must preserve stable ids unless the edit explicitly creates a
new authored node. New ids are allocated in the definition domain and must be
deterministic enough for reviewable textual diffs.

## Deterministic Formatting And Diffs

Accepted edits must produce:

- canonical field ordering;
- stable child ordering;
- stable id formatting;
- source-map paths for changed nodes;
- explicit before/after values;
- reviewable textual diffs.

An operation that cannot produce a deterministic textual patch remains
preview-only. Preview-only edits must not activate, persist, or update the
active definition set.

## Layout And Composition Diagnostics

Layout/composition diagnostics include:

- stable diagnostic code;
- severity;
- source location;
- affected target profile;
- affected host, suite, or surface when applicable;
- owning domain;
- edit operation id;
- activation impact;
- suggested fix.

Diagnostics must cover:

- missing target node;
- duplicate or unstable authored ids;
- invalid container child count;
- incompatible target profile;
- unsupported layout feature;
- composition conflict after edit;
- non-deterministic diff output;
- preview-only activation attempt.

## Implementation Activation

`WR-047` is retained as historical decomposition/provenance for the first
PM-004 definition-layer edit slice. It is not current implementation authority.

Any current or future implementation must be activated by an owning GitHub
issue. The bounded first slice remains the core definition-layer edit-operation
path. App-hosted Designer/Lab UI and broad visual editor UX remain later PM-004
or downstream work. The slice must not move provider behavior, editor command
execution, runtime `WidgetId`, ECS entity ids, renderer handles, or app sessions
into Canonical UI IR.

Historical `PT-*`, `PM-*`, and `WR-*` labels may remain as decomposition or
provenance vocabulary; they do not grant current implementation authority.

## Required Fitness Functions

The implementation slice must add focused validation for:

- stable id preservation after move/reorder/edit operations;
- deterministic textual diff output;
- preview-only edit rejection during activation;
- source-mapped diagnostics for invalid layout edits;
- target-profile compatibility diagnostics after composition;
- unchanged provider/runtime/app ownership boundaries.

## Non-Goals

PM-004 design acceptance does not:

- implement app-hosted visual Designer UI;
- implement theme/token resolution from PM-UI-DESIGN-005;
- implement component recipe libraries from PM-UI-DESIGN-006;
- implement view-model or intent binding from PM-UI-DESIGN-007;
- implement preview scenario matrices from PM-UI-DESIGN-008;
- implement persistence activation from PM-UI-DESIGN-009;
- claim production readiness from PM-UI-DESIGN-010.

## Acceptance Bar

PM-004 remains an accepted design when:

- this accepted design exists and is discoverable from current UI design
  navigation;
- the canonical UI roadmap carries any durable sequence that remains relevant;
- implementation is activated only by an owning GitHub issue;
- delivery evidence belongs to the reviewed pull request;
- current repository validation passes at the accepted revision.
