---
title: Crate Inventory
description: Active workspace crate inventory and layer ownership.
status: active
owner: workspace
layer: workspace
canonical: true
last_reviewed: 2026-09-10
related_docs:
  - ../../../CRATES.md
  - ./crate-docs-status.md
  - ../guidelines/architecture.md
---

# Crate Inventory

This document records active workspace crates and their intended layer. Update it when workspace membership changes.

## Layer rules

- `foundation`: low-level reusable primitives with no domain/runtime/app dependencies.
- `domain`: engine-agnostic reusable contracts and logic.
- `engine/runtime`: runtime composition, plugins, renderer/backend integration, and app loop glue.
- `net`: transport/session/replication/history contracts and transport adapters.
- `app`: runnable applications and tools.
- `adapter/tool`: external host integrations and tooling glue.

## Active workspace crates

| Crate | Path | Layer | Purpose | Public API |
| --- | --- | --- | --- | --- |
| `id` | `foundation/id` | foundation | Typed identity primitives and allocators. | evolving |
| `ecs` | `domain/ecs` | domain | Entity/component/resource world, queries, reflection/change tracking, ECS system/schedule/access semantics, deterministic serial execution, and deferred structural commands. | evolving |
| `ecs_macros` | `domain/ecs_macros` | domain | Derive macros for ECS component/resource/bundle/reflection contracts. | evolving |
| `id_macros` | `foundation/id_macros` | foundation | Attribute macro support for typed ID wrappers. | evolving |
| `diagnostics` | `foundation/diagnostics` | foundation | Structured diagnostic reporting vocabulary. | evolving |
| `ratification` | `foundation/ratification` | foundation | Shared ratification report vocabulary. | evolving |
| `schema` | `foundation/schema` | foundation | Portable schema identity, version, path, value, shape, field, constraint, metadata, and descriptor vocabulary. | evolving |
| `commands` | `foundation/commands` | foundation | Portable command contract vocabulary. | evolving |
| `resource_ref` | `foundation/resource_ref` | foundation | Portable external resource references. | evolving |
| `geometry` | `domain/geometry` | domain | Geometric primitives and queries. | evolving |
| `asset` | `domain/asset` | domain | Asset identity, descriptors, import planning, dependency graph, diagnostics, and ratification contracts. | evolving |
| `product` | `domain/product` | domain | Shared formed-product descriptors, product jobs, query snapshots, diagnostics, policies, and ratification contracts. | evolving |
| `world_ops` | `domain/world_ops` | domain | Chunk/world operation logs, dirty tracking, build queues, invalidation, and replication deltas. | evolving |
| `world_sdf` | `domain/world_sdf` | domain | SDF chunk payloads and collision query contracts. | evolving |
| `graph` | `domain/graph` | domain | Domain-neutral graph definitions, typed ports, validation, traversal, and cycle policy. | evolving |
| `texture` | `domain/texture` | domain | Texture product descriptors, previews, samplers, color-space, compression, ratification, and lineage contracts. | evolving |
| `material_graph` | `domain/material_graph` | domain | Authored material graph documents, catalog boundaries, ratification, lowering, and formed material descriptors. | evolving |
| `procgen` | `domain/procgen` | domain | Deterministic procgen documents, planning metadata, ratification, lowering, and product outputs. | evolving |
| `drawing` | `domain/drawing` | domain | Drawing documents, strokes, brushes, composition, commands, ratification, and tile formation contracts. | evolving |
| `scene` | `domain/scene` | domain | Scene transform value contracts and schema descriptors. | evolving |
| `ui_math` | `domain/ui/ui_math` | domain | UI geometry primitives. | evolving |
| `ui_input` | `domain/ui/ui_input` | domain | UI input, focus, routing, keyboard, pointer, stylus packets, and shortcut contracts. | evolving |
| `ui_layout` | `domain/ui/ui_layout` | domain | Stateless UI layout algorithms and constraints. | evolving |
| `ui_text` | `domain/ui/ui_text` | domain | Text styles, buffers, atlas metrics, layout, cursor, and selection contracts. | evolving |
| `ui_theme` | `domain/ui/ui_theme` | domain | UI color, spacing, radius, typography, and theme tokens. | evolving |
| `ui_render_data` | `domain/ui/ui_render_data` | domain | Renderer-facing UI frame and primitive contracts. | evolving |
| `ui_surface` | `domain/ui/ui_surface` | domain | Surface definition, mount, observation, session, presentation, intent, capability, and ratification contracts. | evolving |
| `ui_graph_editor` | `domain/ui/ui_graph_editor` | domain | Backend-neutral graph editor view models and edit actions. | evolving |
| `ui_definition` | `domain/ui/ui_definition` | domain | Authored UI templates, slots, menus, availability, validation, normalization, and retained UI formation. | evolving |
| `ui_tree` | `domain/ui/ui_tree` | domain | Retained UI tree, widget IDs, nodes, and computed layout records. | evolving |
| `ui_runtime` | `domain/ui/ui_runtime` | domain | Retained UI runtime orchestration, input dispatch, layout, frame output, and runtime state. | evolving |
| `ui_widgets` | `domain/ui/ui_widgets` | domain | Widget constructors over `ui_tree` contracts. | evolving |
| `editor_core` | `domain/editor/editor_core` | domain | Editor command, capability, transaction, ratification, migration, selection, session, sharing, and workflow contracts. | evolving |
| `editor_definition` | `domain/editor/editor_definition` | domain | Editor-owned definition documents, layout, menu, shortcut, theme, command binding, panel registry, and surface schemas. | evolving |
| `editor_preview` | `domain/editor/editor_preview` | domain | Engine-agnostic preview session, command, event, reload decision/status, runtime product, and bootstrap DTOs. | evolving |
| `editor_inspector` | `domain/editor/editor_inspector` | domain | Inspector model, editing, target, resolution, bridge, schema interop, session, and validation contracts. | evolving |
| `editor_scene` | `domain/editor/editor_scene` | domain | Editor scene model, commands, descriptors, proposal adapter, bridge, and scene command contracts. | evolving |
| `editor_viewport` | `domain/editor/editor_viewport` | domain | Editor viewport camera, expression, hit, overlay, snap, and viewport contracts. | evolving |
| `editor_shell` | `domain/editor/editor_shell` | domain | Editor shell composition, workspace identity, observation, expression, command routing, and view models. | evolving |
| `editor_persistence` | `domain/editor/editor_persistence` | domain | Editor persistence formats, RON codec, scene migration/normalization/formation, and change-log contracts. | evolving |
| `engine` | `engine` | engine/runtime | App/runtime/plugin composition, render/input/time/scene/world/net integration, and renderer execution. | evolving |
| `engine_render_macros` | `engine_render_macros` | engine/runtime | Render derive macros for GPU parameter contracts. | evolving |
| `engine_sim` | `net/engine_sim` | net | Simulation identity, tick, hash, profile, RNG, command-frame, and codec vocabulary. | evolving |
| `engine_net` | `net/engine_net` | net | Transport-agnostic protocol, replication, session, runtime, simulation, and transport contracts. | evolving |
| `engine_net_macros` | `net/engine_net_macros` | net | Attribute macros for network component/entity metadata. | evolving |
| `engine_replay` | `net/engine_history` | net | Replay archive, recorder, controller, policy, and validation substrate. | evolving |
| `runenwerk_editor` | `apps/runenwerk_editor` | app | Runnable editor app and authoring workflows. | internal/evolving |
| `runenwerk_draw` | `apps/runenwerk_draw` | app | Focused drawing app shell and shared engine/UI/render runtime integration. | internal/evolving |
| `runenwerk_runtime_preview` | `apps/runenwerk_runtime_preview` | app | External runtime preview child process and preview/play app shell. | internal/evolving |
| `native_tablet_input` | `adapters/native_tablet_input` | adapter/tool | Native tablet packet normalization proof for platform-neutral `ui_input` stylus events. | internal/evolving |

`domain/scheduler` is no longer an active workspace crate after RunenECS C8. Reusable ECS scheduling semantics are owned by `domain/ecs`; Engine retains application lifecycle and publication policy.

## Documentation status

Crate documentation status is tracked in:

```text
docs-site/src/content/docs/workspace/crate-docs-status.md
```
