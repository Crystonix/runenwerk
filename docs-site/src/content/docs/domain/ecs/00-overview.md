---
title: Overview
description: Engine-agnostic documentation for the ecs domain module.
status: active
owner: ecs
layer: domain
canonical: true
last_reviewed: 2026-09-06
---

# ECS (Entity-Component-System) Domain Overview

## Purpose

- Provide a deterministic, engine-agnostic runtime for entity/component/resource state.
- Keep gameplay/simulation logic data-oriented and composable.
- Expose a small, typed API surface that is easy to integrate with scheduler-driven runtimes.

## Current Foundation Status

The ECS foundation currently includes:

- entities, components, resources
- archetype + dense storage layout
- typed queries with `Added<T>` / `Changed<T>`
- scheduler runtime integration
- deferred structural commands and `BatchCommands`
- typed event channels + observer triggers
- `QueryOrphaned<T>` removed-component stage window
- `ResView<T>` system param alias
- `StatefulComponent` generation/version tracking

## Core Concepts

- **Entity**: stable identifier for domain objects.
- **Component**: per-entity typed state.
- **Resource**: world-level singleton state.
- **System**: typed function operating on queries/resources/commands/events.
- **Query**: typed access to matching component sets, with filters.
- **Command**: deferred structural mutation applied at stage boundaries.
- **Event Channel**: typed event stream with configurable capacity/overflow/lifetime policies.
- **Secondary Indexes**: optional typed lookup acceleration structures.

## Module Boundary Summary

- `world`: world state and orchestration APIs.
- `commands`: deferred command abstractions and queue/apply behavior.
- `query`: query/filter/access runtime.
- `system`: param extraction + runtime scheduling bridge.

## Invariants

- Structural mutations are deferred and become visible only after stage flush.
- Failed schedule runs do not replay deferred commands in later runs.
- Query filter semantics (`Added`/`Changed`) are independent from reporting change logs.
- Event channels enforce configured overflow/lifetime behavior.

## References

- [README.md](./README.md)
- [usage-guide.md](./usage-guide.md)
- [advanced-guide.md](./advanced-guide.md)
- [architecture.md](./architecture.md)
- [features.md](./features.md)
