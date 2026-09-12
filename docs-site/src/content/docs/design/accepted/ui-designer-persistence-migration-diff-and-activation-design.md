---
title: UI Designer Persistence Migration Diff And Activation Design
description: Accepted design for PM-UI-DESIGN-009 deterministic UI definition persistence, migration dry-runs, reviewable diffs, and fail-closed activation gates.
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
  - ./ui-designer-canonical-ir-and-composition-design.md
  - ./ui-designer-target-projection-profiles-design.md
  - ./ui-designer-visual-layout-and-interface-composition-design.md
  - ./ui-designer-theme-tokens-modes-skins-and-state-variants-design.md
  - ./ui-designer-component-surface-and-widget-recipe-library-design.md
  - ./ui-designer-view-model-capability-and-intent-binding-design.md
  - ./ui-designer-live-preview-fixtures-scenarios-and-target-matrix-design.md
  - ./ui-designer-interface-lab-platform-design.md
related_roadmaps:
  - ../../domain/ui/roadmap.md
---

# UI Designer Persistence Migration Diff And Activation Design

## Status

This is the accepted implementation design for `PM-UI-DESIGN-009`.

It defines the ownership and contract shape for deterministic authored UI
definition persistence, migration dry-runs, reviewable textual diffs, and
fail-closed activation gates. It does not implement code or authorize product
changes by itself. Implementation requires an owning GitHub issue, canonical
roadmap sequencing where relevant, and pull-request-owned review and exact-head
validation.

## Goal

UI/interface definitions must move from persisted authored documents to active
projection input through a reproducible, inspectable gate:

```text
authored UI definition document
  -> schema/version gate
  -> migration dry-run
  -> deterministic normalized document
  -> reviewable textual diff
  -> validation bundle
  -> activation decision
```

The gate protects source truth. It does not execute editor commands, mutate
gameplay state, own provider sessions, create renderer resources, or bypass
target-profile validation.

## Architecture Governance Result

Architecture governance accepts this implementation direction:

- `domain/ui/ui_definition` owns runtime-neutral persistence contracts for
  authored UI definition schema versions, migration requests, dry-run reports,
  unknown-field preservation policy, deterministic textual diff descriptors,
  activation requests, activation decisions, and blocking diagnostics.
- `domain/editor/editor_definition` owns editor/workbench-specific persisted
  extension vocabulary, workbench profile references, menu/shortcut extension
  metadata, editor command descriptors, and adapter-owned translation into the
  generic persistence gate.
- Future game UI target domains own game-runtime-specific persisted extension
  vocabulary and adapter-owned translation into the generic persistence gate.
- `apps/runenwerk_editor` may own concrete project file IO, save/load
  orchestration, user-facing diff review, and activation UI, but it must call
  domain contracts instead of owning canonical UI/interface persistence truth.
- Runtime, provider, renderer, and app layers consume activated projections or
  activation reports; they do not decide whether authored definitions are
  schema-compatible, migratable, or diffable.

No new ADR is required for the first PM-009 design because ADR-0004 separates
description from execution, ADR-0005 treats projections as derived state,
ADR-0001 keeps commands domain-owned, and ADR-0012 keeps host capability policy
explicit. A future ADR or accepted design update is required before generic UI
persistence owns project storage, provider sessions, runtime state, renderer
handles, gameplay truth, or concrete editor command execution.

## Persistence Contract

Every persisted authored UI definition package includes:

- stable document id;
- schema version;
- definition category or target family;
- source package provenance;
- deterministic node, recipe, token, binding, fixture, and target-profile
  references;
- extension ownership metadata for editor/workbench or game-runtime vocabulary;
- compatible unknown-field buckets where preservation is allowed;
- source-map-capable paths for diagnostics and diffs.

Persistence serialization must be deterministic. Field order, id formatting,
child ordering, extension ordering, and unknown-field ordering must be stable
enough that identical authored definitions produce identical textual output.

## Migration Dry-Run Contract

Migration requests include:

- source document id;
- source schema version;
- target schema version;
- source package provenance;
- target profile when migration depends on profile compatibility;
- requested migration mode: inspect, dry-run, or activation preflight;
- unknown-field preservation policy;
- source location metadata when available.

Migration is always a dry-run before activation. It reports:

- original and target schema versions;
- changed fields;
- incompatible fields;
- dropped fields and the reason they cannot be preserved;
- preserved compatible unknown fields;
- source-mapped diagnostics;
- deterministic migrated document preview;
- reviewable textual diff descriptor.

The migration step must not mutate the active definition set by itself.
Unsupported schema versions, incompatible migrations, malformed stable ids,
unknown required fields, and target-profile-incompatible extensions fail before
activation.

## Diff Contract

Textual diff descriptors include:

- stable diff id;
- source document id;
- before and after schema versions;
- changed paths;
- before and after values when they are safe to display;
- source locations;
- ordering policy used for serialization;
- affected target profiles;
- activation impact;
- deterministic diff text or a diagnostic explaining why a diff cannot be
  produced.

Any edit, migration, or formatting path that cannot produce a deterministic
reviewable textual diff remains preview-only and cannot activate.

## Activation Gate Contract

Activation requests include:

- document id;
- target profile;
- validation mode: dry-run, activate, or rollback preflight;
- migration report reference;
- diff descriptor reference;
- expected diagnostics;
- required capability evidence;
- preview fixture evidence references when available;
- source package provenance.

Activation decisions are fail-closed. A definition can activate only when:

- schema and version gates pass;
- required migrations are supported;
- incompatible fields are absent or explicitly blocked;
- compatible unknown fields are preserved where policy allows;
- deterministic textual diff output exists;
- Canonical UI IR validation passes;
- target-profile compatibility passes;
- binding, recipe, token, fixture, capability, and preview-only guards pass;
- expected diagnostics match actual diagnostics;
- no blocking diagnostics remain.

Activation may produce a report consumed by apps, runtimes, and preview
orchestration, but the report is an observation artifact. It must not execute
commands, write project files, mutate gameplay state, or create runtime
resources.

## Diagnostics

Persistence, migration, diff, and activation diagnostics include:

- stable diagnostic code;
- severity;
- source location when available;
- document id;
- source and target schema versions when relevant;
- affected target profile;
- changed path or incompatible path;
- owning domain;
- source package;
- expected and actual diagnostic references;
- activation impact;
- suggested fix.

The first taxonomy covers unsupported schema versions, unknown required fields,
malformed stable ids, incompatible migrations, unpreservable unknown fields,
non-deterministic serialization, non-deterministic diff output, missing
migration reports, missing diff descriptors, target-profile incompatibility,
expected diagnostic mismatches, and preview-only activation attempts.

## Implementation Activation

This accepted design does not select or activate an implementation slice.

The next legal implementation action is to create or select one bounded owning
GitHub issue. That issue should cover only the first generic
`domain/ui/ui_definition` persistence, migration dry-run, deterministic diff,
activation request, activation decision, and diagnostic contract slice.

The first slice must not implement app-hosted project save/load UI, user-facing
diff review UI, runtime activation plumbing, provider session orchestration,
gameplay mutation, renderer resources, screenshot capture, or production
readiness.

Historical `PT-*`, `PM-*`, and `WR-*` labels may remain as decomposition or
provenance vocabulary; they do not grant current implementation authority.

## Required Fitness Functions

The first implementation slice must add focused validation for:

- current-version documents passing migration without diagnostics;
- unsupported schema versions blocking activation;
- incompatible migration diagnostics;
- compatible unknown field preservation where policy allows;
- unpreservable unknown field diagnostics;
- deterministic textual serialization or diff descriptor output;
- preview-only or non-diffable edits rejecting activation;
- activation requiring a successful migration report and diff descriptor;
- target-profile compatibility diagnostics before activation;
- expected diagnostic mismatch diagnostics;
- editor/workbench and game-runtime examples without sharing project IO,
  provider, runtime, renderer, or gameplay ownership.

## Non-Goals

PM-009 design acceptance does not:

- implement app-hosted persistence UI, save/load UI, or diff review UI;
- implement concrete project file IO in `apps/runenwerk_editor`;
- implement runtime activation plumbing;
- implement provider session orchestration;
- implement screenshot capture, visual regression, or renderer golden evidence;
- implement accessibility/performance reporting from `PM-UI-DESIGN-010`;
- move editor, gameplay, render, material, scene, asset, simulation, save-game,
  network, project, provider, app, runtime, or screenshot truth into generic UI
  persistence contracts.

## Acceptance Bar

PM-009 remains an accepted design when:

- this accepted design exists and is discoverable from current UI design
  navigation;
- the canonical UI roadmap carries any durable sequence that remains relevant;
- implementation is activated only by an owning GitHub issue;
- delivery evidence belongs to the reviewed pull request;
- current repository validation passes at the accepted revision.
