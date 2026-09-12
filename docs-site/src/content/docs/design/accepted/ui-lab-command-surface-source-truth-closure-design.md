---
title: UI Lab Command And Surface Source Truth Closure Design
description: Accepted design for PM-UI-LAB-PERF-003 command catalog, surface registry, ownership, and module-structure source-of-truth closure.
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
  - ./ui-lab-perfectionist-audit-design.md
  - ./ui-lab-command-catalog-and-surface-registry-design.md
  - ./ui-lab-runtime-evidence-platform-closure-design.md
  - ../implemented/ui-lab-productization-design.md
  - ../active/editor-tool-suite-registry-and-workbench-host-design.md
  - ../active/editor-ui-workspace-tool-surface-architecture.md
related_reports:
  - ../../reports/closeouts/pm-ui-lab-perf-001-governance-audit-doctrine-and-code-truth-matrix/closeout.md
  - ../../reports/closeouts/pm-ui-lab-perf-002-runtime-evidence-platform-closure/closeout.md
related_roadmaps:
  - ../../domain/ui/roadmap.md
---

# UI Lab Command And Surface Source Truth Closure Design

## Status

Accepted for `PM-UI-LAB-PERF-003`.

This design clears the design gate for command, surface, ownership, and
module-structure source-of-truth closure only. It does not authorize product
code by itself. Any implementation requires an owning GitHub issue, canonical
roadmap sequencing where relevant, and a pull request that owns the reviewed
feature head and exact-head validation evidence.

## Goal

Editor Lab V1 no-gap certification requires one normal source of truth for
commands and one normal source of truth for surfaces:

```text
authored command presentation and shortcuts
  -> app-owned EditorCommandCatalog descriptors
  -> shell route-action projection and diagnostics
  -> app-owned command dispatch

tool-suite surface declarations
  -> editor_shell ToolSurfaceDefinitionRegistry metadata
  -> workspace/profile/provider projection
  -> app-owned provider execution
```

Compatibility adapters may preserve legacy `KnownEditorCommand`,
`ToolbarCommandKind`, `ToolbarMenuKind`, `PanelKind`, and `ToolSurfaceKind`
edges, but normal command/surface behavior must not depend on those adapters as
parallel authorities.

## Current Code Truth

Current code already contains useful foundations from the earlier
`PM-UI-LAB-002` work:

- `apps/runenwerk_editor/src/shell/command_catalog/mod.rs` defines
  `KnownEditorCommand`, `EditorCommandDescriptor`, availability rules, aliases,
  labels, routed shell actions, and shell-command conversion.
- `apps/runenwerk_editor/src/shell/command_resolution.rs` resolves active
  route actions through `KnownEditorCommand::from_key`.
- `apps/runenwerk_editor/src/shell/dispatch_shell_command.rs` and
  `apps/runenwerk_editor/src/shell/toolbar_adapter.rs` still need to prove that
  dispatch, toolbar, menu, disabled reasons, and authored route projection all
  consume the catalog rather than re-owning labels or enablement.
- `apps/runenwerk_editor/src/runtime/systems/input_bridge.rs` still carries
  shortcut-to-command state that must be catalog-derived for normal commands.
- `domain/editor/editor_shell/src/tool_suite/{definition,registry}.rs` defines
  registry-owned surface metadata and provider-family assignment validation.
- `domain/editor/editor_shell/src/workspace/surface_contract.rs` still contains
  long legacy `ToolSurfaceKind` mapping tables that must become migration
  adapters rather than normal surface authority.
- `apps/runenwerk_editor/src/shell/providers/mod.rs` uses stable-key support
  helpers and app-owned provider implementations, but provider ids, support
  modes, family assignment, and surface labels must be checked against
  registry metadata.

No current code inspection requires moving command or surface authority into
`domain/ui/ui_definition`.

## Architecture Governance

Governance result for PM003:

- DDD bounded context owner: `editor`.
- Command descriptor owner: `apps/runenwerk_editor` owns concrete Runenwerk
  editor command descriptors, dynamic app availability, shell-command dispatch
  targets, shortcut/menu/toolbar projection data, and app diagnostics.
- Surface metadata owner: `domain/editor/editor_shell` owns app-neutral
  `ToolSurfaceDefinitionRegistry` contracts: stable keys, labels, roles,
  capabilities, provider family, route kind, persistence mode, retention, and
  creation policy.
- Provider execution owner: `apps/runenwerk_editor` owns concrete provider
  instances, provider ids, provider support decisions, runtime frame building,
  interaction mapping, and app-specific diagnostics.
- Definition owner: `domain/editor/editor_definition` may declare editor
  command, menu, shortcut, binding, and surface references but must not execute
  app commands or provider behavior.
- Generic UI owner: `domain/ui/ui_definition` remains behavior-free and may
  describe route slots or authored UI structure only.
- ADR need: no new ADR is required while the implementation keeps command
  execution app-owned, surface contracts in `editor_shell`, and generic UI
  behavior-free. Add an ADR or accepted design update before moving concrete
  editor command semantics, provider-family authority, or surface execution
  into `ui_definition` or another cross-domain platform.
- ATAM-lite priority order: source-of-truth correctness first, migration
  compatibility second, runtime diagnostics third, author ergonomics fourth,
  performance fifth.
- Ownership mode: stream-aligned editor product work with complicated-subsystem
  support from editor shell and UI definition maintainers.

## Command Source-Truth Contract

`EditorCommandCatalog` is the normal source for:

- stable command key and legacy aliases;
- label and presentation grouping;
- route targets used by authored UI and shell frame projection;
- toolbar, menu, keybinding, and palette/search projection;
- availability evaluator, disabled reason, and diagnostic code;
- routed shell action and app dispatch target.

`KnownEditorCommand` may remain as the migration key while the catalog closes.
It must not be an independent descriptor source. New normal command APIs should
ask the catalog for descriptors and projections.

`assets/editor/ui/editor_bindings.ron` remains authored presentation input. It
must reference catalog route keys and must not override catalog labels,
availability, disabled reasons, or dispatch behavior except for explicitly
presentation-only placement data.

`domain/editor/editor_shell` may project routed shell actions into retained UI
frames, but it must not invent editor command fallbacks, labels, or disabled
reasons for normal Editor Lab commands. Missing catalog routes should fail
closed with diagnostics.

## Surface Source-Truth Contract

`ToolSurfaceDefinitionRegistry` is the normal source for:

- stable surface key;
- label and role;
- surface definition id;
- provider family;
- route kind;
- capability set;
- session retention class;
- persistence mode;
- creation policy;
- target-profile and host compatibility metadata;
- explicit legacy mapping where migration still needs it.

`ToolSurfaceKind` and `PanelKind` may remain at persistence, test, import, and
legacy route edges. Normal workspace/profile/provider projection should use
stable surface keys and registry definitions first.

`apps/runenwerk_editor/src/shell/providers/mod.rs` owns concrete provider
execution. Provider descriptors and support checks must be validated against
registry metadata so app providers cannot silently create an undeclared normal
surface path.

## Module Boundary Contract

Implementation must keep source ownership visible in module names:

- command contracts and projection helpers belong under
  `apps/runenwerk_editor/src/shell/command_catalog/`;
- app provider execution stays under `apps/runenwerk_editor/src/shell/providers/`;
- shell command dispatch adapters stay under `apps/runenwerk_editor/src/shell/dispatch/`
  or the existing app shell dispatch modules;
- app-neutral surface metadata stays under
  `domain/editor/editor_shell/src/tool_suite/` and
  `domain/editor/editor_shell/src/workspace/`;
- no new catch-all `utils`, `helpers`, or `_internal` modules are allowed.

## Implementation Shape

Use a Strangler migration:

1. add catalog and registry audits that expose every duplicated normal path;
2. route toolbar/menu/keybinding/palette/dispatch projections through catalog
   descriptors while preserving legacy enum adapters;
3. route surface labels, capabilities, retention, provider family, creation
   policy, and provider support checks through registry definitions;
4. leave `ToolSurfaceKind`, `PanelKind`, `ToolbarCommandKind`, and
   `ToolbarMenuKind` only at compatibility edges;
5. add diagnostics and tests before removing fallback behavior.

## Fitness Functions

The owning implementation issue must include focused validation before closeout:

```text
cargo test -p runenwerk_editor command_catalog
cargo test -p runenwerk_editor command_source_truth
cargo test -p runenwerk_editor surface_source_truth
cargo test -p editor_shell tool_suite
cargo test -p editor_shell surface_contract
cargo validate
git diff --check
CI=true pnpm --dir docs-site build
```

Tests must prove:

- every normal authored command route resolves to exactly one catalog command;
- toolbar, menu, keybinding, palette/search, route action, disabled reason, and
  dispatch projections use the same catalog descriptor;
- shell fallback tables are not needed for normal Editor Lab command routes;
- every normal surface provider support path is declared by registry metadata;
- legacy surface enum mappings are explicit compatibility adapters with typed
  diagnostics for missing or stale mappings;
- module placement follows subdomain ownership boundaries.

Repository-owned exact-head CI and Documentation Build are the acceptance
evidence for the reviewed revision.

## Implementation Activation

`WR-107` is retained as historical decomposition/provenance for the command and
surface source-truth closure slice. It is not current implementation authority.

Any current implementation is activated by an owning GitHub issue. The bounded
expected write scope remains concentrated in:

- `apps/runenwerk_editor/src/shell/command_catalog/`
- `apps/runenwerk_editor/src/shell/command_resolution.rs`
- `apps/runenwerk_editor/src/shell/dispatch_shell_command.rs`
- `apps/runenwerk_editor/src/shell/toolbar_adapter.rs`
- `apps/runenwerk_editor/src/runtime/systems/input_bridge.rs`
- `apps/runenwerk_editor/src/shell/providers/`
- `domain/editor/editor_shell/src/tool_suite/`
- `domain/editor/editor_shell/src/workspace/surface_contract.rs`
- `assets/editor/ui/editor_bindings.ron`
- `apps/runenwerk_editor/src/shell/tests.rs`

The pull request owns the reviewed feature head, complete diff, validation
evidence, and acceptance state. Historical PM/PT/WR labels remain provenance,
not permission.

## Non-Goals

PM003 does not:

- redesign direct-manipulation Editor Lab UX;
- change persistence, project IO, diff/apply, public preludes, usage guides, or
  examples;
- claim final no-gap certification;
- move concrete editor command semantics or provider execution into
  `domain/ui/ui_definition`;
- remove legacy enum compatibility before migration evidence proves the normal
  path no longer depends on it.

## Stop Conditions

Stop before implementation if:

- command labels, disabled reasons, route actions, shortcuts, or dispatch still
  need two normal authorities;
- surface labels, provider families, capabilities, retention, or creation
  policy still need two normal authorities;
- closing the normal path requires breaking persisted legacy surface data
  without an accepted migration plan;
- implementation would move editor/app execution into `ui_definition`;
- the slice starts PM004, PM005, or PM006 scope.
