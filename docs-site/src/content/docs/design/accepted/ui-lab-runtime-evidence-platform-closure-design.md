---
title: UI Lab Runtime Evidence Platform Closure Design
description: Accepted design for PM-UI-LAB-PERF-002 native-or-typed-impossible Editor Lab runtime evidence closure.
status: accepted
owner: editor
layer: app/runtime-evidence
canonical: true
last_reviewed: 2026-09-12
related_designs:
  - ./ui-lab-perfectionist-audit-design.md
  - ./ui-lab-preview-lab-runtime-evidence-design.md
  - ./ui-lab-persistence-project-io-diff-apply-rollback-design.md
  - ../implemented/ui-lab-productization-design.md
related_reports:
  - ../../reports/closeouts/pm-ui-lab-006-preview-lab-and-runtime-evidence/closeout.md
  - ../../reports/closeouts/pm-ui-lab-perf-001-governance-audit-doctrine-and-code-truth-matrix/closeout.md
related_roadmaps:
  - ../../domain/ui/roadmap.md
---

# UI Lab Runtime Evidence Platform Closure Design

## Status

Accepted for `PM-UI-LAB-PERF-002`.

This design clears the design gate for the runtime evidence platform closure
only. It does not authorize product code by itself. Any implementation requires
an owning GitHub issue, canonical roadmap sequencing where relevant, and a pull
request that owns the reviewed feature head and exact-head validation evidence.

## Goal

Editor Lab V1 no-gap certification needs evidence results that are either
native runtime artifacts or explicit typed platform-impossible diagnostics.

The PM006 Preview Lab already records retained visual artifacts, diagnostics,
accessibility snapshots, performance snapshots, unsupported checks, reload,
apply, rollback, and degraded-provider proof. PM002 of the perfectionist
sequence does not replace that harness. It tightens the evidence platform so
every unsupported PM006 check is re-evaluated through a capability probe and
recorded as one of:

- captured native/runtime artifact;
- typed platform-impossible diagnostic with backend, environment, and reason;
- blocking failure.

## Current Code Truth

Current PM006 code and artifacts provide a useful base:

- `apps/runenwerk_editor/src/shell/editor_lab_evidence/mod.rs` defines
  `EditorLabPreviewScenario`, `EditorLabEvidenceRun`,
  `EditorLabEvidenceArtifact`, accessibility/performance snapshots,
  unsupported-check diagnostics, and manifest validation.
- `apps/runenwerk_editor/src/shell/tests.rs` test
  `pm_ui_lab_006_runtime_evidence_reports_preview_lab` executes app-hosted
  Editor Lab paths and writes closeout artifacts when
  `RUNENWERK_WRITE_PM_UI_LAB_006_EVIDENCE=1`.
- PM006 artifacts record unsupported diagnostics for
  `native_screenshot_capture`, `native_focus_traversal`,
  `pixel_contrast_sampling`, `native_screenshot_timing`, and
  `gpu_visual_diff_timing`.

No-gap blockers:

- `EditorLabEvidenceArtifactKind` does not yet distinguish native screenshot,
  GPU visual diff, focus traversal, contrast sample, or timing artifacts from
  retained/debug reports.
- Unsupported checks are free-form diagnostics, not the output of a typed
  capability probe.
- Manifest validation rejects descriptor-only evidence, but it does not yet
  require native-or-platform-impossible classification for no-gap targets.
- Focus, contrast, screenshot timing, and GPU visual-diff timing are recorded
  as unsupported in PM006 without a reusable platform-impossible result model.

## Architecture Governance

Governance result for PM002:

- DDD bounded context owner: `editor`.
- App owner: `apps/runenwerk_editor` owns runtime evidence execution,
  capability probes, artifact writing, native screenshot adapters,
  accessibility/focus inspection, contrast sampling, timing capture, and
  platform-impossible diagnostics.
- Supporting owner: `domain/editor/editor_shell` owns retained UI composition,
  route labels, disabled reasons, and app-neutral surface view models. It does
  not own evidence execution or artifact storage.
- Supporting owner: `domain/editor/editor_definition` remains runtime-neutral
  and may expose document ids or operation report data consumed by evidence.
- Supporting owner: `domain/ui/ui_definition` remains behavior-free. It may
  provide diagnostics and retained UI structures, but it must not execute
  scenarios, own native windows, write artifacts, or measure runtime
  performance.
- Renderer or windowing helpers may be consumed only through app-owned
  adapters. They must not become Editor Lab source truth.
- ADR need: no ADR is required while evidence execution stays app-owned and
  domain crates remain descriptor/report owners. Add an ADR or accepted design
  update before moving native capture, focus, contrast, timing, or visual diff
  ownership into a reusable cross-domain platform crate.
- ATAM-lite priority order: truthful evidence classification first,
  reproducibility second, user-visible runtime proof third, CI portability
  fourth, performance overhead fifth.
- Ownership mode: stream-aligned editor product work with
  complicated-subsystem support from app runtime/windowing and retained shell
  owners.

## Evidence Platform Contract

PM002 should extend the PM006 harness with typed no-gap evidence concepts.
Exact Rust names may change during implementation, but the contracts must keep
these semantics.

`EditorLabEvidenceCapability`
: Stable enum for no-gap evidence targets: native screenshot, GPU visual diff,
  native focus traversal, pixel contrast sampling, native screenshot timing,
  GPU visual-diff timing, diagnostics snapshot, degraded-provider snapshot,
  reload, apply, rollback, and failure-preservation proof.

`EditorLabEvidenceCapabilityProbe`
: App-owned probe result containing capability, backend/environment metadata,
  support status, and reason. This is the only source that may justify a
  platform-impossible result.

`EditorLabEvidenceResult`
: Typed result for one capability: `Captured`, `PlatformImpossible`, or
  `Failed`. Captured results include artifact paths and reproduction command.
  Platform-impossible results include probe data, backend, environment, and
  reviewer-facing reason. Failed results are blocking diagnostics.

`EditorLabEvidenceArtifactKind`
: Must distinguish retained UI debug from native screenshots, GPU visual diffs,
  focus traversal reports, contrast sample reports, timing reports,
  diagnostics snapshots, activation reports, rollback reports, and unsupported
  or platform-impossible reports.

`EditorLabEvidenceManifest`
: Must validate that every no-gap required capability has a captured or
  platform-impossible result. Free-form unsupported checks are not sufficient
  for PM002 closeout.

## Runtime Evidence Matrix

| Target | Captured evidence | Platform-impossible evidence | Blocking failure |
|---|---|---|---|
| Native screenshot | Window/runtime screenshot artifact with command, backend, scenario id, dimensions, and artifact path. | Probe names the missing native window/capture API or headless backend limitation. | Retained artifact only, with no probe result. |
| GPU visual diff | Before/after image or retained-to-native comparison artifact with threshold and changed/unchanged region summary. | Probe names unavailable GPU readback or diff backend and records why no equivalent can run. | Static descriptor or status text passed as visual diff. |
| Native focus traversal | Ordered focus traversal report from native or retained focus owner, including routes and disabled reasons. | Probe records that native focus cannot be driven in the current backend and retained route inspection is the accepted fallback. | No focus/focusable-route artifact. |
| Pixel contrast sampling | Pixel or retained-token contrast report with sampled subject, foreground/background source, ratio, and limits. | Probe records absence of pixel access and links retained token evidence. | Human note without sample data. |
| Timing | Screenshot/diff/setup/formation timing report with scenario id and environment. | Probe records unavailable timing source and retained timing substitute. | Timing claim with no artifact. |
| Diagnostics/degraded/reload/apply/rollback/failure preservation | Scenario-linked RON or text artifacts from the app-hosted path. | Not applicable unless the runtime cannot execute the scenario; that becomes a blocking design issue. | Console-only or descriptor-only evidence. |

## Implementation Shape

Use a Strangler migration over PM006:

1. Add typed capability, probe, result, and artifact-kind contracts beside the
   existing PM006 manifest.
2. Keep PM006 artifact compatibility while adding a no-gap manifest version or
   extension that records per-capability results.
3. Convert unsupported checks into typed `PlatformImpossible` results only when
   a probe justifies the classification.
4. Add focused manifest validation rejecting missing no-gap capability results.
5. Extend the artifact-writing PM006 test or add a PM002 test that writes the
   PM002 evidence bundle under the PM002 closeout directory.
6. Preserve app ownership of artifact writing and runtime probing.

## Validation

The owning implementation issue must include focused validation such as:

```text
cargo fmt --all --check
cargo test -p runenwerk_editor editor_lab_evidence
cargo test -p runenwerk_editor pm_ui_lab_perf_002
$env:RUNENWERK_WRITE_PM_UI_LAB_PERF_002_EVIDENCE='1'; cargo test -p runenwerk_editor pm_ui_lab_perf_002_runtime_evidence_platform_closure -- --nocapture
cargo validate
git diff --check
CI=true pnpm --dir docs-site build
```

Tests must prove:

- descriptor-only evidence is rejected;
- each no-gap capability has a captured, platform-impossible, or failed
  result;
- platform-impossible results require probe metadata;
- native/retained artifacts are scenario-linked and reproducible;
- app-owned evidence code does not move into `ui_definition` or
  `editor_definition`.

Repository-owned exact-head CI and Documentation Build are the acceptance
evidence for the reviewed revision.

## Implementation Activation

Historical `WR-105` and its roadmap-intake record are retained as provenance for
the original runtime-evidence platform closure slice. They do not authorize
current work.

Any current implementation is activated by an owning GitHub issue. The bounded
expected write scope remains:

- `apps/runenwerk_editor/src/shell/editor_lab_evidence/mod.rs`
- `apps/runenwerk_editor/src/shell/tests.rs`
- PM002 closeout/evidence documentation as needed.

Runtime artifacts belong under the PM002 closeout directory. Raw artifacts must
not be mixed into prose docs. The pull request owns the complete diff, reviewed
feature head, validation evidence, and acceptance state.

## Non-Goals

PM002 does not:

- implement command catalog or surface registry source-of-truth closure;
- redesign direct-manipulation UX;
- change persistence/diff/apply APIs or public examples;
- complete final no-gap certification;
- move screenshot, focus, contrast, timing, provider sessions, or artifact
  writing into `domain/ui/ui_definition`;
- treat a free-form unsupported string as no-gap evidence.

## Stop Conditions

Stop before implementation if:

- ownership of native capture, focus, contrast, timing, or visual diff cannot
  stay app-owned;
- platform-impossible evidence would be asserted without probe metadata;
- evidence would be descriptor-only, status-panel-only, console-only, or
  retained-only where a native capability is available;
- a reusable cross-domain evidence platform is required without an accepted ADR
  or design update;
- PM002 scope begins command/surface closure, direct manipulation UX closure,
  API ergonomics closure, or final certification work.
