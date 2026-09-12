---
title: Architecture
description: Runenwerk workspace boundary and code-placement guidance.
status: active
owner: workspace
layer: workspace
canonical: true
last_reviewed: 2026-09-11
---

# Architecture

This document defines the active Runenwerk workspace boundaries and where new Runenwerk-owned code belongs.

For the canonical Runenwerk-wide architecture spine, see
[`../architecture/runenwerk-platform-architecture.md`](../architecture/runenwerk-platform-architecture.md).

Project-neutral software-design defaults are owned by
[Dornglut Engineering's Software Design Standard](https://github.com/dornglut/engineering/blob/main/standards/software-design.md).

For the exact current local workspace-member inventory, see
[`../workspace/crate-inventory.md`](../workspace/crate-inventory.md). For detailed dependency guidance, see [`dependency-rules.md`](dependency-rules.md). Cross-repository family membership, repository relationships, and source-authority transfer governance remain owned by Dornglut Engineering rather than by a Runenwerk guideline.

This page remains the workspace boundary and placement guide; it is not a second crate inventory or repository-family authority.

## Top-Level Areas

- `foundation/`: low-level Runenwerk-owned shared primitives.
- `domain/`: engine-agnostic Runenwerk-owned domain contracts and logic, including UI and editor domain crates.
- `engine/`: Runenwerk runtime composition and plugin integration.
- `net/`: remaining Runenwerk simulation/history/network-authoring and migration surfaces. Standalone RunenNet owns reusable realtime-networking semantics, and `runen-net-quic` owns concrete QUIC realization where a maintained consumer requires it.
- `apps/`: runnable Runenwerk applications and product/tool process wiring.
- `adapters/`: Runenwerk integration glue for external host/runtime boundaries.
- `assets/`: data assets consumed by Runenwerk domains/runtime/apps.
- `docs-site/`: canonical long-form Runenwerk documentation.

Standalone peer-framework implementations such as RunenSpatial and RunenGPU are dependencies, not local workspace areas. Their reusable semantics remain owned by their repositories.

## Dependency Direction

Apply [`dependency-rules.md`](dependency-rules.md) to Runenwerk-local placement under the Engineering owner split above. In local placement terms:

- foundation does not depend on higher Runenwerk layers;
- domain code may depend on foundation and justified lower-level contracts but not on runtime/app wiring or concrete backends it does not own;
- engine/runtime composes domains and peer frameworks without taking their semantic ownership;
- apps and adapters compose higher-level integration but do not define reusable framework/domain invariants.

Avoid sideways coupling between application crates through private internals.

Cross-repository family membership, repository relationships, and source-authority transfers are owned by [Dornglut Engineering](https://github.com/dornglut/engineering/blob/main/architecture/runen-family.md) and [Engineering ADR 0008](https://github.com/dornglut/engineering/blob/main/adrs/0008-adopt-bounded-source-authority-handoffs.md). Runenwerk-local adapter and product-integration consequences are owned by [Framework Integration Architecture](../architecture/repository-family-architecture.md). [ADR 0014](../adr/accepted/0014-repository-family-extraction-boundaries.md) remains a Runenwerk-local accepted decision rather than organization-level family authority.

## Ownership Boundaries

- `foundation/*` owns only the low-level reusable vocabulary explicitly assigned to each foundation crate.
- `domain/*` owns Runenwerk-local engine-agnostic semantic contracts while those contracts remain local authority.
- `engine` owns Runenwerk application/runtime composition and plugin integration.
- `net/*` owns only the remaining Runenwerk-specific or migration responsibilities documented by the current networking authority; it must not duplicate reusable RunenNet semantics.
- `apps/*` owns application/product process wiring and app-local policy.
- `adapters/*` owns explicit Runenwerk translation/interop glue.

If logic is reusable across engine hosts, first identify its semantic owner. Reuse an existing peer framework when that framework owns the invariant; otherwise keep genuinely Runenwerk-owned reusable semantics in the appropriate local domain until a separately accepted extraction exists. Runenwerk-specific runtime/product glue stays in Runenwerk integration surfaces.

## Placement Rules

When adding code:

1. Choose the semantic owner first; current code location is evidence, not permanent ownership.
2. Inspect [`../workspace/crate-inventory.md`](../workspace/crate-inventory.md) for current local package locations and the owning framework/domain docs for semantics.
3. Reuse local helpers in that owner before adding new abstractions.
4. Expose narrow public interfaces instead of reaching into internals across crates or repositories.
5. Preserve the dependency rules and one-way framework direction.
6. Add or update local docs when behavior or scope changes.

## Architecture Guardrails

- Prefer explicit types, deterministic control flow, and clear ownership.
- One semantic invariant set has one authority.
- Do not add silent failure paths or broad catch-all error handling.
- Do not move code across domains or repositories unless the ownership boundary itself is changing under accepted authority.
- Do not infer one universal ID, graph, registry, database, transaction, or runtime merely from repeated vocabulary.
- Keep current documentation aligned with executable workspace/dependency facts without duplicating the same inventory across multiple canonical pages.

See also:

- root `AGENTS.md` for the Runenwerk executor contract;
- [`../workspace/crate-inventory.md`](../workspace/crate-inventory.md) for current local workspace members;
- [`dependency-rules.md`](dependency-rules.md) for dependency and peer-framework rules;
- `code-patterns.md` for implementation patterns used across domains;
- [`domain-program-architecture-pattern.md`](domain-program-architecture-pattern.md) for the optional durable domain-program/compiler/evaluator pattern when a domain actually needs it.
