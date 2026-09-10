---
title: "Crate Documentation Status"
description: "Current documentation coverage status for Runenwerk workspace crates."
status: active
owner: workspace
layer: workspace
canonical: true
last_reviewed: 2026-09-11
---

# Crate Documentation Status

This page tracks whether each workspace crate has current, discoverable documentation. Use [`crate-inventory.md`](./crate-inventory.md) for the canonical human-readable current workspace inventory.

Status labels:

- `current`: factual current architecture or usage docs exist.
- `thin`: some docs exist, but they do not yet cover the crate's current public surface.
- `roadmap`: roadmap/proposal docs exist but current-state docs are missing.
- `missing`: no useful crate-specific docs yet.

## Foundation

| Crate | Path | Status | Primary docs |
| --- | --- | --- | --- |
| `id` | `foundation/id` | current | [`../foundation/id/README.md`](../foundation/id/README.md) |
| `id_macros` | `foundation/id_macros` | current | [`../foundation/id-macros/README.md`](../foundation/id-macros/README.md) |
| `diagnostics` | `foundation/diagnostics` | current | [`../foundation/diagnostics/current-state.md`](../foundation/diagnostics/current-state.md) |
| `ratification` | `foundation/ratification` | current | [`../foundation/ratification/README.md`](../foundation/ratification/README.md) |
| `schema` | `foundation/schema` | current | [`../foundation/schema/README.md`](../foundation/schema/README.md) |
| `commands` | `foundation/commands` | current | [`../foundation/commands/README.md`](../foundation/commands/README.md) |
| `resource_ref` | `foundation/resource_ref` | current | [`../foundation/resource-ref/README.md`](../foundation/resource-ref/README.md) |

## Domain

| Crate | Path | Status | Primary docs |
| --- | --- | --- | --- |
| `runen-ecs` | `domain/ecs` | current | [`../domain/ecs/README.md`](../domain/ecs/README.md) |
| `runen-ecs-macros` | `domain/ecs_macros` | thin | [`../domain/ecs-macros/README.md`](../domain/ecs-macros/README.md) |
| `runen-ecs-conformance` | `domain/ecs_conformance` | current | [`../domain/ecs/README.md`](../domain/ecs/README.md) |
| `geometry` | `domain/geometry` | current | [`../domain/geometry/README.md`](../domain/geometry/README.md) |
| `asset` | `domain/asset` | thin | [`../domain/00-overview.md`](../domain/00-overview.md) |
| `product` | `domain/product` | thin | [`../domain/00-overview.md`](../domain/00-overview.md) |
| `world_ops` | `domain/world_ops` | thin | [`../domain/world-ops/README.md`](../domain/world-ops/README.md) |
| `world_sdf` | `domain/world_sdf` | thin | [`../domain/world-sdf/README.md`](../domain/world-sdf/README.md) |
| `graph` | `domain/graph` | current | [`../domain/graph/README.md`](../domain/graph/README.md) |
| `texture` | `domain/texture` | thin | [`../domain/texture/README.md`](../domain/texture/README.md) |
| `material_graph` | `domain/material_graph` | thin | [`../domain/material-graph/README.md`](../domain/material-graph/README.md) |
| `procgen` | `domain/procgen` | thin | [`../domain/procgen/README.md`](../domain/procgen/README.md) |
| `drawing` | `domain/drawing` | current | [`../domain/drawing/README.md`](../domain/drawing/README.md) |
| `scene` | `domain/scene` | thin | [`../domain/scene/README.md`](../domain/scene/README.md) |
| `domain/ui/*` | `domain/ui` | current | [`../domain/ui/architecture.md`](../domain/ui/architecture.md) |
| `domain/editor/*` | `domain/editor` | thin | [`../domain/editor/README.md`](../domain/editor/README.md) |

The former `domain/scheduler` crate was retired by RunenECS C8. Its retained documentation pages are superseded historical navigation, not active crate documentation.

## Engine And Net

| Crate | Path | Status | Primary docs |
| --- | --- | --- | --- |
| `engine` | `engine` | current | [`../engine/index.md`](../engine/index.md) |
| `engine_render_macros` | `engine_render_macros` | current | [`../engine/reference/plugins/render/render-macros.md`](../engine/reference/plugins/render/render-macros.md) |
| `engine_sim` | `net/engine_sim` | current | [`../net/engine-sim/README.md`](../net/engine-sim/README.md) |
| `engine_net` | `net/engine_net` | current | [`../net/engine-net/README.md`](../net/engine-net/README.md) |
| `engine_net_macros` | `net/engine_net_macros` | current | [`../net/engine-net-macros/README.md`](../net/engine-net-macros/README.md) |
| `engine_replay` | `net/engine_history` | current | [`../net/engine-history/README.md`](../net/engine-history/README.md) |

## Apps And Adapters

| Crate | Path | Status | Primary docs |
| --- | --- | --- | --- |
| `runenwerk_editor` | `apps/runenwerk_editor` | current | [`../apps/runenwerk-editor/current-architecture.md`](../apps/runenwerk-editor/current-architecture.md) |
| `runenwerk_draw` | `apps/runenwerk_draw` | current | [`../apps/runenwerk-draw/README.md`](../apps/runenwerk-draw/README.md) |
| `runenwerk_runtime_preview` | `apps/runenwerk_runtime_preview` | current | [`../reports/closeouts/m5-runtime-preview/closeout.md`](../reports/closeouts/m5-runtime-preview/closeout.md) |
| `native_tablet_input` | `adapters/native_tablet_input` | current | [`../adapters/native-tablet-input/README.md`](../adapters/native-tablet-input/README.md) |

## Long-Term Maintenance

When workspace crates are added, removed, split, or renamed:

1. Update `Cargo.toml`.
2. Update [`crate-inventory.md`](./crate-inventory.md) in the same accepted cut.
3. Update this page when documentation coverage changes.
4. Update the owning crate/domain documentation when semantics or public surface change.
5. Run `cargo validate`; repository CI must validate the unchanged reviewed head before merge.
