---
title: UI Designer Workbench Product Design
description: Accepted product design for the standalone and embedded Runenwerk UI Designer workbench, including catalog, canvas, inspector, scenario evidence, performance, and game UI dependency seams.
status: accepted
owner: editor
layer: domain/ui-definition / domain/editor / app
canonical: true
last_reviewed: 2026-09-12
related_adrs:
  - ../../adr/accepted/0001-use-domain-owned-commands.md
  - ../../adr/accepted/0004-separate-description-from-execution.md
  - ../../adr/accepted/0005-projections-are-derived-state.md
  - ../../adr/superseded/0006-editor-surface-provider-plugin-seam.md
  - ../../adr/accepted/0009-ui-interaction-formation-v2.md
  - ../../adr/accepted/0010-graph-substrate-canvas-boundary.md
  - ../../adr/superseded/0012-capability-workbench-clean-break.md
related_designs:
  - ./ui-designer-interface-lab-platform-design.md
  - ../implemented/ui-lab-productization-design.md
  - ../implemented/editor-product-ux-lab-and-game-ui-ready-foundations-design.md
  - ../active/editor-ui-workspace-tool-surface-architecture.md
  - ../deferred/game-runtime-ui-projection-and-hud-platform-design.md
  - ./ui-designer-canonical-ir-and-composition-design.md
  - ./ui-designer-target-projection-profiles-design.md
  - ./ui-designer-visual-layout-and-interface-composition-design.md
  - ./ui-designer-theme-tokens-modes-skins-and-state-variants-design.md
  - ./ui-designer-component-surface-and-widget-recipe-library-design.md
  - ./ui-designer-view-model-capability-and-intent-binding-design.md
  - ./ui-designer-live-preview-fixtures-scenarios-and-target-matrix-design.md
  - ./ui-designer-persistence-migration-diff-and-activation-design.md
  - ./ui-designer-production-readiness-and-evidence-design.md
  - ./ui-lab-command-catalog-and-surface-registry-design.md
  - ./ui-lab-app-hosted-editor-lab-surface-shell-design.md
  - ./ui-lab-operation-driven-visual-authoring-design.md
  - ./ui-lab-persistence-project-io-diff-apply-rollback-design.md
  - ./ui-lab-preview-lab-runtime-evidence-design.md
related_roadmaps:
  - ../../domain/ui/roadmap.md
---

# UI Designer Workbench Product Design

## Status

This is an accepted product design, not an implementation contract.

It defines the target shape for a proper Runenwerk UI Designer workbench. It
does not authorize code changes, work-state changes, completion claims, or
game-runtime UI implementation by itself.

Implementation still requires architecture governance when ownership changes,
an owning GitHub issue, canonical roadmap sequencing where relevant, and a pull
request that owns review and exact-head validation evidence.

## Purpose

Runenwerk needs a first-class UI Designer workbench that lets authors create,
inspect, validate, preview, persist, diff, and apply UI/interface definitions
through direct product controls.

The workbench must close the current gap between strong UI Designer contracts
and the app-visible authoring surface. Existing contracts already define
Canonical UI IR, target profiles, visual edit operations, tokens, recipe
libraries, view-model bindings, preview scenarios, persistence, and production
evidence. This design defines the product workspace that makes those contracts
usable.

## Product Decision

The UI Designer is a real workbench with two host forms:

- standalone app, similar in status to Material Lab;
- embedded `Editor Design` workspace inside the main editor.

Both host forms use the same Designer documents, Canonical UI IR pipeline,
recipe/catalog contracts, target projection profiles, scenario matrices,
diagnostics, and evidence model.

The standalone app is the primary product proof. The embedded workspace is a
hosted integration of the same product, not a separate debug surface.

## Critical Review Findings

This section records the design review concerns that must shape any later
implementation. The first draft had the right direction, but it was too easy to
misread as a broad aspiration instead of a bounded product contract.

Required corrections:

- define a bounded V1 product slice before implementation starts;
- name normal author workflows, not only panels and contracts;
- define the document/package model so app state does not become hidden source
  truth;
- make code-truth and planning-truth drift a hard preflight gate;
- separate compatibility proof for `game.runtime` from game HUD implementation;
- treat performance and resize as product acceptance gates, not polish;
- require visible evidence for catalog, canvas, inspector, diagnostics, diff,
  and scenario behavior before any runtime-proven claim.

The revised design below uses those findings as constraints.

## Non-Scope

This design does not:

- implement the standalone app;
- implement game-runtime HUD behavior;
- reopen completed `PT-UI-DESIGN` or `PT-UI-LAB` decomposition milestones;
- move generic UI truth into `apps/runenwerk_editor`;
- make projection output authoritative state;
- make editor shell policy part of `game.runtime` UI contracts;
- replace GitHub issue/PR authority or canonical roadmap sequencing;
- claim `runtime_proven` or `perfectionist_verified` quality.

## Architecture Governance Result

Architecture governance for this design records:

- DDD bounded context owner for generic UI definition truth:
  `domain/ui/ui_definition`.
- DDD bounded context owner for token graph truth: `domain/ui/ui_theme`.
- DDD bounded context owner for editor/workbench extensions:
  `domain/editor/editor_definition` and `domain/editor/editor_shell`.
- App owner for concrete hosted product behavior: `apps/runenwerk_editor`.
- Game-runtime UI remains a downstream target-profile and runtime owner
  problem. This workbench must prove compatibility seams without implementing
  game HUD behavior.
- Team Topologies label: stream-aligned editor product work with
  complicated-subsystem support from `domain/ui`, `domain/editor`, engine
  runtime, and app evidence owners.
- ATAM-lite priority order: ownership/correctness first, deterministic diffs
  and fail-closed diagnostics second, author ergonomics third, runtime evidence
  fourth, performance fifth.

No new ADR is required for this active design while it only defines product
shape and preserves existing ownership. A new ADR or accepted design update is
required before adding a game UI owner crate, changing dependency direction,
moving generic UI truth into app code, or making preview/evidence/projection
artifacts authoritative.

## Current Code Truth

The current code has UI Designer-related contracts and surfaces, but not a full
product workbench.

- `apps/runenwerk_editor/src/runtime/app.rs::RunenwerkRuntimeWorkbench` has
  `FullEditor` and `MaterialLab`; there is no standalone UI Designer workbench
  variant.
- `apps/runenwerk_editor/src/runtime/app.rs::run_material_lab_workbench`
  provides the Material Lab standalone pattern that a future UI Designer app
  should follow.
- `apps/runenwerk_editor/src/bin/runenwerk_material_lab.rs` exists; there is no
  equivalent `runenwerk_ui_designer.rs` binary.
- `apps/runenwerk_editor/src/shell/providers/self_authoring.rs::ui_designer_workbench_view_model`
  builds a UI Designer workbench view model, but current normal workflows still
  lack a complete product catalog, direct canvas editing, full inspector, and
  scenario/evidence flow.
- `domain/editor/editor_shell/src/composition/build_editor_lab_surface.rs::build_ui_designer_workbench`
  composes a workbench-like surface, but this is not yet the complete product
  described here.
- `apps/runenwerk_editor/src/runtime/systems/frame_submit.rs::submit_editor_frame_system`
  rebuilds the shell expression frame during frame submission.
- `apps/runenwerk_editor/src/shell/controller.rs::build_expression_frame_with_surface_resources`
  rebuilds frame model and UI tree projection artifacts for the shell path.
- `apps/runenwerk_editor/src/shell/controller.rs::handle_split_resize_event`
  updates split fractions on pointer movement, which can drive expensive
  rebuild/relayout work during resize.

The planning sources currently contain evidence claims for later UI/editor UX
milestones. This design treats source code and runtime evidence as the
implementation truth for product readiness. Any planning drift must be
reconciled by a separate planning or closeout repair, not by this design alone.

### Drift Preflight Gate

Before implementing this workbench, run a code-truth and planning-truth
preflight. The preflight must classify every UI Designer and Editor UX planning
claim as one of:

- code-backed and evidence-backed;
- code-backed but missing product evidence;
- design-backed only;
- stale or contradictory;
- unrelated to the standalone UI Designer workbench.

No implementation slice may use a stale planning claim as permission to skip
catalog, canvas, inspector, evidence, or performance work.

## External Reference Patterns

The workbench should learn from mature tools without copying their architecture
blindly:

- Figma Auto Layout: responsive layout needs first-class flow, spacing,
  padding, resizing, and grid behavior. Reference:
  <https://help.figma.com/hc/en-us/articles/360040451373-Guide-to-auto-layout>
- Figma components, variants, properties, variables, and libraries:
  component reuse must be discoverable through assets/catalogs and editable
  through property controls. References:
  <https://help.figma.com/hc/en-us/articles/5579474826519-Explore-component-properties>,
  <https://help.figma.com/hc/en-us/articles/360056440594-Create-and-use-variants>,
  <https://help.figma.com/hc/en-us/articles/15343816063383-Modes-for-variables>,
  <https://help.figma.com/hc/en-us/articles/360025508373-Publish-a-library>
- Storybook Controls and interaction tests: component scenarios need live args,
  state controls, and executable interaction evidence. References:
  <https://storybook.js.org/docs/essentials/controls>,
  <https://storybook.js.org/docs/9/writing-tests/interaction-testing>
- Unity UI Builder: a usable editor shape includes hierarchy, library,
  viewport/canvas, code preview, stylesheets, and inspector. Reference:
  <https://docs.unity.cn/Packages/com.unity.ui.builder%401.0/manual/uib-interface-overview.html>
- Unreal UMG Widget Blueprint Editor: game UI authoring benefits from distinct
  Designer/Graph modes, palette, hierarchy, visual designer, details, and
  animations. Reference:
  <https://dev.epicgames.com/documentation/unreal-engine/widget-blueprints-in-umg-for-unreal-engine>
- Godot UI: game UI needs Control nodes, content controls, layout controls,
  containers, anchors, skinning, and themes as first-class concepts. Reference:
  <https://docs.godotengine.org/en/4.4/tutorials/ui/index.html>
- Material Design and Material Web: tokens, component categories, states, and
  accessible interaction patterns are useful as system structure, not as a
  visual style to copy. References:
  <https://developer.android.google.cn/design/ui/mobile/guides/components/material-overview>,
  <https://material-web.dev/theming/material-theming/>
- Rive: design and animate modes, state machines, and data-binding-style
  runtime contracts are useful precedents for mode-specific authoring and
  designer/developer collaboration. References:
  <https://rive.app/docs/editor/state-machine>,
  <https://rive.mintlify.dev/docs/editor/fundamentals/design-vs-animate-mode>

## Workbench Information Architecture

The UI Designer workbench uses a stable, product-oriented layout:

```text
top bar
  document, target profile, mode, scenario, evidence, apply status

left rail / left panel
  library/catalog
  hierarchy/navigator

center
  canvas / target preview / scenario playback

right panel
  inspector
  properties
  tokens/theme/state
  binding/intent
  accessibility

bottom panel
  diagnostics
  diff/apply review
  preview console
  evidence manifest
```

The default first screen is the usable Designer, not a landing page, marketing
page, or status-only placeholder.

## V1 Product Cut

V1 is the smallest honest version of the full workbench. It must be useful for
real authoring, but it may intentionally defer broad package ecosystems and
advanced game-runtime behavior.

V1 includes:

- standalone app launch;
- embedded `Editor Design` host parity;
- one editable UI package loaded from app-owned fixture/project state;
- searchable recipe catalog with target-profile filtering;
- hierarchy/navigator with stable authored ids and diagnostics badges;
- canvas preview with selection, insertion guides, and degraded-state handling;
- inspector for selected document, node, layout, recipe, token reference,
  binding reference, and accessibility descriptor;
- operation-driven insert, move, reorder, layout edit, token-reference edit,
  binding-reference edit, and accessibility edit;
- Canonical UI IR diff and deterministic textual patch preview;
- apply/reject/rollback review flow;
- scenario matrix with at least `editor.workbench` and `game.runtime`
  compatibility descriptor axes;
- source-versioned evidence packet;
- focused performance counters for frame build, canvas projection, catalog
  projection, diagnostics projection, and resize relayout behavior.

V1 explicitly defers:

- arbitrary third-party recipe package loading;
- full package publishing workflow;
- multiplayer/collaborative editing;
- visual animation timeline authoring;
- full game HUD runtime behavior;
- world-space/entity-attached UI;
- renderer-owned GPU UI specialization;
- perfectionist no-gap certification.

## Required V1 Workflows

The workbench is not complete because panels exist. It is complete only when
normal author workflows can be completed without debug action lists.

### Shared Component Catalog Workflow

An author can:

1. open the standalone UI Designer app;
2. browse searchable component, widget, surface, layout, and template recipes;
3. filter by `editor.workbench` or `game.runtime` compatibility;
4. inspect a recipe preview, required slots, required tokens, supported states,
   and accessibility requirements;
5. insert a compatible recipe into the current document;
6. see the hierarchy, canvas, inspector, diagnostics, and diff update from the
   same source version.

### Editor Tool Panel Workflow

An author can:

1. open an editor/workbench panel document;
2. insert or move layout containers and controls;
3. edit layout, token references, labels, accessibility metadata, and command
   intent descriptors;
4. run an editor/workbench scenario;
5. inspect diagnostics and deterministic textual diff;
6. apply or reject the change;
7. reload and prove the authored state survived persistence.

### Game Runtime Compatibility Workflow

An author can:

1. select the `game.runtime` target profile;
2. inspect compatible HUD-oriented recipes and incompatible editor-only recipes;
3. preview safe-area, input-modality, localization, accessibility, and sizing
   axes through descriptors and fixtures;
4. bind only to read-only fixture view-model packets;
5. emit intent proposals only as validated descriptors;
6. capture compatibility evidence without implementing runtime HUD behavior.

This workflow proves readiness seams. It is not a game UI runtime proof.

## Document And Package Model

The workbench edits packages and documents. It does not edit runtime widgets as
source truth.

### UI Package

A UI package is the app-visible unit of authoring. It contains:

- package id and schema version;
- source package provenance;
- target-profile support declarations;
- UI documents;
- recipe package references;
- token/theme package references;
- view-model package references;
- scenario and fixture references;
- migration and evidence metadata.

### UI Document

A UI document is the authored source for one surface, component, widget,
layout, template, or scenario-facing preview root.

It contains:

- stable document id;
- document kind;
- target-profile compatibility;
- Canonical UI IR source-map path;
- root node or recipe reference;
- authored metadata;
- diagnostics provenance.

### Workbench Session

A workbench session is app-owned and derived from the open package. It contains:

- current document id;
- current mode;
- current target profile;
- current scenario;
- selection;
- pan/zoom and panel layout;
- preview overrides;
- dirty operation set;
- evidence capture state.

Session state is not source truth. It must be disposable and reconstructable
from package documents plus app preferences.

### Evidence Packet

An evidence packet is source-versioned derived output. It contains:

- evidence packet id;
- package id and source version;
- document id;
- target profile;
- scenario id;
- captured artifacts or typed unsupported reasons;
- diagnostics snapshot;
- performance counters;
- freshness status.

Evidence packets may block claims. They must not become authored UI truth.

## Workbench Modes

The workbench has explicit modes. Modes change available tools and inspector
sections, but they do not change source-truth ownership.

### Design Mode

Design mode edits authored UI structure:

- insert recipes;
- move nodes;
- reorder children;
- wrap and unwrap containers;
- edit layout behavior;
- edit slots;
- select and inspect nodes;
- preview deterministic textual diffs.

### Theme Mode

Theme mode edits token, mode, skin, and state-variant references:

- primitive tokens;
- semantic tokens;
- component tokens;
- state variants;
- mode overrides;
- skin and platform overrides;
- accessibility overrides;
- token provenance and diagnostics.

### Bind Mode

Bind mode connects UI definitions to external read-only data and validated
intent proposals:

- view-model package browser;
- field bindings;
- formatting and fallback policy;
- capability requirements;
- intent declarations;
- denied-capability diagnostics;
- stale/missing view-model diagnostics.

### Preview Mode

Preview mode runs target-profile-aware scenarios:

- target profile selection;
- viewport and safe-area selection;
- input modality selection;
- localization and text expansion;
- accessibility modes;
- theme/mode/skin variants;
- scenario replay;
- performance/readability evidence capture.

### Review Mode

Review mode is the activation gate:

- Canonical UI IR diff;
- deterministic textual patch;
- diagnostics;
- migration dry-run;
- apply/reject/rollback;
- evidence freshness;
- source-map paths;
- unresolved risk list.

## Product Surfaces

### Top Bar

The top bar shows:

- current project/package;
- current document;
- target profile, such as `editor.workbench` or `game.runtime`;
- current mode;
- scenario or fixture;
- dirty state;
- validation state;
- evidence freshness;
- apply/review state.

The top bar may route commands, but it must not become the source of command
truth. Commands come from the owning command catalog and app bridge.

### Library And Catalog

The library is the normal entry point for adding UI.

It must support:

- searchable component, widget, surface, layout, and template recipes;
- target-profile filtering;
- package/source provenance;
- preview thumbnails or retained-preview snippets;
- supported states and modes;
- slot compatibility;
- required token families;
- accessibility requirements;
- disabled/unavailable reasons.

The library must be virtualized and cached. Large recipe packages must not
force every thumbnail or retained preview to rebuild every frame.

Catalog rows must show whether an item is:

- compatible and insertable;
- compatible but missing required context;
- incompatible with the current target profile;
- hidden until an experimental flag or owning package is enabled;
- deprecated or migration-only.

Disabled entries must explain the blocking rule and source package.

### Hierarchy And Navigator

The hierarchy shows authored document structure, not runtime widget accidents.

It must support:

- stable authored ids;
- source-map paths;
- recipe references;
- slot paths;
- hidden, locked, degraded, and diagnostic badges;
- drag/drop reorder through visual edit operations;
- selection synchronization with canvas and inspector;
- search and filtering;
- explicit unsupported state messages.

Hierarchy edits must produce Canonical UI IR diffs before activation.

### Canvas And Preview

The canvas is a tool-surface/canvas product area over retained UI products.

It must support:

- target frame selection;
- pan and zoom;
- selection overlays;
- insertion guides;
- slot guides;
- resize handles;
- drag/drop recipe insertion;
- layout container affordances;
- safe-area overlays;
- input-mode preview;
- scenario playback;
- degraded preview states with typed reasons.

The canvas must not mutate runtime widgets directly. Direct manipulation emits
typed visual edit operations.

### Inspector And Properties

The inspector shows fields for the selected authored entity:

- document;
- node;
- recipe instance;
- slot;
- layout container;
- token reference;
- state variant;
- binding;
- intent;
- accessibility descriptor;
- target-profile compatibility descriptor.

Inspector controls must be typed. Numeric fields use numeric controls, boolean
fields use toggles, enums use select/segmented controls, colors use swatches,
and unsupported fields show disabled reasons.

### Tokens, Themes, States, And Skins

The token surface exposes the deterministic token graph:

```text
primitive tokens
  -> semantic tokens
  -> component tokens
  -> state variants
  -> mode overrides
  -> theme package
  -> skin package
  -> platform overrides
  -> accessibility overrides
  -> preview overrides
```

Token editing must preserve provenance and produce deterministic diffs. Preview
overrides are not activation truth unless converted into authored token graph
diffs.

### Binding And Intent Surface

The binding surface exposes read-only view-model packets and validated intents.

It must support:

- available view-model packages;
- field paths and value types;
- binding expressions allowed by accepted contracts;
- fallback text/value policy;
- intent proposal descriptors;
- capability requirements;
- denied, missing, stale, and incompatible diagnostics.

UI definitions emit intent proposals. Owning domains, apps, or runtime examples
apply accepted mutation.

### Scenario Matrix And Evidence

The scenario matrix is the native equivalent of a component lab:

- target profile;
- platform/device/safe area;
- viewport size and density;
- input modality;
- localization/text expansion;
- theme/mode/skin;
- accessibility settings;
- state fixture;
- performance/readability budget;
- expected diagnostics and evidence packet names.

Scenario output is derived evidence. It must not become source truth.

### Diff, Apply, Rollback, And Diagnostics

The review surface must show:

- Canonical UI IR diff;
- deterministic textual patch;
- source-map paths;
- validation diagnostics;
- migration dry-run diagnostics;
- target-profile compatibility diagnostics;
- accessibility diagnostics;
- performance/readability diagnostics when available;
- apply result;
- rollback path.

Unsupported, denied, stale, or unsafe states fail closed with typed diagnostics.

### Preview Console

The console shows recent workbench events:

- command proposal;
- operation result;
- provider event;
- preview scenario event;
- diagnostics event;
- evidence capture event.

It is not a replacement for product controls.

## Authoring Pipeline

Every authoring gesture follows the same shape:

```text
user gesture
  -> typed workbench operation
  -> Canonical UI IR diff
  -> deterministic textual patch
  -> validation
  -> target projection preview
  -> review
  -> activation or rejection
```

If a gesture cannot produce a deterministic Canonical UI IR diff, it remains
preview-only and must explain why.

## Recipe And Library Contract

A recipe entry must expose:

- stable recipe id;
- kind: component, widget, surface, layout, template, or pattern;
- human label and category;
- target-profile compatibility;
- Canonical UI IR node template;
- named slots and accepted children;
- required token families;
- default token ids where allowed;
- supported state variants;
- accessibility role and label strategy;
- focus and navigation descriptors;
- source package/provenance;
- preview fixtures;
- diagnostics.

The app may host recipe browsers and preview surfaces. Generic recipe truth
stays in `domain/ui/ui_definition` and token resolution stays in
`domain/ui/ui_theme`.

## Standalone App Contract

The standalone app should follow the Material Lab pattern without sharing
Material Lab semantics.

Future implementation belongs in:

- `apps/runenwerk_editor/src/runtime/app.rs::RunenwerkRuntimeWorkbench` for the
  new workbench variant;
- `apps/runenwerk_editor/src/runtime/app.rs::run_ui_designer_workbench` for the
  standalone launch path;
- `apps/runenwerk_editor/src/bin/runenwerk_ui_designer.rs` for the binary entry;
- `apps/runenwerk_editor/src/editor_app/state.rs` for app-owned Designer
  session state;
- `apps/runenwerk_editor/src/shell/workbench_host.rs` for concrete app
  composition;
- `apps/runenwerk_editor/src/shell/providers/` for app provider view-model
  construction and command bridges.

The standalone app must launch directly into the Designer workbench. It must not
require opening the full editor and then discovering an internal workspace.

## Embedded Editor Workspace Contract

The embedded `Editor Design` workspace hosts the same product contracts inside
the full editor shell.

Future implementation touches:

- `domain/editor/editor_shell/src/workspace/state.rs::bootstrap_editor_design_layout`
  for workspace layout;
- `domain/editor/editor_shell/src/composition/` for product-oriented
  composition helpers;
- `domain/editor/editor_shell/src/surfaces/editor_definition.rs` for
  app-neutral view models;
- `apps/runenwerk_editor/src/shell/providers/self_authoring.rs` or successor
  provider modules for app-owned view-model translation.

The embedded workspace may adapt to available editor host context, but normal
Designer workflows must not depend on generic debug text/action panels.

## Product Readiness States

Every visible workbench surface has an explicit readiness state:

- `Product`: normal author workflow is implemented and evidence-backed.
- `FallbackOnly`: visible only for unsupported, missing-provider, or degraded
  conditions.
- `Diagnostic`: visible as an inspection/debug aid and labelled as such.
- `HiddenUntilProductized`: not reachable in normal workflows.

The workbench cannot claim product readiness while normal workflows expose
generic text/action self-authoring panels, misleading placeholders, or
unlabelled diagnostic surfaces.

## Dependency Direction

Allowed direction:

```text
foundation
  -> domain/ui
  -> domain/editor
  -> engine/runtime
  -> apps/runenwerk_editor
```

The workbench may consume:

- `domain/ui/ui_definition` contracts;
- `domain/ui/ui_theme` contracts;
- retained UI and UI runtime primitives;
- `domain/editor/editor_definition` editor/workbench packages;
- `domain/editor/editor_shell` Workbench host contracts;
- app command bridges and provider fixtures.

The workbench must not move:

- generic UI definition truth into app code;
- editor provider vocabulary into `game.runtime` contracts;
- game HUD semantics into editor shell;
- gameplay mutation into UI definition or projection code;
- render/runtime truth into Designer documents.

## Game UI Dependency Order

Game-runtime UI implementation must stay downstream of Designer workbench proof.

```text
UI Designer Workbench Product Design
  -> UI Designer workbench foundations
  -> standalone workbench runtime proof
  -> editor surface readiness and shell polish
  -> game UI readiness seam
  -> game-runtime UI target extension
  -> read-only game view models and validated intents
  -> runtime UI expression submission
  -> SDF screen HUD proof
```

The game UI readiness seam proves that recipes, target profiles, fixtures,
safe-area/layout axes, input-modality axes, localization axes, accessibility
axes, evidence descriptors, and diagnostics can represent `game.runtime` UI.
It does not implement game HUD behavior.

Game HUD implementation starts only after an accepted game-runtime owner
boundary exists. The first game-runtime UI implementation must consume the
Designer contracts through read-only view-model packages, validated intent
proposals, and engine UI expression submission. It must not backfill missing
Designer features with runtime-only shortcuts.

## Performance And Resize Contract

The UI Designer must be designed for interaction performance before the surface
is large.

Required performance rules:

- Workbench frame construction is dirty-driven where possible.
- Library, hierarchy, inspector, diagnostics, and canvas view models are
  memoized by document id, source version, target profile, scenario id,
  selection id, and relevant filter state.
- Canvas projection is isolated from shell chrome so canvas changes do not
  rebuild unrelated panels.
- Library previews and scenario thumbnails are virtualized and cached.
- Resize uses a transaction model: live overlay or throttled update during
  pointer move, final authored split/layout commit on controlled cadence or
  pointer-up.
- Text shaping and layout data are cached across stable labels, fonts, and
  bounds.
- Diagnostics update by affected document subtree where the owning ratifier can
  identify a bounded invalidation region.
- Evidence capture is explicit and source-versioned, not an accidental side
  effect of every frame.

Initial performance evidence must capture:

- total frame build time;
- shell chrome build time;
- canvas projection time;
- library/catalog projection time;
- diagnostics projection time;
- resize event frequency;
- relayout count during resize;
- scenario preview build time;
- cache hit/miss counts for major view-model families.

Initial acceptance budgets are qualitative until measured baselines exist:

- resizing a split or canvas frame must remain responsive under continuous
  pointer movement;
- resize interaction must not rebuild unchanged catalog rows;
- resize interaction must not recompute unchanged diagnostics;
- canvas selection movement must not rebuild unrelated shell chrome;
- catalog search/filter must remain virtualized for large recipe sets;
- evidence capture must be explicit and may not run implicitly every frame.

The first implementation slice must record measured baselines and then replace
these qualitative rules with concrete budgets.

## Invariants

- Authored UI definitions are the only source truth for UI/interface structure.
- Visual editing and textual editing converge through Canonical UI IR.
- Stable authored ids survive visual edits, resize, relayout, preview, and
  activation.
- Projection output is derived state.
- Preview overrides are not activation truth.
- UI definitions bind only to read-only view-model packets.
- UI definitions emit validated intent proposals only.
- App providers execute or bridge commands through owning domains.
- Unsupported features fail closed with typed diagnostics.
- Normal product workflows do not depend on generic debug action lists.
- Game-runtime UI contracts do not depend on editor shell ownership.
- Workbench session state is reconstructable and does not become source truth.
- Product readiness claims require runtime evidence, not only descriptors.
- Planning claims are not evidence unless backed by current closeout artifacts
  and code truth.

## Failure Modes And Diagnostics

The workbench must diagnose:

- unknown document/package/schema version;
- duplicate authored id;
- invalid source-map path;
- unknown recipe id;
- incompatible target profile;
- invalid slot child;
- missing required slot;
- missing required token family;
- unresolved token alias;
- unsupported state variant;
- missing accessibility semantics;
- invalid focus/navigation descriptor;
- unknown view-model package;
- stale or missing view-model data;
- invalid binding value type;
- denied capability;
- invalid intent payload;
- non-deterministic visual operation;
- failed textual patch generation;
- failed migration dry-run;
- stale evidence packet;
- unsupported preview route;
- planning/code-truth mismatch when a readiness claim is stale;
- product surface exposed as normal workflow without product readiness.

Diagnostics include stable code, severity, source path, owning package, target
profile, suggested recovery where safe, and whether activation is blocked.

## Public API And Ergonomics

Public APIs must support normal workflows first:

- create/open UI package;
- browse recipes;
- insert recipe;
- select node;
- edit properties;
- bind view-model field;
- declare intent;
- run scenario;
- inspect diagnostics;
- review diff;
- apply or reject;
- capture evidence;
- reload and verify persisted state.

Advanced APIs for package migration, custom target profiles, custom recipe
packages, and evidence harnesses belong in owning modules rather than broad
prelude exports unless they become common workflows.

Usage docs and examples must show complete, realistic authoring flows rather
than internal shortcuts.

## Implementation Order

This is design-level sequencing, not work authorization.

1. Reconcile code truth and planning truth for the existing UI Designer and
   Editor UX claims.
2. Lock the V1 package/document/session/evidence model and route it through
   accepted design or implementation-contract review.
3. Add or ratify app-neutral workbench view-model families in
   `domain/editor/editor_shell/src/surfaces/editor_definition.rs`.
4. Build the standalone shell route and binary entry without adding new source
   truth.
5. Replace normal debug/self-authoring action panels with product surfaces:
   catalog, hierarchy, canvas, inspector, diagnostics, diff/review, console.
6. Add operation-driven visual editing over Canonical UI IR diffs.
7. Add token/theme/state and binding/intent panels.
8. Add scenario matrix and evidence capture.
9. Add performance instrumentation and resize transaction behavior.
10. Prove embedded `Editor Design` uses the same product contract.
11. Only then run the game UI readiness seam before game-runtime HUD work.

Each code-bearing slice is activated by an owning GitHub issue and delivered
through a reviewed pull request; this order alone grants no permission to
implement.

## Fitness Functions

Before any implementation slice can claim product progress, it needs focused
fitness functions for the exact boundary it touches.

Required first fitness functions:

- standalone launch opens a UI Designer workbench, not the full editor or
  Material Lab;
- catalog entries show target-compatible recipes with disabled reasons for
  incompatible entries;
- recipe insertion produces a Canonical UI IR diff and deterministic textual
  patch;
- hierarchy selection, canvas selection, and inspector selection stay in sync;
- inspector edits produce typed operations, not ad hoc app mutation;
- unsupported operations remain preview-only and emit typed diagnostics;
- resize does not rebuild unchanged library/catalog and diagnostics projections;
- game-runtime target-profile preview descriptors do not import editor shell
  vocabulary;
- evidence packets are source-versioned and stale evidence is rejected;
- reload after apply reconstructs the same authored package and document state;
- visible workbench surfaces report `Product`, `FallbackOnly`, `Diagnostic`, or
  `HiddenUntilProductized` readiness;
- a planning/code-truth drift report exists before implementation claims start.

## Acceptance Bar

The first implementation milestone may claim bounded product progress only when:

- the standalone app launches into the Designer workbench;
- the V1 shared component catalog workflow completes without debug action
  lists;
- the V1 editor tool panel workflow completes through deterministic diff,
  apply or reject, reload, diagnostics, and evidence capture;
- the V1 game runtime compatibility workflow proves descriptor axes without
  implementing HUD behavior;
- product readiness states classify every visible workbench surface;
- resize and canvas interaction have measured baseline evidence;
- docs and examples explain the normal workflow without internal shortcuts.

It may not claim:

- full game-runtime UI;
- perfectionist no-gap quality;
- arbitrary third-party library ecosystem readiness;
- completed package publishing;
- complete editor all-surface migration.

## Stop Conditions

Stop and redesign if:

- implementing a product workflow requires app-owned generic UI truth;
- a visual edit cannot round-trip through Canonical UI IR;
- a normal workflow still depends on generic text/action debug panels;
- game-runtime target-profile descriptors need editor shell types;
- resize or canvas interaction requires rebuilding unrelated heavy panels every
  pointer move;
- evidence cannot identify source version, target profile, scenario, and
  freshness;
- public examples require internal shortcuts to use normal workflows;
- implementation starts from game HUD runtime code instead of the Designer
  compatibility seam;
- product readiness is inferred from stale planning metadata without current
  code-backed evidence.

## Validation Plan

For documentation-only changes to this design:

```text
cargo validate
git diff --check
CI=true pnpm --dir docs-site build
```

Future code implementation must add focused tests for changed crates and pass
the repository-owned exact-head CI and Documentation Build at the reviewed
revision.

## Open Questions

- Should the standalone app be named `runenwerk_ui_designer`,
  `runenwerk_interface_lab`, or another product name?
- Should recipe package storage remain in `domain/ui/ui_definition` initially
  or move into a new crate after acceptance?
- What is the first hard performance budget for resize and canvas drag?
- Which evidence captures are locally native today, and which need typed
  platform-impossible results until renderer/app support improves?
