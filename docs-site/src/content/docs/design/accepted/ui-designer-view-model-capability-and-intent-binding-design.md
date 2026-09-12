---
title: UI Designer View-Model Capability And Intent Binding Design
description: Accepted design for PM-UI-DESIGN-007 read-only view-model bindings, capability gates, and validated UI intent declarations.
status: accepted
owner: editor
layer: domain/ui-definition
canonical: true
last_reviewed: 2026-05-22
related_adrs:
  - ../../adr/accepted/0001-use-domain-owned-commands.md
  - ../../adr/accepted/0004-separate-description-from-execution.md
  - ../../adr/accepted/0005-projections-are-derived-state.md
  - ../../adr/superseded/0006-editor-surface-provider-plugin-seam.md
  - ../../adr/superseded/0012-capability-workbench-clean-break.md
related_designs:
  - ./ui-designer-canonical-ir-and-composition-design.md
  - ./ui-designer-target-projection-profiles-design.md
  - ./ui-designer-visual-layout-and-interface-composition-design.md
  - ./ui-designer-theme-tokens-modes-skins-and-state-variants-design.md
  - ./ui-designer-component-surface-and-widget-recipe-library-design.md
  - ./ui-designer-interface-lab-platform-design.md
  - ../active/editor-tool-suite-registry-and-workbench-host-design.md
  - ../superseded/runenwerk-capability-workbench-target-architecture.md
related_roadmaps:
  - ../../domain/ui/roadmap.md
---

# UI Designer View-Model Capability And Intent Binding Design

## Status

This is the accepted implementation design for `PM-UI-DESIGN-007`.

It defines the ownership and contract shape for read-only view-model bindings,
capability-gated target-profile compatibility, and validated UI intent
declarations. It does not implement code or authorize product changes by itself.
Implementation requires an owning GitHub issue, canonical roadmap sequencing
where relevant, and pull-request-owned review and exact-head validation.

## Goal

The UI Designer needs UI definitions that can display domain-owned state and
emit user intents without moving semantic authority into generic UI contracts:

```text
domain-owned state
  -> read-only view-model package
  -> UI binding reference
  -> target-profile projection
  -> validated intent proposal
  -> owning domain command or runtime intent gate
```

The binding layer must make normal UI authoring discoverable while preserving
the command and capability boundaries from the accepted architecture ADRs.

## Architecture Governance Result

Architecture governance accepts this implementation direction:

- `domain/ui/ui_definition` owns runtime-neutral binding references, binding
  value types, intent declaration references, capability requirements, target
  compatibility checks, and typed binding diagnostics.
- `domain/editor/editor_definition` owns editor/workbench-specific view-model
  package declarations, editor command intent adapters, menu/shortcut intent
  adapters, and workbench capability vocabulary.
- Future game-runtime UI domains own game-specific view-model packages and game
  intent adapters.
- `foundation/commands` may provide shared descriptor vocabulary, but concrete
  command families remain domain-owned.
- `apps/runenwerk_editor` may host previews, command bridges, and Designer/Lab
  interaction surfaces, but it must not own generic binding or domain semantic
  truth.

No new ADR is required for the first PM-007 design because ADR-0001 already
requires domain-owned commands, ADR-0004 separates description from execution,
ADR-0005 treats projections as derived state, ADR-0006 keeps provider proposals
behind command boundaries, and ADR-0012 defines host capability policy. A new
ADR or accepted design update is required before generic UI definitions own
concrete command payloads, direct mutation, cross-domain reflection authority,
or game-runtime state truth.

## Binding Contract

Every generic binding declaration includes:

- stable binding id;
- target profile compatibility;
- view-model package id and source package provenance;
- read-only field reference;
- declared value type;
- optional formatter or localization strategy reference;
- required capability ids;
- freshness and missing-data policy;
- diagnostic provenance.

Bindings may reference domain-owned view-model fields, but they never own the
underlying domain state. A binding reads a projected value packet supplied by
the owning domain or target adapter. Missing packages, denied capabilities,
type mismatches, stale data, unsupported target profiles, and missing formatter
strategies fail closed with typed diagnostics before activation.

## Intent Contract

Every generic UI intent declaration includes:

- stable intent id;
- user-facing trigger source, such as button, menu, shortcut, selection, focus,
  or pointer gesture;
- target profile compatibility;
- required capability ids;
- owning domain or adapter family id;
- domain-owned command descriptor or game intent descriptor reference;
- payload binding references, not embedded domain mutation logic;
- validation mode for preview, dry-run, and activation;
- diagnostic provenance.

UI definitions emit validated intent proposals. They do not execute commands,
mutate domain truth, bypass capability policy, or create concrete command
payloads without the owning domain or target adapter validating the proposal.
Denied capabilities and command, shortcut, focus, or payload conflicts block
activation.

## Target Profiles

The editor/workbench target profile consumes generic bindings through
editor/workbench adapters for asset browsers, material inspectors, diagnostics,
graph editors, tool surfaces, menus, shortcuts, and command palettes. Workbench
capability policy remains the gate for provider proposals, shell commands, and
editor-domain mutations.

The game-runtime target profile consumes generic bindings for combat HUDs,
inventory, abilities, quests, dialogue, minimaps, player status, party frames,
boss frames, world-space UI, safe-area layouts, and platform prompts. Game
intent adapters must remain game-owned and must not depend on editor shell
ownership.

## Diagnostics

Binding and intent diagnostics include:

- stable diagnostic code;
- severity;
- source location when available;
- binding id or intent id;
- affected node, slot, target profile, and trigger source;
- owning domain;
- source package;
- required and denied capability ids;
- activation impact;
- suggested fix.

The first taxonomy covers unknown view-model package references, unknown field
references, value type mismatches, stale or missing data policy violations,
unknown capability ids, denied capabilities, unsupported target profiles,
unknown command or intent descriptors, invalid payload bindings, command and
shortcut conflicts, focus conflicts, preview-only validation attempts, and
direct mutation attempts.

## Implementation Activation

This accepted design does not select or activate an implementation slice.

The next legal implementation action is to create or select one bounded owning
GitHub issue. That issue should cover only the first generic
`domain/ui/ui_definition` binding and intent declaration contract slice and
should not implement app-hosted Designer/Lab binding UI, editor-specific package
persistence, game-runtime package loading, concrete command execution, preview
fixture matrices, persistence activation, or production readiness.

Historical `PT-*`, `PM-*`, and `WR-*` labels may remain as decomposition or
provenance vocabulary; they do not grant current implementation authority.

## Required Fitness Functions

The first implementation slice must add focused validation for:

- read-only binding declarations that preserve stable ids;
- binding value type compatibility diagnostics;
- missing or stale view-model package diagnostics;
- denied capability diagnostics;
- unsupported target-profile diagnostics;
- intent declarations that produce proposals rather than direct mutation;
- command, shortcut, payload, and focus conflict diagnostics;
- preview/dry-run/activation validation differences;
- editor/workbench and game-runtime examples without sharing semantic authority.

## Non-Goals

PM-007 design acceptance does not:

- implement app-hosted binding or intent Designer/Lab UI;
- implement editor-specific binding package storage;
- implement game-runtime view-model package loading;
- execute editor commands or game intents;
- implement renderer material lowering or GPU policy;
- implement live preview fixture matrices from PM-UI-DESIGN-008;
- implement persistence activation from PM-UI-DESIGN-009;
- implement production readiness from PM-UI-DESIGN-010;
- move editor, gameplay, render, material, scene, asset, simulation, save-game,
  network, project, provider, app, or runtime truth into generic UI bindings.

## Acceptance Bar

PM-007 remains an accepted design when:

- this accepted design exists and is discoverable from current UI design
  navigation;
- the canonical UI roadmap carries any durable sequence that remains relevant;
- implementation is activated only by an owning GitHub issue;
- delivery evidence belongs to the reviewed pull request;
- current repository validation passes at the accepted revision.
