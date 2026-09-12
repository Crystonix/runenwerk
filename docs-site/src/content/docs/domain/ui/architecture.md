---
title: UI Domain Current-State Architecture
description: Current-state architecture, ownership boundaries, and migration direction for Runenwerk UI.
status: active
owner: ui
layer: domain
canonical: true
last_reviewed: 2026-09-12
---

# UI Domain Current-State Architecture

This document records current code truth and current UI-domain ownership
boundaries.

For the top-down target framework architecture, see:
[Runenwerk UI Framework Architecture](../../architecture/ui-framework-architecture.md).

This file must not duplicate the whole target architecture. It records current
reality, current crate map, current migration seams, and current known gaps.

## Purpose
Establish the factual, current-state architecture for Runenwerk UI, define correct ownership boundaries, and document remaining migration direction from implemented substrate extraction to full editor/runtime convergence.

This page is intentionally current-state-first.

## Scope
This document covers:

- `domain/ui/*` crates
- retained UI runtime crates under `domain/ui/*`
- temporary workspace/tool-surface host ownership in `editor_shell`
- runtime/app glue in `apps/runenwerk_editor`
- engine render/UI integration paths used to submit and draw UI frames
- general UI definition/formation contracts where they clarify `domain/ui` ownership

This document does not define visual design direction, docking product UX, or authored editor-definition workflows.

## Current Reality
As of the audited repository state:

- `domain/ui/*` now owns both primitive crates and retained runtime crates (`ui_tree`, `ui_runtime`, `ui_widgets`).
- `domain/ui/ui_composition` now owns the first app-neutral structural
  composition and persistence checkpoints: versioned definitions, ratified
  structural state, typed policy-authorized transactions, structural-only
  history, explicit promotion, content-liveness vocabulary, neutral fixture
  contracts, canonical linked bundles, and atomic generation activation.
- Editor and Draw now form and project static structure through
  `ui_composition`; legacy editor workspace structure is read-only
  compatibility input until docking runtime completes.
- `domain/ui/ui_adaptive_composition` now derives headless projection, reflow,
  hit-test, preview, drag/resize-session, proposal, accessibility, and explicit
  promotion-delta products from immutable composition snapshots. It has no
  structural commit authority and no editor, Draw, engine, windowing, or
  product semantics.
- `ui_surface` remains temporary compatibility input for later mapped
  replacement. It is not a parallel target authority.
- Workspace identity/projection/reducer/tool-surface host infrastructure is implemented and belongs to `editor_shell`.
- `apps/runenwerk_editor` owns app runtime bridging and viewport runtime resources/bindings.
- Engine render integration for UI frame submission/extraction is implemented.
- Viewport slot semantic ownership is in `editor_viewport`; renderer-facing embed payload slots are opaque IDs in `ui_render_data`, mapped through integration adapters.
- Core shell control flows (outliner entity selection, viewport product selection, inspector field activation) now route through prepared `SurfacePresentationModel` + typed `SurfaceIntent` + host-side ratification adapters.
- Engine overlay/debug UI paths now route through substrate frame generation (`ui_runtime::build_ui_frame`) instead of ad hoc primitive assembly.
- `domain/ui/ui_definition` now provides the first authored UI definition and retained formation layer. Toolbar/menu structure, normal shell chrome, and console surface structure form through checked-in definitions. Authored scroll nodes support explicit horizontal, vertical, or two-axis behavior with per-axis input policy, while broader workspace/profile defaults and richer unavailable editor feature representation still need follow-up hardening.
- Prior fallback seams removed:
  - no `first_frame()`-based routing in editor runtime systems
  - no `ViewportId(0)` fallback in shell viewport adapter
- The normal viewport migration path is closed. The runtime still has one
  compatibility-only bootstrap selection seam in
  `apps/runenwerk_editor/src/runtime/viewport/routing.rs::resolve_structural_viewport_products`
  before first structural projection artifacts exist; private helpers
  `select_viewport_id_with_bootstrap_policy` and `bootstrap_single_viewport_id`
  require exactly one observed viewport and are not a path for new
  viewport/product work.
- Substrate output now has baseline snapshot tests and a lightweight gallery harness example (`domain/ui/ui_runtime/examples/substrate_gallery.rs`).

## Current Crate Map

- `domain/ui/ui_composition`
  - app-neutral targets, roots, region graphs, opaque mounted-content
    references, ratified structural state, typed transactions, structural
    undo/redo, liveness/fallback vocabulary, promotion candidates, deterministic
    core/app-extension envelopes, immutable generation activation/recovery,
    stable diagnostics, and headless conformance fixtures. It has no production
    dependency on another UI crate.
- `domain/ui/ui_adaptive_composition`
  - transient adaptive projection, constraints, hit testing, snap/dock
    proposals and previews, drag/resize sessions, semantic-input handling,
    accessibility/inspection metadata, explicit promotion deltas, neutral
    fixtures, and performance probes. It consumes immutable
    `ui_composition` snapshots and emits proposals; it never commits or
    persists structural state.

- `domain/ui/ui_math`
  - UI math/geometry primitives (`UiRect`, `UiSize`, `UiPoint`, `UiVector`, `UiInsets`, axis types).
- `domain/ui/ui_input`
  - input and focus contracts (events, pointer/keyboard/text contract types, stylus-capable pointer packets, focus ids, shortcut contracts).
- `domain/ui/ui_layout`
  - stateless layout algorithms/contracts (`StackLayout`, `SplitLayout`, constraints, alignment, size policy).
- `domain/ui/ui_text`
  - text primitives and atlas-based layout contracts (`TextStyle`, buffer/cursor/selection, `TextLayouter`, `AtlasTextLayouter`).
- `domain/ui/ui_theme`
  - theme token scales and defaults (colors, spacing, radius, typography). The
    default editor substrate theme is a compact black/dark-gray theme with zero
    radius tokens; app/editor-owned authored themes can override those tokens
    through the editor definition activation path.
- `domain/ui/ui_render_data`
  - renderer-facing `UiFrame`/surface/layer/primitive contracts used by engine renderer extraction.
- `domain/ui/ui_tree`
  - retained tree contracts (`WidgetId`, node kinds/payloads, anchored popup nodes with explicit layer order, tree traversal, computed layout records).
- `domain/ui/ui_runtime`
  - retained runtime orchestration (layout engine, anchored popup layout/hit-testing, input routing, popup overlay layer ordering, runtime state, frame output generation). Button hit testing uses the full button bounds, including padding, while text and other content layout still uses content bounds. Scrollbars are overlay primitives: scroll layout no longer reserves a permanent gutter, two-axis scroll containers keep horizontal and vertical offsets independent, and scrollbar primitives are emitted only for active scroll interaction state. In two-axis containers, scrollbar visibility is tracked per axis: vertical wheel activity reveals only the vertical bar, horizontal middle-drag activity reveals only the horizontal bar, and viewport hover does not reveal every scrollbar.
- `domain/ui/ui_widgets`
  - ergonomic widget/control constructors over `ui_tree` node contracts.

## Adjacent Definition Layer

`domain/ui/ui_definition` is the UI domain crate for general authored UI definitions and their formation pipeline.

This layer owns general authored UI definitions and their formation pipeline:

- authored UI node, layout, menu, popover, theme-reference, and action-slot definitions;
- stable authored UI ids that are distinct from runtime `WidgetId`, focus, capture, and shell session ids;
- validation and normalization for generic UI structure, references, and availability descriptors;
- execution-neutral normalized UI templates that do not encode retained `UiNodeKind`, runtime `WidgetId`, ECS entity ids, or concrete command execution;
- first retained-tree formation target for templates, slots, repeaters, embeds, menus, and availability products consumed by `ui_tree`, `ui_widgets`, and `ui_runtime`.

The authored and normalized UI definition model is source/IR. It should remain stable if a future accepted design adds compiled-reactive or ECS-driven UI execution. Those strategies would be additional formation targets from the normalized model, not a reason to rewrite authored templates.

It must not own editor workspace profiles, `ToolSurfaceKind`, panel/tab identity, app provider registries, concrete command execution, or editor-specific command semantics. Those belong in editor definition/shell/app layers.

Applying an editor-owned definition into the live editor is outside
`domain/ui/ui_definition`. The generic crate can normalize and form retained UI
products, but the app/editor activation seam decides whether an applied editor
definition actually changes live shell/runtime resources.

## Interaction Formation Ownership

ADR 0009 adds an accepted interaction formation layer between normalized UI
definitions and retained UI products. That layer is execution-neutral contract
data, not a second runtime:

```text
NormalizedUiTemplate
  -> FormedInteractionModel
  -> FormedRetainedUiProduct
  -> ui_runtime enforcement
```

Ownership is split intentionally:

- `domain/ui/ui_definition` owns generic interaction vocabulary, validation,
  normalization, source maps, and formed interaction outputs;
- `domain/ui/ui_runtime` owns retained enforcement for layout, clipping, hit
  testing, focus, scroll ownership, input ownership, and frame output;
- `domain/editor/editor_definition` owns editor-specific descriptors that refer
  to generic contracts without making editor commands generic UI semantics;
- `domain/editor/editor_shell` owns shell composition and compatibility
  adapters from current shell state into formed contracts;
- `apps/runenwerk_editor` owns viewport arbitration, runtime integration, app
  IO, fixture loading, and concrete command execution.

The migration guardrail for every Interaction V2 slice is:

```text
definition vocabulary
  -> validation rule
  -> FormedInteractionModel record
  -> retained UI formation adapter
  -> ui_runtime enforcement
  -> editor/app guard
```

Shell polish and popup/adornment/drop-preview documents can supply migration
evidence, but they do not move long-term popup, scroll, focus, docking, chrome,
status, or viewport-input ownership out of this UI/editor/app split.

The current retained UI migration slice catalog is:

| Slice | `domain/ui` responsibility | Adjacent owner responsibility |
|---|---|---|
| `IV2-menu-stack` | generic popup/menu scope records, validation, retained formation, layer order, hit testing, outside-dismiss, and focus-return enforcement | `domain/editor/editor_definition` and `editor_shell` provide editor menu descriptors and shell adapters; `apps/runenwerk_editor` executes routed commands |
| `IV2-scroll-ownership` | scroll owner and axis policy, boundary-consumption, scrollbar capture, and input ownership reporting | editor/app layers keep viewport zoom and provider behavior outside UI authority until UI declines ownership |
| `IV2-menu-sizing` | menu sizing/stretch policy, retained menu measurement, clamp, and scroll fallback | editor descriptors provide menu intent without one-off shell/runtime defaults |
| `IV2-chrome-slots` | generic chrome slot vocabulary and retained slot formation/enforcement | editor shell owns tab/workspace chrome semantics, labels, commands, and route mapping |
| `IV2-dock-drop-zones` | generic drop-zone/hit priority vocabulary where reusable; retained runtime enforces formed preview hit policy | editor shell owns workspace docking intent, preview-only state, and workspace mutation on commit |
| `IV2-status-and-viewport-arbitration` | status overflow policy and UI input ownership reporting | app/editor layers own metrics data, viewport resources, and fail-closed viewport input arbitration |

Landed slice status as of 2026-05-15:

- `IV2-menu-stack` now has generic definition/runtime support through
  `domain/ui/ui_definition/src/interaction.rs`,
  `domain/ui/ui_definition/src/validate.rs::validate_menu`,
  `domain/ui/ui_runtime/src/input/hit_test.rs`, and
  `domain/ui/ui_runtime/src/runtime/ui_runtime.rs::dispatch_keyboard_event`.
  Toolbar and tab-stack menu adapters live in
  `domain/editor/editor_shell/src/composition/toolbar_definition.rs::build_defined_toolbar_menu_popup_with_binding`
  and
  `domain/editor/editor_shell/src/composition/build_editor_shell.rs::build_editor_shell_frame_with_docking_visual_state`.
- `IV2-scroll-ownership` now forms retained scroll owners in
  `domain/ui/ui_definition/src/form.rs::form_retained_ui` and reports wheel
  boundary ownership in
  `domain/ui/ui_runtime/src/input/pointer.rs::apply_scroll_wheel_delta`.
- `IV2-menu-sizing` now carries formed menu-sizing records in
  `domain/ui/ui_definition/src/interaction.rs`, rejects item menus without
  sizing policy in `domain/ui/ui_definition/src/validate.rs::validate_menu`,
  and proves retained popup clamp plus fill-width scroll fallback in
  `domain/ui/ui_runtime/src/layout/engine.rs::layout_popup`. Toolbar and
  tab-stack adapters publish the formed sizing records from
  `domain/editor/editor_shell/src/composition/`.
- `IV2-chrome-slots` now carries formed chrome-slot records in
  `domain/ui/ui_definition/src/interaction.rs`. The editor shell publishes tab
  and workspace close, command, label, drag, and active-indicator slots from
  `domain/editor/editor_shell/src/composition/build_editor_shell.rs::build_tab_strip_from_frame`
  and
  `domain/editor/editor_shell/src/composition/toolbar_definition.rs::project_workspace_close_buttons`,
  while `domain/ui/ui_runtime/src/input/hit_test.rs` guards structural child
  hit precedence.
- `IV2-dock-drop-zones` now carries formed dock/drop-zone records in
  `domain/ui/ui_definition/src/interaction.rs`. The editor shell publishes tab
  reorder, split-insertion, and floating-host drop zones from
  `domain/editor/editor_shell/src/composition/build_editor_shell.rs::dock_drop_zone_interaction_model`
  while keeping workspace mutation in editor/app command owners, and
  `domain/ui/ui_runtime/src/input/hit_test.rs` guards preview child hit
  precedence.
- `IV2-status-and-viewport-arbitration` now carries formed viewport
  status-region, metric-priority, overflow, and input-arbitration records in
  `domain/ui/ui_definition/src/interaction.rs`. Viewport status overlays are
  horizontal scroll-owned regions from
  `domain/editor/editor_shell/src/composition/build_viewport_panel.rs::viewport_status_overlay`,
  and
  `domain/editor/editor_shell/src/composition/build_editor_shell.rs::viewport_surface_interaction_model`
  forms viewport options/tools popup contracts plus status fallback
  arbitration.
- App production viewport input remains app-owned. The guard is
  `apps/runenwerk_editor/tests/viewport_architecture_guards.rs::production_input_bridge_allows_viewport_scroll_only_after_ui_declines_ownership`.

Related non-`domain/ui` owners currently in the runtime path:

- `domain/editor/editor_shell`
  - shell composition, workspace host model, command routing, compatibility re-exports for substrate types.
- `apps/runenwerk_editor`
  - app runtime resource wiring, viewport presentation/product runtime resources, tool-surface runtime binding registry.
- `engine/src/plugins/render`
  - UI submission registry, prepared UI payloads, renderer extraction and draw path for UI primitives.

## What `domain/ui/*` Currently Owns

- app-neutral structural composition through `ui_composition`, without
  app/editor/provider/native-window semantics
- deterministic composition documents, linked core/app-extension envelopes,
  compare-and-swap generation activation, and explicit last-good recovery
- transient adaptive composition projection, reflow, hit testing, previews,
  drag/resize sessions, proposal classification, semantic input parity,
  accessibility metadata, and explicit promotion deltas through
  `ui_adaptive_composition`
- reusable primitive contracts for UI math, input, layout, text, theme, and render-data payloads
- stylus-capable pointer vocabulary for pressure, tilt, twist, tangential
  pressure, eraser/tool kind, device id, timestamped raw/coalesced/predicted
  samples, hover/contact state, barrel buttons, calibration, and low-latency
  preview classification
- text atlas and layouter contracts plus concrete atlas layouter implementation
- renderer-facing UI frame/primitive data model consumed by engine render feature code
- retained runtime ownership via:
  - `ui_tree` (retained nodes/tree/layout records)
  - `ui_runtime` (tree orchestration, anchored popup layout/hit-testing, popup overlay layer ordering, input routing, frame generation)
  - `ui_widgets` (control/widget constructors)

## What `domain/ui/*` Does Not Yet Own

- editor runtime chrome or docking integration onto the adaptive mechanism;
  that remains the next governed checkpoint
- Draw runtime interaction integration onto `ui_composition`; its static
  structural projection is complete
- fully converged app-side usage of all reusable controls in editor shell surfaces
- product-facing editor workspace wording and app extension semantics
  (correctly editor/app-owned); legacy structural workspace state is now
  read-only compatibility input until the docking-runtime gate
- app/runtime glue and viewport product orchestration (correctly owned by `runenwerk_editor`)

## Relationship Between `domain/ui`, `editor_shell`, `runenwerk_editor`, and Engine Render Integration

### `domain/ui`
Provides reusable engine-agnostic UI primitives/contracts, app-neutral
structural composition, and renderer-facing frame data contracts.

### `editor_shell`
Currently owns:

- legacy workspace structural identity and graph state as read-only
  compatibility input until the docking-runtime gate
- product-facing workspace wording, editor extension state, and
  host/panel/tab/tool-surface semantics
- shell command routing from UI interactions

It is not the target owner of generic structural composition.

### `runenwerk_editor`
Owns app/runtime glue:

- shell controller and dispatch bridge
- viewport runtime resource orchestration
- tool-surface runtime binding resource and viewport runtime systems

### Engine render integration
Owns:

- UI frame submission registry/resources
- prepared UI contribution payloads
- renderer extraction/render path for `UiFrame` primitives

This layer should continue consuming UI frame contracts, not owning UI semantics.

## Current Boundary Violations

1. Viewport semantic-to-render payload slot mapping is intentionally adapter-based and must remain centralized; avoid introducing parallel semantic taxonomies in runtime or renderer layers.
2. Existing shell surfaces still rely primarily on button/label primitives; newer reusable controls (`TextInput`, `Toggle`, `NumericInput`, `Tabs`) are present but not yet broadly adopted.
3. The compatibility-only bootstrap single-viewport seam exists in
   `apps/runenwerk_editor/src/runtime/viewport/routing.rs::resolve_structural_viewport_products`
   before first structural tool-surface binding generation. New viewport and
   product work must use structural viewport binding.
4. Editor structural workspace state and `ui_surface` still predate the
   accepted composition authority. Editor workspace structure is read-only;
   `ui_surface` remains live only where its supersession map has not completed.
   No new independent mutation path may be added to either boundary.

## Target Ownership Model
Target ownership (partially implemented):

- `domain/ui/*` owns reusable UI substrate runtime layers:
  - retained tree/runtime orchestration
  - reusable control runtime
  - input/focus/invalidation behavior
  - shared testing harness
- `domain/ui/ui_composition` owns generic saved and ratified structural
  composition, typed structural transactions, structural-only history, neutral
  persistence envelopes, and atomic generation activation.
- `domain/ui/ui_adaptive_composition` owns transient projection and interaction
  mechanism over immutable composition snapshots and emits proposals for a
  host policy to accept or reject. It does not own structural commits.
- `domain/ui/ui_definition` owns general authored UI definition and formation contracts, while `domain/ui` runtime crates consume formed products.
- `editor_shell` owns product-facing workspace wording, editor-specific
  extension semantics, and shell command routing; structural changes flow
  through `ui_composition` after cutover.
- `runenwerk_editor` owns app/runtime wiring and viewport/editor-specific runtime integrations.
- engine render layer continues to own rendering integration and consumes UI frame contracts as data.

## Migration Direction
The accepted composition migration is a single-branch clean cutover with
reviewable checkpoint gates, not indefinite dual authority:

1. complete core contracts and invariants (complete);
2. add deterministic persistence envelopes and atomic core/app extension
   linking (complete);
3. project editor and Draw static structure, then make legacy editor workspace
   state read-only (complete);
4. add adaptive headless proposals (implemented in WR-185), then integrate the
   selected Region Compass direction in editor runtime (WR-186);
5. route all structural mutation through `ui_composition` transactions after
   the editor docking runtime gate;
6. delete mapped legacy authorities at cleanup and run final truth closeout.

Reusable retained controls and opaque render-data slot mapping continue as
orthogonal substrate work; they must not reintroduce a second composition
authority.

## Testing and Verification Expectations

- Keep architecture guard tests that enforce structural identity and fail-closed routing behavior.
- Keep baseline unit coverage in primitive crates (`ui_math`, `ui_input`, `ui_layout`, `ui_theme`, `ui_render_data`, `ui_tree`, `ui_widgets`).
- Keep retained-runtime interaction coverage for keyboard/text/focus/invalidation and control interactions.
- Keep UI frame snapshot/fixture verification for stable render-data expectations (`domain/ui/ui_runtime/src/output/build_ui_frame.rs` tests).
- Keep the lightweight substrate gallery harness runnable (`domain/ui/ui_runtime/examples/substrate_gallery.rs`).
- Preserve smoke/architecture tests proving no fallback regression for viewport/tool-surface binding behavior.

## Explicit Non-Goals

- documenting fallback seam removal as complete before code actually removes it
- full docking/tab UX productization in this architecture document
- authored editor-definition/meta-editor system specification here
- turning `domain/ui` or `domain/ui/ui_definition` into editor semantics crates
- speculative future feature taxonomy beyond current audited constraints

## Related Architecture and Workspace Docs

- [Runenwerk UI Framework Architecture](../../architecture/ui-framework-architecture.md)
- [Workspace Architecture Boundaries](../../guidelines/architecture.md)
- [Runenwerk Architecture Doctrine](../../guidelines/runenwerk-architecture.md)
- [Module Structure Guidelines](../../guidelines/module-structure-guidelines.md)
- [UI Definition Formation Framework Design](../../design/implemented/ui-definition-formation-foundation-design.md)
- [ADR 0009: UI Interaction Formation V2](../../adr/accepted/0009-ui-interaction-formation-v2.md)
- [Editor / UI / Workspace / Tool-Surface Architecture](../../design/active/editor-ui-workspace-tool-surface-architecture.md)
- [Viewport Expression Upgrade Design](../../design/implemented/workspace-viewport-expression-upgrade-design.md)
- [Workspace Identity Contract and Migration Map](../../design/implemented/workspace-identity-contract-and-migration-map.md)
- [UI Substrate Roadmap](./roadmap.md)

<!-- BEGIN RUNENWERK:UI_COMPONENT_PLATFORM:domain-ui-note -->
## UI Component Platform activation note

The active Component Platform roadmap is `PT-UI-COMPONENT-PLATFORM`: reusable, story-proven `ControlPackage` and surface maturity after `PM-UI-STORY-004`. The platform introduces reusable kernels for control packages, authoring, story proof, catalog/discovery, input/gesture/device, state/binding/host intent, theme/token styling, accessibility/focus, layout/container/virtualization, render/surface output, overlay/popup/layering, text, Surface2D, SpatialCanvas, NodeCanvas, PortGraphCanvas, ProgressionTreeView, TrackSurface/Timeline, transitions/effects, and adoption gates.
<!-- END RUNENWERK:UI_COMPONENT_PLATFORM:domain-ui-note -->
