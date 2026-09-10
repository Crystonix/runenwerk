---
title: "Scheduler Design Goals (Retired)"
description: "Historical navigation for scheduler design goals superseded by RunenECS C8 ownership."
status: superseded
owner: ecs
layer: domain
canonical: false
last_reviewed: 2026-09-10
replaced_by: ../ecs/architecture.md
---

# Scheduler Design Goals (Retired)

These goals described the former standalone `domain/scheduler` package. That package and its generic scheduling authority were retired by RunenECS C8.

Current ownership is deliberately split:

- RunenECS owns reusable ECS system identity, schedule labels, system sets, explicit semantic ordering, access facts, validation, deterministic serial reference execution, and deferred structural-command boundaries.
- Runenwerk Engine owns frame/fixed/render/startup/shutdown lifecycle, host execution policy, product/query-snapshot publication, and other application barriers.

The former generic DAG, wave, application-phase, publication-barrier, graph-export, and scheduler-global telemetry goals are historical evidence only. They do not authorize a replacement scheduler framework or compatibility surface.

Use [RunenECS architecture](../ecs/architecture.md) and the [accepted RunenECS boundary repair plan](../../design/accepted/runenecs-boundary-repair-execution-plan.md) for current authority.
