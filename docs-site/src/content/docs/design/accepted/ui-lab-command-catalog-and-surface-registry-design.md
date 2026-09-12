---
title: UI Lab Command Catalog And Surface Registry Design
description: Accepted design for PM-UI-LAB-002 editor command catalog and registry-owned tool-surface metadata.
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
  - ../../adr/superseded/0012-capability-workbench-clean-break.md
related_designs:
  - ../implemented/ui-lab-productization-design.md
  - ../active/ui-designer-interface-lab-platform-design.md
  - ../active/editor-tool-suite-registry-and-workbench-host-design.md
  - ../active/editor-ui-workspace-tool-surface-architecture.md
  - ../implemented/surface-workflow-contract-redesign.md
  - ./ui-designer-target-projection-profiles-design.md
  - ./ui-designer-component-surface-and-widget-recipe-library-design.md
  - ./ui-designer-view-model-capability-and-intent-binding-design.md
related_roadmaps:
  - ../../domain/ui/roadmap.md
---

# UI Lab Command Catalog And Surface Registry Design

## Status

This is the accepted implementation design for `PM-UI-LAB-002`.

It defines the command catalog and registry-owned surface metadata direction for
the Editor Interface Lab productization work. It does not implement code or
authorize product changes by itself. Implementation requires an owning GitHub
issue, canonical roadmap sequencing where relevant, and pull-request-owned
review and exact-head validation.

## Goal

Editor Lab productization needs one source of truth for editor commands and one
registry-owned source of truth for tool-surface metadata:

```text
authored/menu/toolbar/keybinding declarations
  -> EditorCommandCatalog
  -> route-action projection and availability diagnostics
  -> shell frame route actions
  -> app-owned command execution

tool-suite declarations
  -> ToolSurfaceRegistry metadata
  -> surface definitions, capabilities, retention, provider family, creation policy
  -> workspace/profile/provider projection
  -> app-owned provider execution
```

The goal is cleanup and enforceable ownership, not a new command runtime or a
parallel surface framework.

## Current Friction

Command identity and availability are split across:

- `apps/runenwerk_editor/src/shell/command_resolution.rs::KnownEditorCommand`;
- `apps/runenwerk_editor/src/shell/command_resolution.rs::active_route_actions_by_target`;
- `apps/runenwerk_editor/src/shell/dispatch_shell_command.rs::dispatch_toolbar_command`;
- `domain/editor/editor_shell/src/composition/build_editor_shell.rs::toolbar_action_for_route_slot`;
- `assets/editor/ui/editor_bindings.ron`;
- toolbar adapter code that repeats disabled affordance state.

Tool-surface identity and metadata are split across:

- `domain/editor/editor_shell/src/tool_suite::{definition,registry}`;
- `domain/editor/editor_shell/src/workspace/surface_contract.rs`;
- `domain/editor/editor_shell/src/workspace/state.rs::ToolSurfaceKind`;
- `apps/runenwerk_editor/src/shell/providers/mod.rs`;
- app tool-suite key constants and provider-family assignment tables.

The result is unnecessary duplication: route strings, disabled reasons,
toolbar commands, surface labels, capabilities, retention classes, provider
families, and legacy enum mappings can drift independently.

## Architecture Governance Result

Architecture governance accepts this direction:

- `apps/runenwerk_editor` owns the concrete Runenwerk editor command catalog,
  command execution bridge, dynamic command availability, app diagnostics, and
  provider implementation.
- `domain/editor/editor_shell` owns app-neutral structural command projection
  contracts, routed shell action types, surface definition contracts, tool-suite
  registry validation, workspace projection, and host capability policy.
- `domain/editor/editor_definition` may declare editor/workbench command,
  menu, shortcut, and surface references, but it must not execute app commands.
- `domain/ui/ui_definition` remains generic and behavior-free. It may hold UI
  route slots and authored UI structure, but it must not own editor command
  semantics, provider families, project IO, or shell execution.

No new ADR is required for PM-UI-LAB-002 because the accepted ADRs already
cover domain-owned commands, description-versus-execution, derived projections,
provider seams, and Workbench clean-break identity. A new ADR or accepted
design update is required before generic UI definitions own concrete editor
commands, app provider behavior, or project/runtime execution.

## EditorCommandCatalog Contract

Add an app-owned catalog module at
`apps/runenwerk_editor/src/shell/command_catalog/`.

The catalog owns one descriptor per normal editor command:

- stable command key;
- legacy aliases accepted during migration;
- label and optional menu grouping;
- route slots used by authored UI and bindings;
- toolbar/menu/keybinding/palette projection metadata;
- command kind or app execution target;
- dynamic availability evaluator;
- disabled reason and diagnostic code;
- shell routed action projection;
- app dispatch target.

`KnownEditorCommand` may remain during migration, but it becomes an internal
catalog key or compatibility adapter. It must not remain an independent source
of command truth.

`assets/editor/ui/editor_bindings.ron` remains an authored presentation input
for menu and toolbar layout, but every referenced route must validate against
`EditorCommandCatalog`. Labels and disabled reasons must come from the catalog
unless a field is explicitly presentation-only and checked for consistency.

`EditorShellFrameModel::route_actions_by_route_target` remains the shell input
for formed UI routes. The shell frame builder must stop inventing editor
command fallbacks in
`domain/editor/editor_shell/src/composition/build_editor_shell.rs::toolbar_action_for_route_slot`.
Missing route actions should fail closed as absent actions plus diagnostics
generated before or during frame-model construction.

`apps/runenwerk_editor/src/shell/dispatch_shell_command.rs::dispatch_toolbar_command`
must consume catalog dispatch targets or a catalog-derived command enum. It
must not contain the only implementation of command availability.

## ToolSurfaceRegistry Contract

Do not create a parallel registry. Extend the existing
`domain/editor/editor_shell/src/tool_suite::ToolSuiteRegistry` and its
`ToolSurfaceRegistry` surface projection.

`ToolSurfaceDefinition` must become the normal source for:

- stable surface key;
- label;
- role;
- panel kind;
- provider family;
- route kind;
- persistence mode;
- surface capability set;
- session retention class;
- creation policy;
- target-profile or host compatibility metadata;
- optional legacy compatibility key where migration still needs it.

`domain/editor/editor_shell/src/workspace/surface_contract.rs` remains the
compatibility adapter during migration. Long switch statements keyed by
`ToolSurfaceKind` must be replaced by registry lookups first, with explicit
legacy fallback only for old persistence or old tests.

`ToolSurfaceKind` and `PanelKind` remain compatibility vocabulary at the
workspace and migration edge. New normal APIs must carry
`ToolSurfaceStableKey`, registry definitions, and provider-family metadata.

Provider registration in `apps/runenwerk_editor/src/shell/providers/mod.rs`
must derive support and provider-family matching from stable keys and registry
metadata. Helpers whose names mention legacy support must either actually
handle legacy migration or be renamed to stable-key-only helpers.

## Migration Strategy

Use a Strangler Fig migration:

1. introduce catalog/registry projection APIs while keeping existing public
   behavior;
2. route menu, toolbar, keybinding, and command dispatch through the catalog;
3. route surface metadata, creation candidates, capabilities, and retention
   through registry definitions;
4. leave legacy enum and route adapters only at persistence, test, and import
   boundaries;
5. remove shell fallback tables once route-action coverage is proven by tests.

The implementation must not change command labels, shortcuts, enabled states,
or surface defaults except where the current data is stale. Stale disabled
reasons must be corrected in the catalog and covered by tests.

## Diagnostics And Failure Modes

Catalog validation must report:

- unknown route slot;
- duplicate command key or alias;
- duplicate route target;
- missing dispatch target;
- missing availability evaluator for dynamic commands;
- binding references to unavailable or removed commands;
- command route present in authored UI but absent from frame projection.

Registry validation must report:

- duplicate stable surface key;
- unknown provider family;
- missing capability or retention metadata;
- invalid creation policy;
- legacy enum mapping without a stable key;
- provider support for a key not declared by the installed registry.

These diagnostics may be app-owned if they include app command behavior. Shell
diagnostics remain structural and must not depend on app runtime state.

## Fitness Functions

Any PM-UI-LAB-002 implementation slice must add focused tests before closeout:

- catalog coverage proves every route in `assets/editor/ui/editor_bindings.ron`
  resolves to exactly one catalog command;
- catalog projection proves menu, toolbar, keybinding, palette, and dispatch
  use the same descriptor data;
- shell frame route tests prove `toolbar_action_for_route_slot` fallback is not
  needed for normal command routes;
- stale disabled reasons are rejected or snapshot-tested;
- registry tests prove surface definitions expose capabilities, retention,
  provider family, route kind, persistence, and creation policy;
- provider registry tests prove support checks use stable keys and registered
  provider families;
- legacy compatibility tests prove old persisted `ToolSurfaceKind` data can be
  mapped or rejected with a typed diagnostic.

Minimum validation includes the focused tests plus:

```text
cargo test -p editor_shell
cargo test -p runenwerk_editor command
cargo test -p runenwerk_editor surface
cargo validate
git diff --check
CI=true pnpm --dir docs-site build
```

Repository-owned exact-head CI and Documentation Build are the acceptance
evidence for the reviewed revision.

## Implementation Activation

Historical `WR-094` and `PT-UI-LAB` references are retained only as
accepted-decomposition/provenance vocabulary. They do not grant current
implementation authority.

Any current implementation is activated by an owning GitHub issue whose scope
is bounded to the command catalog and registry-owned surface metadata contract.
The pull request owns the reviewed feature head, complete diff, validation
evidence, and acceptance state.

## Non-Goals

PM-UI-LAB-002 does not build the visual Editor Lab shell, project IO,
operation-driven visual authoring, screenshot capture, runtime evidence
harness, game-runtime UI projection execution, or public API closeout. Those
remain later UI Lab slices.
