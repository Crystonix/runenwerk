---
title: UI Lab Productization Design
description: Implemented productization design for the app-hosted Editor Interface Lab target.
status: implemented
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
  - ../active/ui-designer-interface-lab-platform-design.md
  - ./editor-tool-suite-registry-and-workbench-host-design.md
  - ../active/editor-ui-workspace-tool-surface-architecture.md
  - ./editor-self-authoring-and-final-ui-design.md
  - ./surface-workflow-contract-redesign.md
  - ../accepted/ui-designer-canonical-ir-and-composition-design.md
  - ../accepted/ui-designer-target-projection-profiles-design.md
  - ../accepted/ui-designer-visual-layout-and-interface-composition-design.md
  - ../accepted/ui-designer-theme-tokens-modes-skins-and-state-variants-design.md
  - ../accepted/ui-designer-component-surface-and-widget-recipe-library-design.md
  - ../accepted/ui-designer-view-model-capability-and-intent-binding-design.md
  - ../accepted/ui-designer-live-preview-fixtures-scenarios-and-target-matrix-design.md
  - ../accepted/ui-designer-persistence-migration-diff-and-activation-design.md
  - ../accepted/ui-designer-production-readiness-and-evidence-design.md
  - ../accepted/ui-lab-command-catalog-and-surface-registry-design.md
  - ../accepted/ui-lab-app-hosted-editor-lab-surface-shell-design.md
  - ../accepted/ui-lab-operation-driven-visual-authoring-design.md
  - ../accepted/ui-lab-persistence-project-io-diff-apply-rollback-design.md
  - ../accepted/ui-lab-preview-lab-runtime-evidence-design.md
  - ../accepted/ui-lab-api-docs-examples-runtime-closeout-design.md
related_roadmaps:
  - ../../domain/ui/roadmap.md
related_reports:
  - ../../reports/closeouts/pm-ui-lab-perf-006-final-no-gap-certification-closeout/closeout.md
---

# UI Lab Productization Design

## Status

Implemented. Editor Lab V1 completed its runtime-proven productization sequence,
and the separate final no-gap certification subsequently completed at
`perfectionist_verified` with no known quality gaps. Game-runtime UI projection
execution remains separately owned and deferred; it is not part of this
lifecycle claim. The code-truth reconciliation and milestone decomposition below
are retained as activation-time provenance, not current gap or work-state claims.

It does not reopen the historical UI Designer sequence. The completed
UI Designer milestones are design-contract input with bounded evidence; they
are not runtime-proven proof for an app-hosted Editor Lab.

This design does not authorize implementation or create live work state by
itself. `PT-UI-LAB`, `PM-UI-LAB-*`, old WR rows, and roadmap-intake records are
retained only as architecture/decomposition provenance. Any implementation
requires an owning GitHub issue, canonical roadmap sequencing when relevant,
and a reviewed pull request with exact-head validation evidence.

## Product Goal

The UI Lab target turns the completed UI Designer contracts into a real Editor
Interface Lab inside the editor application. The first product target is Editor
Lab V1, not the full game-runtime UI platform.

The lab must let authors inspect, edit, validate, preview, persist, diff,
apply, and recover authored editor/UI definitions through direct product UI. It
must prove behavior in the launched editor with runtime evidence rather than
only descriptors, docs, or retained preview data.

The target path is:

```text
accepted UI Designer contracts
  -> code-truth reconciliation
  -> Editor Lab product contracts
  -> bounded owning GitHub issues
  -> app-hosted Editor Lab runtime evidence
  -> reviewed exact-head delivery
  -> runtime_proven closeout
  -> separate no-gap audit when justified
```

## Activation-Time Code-Truth Reconciliation

At the original productization point, current code already contained useful
foundations, but the product experience was not yet the planned Editor Interface
Lab.

- `apps/runenwerk_editor/src/shell/providers/self_authoring.rs` and
  `apps/runenwerk_editor/src/shell/self_authoring.rs` provide self-authoring
  behavior through checked-in fixtures and explicit actions. This is a useful
  proving path, but it is not a normal visual lab workflow.
- `domain/editor/editor_shell/src/composition/build_self_authoring_control_panel.rs`
  exposes action/text control panels. Editor Lab V1 must replace normal author
  workflows with direct hierarchy, palette, canvas or preview, inspector, diff,
  diagnostics, and console controls.
- `apps/runenwerk_editor/src/shell/command_resolution.rs`,
  `apps/runenwerk_editor/src/shell/dispatch_shell_command.rs`,
  `domain/editor/editor_shell/src/composition/build_editor_shell.rs`, and
  `assets/editor/ui/editor_bindings.ron` currently split command identity,
  routing, toolbar/menu projection, keybinding projection, availability, and
  disabled reasons. Editor Lab V1 needs one command catalog source of truth.
- `domain/editor/editor_shell/src/workspace/state.rs`,
  `domain/editor/editor_shell/src/workspace/surface_contract.rs`, and
  `apps/runenwerk_editor/src/shell/providers/mod.rs` still carry legacy
  enum-centered surface registration and provider-family knowledge. Editor Lab
  must move toward registry-owned surface metadata while keeping legacy
  persistence compatibility explicit.
- `domain/ui/ui_definition/src/visual_layout/apply.rs` already provides a
  generic visual layout operation foundation. Editor-specific workspace, theme,
  menu, binding, and activation behavior must stay in editor-owned contracts
  and app-owned adapters.
- Public API ergonomics remain incomplete for normal users of
  `ui_definition` and `editor_definition`; examples and focused usage guides
  are part of the product closeout, not optional polish.

## Architecture Governance

Architecture governance findings for this design:

- DDD bounded context owner: `editor`, with generic UI definition mechanics
  owned by `domain/ui/ui_definition` and reusable editor-definition mechanics
  owned by `domain/editor/editor_definition`.
- App adapter owner: `apps/runenwerk_editor` owns concrete Editor Lab provider
  behavior, project IO, runtime activation, screenshot or evidence capture, and
  command execution bridges.
- Core vocabulary: Editor Lab, authored definition, target profile, surface,
  provider family, command catalog, visual operation, project package,
  activation report, rollback, runtime evidence, and closeout.
- Invariants: UI definitions describe UI/interface structure; they do not
  execute editor or game semantics. Projection output is derived state.
  Provider/runtime/app layers consume definitions and reports but do not become
  source truth.
- Translation boundaries: definition documents cross into editor shell through
  typed projection and operation contracts; app providers translate user
  interactions into typed proposals or commands; activation reports cross back
  as diagnostics and review data.
- Clean Architecture direction: `domain/ui/ui_definition` must not import app,
  shell-provider, project IO, renderer, or runtime execution concerns.
  `domain/editor/editor_shell` may host structural shell contracts, but
  semantic execution remains app/provider owned.
- ADR need: no new ADR is required for the existing productization boundary;
  accepted ADRs already cover domain-owned commands, separation of description
  from execution, derived projections, provider seams, graph substrate
  boundaries, and the Workbench clean-break direction. A later slice must add
  or update an ADR if it changes durable ownership, dependency direction, or
  cross-domain authority.
- ATAM-lite priority order: correctness and ownership first, runtime evidence
  second, author ergonomics third, migration compatibility fourth, performance
  and accessibility evidence fifth. A slice may not trade ownership boundaries
  for short-term UI convenience.
- Team Topologies label: stream-aligned editor product work, supported by
  complicated-subsystem owners for UI definition, editor shell, runtime
  evidence, and public API review.

The next legal action for any implementation is to create or select one bounded
owning GitHub issue. The issue owns current scope, dependencies, accepted base,
and evidence requirements; canonical roadmaps own durable sequence where
needed; the pull request owns the reviewed feature head and acceptance evidence.

## Product Contracts

`EditorCommandCatalog` is the single editor command source for labels,
availability, disabled reasons, menu projection, toolbar projection,
keybinding projection, palette/search projection, route targets, diagnostics,
and command dispatch proposals. Existing command-specific compatibility can
remain behind adapters while the catalog becomes the normal authoring path.

`ToolSurfaceDefinitionRegistry` is the single structural source for surface
identity, capability set, retention class, provider family, creation policy,
default workbench placement, and target-profile support. Legacy
`ToolSurfaceKind` and `PanelKind` may remain for migration and compatibility
but must not be the long-term extension point.

`EditorLabOperation` is a typed operation facade. It uses generic UI visual
layout operations for UI-definition layout edits and editor-owned commands for
workspace, theme, menu, binding, surface, and activation edits. It must record
stable ids, source locations, diagnostics, and reviewable diffs.

`EditorLabDocumentStore` is app-owned project IO for Editor Lab packages. It
owns load, save, import, export, migration, last-applied state, failed
activation preservation, and reload behavior.

`DefinitionApplyReview` is the review contract for diff, validation,
migration, activation, diagnostics, failure, rollback, and user-facing apply
results.

Focused public API entry points and usage examples are required for normal
`ui_definition` and `editor_definition` workflows. Broad glob exports are not
an ergonomic product API by themselves.

## Historical Milestone Decomposition

The historical PM sequence remains useful architecture/product decomposition.
It is not live lifecycle state:

- `PM-UI-LAB-001`: design and governance only; code-truth findings and bounded
  implementation candidates.
- `PM-UI-LAB-002`: command catalog and surface registry source-of-truth cleanup.
- `PM-UI-LAB-003`: app-hosted Editor Lab shell and direct authoring panels. Its
  accepted design is
  `docs-site/src/content/docs/design/accepted/ui-lab-app-hosted-editor-lab-surface-shell-design.md`.
- `PM-UI-LAB-004`: operation-driven visual authoring, deterministic diffs,
  history, validation, and diagnostics. Its accepted design is
  `docs-site/src/content/docs/design/accepted/ui-lab-operation-driven-visual-authoring-design.md`.
- `PM-UI-LAB-005`: app project IO, diff/apply, activation reports, and rollback.
  Its accepted design is
  `docs-site/src/content/docs/design/accepted/ui-lab-persistence-project-io-diff-apply-rollback-design.md`.
- `PM-UI-LAB-006`: preview scenarios, screenshot or equivalent visual evidence,
  diagnostics snapshots, accessibility checks, performance evidence, and
  runtime-proof artifacts. Its accepted design is
  `docs-site/src/content/docs/design/accepted/ui-lab-preview-lab-runtime-evidence-design.md`.
- `PM-UI-LAB-007`: public API ergonomics, usage docs, examples, final
  runtime-proven closeout, and intake for the later no-gap audit.

Game-runtime UI projection execution remained outside Editor Lab V1 and remains
separately owned after Editor Lab V1 became runtime-proven.

An owning GitHub issue may adopt, refine, split, or reject this decomposition
when work is activated. Naming a historical PM does not create an active
milestone.

## Bounded Implementation Slice Candidates

The old roadmap-intake proposal is retained as historical provenance only. Its
useful decomposition is preserved here as candidate slices, not WR rows or live
work authority:

| Candidate | Historical mapping | Primary write scope | Runtime evidence |
|---|---|---|---|
| UI Lab command catalog and surface registry | PM-UI-LAB-002 | `apps/runenwerk_editor/src/shell`, `domain/editor/editor_shell/src/{tool_suite,workspace,composition}`, editor bindings assets | Menu, toolbar, keybinding, palette, routing, disabled states, and unavailable-command diagnostics derive from one catalog. |
| Editor Lab shell product surface | PM-UI-LAB-003 | `apps/runenwerk_editor/src/shell/providers`, `domain/editor/editor_shell/src/composition`, editor workspace fixtures | Launched editor shows hierarchy, palette, canvas or preview, inspector, diff, diagnostics, and console workflows without debug action lists. |
| Visual authoring operations | PM-UI-LAB-004 | `domain/ui/ui_definition`, `domain/editor/editor_definition`, editor app adapters | Canvas, hierarchy, and inspector edits round-trip through typed operations with deterministic diffs and undo/redo. |
| Project IO and activation review | PM-UI-LAB-005 | `apps/runenwerk_editor`, editor project package docs and fixtures | Save, reload, import, export, diff, apply, reject, failed activation preservation, and rollback are proven in app. |
| Preview and evidence capture | PM-UI-LAB-006 | editor preview/evidence harness, closeout artifact folders, diagnostics fixtures | Success, warning, error, reload, apply, rollback, degraded-provider, accessibility, and performance states have captured artifacts. |
| Public API and closeout ergonomics | PM-UI-LAB-007 | `domain/ui/ui_definition`, `domain/editor/editor_definition`, docs and examples | Usage examples compile or run, docs validate, public API review passes, and runtime-proven closeout links all evidence. |

Any activated slice must state exact dependencies, write scope, focused tests,
runtime evidence, stop conditions, and accepted base in its owning issue before
implementation. The reviewed pull request then owns the exact delivery diff and
validation evidence.

## Runtime-Proven Acceptance

No Editor Lab implementation may claim `runtime_proven` unless closeout
evidence includes:

- launched editor behavior, not only data descriptors;
- screenshots or equivalent visual artifacts for supported visible states;
- command, provider, validation, persistence, activation, and rollback
  diagnostics;
- focused automated tests for owned contracts;
- current repository and documentation validation at one unchanged reviewed
  head;
- explicit known gaps when an expected check is unsupported by the current
  runtime.

`perfectionist_verified` was not implied by the historical UI Lab sequence; the
separate final no-gap audit later established that stronger claim.

## Validation

Design-only changes use the current repository validation contract:

```text
cargo validate
git diff --check
CI=true pnpm --dir docs-site build
```

Code-bearing slices add focused tests for the exact changed boundary.
Repository-owned exact-head CI and Documentation Build are the acceptance
evidence for the reviewed revision.

## Stop Conditions

Stop before implementation if a slice would:

- require `ui_definition` to execute editor, game, app, project IO, or runtime
  behavior;
- make provider output or retained preview data source truth;
- bypass the owning GitHub issue, canonical roadmap sequence when relevant, or
  reviewed pull-request acceptance;
- claim runtime proof from descriptors, docs, static fixtures, or status panels
  alone;
- add game-runtime UI projection execution before Editor Lab V1 is proven;
- require durable ownership or dependency changes without an accepted ADR or
  design update.

<!-- BEGIN RUNENWERK:UI_COMPONENT_PLATFORM:lab-certification -->
## Component Platform certification inputs

Lab and product UX certification consume Component Platform story evidence for controls, interactions, accessibility, layout, text, theme/token use, overlay/popup behavior, Surface2D, SpatialCanvas, NodeCanvas, PortGraphCanvas, ProgressionTreeView, TrackSurface/Timeline, transitions/effects, diagnostics, and budget evidence.
<!-- END RUNENWERK:UI_COMPONENT_PLATFORM:lab-certification -->
