---
title: UI Designer Production Readiness And Evidence Design
description: Accepted design for PM-UI-DESIGN-010 production readiness, diagnostics inspection, golden evidence, accessibility, compatibility, performance, and example evidence contracts.
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
  - ./ui-designer-view-model-capability-and-intent-binding-design.md
  - ./ui-designer-live-preview-fixtures-scenarios-and-target-matrix-design.md
  - ./ui-designer-persistence-migration-diff-and-activation-design.md
  - ./ui-designer-interface-lab-platform-design.md
related_roadmaps:
  - ../../domain/ui/roadmap.md
---

# UI Designer Production Readiness And Evidence Design

## Status

This is the accepted hardening design for `PM-UI-DESIGN-010`.

It defines the ownership and contract shape for production readiness evidence,
diagnostic inspection, golden projection snapshots, accessibility reports,
compatibility reports, performance budget reports, and example evidence. It
does not implement code or authorize hardening changes by itself. Implementation
requires an owning GitHub issue, canonical roadmap sequencing where relevant,
and pull-request-owned review and exact-head validation.

## Goal

The UI Designer needs a production evidence envelope that proves authored UI
definitions remain definition-driven across editor/workbench and game-runtime
target profiles:

```text
validated UI definition
  -> target projection evidence descriptors
  -> diagnostic inspection report
  -> accessibility and compatibility reports
  -> performance budget report
  -> golden projection snapshot references
  -> production readiness decision
```

The generic UI definition domain owns evidence descriptors and readiness
decisions. App, renderer, runtime, provider, accessibility tooling, and
performance tooling produce concrete artifacts and consume the generic evidence
contract; they do not become source truth for UI definitions.

## Architecture Governance Result

Architecture governance accepts this implementation direction:

- `domain/ui/ui_definition` owns runtime-neutral readiness descriptors,
  evidence packet ids, diagnostic inspection DTOs, projection snapshot
  references, accessibility report references, compatibility report references,
  performance budget report references, example evidence references, readiness
  validation requests, readiness decisions, and typed readiness diagnostics.
- `domain/ui/ui_theme`, `domain/ui`, `domain/editor/editor_definition`, and
  future game UI target domains may provide evidence inputs through explicit
  descriptors owned by their domains.
- `apps/runenwerk_editor` may orchestrate user-facing Designer/Lab readiness
  UI, visual capture, example loading, and report presentation, but it must call
  domain contracts instead of owning generic readiness truth.
- Renderer, runtime, provider, accessibility, and performance tooling produce
  artifacts referenced by evidence descriptors. They do not decide whether
  authored UI definitions are valid source truth.

No new ADR is required for the first PM-010 design because ADR-0004 separates
description from execution, ADR-0005 treats projections as derived state,
ADR-0006 keeps provider execution behind app/provider boundaries, and ADR-0012
keeps host capability policy explicit. A future ADR or accepted design update
is required before generic readiness evidence owns screenshot bytes, renderer
handles, runtime sessions, provider sessions, project storage, gameplay truth,
or concrete editor command execution.

## Evidence Packet Contract

Every production readiness evidence packet includes:

- stable evidence packet id;
- source document or package id;
- target profile;
- source package provenance;
- projection snapshot references;
- diagnostic inspection report references;
- accessibility report references;
- compatibility report references;
- performance budget report references;
- golden artifact references;
- example scenario references;
- expected diagnostic refs;
- artifact freshness and completeness policy.

Evidence packet descriptors reference artifacts produced elsewhere. They do not
embed screenshots, renderer handles, runtime state, provider session state, or
gameplay state.

## Diagnostic Inspection Contract

Inspection reports must let authors understand readiness failures without
runtime internals. A report can include:

- composition and source-map diagnostics;
- style and token diagnostics;
- recipe and slot diagnostics;
- binding and intent diagnostics;
- fixture and preview diagnostics;
- persistence migration, diff, and activation diagnostics;
- accessibility diagnostics;
- compatibility diagnostics;
- performance budget diagnostics.

Inspection report DTOs retain stable diagnostic codes, severity, source
locations, owning domain, target profile, activation impact, and suggested fix.
They remain observation artifacts and must not mutate definitions or execute
commands.

## Readiness Decision Contract

Readiness requests include:

- evidence packet id;
- target profile;
- validation mode: inspect, dry-run, release candidate, or production;
- required evidence kinds;
- expected diagnostics;
- freshness policy;
- source package provenance.

Readiness decisions are fail-closed. Production readiness can pass only when:

- required projection, diagnostic, accessibility, compatibility, performance,
  and example evidence descriptors are present;
- evidence target profiles match the readiness request;
- stale or missing artifacts are blocked unless the request is explicitly
  inspect-only;
- expected diagnostics match actual diagnostics;
- blocking activation-impact diagnostics are absent;
- evidence descriptors do not claim concrete ownership of app, runtime,
  renderer, provider, project, or gameplay truth.

## Diagnostics

Production readiness diagnostics include:

- stable diagnostic code;
- severity;
- source location when available;
- evidence packet id;
- readiness request id;
- target profile;
- missing evidence kind;
- stale artifact reference;
- expected and actual diagnostic references;
- owning domain;
- source package;
- activation impact;
- suggested fix.

The first taxonomy covers missing projection snapshots, missing diagnostic
inspection reports, missing accessibility reports, missing compatibility
reports, missing performance budget reports, stale evidence, target-profile
mismatch, expected diagnostic mismatch, artifact ownership violation, and
preview-only production readiness attempts.

## Implementation Activation

This accepted design does not select or activate an implementation slice.

The next legal implementation action is to create or select one bounded owning
GitHub issue. That issue should cover only the first generic
`domain/ui/ui_definition` readiness evidence packet, inspection report,
readiness request, readiness decision, and readiness diagnostic contract slice.

The first slice must not implement app-hosted readiness UI, screenshot capture,
renderer golden comparison, accessibility engine integration, performance
runner integration, project IO, runtime replay, provider sessions, or concrete
release tooling.

Historical `PT-*`, `PM-*`, and `WR-*` labels may remain as decomposition or
provenance vocabulary; they do not grant current implementation authority.

## Required Fitness Functions

The first implementation slice must add focused validation for:

- required evidence kind coverage;
- target-profile compatibility diagnostics;
- stale evidence diagnostics;
- missing projection snapshot diagnostics;
- missing diagnostic inspection report diagnostics;
- missing accessibility report diagnostics;
- missing compatibility report diagnostics;
- missing performance budget report diagnostics;
- expected diagnostic mismatch diagnostics;
- artifact ownership violation diagnostics;
- preview-only production readiness rejection;
- editor/workbench and game-runtime examples without sharing runtime, provider,
  renderer, project IO, screenshot, or gameplay ownership.

## Non-Goals

PM-010 design acceptance does not:

- implement app-hosted production readiness UI;
- implement visual capture, screenshot diffing, or renderer golden comparison;
- implement accessibility engine integration or performance runner integration;
- implement project save/load or release tooling;
- implement provider sessions or runtime replay loading;
- move editor, gameplay, render, material, scene, asset, simulation, save-game,
  network, project, provider, app, runtime, screenshot, accessibility engine,
  or performance runner truth into generic UI readiness contracts.

## Acceptance Bar

PM-010 remains an accepted design when:

- this accepted design exists and is discoverable from current UI design
  navigation;
- the canonical UI roadmap carries any durable sequence that remains relevant;
- implementation is activated only by an owning GitHub issue;
- delivery evidence belongs to the reviewed pull request;
- current repository validation passes at the accepted revision.
