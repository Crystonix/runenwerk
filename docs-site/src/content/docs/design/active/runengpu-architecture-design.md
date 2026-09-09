---
title: RunenGPU Architecture Design
description: Decision-complete ownership, workload, resource, capability, operational, WGPU, surface, diagnostics, conformance, and extraction architecture for RunenGPU.
status: active
owner: gpu
layer: framework/gpu
canonical: true
last_reviewed: 2026-08-30
related_docs:
  - ../../architecture/repository-family-architecture.md
  - ../../adr/accepted/0014-repository-family-extraction-boundaries.md
  - ../../adr/accepted/0015-separate-gpu-execution-from-rendering.md
  - ./runengpu-g3-access-work-graph-design.md
  - ./runengpu-g3r-definite-initialization-correction.md
  - ./runengpu-g4-context-program-realization-design.md
  - ./runengpu-post-g5c-hardening-design.md
  - ../accepted/runenrender-decomposition-design.md
  - ./runenrender-internal-decomposition-execution-plan.md
  - ./runen-family-operational-hardening-design.md
  - ../../reports/investigations/runengpu-g4-context-program-realization-investigation.md
  - ../../reports/investigations/runengpu-industry-comparison.md
  - ../../reports/investigations/runengpu-public-api-ergonomics-review.md
  - ../../reports/investigations/runengpu-proof-workload-strategy.md
  - ../../reports/investigations/runengpu-runenrender-application-domain-fit.md
  - ../../reports/closeouts/pt-runengpu-g1a-closeout.md
  - ../../reports/closeouts/pt-runengpu-g2-implementation-closeout.md
  - ../../reports/closeouts/pt-runengpu-g3-implementation-closeout.md
  - ../../reports/closeouts/pt-runen-family-operational-hardening-closeout.md
  - ../../workspace/specs/pt-runengpu-g4a-context-admission.ron
  - ../../workspace/specs/pt-runengpu-g4b-program-interface-layout.ron
  - ../../workspace/specs/pt-runengpu-g4c-wgpu-realization-cutover.ron
  - ../../workspace/planning/roadmap.md
---

# RunenGPU Architecture Design

## Status

This document owns RunenGPU's durable broad architecture: repository/package target, ownership and
dependency direction, semantic work/execution boundaries, backend containment, public experience,
operational doctrine, and extraction shape.

Live accepted-delivery state and the current implementation frontier belong to RunenGPU tracker #167,
the roadmap, active bounded issues, and their pull requests. They are deliberately not copied into
this architecture document.

Focused phase designs own exact semantic contracts for their bounded areas. Historical phase-local
migration and deletion wording below records ownership boundaries and cutover intent; it is not a
claim that a named migration seam still exists in current source.

The durable sequence remains:

```text
S0 inventory
    -> G1A logical work-resource identity
        -> G2 capabilities/resources/data
            -> G3 correctness preparation and work graph
                -> G4 context/program/resource admission and reusable realization
                    -> G5 execution/progress/readback/retirement
                        -> G6 representative breadth/scale/ergonomics/cost proof
                            -> G7 surface/generation/loss/reconstruction semantics
                                -> G8 operational/backend-neutrality conformance
                                    -> GX standalone extraction
```

The implementation remains inside Runenwerk until the clean GX cutover is accepted. No phase-local
status table in this document supersedes #167 or the roadmap.

## Mission

RunenGPU owns validated execution of GPU resources and workloads.

It answers:

> How are GPU capabilities, resources, accesses, workloads, programs, backend
> realizations, submissions, results, progress, pressure, lifecycle outcomes, and
> backend failures represented and executed safely?

It does not answer:

- what an image should contain;
- how light transport works;
- how a field, simulation, material, UI, ECS entity, or world object behaves;
- how an application schedules gameplay;
- how windows and event loops are managed;
- how product recovery is selected or presented;
- how persisted capture bundles or PNG, EXR, MP4, or WebM artifacts are encoded.

## Architectural position

```text
Runenwerk adapters and host policy
    -> RunenRender semantic image planning
        -> RunenGPU generic GPU execution
            -> private WGPU backend
```

Independent non-render consumers may use RunenGPU directly:

```text
field or simulation adapter --+
procedural/image tool ---------+--> RunenGPU --> WGPU
baker or offline job ----------+
```

RunenGPU complements WGPU. It does not reimplement Vulkan, Metal, D3D12, WebGPU,
shader compilers, or operating-system window systems.

WGPU is the first backend, not RunenGPU's permanent public capability ceiling. A future
native or backend-specific interoperability path requires separate evidence and a
bounded capability, ownership, lifetime, and synchronization contract; it can never be
a broad `raw_device()`-style escape hatch. Vendor reconstruction/upscaling or
frame-generation systems such as FSR, MetalFX, DLSS, and XeSS belong to RunenRender
reconstruction/presentation implementations and product policy, not RunenGPU core
semantics.

## Repository and package

```text
repository: dornglut/runen-gpu
package: runen-gpu
crate: runen_gpu
initial backend: WGPU, private implementation detail
```

Directional standalone shape:

```text
runen-gpu/
├── Cargo.toml
├── Cargo.lock
├── LICENSE
├── README.md
├── src/
│   ├── lib.rs
│   ├── capability.rs
│   ├── context.rs
│   ├── data.rs
│   ├── error.rs
│   ├── graph.rs
│   ├── pipeline.rs
│   ├── resource.rs
│   ├── program.rs
│   ├── surface.rs
│   ├── workload.rs
│   └── backend/
│       └── wgpu.rs
├── tests/
├── examples/
├── benches/
├── docs/
├── conformance/
└── xtask/
```

Module names are directional, not pre-authorized file names. The public package remains
one release unit until a second backend, independent consumer, release boundary, or
dependency graph proves another package is necessary.

Do not initially create:

```text
runengpu_core
runengpu_wgpu
runengpu_macros
runengpu_testing
runengpu_capture
facade or compatibility packages
```

The external repository is populated only in GX after internal conformance and
extraction-readiness evidence are accepted.

## Dependency rules

The future public package must not depend on or expose:

```text
Runenwerk
RunenRender
RunenSDF or source-domain types
RunenECS or another ECS
RunenUI
Winit
scene, view, material, lighting, transport, overlay, editor, or application types
shader filesystem paths or hot-reload policy
fixed-time scheduling
product recovery policy
persisted capture or artifact policy
PNG, EXR, FFmpeg, or another codec
raw WGPU types as the public authority
```

WGPU may be an internal dependency. Narrow backend-specific facts require a concrete
consumer and separately accepted containment. They do not become universal contracts.

Consumers depend downward:

```text
RunenRender --------+
field GPU adapter ---+
simulation adapter --+--> RunenGPU
procedural tools ----+
offline bakers ------+
```

RunenSDF, RunenECS, and RunenUI remain independent. Cross-framework translation remains
Runenwerk-owned until an independently reusable adapter is proven.

## Ownership

### RunenGPU owns

- context, device-generation, execution-epoch, submission, surface, and readback
  identities at their owning phases;
- normalized capabilities, limits, format and alignment facts, portability classes,
  and requirements;
- backend-neutral logical buffer, texture, texture-view, sampler, and query-set
  descriptions;
- kind-typed logical handles and prepared GPU-data contracts;
- access, graph-entry initialization, hazard, logical lifetime, and later retirement
  validation;
- immutable compute, render, copy, buffer-zero, query-resolve, and logical-present
  work;
- render-attachment and multisample-resolve relationships inside generic render work;
- operation/access-derived capability requirements;
- deterministic work composition and validation;
- context and adapter/device admission;
- program source, entry-point, interface, binding, layout, specialization, and generic
  compute/render pipeline contracts;
- private WGPU resource, program, layout, bind-group, pipeline, command, submission,
  completion, readback, and low-level surface realization at the owning phases;
- headless compute and offscreen graphics execution;
- normalized progress, pressure, cancellation, completion, shutdown, device-loss, and
  reconstruction facts;
- structured backend, timing, provenance, cache, generation, surface, and terminal
  diagnostics.

### RunenGPU does not own

- renderer views, targets, providers, materials, media, emitters, visibility, transport,
  reconstruction, image history, or overlays;
- simulation, field, procedural-world, or application algorithms;
- shader source discovery, file watching, reload scheduling, or last-known-good product
  policy;
- authoritative CPU or domain state;
- ECS storage or scheduling;
- UI semantics, layout, focus, accessibility, hit testing, or text shaping;
- window and event-loop policy;
- product quality selection, recovery decisions, or diagnostics presentation;
- persisted capture selection, retention, redaction, artifact naming, image encoding,
  or video encoding.

### RunenRender owns

- prepared scenes and renderer identities;
- views and logical render targets;
- materials, media, emitters, and environments;
- visibility and provider-interaction semantics;
- lighting, transport, and estimator policy;
- reconstruction, radiance caches, and history semantics;
- overlays, color, output, and image-formation semantics;
- lowering renderer plans into generic RunenGPU work and pipeline descriptors.

### Runenwerk owns

- ECS and domain extraction;
- application scheduling and fixed-time policy;
- windows, event loops, DPI, monitor, resize, and visibility policy;
- shader source discovery, revision, watching, reload scheduling, and product fallback;
- composition of contributions from multiple framework and domain consumers;
- tested cross-framework compatibility manifests;
- product recovery decisions;
- persisted capture selection, schema, retention, redaction, and artifact policy;
- versioned namespaced reproducibility bundles;
- offline jobs, ordered frames, manifests, retries, and failure policy;
- PNG/EXR encoding and external codec invocation;
- diagnostics presentation.

Runenwerk may create one shared RunenGPU context for rendering and non-render workloads.
Composition responsibility does not make reusable GPU or renderer semantics
Runenwerk-owned.

## Framework relationships

### RunenUI

```text
RunenUI paint scene
    -> Runenwerk bridge
    -> RunenRender overlay contribution
    -> RunenGPU work
```

RunenRender does not receive widget state or perform hit testing or text shaping.
RunenUI remains usable with independent backends.

### RunenSDF

RunenSDF remains a CPU/backend-neutral field framework owning field values, numerical
contracts, bounds, operators, transforms, capabilities, and reference queries. GPU or
renderer realization is derived integration state. RunenSDF never depends back on
RunenGPU or RunenRender merely because an application accelerates or displays output.

### RunenECS

RunenECS owns generic ECS semantics. Runenwerk adapters extract required state into
prepared domain and GPU values. RunenGPU neither stores ECS entities/components nor
schedules ECS systems.

## Execution architecture

RunenGPU has one logical GPU work and execution authority. Ordinary and advanced authoring produce
the same typed resources and operations and pass through the same durable execution spine:

```text
typed GPU work
    -> correctness preparation
    -> validated work representation
    -> execution planning
    -> private command / execution realization
    -> submission / progress / terminal outcome
```

These are architectural responsibilities, not required one-to-one public Rust types, runtime objects,
caches, threads, command buffers, or packages.

Context/program/resource admission and reusable backend-object realization are supporting
responsibilities rather than physical scheduling authority. In particular, accepted G4 resources,
programs, layouts, bind groups, and pipelines may be realized and cached before a particular
execution plan is chosen. Future transient realization may instead be influenced by execution
planning when evidence justifies it. Neither arrangement creates another semantic work model.

### Correctness preparation

The validated work graph is the shared internal correctness and inspection authority. It determines
whether composed GPU work is legal before backend execution and preserves the facts needed to explain
that decision, including:

- typed resources and operations;
- normalized resource accesses;
- exact initialization requirements and definite initialization effects under their focused owners;
- inferred data dependencies;
- explicit non-data ordering constraints;
- capability requirements;
- provenance and diagnostic causes.

Focused RunenGPU designs own the exact semantics of those facts. Access/hazard truth,
initialization requirements, and definite initialization effects remain distinct authorities.

The validated work representation constrains legal execution. It does not prescribe one permanent
physical schedule, backend synchronization strategy, command-buffer layout, queue assignment, or
allocation strategy.

### Execution planning

Execution planning consumes validated work plus admitted environment facts and selects one legal
physical execution realization.

A planner must preserve every applicable correctness constraint established by preparation. It may
not reinterpret logical resource identity, invent or remove semantic dependencies, weaken
initialization requirements, bypass capability admission, or create a second work-graph authority.

The baseline planner may directly follow deterministic prepared order. More sophisticated planning is
permitted only where evidence justifies it. Such planning may, when semantically equivalent, choose a
different legal ordering or use backend-appropriate command grouping, synchronization granularity,
transition batching, transient realization, queue assignment, or reuse of previously derived
structural planning information.

Those choices are realization strategies, not new GPU semantics.

Semantic access precision and backend synchronization granularity are deliberately distinct.
RunenGPU may preserve exact byte ranges, texture subresources, or other regions for correctness and
diagnostics while using a coarser physical synchronization strategy when that strategy preserves the
same legal behavior.

### Private realization

Reusable backend-object realization and per-execution command realization both remain private.
Reusable resources/programs/layouts/bind groups/pipelines may be cached according to their accepted
compatibility contracts. Per-execution realization maps a legal plan to private backend commands,
synchronization operations, submissions, and completion mechanisms.

WGPU is the initial backend. Its object model, synchronization vocabulary, memory behavior, or
feature enumeration does not become public RunenGPU semantic authority merely because the initial
implementation requires it.

There is no broad raw-device, raw-queue, or raw-command-encoder escape around the validated execution
authority.

### Stable execution invariants

1. **One semantic work model.** Ordinary helpers, advanced authoring, inspected work, and future
   optimized execution lower through the same typed resource, operation, access, and correctness
   authorities.
2. **Correctness is not scheduling.** Dependency and validity constraints define which executions
   are legal; physical ordering and synchronization are realization choices within those constraints.
3. **Semantic precision is not backend granularity.** Exact semantic regions may be retained for
   correctness and diagnostics without requiring equally fine backend synchronization.
4. **Planning does not create semantics.** Execution planning may optimize a valid workload but may
   not invent application meaning, resource authority, causality, initialization truth, or capability
   guarantees.
5. **Higher-level semantics remain above RunenGPU.** Rendering methods, scenes, materials,
   simulation policy, ECS scheduling, UI, fixed-time behavior, shader-source workflow, and product
   recovery remain owned by their respective consumers/frameworks.
6. **Backend machinery remains below RunenGPU.** Reusable backend realization and per-execution
   command realization stay private unless a separately accepted bounded extension proves a portable
   RunenGPU contract is required.
7. **Optimization mechanisms are evidence-gated.** Reusable planning, transient aliasing,
   multi-queue scheduling, pass fusion, aggressive graph optimization, and similar mechanisms are not
   architectural requirements merely because the architecture leaves room for them.

## Public experience

The validated work graph is the shared internal correctness and inspection authority.
It is not mandatory common-path ceremony and is not the permanent physical schedule.

Directional ordinary path:

```rust
let context = GpuContext::request(context_descriptor).await?;
let simulation = simulation.gpu_work(&context, &state)?;
let rendering = renderer.gpu_work(&context, &scene, request)?;
let submission = context.submit("frame 42", [simulation, rendering])?;
```

Directional inspectable path:

```rust
let prepared = context.prepare("frame 42", [simulation, rendering])?;
inspect(prepared.diagnostics());
let submission = context.submit_prepared(prepared)?;
```

G4 establishes context admission, programs, interfaces, and reusable realizations. G5 establishes
the ordinary prepare/submit terminal. These examples are directional and do not make internal
planning representations part of ordinary API ceremony.

Ergonomic invariants:

- strings are diagnostic labels, never resource, binding, dependency, or cache
  authority;
- resource references are kind-typed;
- G4 pipeline interfaces expose typed validated binding keys;
- builders use lexical or closure scope rather than nested finish ladders;
- G3 infers data dependencies from declared resource access;
- explicit ordering exists only for real non-data constraints;
- public handles are `Clone`, non-`Copy`, opaque values;
- G5 connects last-handle drop to backend retirement after relevant submissions;
- accepted work receives one structured terminal outcome;
- pressure never becomes silent loss or unbounded implicit growth;
- callers do not branch on panic text, logs, `anyhow`, or backend-only enum dumps.

## Context model

A `GpuContext` represents one admitted backend execution authority. G4A creates the
context and admits a device. G4C owns reusable backend realizations. G5 executes work.
G7 owns reusable surface and device-replacement behavior.

Conceptual state:

```text
GpuContext
├── context identity
├── normalized backend, portability and admitted-device facts
├── current device generation
├── resource realization registries
├── program/interface/layout/pipeline registries and cache facts
├── submission and progress authority                 G5
├── pressure and backing-memory accounting            G5
├── surface registry and generation authority         G7
├── completion and readback state                      G5
└── diagnostics stream
```

Requirements:

- no process-global mutable context or implicit product singleton;
- foreign-context and stale-generation values are rejected;
- no authoritative domain state is stored only on the device;
- accepted work receives exactly one completion, cancellation, loss, or shutdown
  outcome once G5 owns execution;
- progress ownership and allowed thread/executor are explicit;
- consumer callbacks are not invoked while internal locks are held;
- submissions, uploads, mappings, readbacks, and backing memory are bounded or have
  explicit growth policy;
- RunenGPU does not decide how many contexts a product creates or how recovery is
  presented.

## Identity model

Runtime identities are opaque, type-distinct, scope-bound, and fallibly allocated.
Accepted logical resource identity is owner-scoped `GpuWorkResourceId`. G2 owns distinct
logical handles:

```text
GpuBufferHandle
GpuTextureHandle
GpuTextureViewHandle
GpuSamplerHandle
GpuQuerySetHandle
```

G4 adds opaque nonzero process-local `GpuContextId` and
`GpuDeviceGeneration`. A new context begins at generation `1`. Every G4C realization
stores exact context/generation affinity. Logical descriptors remain backend-neutral.

Later phase concepts include execution epoch, submission, surface, and readback
identities. Raw diagnostic values never imply persistence, replay, cache, network,
wire, or external-format stability.

RunenGPU owns this owner-scope sequence:

```text
private RunenGPU process-local monotonic owner-scope authority
    ↓
fresh opaque nonzero owner scope
    ↓
GpuWorkResourceIdAllocator
    ↓
typed G2 logical handles
```

The authority is independent of `GpuContext`, `GpuDeviceGeneration`, backend/WGPU,
`RenderFlowId`, and renderer invocation/history identity. Zero is invalid; a production
scope never wraps or reuses, and exhaustion is a structured allocation outcome. Isolated
test allocators never mutate or reset the production authority. Raw scope components are
diagnostic-only, and production callers cannot choose, reconstruct, or inject them.
Owner-scope acquisition occurs in the already-fallible logical-resource allocation path
(lazily on its first successful allocation or equivalently), so `RenderFlow::new` need
not become fallible merely to acquire a scope. No public scalar owner-scope type is
introduced.

The temporary crate-private bridge that seeds a work-resource owner from `RenderFlowId`
is accepted G3 migration evidence and G4C1 deletes it. Renderer invocation/history keys
may map renderer policy to distinct retained typed G2 handles, but renderer IDs, labels,
paths, hashes, and backend addresses do not enter generic RunenGPU resource identity or
registry keys.

## Capability and portability model

RunenGPU exposes normalized facts rather than raw backend enums.

Requirement strength remains:

```text
Required
Preferred { explicit fallback or degradation }
Disabled
```

An unmentioned capability is irrelevant. `Optional` is not a fourth state. `Disabled`
is an explicit admission constraint. Compatible requirement merging is deterministic
and commutative. Required conflicts with Disabled; incompatible preferred fallbacks
fail rather than choosing silently.

Profiles are recipes that produce ordinary requirements; they are not a second
authority.

Initial normalized feature vocabulary remains limited to current consumer evidence,
including compute, render pipelines, copy, indirect draw, storage textures, depth
attachments, timestamp queries, and presentation intent. Initial limits and format
facts remain use-specific and expand only under accepted consumer pressure.

G4A reports normalized backend family, adapter class, portability class, supported and
enabled features, effective limits, format capabilities, and operation-relevant
alignments. Unknown backend facts remain explicit rather than guessed.

G4A admits merged requirements deterministically:

- mandatory requirements reject an unsupported candidate;
- preferred requirements degrade only through explicit declared degradation;
- unrelated features are not opportunistically enabled;
- the least sufficient effective limits are requested;
- requested, supported, enabled, degraded, and rejected facts remain distinct;
- candidate-selection limits imposed by WGPU/platform behavior are reported honestly.

Deferred until concrete pressure:

```text
subgroups
external-resource interop
ray queries and ray pipelines
sparse resources
mesh shaders
video
multiple hardware queues
universal shader IR
```

## Resource model

Resource properties remain independent:

```text
kind
    buffer, texture, texture view, sampler, query set
lifetime
    transient, retained
ownership
    RunenGPU-owned, imported, surface-acquired
transfer and observation
    initial data, upload/update, copy, query resolution, readback, export
reconstruction
    source-backed, externally reconstructed, non-reconstructable
memory intent
    ordinary device use, upload staging buffer, readback buffer
```

Imported, exported, readback, and surface-owned are not lifetime classes. Resource
descriptors include kind-specific shape and format, permitted uses, initialization,
independent lifetime/ownership/memory/reconstruction facts, validated label, and
provenance/source-generation facts where applicable.

Ownership/provenance alone is not a concrete backend import source. G4C1 may create an
owned resource; an imported buffer or texture needs an explicit accepted concrete
import-source contract or yields `ImportSourceUnavailable`/unresolved import.
`SurfaceAcquired` remains G7-only. No renderer semantic ID, public raw WGPU object,
native handle, or generic unsafe import escape hatch fills that gap.

Labels and provenance are diagnostics and reconstruction evidence, never identity,
lookup, binding, dependency, persistence, wire, or cache authority.

Buffer and texture initialization remain distinct. Query sets have no initialized
query contents unless graph-entry coverage is supplied. Texture views reference a typed
parent texture and checked mip/layer/aspect range and cannot exceed parent lifetime,
lease, generation, or subresource range.

G3 owns exact work-time ranges, access categories, graph-entry initialization, hazards,
attachment relationships, query resolution, and causality. G3R/G5R own the corrected
requirement/effect/materialization semantics. G4A/G4C add backend limit, format, alignment,
context, and generation admission. G5 adds runtime lease and retirement checks.

## Typed GPU data

```text
Runenwerk or source-domain adapter
    ECS or domain state
        -> explicit prepared value or bytes
            -> RunenGPU upload/update contract
```

Prepared data distinguishes uniform, storage, vertex, indirect, and transfer roles.
Readback uses a separate decoder contract. Prepared data records checked byte length,
alignment, stride, element count, and provenance. It does not infer GPU safety from
arbitrary Rust memory.

Current `GpuParams`, `GpuUniform`, `GpuStorage`, `ToGpuValue`, `GpuUniformField`, and
derives remain transitional Runenwerk/render byte-preparation helpers. G4B denies them
universal ABI or program-interface authority, adds no macro package, and validates
binding/interface compatibility independently. G5 owns uploads and readback.

`TypeId` and type names may support process-local diagnostics or adapter lookup. They
are not layout, binding, persistence, wire, cache, or shader-interface authority.

## Workload and prepared-graph model

G3 consumers contribute immutable work fragments containing imports, declarations,
exports, work nodes, explicit non-data constraints, outputs, and provenance.

Initial operations:

```text
Compute
Render
Copy
Clear      checked buffer zero only
Resolve    query set to buffer
Present    logical texture consumption only
```

`GpuPreparedWorkGraph` composes bounded fragments and owns deterministic typed
resolution, operation/access/requirement consistency, inferred hazards and dependencies,
deterministic correctness order, graph-entry initialization, logical lifetime validation,
admission inputs, backend execution inputs, and output/completion contracts.

It rejects duplicate, unknown, foreign, cyclic, ambiguous, contradictory, invalid, and
read-before-initialization states. Fragment collection position is not scheduling
authority. Cross-fragment overlapping writes require typed producer/consumer causality.

The prepared graph is the correctness representation, not the permanent physical execution plan.
Its deterministic topological order supports validation, diagnostics, testing, and the baseline
execution strategy. Downstream execution planning may add conservative order or choose another legal
ordering where the graph leaves nodes independent, but may not weaken or reinterpret prepared
correctness constraints.

The graph contains no ECS systems, gameplay actions, UI routes, SDF nodes, material
graphs, renderer feature meaning, or product lifecycle policy.

## G4 program and pipeline boundary

A consumer owns program meaning and source policy. RunenGPU owns admission, interface
validation, and backend realization.

G4B establishes:

- typed source keys, nonzero source revisions, and source-content consistency;
- WGSL as the first concrete source kind;
- typed compute, vertex, and fragment entry-point descriptors;
- typed `(group, binding)` keys;
- explicit binding declarations and program interfaces;
- bind-group and pipeline-layout descriptors;
- specialization schemas and normalized values;
- generic deterministic compute and render pipeline descriptors;
- structured source, interface, specialization, and descriptor failures.

The following are not binding or pipeline correctness authority:

```text
strings or labels
filesystem paths
TypeId or Rust type names
GpuWorkResourceId
RenderFlowId, RenderPassId, or RenderFeatureId
naked u64 hashes
raw WGPU layout or pipeline objects
```

Runenwerk retains source discovery, paths, watching, reload scheduling, and
last-known-good policy. RunenRender retains semantic shader and image-formation choices.
No stable source format, universal shader IR, or macro package is accepted by G4.

## G4 realization and cache boundary

G4C owns opaque context/device-generation-bound realizations for resources, programs,
layouts, bind groups, and compute/render pipelines. Raw WGPU objects remain private.

Realization is explicit or bounded-lazy by kind. Owned persistent resources and query
sets are explicit; an imported resource is eligible only through an explicit accepted
concrete import-source contract; transient graph resources use a bounded realization pass;
texture views are lazy only within a realized parent; layouts and pipelines may be
cache-backed lazy before execution. G5 encoding cannot silently create undeclared
resources.

Every request rejects foreign context and stale generation before backend work. Full
descriptor compatibility is checked before reuse. Registry publication is transactional.

Deterministic descriptor, capability, affinity, and import-source incompatibility is a
structured G4C1 rejection before backend creation. After complete RunenGPU admission, an
unexpected backend validation rejection is a backend-contract/invariant violation;
resource allocation exhaustion or OOM is instead a structured backend
capacity/resource-exhaustion failure; and unavailable or lost device/context is a
structured context/device outcome. G4C1 neither replaces nor recovers a device/context:
G7 later owns replacement/reconstruction facts and Runenwerk owns product recovery.
Backend error text is bounded diagnostic evidence only. These classes do not publish a
realization record, and `RealizationFailed` may be a telemetry aggregate but not their
semantic classification.

An authoritative registry maps one typed logical identity to its realization record and
is scoped to one context/device generation. The same identity plus the same complete
semantic descriptor reuses that record; a changed descriptor rejects; distinct logical
identities never alias merely because descriptors match. A derived cache is optional,
in-memory, discardable, and non-authoritative. It may select a candidate, but full typed
equality follows hashing and authorizes correctness. Admission checks context/device
facts before lookup rather than copying a huge fact set into every map key.

```text
Hit                 reuse
Miss                ordinary realization
Rejected            ordinary realization after structured incompatibility
RejectedCorrupt     ordinary realization after structured corruption
RealizationFailed   no entry published
```

A cache hit changes cost, never semantics. No stable persisted cache format is
authorized by G4. Registry reclamation is not GPU completion and not physical backend
retirement. G4C1 may remove or deactivate an unretained record from future authoritative
lookup only according to logical liveness; that removal neither proves prior
encoded/submitted use is complete nor invalidates a live realized handle, active bridge
borrow, or current execution reference. G4C1 owns no fence or submission-completion
retirement authority and preserves current backend/execution retention mechanics until
G5 replaces them. G5 alone owns in-flight retention, completion, cancellation, delayed
backend retirement/destruction, and shutdown.

Backend resource-object creation is G4C1 work. `GpuBufferInitialization` and
`GpuTextureInitialization` remain checked logical intent, while uploads, updates,
copies, staging, query resolution, map/poll, and readback remain G5 work.
`create_buffer_init`, `queue.write_buffer`, and `queue.write_texture` cannot establish
a second G4C1 transfer authority.

G4 reusable-object realization is deliberately distinct from per-execution planning. A resource,
program, layout, bind group, or pipeline may already have a valid cached realization when execution
planning begins. Conversely, future evidence may justify plan-directed transient realization. Neither
case makes G4 caches or registries the physical scheduling authority.

## G4C seam distinction

Exactly one object-reference migration bridge may remain at each accepted G4C boundary:
`CurrentRenderResourceBridge` after G4C1, `CurrentRenderPipelineBridge` after G4C2,
and `CurrentRenderExecutionBridge` after G4C3, with each successor deleting and fully
superseding its predecessor. Carried-forward predecessor terminals monotonically shrink;
a successor may add only newly realized terminal classes owned by that phase that
exact-current-main uncut consumers still require.

`CurrentRenderDeviceQueue` is separate: it is a crate-private backend-operation loan,
not an object-reference bridge or a second bridge for uniqueness accounting. G4C1
removes generic buffer/texture/view/sampler/query-set creation through it; G4C2 also
removes shader-module/layout/bind-group creation; G4C3 also removes pipeline creation;
G5 migrates encoding/upload/submission/copy/map/readback users and deletes it. Its
source-guarded operation classes and exact call sites only shrink. The loan is
non-public, non-authoritative, purpose-bound, inaccessible through `Deref`/`AsRef`, and
not a generic callback or future native-interop API.

The ordered G4C children perform a clean cutover and delete, at their owning boundary:

- renderer-owned reusable WGPU resource/program/layout/bind-group/pipeline registries
  and caches;
- string/path/pass/feature/hash GPU correctness keys;
- synthetic G2 handle construction outside RunenGPU;
- the `RenderFlowId`-derived temporary resource-owner bridge in G4C1;
- public raw `Device` and `Queue` reach-through;
- G4-owned program/interface/pipeline/cache/realization truth in the G3 sidecar.

After G4C the sidecar may retain only G5-owned execution payload. G5 deletes that
residual payload during encoding/submission cutover.

## WGPU backend and phase ownership

Execution lifecycle:

```text
collect typed fragments
    -> correctness prepare and compose                    G3
    -> admit context/program/resource compatibility       G4
    -> realize/reuse backend objects as required          G4
    -> execution planning                                 execution responsibility
       (baseline may follow prepared order)
    -> realize commands and submit                        G5
    -> progress, complete, read back                      G5
    -> retire transient/backend state safely              G5
    -> acquire/present/reconstruct surfaces               G7
```

The execution-planning responsibility is architectural rather than a new phase code or required Rust
type. More sophisticated planning remains evidence-gated; the current baseline may simply consume the
prepared deterministic order.

WGPU is private implementation for:

- G4A instance, adapter, device, queue, feature/limit/format mapping;
- G4C resources, views, query sets, modules, layouts, bind groups, pipelines and caches;
- G5 command encoding, submission, staging, query-resolution execution, progress,
  completion, mapping, readback and delayed destruction;
- G7 low-level surfaces and loss/reconstruction mechanisms.

No public `Device`, `Queue`, resource, shader module, layout, bind group, pipeline,
surface, or raw enum becomes stable RunenGPU authority.

The Winit-coupled `WgpuCtx` and renderer-owned WGPU caches were migration evidence. G4A detaches
headless context admission while retaining at most one temporary host-compatible selection seam.
Ordered G4C removes reusable renderer-owned realization; its separate
`CurrentRenderDeviceQueue` operation loan remains only until G5 removes the final operation users.
G7 removes the remaining surface compatibility seam. Current-source presence of any historical seam
is owned by code and tracker evidence, not this phase-boundary description.

## Surface boundary

Runenwerk or another host owns window creation and destruction, event loops, raw-handle
lifetime, DPI, monitor, resize, visibility, presentation policy, and product recovery.

RunenGPU G7 owns admitting host-provided raw handles without Winit, low-level surface
creation and retirement, capability/configuration, image acquisition and lease
lifetime, presentation, generations, and structured surface/device outcomes.

RunenRender owns logical output image and color meaning. An image acquired from an old
generation cannot be presented through a newer configuration.

G4 does not create a reusable public surface API. Its temporary compatibility input
exists only to preserve current adapter selection and has explicit G7 deletion
ownership.

## Operational contracts

### G5 progress, pressure, callbacks, and shutdown

G5 specifies native/web progress ownership, allowed thread/executor, callbacks outside
locks, exactly-once terminal delivery, quotas for pending work and backing memory,
structured pressure, cancellation, pending-work shutdown, and release diagnostics.

Once work is accepted, RunenGPU never silently discards it.

### G6 cost characterization

G6 compares equivalent RunenGPU and narrow direct-WGPU workloads for known compute,
image-processing, and offscreen graphics proofs. It records CPU work, allocations,
command recording, submission cost, staging/readback bytes, cold/warm pipelines,
memory high-water marks, GPU timing where supported, and artifact equivalence.

No private bypass may make the framework path appear faster. Performance facts become
budgets only through separately accepted controlled specifications.

### G7 generations, loss, and reconstruction

G7 distinguishes surface outdated/lost/out-of-memory, device lost/out-of-memory, and
backend failure. Device replacement increments generation and invalidates old
realizations. Retained logical handles do not silently bind to a new generation.
Runenwerk chooses retry, recreation, degradation, pause, exit, or user action.

### G8 reproducibility and residual conformance

G8 proves cache behavior, pressure, pending-work shutdown, loss/reconstruction, bounded
diagnostics, reproducibility-bundle inputs, no raw WGPU reach-through, and retained
standalone/downstream public API evidence.

RunenGPU supplies namespaced facts. Runenwerk owns schema, retention, redaction,
validation, migration, capture policy, and artifact encoding.

## Error and diagnostics model

Public errors remain typed and structured across:

```text
identity and allocation
capability admission and degradation
resource and descriptor validation
prepared data and decoding
work operation and graph validation
program/interface/pipeline realization and cache compatibility
pressure, progress, submission, completion, cancellation and readback
surface and device outcomes
loss, reconstruction and shutdown
```

Every error contains programmatically matchable facts plus human operation, label,
cause, provenance where applicable, and corrective action. Panic text, generic string
matching, `anyhow`, and log text are not the normal public contract.

RunenGPU exposes structured diagnostics for adapter selection, requested and granted
capabilities, realization, cache outcomes, graph preparation, execution, pressure,
completion, readback, timings, surface/device outcomes, and shutdown. Hosts own severity,
storage, presentation, retry, persistence, and redaction.

## RenderFlow disposition

Current `RenderFlow` is a transitional combined facade. It is decomposed, not moved,
renamed, wrapped, or extracted wholesale.

| Current responsibility | Target owner |
|---|---|
| logical GPU resource identity, descriptions, access, generic work | RunenGPU G1A-G3 |
| context, program/interface contracts and WGPU realization | RunenGPU G4 |
| encoding, submission, progress, readback and retirement | RunenGPU G5 |
| low-level surfaces and device-loss reconstruction facts | RunenGPU G7 |
| views, targets, rendering and image-formation semantics | RunenRender |
| ECS projection, fixed-step scheduling, source paths, hot reload, windows, UI, capture and recovery policy | Runenwerk adapters |

Specific rules:

- state projection and dispatch-from-state remain outside RunenGPU;
- shader asset paths remain Runenwerk source authority;
- target aliases, history, graphics quality, and procedural image meaning remain
  RunenRender authority;
- strings cease to be resource, binding, dependency, or cache authority;
- temporary owner and sidecar seams are deleted by their exact G4C/G5/G7 owners;
- G2-G7 migrate and delete authority incrementally;
- G8 is final conformance and residual audit, not delayed bulk migration.

## Proof portfolio

Evidence remains separated:

```text
deterministic conformance
boundary integration
environment-dependent backend proof
operational pressure
recovery and reproducibility
performance characterization
visual showcase
```

G4 deterministic proof covers synthetic admission, identities and affinities,
descriptors, typed binding/interface compatibility, specialization, deterministic
hash/equality, cache-key completeness, source/dependency guards, and migration/deletion
guards.

G4 environment-dependent proof covers headless adapter/device request, WGSL modules,
resource/view/sampler/query realization, one compute and one render pipeline, actual
format/alignment facts, and cache behavior. Unsupported environments are explicit.

G5 retains:

- exact inclusive/exclusive prefix scan over 4,097 fixed integer inputs;
- fixed-seed 160x90 Game of Life for 16 steps, live count `2,063`, and FNV-1a-64
  `0xBD710B88594CD584`;
- deterministic compute-to-texture with explicit row padding and format handling;
- saturation, progress, callbacks, cancellation, pending-work shutdown, and readback.

G6 retains known-pattern offscreen graphics, compute-generated indirect draw, offscreen
boids, shared render/non-render context proof, direct-WGPU comparisons, and cold/warm
cost evidence.

G7 reuses G6 workloads for surfaces, resize, generation changes, multi-surface behavior,
affinity, device/surface outcomes, and reconstruction.

RunenRender begins semantic proof with procedural sky/SDF terrain after accepted
standalone RunenGPU, then incremental prepared scenes, provider pressure, synthetic
volume, and cache/history invalidation.

## Conformance

Internal conformance requires:

1. neutral capability, resource, data, context, program, and pipeline validation without
   Runenwerk or a window;
2. owner-scoped logical identity and context/generation affinity tests;
3. accepted G3 access, operation, initialization, hazard, causality, requirement, and
   prepared-graph tests;
4. G4 admission, portability, binding/interface, realization, cache, generation,
   migration and containment proof;
5. G5 headless compute, progress, pressure, callbacks, completion, cancellation,
   readback, pending-work shutdown, and retirement proof;
6. G6 offscreen graphics, shared consumers, and direct-WGPU comparison;
7. G7 structured surface/device/loss/reconstruction outcomes;
8. G8 cache, diagnostics, reproducibility, shutdown, and reach-through audit;
9. no renderer/domain meaning or Runenwerk type in future-transferable source;
10. deterministic evidence separated from environment-dependent evidence;
11. source, dependency, stable-format, compatibility, and duplicate-authority guards.

External repository conformance additionally requires independent locked format, test,
Clippy, rustdoc, docs and metadata validation, declared edition/MSRV, public downstream
consumer proof, license/provenance, exact-revision Runenwerk integration, and deletion
of original authority with no mirror, submodule, compatibility, or moving-branch path.

## Current-source revalidation gate

S0 is complete historical discovery evidence. Every phase re-verifies its affected
current-main subset. Before source changes, the owning issue:

- verifies exact current main and accepted base;
- runs the canonical baseline;
- repeats direct and transitive consumer census;
- confirms no stable persistence, wire, cache, dependency, package, or later-phase
  ownership change;
- binds exact public, migration, deletion, and proof scope;
- stops for a new ADR, package, dependency, compatibility path, or premature owner.

## Extraction sequence

The durable extraction sequence is:

```text
S0 complete inventory
    -> G1A owner-scoped logical GPU work-resource identity
        -> G2 capabilities, resources, typed handles, prepared data
            -> G3 correctness preparation and work graph
                -> G4 context/program/interface/backend reusable realization
                    -> G5 execution/progress/readback/retirement
                        -> G6 representative proof, ergonomics, scale, and direct-backend cost
                            -> G7 surfaces/generations/loss/reconstruction
                                -> G8 operational conformance/diagnostics/backend-neutrality audit
                                    -> GX external dornglut/runen-gpu clean cutover
```

This is sequencing authority only. Completion state, accepted SHAs, active issues, and the current
frontier live in tracker #167 and the roadmap. Only one implementation authority is active at a time.
G2-G7 migrate and delete replaced authority incrementally. G8 is final conformance, not delayed bulk
cleanup.

## External cutover

GX is a clean cutover:

1. populate `dornglut/runen-gpu` from accepted internal source;
2. preserve source provenance and license;
3. establish independent validation, MSRV, documentation, downstream, operational, and
   performance evidence;
4. pin Runenwerk to an exact accepted revision;
5. migrate every active consumer;
6. delete internal GPU authority and temporary adapters;
7. prove no mirror, forwarding package, compatibility namespace, duplicate context,
   duplicate descriptor, or duplicate execution path remains.

No submodule, source include, moving-branch dependency, or long-lived dual path is
accepted.

## Strategic reevaluation gates

Reconsider the split if:

- no independent non-render consumer exists by G6;
- ordinary consumers require raw WGPU;
- measured overhead lacks reusable correctness value;
- a direct-WGPU or smaller existing path satisfies accepted needs with less ownership;
- backend-neutral contracts repeatedly encode WGPU details;
- progress, pressure, or recovery requires product policy inside RunenGPU.

Reevaluation requires an explicit architecture decision and does not authorize a hidden
bypass.

## Explicit non-goals

The initial extraction does not include:

- a second backend;
- aggressive transient aliasing;
- pass fusion;
- automatic multi-queue scheduling;
- backend-independent shader IR;
- graph visualization UI;
- hardware ray tracing as a baseline;
- sparse or external-resource interop without current evidence;
- image/video codec ownership;
- renderer semantics inside RunenGPU;
- a shared RunenCore, compatibility package, macro package, or public raw WGPU escape.

## Definition of done

RunenGPU extraction is complete only when:

- `dornglut/runen-gpu` contains one independently validated public package;
- Runenwerk and RunenRender use only public APIs;
- public contracts contain no Runenwerk, RunenRender, ECS, SDF, UI, Winit,
  application, product, or domain types;
- headless compute, uploads, progress, pressure, completion, asynchronous readback,
  offscreen graphics, and surfaces pass;
- exact conformance and representative showcase evidence remain separate;
- at least one non-render consumer proves independent value;
- cache compatibility, device generations, loss, reconstruction, and terminal shutdown
  are accepted;
- direct-WGPU comparison demonstrates acceptable measured boundary cost;
- simple and inspectable public paths are proven;
- exact revision, MSRV, license, and provenance are recorded;
- every active consumer is migrated;
- original Runenwerk GPU authority and temporary seams are deleted;
- no duplicate context, resource, descriptor, workload, cache, or private WGPU
  reach-through survives.
