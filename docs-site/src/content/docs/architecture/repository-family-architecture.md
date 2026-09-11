---
title: Framework Integration Architecture
description: Canonical Runenwerk integration, adapter, compatibility, recovery, and product-boundary contracts for independently owned Runen frameworks.
status: active
owner: workspace
layer: architecture
canonical: true
last_reviewed: 2026-09-11
related_docs:
  - ./runenwerk-platform-architecture.md
  - ../guidelines/architecture.md
  - ../guidelines/dependency-rules.md
  - ../adr/accepted/0014-repository-family-extraction-boundaries.md
  - ../adr/accepted/0015-separate-gpu-execution-from-rendering.md
---

# Framework Integration Architecture

## Purpose and authority

Runenwerk is the integration and product repository for independently owned Runen
frameworks. This document owns only Runenwerk's downstream integration, adapter,
compatibility, recovery, and product-policy consequences.

Cross-repository family membership, repository roles, and source-authority transfer
rules are owned by Dornglut Engineering:

- [Runen family architecture](https://github.com/dornglut/engineering/blob/main/architecture/runen-family.md)
- [Repository standard](https://github.com/dornglut/engineering/blob/main/standards/repositories.md)
- [ADR 0008: bounded source-authority handoffs](https://github.com/dornglut/engineering/blob/main/adrs/0008-adopt-bounded-source-authority-handoffs.md)

Each standalone framework repository owns its reusable semantics, public contract,
conformance, validation, release policy, and repository-local architecture. Runenwerk
does not restate those contracts as family authority.

This document therefore does **not** own:

- current family membership or organization-level repository roles;
- reusable framework semantics or package topology;
- framework validation, release, or compatibility policy;
- generic extraction/source-transfer procedure;
- live issue, phase, branch, or migration state.

## Integration boundary

Runenwerk consumes accepted public framework contracts through explicit product and
adapter boundaries:

```text
standalone framework public contract
              |
              v
      Runenwerk adapter/integration
              |
              v
      Runenwerk application/tool
```

Framework repositories do not depend on Runenwerk. A Runenwerk adapter may depend on
the public contracts it translates between, but it does not transfer semantic ownership
back into the product repository.

RunenRender remains Runenwerk-owned rendering integration until a separately accepted
external cutover. While local, its rendering semantics remain distinct from RunenGPU
execution semantics and it consumes RunenGPU through the accepted public API. The
accepted direct dependency direction remains:

```text
RunenRender -> RunenGPU
```

## Runenwerk responsibilities

Runenwerk owns the integration and product concerns that are not reusable framework
semantics, including:

- application and engine lifecycle;
- frame/tick and application/domain lifecycle scheduling, including when independently
  owned RunenECS schedules are invoked;
- windows/event loops and native-host policy;
- ECS and domain extraction into integration-facing prepared inputs;
- scene, world, material-authoring, SDF, UI, editor, simulation, and product adapters;
- shader source discovery, revision, watching, reload scheduling, and last-known-good
  product policy where those concerns remain Runenwerk-owned;
- product capability and quality selection;
- cross-framework composition and tested compatibility;
- product recovery decisions;
- diagnostics presentation and support artifacts;
- reproducibility bundles and persisted capture schemas;
- offline job sequencing and artifact encoding;
- application binaries and tools.

Runenwerk does not redefine a framework's internal semantics merely because it invokes,
adapts, displays, persists, or coordinates that framework's public values.

## Adapter contract

A framework must remain useful without its Runenwerk adapter. Runenwerk adapters exist
to translate integration meaning explicitly, not to become shadow framework
implementations.

Adapters may translate:

- identities where an explicit correspondence is required;
- prepared inputs and outputs;
- lifecycle and generation facts;
- diagnostics and provenance;
- resource/source ownership facts;
- Runenwerk product policy into accepted framework requirements.

Adapters must not:

- duplicate framework algorithms or semantic validation;
- mirror authoritative source or create writable parallel authority;
- expose broad compatibility facades merely to preserve predecessor shape;
- hide dependency cycles;
- retain private reach-through after a public boundary exists;
- silently collapse structured failure or pressure into logs;
- create a universal shared core, registry, identity space, or meta-framework merely
  because several adapters use similar vocabulary.

When a missing capability is reusable framework meaning, change it in the owning
framework under its own accepted work. When it is product translation or composition,
keep it in Runenwerk.

## Compatibility and revision binding

Runenwerk owns the compatibility claim for the exact framework revisions and adapters
that it integrates. Before stable publication, integrated framework dependencies use an
exact accepted revision or an exact accepted pre-release version; moving branch
dependencies are not a compatibility contract.

A Runenwerk compatibility manifest or equivalent evidence may bind:

- exact integrated framework and adapter revisions;
- product-relevant backend/runtime family facts where needed;
- persisted product-artifact schema versions;
- the integration tests that establish the claimed combination.

That evidence is Runenwerk product authority. It does not become a shared framework
manifest and is not imported back into framework repositories as product policy.

## Recovery ownership

Frameworks own the classification and reporting of loss, invalidation, generations,
and reconstruction facts defined by their contracts. Runenwerk owns the product action
taken from those facts, such as retrying, recreating, degrading quality, pausing,
exiting, or asking the user for action.

Source-backed, externally reconstructed, and non-reconstructable values remain
explicitly distinct at the integration boundary. Runenwerk does not infer
reconstructability from an implementation detail or cache hit.

## Reproducibility and persisted product artifacts

Runenwerk may assemble a versioned, namespaced reproducibility or capture bundle from
accepted public facts. Depending on the product contract, that bundle may include:

- exact framework and adapter revisions;
- relevant capabilities and permitted device/backend facts;
- prepared-work or integration diagnostics;
- scene/view/input generations and deterministic inputs such as seeds or fixed time;
- provenance, artifacts, checksums, and privacy/redaction metadata.

Runtime handles, pointers, memory addresses, context-local IDs, device generations, and
unversioned diagnostic strings are not silently promoted to stable persisted identity.

A persisted Runenwerk-owned artifact names its owner, format identity and version,
validation/compatibility policy, migration behavior, deterministic encoding
requirements where relevant, and privacy/redaction policy where relevant. Framework
repositories remain the owners of any persisted formats they independently define.

## RunenUI rendering integration

The product-side rendering relationship is:

```text
RunenUI paint scene
    -> Runenwerk bridge
        -> RunenRender overlay contribution
            -> RunenGPU work
```

The bridge consumes accepted renderer-neutral paint primitives, not widget state or UI
actions. RunenRender does not acquire text-shaping, focus, accessibility, or hit-testing
ownership through this bridge.

## RunenSDF rendering and GPU integration

RunenSDF remains backend-neutral. A Runenwerk-owned adapter may translate accepted
field contracts into render-provider inputs or GPU work while preserving the owning
field semantics and capability facts. Such translation does not make RunenSDF depend on
Runenwerk, RunenRender, or RunenGPU.

If a translation becomes independently reusable, its ownership requires a separately
accepted boundary; reuse is not inferred from the existence of one Runenwerk bridge.

## Integration evidence

Runenwerk owns evidence for claims made by its integration/product boundary, including:

- cross-framework integration tests;
- exact-revision compatibility proof;
- product recovery behavior;
- persisted reproducibility/capture validation;
- adapter-level identity, provenance, and lifecycle translation where those facts are
  part of the integration contract.

Framework conformance remains in the framework repository. Runenwerk integration tests
may prove that a public framework contract works in the product, but they do not become
an alternate framework conformance suite.

No product consumer may bypass a framework's public boundary or reach into private
implementation merely to make an integration or performance proof pass.

## Boundary escalation

When Runenwerk wants a framework's internals, first determine which owner is missing a
public value, command, diagnostic, capability, workload, contribution, or test-support
contract. Add reusable meaning to the framework only under its own accepted authority;
add product translation or composition to Runenwerk.

Cross-repository source movement follows Engineering ADR 0008 rather than a local
Runenwerk cutover procedure. RunenRender extraction likewise requires separately
accepted authority; this integration document does not activate or sequence that work.
