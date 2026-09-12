---
title: Editor Asset Pipeline and Content Workflow Design
description: Active architecture design for project assets, imports, artifact cache, asset diagnostics, previews, and editor/runtime reload boundaries.
status: active
owner: editor
layer: domain
canonical: true
last_reviewed: 2026-09-12
related_designs:
  - ../accepted/sdf-first-field-world-platform-design.md
  - ../implemented/editor-rendered-world-and-multi-entity-viewport-design.md
  - ../implemented/field-visualizer-product-workflow-design.md
  - ./material-lab-and-material-preview-design.md
  - ./sdf-prefab-composition-system-design.md
  - ./editor-procedural-content-and-simulation-workflow-plan.md
  - ./gameplay-graph-atr-ir-and-ecs-lowering-design.md
  - ../implemented/editor-workspace-document-mode-panel-architecture.md
  - ./editor-ui-workspace-tool-surface-architecture.md
  - ../implemented/editor-self-authoring-and-final-ui-design.md
  - ./engine-game-runtime-editor-ecs-scripting-hot-reload-design.md
related_roadmaps:
  - ../../apps/runenwerk-editor/roadmap.md
related_reports:
  - ../../reports/audits/editor-ui-priority-code-audit-2026-05-05.md
related:
  - ../../domain/world-sdf/README.md
  - ../../domain/world-ops/README.md
  - ../../domain/spatial/README.md
  - ../../domain/chunking/README.md
---

# Editor Asset Pipeline and Content Workflow Design

## Purpose

Define the architecture that should turn Runenwerk's current loose content files into a project-owned asset pipeline.

The target pipeline must be SDF/field-world first. It must support authored field-world content, SDF graphs and brushes, world-operation sources, deterministic field-product formation, generated `world_sdf` artifacts, dependency tracking, previews, diagnostics, hot reload, and runtime formation without making the editor app, ECS world, renderer, mesh importer, or scene manifest scanner the semantic owner of all content.

Blender, glTF, GLB, and meshes may exist as foreign-source/reference import paths. They are not the canonical representation of the Runenwerk world unless a future owning design explicitly narrows and ratifies that role.

## Current Repository Truth

Implemented today:

- Scene persistence uses `SceneFileV2` in `domain/editor/editor_persistence/src/scene_file.rs`.
- Scene load follows migration, normalization, formation, and apply in `apps/runenwerk_editor/src/persistence/files.rs::load_scene_file_into_runtime_classified`.
- Project persistence has `ProjectFileV2` in `domain/editor/editor_persistence/src/project_file.rs`, with migration from V1 scene-list projects.
- Reusable SDF primitives, composition, transforms, sampling, gradients/normals, and CPU query helpers are owned by standalone `dornglut/runen-sdf`.
- World edit operation records, dirty-region tracking, build queues, and invalidation contracts exist in `domain/world_ops/src/operation_log.rs`, `domain/world_ops/src/dirty.rs`, `domain/world_ops/src/build_queue.rs`, and `domain/world_ops/src/region_invalidation.rs`.
- Generic field-product invalidation and build helper contracts exist in `domain/world_ops/src/product_invalidation.rs`.
- World-scale SDF chunk/page payload records and collision query contracts exist in `domain/world_sdf/src/storage.rs::SdfChunkStore` and `domain/world_sdf/src/collision.rs::CollisionQueryService`.
- Field-product descriptors and ratification live in `domain/world_sdf/src/product.rs` and `domain/world_sdf/src/ratification.rs`.
- `domain/asset` owns IO-free asset ids, source and artifact descriptors, asset taxonomy, deterministic import plans, dependency graph contracts, diagnostics, catalog validation, and asset ratification.
- `apps/runenwerk_editor/src/asset_pipeline/` owns open-project catalog runtime state, app-owned import jobs, and field-product jobs.
- Asset Browser, Import Inspector, Field Product Viewer, and SDF Brush Browser provider surfaces exist under `apps/runenwerk_editor/src/shell/providers/` and are reachable through active panel/tool-surface catalogs.
- Spatial/chunking vocabulary exists in `domain/spatial/src/` and `domain/chunking/src/`.
- Scene manifest discovery in `engine/src/plugins/scene/manifest/catalog.rs::load_scene_manifest_descriptors` can read asset-catalog-backed scene descriptors while preserving loose RON manifest fallback during migration.
- Render import descriptors in `engine/src/plugins/render/resource/import.rs` describe imported render resources only.
- Shader reload helpers exist in `engine/src/plugins/render/shader/hot_reload.rs` and `engine/src/plugins/shared/reload.rs`.
- `assets/editor/config.ron` contains a Blender executable path, `assets/models` contains `.blend` and `.glb` files, and `tools/assets/blender_export.py::main` provides the configured `.blend -> .glb` foreign-reference export helper. These remain compatibility/reference paths, not the canonical world substrate.
- `apps/runenwerk_editor/src/runtime/viewport/producer_field.rs` and `apps/runenwerk_editor/src/runtime/viewport/producer_volume.rs` publish displayable `Rgba8Unorm` debug preview products for the first field/volume producer breadth.

Current state after the 2026-05-16 WR-020 repair closeout:

- Source-backed Asset Core contracts are implemented in `domain/asset`: V1 catalog source roots with backward-compatible deserialization, strict project-relative source/artifact/catalog paths, project catalog descriptors, source descriptors with importer choice, artifact-kind-aware deterministic import plans, separate source-kind and artifact-kind import compatibility, dependency graph contracts, diagnostics, composed catalog ratification, and checked prior-valid artifact preservation for SDF graph, field product, material graph/material, UI definition, and prefab descriptor families.
- Editor adapter work remains deferred to `WR-026`: project catalog load/save, import job orchestration, diagnostics surfacing, and prior-valid preservation UI must consume the domain contracts instead of becoming semantic asset truth.
- Prefabs get source/catalog/design identity through `docs-site/src/content/docs/design/active/sdf-prefab-composition-system-design.md`; runtime prefab instancing remains V2-gated behind rendered-world V1 and source-backed asset identity.
- Material Lab sources are explicit material graph documents from `docs-site/src/content/docs/design/active/material-lab-and-material-preview-design.md`; canvas state must remain a projection.
- no project-owned data hot reload stream or runtime preview application of changed asset/product revisions;
- no persisted asset catalog file load/save loop or project-owned cache garbage collection workflow;
- no complete `world_sdf` artifact/cache bridge into runtime chunk/page stores;
- no dependency graph visualizer, full reimport controls, artifact preview, dirty asset summary, or brick/clipmap diagnostic surface;
- no concrete material graph authoring provider, rendered material preview, Texture3D runtime preview/upload adapter, gameplay graph, particle, physics, animation, simulation-process, or procgen domain/runtime implementation;
- no unified runtime loader mapping from asset artifact ids to all future field, scene, render, UI, graph, script, gameplay, material, procgen, particle, physics, and animation consumers;
- no raw scalar/vector renderer texture-format expansion beyond the first displayable debug preview products;
- no marketplace/package workflow.

## Ownership Rules

### `domain/asset`

The asset domain crate owns engine-agnostic asset truth:

- asset identity;
- asset kind taxonomy;
- source file descriptors;
- imported artifact descriptors;
- dependency graph;
- import settings;
- deterministic import plans;
- asset diagnostics;
- asset ratification;
- catalog validation.

It must not execute external tools, read arbitrary host files, allocate GPU resources, own app UI, own SDF field math, own world edit invalidation semantics, or own `world_sdf` payload storage.

### Standalone RunenSDF

`dornglut/runen-sdf` owns analytic SDF primitives, composition, transforms, sampling, gradients/normals, and CPU query behavior. The asset pipeline may reference SDF-authored sources and formed products, but it must not redefine reusable field semantics or duplicate the framework source.

### `domain/world_ops`

Owns world edit operation records, dirty-region tracking, build queues, invalidation journals, replay windows, and replication deltas for chunked world data. Asset/source changes that affect field products must flow through these invalidation and build contracts.

### `domain/world_sdf`

Owns chunk/page/brick SDF payload records, hierarchy summaries, cave summaries, and collision query contracts. The asset pipeline may catalog artifacts that contain `world_sdf` payloads, but the payload shape and readiness semantics belong here.

### `domain/spatial` and `domain/chunking`

Own world coordinate, chunk, region, clipmap, and desired residency vocabulary. Asset catalogs may name spatial scopes and product coverage; they do not own coordinate meaning or residency policy.

### `domain/editor/editor_core`

Owns document identity, document kinds, active document switching, dirty state, and workspace/document compatibility.

### `domain/editor/editor_persistence`

Owns versioned editor-facing project and authored document DTOs where those formats are editor-specific. It may depend on domain asset DTOs, but app-local paths and host IO stay outside this crate.

### `engine`

Owns runtime loading, render integration, shader reload integration, and scene/template/runtime formation seams. Engine systems consume formed asset products. They do not own editor project catalog policy.

### `apps/runenwerk_editor`

Owns host IO, import job execution, external process policy, asset browser UI, import diagnostics UI, preview instantiation, project-local cache paths, and integration with engine/runtime resources.

### `tools/assets`

Owns reusable helper scripts for external importers such as Blender export. These scripts are tools, not domain authority.

## Core Pipeline

Use this SDF/field-world-first model:

```text
Source file
  -> AssetSourceDescriptor
  -> ImportSettings
  -> ImportPlan
  -> ImportJob or FieldProductJob execution
  -> ImportedArtifact or FieldProductCandidate
  -> Asset and owning-domain ratification
  -> Catalog update
  -> Formed field/runtime/editor product
  -> Preview or runtime load
```

### Source

Source is the authoring input. Examples:

- `.blend`;
- `.glb`;
- SDF graph documents;
- SDF brush/layer documents;
- field-world definition documents;
- world edit operation logs;
- material graph documents;
- procedural texture and PBR material documents;
- Texture2D and Texture3D/volume texture sources;
- particle emitter and particle graph documents;
- physics config documents;
- animation clip, curve, timeline, and animation graph documents;
- procedural generation graph documents;
- gameplay graph, gameplay rule/trigger, ability, quest, and event schema documents;
- `.wgsl`;
- `.ron` scene/UI/graph/material/theme/menu documents;
- scripts;
- textures.

Source descriptors include stable asset id, path, source hash, kind, provenance, and importer choice.

### Import Settings

Import settings describe how a source should be turned into artifacts.

Examples:

- SDF graph formation options;
- SDF brush/layer blend and operation policy;
- world SDF brick/page/chunk formation settings;
- material channel packing and sampling policy;
- PBR parameter validation policy;
- triplanar mapping coordinate and blend policy;
- procedural texture generation and cache policy;
- Texture3D/volume texture dimension, sampler, color-space, and compression policy;
- particle emitter and simulation settings;
- physics collider/material/layer settings;
- animation clip, curve, state machine, and binding settings;
- procedural generation seed, scope, and bake policy;
- gameplay graph compiler profile, ATR IR validation policy, ECS query/event/schedule lowering target, SDF physics relation readiness policy, and authority/network policy;
- field-product scale band and coverage policy;
- Blender scene export options for foreign-reference imports;
- glTF mesh/material import options for foreign-reference imports;
- texture color-space and compression policy;
- shader stage and validation profile;
- script language adapter;
- UI/layout target profile.

### Import Plan

An import plan is deterministic and domain-owned. It says what should happen, but it does not execute host IO.

The plan should include:

- source asset id;
- source revision/hash;
- importer id;
- expected artifact ids;
- dependency inputs;
- cache key;
- validation requirements;
- expected diagnostics.

### Import Job And Field Product Job

An import job is app/tool execution for source files and generated artifacts.

`apps/runenwerk_editor/src/asset_pipeline/import_jobs.rs::run_import_job` executes a domain `ImportPlan`, invokes configured external importer tools declared by that plan, collects outputs, and returns an imported candidate plus diagnostics.

A field product job is app/engine-owned execution for SDF/field-world formation.

`apps/runenwerk_editor/src/asset_pipeline/field_product_jobs.rs::run_field_product_job` executes deterministic formation plans, uses `domain/world_ops` invalidation/build contracts, and returns candidates that must be ratified by the asset domain plus the owning field/world domain before catalog publication.

### Artifact

Artifacts are generated or accepted runtime/editor-ready files. They are not the source of truth when a source asset owns them.

Examples:

- formed `world_sdf` chunk/page/brick payloads;
- field-product packages with distance, material channel, occupancy/support, freshness, and provenance metadata;
- SDF preview products for editor surfaces;
- formed material graph products;
- generated procedural texture products;
- Texture3D/volume texture products;
- particle simulation/render products;
- physics collision/readiness/debug products;
- animation preview/runtime products;
- procedural generation operation-window products;
- gameplay graph formed products, ATR IR products, ECS query/event/schedule descriptors, authority/network descriptors, and source maps;
- `.glb` generated from `.blend` for foreign-reference import;
- validated shader module metadata;
- normalized scene payload;
- texture mip/compression outputs;
- formed UI preview package.

### Ratification

Imported candidates must be ratified before they update the catalog.

Ratification checks:

- source hash matches the import plan;
- generated artifact path is inside the project cache/artifact root;
- artifact kind matches requested asset kind;
- required dependencies are present;
- diagnostics are attached;
- stale artifact replacement is explicit;
- failed imports preserve the last valid artifact.
- field-product candidates declare spatial scope, scale band, source lineage, freshness, consumer class, and rebuild policy;
- `world_sdf` payload candidates satisfy `domain/world_sdf` readiness and storage invariants.

## Asset Kinds

Initial asset kinds:

- scene;
- prefab;
- SDF graph;
- SDF brush/layer;
- field-world definition;
- world edit log;
- field material channel set;
- formed field product;
- `world_sdf` chunk/page artifact;
- clipmap/brickmap product;
- material graph;
- procedural material asset handled through material graph/product contracts;
- PBR material;
- procedural texture;
- Texture2D;
- Texture3D/volume texture;
- particle graph;
- particle emitter;
- physics config;
- physics collision product;
- animation clip;
- animation graph;
- animation curve/timeline;
- procedural generation graph;
- gameplay graph;
- gameplay rule/trigger;
- gameplay ability;
- gameplay quest;
- gameplay ATR IR product;
- gameplay ECS lowering product;
- UI layout;
- graph;
- script;
- material;
- foreign mesh/reference source;
- foreign mesh/reference artifact;
- texture;
- shader;
- theme;
- menu;
- shortcut;
- workspace definition;
- editor definition;
- diagnostics capture.

The taxonomy belongs in `domain/asset/src/kind.rs`. Document kind compatibility belongs in `domain/editor/editor_core/src/document.rs`.

Theme, UI layout, workspace definition, menu, shortcut, command binding, panel
registry, tool-surface definition, and editor definition assets should feed the
existing `domain/editor/editor_definition` and `domain/ui/ui_definition`
semantics. The asset pipeline owns source identity, cataloging,
dependencies, import/cache policy, and hot-reload delivery for those files; it
does not replace the editor-definition or UI-definition crates as the semantic
owners of validation, normalization, formation, or live activation contracts.

Mesh/glTF kinds exist for compatibility, preview, and reference workflows. SDF and field-world kinds are the primary world-authoring content path.

## Project File Evolution

`ProjectFileV1` was scene-list oriented. `ProjectFileV2` now exists and includes:

- project id and name;
- asset source roots;
- artifact/cache root;
- field-product cache root;
- catalog file path;
- startup document id;
- default workspace profile id;
- open document restoration policy;
- import profile defaults;
- compatibility version.

Migration from V1 converts scene entries into project document entries, source roots, and startup document selection while preserving compatibility with old project files.

## Runtime And Hot Reload Boundary

Data/assets may hot reload when ratified and safe.

Good live reload targets:

- scenes;
- SDF graphs and brush/layer documents;
- field-world definitions;
- formed field products;
- `world_sdf` chunk/page payload revisions where consumers can safely swap generation handles;
- material channel products;
- material graphs and procedural texture products where formed products can be swapped safely;
- Texture3D/volume texture products where consumers can refresh bound resource generations safely;
- particle emitters and particle graphs when simulation can restart or retain state explicitly;
- physics config and collision products when the preview/session declares a safe restart or rebuild boundary;
- animation clips, curves, and graphs when playback can resample from a known time/revision;
- gameplay graphs and ATR IR products when ECS query/event/schedule products can be swapped at explicit preview/play boundaries with source maps and authority metadata preserved;
- prefabs;
- UI layouts;
- graph/material definitions;
- shaders;
- scripts where the adapter supports it;
- import settings;
- textures and generated foreign mesh/reference artifacts when consumers can refresh safely.

Usually restart or preview-session reload:

- new Rust component types;
- component memory layout changes;
- new Rust systems;
- plugin graph changes;
- renderer backend changes;
- ECS internals;
- network schema changes.

This boundary must stay aligned with `docs-site/src/content/docs/design/active/engine-game-runtime-editor-ecs-scripting-hot-reload-design.md`.

## UI Surfaces

Required editor surfaces:

- Asset Browser;
- Import Inspector;
- Dependency Graph;
- Import Diagnostics;
- Artifact Preview;
- Field Product Viewer;
- SDF Brush Browser;
- World SDF Brick/Page/Clipmap Diagnostics;
- Shader/Material Preview;
- Material Graph Canvas;
- Material Inspector;
- Texture Viewer;
- Texture3D/Volume Viewer;
- Particle Graph Canvas;
- Particle Preview;
- Physics Authoring;
- Physics Debug;
- Timeline;
- Curve Editor;
- Animation Graph Canvas;
- Procgen Graph Canvas;
- Procgen Preview;
- Simulation Preview;
- Mesh Preview for foreign-reference assets;
- Dirty Asset Summary.

These are tool surfaces. They should use provider routing from `domain/editor/editor_shell/src/surface_provider.rs` and app providers under `apps/runenwerk_editor/src/shell/providers/`.

## Negative Doctrine

Do not:

- treat `engine/src/plugins/scene/manifest/catalog.rs` as the general asset catalog;
- treat `engine/src/plugins/render/resource/import.rs` as an asset pipeline;
- treat mesh, glTF, or GLB as the canonical world representation;
- route field-world changes around `domain/world_ops` invalidation and build contracts;
- let the asset catalog define SDF sampling, field query semantics, or `world_sdf` payload layout;
- persist active runtime/session ids as asset ids;
- let ECS own authored asset documents;
- let the editor app own asset invariants that belong in `domain/asset`;
- let failed import jobs silently replace valid artifacts;
- make `.blend` files runtime inputs;
- make source paths the only durable identity;
- bypass ratification for generated, imported, migrated, or externally supplied asset state.

## Testing Strategy

Add tests for:

- asset id and catalog invariants in `domain/asset`;
- dependency invalidation order;
- import plan determinism;
- field-product descriptor invariants;
- `world_sdf` artifact ratification;
- SDF/field source update to `world_ops` invalidation/build queue behavior;
- material graph ratification and deterministic lowering;
- procedural texture cache key stability;
- Texture3D descriptor metadata validation;
- particle emitter reject-missing-field-product behavior;
- physics config and collision product readiness validation;
- animation clip/curve/source-map validation;
- procgen same-seed deterministic operation-window formation;
- gameplay graph ATR IR deterministic lowering to ECS query/event/schedule products;
- gameplay graph SDF physics relation readiness validation;
- V1 project migration to V2 asset catalog;
- failed import preserving prior artifacts;
- import diagnostics roundtrip;
- shader reload status integration;
- scene load through catalog-backed discovery;
- asset browser provider fail-closed behavior.

## First Implementation Slice

The first implementation slice landed as part of the integrated
UI/editor/asset foundation in M4F-M4I. Asset/import/field-product tool surfaces
are registered and projected through active editor definition catalogs rather
than adding another round of hard-coded shell UI.

The slice landed:

1. Active menu, shortcut, command-binding, panel-registry, and tool-surface-registry consumption in the editor shell/app runtime so new asset surfaces can be definition-driven.
2. `domain/asset` with ids, SDF/field-first kinds, source descriptors, artifacts, catalog, import settings, import plans, diagnostics, and ratification.
3. `ProjectFileV2` and V1 migration in `domain/editor/editor_persistence/src/project_file.rs`.
4. App-owned asset catalog runtime in `apps/runenwerk_editor/src/asset_pipeline/catalog_runtime.rs`.
5. Field-product descriptors and ratification in `domain/world_sdf/src/product.rs` and `domain/world_sdf/src/ratification.rs`.
6. App-owned field product job execution in `apps/runenwerk_editor/src/asset_pipeline/field_product_jobs.rs`.
7. Asset Browser, Import Inspector, Field Product Viewer, and SDF Brush Browser providers.
8. Scene manifest compatibility adapter backed by the asset catalog.
9. Blender export job execution using `tools/assets/blender_export.py::main` for configured `.blend` foreign-reference assets, with a missing-tool diagnostic path that preserves the prior valid artifact.

The WR-020 source-backed asset core closeout later hardened the `domain/asset`
contract with backward-compatible V1 catalog source roots, one strict
project-relative path validator for source/catalog/artifact publication,
project catalog descriptors, importer-aware source descriptors,
artifact-kind-aware cache keys, separate source and artifact compatibility
checks, composed catalog ratification, import-plan/source ratification, and
checked prior-valid artifact preservation.
Editor app adapters remain downstream `WR-026` work.

This makes the asset pipeline real without forcing every future asset kind to be complete in the first patch. M5 starts from this point with runtime preview, project-owned data hot reload streams, and restart boundaries.
