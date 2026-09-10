---
title: Architecture Governance Review
description: Review of Clean Architecture, DDD, ADRs, fitness functions, ATAM-lite, Strangler Fig migration, and Team Topologies against the Runenwerk workspace.
status: active
owner: workspace
layer: workspace
canonical: true
last_reviewed: 2026-09-10
related:
  - ./planning-methods.md
  - ./planning-and-implementation-workflow.md
  - ./diagrams/design-intake-roadmap-automation.puml
  - ./roadmap-decision-register.md
  - ./design-implementation-triage.md
  - ../guidelines/domain-map.md
  - ../guidelines/module-structure-guidelines.md
  - ../guidelines/runenwerk-architecture.md
  - ../adr/README.md
---

# Architecture Governance Review

## Purpose

This review checks whether common architecture and planning methods should improve Runenwerk's current docs, dependency rules, and roadmap process.

The recommendation is to use these methods as governance lenses, not as imported folder templates. Runenwerk already has its own domain and reality doctrine; the useful improvement is to make dependency direction, decision history, migration, tradeoff analysis, enforcement, and ownership more explicit.

## Review Findings

| Method | Current repo fit | Improvement |
|---|---|---|
| Clean Architecture | Strong fit as a dependency-direction rule. `guidelines/domain-map.md` already states allowed direction and `guidelines/runenwerk-architecture.md` has replaceability and Rust-enforced boundary laws. | Name the Clean Architecture dependency rule explicitly: inner policy/domain contracts must not depend on outer app, engine, adapter, UI, transport, or persistence details. Do not copy Clean Architecture folder taxonomy. |
| DDD | Strong fit. Module structure already says to organize by subdomain responsibility and real ownership boundaries. | Treat each significant crate/subsystem as a bounded context candidate with explicit owner, vocabulary, invariants, public contracts, and translation boundaries. |
| ADRs | Already implemented. `adr/README.md` has accepted, proposed, superseded, and rejected ADR lifecycle folders. | Require an ADR when a roadmap item changes dependency direction, source-of-truth ownership, cross-domain contracts, or long-term migration policy. Do not create ADRs for temporary score changes. |
| Fitness functions | Partially implemented. Docs validation, full gate, architecture guard tests, and source-marker guard tests already act as fitness functions. | Make fitness functions a named enforcement layer and add a future cargo-metadata dependency-direction check against `guidelines/domain-map.md`. |
| ATAM-lite | Weakly present. Designs and closeouts discuss tradeoffs, but no reusable tradeoff-review shape exists. | Add ATAM-lite before promoting `B3`/`B4` work when quality attributes conflict, such as latency vs determinism, autonomy vs consistency, or editor speed vs architectural isolation. |
| Strangler Fig | Partially present as migration roadmaps and deferred legacy replacement work. | Use a formal migration shape for replacing old bridges: identify old path, route new path in parallel, prove equivalence, switch callers, then delete the old path with guards. |
| Team Topologies | Not a literal team model in the repo, but ownership metadata exists in frontmatter and domain maps. | Use lightweight ownership modes for planning: stream-aligned product surface, platform substrate, complicated subsystem, and enabling work. Do not invent a human org chart in docs. |

## Recommended Operating Model

Use the methods in this order:

1. DDD identifies the owning bounded context and vocabulary.
2. Clean Architecture dependency direction checks whether the owner is allowed to depend on the collaborator.
3. ADRs record long-term architecture decisions.
4. ATAM-lite records tradeoffs before risky promotions.
5. Strangler Fig shapes migrations that replace an existing path.
6. Fitness functions enforce the chosen boundary.
7. Team Topologies labels the ownership mode and expected collaboration shape.

This order keeps the repo from turning methods into ceremony. The method only applies when it changes a decision or prevents architectural drift.

## Concrete Improvements

### Clean Architecture

Use the dependency rule as the concrete improvement:

- `foundation` must not know domain, engine, net, app, adapter, UI framework, or backend details.
- reusable `domain` crates must not know engine, net, app, adapter, or external runtime glue.
- `engine` and `net` may orchestrate domain contracts but must not become the semantic owner of gameplay, field-world, drawing, UI definition, or asset meaning.
- `apps` compose products and process wiring, but app-local state must not become a hidden domain source of truth.

### DDD

Use DDD to sharpen module placement:

- every new subsystem names its bounded context candidate;
- every boundary declares its ubiquitous vocabulary in owning docs or APIs;
- every cross-context interaction goes through translation contracts;
- shared crates are introduced only when at least two real consumers need the stable contract.

### ADRs

Create or update an ADR when a decision:

- changes dependency direction;
- changes which domain owns truth;
- accepts or rejects a long-term execution strategy;
- promotes a deferred architecture policy into implementation;
- replaces a migration strategy that previous docs relied on.

Keep roadmap score changes in [roadmap-decision-register.md](./roadmap-decision-register.md) unless they change one of those long-term decisions.

### Fitness Functions

The current enforcement stack already includes:

- `cargo validate`;
- `git diff --check`;
- the production documentation build;
- Rust architecture guard tests such as editor viewport and runtime boundary guards;
- focused crate tests for domain invariants.

Recommended next enforcement improvement: add a dependency-direction fitness function that reads `cargo metadata` and fails if a crate violates `guidelines/domain-map.md`. This should live as a tool or test before it is made part of the repository baseline.

### ATAM-lite

Use ATAM-lite for high-impact boundary choices, especially `B3` and `B4` items. The lightweight record should include:

- quality attributes under tension;
- candidate architecture options;
- sensitivity points;
- tradeoffs;
- risks;
- non-risks;
- decision or evidence needed next.

Do not run ATAM-lite for every small implementation task. Use it when a choice would be expensive to reverse or when reasonable options pull architecture in different directions.

### Strangler Fig

Use Strangler Fig for migration, not for greenfield modules.

Migration shape:

1. Freeze the old path behind a named compatibility boundary.
2. Add the new path beside it without deleting the old path.
3. Route one caller or product slice through the new path.
4. Add equivalence, parity, or source-marker guards.
5. Switch remaining callers.
6. Delete the old path and keep a regression guard.

This fits viewport replacement, render contract migration, product-job migration, UI surface routing, net replication hardening, and any future runtime/editor split.

### Team Topologies

Use Team Topologies as lightweight ownership labels:

| Ownership Mode | Runenwerk Use |
|---|---|
| Stream-aligned | Product surfaces such as editor workflows, Draw workflows, and runtime-preview flows. |
| Platform | Shared execution substrate, product jobs, query snapshots, publication barriers, diagnostics, and docs workflow. |
| Complicated subsystem | Render internals, ECS runtime internals, net replication, native tablet backends, and future physics/animation internals. |
| Enabling | Architecture audits, public API reviews, docs refactors, migration work, and fitness-function development. |

Use these labels to set expectations for collaboration and review, not to create process overhead.

## Current Recommendation

Adopt these methods as a governance addendum to the planning model:

- use Clean Architecture dependency direction and DDD ownership on every code placement decision;
- use ADRs only for durable architectural decisions;
- use fitness functions whenever a boundary is important enough to document;
- use ATAM-lite before promoting blocked architecture work;
- use Strangler Fig for replacement migrations;
- use Team Topologies labels to clarify who owns or supports a track.

This improves the current planning system without replacing the existing Runenwerk architecture doctrine.

## Workflow Use

Apply these governance lenses under the repository executor contract in [`AGENTS.md`](../../../../../AGENTS.md). Architectural or extraction work must inspect current code and accepted authority, compare alternatives, define ownership and migration, record durable decisions in the owning ADR or design, and validate the reviewed head before merge.

Do not create a separate routine, generated prompt, checklist database, or lifecycle state machine for architecture governance. The owning issue, accepted design or ADR, pull request, code, tests, and repository validation remain authoritative.

Design intake into the roadmap system is shown in [design-intake-roadmap-automation.puml](./diagrams/design-intake-roadmap-automation.puml).

## References

- [Clean Architecture dependency rule](https://blog.cleancoder.com/uncle-bob/2012/08/13/the-clean-architecture.html)
- [CMU SEI Architecture Tradeoff Analysis Method](https://www.sei.cmu.edu/library/the-architecture-tradeoff-analysis-method/)
- [Thoughtworks fitness function-driven development](https://www.thoughtworks.com/en-au/insights/articles/fitness-function-driven-development)
- [Martin Fowler Strangler Fig Application](https://martinfowler.com/bliki/StranglerFigApplication.html)
- [Atlassian Team Topologies summary](https://www.atlassian.com/devops/frameworks/team-topologies)
