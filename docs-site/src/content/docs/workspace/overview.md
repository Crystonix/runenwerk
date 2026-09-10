---
title: Runenwerk Workspace
description: Short orientation map for workspace documentation.
status: active
owner: workspace
layer: workspace
canonical: true
last_reviewed: 2026-09-10
---

# Runenwerk Workspace

This page is a short orientation map.

## Current workspace navigation

- [Start Here](start-here.md)
- [Documentation Structure](documentation-structure.md)
- [Planning Records](planning/README.md)

## Top-level areas

- `foundation/`: shared low-level primitives.
- `domain/`: engine-agnostic contracts, models, and invariants.
- `engine/`: runtime app loop and plugins.
- `net/`: transport, session, replication, and replay crates.
- `apps/`: runnable applications and tools.
- `adapters/`: external runtime and host integrations.
- `assets/`: data assets consumed by runtime code.
- `docs-site/`: documentation source.

Physical workspace location is implementation evidence, not permanent semantic or
repository-family ownership. Use the current architecture spine and accepted ADRs for
cross-domain ownership decisions.

## Core references

- [Runenwerk Platform Architecture](../architecture/runenwerk-platform-architecture.md)
- [Architecture Boundaries](../guidelines/architecture.md)
- [Repository Family Architecture](../architecture/repository-family-architecture.md)
- [Programming Principles](../guidelines/programming-principles.md)
- [Domain Overview](../domain/00-overview.md)
- [Engine Docs](../engine/index.md)
- [Net Docs](../net/README.md)
