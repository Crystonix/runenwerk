---
title: Separate GPU Execution from Rendering
description: Accepted ownership and dependency decision establishing RunenGPU as the shared GPU execution framework beneath RunenRender.
status: accepted
owner: workspace
layer: architecture
canonical: true
last_reviewed: 2026-07-26
related_designs:
  - ../../architecture/repository-family-architecture.md
  - ../../design/active/runengpu-architecture-design.md
  - ../../design/accepted/runenrender-decomposition-design.md
  - ../../design/active/runenrender-internal-decomposition-execution-plan.md
related_roadmaps:
  - ../../workspace/planning/roadmap.md
---

# ADR 0015: Separate GPU Execution from Rendering

## Decision

Create two independent framework repositories with one public package each:

```text
product       repository                  package       crate
RunenGPU      dornglut/runen-gpu          runen-gpu     runen_gpu
RunenRender   dornglut/runen-render       runen-render  runen_render
```

The required dependency direction is:

```text
Runenwerk adapters and host policy
    -> RunenRender semantic image planning
        -> RunenGPU generic GPU execution
            -> WGPU backend
```

Non-render consumers may use RunenGPU directly.

RunenGPU may use WGPU as its first internal backend. RunenRender must not own a WGPU device, queue, surface, allocator, command encoder, or competing GPU resource/error model.

Each repository begins with one public package. Internal modules provide ownership boundaries until a second backend, independently reusable consumer, release unit, or dependency graph proves another package is necessary.

Do not initially create:

```text
runengpu_core
runengpu_wgpu
runengpu_macros
runengpu_testing
runengpu_capture
runenrender_core
runenrender_gpu
runenrender_macros
runenrender_testing
facade or compatibility packages
```

This ADR amends RunenRender backend ownership in ADR 0014. ADR 0014 remains authoritative for repository independence, Runenwerk integration ownership, clean cutover, provenance, and removal of duplicate source authority.

## Context

The current Runenwerk renderer grew as one operational subsystem combining:

- generic GPU resource and execution mechanics;
- renderer and image-formation semantics;
- ECS and application projection;
- window and surface policy;
- shader-file discovery and hot reload;
- fixed-step scheduling;
- built-in UI composition;
- capture and artifact policy;
- product diagnostics and recovery.

That combined shape is useful implementation evidence but is not a reusable framework boundary.

A renderer-only extraction would leave simulations, procedural generation, image processing, tools, and future independent consumers either dependent on renderer vocabulary or using WGPU through parallel ad hoc paths. A WGPU-only wrapper would not provide the ownership, validation, typed composition, lifecycle, and diagnostics required by Dornglut consumers.

The accepted split therefore places a general validated GPU execution framework below a semantic renderer.

## RunenGPU ownership

RunenGPU owns:

- context, adapter, device, queue, and execution identities;
- normalized capabilities, limits, format facts, and requirements;
- backend-neutral logical resources and typed handles;
- prepared GPU-data contracts and readback decoding boundaries;
- resource access, initialization, lifetime, hazard, and retirement validation;
- immutable generic compute, render, copy, clear, resolve, and present work;
- deterministic work composition and validation;
- shader admission, interface validation, and backend pipeline realization;
- uploads, asynchronous readback, submission, completion, and cancellation;
- headless compute and offscreen graphics;
- low-level surface admission, configuration, acquisition, presentation, and outcomes;
- backend, device, timing, provenance, diagnostics, and shutdown facts.

RunenGPU does not own scenes, views, logical render targets, materials, lighting, transport, reconstruction, overlays, field or simulation algorithms, ECS, UI, windows, shader-file policy, capture policy, image encoding, video encoding, or product recovery.

## RunenRender ownership

RunenRender owns how prepared render-facing data becomes one or more images:

- prepared scenes and renderer identities;
- views and logical targets;
- providers, instances, and interaction contracts;
- materials, media, emitters, and environments;
- visibility and provider intersection policy;
- lighting, transport, and estimator policy;
- radiance caches, bounded history, and reconstruction;
- overlays, color, output, and image-formation semantics;
- render quality/degradation policy;
- lowering semantic render plans into generic RunenGPU work.

RunenRender does not own ECS extraction, source authoring, field/SDF mathematics, simulation algorithms, UI state/layout/hit testing/accessibility, windows/event loops, shader filesystem watching, generic GPU execution, or product recovery.

RunenRender depends on RunenGPU and uses only its public contracts after extraction.

## Runenwerk ownership

Runenwerk retains:

- application and engine lifecycle;
- frame, fixed-time, and domain scheduling;
- windows, event loops, DPI, monitor, resize, visibility, and presentation policy;
- ECS, scene, world, material-authoring, field/SDF, UI, editor, and simulation extraction/adapters;
- shader source discovery, revision, filesystem watching, and hot-reload policy;
- product quality and capability selection;
- cross-framework work composition;
- capture selection and artifact policy;
- offline jobs, ordered frame output, manifests, retries, and failure policy;
- PNG/EXR encoding and external FFmpeg or other codec invocation;
- diagnostics presentation and product recovery.

Runenwerk may create one shared RunenGPU context and compose work from RunenRender and non-render consumers. Runenwerk does not gain ownership of reusable GPU or rendering semantics merely because it performs composition.

Reusable adapters may be extracted later only after both public contracts are stable and at least one consumer outside Runenwerk proves independent value.

## Framework independence

RunenSDF, RunenECS, and RunenUI do not depend on RunenGPU or RunenRender merely because an application may accelerate or display their outputs.

The default shape is:

```text
RunenSDF ----+
RunenECS ----+--> Runenwerk adapters/integration
RunenUI -----+
                  |
                  +--> RunenRender --> RunenGPU
                  +--> non-render RunenGPU work
```

Cross-framework translation remains Runenwerk-owned until an independent adapter boundary is proved.

## RunenUI relationship

RunenUI owns semantic UI, state, actions, focus, accessibility, layout, style, text shaping, hit testing, and renderer-neutral paint output.

A future Runenwerk-owned bridge may translate accepted paint primitives into a RunenRender overlay contribution:

```text
RunenUI paint scene
    -> Runenwerk bridge
    -> RunenRender overlay contribution
    -> RunenGPU work
```

The bridge does not expose widget state to RunenRender. RunenRender does not perform UI hit testing or text shaping. RunenUI remains usable with independent standalone backends.

## RunenSDF relationship

RunenSDF remains a CPU/backend-neutral field framework. It owns field values, numerical contracts, bounds, operators, transforms, capabilities, and reference queries.

Rendering or GPU realization is derived integration state. A future reusable adapter may depend on RunenSDF and RunenRender/RunenGPU, but RunenSDF never depends back on it.

## RunenECS relationship

RunenECS remains a generic ECS framework. ECS storage, query, scheduling, and entity/component meaning stay outside RunenGPU and RunenRender. Runenwerk adapters extract prepared domain or GPU values before crossing framework boundaries.

## Public API consequence

The validated work graph is RunenGPU's internal correctness and inspection model. It is not mandatory common-path ceremony.

Ordinary use is conceptually:

```rust
let simulation = simulation.gpu_work(&gpu, &state)?;
let rendering = renderer.gpu_work(&gpu, &scene, request)?;
let submission = gpu.submit("frame 42", [simulation, rendering])?;
```

Advanced inspection is conceptually:

```rust
let prepared = gpu.prepare("frame 42", [simulation, rendering])?;
inspect(prepared.diagnostics());
let submission = gpu.submit_prepared(prepared)?;
```

Both paths use one preparation and validation authority. Ordinary submission validates automatically.

Required ergonomic consequences:

- graph, epoch, admission, realization, and retirement terminology remains internal or advanced;
- strings are labels, not identity, binding, or dependency authority;
- resources and future pipeline bindings use typed references/keys;
- builders use lexical or closure scope rather than repeated nested `.finish()` calls;
- G3 infers data ordering from declared resource access;
- explicit ordering is reserved for real non-data constraints;
- public handles are RAII values with delayed safe retirement after G5;
- structured errors identify operation, resource, cause, provenance where relevant, and corrective action.

## Resource consequence

RunenGPU models unrelated properties independently:

```text
kind
    buffer, texture, texture view, sampler, query set

lifetime
    transient, retained

ownership
    RunenGPU-owned, imported, surface-acquired

transfer and observation
    initial data, update/upload, copy, readback request, export relationship

reconstruction
    source-backed, externally reconstructed, non-reconstructable

memory intent
    ordinary device use, upload staging buffer, readback buffer
```

Imported, exported, readback, and surface-acquired are not interchangeable lifetime classes. Upload/readback memory intent applies only to buffers; textures remain device resources and use explicit copy relationships.

Buffer initialization and texture initialization are distinct. Texture initialization binds checked format, extent, `bytes_per_row`, and `rows_per_image`. A texture view cannot outlive its parent texture lease or checked subresource range.

Labels and provenance are diagnostics/reconstruction evidence, not identity, lookup, binding, dependency, persistence, replay, wire, or cache authority.

## Typed-data consequence

The required boundary is:

```text
Runenwerk or source-domain adapter
    ECS or domain state
        -> explicit prepared typed value or bytes
            -> RunenGPU upload/update contract
```

Uniform, storage, vertex, indirect, transfer, texture-initialization, and readback-decoding layouts are distinct. No universal derive may imply one valid representation for every purpose.

`TypeId` and type names are process-local diagnostics or adapter lookup only. They are not layout, binding, persistence, replay, wire, cache, shader-interface, or cross-process authority.

G2 binds semantic ownership and prepared-data purpose. G4 binds backend layout, validated binding keys, and derive/macro realization. G5 performs uploads, updates, staging, completion, and readback.

## RenderFlow consequence

Current `RenderFlow` is a transitional combined facade. It is decomposed rather than moved, renamed, or wrapped wholesale.

| Current responsibility | Target owner |
|---|---|
| GPU resource identity, descriptions, access, generic work | RunenGPU |
| WGPU context, resources, pipelines, submission | RunenGPU backend |
| views, targets, rendering, image-formation semantics | RunenRender |
| ECS projection and fixed-step scheduling | Runenwerk adapters |
| shader-file paths, hot reload, windows, built-in UI, capture/export policy | Runenwerk adapters |

Useful readability may be reproduced only where it does not retain mixed ownership.

The temporary bridge that seeds logical resource owner scope from `RenderFlowId` is removed through G3/G4. G2-G7 migrate and delete replaced authority incrementally; G8 is the final residual audit, not a delayed bulk migration.

## Extraction sequence

The accepted RunenGPU sequence is:

```text
S0 complete inventory
G1A owner-scoped logical work-resource identity
G2 capabilities, resources, typed handles, prepared-data boundary
G3 access, initialization flow, hazards, generic work, internal graph
G4 context/device admission, WGPU mapping, shaders, pipelines, binding/layout realization
G5 execution, uploads, updates, completion, readback, cancellation, retirement
G6 offscreen graphics and shared compute/render proof
G7 surfaces, generations, thread affinity, and device outcomes
G8 final diagnostics, shutdown, conformance, and residual audit
GX external dornglut/runen-gpu clean cutover
```

S0 and G1A are complete. G2 is active. G2-G7 migrate and delete the authority each phase replaces. G8 is final conformance and residual reach-through audit.

No implementation phase is authorized by this ADR alone. Each phase requires a current-main investigation, decision-complete specification, and one owning issue.

## Proof consequence

Evidence remains separated into:

```text
deterministic conformance
boundary integration
visual showcase
benchmark or stress evidence
```

The primary deterministic compute proof is exact inclusive/exclusive `u32` prefix scan and readback. Headless Game of Life is stateful integration. A known-pattern offscreen draw is graphics conformance. Compute-generated indirect draw is GPU-driven composition. Boids is a representative showcase, not the primary correctness oracle. G7 reuses accepted offscreen workloads for surface proof.

Procedural sky/SDF terrain is the first RunenRender semantic image-formation proof. The SDF history flow is a later temporal/history proof.

Performance measurements are not pass/fail thresholds until the environment and method are separately bound.

## Offline output consequence

Preferred order:

1. Game of Life PNG sequence after G5.
2. Offscreen boids PNG sequence after G6.
3. Procedural sky/SDF/scene sequences after matching RunenRender phases.

Runenwerk owns output clock, seeds, job configuration, bounded in-flight readbacks, filenames, manifests, retry/failure policy, PNG/EXR encoding, and external video encoding. RunenGPU owns completion/readback facts. RunenRender owns image formation. Neither owns MP4/WebM codecs.

## External cutover rule

External repositories are populated only after the corresponding internal boundary is accepted.

The cutover must:

- preserve source provenance and license;
- establish independent locked validation and downstream conformance;
- pin consumers to exact accepted revisions;
- migrate all active consumers;
- delete original internal authority and temporary seams;
- leave no source mirror, forwarding package, compatibility namespace, submodule, source include, moving-branch dependency, or parallel runtime path.

## Rejected alternatives

### Keep WGPU ownership in Runenwerk and extract only helpers

Rejected because it preserves application ownership of the generic GPU execution layer and prevents independent non-render consumers.

### Put generic GPU execution inside RunenRender

Rejected because simulations, tools, image processing, procedural generation, and bakers would depend on renderer semantics or create competing WGPU paths.

### Rename the current renderer to RunenGPU

Rejected because it would erase image-formation ownership and create another broad repository magnet.

### Copy RenderFlow into one of the new repositories

Rejected because RenderFlow combines GPU mechanics, renderer semantics, ECS projection, scheduling, windows, UI, capture, and product policy.

### Wrap every WGPU type one-for-one

Rejected because abstraction is justified only where it adds normalized semantics, ownership, validation, lifecycle, composition, portability, or diagnostics value.

### Create both external repositories immediately

Rejected because clean transfer requires accepted internal boundaries, current consumer migration evidence, and deletion readiness.

### Split each framework into multiple packages immediately

Rejected because package boundaries must follow proven independent dependency/release pressure rather than anticipated organization.

### Preserve compatibility aliases during migration

Rejected because aliases and forwarding modules retain duplicate authority and weaken clean cutover.

### Use one flagship visual demo as conformance

Rejected because visual appeal does not isolate exact resource, lifecycle, execution, or readback correctness.

## Consequences

### Positive

- rendering and non-render compute share one validated execution framework;
- RunenRender remains semantic and backend-independent;
- WGPU ownership is centralized;
- public resources, work, and lifecycle facts become typed and inspectable;
- source-domain frameworks remain independent;
- exact conformance and representative showcases can coexist without conflation;
- independent proof is possible before external extraction;
- later backend/platform evolution is contained behind one boundary.

### Costs

- the current combined renderer must be decomposed incrementally;
- temporary Runenwerk adapters are required while later phases are incomplete;
- current examples and applications migrate as declaration and execution authority moves;
- exact typed-data/layout safety requires explicit G2/G4 work rather than one universal derive;
- external extraction waits for conformance instead of optimizing for immediate repository creation.

These costs are accepted because they remove mixed ownership instead of institutionalizing it.
