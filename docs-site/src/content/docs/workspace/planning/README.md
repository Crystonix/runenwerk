---
title: Planning Records
description: Durable sequencing guidance for Runenwerk without duplicating GitHub live state.
status: active
owner: workspace
layer: workspace
canonical: true
last_reviewed: 2026-09-10
related_docs:
  - ../documentation-structure.md
  - ./roadmap.md
  - ./decision-register.md
---

# Planning Records

Runenwerk keeps planning readable without creating a second work-management system.

## Authority split

```text
GitHub issues / Engineering Portfolio
  live task state, priority, scope, owner, blockers, and activation

accepted ADR or design
  durable architecture, ownership, dependency direction, and migration contracts

roadmap
  durable high-level sequence and dependencies only

pull request
  proposed diff, review findings, exact-head validation, and merge evidence

code and tests
  current behavior

reports and Git history
  historical evidence and chronology
```

Planning Markdown must not mirror live issue state, completed chronology, deferred
queues, branches, workflow runs, or delivery evidence.

## Maintained planning surfaces

- [Roadmap](roadmap.md) — the canonical workspace sequencing page. It contains only
  durable phase ordering, dependency direction, extraction direction, and cross-family
  sequencing constraints.
- [Decision Register](decision-register.md) — navigation to durable accepted decisions;
  it is not a live decision or task database.

There is intentionally no separate Markdown active, deferred, or completion ledger.
GitHub owns live and deferred work state; reports, pull requests, accepted documents,
and Git history preserve completed evidence.

## Planning rules

1. Give each live task one owning GitHub issue.
2. Keep the roadmap at durable milestone/phase and dependency level; do not enumerate
   branch execution state or current issue status.
3. Put architecture and public-contract decisions in accepted ADRs or designs.
4. Keep delivery evidence in pull requests and repository validation, not in planning
   summaries.
5. Keep chronology and historical evidence in reports, closeouts, pull requests, and Git
   history rather than maintaining a completion mirror.
6. Keep intentionally postponed work in its owning GitHub issue/portfolio state rather
   than maintaining a separate deferral mirror.
7. Cross-link the true owner instead of duplicating its content.
8. Do not introduce generated planning databases, execution ledgers, truth certificates,
   or compatibility planning pages without a proven consumer and explicit removal
   condition.

## Updating planning truth

When work changes:

- update the owning issue or Engineering Portfolio for live state;
- update the roadmap only when durable sequence or dependency truth changes;
- update an accepted ADR/design when durable architecture changes;
- record delivery and exact-head evidence in the pull request;
- retain historical evidence in its report/closeout or Git history.

A phase specification may provide a bounded implementation handoff when accepted
authority requires one. It remains subordinate to accepted Markdown and the owning
GitHub issue and does not own lifecycle, activation, current base/head, CI, or delivery
state.

## Review and validation

Planning changes are reviewed against the owning issue, accepted architecture, current
code where behavior claims are made, and the authority owners named in [Workspace Start Here](../start-here.md).

Run the documentation checks required by [TESTING.md](../../../../../../TESTING.md).
Report inspection as inspection and executable validation as validation.
