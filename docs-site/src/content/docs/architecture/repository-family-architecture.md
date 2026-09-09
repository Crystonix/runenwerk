---
title: Repository Family Architecture
description: Canonical repository ownership, dependency direction, integration, operational contracts, release, conformance, and clean-cutover rules for the Runen framework family.
status: active
owner: workspace
layer: architecture
canonical: true
last_reviewed: 2026-08-30
related_docs:
  - ../workspace/planning/roadmap.md
  - ../reports/investigations/repository-family-current-state-investigation.md
  - ../reports/investigations/runenrender-extraction-investigation.md
  - ../reports/investigations/runengpu-g4-context-program-realization-investigation.md
  - ../reports/investigations/runengpu-industry-comparison.md
  - ../reports/investigations/runengpu-runenrender-application-domain-fit.md
  - ../reports/closeouts/pt-runen-family-operational-hardening-closeout.md
  - ../reports/closeouts/pt-runengpu-g3-implementation-closeout.md
  - ../design/active/runensdf-extraction-design.md
  - ../design/accepted/runenecs-extraction-boundary-design.md
  - ../design/active/runengpu-architecture-design.md
  - ../design/active/runengpu-g4-context-program-realization-design.md
  - ../design/accepted/runenrender-decomposition-design.md
  - ../design/active/runenrender-internal-decomposition-execution-plan.md
  - ../design/active/runen-family-operational-hardening-design.md
  - ../workspace/specs/pt-runengpu-g4a-context-admission.ron
  - ../workspace/specs/pt-runengpu-g4b-program-interface-layout.ron
  - ../workspace/specs/pt-runengpu-g4c-wgpu-realization-cutover.ron
  - ../adr/accepted/0014-repository-family-extraction-boundaries.md
  - ../adr/accepted/0015-separate-gpu-execution-from-rendering.md
---

# Repository Family Architecture

## Purpose

Runenwerk is the integration and product repository for independently useful
framework repositories. This document owns repository-level boundaries, dependency
direction, integration policy, family-wide operational doctrine, and clean cutovers.
Focused accepted framework designs own subsystem contracts.

## Repository family

```text
product       repository                    package       crate
RunenSDF      dornglut/runen-sdf            runen-sdf     runen_sdf
RunenSpatial  dornglut/runen-spatial        existing workspace topology
RunenECS      target dornglut/runen-ecs     see accepted RunenECS design
RunenGPU      dornglut/runen-gpu            runen-gpu     runen_gpu
RunenRender   target dornglut/runen-render  runen-render  runen_render
RunenUI       dornglut/runen-ui             existing workspace topology
Runenwerk     dornglut/runenwerk            workspace      integration/product
```

RunenSpatial already has separately owned foundation, demand-planning, and availability
lifecycle packages. A Runenwerk consumer depends only on the package whose semantics it
actually consumes; repository ownership does not imply a dependency on every package.

RunenGPU and RunenRender each begin with one public package. Internal modules carry
responsibility boundaries until a real second consumer, backend, release unit, ABI, or
compile-time boundary proves another package is needed.

Framework repositories do not depend on Runenwerk. Runenwerk may depend on exact
framework revisions directly or through explicit Runenwerk-owned adapters.

## Dependency direction

```text
RunenSDF -----+
RunenSpatial -+
RunenECS -----+--> Runenwerk adapters/integration --> applications
RunenUI ------+
                   |
                   +--> RunenRender --> RunenGPU
                   +--> non-render RunenGPU workloads
```

ADR 0015 accepts one direct foundational framework dependency:

```text
RunenRender -> RunenGPU
```

No dependency cycle is allowed.

## Current program state

| Framework | Current state | Authorized work |
|---|---|---|
| RunenSDF | standalone authority in `dornglut/runen-sdf`; duplicate Runenwerk source retired through issue `#133` / PR `#157` | standalone roadmap and independently accepted adapters only |
| RunenSpatial | standalone host-neutral spatial authority in `dornglut/runen-spatial`; downstream integration consumes accepted public contracts | standalone roadmap and independently accepted downstream adapters/integration only |
| RunenECS | internal ownership and safety repair required | separately bounded investigation, design, and repair |
| RunenGPU | standalone successor accepted at `77c7c8d5ad6922b6f46c6b25e31b1a224c1314a4`; Runenwerk consumer cutover is pinned to that exact revision | successor repository owns framework implementation and conformance; downstream changes use its public API |
| RunenRender | current Runenwerk render integration consumes the accepted RunenGPU public API | separately bounded R-phase work remains required before external RunenRender cutover |
| RunenUI | independent repository/workstream | governed in RunenUI |

RunenGPU semantic authority is already the accepted successor revision. Runenwerk's
physical consumer cutover is governed separately by issue `#449` and is accepted only
when the merged Runenwerk revision passes accepted-main validation. The workspace
manifest and lockfile are implementation evidence, not downstream acceptance on their
own.

Current source location is implementation evidence, not permanent ownership.

## Repository missions

### RunenSDF

Owns reusable signed-field mathematics, validated field vocabulary, numerical policy,
bounds, composition, transforms, capabilities, and CPU reference queries.

Does not own world streaming, ECS, rendering, GPU resources, materials, or product
policy.

### RunenSpatial

Owns host-neutral spatial identity, world-qualified positions and frames, checked
chunk/region addressing and partition topology, hierarchy/clipmap/ring/hash mechanics,
bounded spatial-demand planning, and content-agnostic availability lifecycle mechanics.

Does not own Runenwerk world selection, quantization, rendering/camera origin policy,
ECS, SDF payloads, persistence/replication policy, networking, gameplay, or product
lifecycle.

### RunenECS

Owns entity/component/resource lifecycle, storage/query semantics, deferred structural
mutation, system identity and access contracts, explicit ECS ordering and sets,
schedule validation, deterministic serial reference execution, and explicit reflection.

Does not own general spatial indexing, application frame/tick policy, rendering
extraction, networking, replay, world streaming, or product lifecycle.

### RunenGPU

Owns normalized capabilities and requirements, contexts, logical resources, checked
access and work graphs, program/interface contracts, private backend realization,
execution/submission mechanisms, low-level surface mechanisms, backend outcomes,
progress and pressure facts, device generations, and GPU diagnostics.

Ownership is phased:

```text
G1A-G3  logical identity, resources, access, operations and prepared work
G4A     context and adapter/device admission
G4B     program, interface, binding, layout and pipeline descriptors
G4C     generation-bound WGPU realization and cache compatibility
G5      execution, submission, progress, completion, readback and retirement
G6      offscreen graphics, shared consumers and cost characterization
G7      surfaces, device replacement, loss and reconstruction
G8      operational conformance and residual audit
GX      external repository cutover
```

RunenGPU does not own image formation, simulation algorithms, field mathematics, ECS,
UI, windows/event loops, shader filesystem policy, artifact codecs, or product recovery.

### RunenRender

Owns prepared render scenes, views, providers and interactions, materials and media,
emitters and environments, visibility, transport, radiance caches, history,
reconstruction, overlays, color, presentation intent, and lowering into RunenGPU
workloads.

Does not own WGPU, general GPU execution, ECS extraction, field mathematics, UI
semantics, native windows, shader file watching, vertical-domain products, or
Runenwerk lifecycle.

G4A, G4B, and G4C are GPU/backend decontamination and substrate work. They do not
implement RunenRender semantics. The current render tree is migration evidence and is
not moved, renamed, wrapped, or extracted wholesale. RX is a later mechanical
transfer/cutover after accepted R-phase proof, not where renderer architecture is
invented.

### RunenUI

Owns semantic UI, state/actions, focus/accessibility, layout/style/text, hit testing,
and renderer-neutral paint output. It does not depend on RunenRender or RunenGPU by
default.

### Runenwerk

Owns:

- application and engine lifecycle;
- frame/tick and application/domain lifecycle scheduling, including when independently
  owned RunenECS schedules are invoked;
- windows/event loops and native host policy;
- ECS and domain extraction;
- scene, world, material-authoring, SDF, UI, editor, simulation, and product adapters;
- shader source discovery, revision, watching, reload scheduling, and last-known-good
  product policy;
- product capability and quality selection;
- cross-framework composition and tested compatibility;
- product recovery decisions;
- diagnostics presentation and support artifacts;
- reproducibility bundles and persisted capture schemas;
- offline job sequencing and artifact encoding;
- application binaries and tools.

Runenwerk does not redefine RunenECS-internal system order, access compatibility,
schedule validation, deferred-command boundaries, or serial correctness semantics.

## Adapter rule

A framework must remain useful without its Runenwerk adapter.

Adapters translate:

- identities;
- prepared inputs and outputs;
- lifecycle and generation facts;
- diagnostics and provenance;
- resource/source ownership;
- product policy into accepted framework requirements.

Adapters must not:

- duplicate framework algorithms;
- mirror authoritative source;
- introduce writable parallel authority;
- expose broad compatibility facades;
- hide dependency cycles;
- preserve private reach-through after cutover;
- silently convert structured pressure or failure into logs.

Keep a bridge in Runenwerk until independent consumers prove stable reusable ownership.
A temporary bridge must name its owning deletion phase. G4C deletes GPU realization,
program/interface, cache, synthetic-handle, and temporary resource-owner authority.
G5 deletes the residual execution payload. G7 deletes temporary surface-compatibility
seams.

## Family-wide operational doctrine

### Accepted-work integrity

Once a framework accepts work, it must eventually report exactly one terminal outcome.
Accepted work, completion notifications, requested artifacts, and non-discardable
source state are never silently dropped.

### Structured pressure

Every bounded queue, staging arena, readback pool, retained cache, history set, and
capture buffer returns a structured pressure outcome or an explicitly bounded wait.

Permitted pressure strategies are:

```text
reject with facts
wait with an explicit bound
shed discardable derived work
request caller-owned quality reduction
```

Unbounded implicit growth is not a default policy.

### Derived caches

Derived caches are non-authoritative, discardable, reconstructable, keyed by every
correctness fact, source-generation-bound, validated before reuse, and versioned when
persisted. A cache hit changes cost, not semantics.

G4C initially owns only in-memory, context/device-generation-scoped GPU realization
caches. No stable persisted backend cache format is authorized by G4 planning.

### Compatibility manifest

Runenwerk owns a tested compatibility manifest for the exact framework and adapter
revisions it integrates. It may include WGPU/backend family and persisted artifact
schema facts.

The manifest does not create a shared `RunenCore` package and is not imported back into
framework repositories as product policy.

### Recovery ownership

Frameworks classify loss, invalidate generations, and report reconstruction facts.
Runenwerk decides whether a product retries, recreates, degrades, pauses, exits, or asks
the user for action.

Source-backed, externally reconstructed, and non-reconstructable values remain
explicitly distinct.

### Reproducibility bundle

Runenwerk may assemble a versioned namespaced bundle containing framework revisions,
capabilities, device/backend facts where permitted, prepared-work diagnostics,
scene/view generations, seeds, fixed-time inputs, provenance, artifacts, checksums,
and privacy/redaction metadata.

Runtime handles, pointers, memory addresses, context IDs, device generations, and
unversioned diagnostic strings are never persisted as stable authority.

## RunenUI rendering relationship

```text
RunenUI paint scene
    -> Runenwerk bridge
        -> RunenRender overlay contribution
            -> RunenGPU work
```

The bridge consumes accepted paint primitives, not widget state or actions.
RunenRender does not shape text or perform UI hit testing.

## RunenSDF rendering/GPU relationship

RunenSDF remains CPU/backend-neutral. A Runenwerk or future reusable adapter may
translate accepted field contracts into render providers or GPU work while preserving
numerical and capability semantics. RunenSDF never depends back on the adapter,
RunenRender, or RunenGPU.

## One-package initial rule

Do not create speculative package trees merely to draw architecture boundaries.
RunenSpatial's established multi-package topology is already separately proven and is
not governed by this initial-target list. Initial one-package targets for the remaining
extraction tracks are:

```text
runen-sdf
runen-ecs
runen-gpu
runen-render
runen-ui
```

Additional packages require a proven independent dependency subset, second backend,
release/versioning unit, required proc macro, platform/MSRV separation, or externally
used conformance package.

G4 does not create `runengpu_core`, `runengpu_wgpu`, `runengpu_macros`, a cache package,
a testing package, or a facade.

## No shared-core magnet

Do not create `RunenCore`, a universal ID repository, universal diagnostics package,
or generic plugin/meta-framework to simplify extraction. Values live with the
repository whose invariants they express. Adapters map values explicitly.

## Identity and diagnostics

Each framework owns opaque runtime identities for its concepts. Runtime IDs are not
silently serialized or transmitted. Stable formats require explicit identifiers,
versions, validation, and migration.

Diagnostics are repository-namespaced:

```text
runensdf.*
runenecs.*
runengpu.*
runenrender.*
runenui.*
runenwerk.*
```

Adapters add integration context instead of replacing failures with strings.

## Toolchain and release policy

Every extracted repository defines:

- Rust edition and declared MSRV;
- formatting, locked tests, strict Clippy, rustdoc, and docs validation;
- publication and API stability state;
- license and source provenance;
- dependency and feature policy;
- public downstream conformance.

Before stable publication, Runenwerk uses an exact commit or exact pre-release version.
Moving branch dependencies are forbidden.

## Persisted formats

Rust API compatibility and persisted-format compatibility are separate.

Every persisted source, artifact, trace, replay, cache, compatibility manifest, capture
bundle, or wire format names:

- owning repository;
- format identifier and version;
- validation and compatibility policy;
- migration behavior;
- deterministic encoding requirements where relevant;
- privacy and redaction policy where relevant.

Internal runtime packets, descriptors, cache keys, context IDs, device generations, and
handles are not stable formats by default. G4 defines no stable source, ABI, cache,
capture, persistence, replay, or wire format.

## Conformance

Every framework requires:

- unit, negative, and property/invariant tests for owned semantics;
- at least one downstream public-API consumer;
- stable and declared-MSRV validation;
- formatting, locked tests, strict Clippy, rustdoc, docs, metadata, license, and
  provenance checks;
- no Runenwerk source include, mirror, submodule, or compatibility package;
- operational pressure, shutdown, and recovery proof once the framework owns those
  capabilities.

Runenwerk owns cross-repository integration tests, compatibility-manifest proof,
product recovery proof, and persisted reproducibility/capture validation.

Evidence distinguishes deterministic contract proof from environment-dependent
GPU/window/runtime proof. G4 keeps synthetic admission, descriptor, binding, cache,
affinity, migration, and source/dependency proof separate from live WGPU adapter,
module, resource, and pipeline evidence.

## Performance and anti-cheating

A framework boundary is not justified by architecture diagrams alone.

RunenGPU and RunenRender must characterize:

- CPU preparation and validation cost;
- allocations and memory high-water marks;
- cold/warm shader and pipeline cost;
- staging and readback pressure;
- GPU timing where supported;
- full versus incremental scene preparation;
- derived-cache behavior;
- direct narrow alternatives for representative proofs.

No consumer may bypass the public boundary to make benchmarks appear favorable.
Performance budgets require separately accepted controlled specifications.

## Clean-cutover rule

Each extraction proceeds:

1. inventory current source and every consumer;
2. accept a decision-complete boundary;
3. correct and prove the future public boundary inside Runenwerk;
4. establish independent conformance;
5. create or populate the successor repository on an unmerged branch while Runenwerk remains semantic source authority;
6. validate and accept the successor through its repository-owned workflow;
7. switch semantic source authority to the accepted successor revision and freeze any still-present Runenwerk predecessor copy;
8. pin Runenwerk to the exact accepted successor revision and migrate active consumers;
9. delete the frozen predecessor implementation in the same downstream cutover and remove temporary migration seams;
10. update authority and record closeout.

This handoff follows Dornglut Engineering ADR 0008. Before successor acceptance,
duplicate candidate source exists only on an unmerged successor branch. After successor
acceptance, physical overlap is permitted only as one serialized publication interval:
the accepted successor is the sole semantic source authority and the Runenwerk copy is
frozen, deletion-bound, and not independently writable. Ordinary successor feature or
capability evolution waits until predecessor deletion. A cancelled transfer after
successor acceptance requires an explicit accepted reversal before predecessor semantic
authority can resume. Compatibility packages, forwarding namespaces, mirrors,
submodules, source includes, and moving branch dependencies do not survive a completed
cutover.

If Runenwerk has no active consumer, removing the internal implementation does not
require adding an unused external dependency.

## GPU/render sequencing

```text
S0 inventory
-> internal RunenGPU G1A-G8 proof
-> external RunenGPU cutover GX
-> internal RunenRender R1-R8 proof on accepted RunenGPU
-> external RunenRender cutover RX
-> reusable adapter review
-> advanced renderer work
```

RunenGPU moves before RunenRender because the renderer depends on it. Advanced
renderer/provider work must not harden accidental mixed ownership before the
foundational cutovers.

Within G4:

```text
G4A context admission
-> G4B program/interface contracts
-> G4C WGPU realization and cutover
```

Only one implementation slice is active at a time.

## Extraction gates

No external transfer begins until the track proves:

- complete source and consumer inventory;
- decision-complete public ownership and API direction;
- no unresolved dependency cycle;
- independent downstream conformance;
- validation and versioning policy;
- identity, diagnostics, persisted-format, pressure, and recovery decisions;
- exact move, stay, redesign, and delete map;
- provenance and clean-cutover strategy;
- current exact-head CI;
- acceptable measured boundary overhead;
- no private reach-through or duplicate execution path.