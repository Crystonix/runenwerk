---
title: UI Designer Target Projection Profiles Design
description: Accepted design for PM-UI-DESIGN-003 editor/workbench and game-runtime UI target projection profiles.
status: accepted
owner: editor
layer: domain/ui-definition
canonical: true
last_reviewed: 2026-05-22
related_adrs:
  - ../../adr/accepted/0004-separate-description-from-execution.md
  - ../../adr/accepted/0005-projections-are-derived-state.md
  - ../../adr/superseded/0012-capability-workbench-clean-break.md
related_designs:
  - ./ui-designer-canonical-ir-and-composition-design.md
  - ./ui-designer-interface-lab-platform-design.md
  - ../active/editor-tool-suite-registry-and-workbench-host-design.md
  - ../active/editor-ui-workspace-tool-surface-architecture.md
related_roadmaps:
  - ../../domain/ui/roadmap.md
---

# UI Designer Target Projection Profiles Design

## Status

This is the accepted design contract for `PM-UI-DESIGN-003`.

It accepts two target projection profiles for the UI Designer platform:
editor/workbench UI and game-runtime UI. It does not implement projection code,
does not create game-runtime UI crates, and does not authorize product code by
itself. Later implementation requires an owning GitHub issue, canonical roadmap
sequencing where relevant, and pull-request-owned review and exact-head
validation.

## Goal

Projection turns validated Canonical UI IR into target-specific plans while
keeping source truth in authored UI/interface definitions:

```text
Canonical UI IR
  + target profile
  + host/runtime policy
  + fixtures
  + validated composition
  -> Target Projection Plan
     -> Editor Workbench Projection
     -> Game Runtime UI Projection
```

Projection output is derived state. App, runtime, preview, and provider layers
consume projection output; they must not become canonical UI/interface truth.

## Architecture Governance Result

Architecture governance accepts this split:

- `domain/ui/ui_definition` owns target-profile declarations, projection-plan
  vocabulary, compatibility diagnostics, and source/provenance contracts that
  are generic to UI/interface definitions.
- `domain/editor/editor_definition` owns editor/workbench-specific profile
  extensions and authored editor binding metadata.
- `domain/editor/editor_shell` owns Workbench host vocabulary, provider
  declarations, suite/profile identity, command routing, docking/splits/tabs,
  host policy, and fail-closed editor projection compatibility checks.
- Future game-runtime UI extensions must be owned by a game-runtime UI domain
  design and must not depend on `domain/editor/editor_shell`.
- `apps/runenwerk_editor` owns concrete preview orchestration and app command
  bridges only.

No new ADR is required for PM-003 because this design preserves existing
Workbench ownership and explicitly prevents game-runtime UI from depending on
editor shell ownership. A future ADR or accepted design update is required
before adding a new game-runtime UI owner crate or making projection output
authoritative state.

## Target Profile Contract

Every target profile declares:

- stable target-profile id;
- profile family, such as `editor.workbench` or `game.runtime`;
- supported component and widget recipe families;
- supported layout capabilities;
- supported input/navigation modes;
- host/runtime policy overlays;
- capability gates;
- unsupported feature behavior;
- diagnostics emitted when projection cannot proceed.

Unsupported target-profile features fail closed with typed diagnostics. They do
not silently disappear and do not activate through fallback projection.

## Editor Workbench Projection

The editor/workbench target profile consumes Canonical UI IR through explicit
editor extensions.

The profile covers:

- workbench profiles;
- suites;
- panels;
- docking, splits, tabs, and floating hosts;
- menus;
- shortcuts;
- tool surfaces;
- provider families;
- host policy;
- capability gates;
- editor command routing;
- diagnostics and tool-lab surfaces.

The Workbench projection may depend on `domain/editor/editor_definition` and
`domain/editor/editor_shell` contracts. It may reuse PT-WB-CAP identity and host
policy vocabulary as historical/decomposition terminology.

Workbench projection must not:

- execute app commands directly;
- own provider behavior;
- decide material, scene, asset, gameplay, render, or project truth;
- leak editor shell ownership into game-runtime UI projection.

## Game Runtime UI Projection

The game-runtime target profile consumes Canonical UI IR without depending on
editor shell ownership.

The profile covers:

- HUD layers;
- health, stamina, mana, and resource bars;
- inventory and equipment screens;
- hotbars;
- quest trackers;
- dialogue UI;
- minimaps;
- damage numbers;
- nameplates and boss frames;
- world-space UI;
- split-screen UI;
- safe areas;
- platform prompt glyphs;
- gamepad, touch, keyboard, and mouse navigation;
- accessibility modes;
- game-runtime performance and readability budgets.

Game-runtime projection may use shared `domain/ui` and `domain/ui/ui_definition`
contracts plus future game-runtime UI target extensions. It must not import
`domain/editor/editor_shell`, editor Workbench host policy, editor command
routing, or editor provider vocabulary.

## Reproducibility

Projection must be reproducible from:

- authored UI/interface definitions;
- Canonical UI IR;
- deterministic composition results;
- target profile;
- host/runtime policy;
- fixtures;
- validation options.

Projection reports must expose:

- source definition id;
- target profile id;
- composition revision or equivalent deterministic input identity;
- host/runtime policy overlay identity;
- fixtures used for preview/proof;
- diagnostics emitted before projection;
- derived projection output identity.

## Diagnostics

Projection diagnostics include:

- stable diagnostic code;
- severity;
- source location;
- target profile;
- host or runtime policy source;
- owning domain;
- unsupported feature or denied capability;
- activation impact;
- suggested fix.

Denied capabilities, unsupported target-profile features, invalid host policy,
missing provider family references, invalid command routing, and unsafe
game-runtime/editor coupling all fail closed.

## Implementation Sequence

PM-003 is design-only. Later implementation must:

1. Keep PM-002 Canonical UI IR and deterministic composition as the input.
2. Create or select an owning GitHub issue before any code-bearing target
   projection slice.
3. Keep the issue scope bounded and add focused tests for profile compatibility,
   unsupported features, projection reproducibility, and fail-closed
   diagnostics.
4. Keep Workbench projection and game-runtime projection separated by explicit
   extension ownership.
5. Deliver through a pull request with one unchanged reviewed feature head and
   current repository validation evidence.

Historical `PT-*`, `PM-*`, and `WR-*` labels may remain as decomposition or
provenance vocabulary; they do not grant current implementation authority.

## Non-Goals

PM-003 does not:

- implement projection code;
- implement visual layout editing;
- implement theme or token resolution;
- implement component recipe libraries;
- implement binding activation;
- implement preview scenario matrices;
- implement persistence activation;
- create a game-runtime UI crate;
- make projection output authoritative state.

## Acceptance Bar

PM-003 is accepted when:

- this document is in `design/accepted` with `status: accepted`;
- editor/workbench and game-runtime profiles are explicit;
- the game-runtime target has no dependency on editor shell ownership;
- reproducibility inputs are named;
- fail-closed diagnostics are named;
- current repository validation passes at the accepted revision.
