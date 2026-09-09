---
title: Foundation Ratification Design
description: Reusable candidate acceptance-report vocabulary for generated, projected, imported, migrated, or externally supplied state.
status: implemented
owner: foundation
layer: foundation
canonical: true
last_reviewed: 2026-09-09
related_adrs:
  - ../../adr/accepted/0003-ratification-is-domain-specific.md
---

# `foundation/ratification` Design

## Implementation status

This design is implemented and has been checked against the current `foundation/ratification` public surface and crate documentation. The crate owns `RatificationStatus`, `RatificationSeverity`, `RatificationIssue`, `RatificationReport`, `Ratifier`, and the optional diagnostics bridge described here while preserving the foundation/domain/editor ownership split.

Some API spellings in the older design sections below were explicitly directional rather than exact commitments. Current code and `foundation/ratification/README.md` own the exact API. In particular, the landed `Ratifier<Candidate>` uses `Code` and `Subject` associated types without a separate `Context` associated type, and the landed report helpers use names such as `from_issue`, `from_issues`, `push`, `has_blocking_issues`, and `is_fatal` rather than every candidate helper name listed in the original proposal. These are naming/surface refinements, not ownership or semantic divergence.

## Purpose

`foundation/ratification` exists to provide Runenwerk with a small, reusable vocabulary for deciding whether a candidate value may be accepted by its owning domain.

A candidate may be generated, projected, imported, migrated, externally supplied, or derived from another authoritative state.

The crate standardizes the shape of acceptance reports. It does not own domain-specific validity rules and it does not replace editor/runtime governance.

The long-term goal is to let every domain answer this question consistently:

```text
Can this candidate be accepted, and what issues explain that decision?
```

## Scope

This crate owns low-level acceptance-report vocabulary.

In scope:

- ratification status;
- ratification issue containers;
- issue severity or diagnostics severity mapping;
- ratification reports;
- ratifier trait shape;
- report aggregation helpers;
- acceptance helpers;
- conversion hooks to diagnostics;
- optional test helpers for asserting acceptance/rejection.

The crate must support domain-specific issue codes and subjects without knowing those domain types.

## Non-Scope

This crate explicitly does not own:

- scene-specific validation rules;
- UI surface mount rules;
- editor-shell routing rules;
- render graph validation rules;
- asset pipeline validation rules;
- scheduler validation rules;
- command execution;
- command routing;
- transaction history;
- undo/redo;
- `editor_core::RatifiedChange` replacement;
- reconciliation policy;
- authority model;
- causality IDs;
- propagation structure;
- shared-session outboxes;
- runtime scheduling;
- logging/tracing;
- automatic repair;
- migration execution.

Those concerns belong to domain crates, engine/runtime crates, apps, adapters, or `editor_core` governance.

## Architectural Position

Layer:

```text
foundation
```

Recommended crate path:

```text
foundation/ratification
```

Recommended package name:

```text
ratification
```

Allowed dependencies:

```text
core
alloc optional
foundation/diagnostics optional or direct, depending on final diagnostics dependency policy
serde optional, feature-gated
```

Forbidden dependencies:

```text
domain/*
engine
apps/*
adapters/*
net/*
editor_core
ui_surface
scene
scheduler
```

Dependency direction must remain downward/stable. Domain ratifiers consume foundation ratification; foundation ratification must not know the domains that consume it.

## Ownership Rules

`foundation/ratification` owns generic report mechanics.

It creates:

- `RatificationStatus`;
- generic `RatificationIssue` containers;
- generic `RatificationReport` containers;
- ratifier trait vocabulary;
- report aggregation helpers;
- acceptance/rejection helper methods.

It does not create domain-specific ratification codes. Domain crates create their own issue code enums or code constants.

It does not create domain-specific subjects. Domain crates create their own subject enums or subject descriptors.

It does not mutate domain state. A ratifier observes a candidate and reports whether the candidate should be accepted.

It does not apply commands. Command execution belongs to domain command systems or higher-level governance crates.

It is consumed by:

- domain ratifiers;
- import validators;
- projection validators;
- migration validators;
- editor/runtime governance wrappers;
- tests;
- tooling;
- future AI-assisted workflows.

## Public API Policy

Stable public contracts should include:

- `RatificationStatus`;
- `RatificationIssue<Code, Subject>` or equivalent generic issue type;
- `RatificationReport<Code, Subject>` or equivalent generic report type;
- `Ratifier<Candidate>` trait if a shared trait proves useful;
- report aggregation helpers;
- diagnostic conversion helpers if diagnostics integration is enabled.

Internal or unstable details should include:

- allocation strategy;
- formatting internals;
- optional serde implementation details;
- optional test helper internals;
- builder internals if builders are added.

Public API rules:

- Keep codes and subjects generic.
- Do not add a global enum of all ratification codes.
- Do not add editor governance fields to low-level reports.
- Do not require command metadata to create a ratification report.
- Do not require transaction metadata to create a ratification report.
- Do not make the first version macro-heavy.

## Invariants

The following invariants must always hold:

- A report must have a determinable acceptance status.
- Rejected reports must contain at least one blocking issue unless created by an explicit low-level failure path.
- Accepted reports must not contain blocking errors.
- Accepted-with-warnings reports must contain warnings and no blocking errors.
- Fatal reports must be distinguishable from ordinary rejection if `Fatal` is included.
- Issue codes and subjects must be domain-owned or caller-provided.
- Foundation ratification must not know domain-specific validity rules.
- Ratification must not mutate the candidate being ratified.
- Ratification must not execute commands.
- Ratification must not imply editor history, undo/redo, sharing, or reconciliation semantics.

## Failure Modes

Possible failure modes:

- candidate is rejected because of one or more domain issues;
- candidate is accepted with warnings;
- candidate cannot be safely evaluated;
- context required for ratification is missing;
- a ratifier implementation emits inconsistent status/issue combinations;
- diagnostic conversion fails because subject/code formatting is unavailable.

The crate should prefer explicit report status over panics.

Panics should be limited to test helpers or debug-only invariant assertions.

Domain-specific failure meaning must be expressed through domain-owned issue codes and subjects.

## Diagnostics

This crate should integrate with `foundation/diagnostics`, but should not replace diagnostics.

Recommended relationship:

```text
foundation/ratification
  may convert ratification issues into diagnostics

foundation/diagnostics
  owns diagnostic vocabulary
```

Possible diagnostic family owned by this crate:

```text
foundation.ratification.*
```

Possible codes:

```text
foundation.ratification.report.inconsistent_status
foundation.ratification.report.rejected_without_issue
foundation.ratification.issue.missing_subject
foundation.ratification.issue.missing_code
foundation.ratification.context.missing
```

Domain-specific codes remain owned by domain crates:

```text
ui_surface.mount.unknown_host
editor_shell.route.stale_projection_epoch
scene.hierarchy.cycle
render_graph.resource.unbound
```

## Ratification

This crate defines the ratification vocabulary but does not ratify any concrete domain by itself.

It should support ratification of:

- generated state;
- projected state;
- imported state;
- migrated state;
- externally supplied state;
- derived plans;
- command candidates before execution, where the owning domain chooses to model command validation as ratification.

Examples of domain ratifiers:

```text
UiSurfaceRatifier
EditorShellRatifier
SceneRatifier
RenderGraphRatifier
AssetGraphRatifier
WorkspaceRatifier
```

Each domain ratifier owns:

- candidate type;
- context type;
- issue code type;
- subject type;
- validity rules;
- recovery recommendations if any.

`foundation/ratification` owns only the shared report shape and helper vocabulary.

## Commands

This crate owns no command execution boundary.

It may be used by command systems to validate command candidates, but it must not execute, route, store, undo, redo, or reconcile commands.

Correct split:

```text
foundation/ratification:
  Can this command candidate or resulting state be accepted?

domain command system:
  How is the command applied to domain state?

editor_core governance:
  How does an accepted editor-authoring change enter history, undo/redo, sharing, and reconciliation?
```

The crate must not define one universal command enum.

The crate must not require command metadata for ordinary ratification reports.

## Inspection

Reports must be read-only and inspection-friendly.

Consumers should be able to inspect:

- report status;
- issue count;
- issue severity;
- issue code;
- issue subject;
- issue message;
- optional context/notes;
- whether the report blocks acceptance.

Inspection consumers include:

- tests;
- debug tooling;
- editor panels;
- CLI tools;
- future AI-assisted workflows.

Inspection must not expose mutable domain internals.

Ratification reports are snapshots of evaluation results, not handles to mutable candidates.

## Persistence and Versioning

Ratification reports are normally transient evaluation artifacts.

Rules:

- Reports should not be treated as authoritative persisted domain state.
- Reports may be serialized for tests, logs, debug captures, or tooling if a feature enables serialization.
- Serialized reports should include status, issue code, issue subject, severity, and message.
- Domain-specific issue code stability is the responsibility of the owning domain.
- `RatificationStatus` changes are crate-level public API changes.
- If reports become externally persisted, add an explicit report schema version.

This crate must not persist editor history or undo/redo state. That belongs to editor/runtime governance.

## Extension Points

Intended extension mechanisms:

- domain-owned code enums;
- domain-owned subject enums;
- domain-owned context types;
- report builders;
- aggregation helpers;
- conversion into diagnostics;
- optional serde support;
- optional test assertion helpers.

Extension mechanisms that should be avoided initially:

- global ratifier registry;
- global issue-code enum;
- macro-heavy ratifier generation;
- automatic repair system;
- editor transaction integration;
- command execution integration;
- runtime scheduler integration.

## Testing Strategy

Required tests:

- accepted report has accepted status and no blocking issues;
- accepted-with-warnings report has warnings and no blocking errors;
- rejected report has blocking issues;
- fatal report behavior is deterministic if `Fatal` exists;
- report aggregation preserves issue order;
- report aggregation computes final status correctly;
- generic code/subject types work without domain dependencies;
- optional diagnostics conversion preserves code/severity/subject/message;
- optional serde round-trip preserves report meaning;
- crate dependency graph does not pull in domain/editor/engine/app crates.

Recommended test categories:

```text
unit tests
invariant tests
diagnostic conversion tests
serialization feature tests
dependency-boundary checks
domain integration tests in consuming crates
```

Domain crates should test their own ratifiers. This crate should not test domain-specific validity rules.

## Negative Doctrine

This crate is not a domain validator.

This crate is not a scene ratifier.

This crate is not a UI surface ratifier.

This crate is not an editor-shell ratifier.

This crate is not a render graph validator.

This crate is not an asset pipeline validator.

This crate is not a command executor.

This crate is not a transaction system.

This crate is not undo/redo.

This crate is not `editor_core::RatifiedChange`.

This crate is not a reconciliation system.

This crate is not a sharing/replication system.

This crate is not a logger.

## Current Repo Findings

The current repository is not greenfield:

- The workspace already has `foundation/id` as a true foundation crate.
- `editor_core` already owns rich editor-governance concepts including ratified changes, transaction metadata, command metadata, reality versions, authority scope, reconciliation policy, propagation structure, retention hints, reversibility, and causality.
- `apps/runenwerk_editor` already has a scene-authoring ratification ingress that produces `editor_core::RatifiedChange` for command, transaction, undo, and redo flows.
- `ui_surface` already defines a narrower `RatificationAdapter`, `RatificationOutcome`, and `RatificationDispatchError` for surface intent dispatch across host/domain boundaries.
- `editor_persistence` already validates normalized scene files through local error enums.

Therefore this foundation crate must not be designed as a replacement for `editor_core::RatifiedChange`. It should provide low-level acceptance-report vocabulary that domains can use before or below editor governance.

## Core Boundary

Foundation ratification answers:

```text
Was this candidate accepted, accepted with warnings, rejected, or fatally invalid?
Which issues explain that decision?
```

It does not answer:

```text
Who authored the change?
Which command caused it?
Which transaction owns it?
Can it be undone?
Should it be broadcast to another session?
How does it reconcile with another reality version?
```

That distinction is critical because `editor_core` already owns editor-authoring governance semantics.

## Relationship to Existing `editor_core` Governance

`editor_core::RatifiedChange` is not a low-level validation report. It is a governed editor change record.

It contains concepts such as:

```text
ratification id
transaction metadata
causality id
change origin
authority scope
affected domains
affected scopes
base/result reality versions
command metadata
semantic operations
ratification class
reversibility class
retention hint
stability class
reconciliation policy
propagation structure
migration path
timestamp
```

Those concepts should remain in `editor_core` for now.

The correct split is:

```text
foundation/ratification:
  Can this candidate be accepted by its owning domain?

editor_core:
  How does an accepted editor-authoring change enter history, undo/redo, sharing, and reconciliation?
```

Do not migrate `RatifiedChange`, `TransactionMetadata`, `CommandMetadata`, undo/redo, or reconciliation into foundation as part of this crate.

## Relationship to Existing `ui_surface` Ratification

`ui_surface` currently uses ratification terminology for intent dispatch:

```text
RatificationAdapter
RatificationOutcome
RatificationDispatchError
ratify_surface_intent
```

This is a domain-level host boundary, not necessarily the same as universal candidate validation.

A future migration may either:

1. keep `ui_surface` intent ratification as-is and use foundation ratification only for mount-plan/state validation;
2. rename `ui_surface` intent ratification to avoid overloaded terminology;
3. adapt `ui_surface` ratification results into foundation `RatificationReport` where appropriate.

Recommended path:

```text
Do not rewrite ui_surface intent dispatch first.
Use foundation/ratification for validation/reporting of candidates, not command dispatch.
```

## Primary Type Model

Recommended initial public vocabulary:

```text
RatificationStatus
RatificationSeverity or diagnostics::Severity
RatificationIssue<Code, Subject>
RatificationReport<Code, Subject>
Ratifier<Candidate>
```

### `RatificationStatus`

Recommended variants:

```text
Accepted
AcceptedWithWarnings
Rejected
Fatal
```

Semantics:

- `Accepted`: no blocking issues.
- `AcceptedWithWarnings`: candidate may be accepted but warnings should be surfaced.
- `Rejected`: candidate must not be accepted.
- `Fatal`: candidate cannot be safely evaluated or continuing would be unsafe.

`Fatal` may be omitted initially if `Rejected` is enough, but the design should leave room for it.

### `RatificationIssue`

A domain-owned issue inside a report.

Recommended generic shape:

```text
RatificationIssue<Code, Subject> {
  severity,
  code,
  subject,
  message
}
```

`Code` and `Subject` must be generic so foundation does not know domain-specific enums.

Examples of domain code enums:

```text
UiSurfaceRatificationCode::UnknownSurfaceHost
EditorShellRatificationCode::StaleProjectionEpoch
SceneRatificationCode::HierarchyCycle
RenderGraphRatificationCode::UnboundResource
```

Examples of domain subject enums:

```text
UiSurfaceSubject::SurfaceInstance(SurfaceInstanceId)
EditorShellSubject::CommandRoute(CommandRouteId)
SceneSubject::Entity(EntityId)
RenderGraphSubject::Resource(ResourceId)
```

### `RatificationReport`

A collection of issues and status helpers.

Recommended generic shape:

```text
RatificationReport<Code, Subject>
```

Useful helpers:

```text
accepted
accepted_with_warnings
rejected
fatal
is_accepted
is_rejected
has_warnings
has_errors
push_issue
merge
highest_severity
```

### `Ratifier`

A shared trait is useful if it does not over-constrain domains.

Recommended shape:

```text
Ratifier<Candidate>
```

With associated types:

```text
Context
Code
Subject
```

The trait should not require mutable access to the candidate.

The trait should not require command metadata, transactions, or editor runtime context.

## Integration Plan

Recommended first consumers:

1. `ui_surface` mount-plan or state validation.
2. `editor_shell` projection/route validation.
3. `editor_persistence` normalized scene validation reporting.
4. render graph validation once the render graph boundary is stable.

Do not begin by replacing `editor_core` governance.

Do not begin by rewriting all validation errors in the workspace.

## Migration Plan

Phase 1:

```text
Create foundation/ratification with generic report types and tests.
```

Phase 2:

```text
Add a small ui_surface ratifier for one concrete candidate type.
```

Phase 3:

```text
Add editor_shell projection/route ratification using foundation reports.
```

Phase 4:

```text
Bridge reports into foundation/diagnostics for UI/debug/tooling display.
```

Phase 5:

```text
Evaluate whether editor_core governance wants optional precondition reports, without moving RatifiedChange into foundation.
```


## Phase 5 Evaluation Outcome

Phase 5 is closed by [Foundation Ratification Phase 5 Evaluation](../../reports/closeouts/foundation-ratification-phase5-evaluation.md).

Outcome:

```text
Do not move RatifiedChange into foundation.
Do not replace GoverningChangeError.
Do not change editor_core governance yet.
Keep foundation/ratification as shared report vocabulary.
Complete diagnostics hygiene before starting schema.
```
