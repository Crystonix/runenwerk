---
title: UI Designer Theme Tokens Modes Skins And State Variants Design
description: Accepted design for PM-UI-DESIGN-005 deterministic UI theme token graph, modes, skins, and state variant resolution.
status: accepted
owner: editor
layer: domain/ui-theme
canonical: true
last_reviewed: 2026-05-22
related_adrs:
  - ../../adr/accepted/0004-separate-description-from-execution.md
  - ../../adr/accepted/0005-projections-are-derived-state.md
related_designs:
  - ./ui-designer-canonical-ir-and-composition-design.md
  - ./ui-designer-target-projection-profiles-design.md
  - ./ui-designer-visual-layout-and-interface-composition-design.md
  - ./ui-designer-interface-lab-platform-design.md
  - ../implemented/editor-self-authoring-and-final-ui-design.md
related_roadmaps:
  - ../../domain/ui/roadmap.md
---

# UI Designer Theme Tokens Modes Skins And State Variants Design

## Status

This is the accepted implementation design for `PM-UI-DESIGN-005`.

It defines deterministic styling ownership and the first implementation
direction for token graph and mode resolution. It does not implement code, does
not mark PM-005 complete, and does not authorize product changes by itself.
Implementation requires an owning GitHub issue, canonical roadmap sequencing
where relevant, and pull-request-owned review and exact-head validation.

## Goal

UI styling must resolve through one deterministic token graph:

```text
primitive tokens
  -> semantic tokens
  -> component tokens
  -> state variants
  -> mode overrides
  -> theme package
  -> skin package
  -> platform overrides
  -> accessibility overrides
  -> resolved target-profile style packet
```

Every resolved value must retain provenance for the winning source and rejected
or overridden sources. Unsupported target-profile features, cycles, missing
aliases, incompatible modes, and accessibility conflicts fail closed with typed
diagnostics before activation.

## Architecture Governance Result

Architecture governance accepts this implementation direction:

- `domain/ui/ui_theme` owns runtime-neutral token ids, token value kinds,
  aliases, mode and state selectors, deterministic resolution, provenance, and
  generic styling diagnostics.
- `domain/ui/ui_definition` owns references from Canonical UI IR to token,
  theme, state, and skin identifiers. It does not own concrete resolved theme
  values.
- `domain/editor/editor_definition` owns editor/workbench-specific theme
  definition packages and adapters into generic UI theme contracts.
- Future game-runtime UI theme packages must depend on shared `domain/ui`
  contracts, not editor shell or app session state.
- `apps/runenwerk_editor` may host Theme Designer and Lab surfaces, preview
  orchestration, and project IO, but it must not own generic token graph truth.

No new ADR is required for the first PM-005 implementation slice because this
preserves the accepted description-versus-execution and derived-projection
decisions. A future ADR or accepted design update is required before editor app
state, runtime renderer handles, material graphs, or provider sessions become
theme source truth.

## Token Graph Contract

The generic token graph declares:

- stable token id;
- token family, such as color, spacing, radius, typography, opacity, elevation,
  border, duration, or easing;
- value kind and typed value;
- optional alias target;
- source package id;
- target-profile compatibility;
- optional component scope;
- optional state variant selector;
- optional mode selector;
- optional platform selector;
- optional accessibility selector.

Alias resolution is acyclic. Missing aliases, family mismatches, unsupported
target profiles, non-finite numeric values, malformed color values, and
incompatible selector combinations are blocking diagnostics.

## Deterministic Resolution

Resolution is ordered and stable:

1. primitive token values;
2. semantic aliases;
3. component token defaults;
4. state variants;
5. mode overrides;
6. theme package overrides;
7. skin package overrides;
8. platform overrides;
9. accessibility overrides;
10. preview-only overrides.

Preview-only overrides may be inspected and compared, but they cannot activate
or persist unless they can be represented as a deterministic token graph diff.

## Diagnostics

Token/theme diagnostics include:

- stable diagnostic code;
- severity;
- source location when available;
- token id or alias path;
- target profile;
- owning domain;
- source package;
- winning and losing sources when applicable;
- activation impact;
- suggested fix.

The first taxonomy covers token cycles, missing aliases, family mismatches,
unknown modes, duplicate selectors, incompatible state/mode combinations,
accessibility override conflicts, unsupported target-profile features, malformed
values, and preview-only activation attempts.

## Implementation Activation

`WR-049` is retained as historical decomposition/provenance for the first
PM-005 token-graph slice. It is not current implementation authority.

Any current or future implementation must be activated by an owning GitHub
issue. The bounded first slice remains the generic `domain/ui/ui_theme` token
graph and resolution diagnostics, with narrow `domain/ui/ui_definition` token
reference contracts only when required to keep Canonical UI IR references
typed. It must not implement app-hosted Theme Designer UI, editor-specific
package storage, game-runtime theme package loading, renderer material lowering,
component recipe libraries, binding, preview matrices, persistence activation,
or production readiness.

Historical `PT-*`, `PM-*`, and `WR-*` labels may remain as decomposition or
provenance vocabulary; they do not grant current implementation authority.

## Required Fitness Functions

The implementation slice must add focused validation for:

- deterministic layer ordering;
- alias cycle rejection;
- missing alias rejection;
- token family mismatch rejection;
- mode and state selector precedence;
- accessibility override conflict diagnostics;
- target-profile compatibility diagnostics;
- preview-only override activation rejection.

## Non-Goals

PM-005 design acceptance does not:

- implement app-hosted Theme Designer or Interface Lab UI;
- implement component recipe libraries from PM-UI-DESIGN-006;
- implement view-model or intent binding from PM-UI-DESIGN-007;
- implement live preview fixture matrices from PM-UI-DESIGN-008;
- implement persistence activation from PM-UI-DESIGN-009;
- implement production readiness from PM-UI-DESIGN-010;
- move material, renderer, editor shell, project, or game-runtime truth into
  the token graph.

## Acceptance Bar

PM-005 remains an accepted design when:

- this accepted design exists and is discoverable from current UI design
  navigation;
- the canonical UI roadmap carries any durable sequence that remains relevant;
- implementation is activated only by an owning GitHub issue;
- delivery evidence belongs to the reviewed pull request;
- current repository validation passes at the accepted revision.
