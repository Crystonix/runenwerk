---
title: Foundation Ratification Phase 5 Evaluation
description: Evaluation outcome for editor_core governance precondition reports and diagnostics completion before schema work.
status: completed
owner: foundation
layer: reports
canonical: true
last_reviewed: 2026-09-09
related_adrs: []
related_docs:
  - ../../design/implemented/foundation-ratification-design.md
  - ../../foundation/ratification/README.md
  - ../../design/implemented/foundation-schema-design.md
---

# Foundation Ratification Phase 5 Evaluation

## Purpose

This document closes Phase 5 of the initial `foundation/ratification` migration.

Phase 5 asked whether `editor_core` governance should adopt optional precondition reports from `foundation/ratification`, without moving `RatifiedChange` into foundation.

It also defines the diagnostics completion gate that must be satisfied before starting `foundation/schema` or `foundation/commands`.

## Decision

Do not change `editor_core` governance yet.

Do not move `RatifiedChange` into `foundation/ratification`.

Do not replace `GoverningChangeError`.

Do not replace the existing editor runtime governing-change ingress.

The current split is correct:

```text
foundation/ratification
  shared report/status/issue/ratifier vocabulary

foundation/diagnostics
  shared diagnostic reporting vocabulary

editor_core
  editor governance, RatifiedChange, transactions, history-facing metadata, sharing/reconciliation vocabulary

apps/runenwerk_editor
  runtime orchestration and governing-change ingress

owning domains
  concrete ratification issue codes, subjects, candidates, and validity rules
```

## Rationale

`foundation/ratification` is a vocabulary crate. It standardizes how acceptance reports are shaped.

`editor_core::RatifiedChange` is not just an acceptance report. It is an editor governance artifact that contains transaction metadata, causality, origin, authority scope, semantic operations, versions, retention, reversibility, sharing, reconciliation, and propagation policy.

Moving it into foundation would leak editor governance into the foundation layer.

Replacing `GoverningChangeError` would also be premature. Governance still needs typed control-flow errors for mutation, history, sharing, reconciliation, and runtime orchestration. Ratification reports and diagnostics can enrich reporting, but they must not erase typed control flow.

## Current Migration State

Complete for the current milestone:

```text
Phase 1: foundation/ratification generic report types and tests
Phase 2: ui_surface mounted-surface candidate ratifier
Phase 3: editor_shell projection/route ratification using foundation reports
Phase 4: ratification-to-diagnostics bridge
Phase 5: editor_core governance evaluation and diagnostics completion gate
```

## Diagnostics Completion Gate

Diagnostics must be complete as a foundation crate before schema and commands begin.

Required before moving on:

```text
foundation/diagnostics compiles without warnings
foundation/diagnostics --features serde compiles without warnings
foundation/ratification compiles without warnings
foundation/ratification --features diagnostics compiles without warnings
foundation/ratification --features serde,diagnostics compiles without warnings
root docs list current foundation crates
design docs list ratification as active/current
docs validator passes
```

Not required before moving on:

```text
rewrite every validation error as diagnostics
wire diagnostics into editor UI
wire diagnostics into tracing/logging
add a global diagnostic registry
replace editor_core governance with diagnostics
replace all typed domain errors
```

## Required Validation

Run:

```text
cargo fmt --all -- --check
cargo test -p diagnostics
cargo test -p diagnostics --features serde
cargo test -p ratification
cargo test -p ratification --features diagnostics
cargo test -p ratification --features serde,diagnostics
python3 tools/docs/validate_docs.py
```

Recommended final gate:

```text
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

## When To Revisit Editor Governance Integration

Revisit optional precondition reports only when there is real duplication or friction in at least two of these areas:

```text
editor_core governance preconditions
apps/runenwerk_editor runtime mutation ingress
editor_scene command validation
editor_persistence migration/import validation
shared-change reconciliation
AI/tool command proposal reporting
```

A future integration may add optional report fields or conversion helpers, but should preserve this rule:

```text
Ratification reports explain candidate acceptance.
RatifiedChange records accepted editor governance facts.
GoverningChangeError remains typed control flow.
Diagnostics explain observed issues.
```

## Explicit Non-Decision

This evaluation does not decide the future shape of `foundation/schema` or `foundation/commands`.

It only closes the initial ratification migration enough to move to schema design after diagnostics is clean.

## Next Step

After this document and the diagnostics completion gate are green, start the design for:

```text
docs-site/src/content/docs/design/implemented/foundation-schema-design.md
```

Then design:

```text
docs-site/src/content/docs/design/implemented/foundation-commands-design.md
```

Do not implement either crate before the design doc is reviewed against current repository boundaries.
