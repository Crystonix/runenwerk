---
title: Documentation Structure
description: Canonical placement and authority rules for Runenwerk documentation.
status: active
owner: workspace
layer: workspace
canonical: true
last_reviewed: 2026-09-11
related_docs:
  - ./start-here.md
  - ./engineering-workflow.md
  - ./authority-model.md
---

# Documentation Structure

Runenwerk uses two documentation levels:

```text
repository root
  concise public and contributor entrypoints

docs-site/src/content/docs
  canonical long-form architecture, design, planning, and history
```

When they overlap, the docs-site document owns the detail. Root files summarize and link; they do not duplicate full policy or design.

## Root entrypoints

The root keeps:

```text
README.md
AGENTS.md
ARCHITECTURE.md
TESTING.md
```

These are the primary public and contributor entrypoints. Detailed dependency guidance
lives directly in the canonical docs-site owner rather than through a root forwarding
summary.

Crate inventory, code-placement guidance, dependency rules, and shared vocabulary live
directly in their canonical docs-site owners:

- [`crate-inventory.md`](./crate-inventory.md) for active local workspace membership;
- [`../guidelines/architecture.md`](../guidelines/architecture.md) for Runenwerk placement and boundary guidance;
- [`../guidelines/dependency-rules.md`](../guidelines/dependency-rules.md) for Runenwerk-local dependency direction, adapter boundaries, and framework-consumer rules;
- [`glossary.md`](./glossary.md) for shared vocabulary.

Root documents must not become roadmaps, design dossiers, execution ledgers, or duplicated reference manuals.

## Canonical tree

```text
docs-site/src/content/docs/
  workspace/     process, planning, repository inventories, glossary
  guidelines/    stable engineering and dependency rules
  architecture/  current and target system structure
  adr/           durable decisions and rejected alternatives
  design/        target contracts and migration plans
  foundation/    foundation-specific documentation
  domain/        domain-specific documentation
  apps/          application documentation
  net/           networking documentation
  adapters/      integration and host adapters
  reports/       investigations, proofs, closeouts, benchmarks
  archive/       non-authoritative historical material
```

## Document responsibilities

- **Guideline:** stable engineering rule or doctrine.
- **ADR:** durable decision, alternatives, and consequences.
- **Architecture:** repository or subsystem ownership and dependency structure.
- **Design:** target behavior, public vocabulary, boundaries, and migration.
- **Investigation report:** source-grounded point-in-time reality and unresolved findings.
- **Roadmap:** durable high-level sequence and dependencies; never an execution or live-state ledger.
- **GitHub issue / Engineering Portfolio:** proposed, active, blocked, deferred, completed, and prioritized work state.
- **Closeout report:** historical completion evidence when a PR and issue are not enough.
- **Archive:** superseded or historical context that does not authorize new work.

## Placement rules

1. Put one durable decision in one ADR or accepted design.
2. Put one active or deferred task in one GitHub issue and use the Engineering Portfolio for live priority/status.
3. Put durable high-level sequence and dependencies in the maintained roadmap.
4. Put current behavior in code and tests.
5. Put delivery evidence in the pull request.
6. Cross-link instead of copying full state.
7. Move obsolete active documentation to reports or archive; do not preserve it as a parallel workflow.

## Naming

- Use kebab-case for docs-site Markdown files.
- Use `README.md` for section landing pages.
- Keep names literal and searchable.
- Prefer ownership-oriented names for architecture and task-oriented names for procedures.

When pruning or moving documents, update current inbound links. Historical reports may preserve point-in-time provenance, but they do not authorize current work.
