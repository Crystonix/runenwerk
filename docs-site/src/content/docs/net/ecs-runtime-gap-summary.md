---
title: "ECS Runtime Gap Summary (May 2026, Superseded)"
description: "Historical May 2026 ECS/runtime/multiplayer gap audit; not current repository authority."
status: superseded
owner: net
layer: net
canonical: false
last_reviewed: 2026-09-10
replaced_by: ./multiplayer-replication-implementation-roadmap.md
---

# ECS Runtime Gap Summary (May 2026, Superseded)

This capability audit is a historical May 2026 snapshot and no longer describes current RunenECS or networking ownership.

Later accepted RunenECS C6-C8 work removed or reassigned several capabilities that this audit called current, including generic ECS messaging channels, gameplay ownership/lifecycle policy, scheduler messaging access domains, and the standalone `domain/scheduler` package.

Current authority is split deliberately:

- [RunenECS boundary repair plan](../design/accepted/runenecs-boundary-repair-execution-plan.md) owns the ECS repair/conformance sequence;
- [RunenECS architecture](../domain/ecs/architecture.md) describes the current ECS runtime boundary;
- [Multiplayer replication implementation roadmap](./multiplayer-replication-implementation-roadmap.md) owns current retained networking work.

Do not infer current API support from the historical `Broadcast*`, `WorkQueue*`, `TickBuffer*`, ownership, frame/tick finalization, or scheduler-barrier entries in the former audit. The original snapshot remains available through repository history.
