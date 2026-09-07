---
title: Ratify RunenRender Semantic Rendering Architecture
description: Accepted pre-R1 decision defining RunenRender semantic-rendering ownership, normalized scene/request/representation/planning/admission boundaries, and the dependency-ordered R0-R8/RX track.
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
  - ../../design/active/runenrender-decomposition-design.md
  - ../../design/active/runenrender-internal-decomposition-execution-plan.md
  - ../../architecture/repository-family-architecture.md
  - ../../workspace/planning/roadmap.md
---

# ADR 0021: Ratify RunenRender Semantic Rendering Architecture

## Decision

RunenRender owns **semantic rendering**:

> the formation of renderer-semantic results from renderer-local scene state and
> explicit request-scoped rendering semantics, together with renderer-native planning,
> admission, and lowering that preserve those semantics through RunenGPU execution.

The minimum semantic relation is:

```text
RenderSceneSnapshot
+ RenderRequest
+ admitted request-scoped semantic inputs
    -> RenderResult
```

An image is one result topology, not the root ontology. RunenRender may also form
renderer-semantic scalar, sparse, deep, multiview, spectral, polarized, diagnostic, or
other bounded versioned results when their correctness depends on renderer-owned
semantics.

RunenRender is intentionally narrower than generic observation, scientific measurement,
world querying, simulation, image processing, or generic GPU computation.

## Ownership test

A computation belongs to RunenRender only when correctness materially depends on one or
more renderer-owned semantic concerns such as:

```text
renderer-local scene participation
render observation semantics
render representations and query protocols
visibility
appearance / material / medium / emitter meaning
transport
semantic sampling support
reconstruction
renderer composition
render-output meaning
render-method validity
```

Producing pixels, using geometry, evaluating an SDF, returning a scalar, or executing on
a GPU is not sufficient by itself.

## Existing authority retained

ADR 0014 remains authoritative for repository-family independence, Runenwerk-owned
integration/adapters, clean extraction, exact-revision cutover, and rejection of shared
identity or compatibility mirrors.

ADR 0015 remains authoritative for the direct dependency and physical execution split:

```text
Runenwerk integration and host/product policy
    -> RunenRender
        -> RunenGPU generic GPU execution
            -> private backend
```

RunenRender must not own WGPU directly or recreate RunenGPU resource, access/hazard,
allocation, submission, surface, progress, readback, device, or error authority.

This ADR **narrowly supersedes ADR 0015's image-exclusive RunenRender mission wording**.
Statements such as `semantic image planning`, `semantic image formation`, or "how
prepared render-facing data becomes one or more images" remain valid examples but are
no longer exhaustive definitions of RunenRender ownership.

ADR 0017 remains authoritative for:

```text
One semantic invariant set has one authority.
```

It also remains authoritative for owner-local consistency, explicit foreign-owner
contracts, graph-semantic separation, incremental/full equivalence, shared-extraction
gates, and progressive disclosure. Its conceptual `Observation` vocabulary does not
become a universal RunenRender type.

ADR 0018 remains authoritative for semantic/physical separation, owner-local versions,
consumer-owned admission, provenance, explicit approximation, and the law that physical
realization may constrain but must not silently redefine semantics.

## Normative semantic model

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

An empty `RenderSceneSnapshot` is valid. The architecture does not create separate
foundational scene-rendering and scene-less-rendering models.

## Scene authority and revisions

RunenRender retains one renderer-local immutable scene lineage:

```text
RenderSceneStore
    -> commit(RenderSceneUpdate)
        -> RenderSceneCommit
            ├── RenderSceneSnapshot
            └── RenderSceneChangeSet
```

`RenderSceneRevision` changes only when committed RunenRender-owned scene state changes.
It does not automatically change because a foreign semantic-input generation,
representation availability/residency, RunenGPU device generation, or derived
cache/history generation changed.

If an adapter projects changed source meaning into renderer-owned scene state, it does
so through an ordinary scene commit and the scene revision changes normally.

There is no universal `RenderRevision`.

Generic producer identity, contribution identity, and producer lifecycle are not
foundational renderer semantics. A Runenwerk/source adapter may track its own
source-to-`RenderObjectId` correspondence and submit atomic renderer removals when a
source producer disappears.

## Identity and relationships

Required separation is:

```text
source identity
!= asset identity
!= ECS identity
!= RenderObjectId
!= RenderRepresentationId
!= RepresentationElementId
!= RunenGPU identity
```

Only create renderer-local identities when RunenRender owns an invariant that requires
independent identity. Runtime identities do not imply persistence, wire, replay,
artifact, or cross-process identity.

Do not create a universal scene graph or generic relationship ontology. Concrete typed
relationship families are introduced only when their endpoint validity, dependency,
and invalidation semantics are real.

## Observation, sampling, and outputs

`RenderObservationSpec` is renderer-specific. It may describe perspective,
orthographic, panoramic, fisheye, cubemap, stereo/foveated regions, scalar/single-ray
renderer probes, surface-attached renderer probes, or bounded versioned custom renderer
observations.

No universal 2D image-grid assumption is allowed.

Semantic sampling support answers what region of space/time contributes to a requested
quantity. Algorithmic sampling strategy answers how a `RenderMethod` estimates or
evaluates that support. These are distinct.

`RenderOutputSpec` separates:

```text
output-value meaning
result topology
applicable radiometric / transport representation
semantic accuracy / tolerance
```

from physical `RenderOutputBinding` destination and numeric storage/compute format.

## Render requests and execution requirements

`RenderRequest` is the semantic envelope:

```text
observations
requested outputs
semantic tolerances / accuracy
explicitly allowed semantic approximation
optional typed renderer-semantic constraints/extensions
```

Memory, latency, device preference, surface identity, wait policy, and product preset
policy are not universally part of request meaning. They remain execution or product
requirements.

Execution pressure may choose a semantics-preserving alternative, use an explicitly
permitted bounded approximation, or reject. It must not silently redefine the request.

Method identity belongs in a request only when the method itself is part of semantic
intent, such as an explicit stylization or reproducibility contract.

## Representations and protocols

`RenderRepresentation` remains an open renderer-visible representation family. No
permanent closed root enum such as `Mesh | SDF | Volume | ...` is accepted.

Keep distinct:

```text
intrinsic representation contract/evidence
!= request-relative applicability
!= current availability / realization
```

Intrinsic evidence may include protocol support, coordinate/unit conventions,
spatial/temporal coverage, error/accuracy vocabulary, exact/conservative properties,
refinement semantics, and content provenance.

Request-static applicability is derived during semantic planning. Binding-dependent
applicability is evaluated when actual request-scoped semantic bindings exist.

Availability/residency is operational rather than scene meaning. Source availability,
renderer-derived realization residency, and RunenGPU physical residency remain distinct
owner facts.

`RepresentationOffer` is not foundational authority; it may be a derived planning or
admission view.

Preserve narrow versioned query protocols and narrow semantic results. Do not introduce
one universal provider/intersection contract or an unchecked string/`Any`/`TypeId`
escape hatch.

## Request-scoped semantic inputs

There is no universal dynamic-input ontology.

Concrete observations, representations, materials, media, emitters, render methods, or
renderer extensions define typed semantic prerequisites. Source owners publish the
corresponding values, generations, coverage, validity, and provenance. Physical
accessibility is separate.

A normalized `RenderInputSet` may collect admitted bindings as implementation plumbing;
it is not itself semantic authority. A RunenGPU resource identity is never the semantic
identity of the value it carries.

## RenderMethod and alternatives

`RenderMethod` is a coherent renderer-owned algorithm family. It may declare supported
observations and outputs, required protocols and semantic inputs, accuracy,
determinism/reproducibility, legal approximation, and execution capability
requirements.

Method-internal raster, ray, compute, wavefront, field, regional, volume, or hybrid
topology remains private.

Distinguish:

```text
semantics-preserving alternative
bounded semantic approximation
semantic relaxation/substitution
```

The first preserves requested meaning. The second remains within an explicitly allowed
weaker guarantee. The third changes requested meaning and requires explicit
authorization.

## Conditional semantic planning

`RenderPlan` is a **device-independent conditional semantic plan**.

Planning consumes:

```text
RenderSceneSnapshot
+ RenderRequest
+ representation semantic contracts/evidence
+ RenderMethod contracts
+ declared semantic-input requirements
```

and produces candidate semantic solution families plus their explicit prerequisites.

Planning may derive normalized abstract RunenGPU capability/work requirements. It does
not require current GPU handles, residency, acquired surfaces, concrete output
bindings, physical allocations, concrete pipelines, or submissions.

A plan means:

> these solution families can satisfy the request provided their declared semantic
> prerequisites are admitted.

Semantic binding admission may evaluate declared predicates, eliminate candidates, and
specialize declared choices. It may not invent a new semantic alternative outside the
`RenderPlan` solution space.

## Semantic binding admission and execution admission

These responsibilities remain conceptually distinct:

```text
planning
    what solution families may satisfy the request
    and what prerequisites they require

semantic binding admission
    whether current foreign/request-scoped semantic facts
    satisfy those prerequisites

execution admission
    which semantically admitted solution can execute now
```

Execution admission combines admitted semantic bindings with current representation
availability/realization, physical output bindings, RunenGPU capabilities/device
generation, and execution requirements.

`unsupported != unavailable`.

The selected result is an `AdmittedRenderPlan`. No additional public intermediate Rust
type is required merely to mirror the conceptual distinction.

## RunenGPU lowering

Only admitted rendering lowers:

```text
AdmittedRenderPlan
    -> RenderWorkSet
        -> RunenGPU
```

RunenRender owns renderer-specific work meaning and lowering. RunenGPU owns generic
physical GPU execution.

## Results, derived state, and incrementality

`RenderResult` records applicable renderer-semantic outcome, validity, provenance,
approximation, and completion evidence. Result values may remain in retained physical
bindings or renderer products rather than being embedded in one Rust value.

Derived renderer state is non-authoritative, discardable/reconstructable where
declared, dependency-tracked, bounded or pressure-reporting, and validated before reuse.
A cache hit changes cost, not semantic truth.

For the same admitted semantic inputs:

```text
incremental evaluation
    ==
clean/full evaluation
under owner-declared equality/tolerance
```

Missing, incompatible, pruned, unknown, or untrusted narrow evidence widens invalidation
or triggers owner-local reconstruction/full resynchronization. This does not create a
global Runenwerk transaction or resync authority.

## Compatible advanced requirements retained

This decision preserves existing compatible requirements for:

- renderer-semantic color and presentation intent;
- renderer overlay composition without taking RunenUI authority;
- reconstruction, accumulation, and denoising where renderer-semantic;
- bounded derived state, sessions, diagnostics, variants, queues, histories, and memory;
- multi-observation and multi-output sharing;
- semantic partition/merge for multi-device/distributed orchestration while Runenwerk or
  another orchestrator owns transport/process/retry policy;
- renderer shader/kernel meaning while RunenGPU owns canonical program
  admission/realization and Runenwerk owns authoring source/artifact policy;
- bounded trust and extension mechanisms;
- structural, numerical-within-tolerance, statistical, and constrained-environment
  bitwise determinism distinctions;
- no mandatory deep-copy scene publication;
- no mandatory all-object x all-method planner scan;
- no per-object CPU submission requirement;
- RunenGPU work scaling with algorithm stages rather than logical objects;
- footprint/error-driven refinement;
- bounded/aggregatable diagnostics and inspectable provenance;
- clean extraction with one semantic source authority.

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

R1 must not introduce generic producer lifecycle, generic relationships,
representations, observations, inputs, or GPU placeholders before their owning
semantics exist.

R6 proves both conventional rendering and the broader API: at minimum one perspective
HDR radiance/depth/object-identity workload and one non-image-grid scalar renderer
probe, using permanent R1-R5 contracts and public RunenGPU only.

R8 must prove at least two independent source/adaptor families, two meaningfully
different representation/query families, two meaningfully different render-method
families sufficient to validate method/planning abstraction, non-camera observation,
GPU-produced semantic input, multi-observation/output sharing, incremental/full
equivalence, bounded scale characteristics, public RunenGPU-only lowering, a simpler
renderer comparison, and extraction readiness.

RX remains a mechanical authority transfer after R1-R8 are accepted; it is not a design
phase.

## Shared logical Plan compatibility

This decision is correct whether draft PR #282's shared logical Plan architecture is
accepted or rejected.

A future shared Plan layer may express, compose, inspect, partition, or orchestrate
renderer operations. It may not absorb RunenRender-owned scene semantics, observation
semantics, representation validity, render methods, renderer planning invariants,
semantic binding admission, execution admission, or renderer-specific RunenGPU
lowering.

Directionally:

```text
shared logical Plan
    -> RunenRender semantic request / native planning
        -> RunenRender admission / lowering
            -> RunenGPU
```

Provider realization does not transfer semantic ownership.

## Rejected alternatives

### Keep `semantic scene-to-image formation` as the mission

Rejected because the existing renderer semantics already require non-color outputs,
non-image-grid observations, multiview, spectral/polarized domains, deep/sparse
results, and renderer-semantic probes. Making `image` the root ontology incorrectly
constrains the semantic model.

### Adopt generic `semantic observation formation`

Rejected because it would make RunenRender a candidate owner for unrelated world,
scientific, spatial, telemetry, or application observations whose correctness does not
depend on renderer semantics.

### Adopt generic measurement formation

Rejected because it is both too broad for renderer ownership and too narrow for
stylized/nonphysical rendering.

### Put producer lifecycle in the renderer scene kernel

Rejected because producer identity/lifecycle belongs to source domains or integration.
The renderer needs atomic semantic object mutation, not a generic producer ontology.

### Define representation applicability as permanent scene state

Rejected because applicability is request-relative and may depend on current
request-scoped semantic bindings.

### Combine semantic planning and current execution availability

Rejected because it makes semantic legality depend on residency, device state, or
physical output availability and weakens CPU-only planning and preflight reasoning.

### Create a universal renderer revision or observation/input framework

Rejected because independent semantic authorities retain owner-local versions and
concrete consumers own only the input/observation contracts they need.

## Consequences

### Positive

- RunenRender can represent conventional images, technical outputs, multiview, probes,
  spectral/polarized results, and stylized rendering without becoming a generic query
  engine;
- source, request, availability, realization, and GPU state no longer collapse into one
  revision or plan;
- R1 starts with the smallest stable semantic kernel;
- representation and dynamic-input semantics are introduced only when their consumers
  exist;
- planning is testable without a current GPU environment;
- current execution pressure cannot silently redefine requested meaning;
- future shared logical Plan work can compose RunenRender without taking its semantic
  authority;
- extraction remains one-way and clean.

### Costs

- current image-centric documentation and phase numbering/responsibilities must be
  reconciled before R1;
- some current convenient aggregate concepts such as `RepresentationOffer` and
  `RenderInputSet` become derived/plumbing views rather than root authority;
- R5 admission must track both semantic binding validity and current physical
  executability without conflating them;
- later implementation must prove narrow protocol/method abstractions instead of
  stabilizing them from the founding method alone.

These costs are accepted because they remove ownership ambiguity before public API and
extraction pressure makes it expensive to correct.

## Fitness functions

This decision remains healthy when:

1. a cold reviewer can state what makes a computation renderer-semantic;
2. no current canonical authority defines RunenRender exclusively by image topology;
3. scene, request, request-scoped input, availability, realization, and GPU facts remain
   distinguishable;
4. source producer lifecycle is not required by the renderer scene kernel;
5. renderer observation remains renderer-specific rather than a family-wide wrapper;
6. output meaning is independent of physical output destination;
7. representation evidence, request applicability, and availability remain separate;
8. planning can run without current GPU handles/residency/surfaces;
9. semantic binding admission may narrow but not invent the planned semantic solution
   space;
10. execution admission cannot silently weaken requested semantics;
11. incremental and clean/full evaluation agree for the same admitted semantic inputs;
12. all physical GPU execution uses public RunenGPU contracts;
13. the R0-R8/RX sequence introduces each proof only after its semantics exist;
14. no compatibility, forwarding, mirror, or duplicate semantic authority is created.

## Delivery and activation

Issue #464 owns the bounded R0 documentation delivery.

This ADR does not authorize RunenRender Rust implementation. R1 may be activated only
after the complete R0 authority set is merged, accepted-main validation is green, exact
current `main` is re-resolved, and a fresh R1 declaration/consumer census produces a new
R1 owning issue.
