---
title: RunenRender Architecture and Decomposition Design
description: Canonical long-term semantic-rendering, scene, observation, representation, planning, admission, RunenGPU lowering, scalability, conformance, and extraction architecture for RunenRender.
status: accepted
owner: render
layer: framework/render
canonical: true
last_reviewed: 2026-09-10
related_docs:
  - ../../architecture/repository-family-architecture.md
  - ../../adr/accepted/0014-repository-family-extraction-boundaries.md
  - ../../adr/accepted/0015-separate-gpu-execution-from-rendering.md
  - ../../adr/accepted/0017-cross-authority-consistency-and-graph-semantics.md
  - ../../adr/accepted/0018-semantic-federation-and-physical-realization.md
  - ../../adr/accepted/0021-ratify-runenrender-semantic-rendering-architecture.md
  - ../active/runengpu-architecture-design.md
  - ../active/runengpu-g3-access-work-graph-design.md
  - ../active/runengpu-shader-authoring-artifact-boundary.md
  - ../active/runenrender-internal-decomposition-execution-plan.md
  - ../active/runen-family-operational-hardening-design.md
  - ../../reports/investigations/2026-08-04-runenrender-long-term-capability-and-scalability-review.md
  - ../../reports/investigations/runenrender-extraction-investigation.md
  - ../../reports/investigations/runen-family-operational-hardening-investigation.md
  - ../../reports/investigations/runengpu-runenrender-application-domain-fit.md
  - ../../workspace/planning/roadmap.md
---

# RunenRender Architecture and Decomposition Design

## Status and authority

This document is the sole canonical RunenRender architecture authority inside
Runenwerk. ADR 0021 fixes the R0 semantic-rendering decision; this design expands it
into the long-term framework model and proof constraints.

Fixed decisions include:

- repository identity and one-package initial shape;
- dependency `RunenRender -> RunenGPU`;
- no direct/private WGPU ownership;
- semantic-rendering ownership rather than image-topology ownership;
- one immutable renderer-local scene lineage;
- separation of scene state, request state, request-scoped semantic inputs, operational
  availability, physical realization, and derived state;
- renderer-specific observation semantics without a universal Observation framework;
- output meaning independent of result topology, numeric realization, and physical
  destination;
- heterogeneous representations through narrow versioned protocols;
- request-relative representation applicability distinct from availability/residency;
- coherent render methods distinct from requested semantic meaning;
- conditional device-independent planning;
- semantic binding admission distinct from execution admission;
- generic lowering through public RunenGPU contracts;
- incremental/full semantic equivalence;
- bounded pressure, diagnostics, variants, sessions, histories, and derived state;
- clean internal proof followed by mechanical external cutover.

Future implementation decisions include exact Rust layouts and names below this semantic
vocabulary, scene-storage structures, extension traits, shader code-generation strategy,
method-internal execution topology, concrete advanced methods/representation families,
and any dynamic-library ABI/plugin packaging.

No RunenRender Rust implementation, external repository population, dependency change,
or R-phase activation is authorized by this design alone.

## Mission

RunenRender owns **semantic rendering**:

> the formation of renderer-semantic results from renderer-local scene state and
> explicit request-scoped rendering semantics, together with renderer-native planning,
> admission, and lowering that preserve those semantics through RunenGPU execution.

The minimum relation is:

```text
RenderSceneSnapshot
+ RenderRequest
+ admitted request-scoped semantic inputs
    -> RenderResult
```

An image is one result topology. It is not the root ontology.

RunenRender may therefore own conventional HDR image formation, depth/distance,
normals/orientation, identity/segmentation, motion, transmittance, spectral or polarized
results, multiview, deep/sparse results, scalar renderer probes, stylized/nonphysical
rendering, reconstruction, overlay composition, and presentation intent when their
correctness depends on renderer semantics.

RunenRender is not a generic observation engine, scientific measurement engine, world
query service, simulation framework, image-processing framework, or arbitrary
compute-to-pixels layer.

### Ownership test

A computation belongs to RunenRender only when semantic correctness materially depends
on renderer-owned concepts such as:

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

Producing pixels, using geometry, evaluating an SDF, returning a scalar, or using a GPU
is insufficient by itself.

## Explicit non-ownership

RunenRender does not own:

- ECS or host-world storage, entity lifecycle, queries, or scheduling;
- world generation, chunk streaming, or vertical-domain product semantics;
- source assets, authoring graphs, asset persistence, or editor documents;
- field/SDF mathematical authority;
- reusable host-neutral spatial authority;
- simulations, physics, or particle-simulation meaning;
- UI state, layout, shaping, focus, accessibility, interaction, or hit testing;
- windows, event loops, tracking/XR runtime lifecycle, or product presentation policy;
- generic GPU resources, work validation, access/hazards, realization, allocation,
  submission, progress/completion, readback mechanics, surfaces, or device outcomes;
- shader filesystem discovery/watching, authoring compiler policy, or product
  last-known-good policy;
- MaterialX, USD, glTF, OCIO, ACES, or another interchange/configuration standard as
  source authority;
- distributed process/network orchestration;
- image, dataset, or video encoding;
- product recovery, severity, quality presets, or application lifecycle.

Runenwerk owns cross-framework adapters and host/product policy. RunenGPU owns generic
physical GPU execution.

## Repository and package

```text
repository: dornglut/runen-render
package: runen-render
crate: runen_render
depends on: runen-gpu
```

RunenRender initially contains one public package. Internal modules carry responsibility
boundaries until independent reuse, release/versioning, ABI, backend, compile-time, or
measured build-cost pressure proves another package.

Do not initially create:

```text
runenrender-core
runenrender-gpu
runenrender-wgpu
runenrender-macros
runenrender-plugins
runenrender-capture
runenrender-compat
```

RunenRender must not depend on WGPU, Winit, Runenwerk, RunenECS, RunenSDF, RunenUI, or
application/editor/world/asset/product packages. Adapters translate source-domain facts
into RunenRender contracts; RunenRender lowers only through public RunenGPU contracts.

## Semantic model

The minimal normative ontology is:

```text
RenderSceneSnapshot
    one immutable committed renderer-local semantic scene state

RenderRequest
    one request-scoped renderer-semantic intent

request-scoped semantic inputs
    foreign/source-owned semantic facts required by concrete renderer contracts

RenderResult
    renderer-semantic outcome
```

`RenderMethod` is not part of minimum requested meaning.

The architecture preserves these distinctions:

```text
scene state
!= request state
!= request-scoped foreign semantic state

requested semantics
!= algorithm choice
!= current executability
!= physical realization

semantic accuracy
!= numeric realization

semantic output meaning
!= physical output binding
```

An empty `RenderSceneSnapshot` is valid. Do not create parallel foundational
"scene rendering" and "scene-less rendering" APIs.

## Scene authority

```text
RenderSceneStore
    -> commit(RenderSceneUpdate)
        -> RenderSceneCommit
            ├── RenderSceneSnapshot
            └── RenderSceneChangeSet
```

### `RenderSceneStore`

`RenderSceneStore` owns renderer-local committed semantic scene state, the
`RenderSceneRevision` lineage, atomic renderer-scene mutation, immutable snapshot
publication, and scene-owned change evidence.

It may internally use persistent tables, generation-indexed arenas, structural sharing,
copy-on-write pages, paged storage, lazy views, or compiled dense tables. A successful
small change must not require a mandatory deep copy proportional to the total scene.

### `RenderSceneUpdate`

The R1 foundational update contract mutates renderer-object presence through atomic
insert and remove operations plus whatever concrete R1 validation is actually owned.
R1 does not define same-identity replacement because identity/presence contains no
replaceable renderer-semantic record. Replacement begins only in the first later phase
that owns concrete replaceable object state and defines its equality, revision, and
change-evidence law.

Generic producer identity, contribution identity, producer retirement semantics, and
producer namespaces are not part of the renderer scene kernel.

A Runenwerk/source adapter may retain its own source-to-`RenderObjectId` map and submit
one atomic removal update when a source producer disappears.

### `RenderSceneSnapshot`

A snapshot is immutable and coherent for one scene revision. Planning/lowering must not
reach back through it into mutable ECS, simulation, source-asset, UI-runtime, host-window,
or product state.

### Scene revision law

`RenderSceneRevision` changes only when committed RunenRender-owned scene state changes.
Examples include object insertion/removal, committed representation-record replacement,
material assignment, and renderer-semantic relationship mutation.

It does not automatically change because:

```text
foreign semantic-input generation changed
representation availability/residency changed
RunenGPU device generation changed
derived cache/history generation changed
```

If an adapter projects changed source meaning through a new renderer-scene commit, that
commit changes the scene revision normally.

There is no universal `RenderRevision` spanning scene, input, availability, device, and
cache facts.

## Identity

Required separation:

```text
source-domain identity
!= asset identity
!= ECS identity
!= RenderObjectId
!= RenderRepresentationId
!= RepresentationElementId
!= RunenGPU resource identity
```

`RenderObjectId` is stable renderer-semantic object identity. Representation replacement
must not silently change object identity.

`RenderRepresentationId` identifies one renderer-visible representation of an object.
`RepresentationElementId` exists only for representation protocols that genuinely need
stable representation-local element identity.

Do not create public renderer IDs merely for conceptual symmetry. A renderer ID is
introduced only when RunenRender owns a semantic invariant requiring independent
identity.

Runtime renderer identities are not persistence, wire, replay, cache, artifact, or
cross-process authority unless a separately versioned contract promotes them.

## Relationships

Do not create a universal scene graph or stringly relationship ontology.

Concrete typed relationship families may include instance/prototype, material
assignment, medium boundary, visibility grouping, light/shadow linking, holdout/matte,
or renderer overrides only when their semantics are required.

Each relationship family owns endpoint validity, referential constraints, dependency,
and invalidation behavior. Adding a relationship family does not widen all renderer
objects into a universal property bag.

## Space and time

Renderer-semantic space may include coordinate frame, units, semantic transforms,
orientation/handedness where applicable, and spatial support/coverage.

Renderer-semantic time may include render time/interval, exposure/shutter interval,
temporal support/validity, and motion interval.

Source domains retain authoritative coordinate/time meaning. Runenwerk retains clocks,
scheduling, tracking, late-latching, and product timing policy.

View/region rebasing, GPU-local coordinates, floating-point packing, and precision
origins used only for numerical realization are planning/lowering concerns rather than
scene meaning.

Field/distance representations must classify transforms as exact, conservative,
approximate, or invalid where that distinction affects their semantic guarantees.

## Observation semantics

`RenderObservationSpec` is renderer-specific. It is not ADR 0017's conceptual generic
Observation relationship and does not authorize a family-wide Observation framework.

Potential observation forms include:

```text
perspective
orthographic
panorama / fisheye / cubemap
stereo / foveated region sets
single-ray or scalar renderer probe
surface-attached renderer probe
bounded versioned custom renderer observation
```

`RenderView` may remain convenience vocabulary for view/image families; it is not the
root of all renderer observations.

One request may contain coordinated observations. No universal 2D image-grid assumption
is allowed.

## Semantic sampling support versus algorithmic sampling

Semantic sampling support describes what region of scene/time contributes to requested
meaning, for example pixel footprint, ray support, lens support, filter support, shutter
interval, or temporal integration support.

Algorithmic sampling strategy describes how a `RenderMethod` evaluates or estimates that
support, for example sample count/sequence, Monte Carlo pattern, adaptive sampling,
importance sampling, termination, or work distribution.

```text
semantic sampling support
!= algorithmic sampling strategy
```

Different valid strategies may satisfy the same requested semantics under the declared
equality/tolerance contract.

## Output semantics

`RenderOutputSpec` separates independent semantic axes.

### Output-value meaning

Examples include:

```text
radiance / irradiance
transmittance
depth / distance
surface orientation
motion / velocity
object / instance identity
segmentation
variance / confidence
sample count
derivative
bounded versioned renderer diagnostic/custom semantic
```

### Result topology

Examples include:

```text
scalar
ordered samples
2D sample lattice
coordinated image/region set
sparse samples
deep variable-count samples
bounded versioned custom topology
```

### Radiometric/transport representation

Where applicable, an output may use monochromatic, RGB radiometric approximation,
spectral, or polarized spectral representation. These axes are not attached to outputs
where they are meaningless.

### Accuracy versus numeric realization

Semantic tolerance, error bound, confidence, physical interpretation, and meaningful
range are distinct from f16/f32/f64, mixed precision, fixed point, quantization, texture
format, buffer format, or packing.

### Physical output binding

```text
RenderOutputSpec
    semantic result contract

RenderOutputBinding
    concrete physical destination for one execution
```

A texture, buffer, readback target, or acquired surface does not define radiance, depth,
or identity meaning.

## `RenderRequest`

A render request is the semantic envelope:

```text
observations
requested outputs
semantic tolerances / accuracy requirements
explicitly permitted semantic approximation
optional typed renderer-semantic constraints/extensions
```

Do not universally include memory budget, latency target, device preference, surface
identity, GPU format, wait policy, or product quality preset.

Execution/product policy may translate into a `RenderRequest` plus separate execution
requirements.

A request constrains `RenderMethod` identity only when method identity itself is part of
requested semantics, such as explicit stylization, reproducibility, or a method-specific
semantic extension. Otherwise planning chooses the method.

## Execution requirements and semantic alternatives

Execution requirements may include latency, memory/work budget, wait bound, device
constraint, or determinism/reproducibility requirements that do not themselves redefine
requested semantic output.

Distinguish:

```text
semantics-preserving alternative
    different realization/algorithm; same requested semantic contract

bounded semantic approximation
    same declared semantic quantity/result family with an explicitly weaker guarantee
    inside the allowed approximation envelope

semantic relaxation/substitution
    requested meaning changes; explicit authorization required
```

Pressure may choose the first two when allowed or reject. It must never silently perform
the third.

## Render objects and representations

`RenderObject` is the stable renderer-local semantic unit. It may originate from an ECS
entity, authored node, asset instance, procedural region, simulation, aggregate, or no
stable source object.

`RenderRepresentation` is one renderer-visible form through which an object participates
in semantic rendering. Representation families remain open: analytic primitives,
raster geometry, fields/SDF, volumes, particles/populations, fibers, learned/neural
representations, regional summaries, overlay geometry, or future forms do not require a
closed root enum.

One object may expose multiple representations simultaneously.

## Representation contract, evidence, applicability, and availability

Intrinsic representation contract/evidence may include:

```text
representation identity where required
supported versioned protocols
coordinate/unit conventions
spatial/temporal coverage where intrinsic
accuracy/error vocabulary
exact/conservative properties
refinement semantics
content provenance / source generation where meaningful
```

Request-relative applicability is derived from representation evidence plus the current
request, observation support, requested time/output, required accuracy, and method or
protocol requirements.

Applicability may have two stages:

```text
request-static applicability
    decidable from scene/request/contracts/evidence

binding-dependent applicability
    decidable only after current request-scoped semantic bindings exist
```

Operational availability/realization is separate again. Possible facts may include
available, pending, resident, partial, unavailable, or failed realization.

Do not collapse:

```text
intrinsic representation evidence
request-relative applicability
current availability
physical realization
```

Source owners may own underlying availability. RunenRender owns renderer-specific
interpretation/admission. Renderer-derived structures may have renderer-owned
availability. RunenGPU owns physical GPU allocation/residency.

`RepresentationOffer` may exist as a derived planner/admission candidate view combining
these facts. It is not foundational semantic authority.

## Versioned query protocols and narrow results

Representations support small versioned semantic protocols such as surface, visibility,
volume, attribute, motion, refinement, or population queries when real consumers require
them.

A protocol defines semantic inputs/outputs, space/time conventions, error meaning,
compatibility, and structured unsupported behavior. It does not prescribe one Rust trait
object, shader-language interface, dispatch per query, or GPU representation.

Use narrow results such as `SurfaceHit`, `VisibilityResult`, `VolumeInterval`, or
`TransmittanceResult`. Do not require every representation to fabricate UVs, normals,
materials, velocity, emission, or medium data in one universal interaction record.

## Materials, media, emitters, environments, and appearance

Separate authored/imported source meaning, renderer semantic material/medium/emitter
meaning, method-compiled program meaning, and RunenGPU realization.

Renderer semantic appearance preserves room for physical, technical, measured,
spectral, stylized, and nonphysical models without defining one mandatory universal
appearance ontology.

RunenRender owns only the minimum stable semantic contracts required by actual methods.
Authoring graphs/import remain outside RunenRender.

## Request-scoped semantic inputs

Do not create a universal dynamic-input ontology.

Concrete consumers such as an observation, representation protocol, material, medium,
emitter, render method, or renderer extension define typed input requirements.

Keep distinct:

```text
consumer-defined semantic requirement
source-owner-produced semantic value/binding/generation/provenance
physical accessibility/realization
```

A normalized `RenderInputSet` may collect bindings for invocation plumbing; it does not
own their semantics. A GPU resource identity never becomes semantic input identity.

## `RenderMethod`

A render method is a coherent renderer-owned algorithm family. It may declare:

```text
supported observations and outputs
supported transport/radiometric domains
required protocols/materials/media/emitters
required request-scoped semantic inputs
accuracy and legal approximation
structural/numerical/statistical reproducibility properties
execution capability requirements
```

Method-internal topology may use rasterization, ray tracing, compute, wavefront queues,
field traversal, regional propagation, volume integration, persistent queues, indirect
work, or hybrids. It remains private.

Do not expose arbitrary mix-and-match public strategy objects when the combinations
would weaken method-level semantic validity.

At least two meaningfully distinct methods are required before stabilizing a broad
public method-extension abstraction.

## Conditional semantic planning

`RenderPlan` is device-independent and conditional.

Planning consumes:

```text
RenderSceneSnapshot
+ RenderRequest
+ representation contracts/evidence
+ RenderMethod contracts
+ semantic-input requirements
```

and derives:

```text
candidate semantic methods/representations
request-static applicability
required protocols and semantic-input contracts
binding predicates / unresolved semantic prerequisites
legal semantics-preserving alternatives
legal approximation envelopes
semantic dependencies
logical output obligations
normalized abstract RunenGPU capability/work requirements
```

A plan answers:

> Which solution families can satisfy this request provided their declared semantic
> prerequisites are admitted?

Planning requires no current GPU handles, residency, surface acquisition, concrete
output binding, physical allocation, concrete pipeline, or submission.

Semantic binding admission may evaluate declared predicates, eliminate candidates, and
specialize declared choices. It may not invent new semantic alternatives outside the
`RenderPlan` solution space.

## Semantic binding admission

Concrete request-scoped semantic bindings are admitted by the renderer consumer that
requires them.

Applicable checks may include semantic type, source generation, time/interval,
coordinate contract, coverage, accuracy/confidence, provenance, and compatibility with
the request or representation.

This is consumer-owned semantic compatibility checking in the ADR 0017/0018 sense. It
is not a global family admission service.

## Execution admission

Execution admission combines semantically admitted candidates with current facts:

```text
representation availability/realization
physical RenderOutputBinding values
RunenGPU capabilities / device generation
execution requirements / pressure
```

and selects one currently executable realization.

The normative distinction is:

```text
planning
    what solution families may satisfy the request and their prerequisites

semantic binding admission
    whether current semantic bindings satisfy those prerequisites

execution admission
    which semantically admitted candidate can physically execute now
```

`unsupported != unavailable`.

The selected output is `AdmittedRenderPlan`. No public intermediate type is required
merely to mirror every conceptual stage.

## RunenGPU lowering

Only an admitted plan lowers:

```text
AdmittedRenderPlan
    -> RenderWorkSet
        -> RunenGPU
```

RunenRender may own renderer-specific representation compilation descriptions,
visibility/material/medium/emitter/method/reconstruction/overlay/output program meaning,
derived-state updates, semantic merge work, and typed renderer use of RunenGPU
contracts.

RunenGPU owns GPU context/device admission, generic resource identity/allocation,
access/hazard validation, backend program/layout/pipeline realization, submission,
progress/completion/readback mechanics, surface lifecycle, and device-loss outcomes.

No second GPU lifetime, error, resource, hazard, submission, or surface authority exists
in RunenRender.

## Color, overlay, reconstruction, and presentation intent

RunenRender owns renderer-semantic color/transfer behavior where it affects render-result
meaning, not OS display management or file encoding.

RunenRender may compose renderer-neutral overlay contributions. RunenUI retains UI
state/layout/interaction/accessibility/text semantics; a Runenwerk bridge translates
accepted paint facts into renderer participation.

Spatial/temporal reconstruction, accumulation, and denoising belong to RunenRender when
they determine renderer-result meaning. Their histories/caches are derived state.

RunenRender may own semantic presentation intent. RunenGPU owns low-level surface
execution; Runenwerk owns windows, event loops, XR runtime sessions, presentation
policy, artifact policy, and product recovery.

## `RenderResult`

A result is renderer-semantic outcome evidence, not merely successful GPU submission and
not necessarily a Rust-owned byte container.

Applicable result evidence may include scene revision, request/observation correlation,
outputs formed, semantic-input generations, method/revision, representation/protocol
provenance, declared approximation, semantic validity/error evidence, and
completion/partial-completion state.

Actual values may remain in retained renderer products, physical output bindings,
readback results, or presentation destinations.

GPU backend/device/timing/submission IDs are diagnostics unless a concrete semantic
contract explicitly requires them.

## Derived state and sessions

Derived renderer state may include compiled representations, acceleration structures,
renderer-owned residency/realization, transport estimates, history, reconstruction, or
output accumulation.

Derived state is non-authoritative, discardable/reconstructable where declared,
dependency-tracked, validated before reuse, and bounded or pressure-reporting. A cache
hit changes cost, not semantic truth.

Distinguish source/representation availability, renderer-derived realization residency,
and RunenGPU physical allocation/residency.

`RenderSession` is introduced only when a concrete method requires continuity across
invocations, for example progressive sampling, accumulation, convergence, cancellation,
or temporal reconstruction. Session continuation across scene revisions requires
explicit compatibility/invalidation semantics and never observes partial scene mutation.

## Incremental correctness and resynchronization

For the same admitted semantic inputs:

```text
incremental evaluation
    ==
clean/full evaluation
under owner-declared equality/tolerance
```

Unknown, incompatible, pruned, or untrusted narrow evidence widens invalidation or
triggers owner-local recovery:

```text
scene evidence lost
    -> full renderer-scene resynchronization

input generation evidence lost
    -> revalidate / rebind input

availability evidence lost
    -> refresh availability

derived dependency evidence lost
    -> invalidate / reconstruct affected derived state

session continuity lost
    -> restart affected session/history
```

There is no global Runenwerk transaction, revision, snapshot, or full-resync authority.

## Multi-device and distributed compatibility

`RenderWorkSet` does not permanently assume one device-local fragment. A future work set
may contain device-local partitions, merge work, readback work, or presentation work.

RunenRender owns semantic partitionability and merge meaning. Runenwerk or another
orchestrator owns remote transport, process lifecycle, retries, cluster scheduling, and
job policy.

Outputs merge according to their semantic meaning rather than one universal arithmetic
rule.

## Shader/program boundary

RunenRender owns renderer shader/kernel meaning, semantic variants, and method-specific
program families.

Runenwerk owns authoring source roots, package/source policy, compiler selection where
applicable, canonical artifact generation/source maps, watching/reload scheduling, and
product last-known-good policy.

RunenGPU owns canonical program admission, explicit interfaces/layouts,
specialization/binding compatibility, backend realization, and physical cache
compatibility.

Shader-language interfaces are implementation tools rather than representation/query
semantic protocol authority. Variant count and cold/warm compilation/cache pressure
must remain bounded, inspectable, and characterized.

## Trust, extension, and evolution

Extensible semantic families use namespaced semantic identity, schema/protocol version,
required/optional fields, compatibility rules, and structured unsupported outcomes.

Initial extension guarantees are source-level Rust contracts, not stable dynamic-library
ABI.

Untrusted or third-party authoring must not gain unrestricted host callbacks, arbitrary
backend access, or unrelated global-resource access. Product trust policy remains
Runenwerk-owned.

## Determinism and reproducibility

Useful determinism classes include:

```text
Structural
NumericalWithinTolerance
Statistical
BitwiseUnderConstrainedEnvironment
```

Applicable reproducibility facts may include random generator/revision, seed/stream
allocation, sample ranges, accumulation order, numerical mode, method/protocol revisions,
representation/input generations, and permitted device/backend facts.

Runenwerk owns persisted reproducibility bundles, redaction/privacy, artifact schemas,
and presentation.

## Scalability invariants

### Scene

```text
No mandatory deep copy per commit.
No mandatory full rebuild for a local change.
```

### Planning

```text
No mandatory all-object x all-method comparison.
Use indexed protocol, space, time, relationship, and observation data where needed.
```

### GPU work and submission

```text
RunenGPU work-node count scales with algorithm stages,
not logical object count.

No per-object CPU submission requirement.
GPU culling, compaction, indirect execution, and generated work remain possible.
```

### Memory and pressure

```text
All scene-derived, method-derived, history, residency,
diagnostic, variant, and output state is bounded or pressure-reporting.
```

### Detail

```text
Refinement is semantic-support- and error-driven.
Procedurally unbounded detail is never globally materialized.
```

### Observations and outputs

Compatible observations and outputs may share preparation, compiled representations,
acceleration, history, and other derived state.

### Diagnostics

Diagnostics are bounded and aggregatable. One failing population must not create an
unbounded message storm.

### Selection provenance

Exact selected protocols, representations, methods/revisions, relevant seeds, input
generations, and declared approximations remain inspectable.

The logical world may be finite or unbounded. Every concrete render execution operates
on a finite bounded working set.

## Founding end-to-end proof

The first complete renderer proof is intentionally small and must use permanent R1-R5
contracts:

```text
scene
    analytic sphere
    analytic plane
    field/SDF surface

appearance
    minimum diffuse material
    directional emitter

observation/output proof A
    perspective observation
    HDR radiance
    depth
    object identity

observation/output proof B
    non-image-grid scalar radiance probe

method
    one coherent direct-lighting method

execution
    public RunenGPU only
    minimal physical output bindings
    CPU reference probes
```

This proves both conventional image rendering and the broader-than-image semantic API.
It does not authorize public types named after SDF, direct lighting, preview, or the
founding implementation.

Multi-bounce transport, volumes, populations, temporal history, denoising, deep output,
differentiability, learned representations, distributed execution, and XR remain later
proofs against the same architecture.

## Conformance

Internal RunenRender proof eventually requires:

1. renderer scene commits/snapshots without Runenwerk/ECS/WGPU/RunenSDF/RunenUI types;
2. no mandatory deep copy per commit;
3. deterministic insert/remove presence mutation and atomic reject behavior;
4. equivalent full and incremental semantic result;
5. explicit change evidence and full-resynchronization fallback;
6. stable renderer object identity across representation replacement;
7. at least two independent source/adaptor families without a renderer producer ontology;
8. at least two meaningfully distinct representation/query families;
9. narrow protocol/result proof independent of one method;
10. explicit semantic space/time/observation/output contracts;
11. CPU-only deterministic conditional planning;
12. semantic binding admission distinct from current execution admission;
13. no representation applicability/residency conflation;
14. public RunenGPU lowering only, with no direct/private WGPU path;
15. typed CPU and GPU-produced request-scoped semantic-input proof;
16. renderer-semantic output/merge/reconstruction rules;
17. derived-state dependency/invalidation proof;
18. bounded memory/pressure/diagnostics/variants/sessions where retained;
19. stage-scaled GPU work and no per-object CPU submission requirement;
20. multi-observation and multi-output sharing evidence;
21. one founding method plus a meaningfully distinct second method through the same
    semantic boundaries;
22. non-camera/non-image-grid observation proof;
23. reproducibility/provenance facts without owning persistence policy;
24. no duplicate old renderer semantic path after accepted cutover.

External proof additionally requires independent locked validation, public downstream
consumption, exact accepted RunenGPU revision, provenance, operational conformance, and
clean Runenwerk cutover.

## Performance and scalability characterization

R8 characterizes at least:

```text
full versus incremental scene-update cost
small-change cost against total scene size
request-static and binding-dependent representation-selection cost
protocol query count/divergence
planning candidate count and pruning
work-node count against logical object count
CPU submission count
GPU culling/compaction/indirect behavior
resident/transient memory high-water marks
derived-state hit/miss/invalidation/reconstruction
multi-observation/multi-output sharing
variant count and cold/warm compilation
session convergence/cancellation if retained
comparison with a simpler direct renderer for the same proof
```

Performance evidence is diagnostic until separately accepted controlled budgets exist.
No private RunenGPU/WGPU bypass may make framework measurements look better.

## Current-source revalidation gate

Before every R implementation slice:

- resolve exact accepted `main`;
- repeat the affected declaration and direct/transitive consumer census;
- inspect current scene, request, input, representation, shader/program, output,
  diagnostics, test, example, and benchmark paths as applicable;
- verify identity conversions and persisted/wire uses;
- identify host/source reach-back;
- bind exact public, migration, deletion, proof, and guard scope;
- run repository baseline validation;
- stop for a new ADR/package/dependency/stable-format/compatibility path/backend escape or
  premature later-phase authority.

Historical investigations are evidence, not permission to skip current-source review.

## Definition of done

RunenRender extraction is complete only when:

- one independently validated `runen-render` package exists in `dornglut/runen-render`;
- it depends on an exact accepted RunenGPU revision and not WGPU;
- Runenwerk consumes only public renderer semantic contracts/adapters;
- RunenUI, RunenSDF, RunenSpatial, and RunenECS remain independently authoritative;
- at least two representation/query families and two meaningfully distinct render methods
  prove the shared boundaries;
- non-image-grid observation and GPU-produced semantic input prove the broader model;
- incremental scene lifecycle, request planning/admission, representation selection,
  output semantics, and derived invalidation pass;
- large-scene and bounded-work evidence passes;
- operational/performance/reproducibility facts pass;
- every active consumer is migrated;
- exact provenance is recorded;
- original Runenwerk semantic-rendering authority and temporary seams are deleted;
- no mirror, compatibility package, forwarding namespace, duplicate renderer, or private
  reach-through remains.

## Strategic reevaluation gates

Reconsider the architecture if:

- a materially smaller renderer satisfies all accepted proofs;
- protocols or representations become universal/stringly/runtime-heavy;
- snapshots require systematic deep copies;
- local changes require systematic full rebuilds;
- planning requires systematic all-object x all-method scans;
- GPU work-node or CPU-submission count scales directly with logical object count;
- optional methods cannot share coherent scene/request/observation/output semantics;
- semantic binding and physical execution admission cannot remain distinct;
- backend-neutral contracts repeatedly leak backend details;
- measured boundary cost materially exceeds simpler alternatives without reusable value;
- no meaningfully distinct second method or representation family exists.

Reevaluation is explicit architecture work, not permission for a hidden bypass.

## Relationship to future shared logical Plan work

RunenRender remains correct whether a future shared logical Plan platform is accepted or
rejected.

A shared Plan layer may express, compose, inspect, partition, or orchestrate renderer
operations. It may not absorb RunenRender-owned scene semantics, observation semantics,
representation validity, render methods, conditional render planning, semantic binding
admission, execution admission, or renderer-specific RunenGPU lowering.

Directionally:

```text
shared logical Plan
    -> RunenRender semantic request / native planning
        -> RunenRender admission / lowering
            -> RunenGPU
```

Provider realization does not transfer semantic ownership.