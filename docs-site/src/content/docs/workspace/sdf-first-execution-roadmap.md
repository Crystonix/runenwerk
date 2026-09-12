---
title: SDF-First Execution Roadmap (Completed)
description: Historical completion record for the May 2026 SDF-first execution substrate; not current sequencing authority.
status: completed
owner: workspace
layer: workspace
canonical: false
last_reviewed: 2026-09-12
replaced_by: ./planning/roadmap.md
related_adrs:
  - ../adr/accepted/0008-adopt-sdf-first-field-product-architecture.md
related_designs:
  - ../design/accepted/sdf-first-field-world-platform-design.md
  - ../design/accepted/field-product-contracts-diagnostics-and-residency-design.md
  - ../design/accepted/execution-fabric-and-product-jobs-design.md
  - ../design/accepted/sdf-product-renderer-and-gpu-residency-design.md
  - ../design/accepted/sdf-first-production-capability-map.md
  - ../design/accepted/runenecs-extraction-boundary-design.md
  - ../design/accepted/runenecs-boundary-repair-execution-plan.md
related_roadmaps:
  - ./planning/roadmap.md
  - ../apps/runenwerk-editor/roadmap.md
  - ../net/multiplayer-replication-implementation-roadmap.md
  - ../engine/plugins/render/docs/roadmap.md
related_reports:
  - ../reports/closeouts/sdf-first-execution-phase-1/closeout.md
  - ../reports/closeouts/sdf-first-execution-phase-2/closeout.md
  - ../reports/closeouts/sdf-first-execution-phase-3/closeout.md
  - ../reports/closeouts/sdf-first-execution-phase-4/closeout.md
  - ../reports/closeouts/sdf-first-execution-phase-5/closeout.md
  - ../reports/closeouts/sdf-first-execution-phase-6a/closeout.md
  - ../reports/closeouts/sdf-first-execution-phase-6b/closeout.md
  - ../reports/closeouts/sdf-first-execution-phase-6c/closeout.md
  - ../reports/closeouts/sdf-first-execution-phase-6d/closeout.md
---

# SDF-First Execution Roadmap (Completed)

## Status

The SDF-first execution program recorded here completed through Phase 6D on
2026-05-14. This page is retained as a historical landing page for that program;
it no longer owns current work selection, activation, priority, blockers, status,
or cross-track sequencing.

Current authority is intentionally split:

- [Roadmap](./planning/roadmap.md) owns durable high-level sequence and dependency
  direction;
- GitHub issues and the Engineering Portfolio own live work state, activation,
  priority, owners, and blockers;
- accepted ADRs and designs own durable architecture and semantic contracts;
- pull requests own delivery and validation evidence;
- the closeouts linked below own detailed point-in-time completion evidence.

The former detailed phase roadmap remains available in Git history. It must not be
used as a current priority ledger or to reactivate retired scheduler or messaging
surfaces.

## Completed Program Outcome

The program established the execution substrate required for SDF-first and procgen
product workflows without making renderer state or application scheduling the source
of world truth:

1. **Phase 0 — contract alignment:** product vocabulary and the then-current SDF,
   material, texture, asset/import, editor, ECS, engine, and render-preparation
   surfaces were aligned around product contracts while preserving serial behavior.
2. **Phase 1 — product jobs and publication:** product jobs gained explicit outcome
   and publication metadata, with deterministic publication after deferred work.
3. **Phase 2 — query snapshots and strict consumption:** runtime query snapshots
   gained generation, freshness, invalidation, consumer-class, and diagnostic
   semantics; strict consumers reject invalid product states.
4. **Phase 3 — render product selection:** render preparation derives backend-neutral
   product selections from formed product truth rather than discovering authority
   during submit.
5. **Phase 4 — derived GPU residency:** renderer-owned GPU cache/residency state is
   derived from product identity, generation, selection, and diagnostics.
6. **Phase 5 — procgen readiness:** procgen ownership, generator documents,
   seed/scope/version policy, cache lineage, authored edit layering, runtime/offline
   policy, and product-output boundaries were accepted before implementation.
7. **Phase 6A — procgen domain track:** deterministic procgen documents, planning
   metadata, ratification, bounded lowering, explanations, and product descriptors
   landed in `domain/procgen`.
8. **Phase 6B — editor/runtime proof:** app-owned procgen runtime state, graph/preview
   providers, product/query publication, render selection/residency participation,
   and bounded overlays proved the integration path.
9. **Phase 6C — concrete field-preview proof:** bounded terrain/material generation
   formed scalar-distance and material-channel CPU field-preview products through the
   accepted product path.
10. **Phase 6D — bake/rollback/persistence proof:** offline bake outcomes, rollback
    evidence, app-owned persistence, accepted-product publication, last-good restore,
    and preview reload classification completed the program.

Further procgen, physics, particles, animation, gameplay, world-process, renderer, or
streaming work is not activated by this completed roadmap. Such work must use its
current owning design/roadmap plus an owning GitHub issue and must consume the
established product contracts rather than inventing a parallel execution path.

## Current Ownership After RunenECS C8

The May 2026 roadmap predated the accepted RunenECS C8 scheduler boundary repair.
Current ownership is therefore:

- standalone RunenECS owns reusable ECS system identity, access facts, explicit
  semantic ordering/system sets, schedule validation, deterministic reference
  execution, deferred commands, and ECS deferred-apply boundaries;
- RunenECS access incompatibility does not create semantic order or an application
  lifecycle boundary;
- Runenwerk Engine/application integration owns host frame/fixed/render lifecycle,
  product and query-snapshot publication policy, and other application barriers;
- the former standalone generic `domain/scheduler` package is retired and must not be
  reconstructed as a forwarding package or generic scheduling authority;
- current multiplayer sequencing belongs to the
  [multiplayer replication implementation roadmap](../net/multiplayer-replication-implementation-roadmap.md),
  not the superseded ECS/runtime convergence roadmap.

For the accepted boundary, use the
[RunenECS boundary repair execution plan](../design/accepted/runenecs-boundary-repair-execution-plan.md)
and the current [RunenECS architecture](../domain/ecs/architecture.md).

## Historical Terminology

The original roadmap and its closeouts were written against the architecture that
existed in May 2026. References in that point-in-time evidence to scheduler waves,
publication barriers, the former standalone scheduler package, or the former
ECS/runtime convergence model describe what was implemented and validated then.
They do **not** define current RunenECS architecture.

Do not rewrite retained closeouts merely to make historical terminology look current.
Do not infer from them that access conflicts create order, that application publication
belongs to RunenECS, or that the retired generic scheduler/messaging surfaces should be
restored.

## Completion Evidence

Detailed phase evidence remains in the retained closeouts:

- [Phase 1 — serial product jobs and publication barriers](../reports/closeouts/sdf-first-execution-phase-1/closeout.md)
- [Phase 2 — query snapshots and strict consumer policy](../reports/closeouts/sdf-first-execution-phase-2/closeout.md)
- [Phase 3 — render product selection producers](../reports/closeouts/sdf-first-execution-phase-3/closeout.md)
- [Phase 4 — derived GPU residency](../reports/closeouts/sdf-first-execution-phase-4/closeout.md)
- [Phase 5 — procgen readiness](../reports/closeouts/sdf-first-execution-phase-5/closeout.md)
- [Phase 6A — procgen domain product track](../reports/closeouts/sdf-first-execution-phase-6a/closeout.md)
- [Phase 6B — visible procgen overlay proof](../reports/closeouts/sdf-first-execution-phase-6b/closeout.md)
- [Phase 6C — concrete terrain/material CPU preview proof](../reports/closeouts/sdf-first-execution-phase-6c/closeout.md)
- [Phase 6D — bake, rollback, persistence, and reload proof](../reports/closeouts/sdf-first-execution-phase-6d/closeout.md)

Those reports are historical evidence. Git history retains the original detailed phase
roadmap and its exact contemporary wording.
