---
title: RunenRender Long-Term Capability and Scalability Review
description: Source-grounded review of the semantic, scalability, evolution, interoperability, and future-feature requirements needed before RunenRender implementation.
status: active
owner: render
layer: investigation
canonical: false
last_reviewed: 2026-08-04
related_docs:
  - ../../design/accepted/runenrender-decomposition-design.md
  - ../../design/active/runenrender-internal-decomposition-execution-plan.md
  - ../../design/active/runen-family-operational-hardening-design.md
  - ../../design/active/runengpu-architecture-design.md
  - ../../design/active/runengpu-shader-authoring-artifact-boundary.md
  - ../../architecture/repository-family-architecture.md
  - ../../adr/accepted/0015-separate-gpu-execution-from-rendering.md
  - ../../workspace/planning/roadmap.md
---

# RunenRender Long-Term Capability and Scalability Review

## Purpose

This report tests whether the current RunenRender direction can remain useful across
future renderer algorithms, scene representations, world scales, output products,
hardware capabilities, and application domains without another foundational rewrite.

It is supporting investigation evidence. The canonical decisions are recorded in
[RunenRender Architecture and Decomposition Design](../../design/accepted/runenrender-decomposition-design.md).

The review does not authorize Rust implementation, external package creation, new
dependencies, shader changes, or activation of an R phase.

## Question

The governing question is not whether one first analytic/SDF renderer can be built.

It is:

> Can later rendering techniques normally enter by adding a versioned protocol,
> representation, render method, output semantic, input schema, relationship, or
> derived-state kind without redesigning scene lifecycle, planning, execution
> ownership, or the RunenGPU boundary?

## Current strengths

The existing architecture already establishes durable decisions:

- RunenRender owns semantic image formation;
- RunenRender depends on RunenGPU and does not own WGPU;
- renderer state is prepared outside live ECS and product state;
- scene changes support deterministic insert, replace, remove, and producer retirement;
- representations are not required to be meshes;
- provider capabilities were intended to remain narrow;
- caches and histories are derived, non-authoritative, generation-bound, and discardable;
- Runenwerk owns host policy, product recovery, persisted captures, artifact encoding,
  source discovery, and authoring-toolchain policy;
- RunenGPU owns generic GPU resources, programs, work, realization, execution, surfaces,
  progress, pressure, and backend outcomes.

Those boundaries should remain.

## Current defects

The previous design overcommitted implementation-shaped vocabulary:

- `PreparedRenderScene` contained views and logical targets even though those are
  invocation-specific;
- `Provider` was too vague as the primary public concept;
- `RenderInteraction` attempted to unify unrelated surface, visibility, and volume
  results;
- one semantic transport family over-centered path-oriented rendering;
- `Preview / Standard / High / Ultra / Reference` embedded product presets;
- snapshot language did not forbid deep-copy implementations;
- changed regions were insufficient for non-spatial dependencies;
- capabilities had no versioning or exact protocol semantics;
- representation selection lacked coverage, accuracy, freshness, residency, and
  refinement facts;
- output language leaned toward display RGB and one value per pixel;
- coordinate precision, time, exposure, and sampling footprints were not foundational;
- progressive sessions, differentiability, deep output, distributed merging, and
  GPU-produced dynamic input were under-specified;
- scalability requirements were described directionally rather than as measurable
  invariants.

## External architecture findings

### Hydra 2.0

Hydra 2.0 separates a scene abstraction/transformation pipeline from renderer
execution. Scene indices provide queryable scene views and send explicit added,
removed, dirtied, and renamed notifications. Dirty locators identify which property
subtrees became invalid.

Useful lesson:

```text
immutable/queryable scene view
+ precise change notices
+ renderer-local processing
```

RunenRender should adopt that ownership property without recreating Hydra's plugin
pipeline or path/string authority.

### ANARI 1.1

ANARI demonstrates that one high-level renderer interface can cover:

- surfaces and volumes;
- rasterization and high-fidelity path tracing;
- asynchronous frames;
- accumulation and progress;
- multiple frame channels;
- camera shutter and rolling shutter;
- camera and instance motion;
- stereo and omnidirectional views;
- sparse scientific fields;
- distributed rendering across MPI ranks.

Useful lesson:

```text
semantic scene and frame concepts can remain stable
while renderer algorithms and execution environments vary
```

RunenRender should not copy ANARI's string-parameter model. Typed Rust authority remains
required.

### PBRT 4

PBRT keeps semantic rendering concepts while its GPU path changes execution structure
substantially. The GPU renderer uses wavefront queues, but some operations are fused
when queue traffic would be too expensive. PBRT also uses ray differentials and
sampling footprints to filter high-frequency detail.

Useful lessons:

- semantic planning must not expose a fixed GPU topology;
- render-method implementations need freedom to batch, queue, specialize, and fuse;
- procedurally unbounded detail requires footprint-aware filtering, not only distance
  based level of detail;
- one universal complete surface interaction tends to leak parametric-surface
  assumptions.

### Mitsuba 3

Mitsuba variants can independently change:

- scalar, LLVM, or CUDA execution;
- monochromatic, RGB, or spectral measurement;
- polarized or unpolarized transport;
- single or double precision;
- differentiable or ordinary rendering.

Mitsuba also differentiates complete rendering algorithms for inverse problems.

Useful lesson:

```text
measurement domain
precision
differentiability
execution backend
```

are cross-cutting renderer dimensions. They cannot be hidden inside an RGB material or
display-output type.

### MaterialX and OpenPBR

MaterialX standardizes material graph interchange, shading nodes, physical and
non-photorealistic nodes, and multiple target generators. OpenPBR defines a reusable
surface model.

Useful lesson:

- authoring/interchange graphs, renderer material semantics, compiled material programs,
  and GPU realization are separate authorities;
- RunenRender should accept adapters from material standards rather than adopt one
  authoring graph as its core semantic model.

### OpenEXR

OpenEXR supports arbitrary channels, tiled and multiresolution images, multiview,
multipart files, and deep images with a variable number of depth samples per pixel.

Useful lesson:

```text
render output != one fixed RGBA texture
```

The semantic output model must permit flat, tiled, sparse, multisample, and deep
products even though Runenwerk owns file encoding.

### OpenXR 1.1

OpenXR defines mono, stereo, and stereo-with-foveated-inset view configurations. The
foveated form uses four views with different effective sampling density.

Useful lesson:

- a render request may contain a coordinated view set;
- different output regions may have different sampling footprints;
- tracking, gaze, swapchains, and late-latching remain host/runtime facts;
- RunenRender consumes immutable view and sampling facts.

### Vulkan roadmap and extensions

Current Vulkan capabilities include increasingly capable indirect execution, fragment
shading rate, compute derivatives, cooperative matrices, robustness, ray tracing,
mesh/task processing, sparse resources, and device-generated work.

Useful lesson:

- RunenRender should request semantic capabilities;
- RunenGPU should admit normalized capabilities;
- private lowering selects compute, raster, ray, mesh, sparse, or hybrid realization;
- backend feature names must not become the semantic renderer API.

## Long-term feature envelope

The architecture should preserve a credible path for:

```text
analytic geometry
triangle, curve, point, splat, and procedural geometry
signed-distance and general implicit fields
dense, sparse, adaptive, and unstructured volumes
software and hardware surface queries
raster, compute, ray, volume, path, regional, cellular, and hybrid methods
radiance caches, probes, reservoirs, path guiding, and many-light reuse
virtual geometry, sparse fields, paging, and partial residency
finite, planetary, and logically unbounded worlds
millions or billions of logical objects
procedurally unbounded view-dependent detail
particles, populations, fibers, hair, liquids, and deformation
monochromatic, RGB, spectral, and polarized measurements
technical, sensor, scientific, physical, and stylized output
progressive, tiled, multiview, deep, sparse, and distributed output
differentiable and inverse rendering
neural fields, learned materials, radiance caches, and Gaussian splats
XR, stereo, foveated, rolling-shutter, and nontraditional cameras
multi-device and distributed execution
```

These are not implementation commitments. They are compatibility pressure.

## Recommended semantic spine

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
                        -> RenderResult / RenderSession
```

## Scene revisions and change evidence

### Snapshot meaning

`RenderSceneSnapshot` means a cheap immutable view of one committed semantic revision.
It does not imply:

- a deep copy;
- eager realization of every representation;
- duplicated source assets;
- one allocation per object;
- one GPU resource per object.

Permitted implementations include persistent tables, generation-indexed arenas,
copy-on-write pages, structural sharing, paged databases, lazy queries, and compiled
dense renderer tables.

### Change-set meaning

`RenderChangeSet` should describe:

```text
added and removed objects
changed properties
changed relationships
changed spatial regions
changed temporal intervals
representation-offer changes
material, medium, emitter, or environment changes
availability and residency changes
explicit full resynchronization
```

Spatial regions alone cannot express an environment, material graph, visibility-link,
or output-policy change.

## Identity model

Separate:

```text
RenderObjectId
RenderRepresentationId
RepresentationElementId
RenderProducerId
RenderContributionId
RenderInputSlotId
RenderOutputId
DerivedRenderStateId
source-domain identity
asset identity
ECS identity
RunenGPU resource identity
```

A semantic render object retains identity while switching representations.

`RenderObject` is preferred over `RenderEntity` because RunenRender must not imply ECS
ownership.

## Space, units, precision, and time

The architecture needs explicit contracts for:

```text
RenderSpace
CoordinateFrame
UnitScale
SpatialTransform
RenderOrigin
PrecisionContract
SpatialCoverage
RenderTime
TimeInterval
ExposureModel
TemporalSamplingPolicy
TemporalCoverage
MotionContract
```

Large worlds should use region-relative or view-relative GPU coordinates while source
domains retain authoritative coordinates.

Distance-field transforms must state whether exact, conservative, approximate, or
invalid distance semantics survive the transform.

## Sampling footprints and detail

Introduce a semantic `SampleFootprint` that may represent:

```text
pixel footprint
ray cone
ray differentials
surface-space derivatives
volume filter footprint
temporal footprint
spectral footprint
```

Representations and materials may:

- evaluate at a point;
- evaluate over a footprint;
- provide a conservative frequency/error bound;
- return a filtered approximation;
- request finer representation data.

Procedurally unbounded detail is valid only when refinement is footprint- and
error-driven. It is never globally materialized.

## Representation offers

A render object may publish multiple representations. Each representation publishes an
offer containing:

```text
supported versioned query protocols
spatial and temporal coverage
coordinate and unit conventions
accuracy and confidence
freshness/source revision
residency and availability
refinement and fallback
measurement compatibility
provenance
```

Selection uses the render request, method, view, time, footprint, required accuracy,
freshness, residency, memory pressure, and admitted device capabilities.

## Versioned query protocols

Prefer small versioned semantic protocols:

```text
SurfaceQueryProtocolV1
VisibilityQueryProtocolV1
VolumeTraversalProtocolV1
RasterGeometryProtocolV1
AttributeProtocolV1
MotionProtocolV1
RefinementProtocolV1
PopulationProtocolV1
```

Each protocol defines:

- input and output semantics;
- coordinate and time conventions;
- accuracy/error meaning;
- batching and availability requirements;
- version compatibility;
- structured unsupported behavior.

Protocols do not prescribe one Rust virtual call, one shader-language interface, or
one GPU dispatch.

## Narrow results and lazy attributes

Keep core outcomes minimal:

```text
SurfaceHit
VisibilityResult
VolumeInterval
TransmittanceResult
```

A `SurfaceHit` should identify distance, object, representation, representation
element, local position, side, and accuracy evidence.

Normal, tangent frame, material binding, medium boundary, motion, derivatives, local
coordinates, and custom attributes are requested through explicit protocols.

## Render methods

A `RenderMethod` is a coherent image-formation algorithm family.

It may internally use:

```text
raster visibility
software or hardware ray queries
compute kernels
wavefront queues
fused kernels
regional propagation
volume integration
temporal reconstruction
hybrid combinations
```

Applications do not assemble arbitrary visibility, transport, cache, and reconstruction
strategy objects. Method implementations own coherent combinations.

The exact public extension trait should remain unstabilized until at least two
meaningfully different methods prove the common contract.

## Semantic planning and device admission

Separate:

### `RenderPlan`

Device-independent semantic intent and admissible alternatives:

```text
requested method
required protocols
candidate representations
output meanings
accuracy and determinism requirements
derived-state dependencies
permitted fallbacks
semantic degradations
```

### `AdmittedRenderPlan`

One concrete choice for an execution environment:

```text
selected representations
selected method variant
measurement and precision mode
selected residency state
selected GPU programs
selected derived state
resource and variant estimates
RunenGPU requirements
declared degradations
exact schema and method revisions
```

This supports inspection, replay, multiple devices, remote preflight, and deterministic
comparison.

## Dynamic render inputs

`RenderInputSet` should bind typed slots, not arbitrary buffers.

Each slot records:

```text
semantic meaning
schema and element layout
space and units
temporal coverage
generation
lifetime/access
availability
fallback
```

Bindings may come from CPU-prepared values, retained RunenGPU resources, typed exports
from preceding RunenGPU work, streamed pages, or renderer-derived state.

A GPU-produced simulation result remains owned by its simulation domain. RunenRender
consumes one typed current input without CPU readback.

## Output and measurement model

Use `RenderOutputSet`.

Each output specifies:

```text
semantic meaning
extent or domain
measurement domain
numeric representation
precision
accumulation and merge rule
layout
destination intent
```

Potential measurement domains include:

```text
monochromatic intensity
RGB radiance approximation
spectral radiance
polarized spectral radiance
depth and distance
transmittance
normal and orientation
velocity and motion
identity and segmentation
variance and confidence
derivatives
versioned custom measurements
```

Potential output layouts include:

```text
FlatImage
MultiSampleImage
DeepImage
TileSet
SparseImage
BufferProduct
ReadbackProduct
PresentationProduct
```

Runenwerk owns EXR, PNG, video, dataset, and other artifact encoders.

## Accumulation and merging

Outputs combine differently:

```text
radiance        weighted sum or average
depth           nearest or minimum valid sample
normal          normalized weighted accumulation
identity        selected-sample identity
variance        statistical combination
segmentation    categorical reduction
deep output     ordered depth-sample merge
```

These rules are needed for progressive, tiled, multiview, multi-device, and distributed
rendering.

## Materials, media, and appearance

Separate:

```text
authoring/import graph
renderer semantic material
method-compiled material program
RunenGPU program/resource realization
```

Renderer material semantics must preserve room for BSDF, EDF, VDF, medium boundaries,
layering, subsurface, displacement, measured data, spectral parameters, and stylized
models.

Use broadly applicable `RenderAppearance` for material/medium appearance and
output/display intent. Transport, reconstruction, hatching, cellular propagation, and
other specialized style controls should be typed method extensions.

## Progressive sessions

Distinguish:

```text
RenderInvocation
    one bounded request

RenderSession
    optional compatible continuity across invocations
```

A session may record accumulation identity, method revision, outputs, scene/view/input
compatibility, sample and random-stream ranges, progress, convergence, cancellation,
and terminal state.

Every underlying RunenGPU submission still receives exactly one terminal outcome.

## Derived-state dependency graph

Derived state covers:

```text
compiled representations
acceleration
residency
transport estimates
history
reconstruction
output accumulation
```

Each state node records owner method, exact source dependencies, spatial/temporal
coverage, measurement domain, accuracy/confidence, residency, memory class, retention
priority, and reconstruction recipe.

Invalidation follows dependencies rather than comparing every cache against every scene
generation.

## Large-world and object-count scaling

Required architecture invariants:

```text
no mandatory deep copy per commit
no mandatory full rebuild for a local change
no all-object × all-method planning scan
no work node per logical object
no CPU submission per object
bounded or pressure-reporting memory
bounded and aggregated diagnostics
shared compatible multiview preparation
footprint- and error-driven refinement
recorded automatic-selection evidence
```

The logical world may be finite or unbounded. Every render operates on a finite bounded
working set.

GPU work-node count should scale with algorithm stages. Methods may use culling,
compaction, clustering, indirect work, persistent queues, page requests, and population
kernels internally.

## Multi-device and distributed compatibility

The semantic lowering result is a `RenderWorkSet`, not an assumption that exactly one
context-local fragment always exists.

A work set may contain device-local partitions, merge work, readback work, and
presentation work.

RunenRender owns semantic partitionability and output merge rules. Runenwerk or another
orchestrator owns networking, process management, retries, and distributed job policy.

## Differentiable and inverse rendering

Preserve room for:

```text
RenderParameterId
DifferentiableParameterSet
DerivativeRequest
GradientOutput
ForwardMode
ReverseMode
replayable stochastic samples
loss or adjoint inputs
```

Not every value becomes differentiable. Parameter identity and schemas must remain
stable enough for future derivative requests.

## Neural and learned representations

Neural fields, Gaussian splats, learned materials, neural radiance caches, and learned
reconstruction should enter through protocols, representations, methods, outputs,
inputs, and derived state.

Do not permanently classify every representation as mesh, surface, volume, or SDF.

Learned representations also require model schema versions, quantization and precision
facts, accuracy/confidence evidence, large parameter resources, and explicit training
versus inference ownership.

## XR and nontraditional sensors

View sets should support perspective, orthographic, panoramic, fisheye, stereo,
cubemap, light-field, depth-of-field, rolling-shutter, overscan, asymmetric, and
foveated views.

Runenwerk and platform adapters own tracking, gaze, late-latching, runtime swapchains,
reprojection policy, and display timing.

RunenRender consumes immutable view, exposure, and sampling facts.

## Color and interchange

Separate scene measurement, renderer working representation, creative appearance,
display transform, and output encoding.

RunenRender owns conversion intent and measurement meaning. Runenwerk owns OCIO/ACES
configuration, display selection, persisted color policy, and artifact encoding.

MaterialX, OpenPBR, glTF, USD, and other authoring/interchange systems remain adapters.

## Shader and variant scaling

Renderer semantic protocols must not become shader-language interfaces.

Authoring frontends may compile to canonical WGSL through the accepted Runenwerk and
RunenGPU boundary.

Variant selection must be bounded, inspectable, cacheable, and driven by semantic need.
Relevant strategies may include static method families, specialization values, parameter
buffers, shader linking, generated kernels, or interpreted paths for rare cases.

Variant count, compilation cost, cache pressure, and fallback behavior require
characterization.

## Trust and robustness

Potential extension trust classes:

```text
BuiltInTrusted
ApplicationTrusted
ValidatedThirdParty
UntrustedAuthoringInput
```

Controls include bounded source/module/compiler/variant/resource budgets, explicit
interfaces, no unrestricted host callbacks, no arbitrary backend access, no unrelated
global-resource access, robust memory behavior, and structured compile/device-fault
diagnostics.

Runenwerk owns product trust policy. RunenGPU reports admitted robustness and backend
fault facts.

## Determinism levels

Use explicit levels rather than one boolean:

```text
Structural
NumericalWithinTolerance
Statistical
BitwiseUnderConstrainedEnvironment
```

Reproducibility records random generator and revision, seed, sample ranges,
accumulation order, floating-point mode, method and protocol revisions,
representation/input revisions, and backend/device facts where permitted.

## Founding proof

The first proof remains intentionally narrow:

```text
analytic sphere and plane
field/SDF surface representation
one diffuse material
one directional emitter
one view
one linear HDR output
compute current visibility and direct lighting
fullscreen render conversion
offscreen output
CPU reference probes
```

It validates the permanent boundaries:

```text
scene commit and snapshot
change set
object and representation identity
representation offer
query protocol
narrow result
render request and method
semantic and admitted plans
output specification
RunenGPU work lowering
```

No public type is named after SDF, direct lighting, preview, or the first renderer.

## Rejected alternatives

### Move current `RenderFlow` wholesale

Rejected because it combines ECS projection, generic GPU work, image formation,
surfaces, product policy, source policy, and current implementation vocabulary.

### Public render graph as primary API

Rejected because applications should express renderer semantics rather than GPU pass
topology. Internal methods retain freedom to fuse and reorganize work.

### Universal provider trait

Rejected because unrelated representations would be forced to fabricate capabilities
and fields.

### Closed permanent representation enum

Rejected because future representations cannot be predicted exhaustively.

### Universal interaction record

Rejected because surfaces, visibility, media, splats, and learned representations do
not share one naturally complete result.

### One path-transport architecture

Rejected because raster, sensor, diagnostic, regional, volume, neural, and hybrid
methods do not all fit path-continuation vocabulary.

### Product quality ladder as architecture

Rejected because presets are product recipes. Architecture uses explicit policy,
limits, accuracy, determinism, history, and method-selection facts.

### Direct MaterialX, USD, OCIO, or OpenXR ownership

Rejected because those systems belong to authoring/import, color configuration, or
platform adapters rather than the renderer semantic kernel.

## Architectural acceptance test

The architecture is sufficiently future-resilient when a new technique normally adds
one or more of:

```text
versioned query protocol
representation implementation
render method
measurement domain
output semantic
input schema
scene relationship
method-specific appearance extension
derived-state kind
authoring/import adapter
```

and does not require redesigning:

```text
scene commit and snapshot lifecycle
identity separation
change evidence
request structure
semantic/device admission split
RunenGPU execution boundary
output binding ownership
derived-state invalidation doctrine
```

## Official references reviewed

- Hydra 2.0 Getting Started Guide:
  `https://openusd.org/dev/api/_page__hydra__getting__started__guide.html`
- ANARI 1.1 specification:
  `https://registry.khronos.org/ANARI/specs/1.1/ANARI-1.1.html`
- PBRT 4 wavefront rendering:
  `https://www.pbr-book.org/4ed/Wavefront_Rendering_on_GPUs`
- PBRT 4 texture sampling and antialiasing:
  `https://www.pbr-book.org/4ed/Textures_and_Materials/Texture_Sampling_and_Antialiasing`
- Mitsuba 3 variants:
  `https://mitsuba.readthedocs.io/en/latest/src/key_topics/variants.html`
- Mitsuba 3 gradient-based optimization:
  `https://mitsuba.readthedocs.io/en/latest/src/inverse_rendering/gradient_based_opt.html`
- MaterialX specification:
  `https://materialx.org/Specification.html`
- OpenEXR technical introduction:
  `https://openexr.com/en/latest/TechnicalIntroduction.html`
- OpenXR 1.1 specification:
  `https://registry.khronos.org/OpenXR/specs/1.1/html/xrspec.html`
- Vulkan roadmap milestones:
  `https://docs.vulkan.org/spec/latest/appendices/roadmap.html`
