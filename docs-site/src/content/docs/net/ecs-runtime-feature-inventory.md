---
title: "ECS Runtime Feature Inventory (April 2026, Superseded)"
description: "Historical April 2026 ECS/runtime/network capability snapshot; not current repository authority."
status: superseded
owner: net
layer: net
canonical: false
last_reviewed: 2026-09-10
replaced_by: ../design/accepted/runenecs-boundary-repair-execution-plan.md
---

# ECS Runtime Feature Inventory (April 2026, Superseded)

This page is retained only as historical navigation for the April 2026 audit. It no longer describes the current repository state.

Subsequent accepted RunenECS repairs materially changed the audited boundary:

- C6 removed the mixed broadcast/work-queue/tick-buffer messaging authority from RunenECS;
- C7 separated ownership, networking, replay, and application lifecycle policy from ECS-local semantics;
- C8 moves reusable ECS scheduling/access/deferred semantics into RunenECS and retires the standalone `domain/scheduler` package and its application-shaped barrier authority.

Do not use the former `domain/scheduler`, `Broadcast*`, `WorkQueue*`, `TickBuffer*`, ownership-registry, stage-flush, or scheduler-barrier statements from this audit as current API or architecture evidence.

Use current authority instead:

- [Accepted RunenECS boundary repair plan](../design/accepted/runenecs-boundary-repair-execution-plan.md)
- [Accepted RunenECS extraction boundary](../design/accepted/runenecs-extraction-boundary-design.md)
- [Current RunenECS architecture](../domain/ecs/architecture.md)
- [Current multiplayer replication roadmap](./multiplayer-replication-implementation-roadmap.md)

The original audit remains recoverable from repository history when historical comparison is required.
