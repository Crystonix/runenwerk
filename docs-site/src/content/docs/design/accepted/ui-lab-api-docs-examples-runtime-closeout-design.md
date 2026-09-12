---
title: UI Lab API Docs Examples Runtime Closeout Design
description: Accepted design for PM-UI-LAB-007 public API ergonomics, usage docs, examples, final runtime-proven closeout, and perfectionist-audit handoff.
status: accepted
owner: editor
layer: domain/app/docs
canonical: true
last_reviewed: 2026-09-12
related:
  - ../implemented/ui-lab-productization-design.md
  - ./ui-lab-preview-lab-runtime-evidence-design.md
  - ./ui-lab-persistence-project-io-diff-apply-rollback-design.md
  - ./ui-designer-production-readiness-and-evidence-design.md
---

# UI Lab API Docs Examples Runtime Closeout Design

## Status

Accepted for `PM-UI-LAB-007`.

This design is the design gate for the final Editor Lab V1 productization
milestone. It does not implement public API changes, docs, examples, final
closeout, or the later no-gap audit intake.

## Problem

`PM-UI-LAB-001` through `PM-UI-LAB-006` now provide governance, command and
registry source truth, app-hosted Editor Lab surfaces, typed operations,
project IO, diff/apply, rollback, preview scenarios, runtime artifacts,
accessibility snapshots, performance snapshots, and unsupported-check
diagnostics.

The track still cannot close as a product without the normal user path being
easy to discover:

- `domain/ui/ui_definition/src/lib.rs` publicly glob-exports every module as
  the primary API surface.
- `domain/editor/editor_definition/src/lib.rs` also glob-exports every module,
  including advanced and low-level contracts.
- Normal workflows for authoring, validation, preview, visual operations,
  diff/apply, persistence, diagnostics, and Editor Lab runtime evidence are
  spread across accepted designs, tests, and closeout artifacts.
- No final PT-UI-LAB closeout aggregates PM001-PM006 evidence and states the
  remaining PM007/perfectionist gaps truthfully.

Public API ergonomics are part of product quality in this repository, not
optional polish.

## Current Code Truth

Useful foundations already exist:

- `domain/ui/ui_definition` owns behavior-free authored UI definitions,
  validation, normalization, visual layout operations, preview fixtures,
  persistence activation, production readiness, diagnostics, and retained UI
  structures.
- `domain/editor/editor_definition` owns runtime-neutral editor definitions and
  typed Editor Lab operations.
- `apps/runenwerk_editor` owns project IO, apply/reject, activation reports,
  rollback, preview scenario execution, artifact writing, provider sessions,
  accessibility snapshots, performance snapshots, and runtime closeout
  evidence.

Missing public-product truth:

- no focused prelude or usage modules that make the normal `ui_definition`
  authoring path obvious;
- no focused prelude or usage modules that make the normal `editor_definition`
  operation/validation path obvious;
- no examples that compile or run through the preferred public path;
- no docs index pages that teach normal workflows before architecture details;
- no public API ergonomics review artifact;
- no final runtime-proven PT-UI-LAB closeout that links PM001-PM006 evidence
  and creates the later no-gap audit intake.

## Architecture Decision

Create focused public workflow entry points and documentation without moving
runtime behavior into definition crates.

Source-truth ownership:

- `domain/ui/ui_definition` owns behavior-free UI authoring, validation,
  normalization, visual layout operation, preview fixture, persistence
  activation, readiness, and diagnostics contracts.
- `domain/editor/editor_definition` owns runtime-neutral editor definition
  documents, validation, and Editor Lab operation contracts.
- `apps/runenwerk_editor` owns Editor Lab project IO, live activation, provider
  sessions, rollback, preview evidence execution, artifact storage, and final
  app runtime evidence.
- Docs and examples are projections over those owning contracts. They must not
  invent hidden shortcuts or teach internal-only APIs as the normal path.

The implementation should prefer focused `prelude` or `usage` modules over
more glob-exported internals. Existing glob exports may remain for
compatibility, but normal docs and examples must point at the focused workflow
exports first.

## Public Workflow Contracts

`ui_definition` normal workflow
: Author a template, validate and normalize it, apply visual layout operations,
  inspect diagnostics/diffs, run preview-fixture or persistence-activation
  checks, and produce behavior-free readiness evidence.

`editor_definition` normal workflow
: Build editor definition documents, validate them, apply `EditorLabOperation`
  values over runtime-neutral documents, inspect operation reports/diffs, and
  hand the resulting documents to app-owned Editor Lab project/apply paths.

`Editor Lab app workflow`
: Save/load a project package, build/reject/apply a definition review, inspect
  activation reports, rollback, run preview evidence scenarios, and close with
  artifact-backed runtime proof. This remains app-owned.

`PT-UI-LAB final closeout`
: Aggregate PM001-PM006 evidence, list validation, identify remaining gaps,
  and create a separate intake for `PT-UI-LAB-PERFECTION` or equivalent
  no-gap audit. The final closeout may claim `runtime_proven`; it must not
  claim `perfectionist_verified`.

## Required Deliverables

The implementation WR for PM007 should provide:

- focused public entry points for common `ui_definition` workflows;
- focused public entry points for common `editor_definition` workflows;
- usage docs for normal UI definition authoring, validation, preview,
  persistence activation, visual operations, diagnostics, and readiness;
- usage docs for normal editor definition documents, operations, validation,
  and app handoff;
- examples that compile or run through public APIs rather than test-only
  shortcuts;
- a public API ergonomics review artifact that checks discoverability from
  `lib.rs`, prelude/usage modules, docs index pages, examples, and closeouts;
- final PT-UI-LAB runtime-proven closeout that links PM001-PM006 evidence and
  PM007 docs/examples/API review evidence;
- roadmap intake for the later perfectionist/no-gap audit track.

## Validation

Implementation WR validation must include:

- focused Rust tests for any new prelude/usage module exports;
- example compile/run checks where examples are added;
- docs validation;
- public API ergonomics review evidence;
- roadmap render/validate/check after roadmap intake or archive edits;
- production render/validate/check after track closeout metadata edits;
- final `task ai:goal -- --track PT-UI-LAB --scope non-deferred` showing no
  remaining non-deferred PT-UI-LAB milestones.

Expected commands include:

```text
cargo fmt
cargo test -p ui_definition
cargo test -p editor_definition
cargo test -p runenwerk_editor editor_lab
cargo test -p runenwerk_editor pm_ui_lab_006
task docs:validate
task puml:validate
task roadmap:render
task roadmap:validate
task roadmap:check
task production:render
task production:validate
task production:check
task planning:validate
git diff --check
task ai:goal -- --track PT-UI-LAB --scope non-deferred
```

## Non-Goals

- No runtime behavior, project IO, activation execution, rollback, provider
  sessions, scenario execution, screenshot capture, accessibility tooling,
  performance runners, or artifact writing in `domain/ui/ui_definition`.
- No game-runtime UI projection execution.
- No no-gap or `perfectionist_verified` claim.
- No broad rewrite of all public APIs before the normal workflow path is fixed.
- No hiding native screenshot/GPU visual-diff/accessibility/performance gaps
  that PM006 recorded as unsupported-check diagnostics.

## Stop Conditions

Stop before implementation if:

- normal examples require private or test-only APIs;
- public workflow entry points would move app/runtime behavior into definition
  crates;
- final closeout would have to over-claim from docs-only or descriptor-only
  evidence;
- examples and docs disagree with accepted ownership boundaries;
- the later perfectionist audit cannot be represented as a separate roadmap or
  production intake item;
- a durable dependency-direction change is required without a new accepted
  design or ADR.
