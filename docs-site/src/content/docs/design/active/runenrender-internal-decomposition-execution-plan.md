---
title: RunenGPU and RunenRender Decomposition Execution Plan
description: Durable dependency-ordered program from the combined Runenwerk renderer to clean RunenGPU and RunenRender public boundaries and external repositories.
status: active
owner: render
layer: engine/render
canonical: true
last_reviewed: 2026-09-08
related_docs:
  - ./runengpu-architecture-design.md
  - ./runengpu-g3-access-work-graph-design.md
  - ./runengpu-g4-context-program-realization-design.md
  - ./runengpu-g4b-contracts-g4c-delivery-design.md
  - ./runengpu-shader-authoring-artifact-boundary.md
  - ./runenrender-decomposition-design.md
  - ./runen-family-operational-hardening-design.md
  - ../../reports/investigations/2026-08-04-runenrender-long-term-capability-and-scalability-review.md
  - ../../reports/investigations/runenrender-extraction-investigation.md
  - ../../architecture/repository-family-architecture.md
  - ../../adr/accepted/0014-repository-family-extraction-boundaries.md
  - ../../adr/accepted/0015-separate-gpu-execution-from-rendering.md
  - ../../adr/accepted/0021-ratify-runenrender-semantic-rendering-architecture.md
  - ../../workspace/planning/roadmap.md
---

# RunenGPU and RunenRender Decomposition Execution Plan

## Purpose

Decompose the combined Runenwerk renderer into independently owned boundaries:

```text
RunenGPU
    validated generic GPU execution

RunenRender
    semantic rendering through RunenGPU

Runenwerk
    lifecycle, source/domain projection, windows, scheduling,
    product/recovery/authoring/artifact policy, and integration
```

This document owns durable dependency order, phase responsibility, proof boundaries, and
cutover gates. GitHub issues own activation and live status. Pull requests own delivery
and exact-head evidence. The roadmap owns the high-level sequence.

Implementation requires an owning issue, accepted current architecture, an exact-current
census, and repository validation. No phase is activated merely because it appears here.

## Target repositories

```text
dornglut/runen-gpu
dornglut/runen-render
```

Each begins with one public package.

## Durable sequence

```text
S0
-> G1A -> G2 -> G3
-> G4A -> G4B -> G4C
-> G5 -> G6 -> G7 -> G8
-> GX
-> R0
-> R1 -> R2 -> R3 -> R4 -> R5 -> R6 -> R7 -> R8
-> RX
-> A1
-> V1+
```

```text
S0   current-source and consumer inventory
G*   internal RunenGPU future-public-boundary proof
GX   external RunenGPU clean cutover
R0   RunenRender normative architecture gate
R*   internal RunenRender future-public-boundary proof
RX   external RunenRender clean cutover
A1   reusable adapter review
V1+  advanced renderer program
```

R0 is mandatory. There is no direct `GX -> R1` implementation path.

## Global invariants

Every phase preserves:

- one public package per target repository initially;
- no Runenwerk, product, ECS, SDF, UI, editor, or application types in framework public
  contracts;
- no direct/private WGPU ownership in RunenRender;
- no renderer or domain meaning in RunenGPU;
- no dependency cycle;
- no source mirror, forwarding namespace, compatibility package, source include,
  submodule, or moving-branch dependency;
- no old/new parallel authority after accepted cutover;
- owner-local typed identities rather than universal cross-framework identity;
- accepted GPU work receives exactly one terminal outcome;
- bounded queues, caches, histories, sessions, diagnostics, variants, and backing memory
  expose pressure or bounded waits;
- derived state remains non-authoritative and dependency/generation-bound;
- semantic meaning remains distinct from physical realization;
- Runenwerk owns product recovery, compatibility policy, persisted capture/reproducibility
  artifacts, authoring policy, and artifact encoding;
- proof categories remain separated: correctness, integration, operations, recovery,
  performance, and showcase;
- each implementation phase migrates consumers of replaced authority and deletes that
  authority in the same accepted slice;
- exact-head validation and repository CI remain merge evidence.

# Inventory and RunenGPU program

## S0 — ownership and consumer inventory

S0 is historical discovery evidence. Every implementation phase repeats an exact
current-main affected declaration/consumer census.

## RunenGPU phases

RunenGPU phase detail remains owned by the accepted RunenGPU architecture, focused
phase designs, proof matrix, roadmap, and owning issues. This plan retains only the
durable boundary order needed by RunenRender:

```text
G1A  owner-scoped logical work-resource identity
G2   capabilities, logical resources, typed handles, prepared data
G3   checked access, initialization, hazards, generic work and preparation
G4A  context and adapter/device admission
G4B  program/interface/binding/layout/pipeline contracts
G4C  private backend realization and reusable-authority cutover
G5   execution, progress, completion, readback and retirement
G6   representative offscreen/shared-consumer/cost proof
G7   surfaces, generations, loss and reconstruction
G8   operational conformance and residual no-reach-through audit
GX   standalone RunenGPU authority transfer and Runenwerk exact-revision cutover
```

RunenRender implementation remains downstream of accepted external RunenGPU authority
and consumes only public RunenGPU contracts.

# RunenRender internal proof

## R0 — normative semantic-rendering architecture

R0 is documentation/architecture only.

It establishes:

- semantic rendering as the RunenRender mission;
- explicit non-ownership against RunenGPU, Runenwerk, ECS, SDF, Spatial, UI, assets,
  simulation, persistence, and codecs;
- one renderer-local scene lineage;
- scene/request/request-scoped-input separation;
- renderer-specific observation semantics;
- output meaning/result topology/physical binding separation;
- representation contract/evidence/applicability/availability/realization separation;
- coherent render-method semantics;
- conditional device-independent planning;
- semantic binding admission distinct from execution admission;
- RunenGPU physical lowering boundary;
- incremental/full equivalence;
- the R1-R8/RX dependency order.

R0 does not modify Rust/Cargo, change RunenGPU semantics, populate `dornglut/runen-render`,
or authorize R1.

## Permanent semantic spine

All implementation phases preserve:

```text
RenderSceneStore
    -> commit(RenderSceneUpdate)
        -> RenderSceneCommit
            ├── RenderSceneSnapshot
            └── RenderSceneChangeSet

RenderSceneSnapshot
+ RenderRequest
+ representation/method contracts
    -> conditional RenderPlan

RenderPlan
+ current request-scoped semantic bindings
    -> semantic binding admission

semantically admitted candidates
+ representation availability/realization
+ physical output bindings
+ current RunenGPU environment facts
+ execution requirements
    -> execution admission
        -> AdmittedRenderPlan
            -> RenderWorkSet
                -> RunenGPU
                    -> RenderResult
```

The ordinary public API may collapse stages ergonomically. Their responsibilities remain
distinct and testable.

## R1 — scene lineage and minimal renderer identity

Goal:

- `RenderSceneStore`;
- `RenderSceneRevision`;
- `RenderObjectId`;
- atomic `RenderSceneUpdate` insert/replace/remove;
- immutable `RenderSceneSnapshot`;
- explicit R1-owned `RenderSceneChangeSet`;
- structurally shared or equivalently bounded small-change publication;
- explicit full scene resynchronization;
- no views, representations, acquired output images, execution-environment facts, or
  live source/host state inside the scene snapshot.

R1 deliberately does **not** introduce generic producer identity/lifecycle,
`RenderContributionId`, generic relationships, representation protocols, space/time,
observations, materials, dynamic-input schemas, availability, or GPU placeholders.

Source producer retirement is an adapter-boundary operation:

```text
source disappears
    -> source/Runenwerk adapter resolves affected RenderObjectIds
    -> one atomic renderer removal update
```

Required proof:

- deterministic insert/replace/remove;
- atomic multi-operation commit and rejected-commit no-publication behavior;
- retained old snapshot remains immutable;
- equivalent full and incremental scene construction;
- precise R1 structural change evidence and explicit full resync;
- bounded small-change publication cost against total scene size;
- at least two independent source/adaptor families using the same renderer mutation
  contract;
- source identities/lifecycle remain outside RunenRender;
- no ECS mirror and no RunenGPU identity.

## R2 — semantic space/time, observations, outputs, and minimal requests

Goal:

- coordinate frames, units, semantic transforms, orientation/handedness where required,
  and spatial support/coverage;
- render time/interval, exposure/shutter support, temporal validity, and motion interval;
- `RenderObservationSpec` and coordinated observations;
- semantic sampling support distinct from sampling algorithm;
- `RenderOutputSpec`;
- output-value meaning, result topology, and applicable radiometric/transport domains;
- semantic tolerance/accuracy distinct from numeric realization;
- minimal `RenderRequest`.

No current residency, physical output binding, sampling strategy, or method planning is
owned here.

Required proof includes:

- perspective observation;
- non-image-grid renderer probe;
- coordinated observation set;
- radiance, depth/distance, and identity semantics;
- scene spatial/temporal change evidence only after R2 semantics exist;
- semantic output meaning independent of physical storage/destination.

## R3 — representations, protocols, relationships, and minimum appearance

Goal:

- `RenderRepresentation` as an open family;
- representation identity only where required;
- intrinsic semantic contract/evidence;
- narrow versioned query protocols and narrow results;
- refinement/error semantics;
- concrete typed scene relationships only when a real consumer requires them;
- instance/occurrence semantics only if independently proved;
- minimum material/medium/emitter/environment contracts required by the founding method.

Representation intrinsic evidence may include protocol revisions, coordinate/unit
conventions, spatial/temporal coverage, accuracy/error vocabulary,
exact/conservative/refinement facts, and provenance.

Do not put current availability/residency into authoritative representation meaning.
`RepresentationOffer` may later exist only as a derived planning/admission view.

Initial proof families include at least:

```text
analytic surface
field/SDF surface
```

Required proof:

- exact analytic query;
- conservative field query with bounded termination/error evidence;
- transform validity classification for field/SDF semantics;
- declared coverage and refinement evidence;
- stable `RenderObjectId` across representation replacement;
- at least one real typed relationship;
- protocol version mismatch and structured unsupported outcomes;
- source SDF mathematics remain outside RunenRender.

## R4 — RenderMethod and conditional semantic planning

Goal:

- `RenderMethod` semantic concept and compatibility;
- request-relative representation applicability;
- request-static applicability derived from request/contracts/evidence;
- typed request-scoped semantic-input requirements only for actual R2/R3/R4 consumers;
- binding-dependent applicability represented as explicit predicates/prerequisites;
- `RenderPlan`;
- semantics-preserving alternatives and explicit bounded approximation envelopes;
- normalized abstract RunenGPU capability/work requirements;
- no current physical RunenGPU handles, residency, surfaces, allocations, pipelines, or
  concrete output bindings.

A `RenderPlan` means:

> these solution families can satisfy the request provided their declared semantic
> prerequisites are admitted.

Required proof:

- CPU-only deterministic planning;
- multiple legal solution families for one request;
- protocol, coverage, output, method, and accuracy incompatibility rejection;
- unresolved binding-dependent applicability remains explicit rather than guessed;
- planning requires no current GPU environment;
- no all-object x all-method requirement;
- illegal semantic substitution is not represented as ordinary fallback.

R4 answers:

> What semantic solution families could satisfy this request, and what prerequisites do
> they require?

## R5 — semantic bindings, availability, output bindings, and execution admission

Goal:

- current source-owner semantic input values/bindings/generations/provenance;
- semantic binding admission;
- binding-dependent representation applicability evaluation;
- current representation availability/realization facts;
- `RenderOutputBinding` physical destinations;
- execution requirements and pressure;
- current RunenGPU capability/device-generation facts;
- execution admission;
- `AdmittedRenderPlan`.

Binding admission may evaluate predicates, eliminate candidates, and specialize choices
already declared by R4. It may **not invent a new semantic alternative outside the
`RenderPlan` solution space**.

Required proof:

- valid current semantic binding accepted;
- missing/stale/foreign/temporally incompatible/coverage-incompatible binding rejected;
- binding-generation change does not automatically change `RenderSceneRevision`;
- representation availability/residency change does not automatically change
  `RenderSceneRevision`;
- RunenGPU device-generation change does not automatically change
  `RenderSceneRevision`;
- semantic output meaning differs from physical destination;
- semantic input identity differs from RunenGPU resource identity;
- `unsupported != unavailable`;
- a semantically valid `RenderPlan` can be temporarily inexecutable;
- legal semantics-preserving realization alternatives and bounded permitted
  approximation work;
- semantic substitution is rejected unless explicitly authorized.

R5 answers:

> Which semantically admitted solution can execute now?

## R6 — first complete semantic renderer and RunenGPU lowering

The first end-to-end renderer proof is deliberately bounded.

Scene:

```text
one analytic sphere
one analytic plane
one field/SDF surface
minimum diffuse material
one directional emitter
```

Observation/output proof A:

```text
perspective observation
HDR radiance
depth
object identity
```

Observation/output proof B:

```text
one non-image-grid scalar radiance probe
```

Method/execution:

```text
one coherent direct-lighting method
minimal physical output bindings
public RunenGPU only
CPU reference probes
```

The proof must use permanent R1-R5 scene, request, representation, protocol, method,
planning, binding, admission, and output contracts.

R6 proves both conventional image rendering and the broader semantic-rendering API. It
does not authorize public types named after SDF, direct lighting, preview, or the first
implementation.

## R7 — derived continuity and advanced integration

Introduce only demonstrated advanced requirements:

- explicit derived-state dependency/invalidation structures;
- compiled representations and acceleration;
- renderer-derived residency/realization;
- history, reconstruction, accumulation, and renderer-semantic denoising;
- optional sessions, progress, convergence, continuation, and cancellation;
- compatible multi-observation preparation sharing;
- multi-output sharing and semantic merge;
- readback integration;
- physical presentation/surface binding integration through public RunenGPU;
- semantic partition/merge for multi-device or distributed orchestration.

Runenwerk retains windows, XR/platform runtime lifecycle, presentation/product recovery,
artifact persistence/encoding, remote transport/process lifecycle, retries, and cluster
policy. RunenGPU retains physical surface/device execution.

Required proof includes dependency-driven invalidation, bounded history/session state,
device-generation invalidation for realized derived state, compatible continuation,
incompatible-continuation rejection, reconstruction under pressure, and cache-hit
semantic neutrality.

## R8 — generalization, scale, conformance, and extraction readiness

R8 validates the architecture rather than inventing new foundations merely to satisfy a
matrix.

Required evidence includes:

- large-scene incremental characterization;
- no systematic deep-copy snapshot publication;
- no systematic full rebuild for local change;
- no mandatory all-object x all-method planning;
- no per-object CPU GPU submission;
- RunenGPU work scaling with algorithm stages rather than logical object count;
- bounded memory, histories, sessions, diagnostics, variants, and queues;
- two independent source/adaptor families;
- two meaningfully distinct representation/query families;
- at least two meaningfully distinct render-method families sufficient to prove the
  shared method/planning abstraction;
- non-camera/non-image-grid observation;
- GPU-produced request-scoped semantic input without CPU readback;
- multi-observation and multi-output sharing;
- incremental/full equivalence;
- session/cancellation proof if retained;
- public RunenGPU-only physical lowering and no private reach-through;
- simpler direct renderer comparison for representative proof;
- exact provenance/reproducibility evidence;
- standalone extraction readiness.

Performance evidence remains diagnostic until a separately accepted controlled budget
exists.

## RX — external RunenRender transfer and clean cutover

Prerequisites:

- R0-R8 accepted;
- exact current-source/consumer census repeated;
- standalone boundary proven independently useful;
- exact accepted RunenGPU revision selected;
- no private RunenGPU/WGPU reach-through;
- every active consumer migration and predecessor deletion is ready.

Cutover:

1. populate `dornglut/runen-render` from accepted Runenwerk semantic authority;
2. validate standalone;
3. pin exact accepted RunenGPU revision;
4. accept the standalone successor through its repository-owned workflow;
5. migrate maintained Runenwerk consumers to the accepted successor revision;
6. delete predecessor Runenwerk semantic-rendering authority and temporary seams;
7. prove no source mirror, forwarding namespace, compatibility package, source include,
   submodule, moving-branch dependency, or duplicate renderer remains;
8. record provenance and closeout.

RX is transfer/cutover, not architecture invention.

## A1 — reusable adapter review

Only after both RunenGPU and RunenRender clean cutovers, review whether any Runenwerk
bridge has at least two independent consumers, stable host-neutral semantics, and enough
maintenance duplication to justify extraction. Do not pre-create adapter packages or
change dependency direction merely because one bridge exists.

## V1+ — advanced renderer program

Advanced renderer capability continues through separately accepted protocols,
representations, methods, outputs, semantic inputs, relationships, appearance
extensions, or derived-state kinds. Examples may include:

```text
multi-bounce and bidirectional transport
regional / cellular transport
volumes and sparse scientific fields
populations, fibers, hair, liquids, and deformation
spectral and polarized rendering
differentiable and inverse rendering
learned / neural representations and reconstruction
deep output
XR and foveated rendering
multi-device and distributed rendering
hardware-specialized realizations
```

V1+ does not replace or widen the R0 semantic spine by implication. Any new owner,
dependency, stable format, or shared framework still requires its own accepted evidence.

# Advanced compatibility requirements

## Shader/program ownership

RunenRender owns renderer shader/kernel meaning and semantic variants. RunenGPU owns
canonical program admission, interfaces/layouts/binding compatibility, backend
realization, and physical caches. Runenwerk owns source-root/compiler/artifact/watching
and product last-known-good policy.

## Determinism and reproducibility

Owner-declared determinism may be structural, numerical-within-tolerance, statistical,
or bitwise only under a constrained environment. Applicable method/protocol/input/seed
and environment facts remain inspectable. Runenwerk owns persisted bundles.

## Trust and extension

Versioned renderer extensions remain bounded, typed, and structured. Do not grant
untrusted authoring arbitrary host callbacks, backend access, or unrelated global
resource access. Product trust policy remains Runenwerk-owned.

## Scale invariants

```text
scene publication
    no mandatory deep copy per commit

planning
    no mandatory all-object x all-method scan

GPU work
    stage-scaled, not logical-object-scaled

submission
    no per-object CPU submission requirement

state
    bounded or pressure-reporting

detail
    semantic-support/error driven; never globally materialize unbounded detail

sharing
    compatible observations/outputs may share preparation and derived state

diagnostics
    bounded and aggregatable
```

# Current-source revalidation gate

Before every R implementation slice:

- resolve exact accepted `main`;
- repeat the affected declaration and direct/transitive consumer census;
- inspect all current source paths relevant to the owning phase;
- verify identities and persisted/wire uses;
- identify host/source reach-back and temporary authority;
- bind exact public, migration, deletion, proof, and guard scope;
- run canonical baseline validation;
- stop for a new ADR/package/dependency/stable format/compatibility path/backend escape
  or premature later-phase authority.

Historical reports and prior phases are evidence, not permission to skip current-source
review.

# Shared logical Plan compatibility

RunenRender remains correct whether a future shared logical Plan architecture is
accepted or rejected.

A future shared Plan layer may express, compose, inspect, partition, or orchestrate
renderer operations. It may not absorb RunenRender-owned scene semantics, observation
semantics, representation validity, method semantics, conditional render planning,
semantic binding admission, execution admission, or renderer-specific RunenGPU lowering.

```text
shared logical Plan
    -> RunenRender semantic request / native planning
        -> RunenRender admission / lowering
            -> RunenGPU
```

Provider realization does not transfer semantic ownership.
