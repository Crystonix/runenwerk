---
title: Architecture
description: Canonical top-down architecture spines for Runenwerk.
status: active
owner: workspace
layer: architecture
canonical: true
last_reviewed: 2026-09-11
---

# Architecture

This folder contains canonical top-down architecture spines.

Start with the Runenwerk-wide platform spine, then follow the owning subsystem or
framework architecture for the work at hand.

## Runenwerk-wide spine

- [Runenwerk Platform Architecture](./runenwerk-platform-architecture.md) — current
  integration, semantic-federation, specialized-execution, Workbench, and
  application-composition architecture.

Its durable decision basis is:

- [ADR 0014: Repository Family Extraction Boundaries](../adr/accepted/0014-repository-family-extraction-boundaries.md)
- [ADR 0017: Cross-Authority Consistency and Graph Semantics](../adr/accepted/0017-cross-authority-consistency-and-graph-semantics.md)
- [ADR 0018: Semantic Federation and Physical Realization](../adr/accepted/0018-semantic-federation-and-physical-realization.md)
- [ADR 0019: Batteries-Included Application Composition](../adr/accepted/0019-batteries-included-application-composition.md)

## Subsystem spines

- [Framework Integration Architecture](./repository-family-architecture.md)
- [Runenwerk UI Framework Architecture](./ui-framework-architecture.md)
- [Live UiPlugin Runtime Platform Architecture](./live-uiplugin-runtime-platform-architecture.md)

Active design documents may refine an owning subsystem or delivery slice, but they do
not override accepted ADRs or the current Runenwerk-wide spine.
