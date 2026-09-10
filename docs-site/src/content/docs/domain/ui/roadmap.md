---
title: UI Substrate and Surface Roadmap
description: Current implementation roadmap for Runenwerk UI substrate, UiProgram runtime artifact flow, and surface semantics.
status: active
owner: ui
layer: domain
canonical: true
last_reviewed: 2026-06-16
related:
  - ./architecture.md
  - ./story-acceptance-and-review-checklist.md
  - ../../design/active/runenwerk-ui-story-driven-golden-workflow-design.md
  - ../../design/active/runenwerk-ui-platform-capability-roadmap.md
  - ../../design/active/ui-runtime-rendering-pipeline-roadmap.md
  - ../../design/active/ui-program-architecture.md
  - ../../design/implemented/ui-definition-formation-foundation-design.md
  - ../../reports/audits/editor-ui-priority-code-audit-2026-05-05.md
  - ../../design/implemented/editor-self-authoring-and-final-ui-design.md
  - ../../design/implemented/editor-ui-runtime-v2-and-interaction-formation-design.md
  - ../../design/active/editor-ui-popup-adornment-drop-preview-contract.md
  - ../../design/active/editor-shell-menu-and-tab-chrome-polish-design.md
  - ../../design/active/editor-ui-workspace-tool-surface-architecture.md
  - ../../design/deferred/ui-model-multiple-execution-strategies-design.md
  - ../../adr/accepted/0009-ui-interaction-formation-v2.md
  - ../../design/implemented/workspace-identity-contract-and-migration-map.md
---

# Runenwerk UI Substrate and Surface Roadmap

## Purpose

Track implementation sequencing for UI substrate, retained UI surfaces, UiProgram architecture, runtime artifacts, and the future artifact-backed rendering path from current code truth.

This roadmap is intentionally execution-oriented. Target architecture belongs in active design docs. The artifact-backed rendering roadmap is now tracked in [UI Runtime Rendering Pipeline Roadmap](../../design/active/ui-runtime-rendering-pipeline-roadmap.md).

## Current Code Truth

Implemented and in use:

- substrate crates are present and active: `ui_math`, `ui_input`, `ui_layout`, `ui_text`, `ui_theme`, `ui_render_data`, `ui_tree`, `ui_runtime`, `ui_widgets`;
- `ui_surface` exists with definition/mount/observation/session/presentation/intent/ratification contracts;
- shell/runtime integration routes core editor flows through prepared `SurfacePresentationModel`, typed `SurfaceIntent`, and host-side ratification adapters;
- retained UI node and runtime interaction support exists for reusable controls including text input, numeric input, toggle, tabs, select, table, tree, scroll, split, and viewport embed;
- runtime viewport routing is structural-first. The normal viewport migration path is closed; one compatibility-only bootstrap seam remains in `apps/runenwerk_editor/src/runtime/viewport/routing.rs::resolve_structural_viewport_products`, through private helpers `select_viewport_id_with_bootstrap_policy` and `bootstrap_single_viewport_id`, before first projection artifacts exist. New viewport/product work must use structural viewport binding, not bootstrap selection;
- architecture guard tests enforce no `first_frame()` routing fallback and no `ViewportId(0)` synthesis;
- viewport semantic slot taxonomy remains in `editor_viewport`, with opaque renderer-facing payload slots in `ui_render_data`;
- `ui_definition` exists with authored templates, validation, normalization, retained formation, route/embed products, and checked-in editor UI fixtures; broader workspace profile catalogs/default layouts, unavailable feature representation, and custom workspace catalog behavior still need follow-up hardening;
- ADR 0009 makes Interaction V2 the accepted architecture for popup stack, scroll ownership, focus, menu sizing, chrome slot, docking-zone, and status overflow contracts. Current runtime execution is still retained UI;
- UiProgram architecture crates now exist for semantic program proof slices: `ui_schema`, `ui_program`, `ui_controls`, `ui_program_lowering`, `ui_compiler`, `ui_artifacts`, `ui_accessibility`, and `ui_binding`;
- `ui_program_lowering` exposes the registry-snapshot formation path `form_ui_program_report_from_node_with_registry_snapshot(...)`, and `UiProgramFormationReport` now carries combined diagnostics plus a mandatory catalog report;
- runtime artifacts already split controls, layout, style, state, interaction, binding snapshots, collection diffs, visual, text layout requests, accessibility, and inspection tables;
- `ui_program_lowering/src/lower.rs::lower_control_nodes` now fails closed for unknown authored control kinds and emits no package-contract-derived graph rows;
- `ui_runtime_view` exposes artifact-backed runtime views, including a button-specific report carrying label, route, capability, selected/disabled state, accessibility label, style axes, source-map indexes, and diagnostics;
- `ui_render_primitives` owns backend-neutral button primitive generation from runtime views, resolves theme tokens and text layout through `ThemeTokens` and `AtlasTextLayouter`, and emits `GlyphRunPrimitive` label text before a `UiFrame` reaches renderer-facing code;
- `ui_story` owns the domain-level story manifest, checked-in gallery catalog, registry, runner, CLI summary, report, and mount-eligibility contracts; product host mounting remains downstream story-platform work;
- `runenwerk_editor` now has a standalone `runenwerk_ui_gallery` host and CLI inspection path that consume checked-in `UiStoryRunReport` output with render primitive, render-data, static-mount, preview-frame, and mount-eligibility stages before submitting a prepared `UiFrame` through the existing editor UI composite pass.

Evidence in code:

- `domain/ui/ui_surface/src/*`
- `domain/ui/ui_runtime/src/*`
- `domain/ui/ui_program_lowering/src/*`
- `domain/ui/ui_program/src/*`
- `domain/ui/ui_controls/src/*`
- `domain/ui/ui_compiler/src/*`
- `domain/ui/ui_artifacts/src/*`
- `apps/runenwerk_editor/src/shell/dispatch_shell_command.rs`
- `apps/runenwerk_editor/src/runtime/viewport/routing.rs`
- `apps/runenwerk_editor/tests/viewport_architecture_guards.rs`

## Phases and Status

### Phase 1 - Substrate Ownership Hardening

Status: complete for current scope.

Notes:

- retained tree/runtime/widget ownership is in `domain/ui/*`;
- ad hoc viewport routing fallbacks are removed and guarded.

### Phase 2 - Establish `ui_surface` as Semantic Kernel

Status: complete for baseline scope.

Notes:

- `ui_surface` crate is present and used by production editor flows;
- mounted-surface and capability/session contracts are active.

### Phase 3 - Formal Observation/Session/Presentation/Intent/Ratification Boundaries

Status: partially complete and active.

Completed:

- core outliner/inspector/viewport command paths route through surface presentation and intent contracts.

Remaining:

- extend coverage across additional surface families and non-core interactions;
- keep contract usage consistent in new command paths.

### Phase 4 - Viewport/Embed/Render-Data Seam Consolidation

Status: complete for normal viewport migration, guard-hardened, and active only for compatibility preservation.

Completed:

- semantic slot ownership is in `editor_viewport`;
- renderer-facing payload ownership is in `ui_render_data`;
- structural binding adapters are active in `runenwerk_editor` runtime seams;
- `apps/runenwerk_editor/src/runtime/systems/frame_submit.rs::populate_viewport_layout_map_from_shell_tree` scans structural `ViewportSurfaceEmbed` nodes;
- `apps/runenwerk_editor/src/runtime/systems/viewport_lifecycle.rs::sync_viewport_instances_system` syncs explicit viewport instances from workspace state.

Remaining:

- preserve the one compatibility-only bootstrap seam in `apps/runenwerk_editor/src/runtime/viewport/routing.rs::resolve_structural_viewport_products`;
- do not use bootstrap selection for new viewport/product work.

### Phase 5 - Control Semantics Hardening

Status: active.

Focus:

- broaden use of reusable controls across editor surfaces where ad hoc composition remains;
- keep interaction flows surface-centered and capability-aware.

### Phase 6 - Verification and Docs Hardening

Status: active.

Focus:

- keep guard suites authoritative as behavior evolves;
- keep architecture and roadmap pages synchronized with implemented seams.

### Phase 7 - UI Definition Formation Framework

Status: M3.5 implemented and validated as of 2026-05-06.

Owning design:

- `docs-site/src/content/docs/design/implemented/ui-definition-formation-foundation-design.md`

Decision:

- keep the current retained UI tree plus tool-surface/canvas hybrid as the accepted execution path;
- add a compiler-inspired definition/formation layer above the substrate instead of moving editor semantics into `domain/ui`;
- treat authored and normalized UI definitions as execution-neutral source/IR;
- keep compiled-reactive and ECS-driven UI execution deferred unless a future active design or accepted ADR promotes one. If promoted, they should become additional formation targets from the normalized model, not replacements for authored template identity.

Owning crates:

- `domain/ui/ui_definition`
- `domain/editor/editor_definition`

Scope:

- authored UI templates, layout nodes, controls, menu/popover intent, theme references, route slots, value slots, collection slots, repeaters, template refs, embed slots, and availability references;
- stable authored UI ids distinct from `WidgetId`, focus/capture ids, `PanelInstanceId`, and `ToolSurfaceInstanceId`;
- validation, normalization, diagnostics, source/path maps, and a first retained-tree formation target;
- generic availability descriptors so disabled/unavailable UI can render without routing fake commands;
- editor-specific bindings for toolbar, menus, workspace catalogs, shell chrome, and common provider surface templates in `domain/editor/editor_definition`.

Implemented first-slice evidence:

- `domain/ui/ui_definition` is an active workspace crate with the source model modules, validation, normalization, and retained formation.
- `domain/editor/editor_definition` is an active workspace crate with toolbar, menu, workspace, surface, command route, availability, binding, validation, and editor UI formation helpers.
- Checked-in fixtures under `assets/editor/ui/` parse and validate in tests.
- `editor_shell` forms the toolbar fixture, toolbar popup item bindings, and maps formed route slots to existing shell actions; route slots remain inert until shell mapping.
- `editor_shell` forms normal tab-stack shell chrome from `assets/editor/ui/shell_chrome.ron` while dynamic docking previews remain live shell behavior.
- `editor_shell` forms common provider surface structure from `assets/editor/ui/surfaces/*.ron` for console, inspector, outliner, entity-table, and viewport. Provider data, route proposals, follow-scroll state, viewport overlays, and render/embed payload ownership remain outside templates.
- `apps/runenwerk_editor` owns checked-in fixture loading and validation policy.

Remaining follow-up after M3.5 closeout:

- continue broadening reusable controls and richer surface workflows without moving provider semantics into UI definitions.

Non-goals:

- moving editor workspace profiles, panel/tab/tool-surface semantics, shell routing policy, provider execution, or app IO into `domain/ui/ui_definition`;
- app provider registries, file IO, or runtime instantiation;
- visual editor self-authoring implementation and user-authored document lifecycle inside M3.5; those moved to the promoted M3.6 UI self-authoring workspace after this framework became active;
- baking retained `UiNodeKind`, runtime `WidgetId`, concrete shell commands, or ECS component/entity layout into authored UI templates;
- ECS entities as authored UI/editor identities.

Milestone placement:

- the app roadmap inserts this as M3.5 before the promoted M3.6 UI self-authoring workspace and before the integrated M4 UI/editor/asset foundation so new asset/procedural/editor-design workspace work does not add more hard-coded toolbar, menu, workspace, shell chrome, and provider surface structure.

### Phase 8 - UI Runtime V2 Interaction Formation

Status: implemented for the retained UI target under accepted ADR 0009 as of 2026-05-15. The named Interaction V2 slices are retained implementation contracts; compiled-reactive and ECS-driven execution targets remain deferred.

Owning design:

- `docs-site/src/content/docs/design/implemented/editor-ui-runtime-v2-and-interaction-formation-design.md`
- `docs-site/src/content/docs/adr/accepted/0009-ui-interaction-formation-v2.md`

Decision:

- keep retained UI as the first execution target;
- insert `FormedInteractionModel` between normalized UI definitions and retained UI products;
- solve popup stack, scroll ownership, focus scope, menu sizing, dock/drop-zone, chrome slot, and status overflow behavior as shared contracts instead of surface-local patches;
- keep compiled-reactive and ECS-driven UI deferred until a separate accepted ADR or active design promotes a concrete target;
- keep renderer output as derived product data, not UI authority.

Required Interaction V2 validation coverage:

- popup/menu layer order;
- scroll clipping;
- wheel ownership and boundary propagation;
- outside-dismiss behavior;
- focus return;
- split-border precedence over tab and chrome hits;
- dock-drag/drop-zone precedence;
- radial/menu hit testing where applicable;
- viewport input receives wheel/pointer input only after UI explicitly declines ownership;
- screenshot or primitive-order harness coverage for visual/frame-order regressions where normal unit tests are insufficient.

Implemented retained-slice migration spine:

```text
definition vocabulary
  -> validation rule
  -> FormedInteractionModel record
  -> retained UI formation adapter
  -> ui_runtime enforcement
  -> editor/app guard
```

Retained UI slice catalog:

| Slice | Readiness gate | Downstream consumer |
|---|---|---|
| `IV2-menu-stack` | popup stack/menu scope vocabulary, parent/child anchor validation, retained menu formation, runtime layer/dismiss/focus enforcement, and shell/app guards | toolbar, viewport, tab action, and Switch Type menu polish |
| `IV2-scroll-ownership` | scroll owner/axis/boundary policy, retained scroll formation, runtime input ownership reporting, and viewport fallback guards | scrollable popups, status bars, and panel overflow polish |
| `IV2-menu-sizing` | intrinsic menu sizing, item fill, clamp, and scroll fallback contracts with runtime layout tests | popup width/item stretch fixes |
| `IV2-chrome-slots` | close, active indicator, label, command, and drag slots with shell projection and primitive-order guards | tab/workspace chrome polish |
| `IV2-dock-drop-zones` | tab reorder, split insertion, floating-host, invalid-target, and preview-only drop-zone policy with shell/controller guards | docking preview and tab-strip precedence polish |
| `IV2-status-and-viewport-arbitration` | status overflow and viewport pointer/wheel arbitration with shell projection and app guard tests | FPS/frame-time status and compact viewport metadata polish |

Phase 8 leads WR-024. Shell polish may proceed only as retained UI implementation after the relevant contract slice is defined, or as bounded compatibility evidence that names the old path, target formed contract, and regression guard. It must not define durable popup, scroll, focus, docking, chrome, status, or viewport-input policy in app or shell code.

### Phase 9 - Artifact-Backed Runtime Rendering Pipeline

Status: deferred as a standalone implementation slice. WR-173 /
PM-UI-RUNTIME-RENDERING-001 is retained only as temporary static-gallery
evidence; runtime rendering proof must return through
PM-UI-STORY-004 after the story manifest, registry, runner, report, and
mount-eligibility contracts exist.

Owning design:

- `docs-site/src/content/docs/design/active/ui-runtime-rendering-pipeline-roadmap.md`

Decision:

- a visible button is not allowed to be a hand-written widget proof;
- rendering may begin only after artifact-backed runtime view, layout, style, text, accessibility, and render primitive reports exist;
- renderer/backend code consumes backend-neutral derived primitives only and must not own UI semantics.

Milestone order:

| ID | Milestone | Gate |
|---|---|---|
| M1 | Unknown control kind fail-closed | `ui.program.control.unknown_kind` fails formation and emits no contract-derived graph rows. |
| M2 | Catalog schema/kernel completeness | broken package metadata fails catalog derivation and propagates into formation report. |
| M3 | Authored property validation/value carry-through | valid button properties survive authored source -> UiProgram -> runtime artifact. |
| M4 | Source-map and diagnostic completeness | every graph-family row and diagnostic is traceable. |
| M5 | Runtime artifact snapshots | basic/selected buttons produce stable artifact snapshots. |
| M6 | Artifact-backed runtime view model | `ButtonRuntimeView` derives from artifact + host data. |
| M7 | Binding evaluator integration | selected/disabled state derives from host data and authorization checks. |
| M8 | Runtime layout resolution | button receives stable layout box and layout diagnostics. |
| M9 | Style/theme token resolution | variant/tone/density/size/state resolve through tokens, not renderer colors. |
| M10 | Text layout request/result path | label text measurement is explicit and source-mapped. |
| M11 | Backend-neutral render primitives | button resolves to deterministic primitive snapshots. |
| M12 | Headless render frame snapshots | button output is proven before visible backend integration. |
| M13 | First visible gallery button | visible output consumes the full proven pipeline. |
| M14 | Interaction hit testing and route proposal | click/keyboard emit schema-valid route proposals. |
| M15 | Button visual state loop | hover/pressed/focused/selected/disabled are proven. |
| M16 | Accessibility runtime proof | rendered/runtime button is inspectable and accessible. |
| M17-M20 | Additional control pipelines | label, inspector field, color picker, prompt/list/tree/table receive the same proof chain. |
| M21 | UI Gallery inspector | authored source through rendered preview and interaction trace are inspectable. |
| M22 | Production readiness evidence integration | production-ready claims require complete, fresh, owner-correct evidence. |

Render permission gates:

```text
After M12: headless render output may be trusted.
After M13: visible static button may be shown.
After M15: interactive button may be claimed.
After M16: accessible button may be claimed.
After M22: production-ready UI rendering may be claimed.
```

Stop conditions:

- rendering from authored `.ron` directly;
- rendering from `ControlPackageDescriptor` alone;
- inferring package truth from a control-kind string;
- letting unknown control kinds or broken package metadata pass formation;
- hardcoding visual semantics in backend renderer code;
- mutating editor/game/domain/app truth from generic UI code;
- creating new crates without WR/production authority.

### Phase 10 - Story-Driven Golden Workflow Productization

Status: active design intake; implementation not authorized by this roadmap
section alone.

Owning designs:

- `docs-site/src/content/docs/design/active/runenwerk-ui-story-driven-golden-workflow-design.md`
- `docs-site/src/content/docs/design/active/runenwerk-ui-platform-capability-roadmap.md`

Decision:

- `UiStory` is the canonical unit for UI authoring, preview, validation,
  inspection, proof, and mount eligibility.
- A story may wrap either a bare `UiNodeDefinition` or a full
  `AuthoredUiTemplate`.
- Story manifests live in assets, not hardcoded Rust constants.
- The current hardcoded button gallery path is valid first-slice evidence, but
  it is not the final gallery architecture.
- Gallery and CLI must share the same domain-owned story runner.
- Gallery inspection must consume `UiStoryRunReport`, not button-specific
  reports.
- Normal surfaces using existing controls should be cheap to author, while new
  reusable component classes are component-platform work after story proof
  exists.
- `PT-UI-STORY-PLATFORM` owns only the story proof substrate:
  `UiStoryManifest`, `UiStoryRegistry`, `UiStoryRunner`,
  `UiStoryRunReport`, `UiStoryMountEligibility`, gallery/CLI story execution,
  and story-gated static/runtime rendering proof.
- GraphCanvas, Timeline, RichText/CodeEditor, advanced drag/drop, and generic
  component-level UI effects move to `PT-UI-COMPONENT-PLATFORM`.
- Visual UI Builder and authoring product workflows remain in
  Designer/Workbench tracks.
- Screen-space game HUD behavior remains in `PT-GAME-RUNTIME-UI`.
- World-space/entity-attached UI remains deferred in
  `PT-GAME-WORLDSPACE-UI`.
- Camera/projection/surface-fit contracts remain in
  `PT-VIEWPORT-PROJECTION`.

First production milestone to prepare through normal WR/production gates:

```text
PM-UI-STORY-001 - Story Workflow Authority And Track Activation
```

Outcome:

- activate `PT-UI-STORY-PLATFORM` as the story-first UI production track;
- defer standalone static gallery rendering as a production growth path;
- record the future `UiStoryManifest`, `UiStoryRegistry`, `UiStoryRunner`,
  `UiStoryRunReport`, and `UiStoryMountEligibility` contracts;
- keep runtime code, crate creation, gallery migration, and product mounting
  blocked until their owning WR and production plan exist;
- keep component maturity, visual authoring product work, screen HUD behavior,
  world-space/entity-attached UI, and viewport projection contracts in their
  owning tracks.

Later implementation contract target files and functions:

```text
domain/ui/ui_story/src/gallery.rs::checked_in_gallery_registry
domain/ui/ui_story/src/cli.rs::UiStoryCliReport::from_reports
apps/runenwerk_editor/src/runtime/ui_gallery.rs::run_checked_in_gallery_stories
apps/runenwerk_editor/src/runtime/ui_gallery.rs::inspect_checked_in_gallery_stories
apps/runenwerk_editor/src/runtime/ui_gallery.rs::submit_ui_gallery_frame_system
domain/ui/ui_story/src/runner.rs::UiStoryRunner::run_story
domain/ui/ui_story/src/runner.rs::UiStoryRunner::run_story_with_stage_reports
domain/ui/ui_story/src/report.rs::UiStoryRunReport
domain/ui/ui_story/src/mount.rs::UiStoryMountEligibility::from_report
```

Stop conditions:

- do not implement `domain/ui/ui_story` before crate creation authority exists;
- do not keep `apps/runenwerk_editor/src/runtime/ui_gallery.rs::UI_GALLERY_FIXTURES`
  as a production story registry after cutover;
- do not add renderer-owned UI semantics;
- do not let generic UI mutate editor, game, domain, or app state directly;
- do not mount a surface without story-derived mount eligibility.

## Current Now Tasks

- [x] Finish docking/tab behavior on top of existing structural identity and binding contracts. Status: implemented and test-covered; automated coverage exists for tab reorder, rehome, floating host creation, split resizing, area split/duplicate/reset/close, dynamic split-area composition, and structural identity preservation.
- [x] Expose editor-area/type switching with a retained select/dropdown route. Status: implemented and test-covered; tab chrome renders an editor type selector and maps `SelectChanged` to `SwitchPanelToolSurfaceKind`.
- [x] Add plus/new-tab affordance for tab stacks. Status: implemented and test-covered; tab chrome exposes a plus/new-tab control that allocates panel and tool-surface identities after structural ratification.
- [x] Expand non-viewport surface maturity (entity-table/query, richer inspector controls) using existing surface contracts. Status: implemented as of 2026-05-08; provider-backed surfaces now use typed surface action/session/domain wrappers, entity-table query workflows cover search/selected-only/hierarchy/component filters plus sorting, and inspector controls render bool/numeric/text/enum/read-only alternatives without moving provider behavior into `ui_definition`.
- [x] Complete active UI/editor definition consumption before adding more hard-coded editor surfaces. Status: implemented as of 2026-05-09 for M4A-M4D. Active templates/editor bindings feed the shell frame, command bindings resolve authored route ids to app-owned known commands, shortcuts and menus dispatch through that command spine, active toolbar/menu definitions can replace checked-in fixture menu items, and panel/tool-surface registries drive future shell creation/switch choices without mutating existing workspace state.
- [x] Broaden reusable control adoption in editor surfaces. Status: implemented as of 2026-05-09 for the M4E cleanup. `editor_shell` now owns shared retained surface fixture helpers, shared compact reusable-control polish, and self-authoring control-panel composition; app providers supply DTOs/actions/routes instead of direct reusable-control construction.
- [x] Preserve and extend guard coverage for structural routing, capability gating, and seam ownership. Status: updated as of 2026-05-09; guard coverage rejects direct app-provider reusable-control construction while preserving existing `ui_definition` behavior-isolation and surface-routing checks.
- [ ] Keep future UI execution strategies design-gated. Status: active/open; compiled-reactive UI and ECS-driven UI remain future formation targets only after a new active design or accepted ADR identifies the concrete surface, formation product, invalidation/debug model, and command/ratification boundaries. Any future target must consume normalized definitions plus formed interaction contracts.
- [x] Migrate retained UI slices to Interaction V2 contracts. Status: complete for the named WR-025 retained-slice catalog with code-bearing slices and doctrine-repair evidence landed for `IV2-menu-stack`, `IV2-scroll-ownership`, `IV2-menu-sizing`, `IV2-chrome-slots`, `IV2-dock-drop-zones`, and `IV2-status-and-viewport-arbitration`.
- [x] Complete the M3.5 UI definition formation framework before M3.6 and M4. Status: implemented and validated; crates, fixtures, retained formation, app fixture validation, toolbar route-slot integration, toolbar popup binding data, normal shell chrome formation, and common provider surface fixture formation exist. Provider behavior remains outside `ui_definition`.
- [x] Implement the promoted UI self-authoring workspace before M4. Status: complete as of 2026-05-06; `editor_definition` owns durable editor schemas and validation guards, `editor_shell` exposes the Editor Design workspace/profile and self-authoring surface kinds, and `runenwerk_editor` loads checked-in UI fixtures as editable documents with validation, retained preview, command diff summaries, retained authoring control routes, UI node/theme/workspace-layout draft edits, and explicit apply/rollback.
- [x] Keep UI Designer visible as the promoted self-authoring path. Status: complete/current; UI Designer is not a missing roadmap item, it is the Editor Design/self-authoring workspace tracked by `docs-site/src/content/docs/design/implemented/editor-self-authoring-and-final-ui-design.md`.
- [ ] Continue popup/menu and tab chrome polish only under Interaction V2. Status: ready-next/supporting evidence; `docs-site/src/content/docs/design/active/editor-shell-menu-and-tab-chrome-polish-design.md` tracks immediate retained-UI symptoms, but WR-024 follows WR-025 and may only consume the named `IV2-*` contract slices or provide compatibility evidence. It may not own long-term popup, scroll, focus, menu sizing, chrome, docking-zone, status-overflow, or viewport-input contracts.
- [x] Continue artifact-backed runtime rendering only through story gates. Status: WR-177 / PM-UI-STORY-004 re-runs the former static gallery proof through `UiStoryRunReport` render primitive, render-data, static-mount, preview-frame, and mount-eligibility stages. Product host mounting remains future story-platform work.
- [ ] Prepare the story-driven golden workflow through normal WR/production gates. Status: active design intake; `PM-UI-STORY-001` activates planning authority only, while later PM-UI-STORY milestones introduce the story runner, gallery/CLI execution, runtime rendering proof, and mount eligibility.
- [ ] Keep cross-doc sequencing aligned so workspace index docs do not restate stale phase history. Status: active; generated production docs must remain outputs of their source files and must not be hand-edited.

## Non-Goals for This Track

- redesigning renderer architecture;
- introducing the visual editor or user-authored document lifecycle inside M3.5;
- collapsing surface semantics into shell or runtime substrate layers;
- moving privileged ratification ownership into generic UI substrate code;
- using ECS entities, runtime widget ids, or shell session ids as durable authored UI/editor identity;
- rendering from authored `.ron` files or package descriptors directly;
- creating additional runtime/rendering crates or backend adapters without explicit WR/production authority.

<!-- BEGIN RUNENWERK:UI_COMPONENT_PLATFORM:domain-ui-note -->
## UI Component Platform activation note

The active Component Platform roadmap is `PT-UI-COMPONENT-PLATFORM`: reusable, story-proven `ControlPackage` and surface maturity after `PM-UI-STORY-004`. The platform introduces reusable kernels for control packages, authoring, story proof, catalog/discovery, input/gesture/device, state/binding/host intent, theme/token styling, accessibility/focus, layout/container/virtualization, render/surface output, overlay/popup/layering, text, Surface2D, SpatialCanvas, NodeCanvas, PortGraphCanvas, ProgressionTreeView, TrackSurface/Timeline, transitions/effects, and adoption gates.
<!-- END RUNENWERK:UI_COMPONENT_PLATFORM:domain-ui-note -->
