---
title: Superseded Runenwerk Capability Workbench Target Architecture
description: Supersession record for the former Capability Cells, Platform Planes, and Maturity Layers Runenwerk-wide target.
status: superseded
owner: workspace
layer: history
canonical: false
last_reviewed: 2026-09-10
replacement_docs:
  - ../../architecture/runenwerk-platform-architecture.md
  - ../../adr/accepted/0018-semantic-federation-and-physical-realization.md
  - ../../adr/accepted/0019-batteries-included-application-composition.md
  - ../implemented/editor-tool-suite-registry-and-workbench-host-design.md
---

# Superseded Runenwerk Capability Workbench Target Architecture

This document no longer defines a Runenwerk-wide future target.

The platform-wide Workbench direction is governed by the
[Runenwerk Platform Architecture](../../architecture/runenwerk-platform-architecture.md)
and [ADR 0018](../../adr/accepted/0018-semantic-federation-and-physical-realization.md):
independent semantic owners may expose optional owner-selected inspection projections
to a federated Workbench without adopting a universal Workbench ontology, registry,
identity system, or execution runtime.

The former broad vocabulary is not current platform doctrine:

```text
Capability Cells
Platform Planes
Maturity Layers
family-wide capability-cell identities
a Runenwerk-wide capability registry/policy-plane ontology
```

The useful concrete editor/tool-host direction has a narrower implemented owner in
[Editor Tool Suite Registry And Workbench Host Design](../implemented/editor-tool-suite-registry-and-workbench-host-design.md),
which keeps editor-local tool-suite/workbench composition, provider routing, app-owned integration,
and domain-owned semantics explicit while remaining subordinate to current app-neutral structural composition authority.

Application-level batteries-included composition is separately governed by
[ADR 0019](../../adr/accepted/0019-batteries-included-application-composition.md).

This record remains only as historical design evidence. New work must follow the
replacement authority above.
