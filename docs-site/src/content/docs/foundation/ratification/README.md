---
title: Ratification
description: Current public API and boundaries for the foundation ratification crate.
status: active
owner: foundation
layer: foundation
canonical: true
last_reviewed: 2026-09-09
related:
  - ../../design/implemented/foundation-ratification-design.md
  - ../../reports/closeouts/foundation-ratification-phase5-evaluation.md
---

# Ratification

`foundation/ratification` owns reusable candidate acceptance-report vocabulary.

It standardizes how a domain reports accepted, accepted-with-warnings, rejected,
or fatal candidate status. It does not own concrete validity rules, editor
history, undo/redo, reconciliation, command execution, runtime policy, or AI
behavior.

## Public API

Core entry points from `ratification::`:

- `RatificationStatus`
- `RatificationSeverity`
- `RatificationIssue<Code, Subject>`
- `RatificationReport<Code, Subject>`
- `Ratifier<Candidate>`

With the `diagnostics` feature, the crate also exposes:

- `RatificationDiagnosticMapper<Code, Subject>`
- `ratification_issue_to_diagnostic`
- `ratification_report_to_diagnostic_report`
- `ratification_severity_to_diagnostic_severity`

## Invariants

- Report status is derived from the highest issue severity.
- Empty reports are accepted.
- Warning-only reports are accepted with warnings.
- Error reports are rejected.
- Fatal reports are distinguishable from ordinary rejection.
- Issue codes and subjects are domain-owned generic values.

## Boundary

Ratifiers observe candidates and return reports. They must not mutate the
candidate or execute commands. Owning domains define the candidate type,
context, issue codes, subjects, and acceptance rules.
