---
title: Domain Map
description: Domain Map
status: active
owner: workspace
layer: workspace
canonical: true
last_reviewed: 2026-09-10
---

# Domain Map

This map tracks crate ownership and allowed dependency direction for the active workspace.

## Domain Layer

- `foundation/id`: shared typed-id primitives and allocation contracts
- `foundation/id_macros`: attribute macro support for typed ID wrappers
- `foundation/diagnostics`: structured diagnostic reporting vocabulary
- `foundation/ratification`: shared ratification report vocabulary
- `foundation/schema`: portable schema identity, version, path, value, shape, field, constraint, metadata, and descriptor vocabulary
- `foundation/commands`: portable command contract identity, schema reference, descriptor, proposal, metadata, hint, issue, and optional diagnostics-bridge vocabulary
- `foundation/resource_ref`: portable external resource references with canonical identity encoding and no catalog/runtime lookup ownership
- `domain/ecs`: entity/component/resource storage and queries, explicit reflection, ECS-local change tracking, system identity and parameters, generic schedule labels/system sets, explicit semantic ordering, access facts and validation, deterministic serial reference execution, and deferred structural-command visibility
- `domain/ecs_macros`: derive macros for ECS component/resource/bundle/reflection contracts
- `domain/scene`: scene-domain data contracts
- `domain/asset`: engine-agnostic asset identity, source/artifact descriptors, deterministic import planning, dependency graph, diagnostics, and ratification contracts
- `domain/product`: shared formed-product descriptors, product jobs, query snapshots, render product selection, diagnostics, policies, and ratification contracts
- `domain/geometry`: explicit geometric primitives and geometric queries
- `domain/spatial`, `domain/spatial_index`, and `domain/chunking`: spatial coordinate, index, and streaming contracts
- signed-field mathematics and CPU reference queries are owned by standalone `dornglut/runen-sdf`; Runenwerk retains only integration and `domain/world_sdf` product/world contracts
- `domain/world_ops`: chunk/world operation logs, dirty tracking, build queues, invalidation, and replication deltas
- `domain/world_sdf`: SDF chunk payloads, collision query contracts, formed field-product descriptors, ratification, and cave/sector storage summaries
- `domain/graph`: domain-neutral graph definitions, typed ports, validation, traversal, and cycle policy
- `domain/texture`: texture product, sampler, color-space, compression, ratification, preview, and lineage contracts
- `domain/material_graph`: authored material graph documents, node catalog boundaries, ratification, lowering, source maps, cache keys, and formed material product descriptors
- `domain/procgen`: deterministic procgen documents, terrain/material node catalog semantics, planning metadata, ratification, lowering to world operation windows, and product job/publication descriptors
- `domain/drawing`: drawing document, stroke, brush, paper, layer/composition graph, command, ratification, diagnostic, deterministic ink tile formation, product helper, and tile-lineage contracts
- `domain/ui/*`: UI geometry, input, layout, text, theme, render-data, surface, graph-editor, definition, tree, runtime, and widget contracts
- `domain/editor/*`: editor-facing domain logic (inspector, scene editing, viewport)

There is no active standalone `domain/scheduler` crate. RunenECS owns reusable ECS scheduling semantics; Engine owns application/frame/fixed/render lifecycle and publication policy.

## Engine Layer

- `engine` (`engine`): `engine::App`, plugin composition, scene/render/input/time/UI runtime integration
- `engine_render_macros`: render derive macros for GPU parameter contracts

Primary plugin modules live under:

- `engine/src/plugins/scene`
- `engine/src/plugins/render`
- `engine/src/plugins/input`
- `engine/src/plugins/time`
- `domain/ui/*` with engine scene/render integration
- `engine/src/plugins/grid`
- `engine/src/plugins/shared`
- `engine/src/plugins/debug_metrics`
- `engine/src/plugins/scheduler_diagnostics`

## Networking Layer

- `net/engine_sim` (`engine_sim`): simulation identity, tick, hash, profile, RNG, command-frame, and codec vocabulary
- `net/engine_net` (`engine_net`): protocol/session/replication contracts
- `net/engine_net_quic` (`engine_net_quic`): QUIC transport/runtime adapter
- `net/engine_net_macros` (`engine_net_macros`): proc-macro support for replication/network derives
- `net/engine_history` (`engine_replay`): replay archive, recorder, controller, policy, and validation substrate

## App and Adapter Layer

- `apps/runenwerk_editor`: runnable editor app, shell bridge, runtime viewport/render integration, persistence, and authoring workflows
- `apps/runenwerk_draw`: focused drawing app shell with shared engine/UI/render runtime integration
- `apps/runenwerk_runtime_preview`: external runtime preview child process and headless preview test harness
- `adapters/godot_chunking_addon`: Godot bridge for chunking/spatial integration
- `adapters/native_tablet_input`: native tablet packet normalization proof for platform-neutral `ui_input` stylus events

## Non-Crate Supporting Domains

- `assets/`: authoring/runtime data assets
- `docs-site/`: documentation source and navigation pages

## Dependency Rules

Preferred dependency direction:

- `foundation` <- `domain`
- `foundation` <- `engine`
- `foundation` <- `net`
- `foundation` <- `apps`
- `foundation` <- `adapters`
- `domain` <- `engine`
- `domain` <- `net`
- `domain` <- `apps`
- `engine` <- `apps`
- `net` <- `apps`
- `domain` <- `adapters`

Disallowed direction:

- `domain` depending on `engine`, `net`, `apps`, or `adapters`
- `engine` depending on `apps`
- private cross-app dependencies (`apps/*` directly depending on another `apps/*`)
