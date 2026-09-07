---
title: Engineering Workflow
description: Canonical risk-scaled workflow and validation authority for Runenwerk.
status: active
owner: workspace
layer: workspace
canonical: true
last_reviewed: 2026-08-21
related_docs:
  - ./start-here.md
  - ./authority-model.md
  - ./documentation-structure.md
---

# Engineering Workflow

GitHub issues and pull requests manage work. Accepted ADRs and designs own durable architecture. Code and tests own current behavior. `cargo validate` and exact-head CI own the required validation baseline. Pull-request workflows validate the explicitly selected reviewed feature head; push and dispatch use `github.sha`. A workflow definition may be loaded from a synthetic merge ref while checked-out repository contents prove a distinct revision.

Repository scripts may automate deterministic checks. They do not grant permission, certify truth, manage lifecycle state, or author feature-branch changes.

## Core rules

1. Identify the owner and boundary before changing shared behavior.
2. Inspect current code, tests, and accepted authority instead of relying on memory.
3. Scale investigation and documentation to the risk of the change.
4. Keep one source of authority for each decision or live state.
5. Prefer one coherent implementation slice over mixed ownership.
6. Do not preserve obsolete paths without a real compatibility need and removal condition.
7. Report only validation and runtime behavior that was actually observed.

## Work classes

### Routine

Local fixes and behavior-preserving refactors with a clear owner and unchanged dependency direction. Record the problem, change, focused checks, and baseline status. A separate design is not required.

### Significant

Cross-module behavior, public surfaces, persistence, host integration, or major product paths. Record the owner, affected boundaries, important alternatives, acceptance criteria, migration impact, and validation in one suitable issue, design, ADR, or PR body.

### Architectural or extraction

New repositories or crates, reusable platform contracts, public APIs, dependency-direction changes, renderer/host boundaries, and authority changes. Before implementation, establish:

- current reality grounded in code and tests;
- target ownership and dependency direction;
- important alternatives and trade-offs;
- migration, cutover, deletion, and conformance requirements;
- explicit non-owned responsibilities.

The target must be complete enough to prevent ownership ambiguity. Delivery should still use bounded implementation PRs.

## Standard flow

```text
issue or accepted authority when needed
-> implementation branch
-> focused checks
-> cargo validate
-> pull request
-> review at the exact head
-> squash merge
-> delete the branch
```

A separate planning or closeout PR is justified only when it carries independently reviewable information. Do not create process-only PRs, activation PRs, generated prompts, or temporary authoring workflows by default.

## Validation

Use focused package checks while implementing. Every merge requires:

```text
cargo validate
```

`cargo validate` is the same read-only, lockfile-safe implementation used by GitHub Actions. It validates the repository tooling, workspace formatting, locked tests, strict Clippy, documentation structure, and durable repository invariants.

Documentation changes additionally run the Astro/Starlight production build through the path-scoped documentation workflow. It is supplemental evidence, not a second Rust baseline. Success output is compact; failures preserve status, print bounded diagnostics, retain a short-lived complete artifact, and clean temporary state.

RunenGPU implementation and framework conformance are owned by the external
`dornglut/runen-gpu` repository after its accepted exact-revision handoff. Runenwerk
validates the consumer cutover, public-API integration, and product/runtime behavior
through its own locked baseline and focused integration tests; it does not duplicate
the framework's standalone conformance workflow.

Broader tools such as cargo-deny, cargo-machete, Lychee, ast-grep, benchmarks, or platform matrices are run directly when the affected change or release needs them. They are not a second named Rust gate.

## Pull-request review

Before merge, verify:

- the diff matches the issue or accepted authority;
- no unrelated owner or behavior is changed;
- acceptance criteria and required tests are satisfied;
- public API, persistence, and dependency impacts are explicit;
- `cargo validate` and required workflows pass at the reviewed head;
- no unresolved correctness or ownership finding remains;
- post-merge documentation and planning state are truthful.

## Evidence language

Distinguish exact-head CI, local command output, user-reported output, source inspection, and unverified claims. Inspection can establish structure; it cannot be reported as executed validation or runtime proof.

## Authority

- Code and tests own current behavior.
- Accepted ADRs and designs own durable decisions.
- GitHub issues own active work.
- The roadmap owns high-level sequencing, not execution details.
- Pull requests own delivery and review evidence.
- Historical reports and superseded documents provide context only.

## Stop conditions

Stop before implementation or merge when the owner, dependency direction, public or persistence impact, migration/deletion path, acceptance criteria, required validation, or exact-head CI status remains materially unknown.
