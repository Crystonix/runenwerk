---
title: Overview
description: Engine-agnostic documentation for the ecs domain module.
status: active
owner: ecs
layer: domain
canonical: true
last_reviewed: 2026-09-10
---

# ECS (Entity-Component-System) Domain Overview

## Purpose

- Provide a deterministic, engine-agnostic runtime for entity/component/resource state.
- Keep gameplay and simulation logic data-oriented and composable.
- Expose typed ECS execution semantics that can be embedded by hosts without owning application lifecycle policy.

## Current Foundation Status

The ECS foundation currently includes:

- opaque world-local entities, components, and resources
- archetype + dense storage implementation
- typed queries with `Added<T>` / `Changed<T>`
- ECS-native system registration, schedule labels, system sets, explicit ordering, access validation, and deterministic serial reference execution
- deferred structural commands and ECS-owned deferred-apply boundaries
- current removed-component observation through `QueryOrphaned<T>`
- resource parameters through `Res<T>` / `ResMut<T>` (`ResView<T>` is currently an alias of `Res<T>`)
- explicit reflection and ECS-local change tracking
- optional typed secondary indexes and feature-gated ECS telemetry

RunenECS currently exposes no generic event/channel transport API. Application, network, replay, render, product-publication, frame, fixed-step, startup, and shutdown policy remain outside RunenECS.

## Core Concepts

- **Entity**: opaque world-local runtime handle; not a persistence or network identity.
- **Component**: per-entity typed state.
- **Resource**: world-level singleton state.
- **System**: typed function operating on queries/resources/commands through declared system parameters.
- **Query**: typed access to matching component sets, with filters.
- **Command**: deferred structural mutation made visible at an ECS deferred-apply boundary.
- **Schedule / System Set**: generic ECS identity and explicit semantic-ordering structure.
- **Execution Stage**: current planning/diagnostic grouping derived from semantic ordering; not an Engine lifecycle or publication identity.
- **Secondary Index**: optional typed ECS lookup acceleration.

## Module Boundary Summary

- `world`: world state and world-facing APIs.
- `commands`: deferred command abstractions and queue/apply behavior.
- `query`: query/filter/access runtime.
- `system`: system parameters, execution integration, and ECS-neutral reports.
- crate-private `scheduler`: ECS-owned schedule labels, access facts, registered systems, plan construction, and validation; not a standalone scheduler package or host lifecycle owner.

## Invariants

- Structural mutations are deferred during runtime-managed system execution and become visible only after an ECS deferred-apply boundary.
- Explicit `before` / `after` relations define semantic precedence; access incompatibility does not invent semantic order.
- Failed schedule runs do not replay discarded deferred command queues in later runs.
- Query filter semantics (`Added` / `Changed`) are independent from reporting change logs.
- Planner stages are implementation/planning facts and do not define application frame, render, network, replay, or product-publication semantics.

## References

- [README.md](./README.md)
- [usage-guide.md](./usage-guide.md)
- [advanced-guide.md](./advanced-guide.md)
- [architecture.md](./architecture.md)
- [features.md](./features.md)
