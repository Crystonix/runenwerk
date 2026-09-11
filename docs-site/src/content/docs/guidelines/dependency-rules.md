---
title: Dependency Rules
description: Canonical Runenwerk-local dependency direction, adapter boundaries, and framework-consumer rules.
status: active
owner: workspace
layer: guidelines
canonical: true
last_reviewed: 2026-09-11
related_docs:
  - ../architecture/repository-family-architecture.md
  - ./architecture.md
  - ../adr/accepted/0014-repository-family-extraction-boundaries.md
  - ../adr/accepted/0015-separate-gpu-execution-from-rendering.md
---

# Dependency Rules

## Purpose and authority

This document owns Runenwerk-local dependency direction and the dependency consequences
of integrating independently owned frameworks. It does not own the Dornglut repository
family or a framework's reusable semantics.

Cross-repository family membership, repository relationships, and source-authority
transfer procedure are owned by Dornglut Engineering:

- [Runen family architecture](https://github.com/dornglut/engineering/blob/main/architecture/runen-family.md)
- [Repository standard](https://github.com/dornglut/engineering/blob/main/standards/repositories.md)
- [ADR 0008: bounded source-authority handoffs](https://github.com/dornglut/engineering/blob/main/adrs/0008-adopt-bounded-source-authority-handoffs.md)

Each standalone framework repository owns its reusable semantics, public dependency
contract, conformance, validation, release policy, and repository-local architecture.
Runenwerk consumes those public contracts; it does not redefine them here.

For Runenwerk's adapter, compatibility, recovery, and product-integration contracts,
see [Framework Integration Architecture](../architecture/repository-family-architecture.md).

## Runenwerk layer direction

Runenwerk-local code follows this dependency direction:

```text
foundation -> domain -> engine/runtime -> apps/adapters/tools
```

- Foundation contains low-level reusable vocabulary and may depend only on justified
  lower-level external libraries or other foundation contracts. It does not depend on
  domain, runtime, editor, app, adapter, workflow, UI-framework, or concrete-backend
  code.
- Domain code may depend on foundation and justified lower-level domain contracts, but
  not on engine/runtime, app wiring, or concrete backends it does not own.
- Engine/runtime composes domains and accepted peer-framework public contracts without
  moving their semantic ownership into Runenwerk runtime APIs.
- Apps, adapters, and tools may compose higher-level systems but do not define reusable
  framework or domain invariants merely because they integrate them.

The current local workspace inventory is owned by
[`../workspace/crate-inventory.md`](../workspace/crate-inventory.md). Physical source
location is implementation evidence, not permission to reverse these dependency rules.

## Peer-framework consumption

The local integration direction is:

```text
standalone framework public contract
              |
              v
      Runenwerk adapter/integration
              |
              v
      Runenwerk application/tool
```

A standalone framework must not depend on Runenwerk merely to participate in product
integration. A Runenwerk dependency on a framework records product adoption of that
framework's public contract; it does not transfer reusable semantic ownership into
Runenwerk.

Current family membership and repository-to-repository relationships are read from
Engineering's Runen-family architecture rather than copied into this guideline.

## Adapter rules

A Runenwerk adapter may depend on the public contracts it translates between and on the
Runenwerk integration surface that owns the product translation.

Adapters may explicitly translate:

- identities and correspondence where required;
- prepared inputs and outputs;
- lifecycle and generation facts;
- diagnostics and provenance;
- ownership or availability facts;
- Runenwerk product requirements into accepted framework requests.

Adapters must not:

- reach through private framework implementation;
- duplicate framework algorithms or semantic validation;
- mirror authoritative source or create a writable shadow implementation;
- preserve predecessor shape through broad compatibility facades without a demonstrated
  current consumer and removal condition;
- hide dependency cycles;
- make a framework depend back on Runenwerk;
- introduce a universal shared core merely because several adapters use similar
  vocabulary.

## Consumer revision policy

Runenwerk owns the admission and compatibility claim for the exact framework revisions
it integrates. Before stable publication, maintained framework dependencies used for a
claimed integration are pinned to an exact accepted Git revision or an exact accepted
pre-release/version. A moving branch is not a compatibility contract.

Changing an integrated framework revision is a Runenwerk consumer change and must prove
that the affected adapters, product behavior, and integration evidence remain valid.
That admission decision does not become framework release or compatibility policy.

When a dependency change is part of a cross-repository source-authority handoff,
Engineering ADR 0008 owns the handoff sequence and accepted-successor pinning rules.
This document does not duplicate that procedure.

## Completed handoff consequences

After Runenwerk accepts a framework consumer cutover, its local dependency and adapter
state must agree with the accepted semantic owner. Runenwerk does not retain a writable
predecessor implementation, forwarding namespace, source mirror, private reach-through,
or second runtime path as parallel authority.

If Runenwerk has no real consumer for a newly externalized capability, it need not add an
unused dependency merely because a standalone repository exists.

## Boundary escalation

When Runenwerk integration needs something that an owning framework does not expose,
identify the missing semantic owner before changing dependencies:

- a reusable public value, command, diagnostic, capability, workload, contribution, or
  test-support contract belongs in the owning framework under its accepted authority;
- Runenwerk-specific product translation, composition, admission, presentation, or
  recovery policy belongs in Runenwerk integration or adapters.

Do not solve boundary pressure with private reach-through, a universal shared core,
speculative packages, exposed mutable internals, or a compatibility facade that becomes
a second semantic authority.

## Non-authority

This page does not own or inventory:

- Dornglut repository-family membership or topology;
- standalone framework mission or package definitions;
- framework release, validation, or conformance policy;
- organization-wide source extraction or transfer procedure;
- live migration, branch, issue, or cutover state.

Use the owning Engineering and standalone-framework authorities for those concerns.
