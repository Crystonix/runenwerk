---
title: Events
description: Current RunenECS scope for event and message transport.
status: active
owner: runen-ecs
layer: domain
canonical: true
last_reviewed: 2026-09-10
---

# ECS Events

RunenECS currently does **not** expose a generic event/channel transport API.

The former broadcast-oriented surface (`BroadcastStream`, `BroadcastReader`, `BroadcastWriter`, world broadcast helpers, channel configuration, observers, and drain helpers) was retired before the C8 scheduling cut and is not a compatibility contract.

## Current Boundary

RunenECS owns ECS data and execution semantics:

- components, resources, queries, and change tracking,
- systems and system parameters,
- deferred structural commands,
- schedule labels, system sets, explicit ordering, and validation,
- access facts and deterministic serial reference execution.

Messaging semantics that have a real maintained owner must live with that owner rather than being reconstructed as a generic ECS channel layer. Network/replay/application message transport therefore must not be inferred from the retired broadcast API.

## Scheduling Interaction

Deferred structural mutation is distinct from event transport. `Commands` are collected per system and applied at ECS deferred-apply boundaries. Systems that execute before the same boundary do not observe one another's deferred structural mutations; explicitly ordered dependent work after the boundary does.

Access incompatibility remains diagnostic metadata and does not create semantic ordering or additional deferred-command visibility boundaries.

Current planner stages may describe execution-plan grouping for diagnostics, but they are not event, application-lifecycle, or publication identities.

## Historical Material

Historical reports and audits may still mention the retired event/channel implementation. Those documents are historical evidence only and do not define the current public RunenECS API.
