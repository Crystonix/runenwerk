---
title: Runenwerk Programming Principles
description: Practical principles for long-lived architecture, documentation, and code review.
status: active
owner: workspace
layer: guidelines
canonical: true
last_reviewed: 2026-07-22
related_docs:
  - ./architecture.md
  - ./code-patterns.md
  - ./dependency-rules.md
  - ../workspace/engineering-workflow.md
---

# Runenwerk Programming Principles

Use these principles as review lenses. They do not replace domain ownership, dependency direction, accepted ADRs, tests, or validation. Project-neutral software-design defaults are owned by [Dornglut Engineering's Software Design Standard](https://github.com/dornglut/engineering/blob/main/standards/software-design.md); this page retains the Runenwerk-specific review lens.

## KISS

Prefer the smallest readable design that protects the invariant. Simple means the owner, contract, failure behavior, and validation path are obvious—not that important semantics are omitted.

Reject hidden global state, clever routing, and process indirection that make the implementation harder to explain.

## DRY

Keep one authority for each durable claim. Root entrypoints may summarize canonical docs briefly, but they must link to the owner rather than reproduce it.

Duplicate algorithms, source mirrors, planning databases, compatibility namespaces, and parallel validation commands require an explicit temporary migration need and removal condition.

## YAGNI

Do not add crates, commands, registries, extension points, generic APIs, workflow layers, or compatibility surfaces without a current owner and concrete use.

Future-proofing means leaving clean boundaries, not pre-building unused machinery.

## SOLID

Apply SOLID as a boundary check:

- one owner should have one clear reason to change;
- extension should use owned contracts rather than unrelated internals;
- substitutions must preserve documented behavior;
- interfaces should be narrow and purpose-specific;
- stable policy should not depend on outer runtime, app, adapter, or tooling details.

## Separation of concerns

Separate domain meaning, runtime execution, app wiring, adapters, documentation, tests, planning, and historical evidence. A file or module should make its responsibility clear.

## Avoid premature optimization

Do not optimize architecture, APIs, validation, or documentation for imagined scale. Start with clear contracts and focused evidence; optimize after a measured or bounded pressure is established.

## Law of Demeter

Depend on the direct owner or an explicit public contract. Avoid reaching through another subsystem's internals or making callers understand a transitive implementation chain.

## Review questions

For non-trivial work, ask:

- Is the path direct and understandable?
- Is each durable claim owned once?
- Is every new surface needed by a current consumer?
- Does each owner have a coherent responsibility and legal dependencies?
- Are domain, runtime, product, adapter, test, planning, and history concerns separated?
- Is complexity supported by evidence?
- Do callers use direct contracts rather than internals?

A finding is merge-critical when it affects correctness, ownership, lifecycle, public API, persistence, dependency direction, validation, or maintainability. Record the concrete finding and resolution; do not require a universal matrix or separate principle report for every change.
