---
title: RunenGPU and RunenRender Decomposition Execution Plan
description: Durable dependency-ordered program from the combined Runenwerk renderer to clean RunenGPU and RunenRender public boundaries and external repositories.
status: active
owner: render
layer: engine/render
canonical: true
last_reviewed: 2026-09-07
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
  - ../../workspace/planning/roadmap.md
---

# RunenGPU and RunenRender Decomposition Execution Plan

## Purpose

Decompose the combined Runenwerk renderer into:

```text
RunenGPU
    validated generic GPU execution

RunenRender
    semantic scene-to-image formation through RunenGPU

Runenwerk
    lifecycle, ECS/domain projection, windows, scheduling, source policy,
    compatibility, recovery, authoring, artifacts, and product integration
```

This document owns durable sequence, phase responsibilities, proof boundaries, and
cutover gates. GitHub issues own accepted work and live status. Pull requests own
delivery and exact-head review evidence. The roadmap owns high-level sequence and
dependencies. This plan does not duplicate a live implementation ledger.

Implementation requires an owning GitHub issue, accepted durable architecture or ADR
where required, an exact-current census, and the repository workflow and validation
gates. A phase specification may remain only when a proven executable consumer or
separately accepted bounded handoff requires it; it is not an independent
implementation-authorization certificate.

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
-> R1 -> R2 -> R3 -> R4 -> R5 -> R6 -> R7 -> R8
-> RX
-> A1
-> V1+
```

```text
S0   current-source and consumer inventory
G*   internal RunenGPU future-public-boundary proof
GX   external RunenGPU clean cutover
R*   internal RunenRender future-public-boundary proof
RX   external RunenRender clean cutover
A1   reusable adapter review
V1+  advanced renderer program
```

Cross-cutting requirements are assigned to their owning existing phase. They do not
create G9, R9, or a shared framework package without a separately accepted architecture
decision.

## Global invariants

Every phase preserves:

- one public package per target repository initially;
- no Runenwerk, product, ECS, SDF, UI, editor, or application types in framework public
  contracts;
- no direct WGPU ownership in RunenRender;
- no renderer or domain meaning in RunenGPU;
- no dependency cycle;
- no source mirror, forwarding namespace, compatibility package, source include,
  submodule, or moving-branch dependency;
- no old/new parallel authority after accepted cutover;
- typed identities and references rather than string, path, `TypeId`, feature, pass, or
  naked-hash authority;
- accepted GPU work receives exactly one terminal outcome;
- bounded queues, caches, history, sessions, captures, diagnostics, and backing memory
  expose pressure or bounded waits;
- derived state remains non-authoritative and dependency/generation-bound;
- Runenwerk owns product recovery, compatibility manifests, persisted captures,
  reproducibility bundles, authoring policy, and artifact encoding;
- deterministic contract evidence remains separate from environment-dependent GPU,
  window, device, and performance evidence;
- proof categories remain separated: correctness, integration, operations, recovery,
  performance, and showcase;
- exact-head validation, documentation build, and GitHub Actions are merge evidence;
- each implementation phase migrates every consumer of replaced authority and deletes
  that authority in the same accepted slice.

# Inventory

## S0 — ownership and consumer inventory

S0 classifies current files, shaders, macros, tests, examples, benchmarks, artifacts,
identities, consumers, lifecycle, persistence, and move/stay/redesign/delete scope.

S0 is historical discovery evidence. Every later phase repeats its exact affected
current-main census.

# RunenGPU internal proof

Detailed RunenGPU implementation state remains owned by the RunenGPU designs,
specifications, roadmap, and GitHub issues. This plan records only durable boundaries.

## G1A — logical work-resource identity

Owns owner-scoped logical work-resource identity, fallible allocation, foreign-owner
rejection, and removal of superseded renderer-owned generic identity.

## G2 — capabilities, logical resources, handles, and prepared data

Owns normalized requirements, backend-neutral logical resources, typed handles,
prepared-data boundaries, and independent kind, lifetime, ownership, transfer,
reconstruction, and memory-intent dimensions.

G2 does not create contexts, devices, graphs, execution, or surfaces.

## G3 — access, initialization, hazards, work, and preparation

Owns checked resource access, graph-entry initialization, immutable typed operations,
inferred hazards, typed cross-fragment causality, explicit non-data order,
deterministic preparation, and structured graph failures.

The work graph is correctness and inspection machinery, not mandatory ordinary
ceremony.

## G4A — context and adapter/device admission

Owns asynchronous headless context request, deterministic adapter/device admission,
opaque context and device-generation identity, normalized admitted facts, private WGPU
instance/adapter/device/queue containment, and one bounded temporary host seam until
G7.

## G4B — program, interface, binding, layout, and pipeline contracts

Owns canonical WGSL source admission, typed entry points, explicit shader-resource
interfaces, typed binding keys, bind-group and pipeline-layout descriptors,
specialization, generic compute/render pipeline descriptors, runtime binding
compatibility, and compile-pass/fail proof.

G4B creates no WGPU object.

## G4C — private WGPU realization and cutover

G4C is delivered in ordered slices:

```text
G4C1  resources, views, samplers, query sets
G4C2  canonical modules, layouts, bind groups
G4C3  compute/render pipelines and final realization cutover
```

G4C owns context/device-generation affinity, private registries,
correctness-complete in-memory caches, consumer migration, and deletion of
renderer-owned reusable realization authority.

G4C does not encode or submit work and does not own reusable surfaces.

## G5 — execution, progress, pressure, completion, and retirement

Owns prepare/submit, command encoding, uploads/updates, query resolution, progress,
bounded pending work, completion, cancellation, readback, mapping, delayed destruction,
retirement, and shutdown.

## G6 — offscreen graphics, shared consumers, and cost proof

Owns known-pattern offscreen graphics, compute-to-render composition, shared render and
non-render consumers, cold/warm characterization, direct-WGPU narrow comparisons, and
representative integration workloads.

## G7 — surfaces, generations, loss, and reconstruction

Owns raw-handle surface admission, surface identity/configuration/acquisition/
presentation, thread affinity, device replacement, loss classification, source-backed
reconstruction facts, and deletion of temporary host-surface seams.

Runenwerk retains product recovery policy.

## G8 — operational conformance and residual audit

Owns complete operational evidence for pressure, progress, completion, readback,
shutdown, cache behavior, device/surface loss, reconstruction, diagnostics, capture
facts, performance characterization, standalone conformance, and removal of every
private WGPU or temporary-authority bypass.

## GX — external RunenGPU transfer and cutover

Prerequisites:

- G1A-G8 accepted;
- one render and one independent non-render consumer;
- independent package validation;
- acceptable measured boundary cost;
- no private WGPU reach-through;
- no duplicate internal path.

Cutover:

1. populate `dornglut/runen-gpu` from accepted authority;
2. validate independently;
3. pin Runenwerk to an exact revision or pre-release;
4. migrate active consumers;
5. delete internal authority and temporary seams;
6. prove no mirror, forwarding package, compatibility namespace, source include,
   submodule, moving-branch dependency, or duplicate runtime;
7. record closeout.

# RunenRender internal proof

RunenRender implementation begins only after accepted external RunenGPU cutover and a
separately authorized R-phase issue.

Current render files may change during G phases only where generic GPU/backend authority
is migrated or deleted. The current render tree is not moved, renamed, wrapped, or
extracted wholesale.

## Permanent semantic spine

All R phases preserve:

```text
RenderSceneStore
    -> commit(RenderSceneUpdate)
        -> RenderSceneCommit
            - RenderSceneSnapshot
            - RenderChangeSet

RenderSceneSnapshot
+ RenderRequest
+ RenderInputSet
    -> RenderMethod
        -> RenderPlan
            -> AdmittedRenderPlan
                -> RenderWorkSet
                    -> RunenGPU
```

Exact Rust extension traits remain future phase decisions. The semantic responsibilities
are binding.

## R1 — scene revisions, identities, relationships, and cheap snapshots

Goal:

- renderer-local typed identities;
- `RenderObject` identity separated from source, ECS, asset, representation,
  representation-element, and RunenGPU resource identity;
- atomic `RenderSceneUpdate` insert, replace, remove, relationship change, and
  producer retirement;
- one `RenderSceneCommit` containing an immutable `RenderSceneSnapshot` and explicit
  `RenderChangeSet`;
- structurally shared or equivalently bounded snapshots with no mandatory deep copy;
- typed relationships and deterministic reference validation;
- full-resynchronization fallback without hidden full rebuilds;
- no views, acquired surface images, execution-environment bindings, or live host state
  inside the scene snapshot.

Required proof:

- equivalent full and incremental construction;
- deterministic replacement, removal, relationship update, and producer retirement;
- stable semantic object identity across ordinary R1 object replacement and update;
- additions, removals, R1-owned property and relationship changes, and
  full-resynchronization change evidence;
- small-change cost characterized against total scene size;
- at least two independent producer families;
- no ECS mirror world.

## R2 — space, time, typed dynamic inputs, and availability

Goal:

- coordinate frames, units, spatial transforms, render origins, precision contracts,
  and spatial coverage;
- render time, time intervals, exposure, temporal sampling, temporal coverage, and
  motion contracts;
- typed `RenderInputSlot<T>` schemas and `RenderInputSet` bindings;
- CPU-prepared, retained RunenGPU resource, typed RunenGPU export, streamed,
  externally reconstructed, and renderer-derived inputs;
- generation, availability, lifetime/access, fallback, and provenance for every input;
- source-authoritative world and simulation meaning retained outside RunenRender.

Required proof:

- region-relative or view-relative GPU preparation without losing source-space
  provenance;
- spatial and temporal change evidence expressed through the accepted R2 semantics;
- dynamic-input generation and availability change evidence;
- time-varying input validation;
- typed GPU-produced simulation input with inferred RunenGPU causality and no CPU
  readback;
- missing, stale, foreign, and temporally incompatible input rejection;
- explicit valid fallback only where declared.

## R3 — representation offers, sampling footprints, protocols, and narrow results

Goal:

- multiple `RenderRepresentation` values per `RenderObject`;
- `RepresentationOffer` facts for protocols, coordinate/time coverage, accuracy,
  confidence, freshness, residency, refinement, measurement compatibility, fallback,
  and provenance;
- explicit residency states and pressure-aware availability;
- `SampleFootprint` covering spatial, image, volume, temporal, and future spectral
  sampling domains;
- small versioned semantic protocols rather than one universal provider trait;
- narrow results such as `SurfaceHit`, `VisibilityResult`, `VolumeInterval`, and
  `TransmittanceResult`;
- lazy normal, material, medium, motion, derivative, coordinate, and custom-attribute
  queries;
- no closed permanent representation-family enum.

Initial proof protocols:

```text
SurfaceQueryProtocolV1
VisibilityQueryProtocolV1
AttributeProtocolV1
RefinementProtocolV1
```

Initial representations:

```text
analytic surface
field/SDF surface
```

Required proof:

- exact analytic hit;
- conservative field query with bounded termination/error evidence;
- exact/conservative/approximate/invalid transform evidence for the field/SDF
  representation;
- footprint-driven refinement decision;
- valid coarse fallback under missing fine residency;
- deterministic representation selection;
- stable `RenderObjectId` across `RenderRepresentation` replacement;
- representation-offer, availability, and residency change evidence;
- exact change evidence for representation replacement;
- no per-query dynamic-dispatch requirement in GPU hot paths;
- protocol-version mismatch and unsupported outcomes;
- unrelated protocols do not widen existing implementations.

Volume, raster-geometry, motion, population, fiber, liquid, neural, and
hardware-specialized protocols require later accepted consumers.

## R4 — views, outputs, measurement, materials, methods, and planning

Goal:

- `RenderView`, coordinated `RenderViewSet`, observation models, exposure,
  viewport/image region, masks, sample distribution, and provenance;
- `RenderOutputSet` with semantic meaning, layout, extent/domain, measurement,
  precision, accumulation/merge rule, destination intent, and required/optional status;
- materials, media, emitters, environments, and broadly applicable
  `RenderAppearance`;
- explicit accuracy, determinism, history/session, hard-limit, and soft-performance
  policy;
- `RenderMethod` semantic concept;
- device-independent `RenderPlan`;
- execution-environment-specific `AdmittedRenderPlan`;
- resource, work, output, variant, and residency estimates;
- no product quality ladder, authoring graph, asset importer, color configuration, or
  platform runtime in RunenRender.

Required proof:

- CPU-only deterministic planning;
- distinct semantic plan and execution admission;
- selected/rejected representation and method evidence;
- at least color/radiance, depth/distance, normal, and identity output semantics;
- explicit accumulation rules;
- view/output sharing facts;
- material/medium/emitter meaning independent of representation;
- hard limits distinct from performance goals;
- structural and tolerance determinism policies.

The exact public render-method extension trait remains unstabilized.

## R5 — first complete image-formation method and RunenGPU lowering

Goal:

```text
one analytic sphere
one analytic plane
one field/SDF surface representation
one diffuse material
one directional emitter
one view
one linear HDR output
compute current visibility and direct lighting
fullscreen render conversion
offscreen output
CPU reference probes
```

The proof must use permanent R1-R4 contracts:

```text
scene commit and snapshot
change set
object and representation identity
representation offer
versioned query protocols
narrow results
view and output requests
render method
semantic plan
execution admission
typed inputs
RunenGPU work lowering
```

RunenRender lowers to generic RunenGPU work without direct WGPU access.

The first method does not authorize public types named after SDF, direct lighting,
preview, or the first implementation.

R5 does not require multi-bounce path tracing, temporal history, denoising, cellular
transport, neural representations, deep output, or surfaces.

## R6 — derived-state graph, residency, sessions, and invalidation

Goal:

- explicit `DerivedRenderState` dependency graph;
- compiled-representation, acceleration, residency, transport-estimate, history,
  reconstruction, and output-accumulation state kinds;
- exact source dependencies, spatial/temporal coverage, measurement domain,
  accuracy/confidence, memory class, retention priority, update strategy,
  reconstruction recipe, and device generation;
- bounded or pressure-reporting state;
- narrow dependency-driven invalidation and explicit full invalidation;
- optional `RenderSession` continuity across bounded invocations;
- sample/random-stream ranges, progress, convergence, cancellation, and compatibility;
- no stale cache use as quality degradation.

Required proof:

- changed-property and changed-region invalidation;
- device-generation invalidation;
- compatible session continuation and incompatible continuation rejection;
- bounded history, accumulation, residency, and diagnostics;
- derived-state reconstruction and pressure outcomes;
- cache hits change cost only.

## R7 — multiview, multi-output, surface, readback, and merge integration

Goal:

- offscreen, readback, retained, presentation, tiled, sparse, multisample, and future
  deep output binding classes;
- multiple coordinated views and outputs through one semantic plan;
- explicit output accumulation and merge semantics;
- shared preparation and derived state across compatible views/outputs;
- surface integration only through RunenGPU resources, leases, generations, and
  outcomes;
- RunenRender consumes device/surface facts without owning windows or recovery;
- no direct WGPU dependency.

Required proof:

- offscreen and surface paths share semantic planning;
- multiview and multi-output sharing evidence;
- readback output with semantic provenance;
- surface loss/outdated/device-loss propagation;
- no acquired-surface ownership in scene snapshots;
- merge semantics suitable for later tiled or multi-device work.

Runenwerk owns artifact encoding, XR/platform runtime integration, and presentation
policy.

## R8 — scalability, extension, operational, and extraction conformance

Goal:

- large-scene incremental preparation evidence;
- work-node and CPU-submission scaling independent of logical object count;
- GPU culling, compaction, indirect/generated work, or equivalent bounded execution;
- bounded memory, residency, diagnostics, output, session, and variant behavior;
- first method plus a meaningfully distinct second render method or planning strategy;
- first surface/visibility protocols plus a meaningfully distinct second
  representation/query family;
- GPU-produced dynamic input;
- multiview/multi-output sharing;
- progressive-session and cancellation evidence;
- full versus incremental cost;
- selected/rejected representation and degradation diagnostics;
- cold/warm shader/pipeline and variant characterization inherited through RunenGPU;
- artifact/capture/reproducibility facts;
- comparison with a simpler direct renderer for the same bounded proof;
- no RunenGPU or WGPU private reach-through;
- downstream public-API conformance and extraction readiness.

Required scalability evidence:

```text
no deep copy per local commit
no systematic full rebuild
no all-object × all-method scan
work-node count scales with stages
no per-object CPU submission
bounded or pressure-reporting memory
footprint/error-driven refinement
bounded aggregated diagnostics
recorded automatic-selection revisions
```

Future spectral, polarized, differentiable, neural, XR, deep, multi-device,
distributed, fiber, liquid, regional/cellular, path-guiding, and
hardware-specialized work must be admissible through accepted extension families
without changing the foundational scene and RunenGPU boundaries.

## RX — external RunenRender transfer and cutover

Prerequisites:

- accepted external RunenGPU;
- R1-R8 accepted;
- independent package validation and public downstream proof;
- exact RunenGPU revision;
- multiple producer families;
- multiple representation/query families;
- two meaningfully distinct render methods or planning strategies;
- incremental preparation and derived-state proof;
- large-scene and bounded-work evidence;
- no direct WGPU dependency or Runenwerk/source-domain types;
- acceptable measured value versus simpler alternatives.

RX is a mechanical transfer and clean cutover:

1. populate `dornglut/runen-render` from accepted internal authority;
2. validate independently;
3. pin the exact RunenGPU revision;
4. migrate active Runenwerk consumers;
5. delete internal image-formation authority and temporary seams;
6. prove no mirror, forwarding package, compatibility namespace, source include,
   submodule, moving-branch dependency, or duplicate renderer;
7. record closeout and accepted provenance.

Renderer architecture is accepted before RX. RX does not invent semantics.

## A1 — reusable adapter review

Only after both clean cutovers, review whether a Runenwerk bridge has two independent
consumers and stable reusable ownership. Do not pre-create adapter packages.

## V1+ — advanced renderer program

Advanced work enters through separately accepted protocols, representations, methods,
outputs, inputs, relationships, appearance extensions, or derived-state kinds.

Potential programs include:

```text
multi-bounce and bidirectional path transport
wavefront/fused execution comparison
regional and cellular transport
radiance caches, probes, reservoirs, and path guiding
volumes and sparse scientific fields
populations, fibers, hair, liquids, and deformation
spectral and polarized rendering
differentiable and inverse rendering
neural fields, learned materials, and Gaussian splats
deep output
XR and foveated rendering
multi-device and distributed rendering
hardware ray tracing, mesh/task, sparse, and generated-work realizations
MaterialX/OpenPBR and color-management adapters
```

V1+ does not replace the foundational semantic spine.

## Phase start checklist

Before every implementation issue:

- verify exact accepted `main` and implementation base;
- repeat affected declaration and direct/transitive consumer census;
- run canonical baseline validation;
- confirm no accepted stable persisted, replay, wire, cache, ABI, or external format
  changes without explicit migration/version authority;
- bind exact public, migration, deletion, proof, guard, and stop scope;
- preserve one authority and remove replaced paths in the accepted slice;
- stop for a new ADR, package, dependency direction, compatibility path, unsafe
  backend escape, stable plugin ABI, or premature later-phase authority.

## Strategic reevaluation gates

Reconsider RunenGPU if no independent non-render consumer exists, ordinary consumers
need raw WGPU, or measured overhead lacks reusable correctness value.

Reconsider RunenRender if:

- a smaller renderer satisfies all accepted proofs;
- snapshots require systematic deep copies;
- local changes require systematic full rebuilds;
- protocols become universal, stringly, or runtime-heavy;
- planning requires all-object × all-method scans;
- work-node or CPU-submission count scales directly with scene objects;
- no meaningfully distinct second representation family or method exists;
- backend-neutral contracts repeatedly leak backend details;
- measured cost materially exceeds simpler alternatives without reusable value.

Reevaluation is explicit architecture work, not permission for a hidden bypass.