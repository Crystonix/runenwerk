---
title: "Scheduler Crate (Retired)"
description: "Historical navigation for the scheduler crate retired by RunenECS C8."
status: superseded
owner: ecs
layer: domain
canonical: false
last_reviewed: 2026-09-10
replaced_by: ../ecs/README.md
---

# Scheduler Crate (Retired)

The standalone `domain/scheduler` crate was retired by RunenECS C8.

Its reusable ECS-owned semantics now live in `domain/ecs`: generic schedule labels, system sets, explicit semantic ordering, ECS access facts, validation, deterministic serial reference execution, and deferred-command/deferred-apply boundaries.

Application lifecycle and publication policy remain owned by Runenwerk Engine. The retired scheduler's phases, waves, product/query publication barriers, generic DAG/demo/DOT/filesystem utilities, and scheduler-global telemetry are not current authority and are not compatibility contracts.

Use current authority instead:

- [RunenECS overview](../ecs/00-overview.md)
- [RunenECS architecture](../ecs/architecture.md)
- [Accepted RunenECS extraction boundary](../../design/accepted/runenecs-extraction-boundary-design.md)
- [Accepted RunenECS boundary repair plan](../../design/accepted/runenecs-boundary-repair-execution-plan.md)

Historical implementation details remain available through repository history and historical reports; they must not be used to reconstruct a standalone scheduler owner.
