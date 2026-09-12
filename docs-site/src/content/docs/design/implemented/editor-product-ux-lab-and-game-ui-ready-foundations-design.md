---
title: Editor Product UX Lab And Game UI Ready Foundations
description: Implemented product/architecture design for native editor UX Lab, all-surface certification, and game-runtime UI compatibility seams.
status: implemented
owner: editor
layer: domain/ui-definition / domain/editor / app
canonical: true
last_reviewed: 2026-09-12
related_adrs:
  - ../../adr/accepted/0004-separate-description-from-execution.md
  - ../../adr/accepted/0005-projections-are-derived-state.md
  - ../../adr/accepted/0009-ui-interaction-formation-v2.md
  - ../../adr/superseded/0012-capability-workbench-clean-break.md
related_designs:
  - ../active/ui-designer-interface-lab-platform-design.md
  - ../accepted/ui-designer-target-projection-profiles-design.md
  - ../accepted/ui-designer-component-surface-and-widget-recipe-library-design.md
  - ../accepted/ui-designer-live-preview-fixtures-scenarios-and-target-matrix-design.md
  - ../accepted/ui-designer-production-readiness-and-evidence-design.md
  - ../accepted/ui-lab-perfectionist-audit-design.md
  - ../deferred/game-runtime-ui-projection-and-hud-platform-design.md
related_roadmaps:
  - ../../domain/ui/roadmap.md
related_reports:
  - ../../reports/closeouts/pm-editor-ux-009-final-local-native-no-gap-certification/closeout.md
---

# Editor Product UX Lab And Game UI Ready Foundations

## Status

Implemented. The `PT-EDITOR-UX` evidence chain completed with
`perfectionist_verified` quality and no known quality gaps in the final
local-native no-gap certification closeout. This lifecycle classification covers
the editor-product UX and game-readiness seam only; game-runtime HUD execution
remains separately deferred. Historical decomposition and code-truth snapshots
below are retained as activation-time provenance, not current gap claims.

## Decision

This design defines the editor product UX perfection target historically
decomposed as `PT-EDITOR-UX`. It does not reopen the completed UI Lab work, and
it does not implement the separate future game-runtime UI target historically
called `PT-GAME-RUNTIME-UI`. It consumes completed UI Designer, UI Lab
perfection, and Workbench capability evidence, then adds the missing
product-wide editor UX layer: native scenario coverage, all-surface
certification, widget gates, graph canvas productization, standalone UI Designer
workbench proof, and final local-native no-gap certification.

The target was intentionally long-term. It rejects app-only styling patches,
descriptor-only proof, visible placeholder product surfaces, and screenshot-free
claims for user-visible workflows.

`PT-EDITOR-UX` and the `PM-EDITOR-UX-*` labels below are retained as useful
historical/decomposition vocabulary only. There is no live production-track
authority in this document. Future implementation requires an owning GitHub
issue; canonical roadmap placement is required when cross-family sequence
matters; the delivery pull request owns review and exact-head validation.

## Architecture Governance

Architecture governance accepts this split:

- `domain/ui/ui_theme` owns deterministic token graphs, modes, skins, state
  variants, provenance, and style diagnostics.
- `domain/ui/ui_definition` owns Canonical UI IR, target profiles, fixtures,
  scenarios, recipe/readiness descriptors, read-only view-model binding
  vocabulary, and validated intent descriptors.
- `domain/ui/ui_tree`, `ui_widgets`, `ui_runtime`, and `ui_graph_editor` own
  backend-neutral primitive widgets, retained UI, layout/input contracts, and
  graph-editor substrate without editor product semantics.
- `domain/editor/editor_definition` owns editor/workbench definition packages,
  reusable editor UI package descriptors, and adapters into generic UI
  definition contracts.
- `domain/editor/editor_shell` owns editor workbench product patterns, surface
  readiness, provider routing, shell/panel/palette/inspector/diagnostic
  adapters, and editor-only source-truth closure.
- `apps/runenwerk_editor` owns native UX Lab execution, provider fixtures,
  workbench presets, screenshot and artifact capture, accessibility/performance
  runners, app command execution, and final certification artifacts.

Future game-runtime UI compatibility is an explicit target-profile seam only.
No Editor UX implementation slice may make game-runtime UI depend on
`domain/editor/editor_shell`, Workbench host policy, editor command routing, or
editor provider vocabulary.

No ADR is required for the governance/decomposition target because it does not
change durable source-truth authority. Require an ADR or accepted design update
before moving generic UI truth into app code, moving app evidence capture into
`domain/ui`, changing dependency direction, introducing a game-runtime UI owner
crate, or making projection/evidence artifacts authoritative state.

## UX Lab Contract

The native Editor UX Lab is the product proof system. It is inspired by
scenario catalogs, args/controls, interaction tests, visual checks, and
accessibility checks, but it is a native Runenwerk contract rather than a web
dependency.

The user-facing product area is **UX Lab**. Its durable areas are **Designer**,
**Scenarios**, and **Evidence**. Code and current planning documents use
scenario vocabulary so the product area cannot be confused with external web UI
workshop tooling.

The first implementation slice must introduce or ratify typed contracts for:

- `EditorUxScenarioCatalog`;
- `EditorUxScenarioId`;
- `EditorUxScenarioKind`;
- `EditorUxScenarioArgs`;
- `EditorUxScenarioInteraction`;
- `EditorUxScenarioMatrix`;
- `EditorUxEvidenceManifest`;
- `ToolSurfaceReadiness`.

`ToolSurfaceReadiness` has four states:

- `Product`;
- `FallbackOnly`;
- `Diagnostic`;
- `HiddenUntilProductized`.

Normal product workflows may not expose generic text/action panels, visible
placeholder surfaces, or misleading product-shaped fallbacks.

## Certification Layers

Certification is tiered:

1. Primitive widgets: every retained UI primitive and constructor exposed by the
   UI substrate has state, sizing, label/accessibility, focus, overflow, and
   interaction coverage.
2. Product widget patterns: editor-specific inspectors, palettes, diagnostics,
   tables, trees, tabs, status/toolbars, split/dock chrome, graph nodes, sockets,
   and overlays have reusable scenario coverage.
3. Registered visible surfaces: every registered user-facing editor surface and
   explicit diagnostic/fallback surface is either certified, diagnostic,
   fallback-only, or hidden.
4. Host scenarios: workbench presets, provider fixtures, keyboard/focus routes,
   degraded providers, native captures, and dense/overflow data states are
   proven in app-hosted scenarios.

The visible-widget scan is a hard gate. Every widget in every certified scenario
must report layout bounds, label/accessibility metadata where
interactive, focus reachability where focusable, overflow behavior, and state
coverage.

## Evidence Rules

Descriptor-only, retained-preview-only, status-panel-only, or screenshot-free
proof is insufficient for final editor UX certification.

Final proof requires local-native evidence when the local runtime can reasonably
produce it. Typed unsupported or platform-impossible results are acceptable only
when the target check truly cannot run in the current runtime or adapter.

Required evidence families:

- retained UI debug artifacts;
- local-native screenshots;
- visual diffs where supported;
- focus traversal reports;
- accessibility reports;
- contrast and text-overflow reports;
- scenario interaction reports;
- provider and diagnostics snapshots;
- performance/timing reports;
- scenario/evidence manifests with freshness policy.

## Product Scope

The Editor UX target covers:

- all registered visible user-facing editor surfaces;
- explicit fallback and diagnostic surfaces;
- primitive widgets and product widget patterns used by certified surfaces;
- native UX Lab scenario coverage;
- standalone UI Designer workbench and real canvas workflow;
- graph canvas/node editor productization;
- shell, inspector, palette, preview, diagnostics, table/tree/tab, toolbar,
  status, split, and dock polish;
- game-runtime UI compatibility seams at the target-profile and evidence
  descriptor level only.

It excludes:

- implementing game-runtime HUD behavior;
- adding `domain/game_ui` or equivalent owner crates;
- reopening completed UI Lab milestones;
- app-only style fixes without token/recipe/state ownership;
- generic UI truth owned by `apps/runenwerk_editor`;
- editor-shell dependencies in future game-runtime UI contracts.

## Proposed Decomposition

The historical `PM-EDITOR-UX-*` sequence remains useful as proposed
architecture/product decomposition, but it is not live lifecycle state:

- `PM-EDITOR-UX-001`: governance and source-truth/evidence doctrine.
- `PM-EDITOR-UX-002`: native Editor UX Lab and evidence harness; this precedes
  broad design-system migration so later polish has executable proof.
- `PM-EDITOR-UX-003`: layered design-system migration for primitives and editor
  product patterns over UI Designer token/recipe/state contracts.
- `PM-EDITOR-UX-004`: standalone UI Designer workbench with real canvas,
  hierarchy, inspector, property panels, token/recipe/binding previews,
  scenario matrices, and readiness evidence.
- `PM-EDITOR-UX-005`: graph canvas and node-editor productization, including
  material graph UX and hide-or-certify policy for SDF, procgen, gameplay,
  particle, and animation graph surfaces.
- `PM-EDITOR-UX-006`: shell/product-pattern migration: inspector, palette,
  diagnostics, preview, tables, trees, tabs, toolbar/status, split/dock,
  empty/loading/error/degraded states, and keyboard/focus workflows.
- `PM-EDITOR-UX-007`: all-surface certification over registered visible
  surfaces and explicit diagnostic/fallback surfaces.
- `PM-EDITOR-UX-008`: game UI readiness seam proof without implementing game
  UI; it verifies recipes, target profiles, fixtures, safe-area/layout axes,
  input-modality axes, and evidence descriptors without editor-shell coupling.
- `PM-EDITOR-UX-009`: final local-native no-gap certification.

An owning GitHub issue may adopt, refine, split, or reject this decomposition
when the work is actually activated. This document does not create active
milestones by naming them.

## Code Truth Snapshot

Current repo truth at the original decomposition point:

- `domain/editor/editor_shell/src/workspace/surface_contract.rs` registers many
  visible surface definitions, including UI Designer, material/SDF/procgen,
  gameplay, particle, physics, animation, simulation, and diagnostics surfaces.
- `domain/ui/ui_tree/src/tree/node.rs` exposes retained node kinds for panels,
  labels, buttons, inputs, tabs, selects, tables, trees, product surfaces, graph
  canvases, viewport embeds, scroll, stack, and split layouts.
- `domain/ui/ui_widgets/src` exposes reusable constructors for retained widgets
  but does not yet provide a product-wide native scenario catalog.
- `apps/runenwerk_editor/src/shell/editor_lab_evidence/mod.rs` already models
  evidence kinds such as retained UI debug artifacts, native screenshots, visual
  diffs, focus traversal, contrast, timing, provider snapshots, diagnostics,
  activation, accessibility, performance, platform-impossible, unsupported
  checks, and evidence manifests.
- Material graph canvas has a real graph projection path. Several future graph
  families and generic self-authoring/control-panel surfaces still require
  productization, hide/fallback policy, or certification.
- Workbench profile and layout code knows an editor-design profile and surfaces,
  but this target still requires a standalone product-grade UI Designer
  workbench with native evidence.

## Quality Gates

Final certification has hard zero budgets for:

- clipped text outside intentional scroll;
- incoherent overlap;
- unlabeled interactive widgets;
- unreachable focus targets;
- route collisions;
- stale scenario artifacts;
- visible misleading placeholders;
- generic normal-flow action panels;
- missing native screenshots where local native capture is available.

Scenario matrices must cover empty, small, dense, overflow, selected, disabled,
readonly, loading, warning, error, degraded, long localized text, keyboard
focus, mouse, gamepad-like navigation where supported, high contrast, reduced
motion, small/large viewport, and native capture.

## Current Activation Rule

This design is durable architecture/product guidance, not a live work tracker.
Any implementation requires an owning GitHub issue that names current scope,
accepted base, dependencies, validation, and evidence. Put the work into the
maintained roadmap when sequence across active families matters. The pull
request owns the reviewed feature head, complete diff, and acceptance evidence.

## Stop Conditions

Stop before implementation if:

- a slice tries to treat app-hosted screenshot/evidence artifacts as domain
  truth;
- generic UI contracts start depending on editor shell or app state;
- future game-runtime UI contracts import editor shell vocabulary;
- visible placeholder product surfaces remain normal workflow surfaces;
- UX Lab proof is descriptor-only, retained-only, or screenshot-free for a
  claim that needs local-native evidence;
- a slice cannot name its exact scenario matrix, evidence artifacts, and hard
  gates before product code starts.

<!-- BEGIN RUNENWERK:UI_COMPONENT_PLATFORM:product-ux-certification -->
## Component Platform certification inputs

Lab and product UX certification consume Component Platform story evidence for controls, interactions, accessibility, layout, text, theme/token use, overlay/popup behavior, Surface2D, SpatialCanvas, NodeCanvas, PortGraphCanvas, ProgressionTreeView, TrackSurface/Timeline, transitions/effects, diagnostics, and budget evidence.
<!-- END RUNENWERK:UI_COMPONENT_PLATFORM:product-ux-certification -->
