---
title: UI Lab App-Hosted Editor Lab Surface Shell Design
description: Accepted design for PM-UI-LAB-003 app-hosted Editor Lab shell productization.
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
  - ./ui-lab-command-catalog-and-surface-registry-design.md
  - ./ui-designer-interface-lab-platform-design.md
  - ../implemented/editor-self-authoring-and-final-ui-design.md
  - ../implemented/surface-workflow-contract-redesign.md
  - ./ui-designer-canonical-ir-and-composition-design.md
  - ./ui-designer-target-projection-profiles-design.md
  - ./ui-designer-visual-layout-and-interface-composition-design.md
  - ./ui-designer-view-model-capability-and-intent-binding-design.md
  - ./ui-designer-persistence-migration-diff-and-activation-design.md
related_roadmaps:
  - ../../domain/ui/roadmap.md
---

# UI Lab App-Hosted Editor Lab Surface Shell Design

## Status

This is the accepted implementation design for `PM-UI-LAB-003`.

It clears the design gate for the app-hosted Editor Lab surface shell only. It
does not authorize product code by itself. Implementation requires an owning
GitHub issue, canonical roadmap sequencing where relevant, and a pull request
that owns the reviewed feature head and exact-head validation evidence.

## Goal

The Editor Design workspace must become a usable Editor Lab workbench, not a
debug-style self-authoring control panel. Authors need direct controls for
definition selection, hierarchy inspection, palette-driven creation, retained
preview or canvas viewing, focused inspection, command or apply review,
diagnostics, and preview-console feedback.

The product shell path is:

```text
registered Editor Design surfaces
  -> app-owned Editor Lab state/view-model translation
  -> typed editor-shell surface view models and composition builders
  -> direct controls and provider-local routes
  -> app-owned shell command dispatch
  -> runtime evidence from the launched editor
```

The milestone proves the shell experience and control routing. Operation
history, project IO, complete diff/apply/rollback, preview evidence matrices,
public API examples, and the final no-gap audit remain later milestones.

## Current Code Truth

The current Editor Design profile already mounts a useful surface set through
`apps/runenwerk_editor/src/shell/tool_suites/editor_design_tool_suite.rs`:
definition outliner, UI hierarchy, UI canvas, style inspector, bindings, dock
layout preview, theme editor, shortcut editor, menu editor, definition
validation, and command diff.

The current provider implementation in
`apps/runenwerk_editor/src/shell/providers/self_authoring.rs` is still mostly a
line/action adapter:

- `UiCanvas` can return a formed retained preview for a selected UI template.
- Other Editor Design surfaces call
  `domain/editor/editor_shell/src/composition/build_self_authoring_control_panel.rs`.
- Selection, duplicate, rename, delete, export, apply, rollback, node text
  editing, theme color editing, and workspace layout edits are exposed as
  generic compact action buttons with hard-coded values.
- Diagnostics and command diff are text summaries, not structured review
  surfaces.
- There is no normal palette surface, no structured inspector fields, no
  direct text or enum controls for authored values, no preview-console surface,
  and no degraded-provider shell experience beyond fallback lines.

The app-owned state in `apps/runenwerk_editor/src/shell/self_authoring.rs`
already provides useful behavior: checked-in fixture loading, document
selection, node selection, duplicate/rename/delete/import/export, selected
diagnostics, retained preview formation, basic apply preview, apply snapshots,
and rollback snapshots.

`apps/runenwerk_editor/src/shell/dispatch_shell_command.rs` already keeps
execution app-owned. Provider-local editor-definition actions map to
`ShellCommand` values and then mutate `SelfAuthoringWorkspaceState` or append
console diagnostics. PM-003 must preserve that command boundary.

`PM-UI-LAB-002` made command projection and surface metadata registry-backed.
PM-003 must consume those boundaries instead of reintroducing route, label,
surface, or provider-family side tables.

## Architecture Governance Result

Architecture governance was run for this scope. The historical command and
then-current file scope are retained here as provenance only; they are not a
current work-authorization mechanism:

```text
task ai:architecture-governance -- --task "PM-UI-LAB-003 App-Hosted Editor Lab Surface Shell design gate" --scope "docs-site/src/content/docs/design/active/ui-lab-productization-design.md; docs-site/src/content/docs/workspace/production-tracks.yaml; apps/runenwerk_editor/src/shell/providers/self_authoring.rs; apps/runenwerk_editor/src/shell/self_authoring.rs; apps/runenwerk_editor/src/shell/tool_suites/editor_design_tool_suite.rs; domain/editor/editor_shell/src/composition/build_self_authoring_control_panel.rs; domain/editor/editor_shell/src/composition/build_editor_shell.rs; domain/editor/editor_shell/src/surfaces/editor_definition.rs; domain/editor/editor_definition; domain/ui/ui_definition"
```

Governance decisions:

- DDD owner: the stream-aligned editor app owns the Editor Lab product shell.
- Supporting owners: `domain/editor/editor_shell` owns structural surface
  contracts and app-neutral composition helpers; `domain/editor/editor_definition`
  owns reusable editor definition validation and document mechanics;
  `domain/ui/ui_definition` owns behavior-free UI definition formation and
  generic visual layout contracts.
- Clean Architecture direction: `domain/ui/ui_definition` must not import app,
  shell-provider, project IO, renderer, runtime, or editor execution behavior.
  `apps/runenwerk_editor` may compose domain contracts and own concrete Editor
  Lab state, commands, provider behavior, console output, runtime activation,
  and evidence capture.
- Source truth invariant: authored definitions and app-owned Editor Lab draft
  state are source truth. Formed retained previews, shell surface frames,
  provider output, and screenshot artifacts are derived evidence.
- Translation boundary: provider interactions cross from direct controls into
  typed `SurfaceLocalAction::EditorDefinition` actions, then into app-owned
  `ShellCommand` dispatch. Domain crates expose contracts and composition, not
  app execution.
- ADR need: no new ADR is required for PM-003 if the work only replaces the
  generic line/action shell with typed Editor Lab view models and direct
  controls while preserving accepted command, projection, provider, and
  description-versus-execution decisions. A new ADR or accepted design update is
  required before moving Editor Lab project IO, command execution, runtime
  activation, or provider behavior into generic domain crates.
- Ownership mode: stream-aligned editor product work with complicated-subsystem
  support from editor-shell and UI-definition owners.

## Product Surface Contract

PM-003 introduces an app-hosted Editor Lab shell contract with these normal
surfaces:

| Surface | Responsibility | Ownership |
|---|---|---|
| Definition hierarchy | Browse loaded authored definition documents, show kind/lifecycle/diagnostic state, select without generic action lists. | App state read model, editor-shell composition. |
| Palette | Present valid document, surface, widget, and layout creation affordances for the selected target profile. | App provider consumes registry and editor-definition contracts. |
| Canvas or preview | Show the retained UI preview for UI template documents, selected-node context, and degraded states for non-previewable documents. | `ui_definition` forms retained UI; app provider owns selection/degraded interpretation. |
| Inspector | Expose focused fields for selected document, UI node, theme token, workspace layout, menu, shortcut, or binding document. | Editor-definition contracts plus app command translation. |
| Command diff and apply review | Show selected document diagnostics and current apply-preview summary as a structured review surface. | App state now; PM-005 owns complete diff/apply/rollback productization. |
| Diagnostics | Show typed validation and provider diagnostics with severity, code, message, and source path. | Owning domain diagnostics, app provider presentation. |
| Preview console | Show recent Editor Lab command, provider, and preview events without requiring the global debug action list. | App runtime console or app-owned Editor Lab console adapter. |

Normal PM-003 workflows must be direct product controls. A surface may use a
degraded fallback panel only for unsupported state, provider failure, or an
explicit compatibility route. The generic text/action panel must not be the
normal path for supported Editor Lab surfaces.

## Typed View-Model Boundary

Add or extend editor-shell contracts in
`domain/editor/editor_shell/src/surfaces/editor_definition.rs` for app-neutral
Editor Lab surface view models and actions. The contracts should describe
surface shape, stable row or field ids, labels, selected ids, disabled reasons,
diagnostics, and routeable editor-definition actions.

Expected view-model families:

- `EditorLabDefinitionHierarchyViewModel`
- `EditorLabPaletteViewModel`
- `EditorLabCanvasPreviewViewModel`
- `EditorLabInspectorViewModel`
- `EditorLabReviewViewModel`
- `EditorLabDiagnosticsViewModel`
- `EditorLabConsoleViewModel`

These view models are not source truth and must not execute behavior. The app
provider builds them from `SelfAuthoringWorkspaceState`, command catalog data,
tool-surface registry data, and editor-definition validation reports.

Composition helpers belong under
`domain/editor/editor_shell/src/composition` using responsibility-oriented
module names, for example an `editor_lab` submodule with hierarchy, palette,
canvas, inspector, review, diagnostics, and console builders. Do not extend
`build_self_authoring_control_panel.rs` into a broader catch-all.

## App Provider Boundary

`apps/runenwerk_editor/src/shell/providers/self_authoring.rs` remains the first
implementation owner for Editor Lab provider behavior in PM-003. It may be
renamed only if the implementation can preserve migration clarity and test
history; a broad rename is not required for PM-003.

The provider must:

- request surface metadata from the registry-backed surface contract created in
  PM-002;
- build typed Editor Lab view models from app-owned self-authoring state;
- compose direct controls from editor-shell builders;
- route controls through `SurfaceLocalAction::EditorDefinition`;
- map provider-local actions to app-owned `ShellCommand` values;
- report unavailable or unsupported states with typed diagnostics or explicit
  degraded-provider view models;
- keep hard-coded demo values out of normal user workflows.

`apps/runenwerk_editor/src/shell/self_authoring.rs` may expose additional read
models and narrow command helpers needed by the shell, but it must not become a
generic domain crate during PM-003. PM-004 and PM-005 own operation history and
project IO boundaries.

## Ergonomics Requirements

PM-003 must remove these current friction points from normal workflows:

- document selection as a long list of generic buttons;
- hard-coded edits such as `Edited in self-authoring` and `#5f8cff`;
- action-only theme, menu, shortcut, and binding editing;
- diagnostic text lines without severity/code/path structure;
- command diff text without a reviewable surface shape;
- non-previewable documents that look like missing implementation instead of
  explicit degraded states;
- hidden command feedback that requires reading only global console lines.

Direct controls should use the existing retained UI widget vocabulary: lists,
trees, buttons, text inputs where interaction routing already supports text,
toggles, menus, and compact panels. PM-003 does not need to invent a new UI
framework.

## Migration Strategy

Use a Strangler Fig replacement for the line/action path:

1. Add typed Editor Lab view models and composition builders beside
   `build_self_authoring_control_panel.rs`.
2. Route one Editor Design surface at a time from line/action output to a typed
   direct-control builder.
3. Preserve provider-local action mapping and shell command dispatch while
   changing only presentation and surface ergonomics.
4. Keep a named degraded fallback for unsupported or failed provider states.
5. Add tests that fail if supported Editor Design surfaces regress to generic
   line/action panels.
6. Remove or narrow `build_self_authoring_control_panel.rs` only after no normal
   Editor Lab workflow depends on it.

## Runtime Evidence Requirements

PM-003 closeout may claim `runtime_proven` only when evidence proves the app
shell path, not only descriptors:

- the launched editor or an equivalent app-hosted shell harness opens the
  Editor Design profile;
- mounted Editor Design surfaces resolve through registry metadata and provider
  frames;
- direct controls select an authored definition document and UI node;
- a supported edit runs through the app-owned command boundary and updates the
  retained preview or structured inspector state;
- diagnostics and command/apply review surfaces show structured data;
- a degraded-provider or non-previewable-document state is visible and typed;
- a preview console or app-owned console adapter records user-visible feedback;
- evidence artifacts are stored near the PM-003 closeout.

Screenshots or equivalent visual artifacts are required for the visible shell
states PM-003 claims. PM-006 still owns the broader scenario matrix,
accessibility checks, performance evidence, and visual-diff suite.

## Fitness Functions

The owning PM-003 implementation issue must include focused validation before
closeout:

- editor-shell tests for each Editor Lab composition builder and route table;
- app-shell tests proving every supported Editor Design surface builds a typed
  product surface instead of the generic self-authoring action panel;
- provider tests for degraded non-previewable and provider-failure states;
- dispatch tests for direct controls that select documents, select UI nodes,
  edit a supported text or theme field, and surface diagnostics;
- runtime evidence test or harness that writes a PM-003 proof artifact;
- source or behavior guard preventing hard-coded demo edit values from becoming
  normal workflows;
- current repository validation.

Minimum validation for an implementation slice includes the focused tests plus:

```text
cargo fmt --all --check
cargo test -p editor_shell editor_lab
cargo test -p runenwerk_editor editor_lab
cargo test -p runenwerk_editor pm_ui_lab_003
cargo validate
git diff --check
CI=true pnpm --dir docs-site build
```

Repository-owned exact-head CI and Documentation Build are the acceptance
evidence for the reviewed revision.

## Implementation Activation

Historical `WR-095`, `WR-004`, `WR-046`, and `WR-094` labels are retained only
as decomposition/provenance for the earlier delivery plan. They do not select or
authorize current work.

Any current implementation must be activated by an owning GitHub issue whose
scope remains bounded to the Editor Lab surface shell. Expected code ownership
remains concentrated in:

- `apps/runenwerk_editor/src/shell/providers/self_authoring.rs`
- `apps/runenwerk_editor/src/shell/self_authoring.rs`
- `apps/runenwerk_editor/src/shell/tool_suites/editor_design_tool_suite.rs`
- `domain/editor/editor_shell/src/surfaces/editor_definition.rs`
- `domain/editor/editor_shell/src/composition`
- `domain/editor/editor_shell/src/workspace/profile.rs`

The pull request owns the reviewed feature head, complete diff, validation
evidence, and acceptance state. Canonical roadmaps may sequence this work, but
neither historical WR rows nor PM/PT labels grant implementation authority.

## Non-Goals

PM-003 does not implement:

- operation history, undo/redo, deterministic operation diffs, or full visual
  authoring operation routing owned by PM-004;
- app project packages, save/load/import/export/reload, complete diff/apply,
  activation failure preservation, or rollback productization owned by PM-005;
- preview scenario matrix, visual diffing, accessibility checks, performance
  evidence, or broad degraded-provider evidence owned by PM-006;
- focused public API guides, examples, public API ergonomics review, or final
  runtime-proven track closeout owned by PM-007;
- game-runtime UI projection execution;
- moving editor execution behavior into `domain/ui/ui_definition`.

## Stop Conditions

Stop before implementation if PM-003 would:

- require `ui_definition` to own editor behavior, provider behavior, project IO,
  command execution, runtime activation, or app console state;
- require `editor_shell` composition output or provider frames to become source
  truth;
- bypass the command catalog or surface registry completed in PM-002;
- claim runtime proof without app-hosted shell evidence and visible artifacts;
- implement PM-004, PM-005, PM-006, or PM-007 scope under the shell milestone;
- need durable ownership or dependency changes without a new accepted ADR or
  design update.
