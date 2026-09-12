---
title: UI Lab Perfectionist Audit Design
description: Accepted design for Editor Lab V1 no-gap certification after the runtime-proven UI Lab closeout.
status: accepted
owner: editor
layer: domain/app
canonical: true
last_reviewed: 2026-09-12
related_adrs:
  - ../../adr/accepted/0001-use-domain-owned-commands.md
  - ../../adr/accepted/0004-separate-description-from-execution.md
  - ../../adr/accepted/0005-projections-are-derived-state.md
  - ../../adr/superseded/0006-editor-surface-provider-plugin-seam.md
  - ../../adr/accepted/0010-graph-substrate-canvas-boundary.md
  - ../../adr/superseded/0012-capability-workbench-clean-break.md
related_designs:
  - ../implemented/ui-lab-productization-design.md
  - ../active/ui-designer-interface-lab-platform-design.md
  - ../active/editor-tool-suite-registry-and-workbench-host-design.md
  - ../active/editor-ui-workspace-tool-surface-architecture.md
  - ./ui-lab-command-catalog-and-surface-registry-design.md
  - ./ui-lab-app-hosted-editor-lab-surface-shell-design.md
  - ./ui-lab-operation-driven-visual-authoring-design.md
  - ./ui-lab-persistence-project-io-diff-apply-rollback-design.md
  - ./ui-lab-preview-lab-runtime-evidence-design.md
  - ./ui-lab-api-docs-examples-runtime-closeout-design.md
related_roadmaps:
  - ../../domain/ui/roadmap.md
related_reports:
  - ../../reports/implementation-plans/wr-100-ui-lab-perfectionist-governance-and-no-gap-audit-doctrine/plan.md
---

# UI Lab Perfectionist Audit Design

## Decision

`PT-UI-LAB-PERFECTION` is retained as the historical decomposition label for the
Editor Lab V1 no-gap certification sequence. It consumed the completed UI Lab
runtime-proven work as input and did not reopen that work. It also did not
expand into game-runtime UI projection, which remains separate work.

Editor Lab V1 may claim `perfectionist_verified` only after runtime evidence,
public APIs, examples, docs, current delivery evidence, and code truth agree
with no known quality gaps.

## Historical WR-100 Governance Contract

The detailed `PM-UI-LAB-PERF-001` execution contract is retained at
`docs-site/src/content/docs/reports/implementation-plans/wr-100-ui-lab-perfectionist-governance-and-no-gap-audit-doctrine/plan.md`.

That contract records the historical code-truth reconciliation, evidence
matrix, hard blockers, and follow-on decomposition for the audit sequence. This
accepted design remains the durable doctrine source. `WR-100` is provenance,
not current work authority.

## Architecture Governance

The repository architecture-governance kickoff established:

- DDD bounded context owner: `editor`.
- Supporting owners: `domain/ui/ui_definition` for behavior-free generic UI
  definition mechanics, `domain/editor/editor_definition` for reusable editor
  definition mechanics, `domain/editor/editor_shell` for shell contracts, and
  `apps/runenwerk_editor` for concrete app behavior and runtime evidence.
- Vocabulary: Editor Lab V1, no-gap certification, runtime evidence, command
  catalog, surface registry, direct manipulation, typed operation, structural
  diff, activation report, rollback, focused public API, and drift check.
- Invariants: UI definitions describe UI/interface structure; they do not
  execute editor or game semantics. Runtime projections and previews are
  derived state. App-owned evidence capture must not become domain truth.
- Clean Architecture direction: `domain/ui` and `domain/editor` crates may
  expose typed contracts and validation, but app windowing, provider execution,
  project IO, screenshot capture, and activation execution stay in
  `apps/runenwerk_editor` or an app-owned adapter.
- ADR need: no ADR is required for the audit setup. Later implementation must
  add or update an ADR if it changes durable ownership, dependency direction,
  source-of-truth authority, or cross-domain boundary contracts.
- ATAM-lite priority order: correctness and ownership first, runtime evidence
  second, author ergonomics third, compatibility fourth, performance fifth.
- Team Topologies label: stream-aligned editor product work with
  complicated-subsystem support from UI definition, editor shell, and app
  runtime evidence owners.

## Audit Scope

The audit covers only Editor Lab V1:

- native or typed-impossible runtime evidence;
- command catalog and surface registry authority;
- direct-manipulation hierarchy, palette, canvas, inspector, diagnostics,
  operation diff, undo, redo, and preview workflows;
- persistence, structural diff/apply, activation reports, reload, rollback,
  and failed activation preservation;
- public API ergonomics, focused preludes, usage guides, and examples;
- module structure and ownership-boundary consistency;
- agreement between current documentation, delivery evidence, and code truth.

The audit explicitly excludes game-runtime UI projection implementation. If
that work becomes active, it requires its own owning GitHub issue and accepted
boundary.

## Code-Truth Matrix

`PM-UI-LAB-001` through `PM-UI-LAB-007` closed the historical UI Lab sequence as
`runtime_proven`, but their closeouts intentionally preserved several no-gap
blockers:

- Native window screenshots, GPU visual diffing, native focus traversal, pixel
  contrast sampling, native screenshot timing, and GPU visual-diff timing are
  still unsupported-check diagnostics.
- `EditorCommandCatalog` exists, but normal command projection must still be
  audited for residual label, disabled-reason, route, toolbar, menu,
  keybinding, and fallback duplication.
- `ToolSurfaceDefinitionRegistry` exists, but normal surface identity and
  routing must still be audited for legacy `PanelKind` and `ToolSurfaceKind`
  authority leaks.
- Editor Lab interaction must be checked for retained-label or debug-action
  workflows that bypass direct hierarchy, palette, canvas, inspector, and
  operation controls.
- Diff/apply UX must be checked for coarse serialized document rows where
  structural review is required.
- `ui_definition` and `editor_definition` public APIs must be checked for broad
  glob exports, hard-to-discover workflows, and examples that compile without
  proving normal ergonomics.
- Module structure must be checked against subdomain ownership boundaries.

## Evidence Rules

Descriptor-only, docs-only, retained-preview-only, or status-panel-only proof
is insufficient for no-gap certification.

Every implementation slice must produce runtime evidence that a reviewer can
reproduce or inspect without reading internal provider code. Evidence may use a
typed unsupported result only when the target runtime genuinely cannot perform
that check; an unsupported result is not acceptable for a capability that the
runtime can reasonably expose.

Closeout evidence must include the command used, artifact paths, scenario
coverage, diagnostics snapshots, and any remaining platform constraints.

## Milestone Doctrine

The historical PM labels retain useful audit decomposition:

- `PM-UI-LAB-PERF-001`: governance and design only; no app/domain runtime code.
- `PM-UI-LAB-PERF-002`: runtime evidence platform closure.
- `PM-UI-LAB-PERF-003`: command, surface, ownership, and module-structure
  source-of-truth closure.
- `PM-UI-LAB-PERF-004`: direct-manipulation Editor Lab UX closure.
- `PM-UI-LAB-PERF-005`: persistence, structural diff/apply, public API, and
  examples ergonomics closure.
- `PM-UI-LAB-PERF-006`: final no-gap audit; `perfectionist_verified` is legal
  only when `known_quality_gaps` is empty.

These labels describe the accepted decomposition and evidence chain. They do
not grant current implementation authority.

## Current Work Gates

Any new implementation or audit repair is activated by an owning GitHub issue.
The issue owns current scope, prerequisites, accepted base, and blockers. The
pull request owns the complete diff, reviewed feature head, exact-head
validation evidence, and acceptance state.

Focused tests are required for the exact boundary touched. Broad repository
validation then includes:

```text
cargo validate
git diff --check
CI=true pnpm --dir docs-site build
```

Repository-owned exact-head CI and Documentation Build are the acceptance
evidence for the reviewed revision. A completed phase must still receive a
drift review before a stronger no-gap claim is made, but historical task-based
planning routines no longer authorize or select work.

## Stop Conditions

- A proposed implementation moves editor/app execution into `ui_definition`.
- A proposed implementation makes retained previews authoritative state.
- A command, surface, operation, persistence, diff/apply, or API path has two
  normal sources of truth.
- Evidence cannot be reproduced or inspected from closeout artifacts.
- A milestone tries to claim `perfectionist_verified` with any known gap.
- Prior validation is reused after the reviewed feature head changes.
