---
title: UI Designer And Interface Lab Platform
description: Accepted architecture for a generic UI/interface Designer and Lab platform spanning editor/workbench UI and game-runtime UI.
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
  - ./ui-designer-canonical-ir-and-composition-design.md
  - ./ui-designer-target-projection-profiles-design.md
  - ./ui-designer-visual-layout-and-interface-composition-design.md
  - ./ui-designer-theme-tokens-modes-skins-and-state-variants-design.md
  - ./ui-designer-component-surface-and-widget-recipe-library-design.md
  - ./ui-designer-view-model-capability-and-intent-binding-design.md
  - ./ui-designer-live-preview-fixtures-scenarios-and-target-matrix-design.md
  - ./ui-designer-persistence-migration-diff-and-activation-design.md
  - ./ui-designer-production-readiness-and-evidence-design.md
  - ../implemented/editor-tool-suite-registry-and-workbench-host-design.md
  - ../superseded/runenwerk-capability-workbench-target-architecture.md
  - ../active/editor-ui-workspace-tool-surface-architecture.md
related_roadmaps:
  - ../../domain/ui/roadmap.md
---

# UI Designer And Interface Lab Platform

## Status

Accepted architecture direction. This is not an implementation-completeness claim.

It defines the long-term UI Designer/Lab direction historically decomposed as
`PT-UI-DESIGN`. The platform-level ownership model, canonical definition
pipeline, and separate editor/workbench and game-runtime target profiles are
accepted. App/runtime integrations and the future game-runtime UI target remain
separately owned work. This design does not authorize code, schema, runtime,
UI-surface, or work state changes by itself. Any implementation requires an
owning GitHub issue, architecture governance when ownership changes, canonical
roadmap sequencing when relevant, and a reviewed pull request with exact-head
validation evidence.

`PT-UI-DESIGN`, `PM-UI-DESIGN-*`, and old WR/workflow records are retained only
as architecture/decomposition provenance.

## Purpose

Runenwerk needs a generic UI/interface authoring platform that can design,
inspect, validate, preview, migrate, and prove both editor/workbench UI and
game-runtime UI.

The platform is definition-driven. It owns UI/interface definition mechanics,
not product, editor, gameplay, render, material, asset, scene, simulation,
save-game, network, or project truth.

The target pipeline is:

```text
authored UI/interface definitions
  -> Canonical UI IR
  -> deterministic composition
  -> target projection plans
  -> editor workbench and game runtime UI projections
  -> app/runtime/provider consumers
```

## Relationship To The Workbench Capability Substrate

The completed Workbench capability work provides typed suite/profile/provider
identity, host capability policy, product/service declarations, and multi-host
Workbench composition. Its former Capability Workbench North Star is now
superseded by the current platform architecture; the implemented Workbench
contracts remain valid inputs.

The UI Designer/Lab sits above that substrate as the generic UI/interface
Designer/Lab platform. The editor/workbench target profile may use Workbench
contracts; the game-runtime UI target must not depend on editor shell ownership.

Workbench is one target profile, not the whole Designer platform.

## Architecture Domains

`domain/ui` owns runtime-agnostic UI primitives, retained UI model, layout
primitives, style primitives, focus/input/navigation substrate, accessibility
substrate, and reusable interaction substrate.

`domain/ui/ui_definition` is the current owner for generic authored
UI/interface definition documents, Canonical UI IR, validation, normalization,
migration, diagnostics, source maps, and retained UI formation. The accepted
canonical-IR design fixes this as the near-term owner. A future standalone
`domain/ui_definition` crate remains a possible extraction only after a new
accepted design or ADR.

`domain/editor/editor_definition` owns editor/workbench-specific extensions:
workbench profiles, suites, panels, menus, shortcuts, docking/splits/tabs, tool
surfaces, provider families, host policy references, and editor command
declarations.

`domain/editor/editor_shell` owns Workbench host contracts,
suite/profile/provider declarations, host policy, and fail-closed projection
vocabulary for the editor/workbench target.

Future `domain/game_ui` or `domain/game/interface` owns game-runtime UI target
extensions: HUDs, health bars, stamina/mana bars, inventory screens, equipment
screens, hotbars, quest trackers, dialogue UI, minimaps, damage numbers,
nameplates, boss frames, world-space UI, split-screen UI, safe-area rules,
gamepad/touch navigation, platform prompt glyphs, and game-runtime UI projection
declarations.

`apps/runenwerk_editor` owns concrete Designer/Lab app surfaces, project IO,
live preview orchestration, provider implementation, and app command bridges. It
must not own canonical UI/interface truth.

## Non-Negotiable Architecture Invariants

- Designer documents are source truth only for UI/interface definitions.
- Designer documents may include editor/workbench UI and game-runtime UI
  definitions.
- Designer documents are not source truth for editor semantics, gameplay
  semantics, render semantics, material semantics, scene semantics, asset truth,
  simulation, save-game state, network state, or project truth.
- UI definitions may bind only to domain-owned view models.
- UI definitions may emit validated command/intent declarations only.
- UI definitions must not mutate domain state directly.
- Runtime projections must be reproducible from authored definitions, target
  profile, policy, fixtures, and validated composition.
- Preview, runtime, app, and provider layers are consumers of projection output,
  not source truth.
- Denied capabilities fail closed with typed diagnostics.
- Visual edits preserve stable ids and produce reviewable textual diffs.

## Canonical Definition Pipeline

```text
Authored UI Definition Documents
  -> parse + schema/version check
  -> Canonical UI IR
  -> composition / resolve / validate
  -> Target Projection Plan
     -> Editor Workbench Projection
     -> Game Runtime UI Projection
  -> runtime/app/provider layers consume projections but do not own source truth
```

The Canonical UI IR is the convergence point for visual and textual editing.
Every accepted visual edit must round-trip through this IR before activation.

## Target Profiles

### Editor Workbench UI Target

The editor/workbench target covers:

- workbench profiles;
- suites;
- panels;
- docking, splits, and tabs;
- menus;
- shortcuts;
- tool surfaces;
- provider families;
- host policy;
- capability gates;
- editor command routing;
- diagnostics and tool-lab surfaces.

This target may depend on `domain/editor/editor_shell` and accepted Workbench
contracts.

### Game Runtime UI Target

The game-runtime target covers:

- HUD layers;
- health bars;
- stamina and mana bars;
- inventory screens;
- equipment screens;
- hotbars;
- quest trackers;
- dialogue UI;
- minimaps;
- damage numbers;
- nameplates;
- boss frames;
- world-space UI;
- split-screen UI;
- safe areas;
- gamepad, touch, keyboard, and mouse navigation;
- platform prompt glyphs;
- accessibility modes;
- game-runtime performance and readability budgets.

This target must not depend on editor shell ownership. It may use shared
`domain/ui` and future `domain/ui_definition` contracts plus game-specific
target extensions.

## Definition Composition And Strength Ordering

The composition model must support deterministic strength ordering for:

- base UI library;
- shared component recipes;
- target profile defaults;
- project, game, and editor overrides;
- skin and theme packages;
- platform overrides;
- accessibility modes;
- localization and text expansion modes;
- user preferences;
- preview overrides;
- host/runtime policy overlays.

The model is inspired by layered composition systems, but Runenwerk must define
its own deterministic UI/interface composition semantics. Conflict resolution
must be inspectable and must report the losing source, winning source, affected
target profile, and activation impact.

## Styling Model

The shared styling model must include:

- primitive tokens;
- semantic tokens;
- component tokens;
- state tokens;
- token alias graph;
- token modes;
- theme packages;
- skin packages;
- platform overrides;
- accessibility overrides;
- token cycle diagnostics;
- deterministic style resolution.

Style resolution must be reproducible from authored definitions and the selected
target profile. Token cycles, missing aliases, incompatible modes, and
accessibility overrides must produce typed diagnostics.

## Binding Model

UI definitions may bind to read-only domain-owned view models.

Editor/workbench examples include:

- asset browser;
- material inspector;
- diagnostics;
- graph editors;
- tool surfaces.

Game examples include:

- combat HUD;
- inventory;
- abilities;
- quests;
- dialogue;
- minimap;
- player status;
- party frames;
- boss frames;
- world-space UI.

UI definitions may emit validated intents only. Editor intent examples include
editor commands, menu intents, and shortcut intents. Game intent examples
include `UseItem`, `SelectInventorySlot`, `OpenQuest`, `ChooseDialogueOption`,
`PingMinimap`, and `ActivateHotbarSlot`.

UI definitions must not own or directly mutate domain truth.

## Typed Diagnostics Taxonomy

Designer diagnostics must be typed and grouped by:

- schema/version;
- id/reference;
- token/theme;
- layout/composition;
- command/shortcut/focus;
- host-policy/capability;
- provider/surface;
- accessibility;
- performance-budget;
- migration/compatibility.

Each diagnostic must define:

- source location;
- affected target/profile/host/suite;
- owning domain;
- severity;
- activation impact;
- suggested fix.

Denied capabilities and unsupported target-profile features must fail closed with
typed diagnostics.

## Fixture, Scenario, And Golden Evidence Model

The Designer/Lab platform must support evidence-oriented authoring:

- fixtures for empty, loading, error, denied, offline, and heavy states;
- replayable interaction scenarios;
- expected diagnostics and UI state transitions;
- golden projection snapshots;
- visual captures;
- accessibility reports;
- migration reports;
- performance budget reports.

Fixtures and scenarios must be target-profile-aware so the same Canonical UI IR
can be validated against editor/workbench and game-runtime expectations.

## Round-Trip Editing Guarantees

Visual editing must preserve:

- stable ids;
- deterministic formatting;
- visual/textual convergence through Canonical UI IR;
- compatible unknown field preservation where possible;
- schema fail-closed behavior;
- reviewable diffs;
- migration dry-run before activation.

Any edit that cannot produce a deterministic textual diff must remain a preview
only and must not activate.

## Non-Goals

This design does not itself:

- authorize Rust source, schema, runtime, or UI-surface changes;
- change renderer, product, or domain semantic ownership;
- create a standalone `domain/ui_definition`, `domain/game_ui`, or
  `domain/game/interface` crate;
- make external plugin sandbox decisions;
- create work state merely by naming PT/PM/WR decomposition labels.

Those are future boundary or implementation decisions owned by accepted designs,
ADRs where required, GitHub issues, and reviewed pull requests.

## Acceptance Basis

This design was promoted from `active` to `accepted` after confirming that:

- the domain split is unambiguous;
- editor/workbench and game-runtime target profiles are explicit;
- architecture governance keeps current ownership in `domain/ui/ui_definition`
  and requires a future accepted design or ADR before extracting
  `domain/ui_definition` or introducing a game-UI owner boundary;
- the accepted designs below it still support the current architecture;
- current repository documentation validation passes at the reviewed revision.

Promotion is a documentation/architecture state decision, not a production-track
or WR lifecycle event, and does not imply runtime or product completion.

## Validation

For design-only changes:

```text
cargo validate
git diff --check
CI=true pnpm --dir docs-site build
```

Code-bearing implementations add focused tests for the exact changed boundary.
Repository-owned exact-head CI and Documentation Build are the delivery
acceptance evidence.

## Acceptance Criteria

- `ui-designer-interface-lab-platform-design.md` exists and uses the generic
  title `UI Designer And Interface Lab Platform`.
- The design treats Workbench as one target profile and includes a separate
  game-runtime UI target profile.
- The design defines `domain/ui`, `domain/ui_definition`,
  `domain/editor/editor_definition`, `domain/editor/editor_shell`, future
  `domain/game_ui` or `domain/game/interface`, and `apps/runenwerk_editor`
  ownership boundaries.
- The canonical pipeline uses `Canonical UI IR`, `Target Projection Plan`,
  `Editor Workbench Projection`, and `Game Runtime UI Projection`.
- Workbench-only wording is not used as the platform-level description.
- The composition, styling, binding, diagnostics, fixture/golden, and round-trip
  sections are present.
- The design is sequenced through canonical roadmaps rather than a parallel
  production-track ledger.
- No implementation is inferred from PT/PM/WR labels alone.

<!-- BEGIN RUNENWERK:UI_COMPONENT_PLATFORM:designer-consumption -->
## Component Platform catalog consumption

UI Designer consumes the control catalog, property schemas, state schemas, token requirements, accessibility requirements, binding requirements, story reports, diagnostics, and surface proofs. Designer may preview and author around Component Platform packages, but it does not own reusable control behavior.
<!-- END RUNENWERK:UI_COMPONENT_PLATFORM:designer-consumption -->
