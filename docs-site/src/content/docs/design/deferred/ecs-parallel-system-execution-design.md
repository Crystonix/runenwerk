---
title: ECS Parallel System Execution Design
description: Deferred design for future ECS schedule parallelism, deterministic command merging, world access sharding, and blocked-parallelism diagnostics.
status: deferred
owner: runen-ecs
layer: domain
canonical: true
last_reviewed: 2026-09-10
related_designs:
  - ../accepted/sdf-first-production-capability-map.md
related_roadmaps:
  - ../../domain/ecs/features.md
  - ../../net/ecs-runtime-prioritized-roadmap.md
---

# ECS Parallel System Execution Design

## Deferred status

ECS parallel execution is not active work. Current ECS execution remains serial. Reactivate only through a separate accepted parallelism issue after serial ECS scheduler/access semantics and conformance are stable, with explicit serial/parallel equivalence, deterministic command merge, and ownership constraints. Do not infer activation from existing dependency-wave or conflict metadata.

## Status

Deferred design only. ECS execution remains serial while current scheduler/access semantics stabilize. Product jobs remain the active multithreading path for field/import/render-product work.

## Existing Groundwork

The scheduler already computes dependency waves and conflict diagnostics:

- `domain/scheduler/src/plan.rs::ExecutionScheduler::run_schedule`
- standalone `runen-ecs` runtime `Runtime::run_schedule`
- `domain/scheduler/src/plan.rs::SerialWaveMirrorsStage`

Current execution is serial by wave and serial within each wave. Deferred commands are not thread-safe and are flushed after each wave.

## Future Parallel Contract

Parallel ECS implementation must define:

- `Send + Sync` constraints for systems and resources that may run in parallel;
- explicit read/write access metadata for world sharding;
- per-wave command queues;
- deterministic command merge order;
- diagnostics for systems blocked from parallel execution;
- a serial fallback with identical observable results.

## Non Goals

- No parallel ECS implementation in the current serial ECS slice.
- No public ECS parallel APIs before the design is reactivated and accepted with fitness tests.
- No hidden global mutable state to bypass scheduler access contracts.

## Implementation Sequence

The sequence below is dormant while this design is deferred:

1. Keep standalone `runen-ecs` `Runtime::run_schedule` serial.
2. Add blocked-parallelism reporting from existing wave/conflict metadata.
3. Introduce thread-safe deferred command buffers behind an internal feature gate.
4. Add deterministic merge tests.
5. Add parallel wave execution only after serial/parallel equivalence tests exist.

## Tests

Required future coverage after reactivation:

- serial and parallel schedules produce identical world state;
- command buffers merge deterministically;
- blocked systems report exact access conflicts;
- non-`Send + Sync` systems remain serial with diagnostics;
- scheduler wave diagnostics remain stable.
