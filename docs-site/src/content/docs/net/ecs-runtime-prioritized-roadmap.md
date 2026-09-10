---
title: "ECS Runtime Prioritized Roadmap (Superseded)"
description: "Historical pre-C6/C7/C8 ECS/runtime/network convergence roadmap; not current sequencing authority."
status: superseded
owner: net
layer: net
canonical: false
last_reviewed: 2026-09-10
replaced_by: ./multiplayer-replication-implementation-roadmap.md
---

# ECS Runtime Prioritized Roadmap (Superseded)

This roadmap described the pre-C6/C7/C8 convergence model and is no longer current sequencing authority.

Its former priorities assumed generic ECS `Broadcast*`, `WorkQueue*`, and `TickBuffer*` messaging, scheduler-owned access domains, application-shaped barriers, and a live standalone `domain/scheduler` package. Accepted RunenECS repair work subsequently removed or reassigned those responsibilities.

Use current authority instead:

- [Accepted RunenECS boundary repair plan](../design/accepted/runenecs-boundary-repair-execution-plan.md) for ECS repair and conformance sequencing;
- [Accepted RunenECS extraction boundary](../design/accepted/runenecs-extraction-boundary-design.md) for ECS/Runenwerk ownership;
- [Current multiplayer replication implementation roadmap](./multiplayer-replication-implementation-roadmap.md) for retained networking work;
- [Current RunenECS architecture](../domain/ecs/architecture.md) for implemented ECS semantics.

The original roadmap remains available through repository history for provenance. It must not be used to reactivate removed messaging/scheduler surfaces or to infer current priorities.
