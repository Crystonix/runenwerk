---
title: UI Lab Preview Lab Runtime Evidence Design
description: Accepted design for PM-UI-LAB-006 preview scenarios, visual/equivalent evidence, diagnostics snapshots, accessibility checks, performance evidence, degraded-provider states, and runtime-proof closeout artifacts.
status: accepted
owner: editor
layer: app/runtime-evidence
canonical: true
last_reviewed: 2026-09-12
related:
  - ../implemented/ui-lab-productization-design.md
  - ./ui-lab-persistence-project-io-diff-apply-rollback-design.md
  - ./ui-lab-operation-driven-visual-authoring-design.md
  - ./ui-designer-live-preview-fixtures-scenarios-and-target-matrix-design.md
  - ./ui-designer-production-readiness-and-evidence-design.md
---

# UI Lab Preview Lab Runtime Evidence Design

## Status

Accepted for `PM-UI-LAB-006`.

This design is the design gate for the Preview Lab and runtime evidence
hardening milestone. It does not implement the harness or close the milestone.

## Problem

`PM-UI-LAB-002` through `PM-UI-LAB-005` prove command/registry truth,
app-hosted Editor Lab surfaces, typed operations, project IO, diff/apply,
activation reports, failed activation preservation, reload, and rollback. Their
runtime artifacts are intentionally narrow and mostly headless retained-surface
or report artifacts.

`PT-UI-LAB` still cannot claim broad runtime proof because the Editor Lab lacks
a governed Preview Lab scenario matrix that captures visible states,
diagnostic snapshots, degraded-provider behavior, accessibility checks, and
performance evidence consistently. Without this milestone, later closeout would
either over-claim from narrow PM002-PM005 artifacts or scatter evidence capture
across ad hoc tests.

## Current Code Truth

The current app already has useful evidence ingredients:

- `apps/runenwerk_editor/src/shell/tests.rs` contains PM002-PM005 runtime proof
  tests that open or form app shell state and write closeout artifacts when a
  milestone-specific environment variable is set.
- `apps/runenwerk_editor/src/shell/providers/self_authoring.rs` projects Editor
  Lab hierarchy, palette, inspector, review, diagnostics, preview console,
  operation, package, activation, and rollback state into typed view models.
- `domain/editor/editor_shell/src/composition/build_editor_lab_surface.rs`
  forms retained UI for Editor Lab view models and can produce equivalent
  visual/debug artifacts without a native window.
- `apps/runenwerk_editor/src/runtime/resources.rs` owns live activation and now
  records typed activation reports for success, no-live-activation, and failed
  previous-state-preserved paths.
- `apps/runenwerk_editor/src/shell/editor_lab_project/mod.rs` owns PM005
  package, apply-review, activation-report, and rollback contracts consumed by
  evidence scenarios.

Missing runtime truth:

- no single Editor Lab preview scenario catalog;
- no common evidence run/report shape for success, warning, error, reload,
  apply, rollback, and degraded-provider states;
- no governed rule for when screenshot capture is required versus when retained
  visual/equivalent artifacts are acceptable;
- no accessibility or performance report boundary;
- no artifact manifest that connects scenario inputs, runtime outputs,
  diagnostics, visual artifacts, and unsupported-check diagnostics.

## Architecture Decision

Create an app-owned Preview Lab evidence harness for Editor Lab V1.

Source-truth ownership:

- `apps/runenwerk_editor` owns scenario execution, provider mounting, retained
  surface formation, optional native screenshot capture, artifact writing,
  accessibility checks, performance measurements, unsupported-check
  diagnostics, and closeout manifests.
- `domain/editor/editor_shell` owns typed surface view models and retained UI
  composition contracts only. It does not own app runtime sessions, artifact
  storage, screenshot capture, or performance runners.
- `domain/editor/editor_definition` may own runtime-neutral editor scenario
  descriptors only if they do not mention file paths, app sessions, providers,
  renderer handles, screenshots, or artifact storage.
- `domain/ui/ui_definition` remains behavior-free. It may provide generic UI
  diagnostics, projection descriptors, and retained UI structures, but it must
  not execute app scenarios, own screenshot truth, run accessibility tooling,
  write artifacts, or measure runtime performance.

The harness should emit a durable `EditorLabEvidenceRun` report that references
artifact paths rather than embedding large screenshots or debug dumps into
source-truth descriptors.

## Contracts

`EditorLabPreviewScenario`
: App-owned scenario definition containing id, label, target surface/profile,
  setup operation, expected state family, expected diagnostics, expected
  provider availability, capture requirements, accessibility requirements, and
  performance budget.

`EditorLabEvidenceRun`
: App-owned report for one run containing scenario id, source package/review
  references, app/runtime state summary, provider state, diagnostics snapshot,
  accessibility snapshot, performance snapshot, artifact references, and
  unsupported-check diagnostics.

`EditorLabEvidenceArtifact`
: Stable reference to a generated artifact. Artifact kinds include retained UI
  debug text, provider view-model snapshot, diagnostics JSON/RON, activation
  report, project package, screenshot, visual diff result, accessibility
  report, performance report, and unsupported-check report.

`EditorLabAccessibilitySnapshot`
: App-owned report for checks the current runtime can perform, such as
  accessible labels, disabled reasons, focusable command controls, contrast
  token inspection where available, and unsupported checks recorded as typed
  diagnostics.

`EditorLabPerformanceSnapshot`
: App-owned report for supported timing or size evidence, such as scenario
  setup duration, retained surface formation duration, artifact count/bytes,
  and optional native capture timing. Unsupported GPU/window timing must be a
  typed diagnostic, not a silent pass.

`EditorLabEvidenceManifest`
: Closeout manifest that lists every scenario, expected state, executed command
  or operation path, result status, artifact paths, unsupported checks, and
  remaining known gaps.

## Scenario Matrix

The first implementation WR should prove Editor Lab V1 scenarios across these
state families:

- success: baseline Editor Lab opens, surfaces mount, retained preview or
  equivalent visual artifact is non-empty;
- warning: non-blocking diagnostics are visible in diagnostics/review output;
- error: invalid project package or invalid editor definition produces typed
  diagnostics and preserved input;
- reload: saved project package reloads without live activation;
- apply: accepted apply produces review, queued activation, runtime activation
  report, and provider-surface visibility;
- rollback: snapshot-backed rollback records a typed rollback report and
  provider-surface visibility;
- degraded-provider: missing/unsupported provider state renders a typed
  degraded surface with recovery actions;
- accessibility: enabled and disabled controls expose labels and disabled
  reasons; unsupported accessibility checks are explicit diagnostics;
- performance: supported formation/capture timings and artifact sizes are
  recorded; unsupported performance checks are explicit diagnostics.

Scenario coverage may start with the editor/workbench target profile. Game
runtime UI projection remains deferred until Editor Lab V1 is runtime-proven.

## Evidence Rules

Runtime evidence is acceptable only when it is generated by the app/runtime
path being claimed. Descriptor-only, docs-only, static-fixture-only, or
status-panel-only evidence is not enough.

Visual proof must follow this priority:

1. Native screenshot capture when the current runtime and environment support
   it reliably.
2. Retained UI artifact plus provider view-model snapshot when native capture
   is unavailable in the current headless test environment.
3. Unsupported-check diagnostic when neither visual path is available.

The closeout must not hide unsupported capture, accessibility, or performance
checks. Unsupported evidence is acceptable only when it is typed, scenario
linked, and listed as a known gap or environment limitation.

## Validation

Implementation WR validation must include:

- focused unit tests for scenario descriptors, manifests, diagnostics, and
  unsupported-check reports;
- integration tests that open the Editor Lab, mount provider surfaces, execute
  success/warning/error/reload/apply/rollback/degraded scenarios, and write
  evidence artifacts;
- artifact-writing runtime test, for example
  `$env:RUNENWERK_WRITE_PM_UI_LAB_006_EVIDENCE='1'; cargo test -p runenwerk_editor pm_ui_lab_006_runtime_evidence_reports_preview_lab -- --nocapture`;
- `task docs:validate`;
- `task puml:validate`;
- `task roadmap:render`;
- `task roadmap:validate`;
- `task roadmap:check`;
- `task production:render`;
- `task production:validate`;
- `task production:check`;
- `task planning:validate`;
- `git diff --check`.

## Roadmap Intake Shape

The preferred next WR is one bounded implementation row:

- title: `UI Lab preview lab runtime evidence matrix`;
- primary scope: `apps/runenwerk_editor/src/shell`, `apps/runenwerk_editor/src/runtime`,
  `domain/editor/editor_shell/src/composition`, PM006 closeout artifacts, and
  roadmap/production metadata;
- dependencies: completed `WR-097`;
- closeout: `docs-site/src/content/docs/reports/closeouts/pm-ui-lab-006-preview-lab-and-runtime-evidence/closeout.md`;
- runtime evidence: success, warning, error, reload, apply, rollback,
  degraded-provider, accessibility, performance, unsupported-check, and visual
  or equivalent artifacts.

Split the WR before implementation if native screenshot capture, accessibility
checks, and performance evidence cannot stay bounded in one app-owned harness.

## Non-Goals

- No public API ergonomics review, usage guide, examples, or final PT-UI-LAB
  closeout; those remain `PM-UI-LAB-007`.
- No game-runtime UI projection execution.
- No no-gap or `perfectionist_verified` claim.
- No moving app runtime evidence, screenshot capture, accessibility tooling,
  performance measurement, provider sessions, or artifact writing into
  `domain/ui/ui_definition`.
- No treating unsupported screenshot/accessibility/performance checks as
  passing evidence.

## Stop Conditions

Stop before implementation if:

- PM006 would require `ui_definition` to execute app/runtime/provider behavior;
- evidence would be descriptor-only, docs-only, status-panel-only, or raw
  console-only;
- failed scenarios would not preserve input, diagnostics, or artifact links;
- native screenshot capture is required but cannot be made deterministic and no
  equivalent retained visual artifact is accepted by the implementation
  contract;
- accessibility or performance checks are claimed without actual reports or
  typed unsupported diagnostics;
- the implementation needs a durable ownership or dependency-direction change
  not covered by this design or an accepted ADR.
