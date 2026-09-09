---
title: Ratify RunenRender Semantic Rendering Architecture
description: Accepted pre-R1 decision defining RunenRender semantic-rendering ownership, normalized semantic/operational boundaries, and the dependency-ordered R0-R8/RX track.
status: accepted
owner: render
layer: architecture
canonical: true
last_reviewed: 2026-09-08
related_adrs:
  - ./0014-repository-family-extraction-boundaries.md
  - ./0015-separate-gpu-execution-from-rendering.md
  - ./0017-cross-authority-consistency-and-graph-semantics.md
  - ./0018-semantic-federation-and-physical-realization.md
related_docs:
  - ../../design/accepted/runenrender-decomposition-design.md
  - ../../design/active/runenrender-internal-decomposition-execution-plan.md
  - ../../architecture/repository-family-architecture.md
  - ../../workspace/planning/roadmap.md
---

# ADR 0021: Ratify RunenRender Semantic Rendering Architecture

## Context

The prior RunenRender architecture described the framework as owning **semantic
scene-to-image formation**, while the same architecture already required depth,
distance, orientation, identity, segmentation, transmittance, variance/confidence,
spectral and polarized domains, multiview, custom renderer observations, and
nonphysical/stylized results.

The old phase decomposition also coupled several independent semantic and operational
concerns:

- producer/contribution lifecycle with renderer scene authority;
- representation meaning with request applicability and current residency;
- observation/output semantics with later method/planning work;
- dynamic semantic inputs with scene/time authority;
- device-independent planning with current execution admission.

Those couplings were still correctable before public RunenRender implementation and
external extraction, so a mandatory R0 architecture gate was introduced before R1.

## Decision

RunenRender owns **semantic rendering**:

> the formation of renderer-semantic results from renderer-local scene state and
> explicit request-scoped rendering semantics, together with renderer-native planning,
> admission, and lowering that preserve those semantics through RunenGPU execution.

The minimal semantic relation is:

```text
RenderSceneSnapshot
+ RenderRequest
+ admitted request-scoped semantic inputs
    -> RenderResult
```

An image is one result topology, not the root ontology.

RunenRender is intentionally broader than image formation and intentionally narrower
than generic observation, generic scientific measurement, generic world querying,
simulation, image processing, or generic GPU computation.

A computation belongs to RunenRender only when correctness materially depends on
renderer-owned semantics such as renderer-local scene participation, renderer
observation, render representation/query semantics, visibility, appearance, transport,
semantic sampling support/reconstruction, composition, render-output meaning, or
render-method validity.

Producing pixels, using geometry, evaluating an SDF, returning a scalar, or executing on
a GPU is insufficient by itself.

## Authority retained and superseded

ADR 0014 remains authoritative for repository-family independence, Runenwerk-owned
integration/adapters, exact-revision extraction/cutover, and rejection of source mirrors,
forwarding facades, or shared identity authority.

ADR 0015 remains authoritative for the direct dependency and physical execution split:

```text
Runenwerk integration and host/product policy
    -> RunenRender
        -> RunenGPU generic GPU execution
            -> private backend
```

RunenRender does not own WGPU directly and does not recreate RunenGPU resource,
access/hazard, realization, allocation, submission, progress, readback, surface, device,
or error authority.

This ADR narrowly supersedes only ADR 0015 wording that defines RunenRender
**exclusively** as semantic image formation. Image formation remains a valid and
important RunenRender workload; it is no longer the exhaustive mission.

ADR 0017 remains authoritative for one semantic invariant set per authority, owner-local
consistency, explicit foreign-owner contracts, graph-semantic separation,
incremental/full equivalence, and shared-extraction discipline. Its conceptual
`Observation` vocabulary does not become a universal RunenRender type.

ADR 0018 remains authoritative for semantic/physical separation, owner-local versions,
consumer-owned admission, provenance, explicit approximation, and the rule that a
physical realization may constrain but must not silently redefine semantics.

## Normative semantic decomposition

The foundational model is intentionally small:

```text
RenderSceneSnapshot
    committed renderer-local semantic scene state

RenderRequest
    requested renderer semantics

request-scoped semantic inputs
    foreign/source-owned semantic facts required by concrete renderer contracts

RenderResult
    renderer-semantic outcome
```

`RenderMethod` is not part of minimum requested meaning.

The following distinctions are normative:

```text
scene state
!= request state
!= request-scoped foreign semantic state

requested semantics
!= algorithm choice
!= current executability
!= physical realization

semantic accuracy
!= numerical realization

semantic output meaning
!= physical output binding
```

An empty `RenderSceneSnapshot` is valid. Do not create separate foundational scene and
scene-less renderer APIs.

## Scene, identity, and revision law

RunenRender owns one immutable renderer-local scene lineage:

```text
RenderSceneStore
    -> commit(RenderSceneUpdate)
        -> RenderSceneCommit
            ├── RenderSceneSnapshot
            └── RenderSceneChangeSet
```

`RenderSceneRevision` changes only when committed RunenRender-owned scene state changes.
It does not automatically change when foreign semantic-input generations,
representation availability/residency, RunenGPU device generations, or derived
cache/history generations change.

If an adapter projects changed source meaning into renderer-owned scene state, it uses a
normal scene commit.

There is no universal renderer revision spanning scene, input, availability, device, and
cache state.

Generic producer identity, contribution identity, and producer lifecycle are not
foundational renderer semantics. Source domains or Runenwerk adapters own source
lifecycle and may translate source retirement into atomic `RenderObjectId` removals.

Required identity separation is:

```text
source identity
!= asset identity
!= ECS identity
!= RenderObjectId
!= RenderRepresentationId
!= RepresentationElementId
!= RunenGPU identity
```

Only create renderer identities when RunenRender owns an invariant requiring them.
Runtime identities do not imply persistence, wire, replay, artifact, or cross-process
identity.

Do not create a universal scene graph or generic relationship ontology. Introduce typed
relationship families only when concrete renderer semantics require them.

## Observation, sampling, and output law

`RenderObservationSpec` is renderer-specific. It may describe image-like observations,
coordinated multiview, or non-image-grid renderer probes. It does not authorize a
family-wide Observation framework or a universal 2D grid assumption.

Separate:

```text
semantic sampling support
    what scene/time support contributes to requested meaning

algorithmic sampling strategy
    how a RenderMethod estimates/evaluates that support
```

`RenderOutputSpec` owns semantic result meaning. Keep separate:

```text
output-value meaning
result topology
applicable radiometric/transport representation
semantic accuracy/tolerance
numeric realization
physical RenderOutputBinding
```

Storage format or destination must not define radiance, depth, identity, or another
semantic output.

## Request and execution-policy law

`RenderRequest` is semantic:

```text
observations
requested outputs
semantic tolerance / accuracy
explicitly permitted semantic approximation
optional typed renderer-semantic constraints/extensions
```

Memory, latency, device, surface, wait, and product-preset policy are separate execution
or Runenwerk concerns unless a concrete semantic contract explicitly makes one relevant.

Execution pressure may choose a semantics-preserving alternative, use an explicitly
permitted bounded semantic approximation, or reject. It must not silently substitute a
different semantic result.

Method identity is requested only when method identity itself is semantic intent, for
example an explicit stylization or reproducibility contract.

## Representation law

`RenderRepresentation` is an open renderer-visible representation family. No closed root
`Mesh | SDF | Volume | ...` ontology is accepted.

Keep distinct:

```text
intrinsic representation contract/evidence
!= request-relative applicability
!= current availability / realization
```

Request-static applicability is derived by semantic planning. Applicability that depends
on current foreign semantic facts remains an explicit prerequisite until actual bindings
exist.

Source availability, renderer-derived realization residency, and RunenGPU physical
residency remain separate owner facts.

`RepresentationOffer` may exist as a derived planning/admission view; it is not root
semantic authority.

Representations expose narrow versioned protocols and narrow semantic results rather
than one universal provider/intersection interface or unchecked string/`Any`/`TypeId`
escape hatch.

## Request-scoped semantic-input law

There is no universal dynamic-input ontology.

Concrete observations, representations, materials, media, emitters, render methods, or
renderer extensions define the typed semantic facts they require. Source owners publish
values, generations, coverage, validity, and provenance. Physical accessibility remains
separate.

A normalized `RenderInputSet` may collect bindings as invocation plumbing; it does not
own their semantics. A RunenGPU resource identity is never the semantic identity of the
value it carries.

## RenderMethod, planning, and admission law

`RenderMethod` is a coherent renderer-owned algorithm family. Method-internal raster,
ray, compute, wavefront, field, regional, volume, or hybrid topology remains private.

`RenderPlan` is a **conditional device-independent semantic plan**.

It describes candidate semantic solution families and their explicit prerequisites. It
may derive normalized abstract RunenGPU capability/work requirements, but it does not
require current GPU handles, residency, acquired surfaces, physical allocations,
concrete pipelines, submissions, or concrete output bindings.

The responsibilities are:

```text
planning
    what solution families may satisfy the request
    and what prerequisites they require

semantic binding admission
    whether current request-scoped foreign semantic facts
    satisfy those prerequisites

execution admission
    which semantically admitted candidate can execute now
```

Semantic binding admission may evaluate predicates, eliminate candidates, and
specialize choices declared by the plan. It may not invent a new semantic alternative
outside the `RenderPlan` solution space.

Execution admission combines semantically admitted candidates with current
representation availability/realization, physical output bindings, RunenGPU environment
facts, and execution requirements.

`unsupported != unavailable`.

Only an admitted plan lowers:

```text
AdmittedRenderPlan
    -> RenderWorkSet
        -> RunenGPU
```

No second GPU resource/lifetime/hazard/submission/surface/error model exists in
RunenRender.

## Results, derived state, and incremental correctness

`RenderResult` records applicable renderer-semantic outcome, validity, provenance,
approximation, and completion evidence. Result values may remain in physical bindings or
retained renderer products rather than being embedded in one Rust value.

Derived renderer state remains non-authoritative, dependency-tracked, validated before
reuse, discardable/reconstructable where declared, and bounded or pressure-reporting. A
cache hit changes cost, not semantic truth.

For the same admitted semantic inputs:

```text
incremental evaluation
    ==
clean/full evaluation
under owner-declared equality/tolerance
```

Missing or untrusted evidence widens invalidation or triggers owner-local
revalidation/reconstruction/resynchronization. This does not create a global Runenwerk
transaction, revision, snapshot, or resync authority.

## Dependency-ordered RunenRender track

The durable sequence is:

```text
R0  normative semantic-rendering architecture
R1  scene lineage + minimal renderer identity
R2  space/time + observation/output semantics
R3  representations + intrinsic validity + minimum appearance/scene participation
R4  RenderMethod + conditional device-independent semantic planning
R5  semantic bindings + binding admission + operational availability/output bindings + execution admission
R6  first complete semantic renderer + public RunenGPU lowering
R7  derived state + reconstruction/history/sessions + multiview/multi-output/advanced output integration
R8  generality + scale + conformance + extraction readiness
RX  clean standalone authority transfer
```

R1 begins with only scene lineage, minimal renderer identity, atomic mutation, immutable
snapshots, and structural change evidence. It does not pre-create producer,
representation, observation, input, relationship, or GPU authority.

R6 proves both a conventional perspective HDR radiance/depth/object-identity workload
and one non-image-grid scalar renderer probe through the permanent R1-R5 contracts and
public RunenGPU only.

R8 must prove at least two independent source/adaptor families, two meaningfully
different representation/query families, two meaningfully different render-method
families sufficient to validate the method/planning abstraction, non-camera observation,
GPU-produced semantic input, multi-observation/output sharing, bounded scale behavior,
incremental/full equivalence, public RunenGPU-only lowering, comparison with a simpler
renderer, and extraction readiness.

RX is mechanical authority transfer after R1-R8 are accepted; it is not where renderer
architecture is invented.

## Future shared logical Plan compatibility

This decision remains correct whether a future shared logical Plan architecture is
accepted or rejected.

A shared Plan layer may express, compose, inspect, partition, or orchestrate renderer
operations. It may not absorb RunenRender-owned scene semantics, observation semantics,
representation validity, render methods, renderer planning invariants, semantic binding
admission, execution admission, or renderer-specific RunenGPU lowering.

```text
shared logical Plan
    -> RunenRender semantic request / native planning
        -> RunenRender admission / lowering
            -> RunenGPU
```

Provider realization does not transfer semantic ownership.

## Rejected alternatives

### Keep `semantic scene-to-image formation` as the mission

Rejected because renderer semantics already require non-color outputs, non-image-grid
observations, multiview, spectral/polarized domains, deep/sparse results, and
renderer-semantic probes. Image topology is too narrow as the root model.

### Adopt generic `semantic observation formation`

Rejected because it would make RunenRender a candidate owner for unrelated world,
scientific, spatial, telemetry, or application observations whose correctness does not
depend on renderer semantics.

### Adopt generic measurement formation

Rejected because it is too broad for renderer ownership and too narrow for
stylized/nonphysical rendering.

### Put producer lifecycle in the renderer scene kernel

Rejected because source producer identity/lifecycle belongs to source domains or
integration. The renderer needs atomic semantic object mutation, not a generic producer
ontology.

### Define representation applicability as permanent scene state

Rejected because applicability is request-relative and may depend on current
request-scoped semantic bindings.

### Combine semantic planning and current physical availability

Rejected because semantic legality must not depend on residency, current device state,
or physical output availability.

### Create universal renderer revision, observation, relationship, or input frameworks

Rejected because independent semantic authorities retain owner-local versions and
concrete renderer consumers should own only the contracts they actually need.

## Consequences

Positive consequences:

- conventional images, technical outputs, probes, multiview, spectral/polarized, and
  stylized rendering fit one renderer semantic model without creating a generic query
  engine;
- source, scene, request, availability, realization, and GPU state no longer collapse
  into one revision or plan;
- R1 begins with the smallest stable renderer kernel;
- planning is testable without a current GPU environment;
- current execution pressure cannot silently redefine requested meaning;
- future shared logical planning can compose RunenRender without taking renderer
  semantic authority;
- clean extraction remains possible without compatibility/mirror authority.

Accepted costs:

- image-centric current documentation and the old R-phase decomposition must be
  reconciled before R1;
- convenient aggregates such as `RepresentationOffer` or `RenderInputSet` may remain
  derived/plumbing views rather than root authority;
- R5 must track semantic binding validity and physical executability without conflating
  them;
- protocol/method abstractions require multiple real proofs before broad stabilization.

## Fitness functions

This decision remains healthy when:

1. a cold reviewer can state what makes a computation renderer-semantic;
2. no canonical authority defines RunenRender exclusively by image topology;
3. scene, request, request-scoped input, availability, realization, and GPU facts remain
   distinguishable;
4. source producer lifecycle is not required by the scene kernel;
5. renderer observation remains renderer-specific;
6. output meaning is independent of physical output destination;
7. representation evidence, request applicability, and availability remain separate;
8. planning works without current GPU handles/residency/surfaces;
9. binding admission may narrow but not widen the semantic solution space;
10. execution admission cannot silently weaken requested semantics;
11. incremental and full evaluation agree for the same admitted semantic inputs;
12. all physical GPU execution uses public RunenGPU contracts;
13. each R-phase proof appears only after its owning semantics exist;
14. no compatibility, forwarding, mirror, or duplicate semantic authority is created.

## Activation

This ADR does **not** authorize RunenRender Rust implementation.

R1 may be activated only after the complete R0 authority set is merged, accepted-main
validation is green, exact current `main` is re-resolved, and a fresh R1
declaration/consumer census produces a new owning issue.
