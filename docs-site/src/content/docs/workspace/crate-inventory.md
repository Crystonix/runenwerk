---
title: Crate Inventory
description: Active workspace crate inventory and layer ownership.
status: active
owner: workspace
layer: workspace
canonical: true
last_reviewed: 2026-09-11
related_docs:
  - ../../../CRATES.md
  - ./crate-docs-status.md
  - ../guidelines/architecture.md
  - ../guidelines/dependency-rules.md
---

# Crate Inventory

`Cargo.toml` is executable truth for workspace membership. This page is the canonical human-readable inventory of the active Runenwerk workspace and must contain exactly those local workspace-member paths.

Dependency direction, peer-framework ownership, and clean-cutover rules live in [`../guidelines/dependency-rules.md`](../guidelines/dependency-rules.md). Detailed semantics remain with the owning crate/domain/framework documentation.

## Layer rules

- `foundation`: low-level reusable primitives with no domain/runtime/app dependencies.
- `domain`: engine-agnostic reusable contracts and logic owned by Runenwerk while they remain local.
- `engine/runtime`: runtime composition, plugins, renderer/backend integration, and app-loop glue.
- `net`: remaining Runenwerk simulation/history/network-authoring and migration surfaces.
- `app`: runnable applications and tools.
- `adapter/tool`: external host integrations and tooling glue.

## Active workspace members

### Foundation

| Crate | Path | Layer | Purpose |
| --- | --- | --- | --- |
| `id` | `foundation/id` | foundation | Typed identity primitives and allocators. |
| `id_macros` | `foundation/id_macros` | foundation | Attribute macro support for typed ID wrappers. |
| `diagnostics` | `foundation/diagnostics` | foundation | Structured diagnostic reporting vocabulary. |
| `ratification` | `foundation/ratification` | foundation | Shared ratification report vocabulary. |
| `schema` | `foundation/schema` | foundation | Portable schema identity, version, shape, field, constraint, metadata, and descriptor vocabulary. |
| `commands` | `foundation/commands` | foundation | Portable command contract vocabulary. |
| `resource_ref` | `foundation/resource_ref` | foundation | Portable external resource references. |

### Core domains

| Crate | Path | Layer | Purpose |
| --- | --- | --- | --- |
| `runen-ecs` | `domain/ecs` | domain | Current in-Runenwerk RunenECS source authority pending the separately governed external authority transfer/cutover. |
| `runen-ecs-macros` | `domain/ecs_macros` | domain | Proc-macro companion for the current RunenECS source authority. |
| `runen-ecs-conformance` | `domain/ecs_conformance` | domain | Downstream public-boundary fixture for RunenECS standalone conformance. |
| `geometry` | `domain/geometry` | domain | Geometric primitives and queries. |
| `asset` | `domain/asset` | domain | Asset identity, descriptors, import planning, dependency graph, diagnostics, and ratification contracts. |
| `product` | `domain/product` | domain | Formed-product descriptors, jobs, query snapshots, diagnostics, policies, and ratification contracts. |
| `world_ops` | `domain/world_ops` | domain | World-operation logs, quantization policy, dirty tracking, build queues, invalidation, and replication deltas. |
| `world_sdf` | `domain/world_sdf` | domain | SDF world-product payloads and collision/query integration contracts. |
| `scene` | `domain/scene` | domain | Scene-domain data contracts. |
| `graph` | `domain/graph` | domain | Domain-neutral graph definitions, typed ports, validation, traversal, and cycle policy. |
| `texture` | `domain/texture` | domain | Texture product, sampler, color-space, compression, preview, ratification, and lineage contracts. |
| `material_graph` | `domain/material_graph` | domain | Authored material graphs, catalog boundaries, ratification, lowering, source maps, and formed material products. |
| `procgen` | `domain/procgen` | domain | Deterministic procgen documents, planning metadata, ratification, lowering, and product publication. |
| `drawing` | `domain/drawing` | domain | Drawing documents, strokes, brushes, composition, commands, ratification, and tile formation contracts. |

Reusable spatial identity/addressing mechanics are not local workspace crates; Runenwerk consumes standalone `runen-spatial` and retains world/product integration in the owners above.

### UI domains

| Crate | Path | Layer | Purpose |
| --- | --- | --- | --- |
| `ui_math` | `domain/ui/ui_math` | domain | UI math and geometry primitives. |
| `ui_input` | `domain/ui/ui_input` | domain | UI input, focus, routing, pointer, keyboard, stylus, and shortcut contracts. |
| `ui_layout` | `domain/ui/ui_layout` | domain | Stateless UI layout algorithms and constraints. |
| `ui_text` | `domain/ui/ui_text` | domain | Text styles, buffers, metrics, layout, cursor, and selection contracts. |
| `ui_theme` | `domain/ui/ui_theme` | domain | UI theme and styling tokens. |
| `ui_render_data` | `domain/ui/ui_render_data` | domain | Renderer-facing UI frame/data contracts. |
| `ui_composition` | `domain/ui/ui_composition` | domain | App-neutral authored structural composition and transactional structure. |
| `ui_adaptive_composition` | `domain/ui/ui_adaptive_composition` | domain | Transient adaptive projection, reflow, hit-test, preview, and proposal products. |
| `ui_surface` | `domain/ui/ui_surface` | domain | UI surface definition, mount, observation, session, presentation, and compatibility contracts. |
| `ui_graph_editor` | `domain/ui/ui_graph_editor` | domain | Backend-neutral graph-editor view models and edit actions. |
| `ui_definition` | `domain/ui/ui_definition` | domain | Authored UI definitions, validation, normalization, and retained-UI formation. |
| `ui_schema` | `domain/ui/ui_schema` | domain | UI schema contracts. |
| `ui_program` | `domain/ui/ui_program` | domain | UI program contracts. |
| `ui_controls` | `domain/ui/ui_controls` | domain | Reusable UI control contracts. |
| `ui_artifacts` | `domain/ui/ui_artifacts` | domain | UI artifact contracts. |
| `ui_runtime_view` | `domain/ui/ui_runtime_view` | domain | Runtime-facing UI view contracts. |
| `ui_render_primitives` | `domain/ui/ui_render_primitives` | domain | Renderer-neutral UI primitive contracts. |
| `ui_headless_render` | `domain/ui/ui_headless_render` | domain | Headless UI rendering/evaluation support. |
| `ui_headless_render_data` | `domain/ui/ui_headless_render_data` | domain | Headless UI render-data contracts. |
| `ui_static_mount` | `domain/ui/ui_static_mount` | domain | Static UI mounting contracts. |
| `ui_compiler` | `domain/ui/ui_compiler` | domain | UI definition/program compilation. |
| `ui_evaluator` | `domain/ui/ui_evaluator` | domain | UI evaluation contracts and implementation. |
| `ui_state` | `domain/ui/ui_state` | domain | UI state contracts. |
| `ui_binding` | `domain/ui/ui_binding` | domain | UI binding contracts. |
| `ui_hosts` | `domain/ui/ui_hosts` | domain | Host-neutral UI host contracts. |
| `ui_testing` | `domain/ui/ui_testing` | domain | Reusable UI testing support. |
| `ui_accessibility` | `domain/ui/ui_accessibility` | domain | Accessibility semantics and contracts. |
| `ui_geometry` | `domain/ui/ui_geometry` | domain | UI-specific geometry contracts. |
| `ui_tree` | `domain/ui/ui_tree` | domain | Retained UI tree, widget identities, nodes, and computed layout records. |
| `ui_runtime` | `domain/ui/ui_runtime` | domain | Retained UI runtime orchestration, input dispatch, layout, and frame output. |
| `ui_widgets` | `domain/ui/ui_widgets` | domain | Widget constructors over retained UI contracts. |
| `ui_program_lowering` | `domain/ui/ui_program_lowering` | domain | UI-program lowering into retained/runtime products. |
| `ui_story` | `domain/ui/ui_story` | domain | UI story/proof contracts. |
| `ui_app_integration` | `domain/ui/ui_app_integration` | domain | App-facing integration over UI domain contracts. |

### Editor domains

| Crate | Path | Layer | Purpose |
| --- | --- | --- | --- |
| `editor_core` | `domain/editor/editor_core` | domain | Editor command, capability, transaction, ratification, selection, session, and workflow contracts. |
| `editor_definition` | `domain/editor/editor_definition` | domain | Editor-owned definitions, layout, menus, shortcuts, themes, command bindings, panels, and surfaces. |
| `editor_preview` | `domain/editor/editor_preview` | domain | Engine-agnostic preview session, command, event, reload, product, and bootstrap contracts. |
| `editor_shell` | `domain/editor/editor_shell` | domain | Editor-shell composition, workspace identity, observation, expression, routing, and view models. |
| `editor_viewport` | `domain/editor/editor_viewport` | domain | Editor viewport camera, expression, hit, overlay, snap, and viewport contracts. |
| `editor_scene` | `domain/editor/editor_scene` | domain | Editor scene model, commands, descriptors, proposal adapters, and bridges. |
| `editor_inspector` | `domain/editor/editor_inspector` | domain | Inspector model, editing, target resolution, bridge, schema interop, session, and validation. |
| `editor_persistence` | `domain/editor/editor_persistence` | domain | Editor persistence formats, codecs, migration, normalization, formation, and change-log contracts. |

### Engine and networking

| Crate | Path | Layer | Purpose |
| --- | --- | --- | --- |
| `engine` | `engine` | engine/runtime | App/runtime/plugin composition and render/input/time/scene/world/net integration. |
| `engine_render_macros` | `engine_render_macros` | engine/runtime | Render derive macros for GPU parameter contracts. |
| `engine_net` | `net/engine_net` | net | Temporary live replication/protocol-payload/input/prediction migration surface; standalone RunenNet owns reusable networking semantics. |
| `engine_net_macros` | `net/engine_net_macros` | net | Remaining declarative replication metadata macros pending RN8 disposition. |
| `engine_sim` | `net/engine_sim` | net | Simulation identity, tick, hash, profile, RNG, command-frame, and codec vocabulary. |
| `engine_replay` | `net/engine_history` | net | Replay/history/archive/controller/policy/validation substrate. |

Concrete QUIC realization is consumed from external `runen-net-quic` where required; there is no local `net/engine_net_quic` workspace crate.

### Apps and adapters

| Crate | Path | Layer | Purpose |
| --- | --- | --- | --- |
| `runenwerk_editor` | `apps/runenwerk_editor` | app | Runnable editor app and authoring workflows. |
| `runenwerk_draw` | `apps/runenwerk_draw` | app | Focused drawing app shell and shared engine/UI/render runtime integration. |
| `runenwerk_runtime_preview` | `apps/runenwerk_runtime_preview` | app | External runtime-preview child process and preview/play app shell. |
| `native_tablet_input` | `adapters/native_tablet_input` | adapter/tool | Native tablet packet normalization for platform-neutral UI stylus events. |

## External peer frameworks consumed by Runenwerk

Exact dependency revisions are executable truth in root `Cargo.toml` / `Cargo.lock`.

- `runen-net` and `runen-net-quic` — standalone RunenNet authority/transport realization consumed during the remaining RN8 migration.
- `runen-spatial` — standalone reusable spatial identity/addressing mechanics.
- `runen-gpu` — standalone GPU execution authority consumed by Runenwerk/RunenRender integration.

RunenECS is intentionally still listed above as local workspace source authority until the separately governed successor acceptance and Runenwerk consumer cutover complete.

## Documentation status

Documentation coverage is tracked separately in [`crate-docs-status.md`](./crate-docs-status.md). Do not duplicate live crate membership there beyond the coverage rows needed for documentation status.
