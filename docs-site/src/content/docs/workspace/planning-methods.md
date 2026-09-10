---
title: Roadmap Planning Methods
description: Workspace planning methods for dependency topology, roadmap scorecards, product discovery, and risk tracking.
status: active
owner: workspace
layer: workspace
canonical: true
last_reviewed: 2026-09-10
related:
  - ./architecture-governance-review.md
  - ./design-implementation-triage.md
  - ./roadmap-items.yaml
  - ./roadmap-archive.yaml
  - ./roadmap-deferred.yaml
  - ./roadmap-decision-register.md
  - ./repo-execution-priority-checklist.md
  - ./roadmap-index.md
  - ./diagrams/value-weighted-dependency-roadmap.puml
---

# Roadmap Planning Methods

## Purpose

This page defines the workspace planning model used to compare roadmap work without flattening architecture gates into a single priority number.

The long-term rule is: topology first, gate second, score third. A score can rank comparable work, but it must not override an owning roadmap, a dependency level, or a blocker gate.

## Canonical Stack

| Method | Use For | Do Not Use For |
|---|---|---|
| Layered PDM / Activity-on-Node roadmap | Dependency topology, parallelizable work, sequencing gates. | Estimating product value by itself. |
| Architecture-adjusted WSJF | Ranking comparable roadmap candidates inside the same gate and dependency level. | Skipping dependency order or treating speculative estimates as facts. |
| RICE | User-facing or product-facing work with meaningful reach evidence. | Architecture substrate, internal contracts, or work with no credible reach estimate. |
| Opportunity Solution Tree | Discovery before choosing product/user-facing solutions. | Replacing implementation sequencing after a solution is selected. |
| Kano | Classifying user-facing experience work as basic, performance, or delight. | Ranking architecture substrate that users do not experience directly. |
| RAID | Tracking risks, assumptions, issues, and dependencies that change confidence or gates. | Producing a priority order alone. |
| Decision register | Recording why the current call is what it is. | Detailed phase sequencing owned by domain roadmaps. |
| Dependency structure matrix | Auditing dense cross-domain dependencies when the PDM graph becomes hard to read. | Day-to-day workspace triage unless the dependency graph becomes tangled. |
| Clean Architecture dependency rule | Checking that source dependencies point toward stable policy/domain contracts. | Replacing Runenwerk's domain/reality doctrine with generic folder layers. |
| DDD bounded contexts | Choosing module and crate boundaries around ownership, vocabulary, and invariants. | Creating premature shared abstractions before a boundary has real consumers. |
| ADRs | Capturing durable architecture decisions, rejected alternatives, and consequences. | Recording transient score changes or implementation todos. |
| Architecture fitness functions | Enforcing boundaries through tests, metadata checks, docs validation, and gates. | Prose-only architecture governance. |
| ATAM-lite | Reviewing quality-attribute tradeoffs before expensive architecture choices. | Routine small implementation tasks. |
| Strangler Fig migration | Replacing old paths while old and new paths coexist under guards. | Greenfield features with no existing path to retire. |
| Team Topologies labels | Clarifying ownership mode and collaboration shape for roadmap work. | Inventing a human org chart inside repository docs. |

MoSCoW can be meeting shorthand, but it is not the canonical roadmap model for this repository. It hides dependency order, effort, confidence, and blocker state, which are the main planning risks here.

## Architecture-Adjusted WSJF

Use architecture-adjusted WSJF as the primary score in the workspace decision register:

```text
A-WSJF = ((V + TC + RR/OE + DU) * C) / E
```

Fields:

| Field | Meaning | Scale |
|---|---|---|
| `V` | Value weight. Reuses the `V1` to `V5` model from implementation triage. | 1 to 5 |
| `TC` | Time criticality. Measures how much sequencing value decays if delayed. | 1 to 5 |
| `RR/OE` | Risk reduction or opportunity enablement. Measures how much uncertainty or future option cost is reduced. | 1 to 5 |
| `DU` | Dependency unlock. Measures how many important downstream tracks become easier or possible. | 1 to 5 |
| `E` | Relative effort to the next usable milestone, not the whole dream feature. | 1, 2, 3, 5, 8, 13 |
| `C` | Confidence in the estimates and evidence. | 1.0, 0.8, 0.5, 0.3 |

Interpretation:

- `B1` and `B2` items can be implementation candidates.
- `B3` and `B4` items can be discovery, proof, or contract candidates.
- `B5` items remain policy-deferred even when a speculative score looks high.
- Scores compare work inside a lane and dependency level first.

## RICE

Use RICE only when reach is meaningful:

```text
RICE = (Reach * Impact * Confidence) / Effort
```

For this repository, most architecture substrate work should keep `RICE` as `N/A`. A RICE score becomes appropriate when the roadmap item has a concrete user path, target workflow, or adoption population. Good candidates include editor UX flows, drawing workflows, tablet input, and multi-window presentation after their product target is clear.

## Discovery And Risk Tools

Use an Opportunity Solution Tree before committing product-facing work whose problem statement is still weak. The tree should connect one outcome to opportunities, candidate solutions, and assumption tests.

Use Kano for user-facing experience classification:

| Class | Meaning |
|---|---|
| Basic | Users expect it; absence damages trust. |
| Performance | Better execution proportionally improves perceived quality. |
| Exciter | Delightful if present, acceptable if absent. |
| Neutral | Not a user-facing differentiator now. |

Use RAID notes when a score is hard to trust. A high-value item with weak confidence should usually produce an evidence task before implementation.

## Architecture Governance Methods

Use [Architecture Governance Review](./architecture-governance-review.md) when a roadmap item changes ownership, dependency direction, public contracts, or migration policy. Execute accepted work under the repository contract in [`AGENTS.md`](../../../../../AGENTS.md) and the current organization-wide Engineering authority.

The critical adoption choice is selective use:

| Method | Runenwerk Rule |
|---|---|
| Clean Architecture | Use the dependency rule: domain policy and stable contracts do not depend on app, adapter, framework, transport, persistence, or backend details. |
| DDD | Use bounded contexts for module boundaries: owner, vocabulary, invariants, and translation contracts come before file placement. |
| ADRs | Use ADRs for durable architecture decisions that should survive beyond one roadmap phase. |
| Fitness functions | If a boundary is important enough to document, prefer making it testable through validation, architecture guard tests, cargo metadata checks, or CI gates. |
| ATAM-lite | Use before promoting `B3` or `B4` architecture work when quality attributes conflict. |
| Strangler Fig | Use for migrations that replace an old path while old and new paths temporarily coexist. |
| Team Topologies | Use lightweight ownership labels: stream-aligned, platform, complicated subsystem, or enabling. |

## Operating Rule

When choosing the next item:

1. Respect completed and active owning roadmaps.
2. Remove `B5` items unless policy has changed.
3. Pick the highest dependency level that is unblocked.
4. Compare items inside that level by lane and A-WSJF.
5. If scores conflict with engineering judgment, update the evidence and the decision register instead of silently ignoring the score.

External references:

- [SAFe Weighted Shortest Job First](https://framework.scaledagile.com/wsjf)
- [Intercom RICE prioritization](https://www.intercom.com/blog/rice-simple-prioritization-for-product-managers/)
- [Product Talk Opportunity Solution Trees](https://www.producttalk.org/opportunity-solution-trees/)
- [Clean Architecture dependency rule](https://blog.cleancoder.com/uncle-bob/2012/08/13/the-clean-architecture.html)
- [CMU SEI Architecture Tradeoff Analysis Method](https://www.sei.cmu.edu/library/the-architecture-tradeoff-analysis-method/)
- [Thoughtworks fitness function-driven development](https://www.thoughtworks.com/en-au/insights/articles/fitness-function-driven-development)
- [Martin Fowler Strangler Fig Application](https://martinfowler.com/bliki/StranglerFigApplication.html)
- [Atlassian Team Topologies summary](https://www.atlassian.com/devops/frameworks/team-topologies)
