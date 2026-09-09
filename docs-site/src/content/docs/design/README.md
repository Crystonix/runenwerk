---
title: Design Documents
description: Architecture design documents for active, accepted, deferred, and superseded Runenwerk design work.
status: active
owner: workspace
layer: workspace
canonical: true
last_reviewed: 2026-09-09
---

# Design Documents

## Purpose

This folder contains architecture design documents for Runenwerk.

Design documents define intended architecture, ownership boundaries, invariants, tradeoffs, migration paths, and validation expectations before or alongside implementation.

A design document is not an ADR, not a roadmap, and not a report.

## Folder Structure

```text
content/docs/design/
├── README.md
├── active/
├── accepted/
├── implemented/
├── deferred/
├── superseded/
├── rejected/
├── archived/
└── templates/
```

## `active/`

Use `active/` for designs that are currently being discussed, implemented, or validated.

A design belongs here when the direction is useful but not yet fully accepted or not yet checked against implementation.

Index:

- [`active/README.md`](active/README.md)

Examples:

```text
active/editor-ui-workspace-tool-surface-architecture.md
active/workspace-viewport-expression-upgrade-design.md
```

## `accepted/`

Use `accepted/` for designs whose direction is approved.

A design may be accepted before it is fully implemented.

Index:

- [`accepted/README.md`](accepted/README.md)

Examples:

```text
accepted/foundation-diagnostics-design.md
accepted/workspace-ai-friendly-engine-architecture.md
```

## `deferred/`

Use `deferred/` for designs that are valid but intentionally postponed.

Deferred designs should explain why they are not being implemented now and what would reactivate them.

Index:

- [`deferred/README.md`](deferred/README.md)

## `superseded/`

Use `superseded/` for designs that have been replaced.

A superseded design must link to the replacement design, ADR, or guideline.

Index:

- [`superseded/README.md`](superseded/README.md)

## `implemented/`

Use `implemented/` for accepted designs that have been checked against actual code.

A design belongs here only when the implementation exists, tests pass, and any known divergence from the accepted design has been resolved or documented.

Index:

- [`implemented/README.md`](implemented/README.md)

Examples:

```text
implemented/foundation-ratification-design.md
```

## `rejected/`

Use `rejected/` for designs that were explicitly considered and not chosen.

Rejected designs should explain the rejected approach, why it was rejected, and what design, ADR, or guideline should be followed instead.

Index:

- [`rejected/README.md`](rejected/README.md)

## `archived/`

Use `archived/` for historical or imported design material that is no longer authoritative and does not fit `rejected/` or `superseded/`.

Archived designs must link to the replacement document or explain why no replacement exists.

Index:

- [`archived/README.md`](archived/README.md)

## `templates/`

Use `templates/` for reusable design document templates.

Examples:

```text
templates/crate-design-template.md
```

## Design Document Requirements

A design document should normally include:

```text
Purpose
Scope
Non-scope
Architectural position
Ownership rules
Dependency rules
Public API policy
Invariants
Failure modes
Extension points
Testing strategy
Migration plan
Validation plan
```

Not every design needs every section, but missing sections should be intentional.

## Status Rules

Use frontmatter status values consistently:

```text
active
accepted
implemented
deferred
superseded
rejected
archived
```

A design can move through:

```text
active -> accepted -> implemented
```

A design can also become:

```text
deferred
superseded
rejected
archived
```

## Promotion Rules

Move a design to `accepted/` only when the architectural direction is approved.

Mark a design as `implemented` only when the code has been checked against the design.

Move a design to `superseded/` when another design, ADR, or guideline replaces it.

Move a design to `deferred/` when the idea remains valid but is intentionally postponed.

## Relationship to ADRs

ADRs record decisions.

Design documents explain architecture.

If a design creates a long-term architectural rule, create or update an ADR.

If an ADR and a design conflict, the ADR wins.

## Relationship to Roadmaps

Roadmaps define implementation sequence.

Design documents define target architecture.

A roadmap should link to the design it implements rather than restating the design.

## Naming Rules

Use kebab-case filenames.

Preferred:

```text
foundation-diagnostics-design.md
foundation-ratification-design.md
crate-design-template.md
```

Use domain-first prefixes so design indexes can be grouped predictably.

Recommended prefixes:

```text
foundation-
editor-
ui-
engine-
net-
apps-
workspace-
runenwerk-
domain-<subdomain>-
```

When a design spans multiple areas, prefer `workspace-` or `runenwerk-` instead of an ambiguous single-domain prefix.

Use an intent suffix that matches the document's role.

Recommended suffixes:

```text
-design.md
-architecture.md
-migration-map.md
-evaluation.md
-preserved-target-draft.md
```

Avoid:

```text
foundation_diagnostics.md
FoundationDiagnostics.md
design.md
notes.md
editor.md
ui.md
```

## Negative Doctrine

Do not put temporary task notes here.

Do not put benchmark reports here.

Do not put completed phase evidence here.

Do not use design documents as roadmaps.

Do not leave accepted designs mixed with active design drafts.
