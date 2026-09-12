---
title: Runenwerk Editor Final Implementation Roadmap
description: Repo-truth implementation roadmap from current editor MVP state to feature-complete UI, editor, asset pipeline, runtime preview, and self-authoring workflows.
status: active
owner: editor
layer: app
canonical: true
last_reviewed: 2026-09-12
related_designs:
  - ../../design/accepted/sdf-first-field-world-platform-design.md
  - ../../design/implemented/ui-definition-formation-foundation-design.md
  - ../../design/implemented/editor-workspace-document-mode-panel-architecture.md
  - ../../design/active/editor-ui-workspace-tool-surface-architecture.md
  - ../../design/implemented/editor-tool-suite-registry-and-workbench-host-design.md
  - ../../design/accepted/editor-native-multi-window-presentation-design.md
  - ../../design/implemented/workspace-viewport-expression-upgrade-design.md
  - ../../design/implemented/render-product-surface-foundation-bundle-design.md
  - ../../design/implemented/viewport-dynamic-product-target-allocation-design.md
  - ../../design/implemented/editor-self-authoring-and-final-ui-design.md
  - ../../design/implemented/editor-ui-runtime-v2-and-interaction-formation-design.md
  - ../../design/active/editor-asset-pipeline-and-content-workflow-design.md
  - ../../design/active/editor-procedural-content-and-simulation-workflow-plan.md
  - ../../design/active/gameplay-graph-atr-ir-and-ecs-lowering-design.md
  - ../../design/active/engine-game-runtime-editor-ecs-scripting-hot-reload-design.md
related_adrs:
  - ../../adr/accepted/0009-ui-interaction-formation-v2.md
  - ../../adr/accepted/0007-external-runtime-preview-process.md
related_roadmaps:
  - ../../workspace/planning/roadmap.md
  - ../../domain/ui/roadmap.md
  - ../../engine/plugins/render/docs/roadmap.md
related_reports:
  - ../../reports/audits/editor-ui-priority-code-audit-2026-05-05.md
  - ../../reports/closeouts/m5-runtime-preview/closeout.md
related:
  - ./execution-priority-checklist.md
  - ./current-architecture.md
  - ./viewport-expression-implementation-roadmap.md
  - ./mvp/first-3d-editor-mvp.md
  - ./mvp/acceptance-criteria.md
  - ./mvp/implementation-sequence.md
  - ../../domain/world-sdf/README.md
  - ../../domain/world-ops/README.md
  - ../../domain/spatial/README.md
  - ../../domain/chunking/README.md
---

# Runenwerk Editor Final Implementation Roadmap

## Goal

Make the Runenwerk editor feature-complete end to end:

- project creation and project loading;
- document tabs for scenes, prefabs, SDF and field-world documents, UI, graphs, scripts, materials, textures, particles, physics, animation, procedural generation, gameplay graphs, gameplay rules/triggers/abilities/quests, imports, runtime/debug documents, and editor definitions;
- task workspaces with scoped modes and mode-aware tools;
- reusable dock/tab/area UI with editor type switching and new-tab creation;
- complete SDF-first 3D scene authoring with transform tools, common entity actions, SDF primitive and brush workflows, inspector/component editing, undo/redo, persistence, validation, and viewport feedback;
- an SDF/field-world-first asset pipeline that imports, validates, forms field products, caches, previews, hot-reloads, and publishes authored content;
- authoring workspaces for UI, graphs/materials/textures, SDF modeling, procedural generation, gameplay graph ATR IR, particles, physics, animation, scripting, debugging, and editor design;
- play/simulate/runtime preview with explicit data hot reload and code rebuild boundaries;
- current self-authoring workflows for workspace layouts, UI layouts, menus, shortcuts, themes, command bindings, and tool-surface definitions before later asset/procedural/gameplay editors are built on top of more hard-coded UI.

This is not a deferral list. Every phase below is required for feature completion. Later phases are ordered after their prerequisites, not optional.

## Repo Truth Audit

Current implemented baseline:

- Editor MVP acceptance is closed in `docs-site/src/content/docs/apps/runenwerk-editor/execution-priority-checklist.md`.
- Workspace profile identity exists in `domain/editor/editor_shell/src/workspace/profile.rs::WorkspaceProfile`.
- Profile-addressed workspace layout persistence exists in `apps/runenwerk_editor/src/persistence/workspace_layout.rs::default_workspace_layout_path_for_profile`.
- Structural workspace identities exist in `domain/editor/editor_shell/src/workspace/state.rs::WorkspaceState`.
- Tab reorder, tab rehome, floating host placeholders, and split resizing exist in `domain/editor/editor_shell/src/workspace/reducer.rs::WorkspaceMutation`.
- Provider DTOs exist in `domain/editor/editor_shell/src/surface_provider.rs`.
- Concrete app providers exist in `apps/runenwerk_editor/src/shell/providers/mod.rs::EditorSurfaceProviderRegistry`.
- Retained UI substrate and widgets exist in `domain/ui/*`, including select, tree, table, tabs, toggle, numeric input, text input, scroll, split, and viewport embed.
- Scene file migration, normalization, and formation exist in `domain/editor/editor_persistence/src/scene_migration.rs`, `domain/editor/editor_persistence/src/scene_normalization.rs`, and `domain/editor/editor_persistence/src/scene_formation.rs`.
- Reusable SDF primitives, composition, transforms, sampling, gradients/normals, and CPU queries are owned by standalone `dornglut/runen-sdf`; Runenwerk consumes product/world contracts through `domain/world_sdf` and editor-owned authoring projections.
- World edit operation logs, dirty-region tracking, build queues, and invalidation contracts exist in `domain/world_ops/src/operation_log.rs`, `domain/world_ops/src/dirty.rs`, `domain/world_ops/src/build_queue.rs`, and `domain/world_ops/src/region_invalidation.rs`.
- World-scale SDF chunk/page payload records and collision query contracts exist in `domain/world_sdf/src/storage.rs::SdfChunkStore` and `domain/world_sdf/src/collision.rs::CollisionQueryService`.
- Spatial coordinates, chunk coordinates, clipmap windows, and chunk residency planning exist in `domain/spatial/src/` and `domain/chunking/src/`.
- Scene manifests are discovered from loose RON files by `engine/src/plugins/scene/manifest/catalog.rs::load_scene_manifest_descriptors`.
- Render import contracts in `engine/src/plugins/render/resource/import.rs` describe render resources only; they are not a general asset pipeline.
- Render prepared-frame material plumbing exists in `engine/src/plugins/render/frame/contributions.rs::PreparedMaterialFeatureContribution`, and the material feature slot exists as `engine/src/plugins/render/features/mod.rs::MATERIAL_RENDER_FEATURE_ID`.
- Shader reload helpers exist in `engine/src/plugins/render/shader/hot_reload.rs::poll_shader_hot_reload` and `engine/src/plugins/shared/reload.rs`.
- `assets/editor/config.ron` contains a Blender path, and `assets/models/*.blend` plus `assets/models/*.glb` exist, but those files are foreign-source/reference content today. They are not the canonical world representation and no importer/catalog/cache pipeline currently owns them.

Current post-M3 gaps:

- M1 structural seams are closed: `DocumentKind` has the explicit M1 taxonomy, `EditorSession` owns ordered document tabs, active switching, dirty/save/close transitions, document compatibility validation, and mode ids/descriptors/registry compatibility rules; app-local generic document-tab runtime state is split from scene-specific document state.
- M2 shell seams are closed: tab chrome, editor type switching, new-tab allocation, close/split/duplicate/reset area commands, dynamic split composition, projected-host split resizing, and workspace layout persistence have automated coverage.
- M3 scene-authoring seams are closed: scene command intents cover child creation, subtree duplication, batch delete, SDF primitive creation, transform set/reset, and component add/remove; rotate/scale viewport tools, transform preview, retained outliner tree rows, common reflected inspector editing, SDF authoring DTOs, and normalized save/load paths have focused coverage.
- The M3.5 UI definition/formation closeout is implemented: `domain/ui/ui_definition`, `domain/editor/editor_definition`, checked-in RON fixtures under `assets/editor/ui/`, retained formation, inert route/embed products, toolbar/menu fixture formation, normal shell chrome formation, common provider surface fixture formation, and app-owned fixture validation exist. Provider data, viewport overlays, editor mutations, and route execution remain outside `ui_definition`.
- The M4 asset foundation exists: `domain/asset` owns asset ids, taxonomy, source/artifact descriptors, dependency graph, deterministic import plans, diagnostics, and ratification; `ProjectFileV2` migration exists in `domain/editor/editor_persistence/src/project_file.rs`; `world_sdf` owns field-product descriptors and ratification; `world_ops` owns generic product invalidation/build helpers; and the editor app owns initial catalog runtime, import jobs, field-product jobs, and first Asset Browser/Import Inspector/Field Product Viewer/SDF Brush Browser providers. M5 now adds external runtime preview, project-owned reload status classification, world_sdf runtime intake, and restart boundaries for the existing product families.
- `domain/material_graph` and `domain/texture` now exist as initial M6 domain-contract crates with ratifiers, descriptor/product contracts, source/lineage metadata, and focused tests.
- Descriptor-first material and texture providers exist for material graph canvas, material inspector, material preview, Texture2D/generated texture inspection, and Texture3D/volume inspection. They expose domain descriptors, source/cache/reload diagnostics, and fail-closed adapter boundaries without making editor canvas state authoritative.
- `domain/procgen` now exists for the Phase 6A domain-first procgen product track: graph-backed deterministic documents, first-slice terrain/material node semantics, ratification, lowering to `world_ops` operation windows, and product job/publication descriptors. There is still no `domain/particles`, `domain/physics`, `domain/animation`, or `domain/simulation_process`.
- Concrete editor providers for procedural generation preview, particles, physics authoring/debug, animation timeline, curve editing, and simulation preview do not exist yet; their M6 workspace surfaces still route through fail-closed placeholder diagnostics.
- There is no `domain/gameplay_graph`, Action/Trigger/Rule IR, gameplay graph compiler, gameplay graph to ECS query/event/schedule lowering, or gameplay graph editor/debug provider.

## Implementation Readiness

- M1 through M3 are complete against current editor, shell, UI, scene, SDF, and persistence docs.
- M3.5 is closed as the UI/editor infrastructure slice: the closeout candidate landed on 2026-05-06, and the follow-up toolbar/provider fixture migration seams were closed afterward.
- M3.6 is complete as of 2026-05-06 for authored definition editing, retained preview, and explicit apply/rollback snapshots. Follow-up self-authoring maturity now wires applied theme definitions into the live editor host theme, forms applied workspace layout definitions into live `WorkspaceState`, exports definitions as versioned packages, and activates UI template/editor-binding/menu/shortcut/command-binding/panel-registry/tool-surface-registry catalogs before the next shell frame.
- M3.7 is complete as a no-compromise viewport expression architecture closeout as of 2026-05-08. Multi-viewport previews now have explicit viewport instances, viewport-scoped products, per-viewport render jobs, persisted restore metadata and runtime settings, lifecycle-before-frame-submit sync, viewport-keyed camera/debug/root commands, camera orbit/pan/zoom routing, and duplicate/close lifecycle cleanup. The follow-on provider surface workflow redesign and surface/product maturity pass are also complete as of 2026-05-08 for typed surface wrappers, entity-table query workflows, inspector enum mutation routing, reusable-control polish, visible descriptor-only field/atlas/volume/brickmap/history viewport products, and guard coverage. The M4A-M4I integrated UI/editor/asset foundation and M5 external runtime preview/reload boundary are complete as of 2026-05-09; the SDF-first execution program subsequently completed through Phase 6D and is now a historical substrate record rather than a current activation gate.
- Native multi-window editing is designed in `docs-site/src/content/docs/design/accepted/editor-native-multi-window-presentation-design.md`. It follows the render product-surface foundation and should land before second-monitor workflows are treated as product-ready.
- M4 is the integrated UI/editor/asset foundation and now ends at M4I. M4A-M4E finished active UI/editor consumption and reusable-control cleanup; M4F-M4I add the first SDF/field-first asset contracts, `ProjectFileV2`, field-product descriptors, generic product invalidation, app-owned import/field-product jobs, first asset surfaces, scene-manifest catalog adapter, and displayable `Rgba8Unorm` viewport debug products. M5 is complete for external runtime preview, project-owned data reload classification, and restart boundaries over the existing product families.
- M6 is not one implementation ticket. M6.0 shared workspace substrate has landed, and M6.1 material/texture now has descriptor-first domain contracts plus provider surfaces. Each remaining M6 sub-milestone requires its own current owning design/domain authority and GitHub activation, and must consume the already-completed product-job, query-snapshot, publication, strict-consumer, render-selection, and derived-residency substrate.
- M7 is implementation-ready only for preview/play/session boundaries first. Gameplay graph, particles, physics, animation, procgen, and simulation hot reload depend on their formed-product contracts from M6 and on the execution fabric substrate.
- Later self-authoring packaging/extensibility is implementation-ready for the retained UI path only. Compiled-reactive or ECS-driven UI execution remains blocked; neither strategy was promoted before M2, and any future promotion requires a separate active design or accepted ADR plus a roadmap update.
- M9 is release-readiness verification, not a feature construction phase.

## Next Execution Order

Use this order after the 2026-05-08 M3.7, surface workflow, and self-authoring maturity closeout. The sequence is intentionally more detailed than the milestone names because the next work crosses editor definitions, shell routing, input, providers, asset catalog contracts, runtime preview, and later UI execution strategies.

Do not open a later implementation phase while an earlier phase still has live activation, routing, or transactional replacement gaps, except for a narrow design-only update that unblocks the next phase.

### M4A - Active Command Binding Spine

Status: complete as of 2026-05-09. Implemented in `apps/runenwerk_editor/src/shell/command_resolution.rs`, `apps/runenwerk_editor/src/shell/applied_editor_definition/catalogs.rs`, and `domain/editor/editor_shell/src/composition/build_editor_shell.rs`. Active command-binding activation is transactional, authored route targets resolve through an app-owned `KnownEditorCommand` vocabulary, and `ui_definition` remains inert route data.

Purpose: make command bindings the app-owned bridge from authored route ids to known editor/app commands. This must land before shortcuts and menus consume active definitions, otherwise both paths will grow separate command mapping logic.

Implementation order:

- `domain/editor/editor_definition/src/command.rs::EditorCommandBindingSetDefinition`
  - keep the schema as data only: route target, known command key, capabilities, and undoability metadata;
  - do not add closures, script entry points, provider behavior, retained widgets, or ECS mutation details.
- `apps/runenwerk_editor/src/shell/applied_editor_definition/catalogs.rs::ActiveEditorDefinitionCatalogs`
  - add transactional command-binding replacement validation before mutation;
  - reject duplicate route targets, duplicate binding ids in a set, and command keys that are not recognized by the app-owned dispatcher;
  - preserve the previous active binding set on invalid activation.
- `apps/runenwerk_editor/src/shell/dispatch_shell_command.rs`
  - expose the app-owned known-command vocabulary that active command bindings may target;
  - keep actual execution in the existing shell/app/domain command paths.
- `domain/editor/editor_shell/src/composition/build_editor_shell.rs::route_target_to_action`
  - route formed UI route targets through active command bindings instead of hard-coded toolbar-only route strings where possible.

Validation:

- add regression coverage beside the existing active catalog tests for invalid command-binding activation rollback;
- add guard coverage that `ui_definition` has no command execution or provider behavior;
- add focused tests proving unknown command keys fail closed before catalog replacement.

Exit criteria:

- an authored route id resolves to a known app/domain command through one command-binding path;
- invalid command-binding documents leave the previous active catalog unchanged;
- definitions still cannot execute behavior directly.

### M4B - Active Shortcut Dispatch

Status: complete as of 2026-05-09. Implemented in `apps/runenwerk_editor/src/shell/shortcut_resolution.rs` and `apps/runenwerk_editor/src/runtime/systems/input_bridge.rs`. Active shortcut sets parse to engine `KeyChord`s, dispatch through `KnownEditorCommand`, and invalid shortcut activation preserves the previous active set.

Purpose: replace hard-coded editor shortcuts with active shortcut sets that resolve through the command-binding spine.

Implementation order:

- `domain/editor/editor_definition/src/shortcut.rs::EditorShortcutSetDefinition`
  - keep shortcuts as data: chord, command reference, context, and diagnostics;
  - define conflict diagnostics in editor-definition validation instead of app input code.
- `apps/runenwerk_editor/src/runtime/systems/input_bridge.rs::dispatch_shortcuts`
  - resolve chords against active shortcut sets;
  - map shortcut command references through `ActiveEditorDefinitionCatalogs::command_for_route_target` or an app-owned resolver built on the same catalog;
  - dispatch only known app/domain commands.
- `apps/runenwerk_editor/src/shell/applied_editor_definition/catalogs.rs::ActiveEditorDefinitionCatalogs`
  - validate shortcut replacement transactionally: malformed chords, unknown command references, duplicate chords in the same context, and unsupported contexts must not replace the previous active shortcut set.

Validation:

- shortcut conflict tests in `domain/editor/editor_definition`;
- app tests proving active shortcuts dispatch undo/redo/tool commands through the same command-binding resolver as menus;
- rollback tests proving invalid shortcut activation keeps the previous active shortcut set;
- guard test proving shortcut dispatch does not fall back to string mutation or provider-local behavior.

Exit criteria:

- user-authored shortcut definitions drive input dispatch before the next frame;
- hard-coded shortcuts are reduced to fallback defaults or compatibility fixtures;
- invalid shortcuts never partially replace a working active set.

### M4C - Active Menu And Toolbar Consumption

Status: complete as of 2026-05-09 for in-editor toolbar/menu consumption. Active File/Edit/Window/Workspace menu definitions can supply labels/items and route command keys through the app-owned resolver while checked-in fixtures remain the first-boot fallback. Native OS menus remain deferred.

Purpose: make active menu definitions form toolbar/menu view models instead of keeping `toolbar_adapter` as a mostly hard-coded product.

Implementation order:

- `domain/editor/editor_definition/src/menu.rs::EditorMenuDefinition`
  - keep menu items as labels, hierarchy, availability references, and command references;
  - keep availability generic and descriptive, not provider behavior.
- `apps/runenwerk_editor/src/shell/toolbar_adapter.rs`
  - form visible toolbar/menu items from active menu definitions and command bindings;
  - preserve checked-in fixtures only as fallback defaults when no active user definition exists.
- `domain/editor/editor_shell/src/composition/toolbar_definition.rs`
  - keep formation generic over menu models and route slots;
  - avoid shell-only hard-coded menu item lists where active definitions can supply the product.
- `domain/editor/editor_shell/src/composition/build_editor_shell.rs`
  - route menu item activation through the same app-owned command-binding resolver used by shortcuts.

Validation:

- app tests proving a valid menu activation changes the next shell frame;
- invalid menu activation rollback tests;
- tests proving disabled/unavailable menu entries render from generic availability descriptors without executing provider behavior;
- guard coverage that menu definitions do not bypass command-binding resolution.

Exit criteria:

- File/Edit/Window/Workspace menu products are active-definition-driven;
- fallback checked-in menus still work for first boot and compatibility;
- menu commands and shortcut commands share one resolver.

### M4D - Active Panel And Tool-Surface Registry Projection

Status: complete as of 2026-05-09 for active registry projection. Panel registries now validate default tool-surface compatibility, active tool-surface registries drive future switch/create ordering, active panel ordering is projected into the frame model for future choices, and registry activation does not mutate existing `WorkspaceState`.

Purpose: finish UI-B by making active panel and tool-surface registries drive future creation/switch choices, without rewriting existing workspace state unless a workspace layout definition is applied.

Implementation order:

- `domain/editor/editor_definition/src/registry.rs::EditorPanelRegistryDefinition`
  - preserve authored registry order and validate duplicate ids, unknown definition keys, workspace compatibility, and default tool-surface compatibility.
- `domain/editor/editor_definition/src/registry.rs::EditorToolSurfaceRegistryDefinition`
  - preserve authored order, deduplicate known ids deterministically, and ignore or diagnose unknown ids according to the existing compatibility policy.
- `apps/runenwerk_editor/src/shell/applied_editor_definition/compatibility.rs`
  - keep removal/incompatibility checks before active registry replacement.
- `domain/editor/editor_shell/src/composition/build_editor_shell.rs::available_tool_surface_kinds`
  - consume active registry-projected choices for switch/new-tab menus.
- `domain/editor/editor_shell/src/workspace/surface_contract.rs::panel_kind_definition_key`
  - keep structural `PanelKind` and `ToolSurfaceKind` mapping vocabulary domain-owned.

Validation:

- tests for removed active definitions blocking replacement when the current workspace still mounts them;
- tests for duplicate/unknown authored registry ids and stable projected ordering;
- tests proving existing workspace state does not change on registry activation alone;
- tests proving future new-tab/switch choices reflect the active registry.

Exit criteria:

- active registries drive creation and switching choices;
- existing workspace state is preserved unless an authored workspace layout is explicitly applied;
- no partial panel/tool-surface catalog replacement occurs.

### M4E - Reusable Control And Surface Polish Before New Asset Surfaces

Status: complete as of 2026-05-09. Implemented in `domain/editor/editor_shell/src/composition/surface_definition_context.rs`, `domain/editor/editor_shell/src/composition/surface_control_polish.rs`, and `domain/editor/editor_shell/src/composition/build_self_authoring_control_panel.rs`. Surface fixture builders now share scoped context, widget-id registration, node lookup, panel/title/body polish, and compact reusable control helpers; self-authoring control-panel UI composition lives in `editor_shell` while app providers supply lines/actions/routes; architecture guards prevent app providers from constructing reusable controls directly.

Purpose: keep the editor from adding new ad hoc UI while the asset surfaces are being introduced.

Implementation order:

- `domain/ui/ui_widgets/src`
  - keep select, tree, table, tabs, toggle, numeric input, text input, scroll, split, and viewport embed as the default reusable controls.
- `domain/editor/editor_shell/src/composition/surface_control_polish.rs`
  - move repeated shell/provider control polish into shared composition helpers when the same pattern appears across surfaces.
- `apps/runenwerk_editor/src/shell/providers/*`
  - replace surface-specific row/button/select composition with reusable controls where the provider semantics are already represented by DTOs and route proposals.

Validation:

- snapshot or structural tests for surface view-model composition where stable;
- interaction tests for reusable select/toggle/numeric/text/table/tree routes;
- architecture guards proving provider data and route proposals stay outside `ui_definition`.

Exit criteria:

- complete: new asset/import/field-product surfaces can be built through the provider/catalog seams without inventing another local control style;
- complete: reusable-control adoption remained a compatibility cleanup, not a behavior rewrite.

### M4F - Asset Domain Contracts And Project File Foundation

Status: complete as of 2026-05-09 for the first SDF/field-first contract slice. `domain/asset` owns IO-free asset descriptors, import plans, diagnostics, dependency graph, and ratification; `ProjectFileV2` migrates existing V1 files; `world_sdf` owns formed field-product descriptors/ratification; and `world_ops` owns generic product invalidation/build helpers without depending on assets.

Purpose: add the SDF/field-first asset model before app runtime import jobs or asset surfaces depend on it.

Implementation order:

- `domain/asset/src/lib.rs`
  - add the engine-agnostic asset domain crate.
- `domain/asset/src/id.rs`
  - define typed ids for assets, sources, artifacts, import jobs, and revisions.
- `domain/asset/src/kind.rs`
  - define SDF, field-world, `world_sdf`, material, texture, gameplay graph, particles, physics, animation, procgen, UI, script, shader, and foreign-reference kinds.
- `domain/asset/src/source.rs`
  - model source roots, source hashes, provenance, authored field documents, and foreign source roots.
- `domain/asset/src/artifact.rs`
  - model formed products, source maps, cache keys, generated files, and validity.
- `domain/asset/src/dependency_graph.rs`
  - model dependencies and reverse dependencies for invalidation and reload.
- `domain/asset/src/import_settings.rs`
  - model import settings without executing tools.
- `domain/asset/src/import_plan.rs`
  - build deterministic import plans without app IO.
- `domain/asset/src/diagnostics.rs`
  - define stable pipeline diagnostic codes.
- `domain/asset/src/ratification.rs`
  - ratify imported, migrated, generated, and externally supplied asset candidates.
- `domain/world_sdf/src/product.rs` and `domain/world_sdf/src/ratification.rs`
  - define and ratify formed field payload descriptors before they become catalog-visible artifacts.
- `domain/editor/editor_persistence/src/project_file.rs`
  - add `ProjectFileV2` with asset roots, startup document, source roots, cache roots, profile defaults, and migration from `ProjectFileV1`.

Validation:

- `cargo metadata --no-deps` after adding the crate;
- `cargo test -p asset -p editor_persistence`;
- docs update to `DOMAIN_MAP.md` and `CRATES.md` when the crate lands;
- tests proving mesh/glTF remain foreign-reference kinds, not the primary world substrate.

Exit criteria:

- project files can name asset roots and cache roots;
- import planning is deterministic and IO-free at the domain layer;
- field-world products have domain-owned descriptor and ratification contracts.

### M4G - First Asset Runtime Catalog And Editor Surfaces

Status: complete as of 2026-05-09 for first read-only surfaces. The editor app owns the asset catalog runtime and providers for Asset Browser, Import Inspector, Field Product Viewer, and SDF Brush Browser; the shell exposes the new panel/tool-surface kinds through existing surface catalogs and persisted workspace V4 while default layouts remain unchanged.

Purpose: make asset surfaces real editor products through active panel/tool-surface catalogs instead of hard-coded shell additions.

Implementation order:

- `apps/runenwerk_editor/src/asset_pipeline/catalog_runtime.rs`
  - own open-project asset catalog state, diagnostics, dirty asset sets, dirty field-product sets, and preview state.
- `apps/runenwerk_editor/src/shell/providers/asset_browser.rs::AssetBrowserProvider`
  - browse project assets through provider DTOs.
- `apps/runenwerk_editor/src/shell/providers/import_inspector.rs::ImportInspectorProvider`
  - inspect import settings, diagnostics, sources, artifacts, dependencies, and reimport actions.
- `apps/runenwerk_editor/src/shell/providers/field_product_viewer.rs::FieldProductViewerProvider`
  - preview SDF fields, material channels, `world_sdf` products, provenance, freshness, and invalidation diagnostics.
- `apps/runenwerk_editor/src/shell/providers/sdf_brush_browser.rs::SdfBrushBrowserProvider`
  - browse SDF brush/layer assets.
- `apps/runenwerk_editor/src/shell/providers/mod.rs::EditorSurfaceProviderRegistry`
  - register these surfaces through existing provider seams and make their availability visible through active panel/tool-surface catalogs.

Validation:

- provider tests proving asset surfaces produce DTOs without app IO leaking into domain crates;
- shell tests proving active registries can expose the new surfaces;
- no new hard-coded menu, shortcut, or panel insertion path outside active definition catalogs.

Exit criteria:

- the first asset surfaces are reachable only through active editor surface catalogs or checked-in fallback definitions;
- provider behavior remains app-owned.

### M4H - Import Execution And Field Product Jobs

Status: complete as of 2026-05-09 for the first executable workflow. App-owned import jobs run deterministic domain plans, preserve the prior valid artifact on controlled failures, report missing Blender as a foreign-reference import diagnostic, form ratified field-product candidates, and expose a scene-manifest asset-catalog adapter while preserving loose manifest fallback.

Purpose: execute the domain import and field-product plans with app-owned IO and failure preservation.

Implementation order:

- `apps/runenwerk_editor/src/asset_pipeline/import_jobs.rs::run_import_job`
  - execute `domain/asset` import plans with app-owned IO, host configuration, external tool policy, and diagnostics.
- `apps/runenwerk_editor/src/asset_pipeline/field_product_jobs.rs::run_field_product_job`
  - form SDF graph, field-world, material channel, and `world_sdf` products through app/engine runtime policy.
- `tools/assets/blender_export.py::main`
  - support `.blend` to `.glb` as foreign-reference import only, preserving the prior valid artifact on failure.
- `engine/src/plugins/scene/manifest/catalog.rs::load_scene_manifest_descriptors`
  - migrate loose scene manifest scanning behind an asset-catalog-backed query, with compatibility adapter coverage during migration.

Validation:

- import one authored SDF/field source and form a ratified field product;
- reimport a changed source and prove dependency invalidation reaches `domain/world_ops`;
- prove failed imports preserve the previous valid artifact;
- prove `.blend` import diagnostics do not make mesh/glTF canonical world truth.

Exit criteria:

- asset catalog, import jobs, field-product jobs, diagnostics, and editor surfaces form one recoverable workflow.

### M4I - Viewport Product Producer Breadth And History Workflows

Status: complete as of 2026-05-09 for displayable debug product breadth. Scalar/vector field, atlas, volume-slice, brickmap, and history color products use `Rgba8Unorm` debug descriptors and dynamic target slots first; raw scalar/vector render-target formats and broader history command coverage remain deferred until a later accepted renderer/history feature requires them.

Purpose: connect the viewport product infrastructure to real non-scene producers and broader history behavior.

Implementation order:

- `apps/runenwerk_editor/src/runtime/viewport/producer_scene.rs`
  - keep scene color/picking/overlay products explicit and per viewport.
- `apps/runenwerk_editor/src/runtime/viewport/producer_field.rs`
  - add scalar/vector field preview products.
- `apps/runenwerk_editor/src/runtime/viewport/producer_volume.rs`
  - add atlas, volume, and brickmap debug products.
- `apps/runenwerk_editor/src/runtime/viewport/render_jobs.rs`
  - schedule producer-specific jobs per visible viewport.
- `apps/runenwerk_editor/src/editor_runtime/history/undo_redo.rs`
  - broaden undo/redo workflows for asset-backed scene changes, field product preview changes, and provider-driven commands only after the owning domains expose ratified commands.

Validation:

- product producer tests for field, volume, atlas, brickmap, and history buffer descriptors;
- app tests proving viewport-local product selection remains per-viewport;
- undo/redo tests for any new ratified provider command that mutates authored state.

Exit criteria:

- viewport products are not just descriptor placeholders;
- history workflows cover the first asset/field product command paths without bypassing ratification.

### M5 - Runtime Preview, Data Hot Reload, And Restart Boundaries

Status: complete as of 2026-05-09 for the concrete external-runtime slice. Runtime preview, simulate, and play execution now target an external app-owned runtime child process over loopback QUIC, governed by `docs-site/src/content/docs/adr/accepted/0007-external-runtime-preview-process.md`; the hardened slice has a checked `editor_preview` protocol codec, serialized bootstrap line, headless command/event loop, heartbeat/shutdown round trip, bounded editor command queue, and project-owned reload status over the existing scene/asset/field-product/`world_sdf`/shader/UI-definition contracts while future M6 domains fail closed as unsupported or restart-required. Closeout evidence is recorded in `docs-site/src/content/docs/reports/closeouts/m5-runtime-preview/closeout.md`.

Purpose: connect asset/product changes to runtime preview after catalog and import execution exist.

Implementation order:

- M5.0 design gate:
  - `docs-site/src/content/docs/design/active/engine-game-runtime-editor-ecs-scripting-hot-reload-design.md`
    - record the external runtime preview process boundary, ownership rules, reload/restart boundary, and validation expectations.
  - `docs-site/src/content/docs/adr/accepted/0007-external-runtime-preview-process.md`
    - accept the long-term process, protocol, transport, and engine/app ownership decision.
- `apps/runenwerk_editor/src/editor_app/state.rs`
  - separate edit, preview, simulate, and play session state.
- `domain/editor/editor_preview/src/lib.rs`
  - own engine-agnostic preview session, command, event, reload status, reload decision, runtime product DTOs, checked preview payload metadata, and serialized bootstrap format.
- `net/engine_net/src/protocol/`
  - add generic bidirectional typed payload messages without editor-specific enum variants.
- `apps/runenwerk_runtime_preview/src/main.rs`
  - run the external preview child process with a separate preview/play window, loopback QUIC endpoint, bootstrap connection output, and headless command/event mode that waits for explicit shutdown.
- `apps/runenwerk_editor/src/runtime/preview_process/`
  - own editor-side child spawn/connect/heartbeat/shutdown, bounded command queueing, mode requests, and status ingestion.
- `apps/runenwerk_editor/src/asset_pipeline/catalog_runtime.rs`
  - classify existing catalog/import/field-product changes as live reload, preview-session restart, runtime-process restart, unsupported, failed-preserved, or rejected;
  - project reload status lines into the existing Asset Browser, Import Inspector, and Field Product Viewer surfaces.
- `engine/src/plugins/world/build/`
  - provide a narrow runtime intake helper for ratified `world_sdf` payload packages, reusing build-output invariants instead of letting the editor write `SdfChunkStore` internals.
- `engine/src/plugins/shared/reload.rs`
  - reuse reload status payloads for asset and runtime product streams.
- `docs-site/src/content/docs/design/active/engine-game-runtime-editor-ecs-scripting-hot-reload-design.md`
  - keep the data hot reload versus Rust rebuild/restart boundary aligned with implementation.

Validation:

- data/assets reload live where safe;
- unsafe structural changes require restart or session rebuild;
- play/simulate cannot mutate authored documents without explicit commands.
- external runtime preview process spawn/connect/heartbeat/shutdown works in headless tests.
- authored material graph, procgen, particle, physics, animation, gameplay graph, graph execution, and scripting families fail closed until their owning runtime/product contracts exist; formed material and texture product families may require preview-session restart until live-reload adapters exist.
- validation passed with `cargo metadata --no-deps --format-version 1`, `cargo test -p editor_preview -p engine_net -p engine_net_quic`, `cargo test -p runenwerk_editor -p runenwerk_runtime_preview -p engine -p asset -p world_sdf`, `python3 tools/docs/validate_docs.py`, `./quiet_editor_gate.sh`, and `./quiet_full_gate.sh`.

Exit criteria:

- preview/play has explicit state boundaries;
- reload failures produce diagnostics and preserve prior valid runtime products where safe.

### M6 - Procedural And Gameplay Authoring Domains

Purpose: build later procedural editors on the asset/product/reload foundation instead of adding disconnected graph tools.

Implementation order:

- shared graph/workspace substrate for new document kinds;
- `domain/material_graph` and `domain/texture`;
- `domain/procgen`;
- narrower gameplay event/action/state/quest contracts;
- `domain/gameplay_graph` ATR IR and ECS query/event/schedule lowering;
- `domain/particles`;
- `domain/physics`;
- `domain/animation`;
- simulation/world-process previews.

Validation:

- each domain lands with source lineage, diagnostics, failed-product preservation, provider tests, reload boundary tests, and consumes the completed product-job, publication, query-snapshot, strict-consumer, render-selection, and derived-residency contracts established by the SDF-first execution program;
- gameplay graph lowering consumes semantic gameplay contracts and emits formed products with source maps, not live graph traversal.

Exit criteria:

- each implemented domain has a product contract, editor provider, diagnostics, save/load path, and runtime/preview boundary.

### M7 - Scripting And Runtime Gameplay Bridge

Purpose: add language-neutral scripting and runtime gameplay execution only after formed product contracts exist.

Implementation order:

- future `domain/script_runtime/src/lib.rs`
  - define language-neutral script asset/runtime command contracts.
- future `adapters/script_rhai/src/lib.rs`
  - implement Rhai as the first adapter candidate without leaking Rhai types into domain contracts.
- future `engine/src/plugins/gameplay_graph`
  - instantiate formed gameplay graph products into ECS query descriptors, event subscriptions, schedule edges, source maps, and authority metadata.

Validation:

- scripts request domain/runtime commands instead of mutating ECS internals directly;
- gameplay execution consumes formed products, not authored graph nodes.

Exit criteria:

- runtime scripting and gameplay bridge preserve Rust/domain ownership of correctness.

### M8 - Packaging, Externalization, And Native Integration

Purpose: harden long-lived exchange after active definitions, assets, runtime preview, and provider catalogs are stable.

Implementation order:

- `domain/editor/editor_definition/src/package.rs`
  - package manifests, dependencies, compatibility ranges, and migration metadata.
- `domain/ui/ui_definition/src/package.rs`
  - UI-template/theme package manifests with no app IO or runtime execution.
- `apps/runenwerk_editor/src/persistence/editor_definition_package.rs`
  - app-owned import/export policy.
- native OS menus/shortcuts
  - add adapter design only after active in-editor menus/shortcuts/command bindings are already working.
- external marketplace workflows
  - add trust, package identity, compatibility, migration, and diagnostics design before implementation.

Validation:

- package import/export preserves ids, source maps, migrations, and diagnostics;
- invalid packages cannot activate;
- native OS integration mirrors active definition catalogs instead of becoming a second source of truth.

Exit criteria:

- specialized editor packages and native integration consume the same active definition model.

### Design First Gates

These items were easy to miss in the compressed roadmap. They are real future
tracks; some still need design or ADR work before implementation, while
Interaction V2 is implemented for the retained UI target under accepted ADR
0009 and its retained slice catalog is the current reference boundary.

- UI Runtime V2 and interaction formation
  - implemented design: `docs-site/src/content/docs/design/implemented/editor-ui-runtime-v2-and-interaction-formation-design.md`;
  - accepted ADR: `docs-site/src/content/docs/adr/accepted/0009-ui-interaction-formation-v2.md`;
  - retain the current retained UI execution target while using `FormedInteractionModel` for popup stack, scroll ownership, focus, menu sizing, chrome slots, docking zones, and status overflow;
  - the implemented retained slices follow the migration spine `definition vocabulary -> validation rule -> FormedInteractionModel record -> retained UI formation adapter -> ui_runtime enforcement -> editor/app guard`;
  - retain the named WR-025 slice catalog as the implementation reference:
    `IV2-menu-stack`, `IV2-scroll-ownership`, `IV2-menu-sizing`,
    `IV2-chrome-slots`, `IV2-dock-drop-zones`, and
    `IV2-status-and-viewport-arbitration`;
  - code-bearing retained slices landed on 2026-05-15 for
    `IV2-menu-stack`, `IV2-scroll-ownership`, `IV2-menu-sizing`,
    `IV2-chrome-slots`, `IV2-dock-drop-zones`, and
    `IV2-status-and-viewport-arbitration`;
    `domain/editor/editor_shell/src/composition/build_editor_shell.rs::build_editor_shell_frame_with_docking_visual_state`
    adapts toolbar/tab-stack/viewport menu scopes, sizing records, structural
    tab chrome slots, dock/drop-zone records, and viewport status/input
    arbitration records, while
    `domain/editor/editor_shell/src/composition/toolbar_definition.rs::project_workspace_close_buttons`
    adapts workspace chrome slots and
    `apps/runenwerk_editor/tests/viewport_architecture_guards.rs::production_input_bridge_allows_viewport_scroll_only_after_ui_declines_ownership`
    keeps production viewport wheel fallback behind UI ownership decline;
  - renderer/product-surface output remains derived data, not UI authority;
  - downstream shell/menu/chrome work consumes these implemented contracts rather than defining local policy;
  - editor shell polish is a retained-UI implementation slice after the relevant Interaction V2 contract, or bounded compatibility evidence; it is not the owner of durable popup, scroll, focus, docking, status-overflow, or viewport-input policy.
- compiled-reactive UI execution
  - promote only through a new active design or accepted ADR;
  - define the first surface that needs it, the formation product, scheduling model, invalidation model, debug/source-map story, and guard tests;
  - implement it as another formation target from normalized UI definitions, not as a replacement for retained UI.
- ECS-driven UI execution
  - promote only for a concrete world-bound UI case where retained UI plus observation bridges is insufficient;
  - define ECS identity, authored id mapping, command boundaries, rollback, and authority constraints;
  - never allow ECS-driven UI to mutate editor/domain state outside command and ratification paths.
- world-space and screen-projected runtime UI
  - design attachment bindings, viewport projection, simulation observation, and command routing before implementation.
- in-game editors
  - design capability gates, command/ratification boundaries, and project safety rules before runtime surfaces can edit authored content.
- payload ECS enum variants
  - wait until the reflection/adapter layer exposes concrete payload-backed enum fields;
  - no-payload enum storage remains the only implemented reflected enum mutation path until then.
- native OS menu/shortcut integration
  - wait until active in-editor menus, shortcuts, and command bindings are consumed live.
- external marketplace or third-party editor packages
  - wait for package metadata, trust policy, compatibility ranges, migration diagnostics, and rollback semantics.
- gameplay graph ATR/ECS lowering
  - implement only after narrower gameplay event, action, state, quest, authority, and source-map contracts exist.
- rendered-world / multi-entity viewport
  - implemented design: `docs-site/src/content/docs/design/implemented/editor-rendered-world-and-multi-entity-viewport-design.md`;
  - WR-018 V1 first implemented multi-entity SDF primitive viewport rendering; current production extraction uses `apps/runenwerk_editor/src/runtime/systems/frame_submit.rs::extract_viewport_scene_render_packet_with_material_slots`;
  - material-slot selection is an additive production extension beyond WR-018 V1; terrain, prefab instancing, streaming, and broader general-mesh/world extraction remain separate product tracks.
- Field Visualizer
  - implemented design: `docs-site/src/content/docs/design/implemented/field-visualizer-product-workflow-design.md`;
  - the viewport product workflow and presentation controls are implemented; additional field-product producers and broader product maturity remain separately owned.
- Material Lab
  - active design: `docs-site/src/content/docs/design/active/material-lab-and-material-preview-design.md`;
  - implementation starts after rendered-world V1 and field visualizer product routing are stable;
  - source-backed `MaterialGraphDocument` is material truth, not canvas state.
- UI Designer
  - not missing: it is the already-promoted self-authoring path in `docs-site/src/content/docs/design/implemented/editor-self-authoring-and-final-ui-design.md`;
  - roadmap wording should continue to call this UI Designer / self-authoring so the feature is visible to product planning.
- SDF Prefabs
  - active V2-gated design: `docs-site/src/content/docs/design/active/sdf-prefab-composition-system-design.md`;
  - prefab runtime instancing waits for rendered-world V1 and source-backed prefab descriptor identity.

### What Was Missing Or Under-Specified

- The previous M4/M5 split made UI live replacement and asset work look sequential but did not explain that asset surfaces must enter through active panel/tool-surface catalogs. That is now explicit.
- Menu and shortcut consumption were listed, but the command-binding resolver was not called out as the first required spine. It is now the first M4 subphase.
- Compiler-inspired UI and ECS-driven UI plans were mentioned as deferred, but not positioned in the future order. They are now design-first gates after retained UI, assets, runtime preview, and concrete need exist.
- Native OS menus/shortcuts and external marketplace workflows were listed as future work, but their prerequisites were vague. They are now tied to active definition consumption and package metadata.
- Payload ECS enums were listed as deferred, but the trigger condition was not explicit. They now wait for concrete payload-backed reflected fields.
- Material Lab was present as provider stubs but not roadmap-visible as a product track. It is now explicit and gated behind rendered-world V1 plus field visualizer routing.
- UI Designer looked missing because the roadmap used self-authoring language. It is now identified as the same promoted path, not a separate missing feature.
- SDF prefabs were grouped with deferred SDF detail drafts. The design is now active, but implementation remains V2-gated behind rendered-world V1 and source-backed asset identity.
- Repeated editor UI polish issues were listed as individual bugs. Accepted ADR 0009 and Interaction V2 now turn popup, scroll, focus, menu sizing, chrome, docking, and status overflow into shared contracts.
- Editor shell polish is now explicitly behind Interaction V2. It may consume
  defined retained-UI contract slices or provide compatibility evidence, but it
  may not define long-term popup, scroll, focus, docking, status, or viewport
  input policy on its own.

## Milestones

### M0 - Governance And Evidence Baseline

Exit criteria:

- This roadmap is canonical for app-level sequencing.
- Asset pipeline architecture is captured in `docs-site/src/content/docs/design/active/editor-asset-pipeline-and-content-workflow-design.md`.
- Procedural authoring, material/texturing, particles, physics, animation, and simulation workflows are captured in `docs-site/src/content/docs/design/active/editor-procedural-content-and-simulation-workflow-plan.md`.
- Gameplay graph ATR IR, compiler passes, SDF physics relations, and ECS query/event/schedule lowering are captured in `docs-site/src/content/docs/design/active/gameplay-graph-atr-ir-and-ecs-lowering-design.md`.
- The roadmap explicitly follows the SDF-first field-world direction in `docs-site/src/content/docs/design/accepted/sdf-first-field-world-platform-design.md`, standalone `dornglut/runen-sdf`, and `docs-site/src/content/docs/domain/world-sdf/README.md`.
- UI execution strategy is closed for M1 through M7 and M3.5/M3.6: retained tree UI plus tool-surface/canvas hybrid is the implementation path. Compiled-reactive or ECS-driven UI execution remains deferred and may not enter self-authoring as a first-time decision.
- ADR 0009 makes Interaction V2 the shared editor UI behavior contract before further popup, scroll, chrome, docking, status-overflow, and viewport-input work. The required spine is definition vocabulary, validation rule, `FormedInteractionModel`, retained UI formation adapter, `ui_runtime` enforcement, and editor/app guard. The `IV2-menu-stack`, `IV2-scroll-ownership`, `IV2-menu-sizing`, `IV2-chrome-slots`, `IV2-dock-drop-zones`, and `IV2-status-and-viewport-arbitration` runtime-backed slices are implemented and doctrine-repaired; downstream implementation must consume the named contract slice it depends on instead of defining local policy. `docs-site/src/content/docs/reports/closeouts/wr-025-interaction-v2-doctrine-repair/closeout.md` closes the invalid dock/drop target and viewport/status behavior evidence.
- Existing MVP, UI, editor, render, and runtime design docs link to this roadmap without restating stale phase order.
- `python3 tools/docs/validate_docs.py` passes.

### M1 - Editor Structural Core Closed

Status: complete as of 2026-05-05. The M1 scope is implemented and covered by focused editor core, scene, shell, app, scene-authoring smoke, viewport architecture guard, formatting, and docs validation checks. M2 and M3 are also complete; M3.5 UI definition formation framework is validated, M3.6 UI self-authoring is complete as of 2026-05-06, M4A-M4I active editor-definition/asset foundation work is complete as of 2026-05-09, and M5 external runtime preview/reload boundary is complete as of 2026-05-09. Procedural domains and gameplay graph remain deferred to M6.

Purpose: close the structural seams that every later feature depends on.

Implementation targets:

- `domain/editor/editor_core/src/document.rs::DocumentKind`
  - replace the coarse `Asset`, `Resource`, `Tool`, and catch-all usage with explicit document kinds for scene, prefab, SDF graph, SDF brush/layer, field-world definition, field product preview, material graph, material, procedural texture, Texture3D/volume texture, procedural generation graph, gameplay graph, gameplay rule/trigger, ability, quest, particle graph, particle emitter, physics scene/config, animation clip, animation graph, timeline, UI layout, graph, script, foreign mesh/reference import, asset catalog, runtime debug, workspace definition, theme, shortcut, menu, command binding, panel registry, and tool-surface definition.
- `domain/editor/editor_core/src/session.rs::EditorSession`
  - add ordered document-tab state, active document switching commands, close/dirty/save state transitions, and compatibility validation hooks.
- `apps/runenwerk_editor/src/editor_runtime/document/mod.rs`
  - split scene runtime document state from generic document-tab runtime state.
- `domain/editor/editor_shell/src/surface_provider.rs`
  - close the provider DTO contract for workspace profile, document kind, surface definition, capabilities, and provider-local route results.
- `apps/runenwerk_editor/src/shell/providers/`
  - split the current monolithic provider module into subdomain modules with `mod.rs` boundaries: `outliner`, `entity_table`, `viewport`, `inspector`, `console`, and provider registry composition.
- `domain/editor/editor_core/src/ratification.rs`
  - add a domain-owned scene-change ratification parameter object for operation orchestration.
- `domain/editor/editor_scene/src/operations/execute_scene_command.rs`
  - move single-command scene operation orchestration behind narrow domain-owned context traits.
- `domain/editor/editor_scene/src/operations/execute_scene_transaction.rs`
  - move transaction orchestration behind the same context family.
- `apps/runenwerk_editor/src/editor_runtime/runtime.rs::RunenwerkEditorRuntime`
  - implement those context traits without moving app IO, renderer, or engine integration into domain crates.
- `domain/editor/editor_core/src/session.rs::EditorMode`
  - replace the global enum with mode ids, mode descriptors, and workspace/document compatibility rules.

Validation:

- `cargo test -p runenwerk_editor -p editor_core -p editor_scene -p editor_shell`
- `cargo test -p runenwerk_editor --test scene_authoring_workflow_smoke`
- `cargo test -p runenwerk_editor --test viewport_architecture_guards`

### M2 - Final Shell, Docking, Tabs, And Area UI

Purpose: productize the editor shell as a real work surface, not a fixed MVP panel layout.

Status: complete as of 2026-05-05. The reducer, persistence, shell command, tab chrome, interaction mapping, app allocator/pruning seams, dynamic workspace graph projection/composition, and projected-host split resizing are implemented and covered by focused shell/app tests. The old fixed-layout projection remains only as a compatibility view for legacy default-layout guards.

Implementation targets:

- `domain/editor/editor_shell/src/workspace/reducer.rs::WorkspaceMutation`
  - add workspace mutations for new panel tab allocation, close tab, close other tabs, split area horizontally, split area vertically, duplicate area, close area, reset area, lock area type, and saved layout preset application.
- `domain/editor/editor_shell/src/workspace/state.rs::WorkspaceState`
  - preserve `PanelInstanceId` and `ToolSurfaceInstanceId` identity through every new structural mutation.
- `domain/editor/editor_shell/src/composition/build_editor_shell.rs::build_tab_strip_from_frame`
  - render editor type selector, area tabs, plus/new-tab button, split/menu/close controls, and consistent area chrome.
- `domain/editor/editor_shell/src/commands/shell_command.rs::ShellCommand`
  - add commands for new panel tab creation, area close, tab close, area split, area menu actions, and document-tab actions.
- `domain/editor/editor_shell/src/commands/map_interactions.rs::map_interactions_to_shell_commands`
  - map `UiInteraction::SelectChanged` to `SwitchPanelToolSurfaceKind`, map tab controls to tab commands, map tree/table/toggle/numeric interactions for shell-owned controls, and keep provider-local actions routed through provider proposals.
- `apps/runenwerk_editor/src/shell/state.rs::RunenwerkEditorShellState`
  - allocate new panel and tool-surface identities through `WorkspaceIdentityAllocator` and prune surface sessions only after structural state is ratified.
- `domain/editor/editor_shell/src/workspace/persisted.rs`
  - persist the full saved workspace layout, including tab stacks, active panels, floating hosts, split fractions, area type locks, and durable layout identity.

Validation:

- `cargo test -p editor_shell`
- `cargo test -p ui_runtime -p ui_widgets`
- `cargo test -p runenwerk_editor -p editor_shell -p ui_runtime`

### M3 - Scene Authoring Feature Complete

Purpose: finish the core 3D editor before expanding into every other workspace.

Status: complete as of 2026-05-05. The M3 scope is implemented and covered by focused scene authoring smoke, app runtime, shell, inspector, `editor_scene`, formatting, docs validation, and full gate checks. M3.5 UI definition formation framework is implemented and validated as of 2026-05-06; M3.6 UI self-authoring is complete and the integrated M4 UI/editor/asset foundation follows it.

Implementation targets:

- `domain/editor/editor_scene/src/scene_command.rs::SceneCommandIntent`
  - add duplicate entity/subtree, create SDF primitive, create child, batch delete, set transform, reset transform, and add/remove common component actions.
- `apps/runenwerk_editor/src/editor_features/scene_commands.rs`
  - expose create, delete, duplicate, rename, reparent, add component, remove component, and common SDF primitive actions as shell/provider commands.
- `apps/runenwerk_editor/src/editor_features/viewport/tools.rs`
  - add rotate and scale tool definitions alongside select and translate.
- `apps/runenwerk_editor/src/editor_features/viewport/interaction.rs`
  - implement rotate and scale gizmo interaction, snap support, cancellation, preview, and commit behavior through scene transactions.
- `apps/runenwerk_editor/src/editor_runtime/transform_preview.rs`
  - generalize preview state for translate, rotate, and scale without making preview state authoritative.
- `domain/editor/editor_inspector/src/model/`, `domain/editor/editor_inspector/src/editing.rs`, and `apps/runenwerk_editor/src/editor_panels/inspector_panel.rs`
  - make common reflected fields editable through reusable controls and typed inspector edit values.
- `domain/editor/editor_scene/src/sdf_authoring/`
  - add scene-authoring contracts for SDF primitives, boolean composition intent, brush/layer metadata, and SDF preview diagnostics without making scene graph ownership replace standalone RunenSDF semantics or `domain/world_sdf`.
- `apps/runenwerk_editor/src/editor_features/viewport/sdf_tools.rs`
  - expose SDF brush, add/subtract, smooth/blend, and surface-pick interactions as editor tools routed through scene/world commands.
- `domain/editor/editor_shell/src/composition/build_outliner_panel.rs`
  - replace ad hoc row buttons with the retained tree control where hierarchy semantics require expand/collapse and selection.
- `apps/runenwerk_editor/src/persistence/files.rs`
  - keep scene save/load going through migration, normalization, formation, apply, and retained change log persistence.

Validation:

- `cargo test -p runenwerk_editor --test scene_authoring_workflow_smoke`
- `cargo test -p runenwerk_editor -p editor_inspector -p editor_scene`
- GPU smoke manually when visual acceptance matters:
  `RUNENWERK_ENABLE_GPU_SMOKE=1 RUNENWERK_ENABLE_MACOS_MAIN_THREAD_GPU_SMOKE=1 cargo test -p runenwerk_editor --test viewport_gpu_truth_smoke -- --ignored`

### M3.5 - UI Definition Formation Framework

Purpose: introduce the first full UI definition framework before M3.6 and M4, including generic UI templates, RON-authored fixture loading, editor bindings, retained UI formation, and migration of stable editor chrome/common surface structure. M3.6 starts the visual editor and user-authored document lifecycle on top of this framework.

Owning design:

- `docs-site/src/content/docs/design/implemented/ui-definition-formation-foundation-design.md`

Rationale:

- M4 and later milestones will add many asset, workspace, menu, diagnostics, and preview surfaces; adding them through more shell/app hard-coding would multiply the current toolbar/layout friction.
- The framework belongs above the retained UI substrate, not inside editor-specific shell code.
- Authored UI definitions should be stable source/IR. Retained UI is the first formation target, not the permanent authored format.
- M3.6 should edit and manage definitions immediately after this slice; it should not be the first milestone to define the underlying UI definition contracts.
- The milestone should prove the model by migrating the toolbar, menus, workspace switcher, shell chrome, and common provider surface structure instead of leaving legacy builders as parallel sources of truth.

Implementation targets:

- `domain/ui/ui_definition/src/lib.rs`
  - add the engine-agnostic UI definition crate under the UI domain crate family.
- `domain/ui/ui_definition/src/identity.rs`
  - define stable authored UI ids that are distinct from `WidgetId`, focus/capture ids, `PanelInstanceId`, `ToolSurfaceInstanceId`, and ECS entity ids.
- `domain/ui/ui_definition/src/template.rs`, `src/node.rs`, `src/slot.rs`, `src/menu.rs`, `src/embed.rs`, and `src/availability.rs`
  - model authored UI templates, structural nodes, controls, menus, slots, repeaters, template refs, embed slots, and availability definitions without editor-specific command semantics, retained `UiNodeKind`, runtime `WidgetId`, ECS entity ids, or compiled update functions.
- `domain/ui/ui_definition/src/normalize.rs`
  - canonicalize ordering, resolve generic references, and report duplicate ids or malformed structures.
- `domain/ui/ui_definition/src/validate.rs`
  - validate ids, slot references, template refs, repeaters, embed slots, availability refs, and unsupported node combinations with structured diagnostics.
- `domain/ui/ui_definition/src/form.rs`
  - form validated definitions into retained UI products consumed by `ui_tree`, `ui_widgets`, and `ui_runtime` as the first target; formation emits route slots, embed slots, authored paths, and availability/diagnostic state, not command execution.
- `domain/editor/editor_definition/src/lib.rs`
  - add the editor-specific definition crate above `ui_definition` and below `editor_shell`.
- `domain/editor/editor_definition/src/toolbar.rs`, `src/menu.rs`, `src/workspace.rs`, `src/surface.rs`, `src/command.rs`, `src/availability.rs`, `src/binding.rs`, `src/validate.rs`, and `src/form_editor_ui.rs`
  - define editor menu ids, workspace catalog entries, command route ids, availability descriptors, toolbar bindings, shell chrome bindings, and common provider surface template bindings without importing active shell runtime state or app IO.
- `assets/editor/ui/*.ron`
  - add checked-in RON templates for toolbar, shell chrome, common provider surfaces, and editor bindings.
- `domain/editor/editor_shell/src/composition/build_toolbar.rs::build_toolbar`
  - migrate toolbar structure to formed definitions and remove `stable_name` string routing as the source of truth.
- `domain/editor/editor_shell/src/composition/build_editor_shell.rs::build_tab_strip_from_frame`
  - template-form stable tab-stack/area chrome while keeping workspace mutation and panel identity ownership in `editor_shell`.
- `domain/editor/editor_shell/src/composition/build_inspector_panel.rs::build_inspector_panel`, `build_outliner_panel.rs::build_outliner_panel`, `build_entity_table_panel.rs::build_entity_table_panel`, `build_viewport_panel.rs::build_viewport_panel`, and `build_console_panel.rs::build_console_panel`
  - migrate repeated list/tree/table/form/chrome structure to templates while providers continue supplying data, embed payloads, and route results.
- `DOMAIN_MAP.md` and `docs-site/src/content/docs/domain/00-overview.md`
  - keep the ownership maps aligned with the active workspace crates.

Completeness status:

- Implemented: crate skeletons and metadata, generic source model modules, editor definition modules, checked-in fixtures, fixture parse/validation tests, retained formation, inert route/embed/path products, app-owned fixture loading, toolbar route-slot integration, toolbar popup item routing through editor binding data, normal shell chrome formation, and common provider surface fixture formation for console, inspector, outliner, entity-table, and viewport stable structure.
- Preserved outside `ui_definition`: provider DTO construction, surface-local routes, editor/app mutations, inspector edit commits, entity/outliner selection semantics, viewport options/status overlays, and render-product/embed payload ownership.
- M3.7 truth after this follow-up: explicit viewport instance identity is a runtime resource used by normal frame/input/provider binding paths, workspace persistence restores viewport instance ids and viewport runtime settings, viewport lifecycle sync runs before shell frame projection and persists viewport-owned settings, frame submit no longer owns live viewport singleton state, per-viewport render jobs and viewport-scoped product targets are published from viewport runtime systems, the viewport shader no longer carries multi-rectangle containment, camera/debug/root state has viewport-keyed command paths, and camera orbit/pan/zoom routes through viewport-local state. The surface workflow and product-maturity follow-up is complete: active provider-backed surfaces route through typed surface action/session/domain wrappers, entity-table query workflows and inspector enum controls are implemented, reusable surface controls are broader, descriptor-only field/atlas/volume/brickmap/history products are visible but unavailable, and provider behavior remains outside `ui_definition`. Remaining work is broader product implementation outside M3.7: integrated M4 active UI/editor catalog consumption, asset/catalog/import foundations, real field/asset/volume producers, and concrete temporal/history buffers.

Non-goals:

- no editor workspace/profile catalog implementation in `domain/ui/ui_definition`;
- no app provider registry, file IO, or runtime instantiation;
- no visual workspace layout designer, menu editor, theme editor, or shortcut editor inside M3.5; those are M3.6;
- no compiled-reactive or ECS-driven UI execution path;
- no retained-tree-specific authored source model that would force template rewrites for future accepted execution strategies;
- no authored ids derived from runtime/session ids or ECS entities.

Validation:

- when code lands: `cargo test -p ui_definition -p editor_definition -p ui_tree -p ui_runtime -p ui_widgets -p editor_shell`;
- after crate metadata changes: `cargo metadata --no-deps`;
- for docs-only planning updates: `python3 tools/docs/validate_docs.py`.

### M3.6 - UI Self-Authoring Workspace And Styling

Purpose: move the former final self-authoring/UI design work into the Now track so Runenwerk can style, inspect, validate, preview, and author UI definitions before later asset, procedural, gameplay, runtime, overlay, and in-game UI surfaces are built.

Status: complete as of 2026-05-06 for the self-authoring document lifecycle and retained preview path. Implemented: versioned UI definition migration wrapper, editor-owned workspace/profile/layout/theme/menu/shortcut/command-binding/panel/tool-surface schemas, editor-definition validation guards for runtime/session identity leakage, Editor Design workspace/profile, structural self-authoring tool-surface kinds, app-owned fixture document loading, retained UI preview, validation diagnostics, command diff/apply preview, retained provider control routes, draft UI hierarchy text edits, draft theme color edits, draft workspace-layout add-tab/split-root/close-tab edits, explicit apply/rollback shell commands, live activation for applied theme definitions, and provider surfaces for definition outliner, UI hierarchy, UI canvas, style inspector, bindings, dock/layout preview, theme editor, shortcut editor, menu editor, definition validation, and command diff. Follow-up maturity completed on 2026-05-08: applied workspace layout definitions now form and replace live `WorkspaceState`, UI templates and editor bindings feed live toolbar/shell-chrome formation, menu/shortcut/command-binding definitions activate into app-owned catalogs, panel/tool-surface registries activate with active-workspace compatibility checks, tool-surface registries feed future switch/create choices, reusable field/control polish covers the active editor surfaces, and export writes a versioned package instead of a bare definition document. M4A-M4I completed on 2026-05-09: active menu consumption, active shortcut dispatch, active panel/tool-surface registry projection, shared reusable-control composition, first asset/import/field-product provider surfaces, and first field/volume debug products are now implemented. M5 completed on 2026-05-09: external runtime preview process contracts, reload status classification/projection, world_sdf runtime intake, and restart boundaries are implemented. Still separate future tracks: richer asset import controls, persisted catalog/cache management, payload ECS enum variants, native OS menu/shortcut integration, external marketplace workflows, compiled-reactive UI execution, and ECS-driven UI execution.

Owning design:

- `docs-site/src/content/docs/design/implemented/editor-self-authoring-and-final-ui-design.md`

Rationale:

- Later editor features will need many custom surfaces. Building those first in Rust and only adding self-authoring at the end would recreate the hard-coded toolbar/surface problem at a larger scale.
- The UI workspace should be the tool used to build later editor UI, debug overlays, and runtime/game UI templates, while behavior and command execution remain owned by their domains/app layers.
- This milestone starts visual authoring, but it remains bounded to definition documents, retained preview/formation, explicit command routing, and ratified changes.

Implementation targets:

- `domain/editor/editor_core/src/document.rs::DocumentKind`
  - activate the UI layout, workspace definition, theme, shortcut, menu, command binding, panel registry, tool-surface definition, and editor definition document kinds for the authoring workspace.
- `domain/editor/editor_definition/src/lib.rs`
  - extend the M3.5 editor binding layer into durable workspace profile, layout, menu, shortcut, command binding, panel registry, and tool-surface definition schemas.
- future `domain/ui/ui_definition/src/migration.rs`
  - add versioned authored UI definition migrations where needed for saved templates and style documents.
- `domain/editor/editor_shell/src/workspace/state.rs`
  - keep active workspace state session-owned and separate from authored workspace definitions while supporting preview/apply/rollback of formed workspace definitions.
- `apps/runenwerk_editor/src/shell/providers/editor_design_outliner.rs::EditorDesignOutlinerProvider`
  - show authored definition hierarchy, references, validation state, and dirty state.
- `apps/runenwerk_editor/src/shell/providers/ui_hierarchy.rs::UiHierarchyProvider`
  - show and edit the authored UI tree through ratified commands.
- `apps/runenwerk_editor/src/shell/providers/ui_canvas.rs::UiCanvasProvider`
  - preview formed UI definitions with selection handles, responsive preview sizes, and state fixtures.
- `apps/runenwerk_editor/src/shell/providers/style_inspector.rs::StyleInspectorProvider`
  - edit theme tokens, style references, layout constraints, spacing, typography, colors, and control state styling.
- `apps/runenwerk_editor/src/shell/providers/bindings_panel.rs::BindingsPanelProvider`
  - edit slot, action route, availability, collection, selection, and embed bindings without executing commands directly.
- `apps/runenwerk_editor/src/shell/providers/dock_layout_preview.rs::DockLayoutPreviewProvider`
  - preview formed workspace definitions before applying them.
- `apps/runenwerk_editor/src/shell/providers/theme_editor.rs::ThemeEditorProvider`
  - edit and preview theme definitions.
- `apps/runenwerk_editor/src/shell/providers/shortcut_editor.rs::ShortcutEditorProvider`
  - edit shortcuts with conflict diagnostics.
- `apps/runenwerk_editor/src/shell/providers/menu_editor.rs::MenuEditorProvider`
  - edit menu definitions and command bindings.
- `apps/runenwerk_editor/src/shell/providers/definition_validation.rs::DefinitionValidationProvider`
  - show blocking errors, warnings, migrations, unresolved references, capability issues, command route issues, and source paths.
- `apps/runenwerk_editor/src/shell/providers/command_diff.rs::CommandDiffProvider`
  - show the exact ratified definition change before apply/activation.
- `docs-site/src/content/docs/design/deferred/ui-model-multiple-execution-strategies-design.md`
  - keep the M0 UI execution-strategy decision visible during self-authoring. M3.6 must not introduce compiled-reactive or ECS-driven UI execution for the first time; any future promotion requires a separate active design or accepted ADR, guard tests, and roadmap update.

Validation:

- Implemented and covered: create, duplicate, rename, delete, import, export, validate, preview, apply, rollback, migrate, retained provider control routing, draft UI node text edits, draft theme color edits, draft workspace-layout add-tab/split-root/close-tab edits, live host theme activation for applied theme definitions, live workspace-state replacement for applied workspace layout definitions, versioned export packaging, and reusable field/control polish at the app/domain seam; validation blocks malformed definitions from becoming active; checked-in UI fixtures load as editable definition documents; retained previews form before apply; active runtime/session-only id vocabulary is rejected in editor authored ids; self-authoring follows the retained UI execution strategy closed in M0 and does not choose compiled-reactive or ECS-driven UI execution for the first time.
- Completed after M3.6 in the integrated M4 track: live menu/shortcut/command-binding/panel/tool-surface consumption as of 2026-05-09. Still deferred beyond M3.6: asset/procedural/gameplay/runtime overlay authoring after the asset foundations start; payload ECS enum variants when reflected payload fields exist; native OS menu/shortcut integration; external marketplace workflows; compiled-reactive UI execution; and ECS-driven UI execution.

### M3.7 - Viewport Expression Architecture Closeout

Purpose: replace the tactical split-viewport bridge with the final panel-owned viewport expression architecture before the editor depends on viewports for broader asset, field, material, runtime debug, and procedural previews.

Owning roadmap:

- `docs-site/src/content/docs/apps/runenwerk-editor/viewport-expression-implementation-roadmap.md`

Owning design:

- `docs-site/src/content/docs/design/implemented/workspace-viewport-expression-upgrade-design.md`
- `docs-site/src/content/docs/design/implemented/render-product-surface-foundation-bundle-design.md`

Status: complete as of 2026-05-08. Explicit viewport instance identity, persisted restore metadata and runtime settings, lifecycle-before-frame-submit sync, viewport-scoped product targets, per-viewport render jobs, shader containment cleanup, viewport-keyed camera/debug/root commands, viewport-local camera input, duplicate settings copy, and closed-surface cleanup are implemented. Further viewport producer breadth or polish belongs to later product milestones, not M3.7 migration work.

Implementation targets:

- `apps/runenwerk_editor/src/runtime/viewport/instance_registry.rs`
  - introduce explicit viewport instance allocation, duplication, restore, close, and `ToolSurfaceInstanceId -> ViewportId` mapping.
- `apps/runenwerk_editor/src/runtime/viewport/render_state.rs`
  - make bounds, dimensions, camera, presentation/debug mode, source version, throttling, and target status per-viewport state.
- `apps/runenwerk_editor/src/runtime/viewport/render_jobs.rs`
  - derive one `ViewportRenderJob` per visible viewport.
- `apps/runenwerk_editor/src/runtime/viewport/surface_set.rs`
  - replace shared static surface labels with viewport-scoped concrete product targets.
- `apps/runenwerk_editor/src/runtime/viewport/producer_scene.rs`
  - render scene color, picking ids, overlay, and later depth/debug products per viewport job.
- `assets/shaders/editor_viewport_scene_product.wgsl`
  - remove `viewport_b`, `viewport_c`, and `viewport_d`; render one viewport-local target per job.
- `apps/runenwerk_editor/src/runtime/systems/frame_submit.rs::submit_editor_frame_system`
  - stop owning viewport lifecycle and render-product synchronization.
- `apps/runenwerk_editor/tests/viewport_architecture_guards.rs`
  - reject shared viewport products, multi-rect shader uniforms, frame-submit viewport identity seeding, and first-observed viewport fallbacks.

Exit criteria:

- every visible viewport has explicit instance identity and independent scene color, picking, and overlay product targets;
- viewport camera, product selection, overlay selection, diagnostics visibility, bounds, and resize state are independent per viewport;
- UI embeds reference `(ViewportId, ViewportSurfacePresentationSlot)` and never sample a shared global viewport product accidentally;
- picking and transform interaction use viewport-local coordinates and viewport-local camera state;
- closing, splitting, duplicating, hiding, resizing, and restoring viewport surfaces preserve explicit lifecycle semantics;
- no normal runtime path depends on `viewport_b`-style shader containment or shared static viewport product labels.

Validation:

- `cargo test -p runenwerk_editor viewport`
- `cargo test -p runenwerk_editor --test startup_render_smoke`
- `cargo test -p runenwerk_editor --test viewport_architecture_guards`
- `RUNENWERK_ENABLE_GPU_SMOKE=1 RUNENWERK_ENABLE_MACOS_MAIN_THREAD_GPU_SMOKE=1 cargo test -p runenwerk_editor --test viewport_gpu_truth_smoke -- --ignored`
- `python3 tools/docs/validate_docs.py`
- `./quiet_full_gate.sh`

### M4 - Integrated UI/Editor Live Replacement And Asset Pipeline Foundation

Purpose: finish the active editor-definition consumption path and make project
content explicit and field-world-first instead of loose files discovered by
unrelated runtime systems. Asset browser, import inspector, field-product
viewer, and SDF brush browser surfaces should land through the same active
panel/tool-surface definition seams that the Editor Design workspace can author.

Status: complete as of 2026-05-09 for M4A-M4I. Active editor-definition consumption, reusable-control cleanup, the first SDF/field-first asset contracts, `ProjectFileV2`, field-product descriptors, generic product invalidation, app-owned import and field-product jobs, first asset provider surfaces, scene-manifest catalog adapter, and displayable `Rgba8Unorm` field/volume viewport debug products have landed. Do not add future asset surfaces through new hard-coded toolbar, shortcut, menu, or panel paths.

Implementation targets:

- `apps/runenwerk_editor/src/shell/applied_editor_definition/catalogs.rs`
  - implemented for shortcut, command-binding, panel-registry, and tool-surface-registry replacement before mutation; invalid activation leaves the previous active catalog unchanged.
- `apps/runenwerk_editor/src/shell/toolbar_adapter.rs` and `domain/editor/editor_shell/src/composition/toolbar_definition.rs`
  - implemented for active menu and command-binding consumption when forming toolbar/menu view models, while preserving fallback checked-in fixtures.
- `apps/runenwerk_editor/src/runtime/systems/input_bridge.rs::dispatch_shortcuts`
  - implemented with active shortcut resolution through app-owned known commands, while preserving hard-coded shortcuts as fallback defaults until an active set is installed.
- `domain/editor/editor_shell/src/composition/build_editor_shell.rs`
  - implemented for app-supplied route-action mapping and active tool-surface registry ordering in switch/new-tab choices; active panel ordering is projected for future choices without mutating the existing workspace unless a workspace layout definition is applied.
- `apps/runenwerk_editor/src/shell/providers/mod.rs`
  - implemented first asset/import/field-product surfaces through app-owned providers while their availability is surfaced through active panel/tool-surface catalogs.
- `domain/asset/src/lib.rs`
  - added a new engine-agnostic asset domain crate.
- `domain/asset/src/id.rs`
  - define typed ids for assets, sources, imported artifacts, import jobs, and asset revisions.
- `domain/asset/src/kind.rs`
  - define asset kinds for scene, prefab, SDF graph, SDF brush/layer, field-world definition, world edit log, field material channels, formed field product, `world_sdf` chunk/page artifact, clipmap/brickmap product, material graph, material, procedural material, procedural texture, Texture2D, Texture3D/volume texture, gameplay graph, gameplay rule/trigger, gameplay ability, gameplay quest, gameplay ATR IR product, gameplay ECS lowering product, particle graph, particle emitter, physics config, animation clip, animation graph, procgen graph, UI layout, UI definition, graph, script, shader, theme, menu, shortcut, workspace definition, editor definition, and foreign mesh/reference source. Mesh/glTF kinds are interop/reference kinds, not the primary world substrate.
- `domain/asset/src/source.rs`
  - model source files, source hashes, source provenance, external source roots, authored SDF/field documents, and foreign source roots.
- `domain/asset/src/artifact.rs`
  - model imported artifacts, formed field product references, `world_sdf` payload references, gameplay ATR IR products, gameplay ECS query/event/schedule lowering products, source maps, cache keys, generated files, runtime package refs, and artifact validity.
- `domain/asset/src/dependency_graph.rs`
  - model dependencies and reverse dependencies for import invalidation and hot reload.
- `domain/asset/src/import_settings.rs`
  - model import settings for SDF graph/source formation, field-world layers, `world_sdf` brick/page/chunk product formation, material channel policy, procedural material/texturing, PBR parameter validation, triplanar mapping, Texture2D, Texture3D/volume texture products, gameplay graph compiler profile, ATR IR validation, ECS query/event/schedule lowering, SDF physics relation readiness, authority/network policy, particles, physics configs, animation clips/graphs, procgen graphs, shaders, UI definitions, scripts, authored RON documents, and Blender/glTF as foreign-source compatibility.
- `domain/asset/src/import_plan.rs`
  - builds deterministic import plans without executing external tools.
- `domain/asset/src/diagnostics.rs`
  - define stable asset pipeline diagnostic codes.
- `domain/asset/src/ratification.rs`
  - ratify imported, migrated, generated, and externally supplied asset candidates.
- `domain/world_sdf/src/product.rs`
  - define engine-agnostic descriptors for formed SDF chunk/page/brick products, product scope, scale band, source lineage, freshness, consumer class, and retention/rebuild policy.
- `domain/world_sdf/src/ratification.rs`
  - ratify formed `world_sdf` payload candidates before they become catalog-visible artifacts.
- `domain/world_ops/src/build_graph.rs` and `domain/world_ops/src/build_queue.rs`
  - remain the generic build primitives; `domain/world_ops/src/product_invalidation.rs` connects source/product changes to invalidation and build plans without owning artifact IO.
- `domain/editor/editor_persistence/src/project_file.rs`
  - added `ProjectFileV2` with asset catalog roots, startup document, source roots, field-product cache root, workspace profile defaults, and migration from `ProjectFileV1`.
- `apps/runenwerk_editor/src/persistence/files.rs::read_project_file`
  - loads and migrates V1 project files into V2 project DTOs.
- `DOMAIN_MAP.md` and `CRATES.md`
  - record the new asset domain ownership.

Validation:

- `cargo metadata --no-deps --format-version 1`
- `cargo test -p asset -p editor_persistence -p world_sdf -p world_ops`
- `cargo test -p editor_shell -p runenwerk_editor`
- `cargo test -p runenwerk_editor --test viewport_architecture_guards`
- `cargo test -p runenwerk_editor --test startup_render_smoke`
- `python3 tools/docs/validate_docs.py`
- `./quiet_editor_gate.sh`
- `./quiet_full_gate.sh`

### M5 - Runtime Preview, Data Hot Reload, And Restart Boundaries

Status: complete as of 2026-05-09 for the concrete external-runtime slice. External runtime preview process ownership is accepted in `docs-site/src/content/docs/adr/accepted/0007-external-runtime-preview-process.md`; implementation landed without pulling M6 domain work forward, and the hardening pass adds checked protocol/bootstrap handling plus real headless command/event lifecycle coverage. Closeout evidence is recorded in `docs-site/src/content/docs/reports/closeouts/m5-runtime-preview/closeout.md`.

Purpose: make ratified project content executable inside clear runtime preview and reload boundaries without moving asset semantics into the engine or app shell.

Implementation targets:

- `docs-site/src/content/docs/design/active/engine-game-runtime-editor-ecs-scripting-hot-reload-design.md`
  - keep the external runtime process, reload/restart boundary, ownership rules, and validation expectations aligned with implementation.
- `docs-site/src/content/docs/adr/accepted/0007-external-runtime-preview-process.md`
  - remain the governing decision for M5 preview, simulate, and play isolation.
- `domain/editor/editor_preview/src/lib.rs`
  - define `PreviewSessionId`, `PreviewMode`, `PreviewCommand`, `PreviewEvent`, `ReloadDecision`, `ReloadStatus`, `RuntimeProductRef`, runtime payload DTOs, checked preview payload metadata, and serialized bootstrap stdout format without depending on engine, net, or app crates.
- `net/engine_net/src/protocol/`
  - carry generic bidirectional typed payload commands so preview DTOs can cross transport without making network protocol enums editor-specific.
- `net/engine_net_quic/src/`
  - carry those generic payload messages over loopback QUIC.
- `apps/runenwerk_runtime_preview/src/main.rs`
  - own the child process, separate preview/play window, bootstrap connection output, and headless command/event mode that waits for explicit shutdown.
- `apps/runenwerk_editor/src/editor_app/state.rs`
  - separate edit, preview, simulate, and play session state.
- `apps/runenwerk_editor/src/runtime/preview_process/`
  - own editor-side child spawn/connect/heartbeat/shutdown, bounded pending command queueing, mode transition requests, event ingestion, and child lifecycle fallback.
- `apps/runenwerk_editor/src/runtime/app.rs`
  - compose runtime preview/play resources without making the generic engine editor-shaped.
- `domain/editor/editor_core/src/session.rs`
  - validate play/simulate mode transitions through scoped mode contracts.
- `domain/editor/editor_shell/src/workspace/profile.rs::default_workspace_profile_registry`
  - expose preview/simulate/play compatibility only through profiles and document kinds that support those modes.
- `apps/runenwerk_editor/src/asset_pipeline/catalog_runtime.rs`
  - route safe data reload streams and status changes from the existing catalog, import jobs, and field-product jobs into editor/runtime consumers while preserving prior valid products on failure;
  - classify current scene, asset, field-product, `world_sdf`, shader, and UI-definition families as live reload, preview-session restart, runtime-process restart, unsupported, failed-preserved, or rejected;
  - expose reload status lines through the existing Asset Browser, Import Inspector, and Field Product Viewer surfaces.
- `domain/world_sdf/src/storage.rs::SdfChunkStore`
  - receive ratified chunk/page/brick payload artifacts through runtime/app integration; do not let the editor write private storage internals directly.
- `engine/src/plugins/render/shader/hot_reload.rs::poll_shader_hot_reload`
  - integrate shader asset revisions and diagnostics with the asset reload stream.
- `engine/src/plugins/shared/reload.rs`
  - reuse reload status payloads for asset, field product, `world_sdf` payload, shader, scene, prefab, UI, graph, and script hot reload status.
- `apps/runenwerk_editor/src/runtime/preview_process/shader_status.rs::shader_reload_status_to_preview_status`
  - map engine shader reload payloads into `editor_preview::ReloadStatus` at the app boundary.
- `docs-site/src/content/docs/design/active/engine-game-runtime-editor-ecs-scripting-hot-reload-design.md`
  - keep the data hot reload versus Rust rebuild/restart boundary aligned with implementation.

Validation:

- changed asset/product revisions are classified as live-reloadable or restart-required through explicit status payloads;
- edit, preview, simulate, and play transitions fail closed for incompatible workspace/document/mode combinations;
- external runtime preview process spawn/connect/heartbeat/shutdown works in headless mode, including actual child-process bootstrap and exit;
- failed reloads preserve the prior valid runtime product;
- ratified field and `world_sdf` products can be previewed or loaded through app/engine integration without treating mesh/glTF as world truth;
- authored material graph, procgen, particle, physics, animation, gameplay graph, graph execution, and scripting families fail closed until their owning runtime/product contracts exist; formed material and texture product families may require preview-session restart until live-reload adapters exist;
- `cargo metadata --no-deps --format-version 1`;
- `cargo test -p editor_preview -p engine_net -p engine_net_quic`;
- `cargo test -p runenwerk_editor -p runenwerk_runtime_preview -p engine -p asset -p world_sdf`;
- `python3 tools/docs/validate_docs.py`;
- `./quiet_full_gate.sh`.

### M6 - Procedural Authoring Workspaces Beyond Scene Editing

Purpose: make the editor a multi-document procedural authoring environment.

Detailed feature slices, milestone gates, and remaining decisions for material graphs, procedural texturing, Texture3D, procgen, particles, physics, animation, and world processes are owned by `docs-site/src/content/docs/design/active/editor-procedural-content-and-simulation-workflow-plan.md`. Gameplay graph ATR IR and ECS lowering are owned by `docs-site/src/content/docs/design/active/gameplay-graph-atr-ir-and-ecs-lowering-design.md`.

The SDF-first execution program is a completed substrate, not a live activation gate. Remaining M6 work is activated by its current owning design/roadmap plus an owning GitHub issue, and product-domain work must consume the completed product-job, publication, query-snapshot, strict-consumer, render-selection, and derived-residency contracts instead of inventing private execution paths.

Current M6 focus:

- Treat M6.2C procgen bake, rollback, persistence, and runtime preview reload
  classification as complete on the accepted
  `docs-site/src/content/docs/domain/procgen/README.md` contract, the Phase 6A
  `domain/procgen` crate, the Phase 6B editor/runtime overlay proof, the Phase
  6C concrete CPU field-preview proof, and the Phase 6D bake closeout.
- Further procgen worker pools, renderer rebuilds, GPU upload, caves, stamps,
  scatter, and package-level persistent cache sidecars are outside the completed
  Phase 6D program; any such work requires current owning design/issue authority
  and must continue through the established product substrate.
- Rendered SDF/GPU overlays and material/SDF preview handoff are separate later
  product slices; their activation belongs to their current owning roadmap/design
  and GitHub issue rather than this completed execution program.

Other gated M6 tracks:

- Editor Tool Suite Registry And Workbench Host: implemented for the surviving
  editor-local provider, capability, registration, and workbench-integration
  contracts. Future graph/product tools consume the
  [implemented Tool Suite design](../../design/implemented/editor-tool-suite-registry-and-workbench-host-design.md);
  generic structural composition identity and semantics remain governed by
  [accepted ADR 0013](../../adr/accepted/0013-app-neutral-ui-composition-clean-cutover.md).
- M6.3 gameplay graph, M6.4 particles, M6.5 SDF physics, M6.6 animation, and
  M6.7 world-process work are not activated by the completed SDF program; each
  requires current owning contracts and GitHub activation while consuming the
  established execution substrate.
- M6.1 intentionally leaves rendered material preview adapters, Texture3D GPU
  upload/runtime adapters, broad document persistence/import UX, and full PBR
  preview capability matrices for later P3/P9 work.

Remaining sub-milestones:

- M6.2 Procgen product track: deterministic generator documents, seed/scope contracts, procgen-owned prototype/candidate/reservation/instance-plan metadata, bounded preview, world-operation lowering, bake/rollback, explanation data, and changed-region diagnostics on top of the accepted procgen domain contract and completed SDF-first execution substrate.
- M6.3 Gameplay graph product track: prerequisite gameplay event/action/state/quest contracts, Action/Trigger/Rule IR, compiler passes, ECS query/event/schedule lowering, SDF physics `HIT` relation readiness, authority diagnostics, and source maps on the completed execution substrate.
- M6.4 Particles product track: deterministic emitter documents, SDF/field spawn and collision coupling, preview products, count/bounds diagnostics, and backend-neutral formed products on the completed execution substrate.
- M6.5 SDF physics product track: collision product descriptors, rigid/kinematic/character body contracts, physics material links, field-query readiness, and debug surfaces on the completed execution substrate.
- M6.6 Animation product track: clips, curves, timeline, state/blend graphs, procedural motion hooks, source maps, and preview diagnostics on the completed execution substrate.
- M6.7 World-process product track: bounded material-transport previews, timescale/solver-budget contracts, bake/commit to governed `world_ops`, rollback, and product freshness diagnostics on the completed execution substrate.

Finished M6 baselines:

- M6.0 shared workspace substrate is complete in the current worktree for workspace profiles, tool-surface/panel vocabulary, persisted layout support, runtime/debug/diagnostic surface routing, and fail-closed provider diagnostics.
- M6.1 material/texture descriptor-first closeout is complete: `domain/material_graph` and `domain/texture` exist with accepted domain docs, initial ratifiers, descriptors, source/lineage metadata, deterministic lowering, material/texture artifact payload kinds, import settings, runtime product kinds, reload classification for formed material/texture products, and concrete descriptor-first material/texture provider surfaces. Closeout evidence is recorded in `docs-site/src/content/docs/reports/closeouts/m6-material-texture-descriptor-preview/closeout.md`.
- P1/SDF modeling core is complete for the CPU/editor-surface boundary: command-backed operation documents, source-backed SDF graph documents, Add/Subtract/Intersect/SmoothAdd/SmoothSubtract/SmoothIntersect lowering to `world_ops` records, invalidation dirty-reason classification, app-held commit logs, deterministic CPU field-preview products, and concrete field-layer/SDF graph/field-product surfaces. Closeout evidence is recorded in `docs-site/src/content/docs/reports/closeouts/p1-sdf-modeling-core/closeout.md`.

Implementation targets:

- `domain/editor/editor_shell/src/workspace/profile.rs::default_workspace_profile_registry`
  - add `Field World`, `SDF Modeling`, `Materials`, `Textures`, `Procedural Generation`, `Gameplay`, `Particles`, `Physics`, `Animation`, `Layout`, `UI`, `Graphs`, `Scripting`, `Simulation`, `Debug`, and `Editor Design` workspace profiles.
- `domain/editor/editor_core/src/session.rs`
  - validate mode activation against `(workspace_profile, document_kind)`.
- `domain/material_graph/src/lib.rs`
  - own material graph semantics, PBR parameter contracts, procedural texturing nodes, triplanar mapping semantics, ratification, lowering, and formed material products.
- `domain/texture/src/lib.rs`
  - own Texture2D, Texture3D/volume texture descriptors, color space, sampler, compression, generated texture cache metadata, and texture diagnostics.
- future `domain/procgen/src/lib.rs`
  - own procedural generation documents, seed contracts, generator graphs, and lowering to bounded world operation windows.
- future `domain/gameplay/events/src/lib.rs`
  - own gameplay event ids, payload schemas, channel descriptors, authority class, and source-map subjects used by gameplay graph lowering.
- future `domain/gameplay/actions/src/lib.rs`
  - own action request, action plan, action result, and effect vocabulary before gameplay graph transforms can request actions.
- future `domain/gameplay/state/src/lib.rs`
  - own state machine, state membership, transition, and condition contracts before gameplay graph state transforms lower to runtime products.
- future `domain/gameplay/quests/src/lib.rs`
  - own quest, objective, progress, and completion contracts before gameplay graph quest transforms lower to runtime products.
- future `domain/gameplay_graph/src/lib.rs`
  - own gameplay graph semantics, Action/Trigger/Rule IR, compiler passes, SDF physics relation binding, and formed ECS query/event/schedule/runtime products while depending on narrower gameplay event/action/state/quest contracts for their semantics.
- future `domain/particles/src/lib.rs`
  - own emitter definitions, particle graph semantics, SDF/field coupling contracts, and formed particle products.
- future `domain/physics/src/lib.rs`
  - own body, collider, constraint, trigger, physics material, collision product, and readiness contracts.
- future `domain/animation/src/lib.rs`
  - own clips, curves, state machines, blend trees, procedural motion, root motion, animation events, and binding diagnostics.
- `apps/runenwerk_editor/src/shell/providers/ui_hierarchy.rs::UiHierarchyProvider`
  - show and edit UI document hierarchy.
- `apps/runenwerk_editor/src/shell/providers/ui_canvas.rs::UiCanvasProvider`
  - preview retained UI layout documents through the existing UI substrate.
- `apps/runenwerk_editor/src/shell/providers/graph_canvas.rs::GraphCanvasProvider`
  - host graph documents using `domain/graph`.
- `apps/runenwerk_editor/src/shell/providers/field_layer_stack.rs::FieldLayerStackProvider`
  - show and edit authored field-world layers, SDF operation ordering, and material channel bindings through commands.
- `apps/runenwerk_editor/src/shell/providers/sdf_graph_canvas.rs::SdfGraphCanvasProvider`
  - host SDF graph documents while using standalone RunenSDF for reusable field semantics and `domain/graph` only for graph substrate behavior.
- `apps/runenwerk_editor/src/shell/providers/material_graph_canvas.rs::MaterialGraphCanvasProvider`
  - edit material graphs without making graph canvas state material truth.
- `apps/runenwerk_editor/src/shell/providers/material_inspector.rs::MaterialInspectorProvider`
  - edit PBR parameters, procedural node settings, SDF/field inputs, and material channel bindings.
- `apps/runenwerk_editor/src/shell/providers/material_preview.rs::MaterialPreviewProvider`
  - preview material products on SDF primitives, field products, and reference meshes.
- `apps/runenwerk_editor/src/shell/providers/procgen_graph_canvas.rs::ProcgenGraphCanvasProvider`
  - edit procedural generation graphs that lower to deterministic world operation windows.
- `apps/runenwerk_editor/src/shell/providers/procgen_preview.rs::ProcgenPreviewProvider`
  - preview, bake, and rollback bounded generated worlds.
- `apps/runenwerk_editor/src/shell/providers/gameplay_graph_canvas.rs::GameplayGraphCanvasProvider`
  - edit constrained gameplay graphs without making graph canvas state runtime authority.
- `apps/runenwerk_editor/src/shell/providers/gameplay_compiler_diagnostics.rs::GameplayCompilerDiagnosticsProvider`
  - inspect ATR IR, ECS query/event/schedule lowering, SDF physics relation readiness, source maps, and authority diagnostics.
- `apps/runenwerk_editor/src/shell/providers/particle_graph_canvas.rs::ParticleGraphCanvasProvider`
  - edit emitters and particle graphs.
- `apps/runenwerk_editor/src/shell/providers/particle_preview.rs::ParticlePreviewProvider`
  - preview particles, field collision, counts, bounds, and simulation diagnostics.
- `apps/runenwerk_editor/src/shell/providers/physics_authoring.rs::PhysicsAuthoringProvider`
  - edit bodies, colliders, materials, layers, masks, constraints, and triggers.
- `apps/runenwerk_editor/src/shell/providers/physics_debug.rs::PhysicsDebugProvider`
  - inspect contacts, sweeps, activation, constraints, and missing `world_sdf` readiness.
- `apps/runenwerk_editor/src/shell/providers/timeline.rs::TimelineProvider`
  - edit clips, events, and playback ranges.
- `apps/runenwerk_editor/src/shell/providers/curve_editor.rs::CurveEditorProvider`
  - edit typed animation/procedural curves.
- `apps/runenwerk_editor/src/shell/providers/animation_graph_canvas.rs::AnimationGraphCanvasProvider`
  - edit state machines, blend trees, and procedural motion graphs.
- `apps/runenwerk_editor/src/shell/providers/simulation_preview.rs::SimulationPreviewProvider`
  - preview and bake material transport, erosion, snow, water, sediment, and other world-process effects.
- `apps/runenwerk_editor/src/shell/providers/script_editor.rs::ScriptEditorProvider`
  - edit script assets while keeping scripting language-neutral at the domain boundary.
- `apps/runenwerk_editor/src/shell/providers/diagnostics.rs::DiagnosticsProvider`
  - show project, import, runtime, and validation diagnostics.
- `apps/runenwerk_editor/src/shell/providers/runtime_debug.rs::RuntimeDebugProvider`
  - inspect ECS/runtime state without making authored documents ECS entities.

Validation:

- M6.0 through M6.7 each have explicit closeout notes or checklist entries before M6 is marked complete;
- one document tab per implemented document kind opens, saves, closes, reopens, and reports dirty state correctly;
- procedural graph lowering never depends on editor graph canvas state;
- gameplay graph lowering never depends on editor graph canvas state and forms ECS query/event/schedule products with source maps;
- generated products keep source lineage, seed, version, and diagnostics;
- unsupported workspace/document/surface combinations fail closed with provider diagnostics;
- no provider bypasses command or ratification boundaries.

### M7 - Scripting And Runtime Gameplay Bridge

Purpose: add the language-neutral scripting boundary and runtime gameplay bridge after M5 has established preview/reload boundaries and M6 has formed procedural/gameplay product contracts.

Implementation targets:

- future `domain/script_runtime/src/lib.rs`
  - define language-neutral script asset/runtime command contracts.
- future `adapters/script_rhai/src/lib.rs`
  - implement the first Rhai adapter without leaking Rhai types into domain contracts.
- future `engine/src/plugins/gameplay_graph/`
  - instantiate formed gameplay graph products into ECS query descriptors, event subscriptions, fixed executor descriptors, schedule edges, runtime registries, source maps, and authority/network metadata without interpreting authored graph nodes.
- future `domain/gameplay_graph/src/lowering/sdf_physics.rs`
  - bind `RELATE HIT` and other SDF/field relations to `domain/world_sdf`, future `domain/physics`, or engine collision/query products through readiness diagnostics.

Validation:

- gameplay graph products fail closed when ECS query/event/schedule lowering, SDF physics readiness, source maps, or authority metadata cannot be preserved;
- structural Rust changes require registry refresh and play-session restart;
- play/simulate does not mutate authored documents without explicit commands;
- runtime execution consumes formed gameplay graph products, not live authored graph traversal;
- script actions request domain/runtime commands instead of directly mutating ECS internals.

### M8 - UI Authoring Packaging, Extensibility, And Externalization

Purpose: harden the M3.6 self-authoring system for specialized editor packaging, external definition exchange, and long-lived project migration after the main asset/procedural/runtime feature tracks exist.

Implementation targets:

- `domain/editor/editor_definition/src/package.rs`
  - define exportable editor-definition package manifests, dependency references, compatibility ranges, and migration metadata.
- `domain/ui/ui_definition/src/package.rs`
  - define exportable UI-template/theme package manifests without app IO or runtime execution.
- `apps/runenwerk_editor/src/persistence/editor_definition_package.rs`
  - import/export packages through app-owned file IO and project policy.
- `apps/runenwerk_editor/src/shell/providers/editor_package_publisher.rs::EditorPackagePublisherProvider`
  - validate and publish specialized editor packages from authored definitions.
- `apps/runenwerk_editor/src/shell/providers/definition_migration_report.rs::DefinitionMigrationReportProvider`
  - show cross-project migration reports and unresolved compatibility issues.
- `docs-site/src/content/docs/design/deferred/ui-model-multiple-execution-strategies-design.md`
  - keep the M0 UI execution-strategy decision visible. M8 must not introduce compiled-reactive or ECS-driven UI execution for the first time; any future promotion requires a separate active design or accepted ADR, guard tests, and roadmap update.

Validation:

- package import/export preserves authored ids, source maps, diagnostics, migrations, and compatibility metadata;
- published specialized editor packages cannot activate invalid UI/editor definitions;
- package publishing follows the retained UI execution strategy closed in M0 and does not choose compiled-reactive or ECS-driven UI execution for the first time;
- active runtime/session-only ids are never exported as authored ids.

### M9 - Hardening, Release Readiness, And Full Gate

Purpose: close the editor as a product-quality tool.

Implementation targets:

- `docs-site/src/content/docs/apps/runenwerk-editor/current-architecture.md`
  - update current architecture after each closed milestone.
- `docs-site/src/content/docs/workspace/planning/roadmap.md`
  - update only when editor work changes durable workspace sequence or dependency direction; GitHub issues and the Engineering Portfolio own live priority, activation, blockers, and status.
- `apps/runenwerk_editor/tests/`
  - add smoke tests for project open/save, SDF/field asset import, field-product formation, procedural material/texturing, Texture3D/volume inspection, procgen preview/bake, gameplay graph ATR IR lowering to ECS query/event/schedule products, SDF physics `HIT` relation diagnostics, particles, physics authoring/debug, animation preview, world-process preview/bake, document tabs, docking, scene authoring, UI authoring, runtime preview, and self-authoring.
- `domain/editor/editor_shell/src/tests.rs`
  - keep projection golden tests authoritative for workspace structure and routing.
- `domain/ui/ui_runtime/src/output/build_ui_frame.rs`
  - keep UI frame snapshot expectations stable where render-data output is contractual.

Full validation:

- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- `python3 tools/docs/validate_docs.py`
- `./quiet_full_gate.sh` for milestone closeout and final release-readiness validation.

## Completion Criteria

The roadmap is complete when a user can:

1. create or open a project;
2. import source assets and see catalog diagnostics;
3. create and edit SDF scenes, field-world layers, prefabs, UI layouts, graphs, scripts, materials, procedural textures, Texture3D assets, gameplay graphs, gameplay rules/triggers/abilities/quests, particles, physics configs, animation clips/graphs, procedural generators, menus, themes, shortcuts, and workspace/editor definitions;
4. use document tabs, workspace switching, scoped modes, docking, editor type switching, area tabs, plus/new-tab controls, and saved layouts;
5. author a 3D SDF scene with create/delete/duplicate, hierarchy operations, SDF primitive/brush actions, translate/rotate/scale, snapping, inspector/component editing, undo/redo, and persistence;
6. preview imported assets, SDF fields, `world_sdf` chunk/page/brick products, procedural PBR materials, triplanar/Texture3D products, gameplay graph lowering products, ECS query/event/schedule source maps, particles, physics debug products, animation products, procedural generation outputs, and authored documents in appropriate tool surfaces;
7. run play/simulate/preview with safe data hot reload and clear restart boundaries for structural Rust changes;
8. validate, migrate, import, export, and publish authored definitions, including gameplay graph ATR IR products that lower into deterministic runtime products;
9. recover from failed imports, invalid documents, bad definitions, and stale runtime previews without corrupting project state;
10. pass the full repository validation gate.

## Rule

Do not add new editor/UI feature paths by bypassing these ownership seams. If a needed feature does not fit, update the owning design first, then update this roadmap, then implement through the appropriate domain, engine, app, or tool boundary.

## Finished Roadmap Baselines

The detailed milestone sections above keep historical implementation targets in
place for traceability. Use this bottom section as the completed-work summary:

- editor MVP acceptance, M1 structural seams, M2 shell seams, and M3 scene
  authoring are complete;
- M3.5 UI definition formation, M3.6 self-authoring, and M3.7 viewport
  expression architecture are complete;
- M4A-M4I integrated UI/editor/asset foundations are complete, including active
  command/menu/shortcut/catalog consumption, `domain/asset`, `ProjectFileV2`,
  app-owned import and field-product jobs, first asset surfaces, and viewport
  debug products;
- M5 runtime preview, project-owned reload classification, `world_sdf` runtime
  intake, and restart boundaries are complete for the existing product
  families;
- M6.0 shared workspace substrate, M6.1 material/texture descriptor contracts,
  P1 SDF modeling core, and Batch 1 SDF-first product contract alignment are
  complete;
- remaining M6.2+ product-domain work must consume the completed SDF-first
  execution substrate; current activation, priority, and blockers belong to the
  owning roadmap/design and GitHub issue.

<!-- BEGIN RUNENWERK:UI_COMPONENT_PLATFORM:workbench-adoption -->
## Component Platform workbench adoption

Workbench surfaces consume story-proven reusable controls and surfaces. Toolbar uses Button/Toggle/Navigation. Inspector uses Label/Input/NumericInput/Picker. Outliner uses Tree/List. Entity table uses Table/List/Search/Sort. Menus consume Overlay/Menu/Popup contracts. Material graph consumes PortGraphCanvas. Timeline surfaces consume TrackSurface/Timeline. Skill/progression views consume NodeCanvas/ProgressionTreeView where applicable.

Workbench does not own reusable control semantics; it maps host intent proposals to editor/domain commands.
<!-- END RUNENWERK:UI_COMPONENT_PLATFORM:workbench-adoption -->
