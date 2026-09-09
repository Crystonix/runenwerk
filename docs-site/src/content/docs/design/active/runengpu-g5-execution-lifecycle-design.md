---
title: RunenGPU G5 Execution Lifecycle Design
description: Decision-complete post-G4R design for executable logical GPU work, bounded execution preparation, backend-neutral submission/progress/readback, G7A surface sequencing, and the final renderer cutover.
status: active
owner: gpu
layer: framework/gpu
canonical: true
last_reviewed: 2026-08-18
related_docs:
  - ./runengpu-architecture-design.md
  - ./runengpu-g3-access-work-graph-design.md
  - ./runengpu-g3r-definite-initialization-correction.md
  - ./runengpu-g4-context-program-realization-design.md
  - ./runengpu-g4c2-presentation-surface-binding-boundary.md
  - ../accepted/runenrender-decomposition-design.md
  - ../../architecture/repository-family-architecture.md
  - ../../adr/accepted/0015-separate-gpu-execution-from-rendering.md
  - ../../reports/investigations/runengpu-g5-execution-lifecycle-investigation.md
  - ../../workspace/specs/pt-runengpu-g5a-executable-work-contracts.ron
  - ../../workspace/planning/roadmap.md
---

# RunenGPU G5 Execution Lifecycle Design

## Authority

This is the fresh G5 planning candidate owned by issue `#284`, derived from exact accepted
post-G3R/G4R main:

```text
d7afaba20a27901e7a6bc4d6d75e6c761c7cbf86
```

The earlier WGPU-27 planning PR `#285` is superseded historical evidence. It is not a predecessor,
merge base, or implementation authority.

This document owns durable G5 architecture. GitHub issues own activation/current state. The G5A RON
file is subordinate implementation-handoff detail only. This document does not authorize G5 Rust by
itself.

## Mission

RunenGPU is the backend-neutral logical-semantics layer for GPU resources and execution. G5 extends
that boundary from validated/realized work through observable execution while WGPU remains private
physical realization.

```text
Runenwerk / domain adapters
        |
        +------------------------------+
        |                              |
        v                              v
RunenRender image semantics     non-render consumers
        |                              |
        +---------------+--------------+
                        v
              RunenGPU logical GPU semantics
              resources / programs / work
              access / initialization / execution
                        |
                        v
              private physical realization
                        |
                        v
                      WGPU 30
                        |
             Vulkan / Metal / D3D12 / WebGPU / ...
```

G5 must not turn RunenGPU into a public WGPU command wrapper.

## Ordered delivery

The accepted durable sequence remains:

```text
accepted G4R
    -> G5A executable logical work closure
    -> G5B reusable surface-independent execution lifecycle
    -> G7A minimal durable surface foundation
    -> G5C final renderer/UI/timing/capture execution cutover
    -> G6 breadth/scale/offscreen/cost proof
    -> G7B complete loss/generation/reconstruction
    -> G8 conformance and zero reach-through
    -> GX standalone transfer
```

Only G5A may activate directly after this planning result is accepted.

# 1. Governing laws

## 1.1 One logical work authority

`GpuWorkOperation`, `GpuWorkFragment`, and `GpuPreparedWorkGraph` remain the sole RunenGPU authority
for:

- logical GPU operations and resources;
- exact G3R initialization requirements/effects;
- access/hazard intent;
- dependencies and deterministic prepared order;
- operation-derived capability requirements.

G5 completes operation payloads and adds execution lifecycle. It does not create another public
command graph, execution DAG, backend command-buffer IR, or permanent renderer sidecar with duplicate
GPU semantics.

Private backend encode plans are derived implementation state only.

## 1.2 Logical semantics stay above realization

Public RunenGPU may expose semantic concepts such as:

```text
GpuComputeOperation
GpuRenderOperation
GpuUploadOperation
GpuReadbackOperation
GpuPreparedSubmission
GpuSubmissionId
GpuSubmissionStatus
GpuReadbackId
GpuReadbackStatus
```

Public correctness must not depend on:

```text
wgpu::Device
wgpu::Queue
wgpu::CommandEncoder
wgpu::CommandBuffer
wgpu::SubmissionIndex
wgpu::PollType
wgpu::BufferView
wgpu::Surface
native backend synchronization handles
```

## 1.3 G3R remains initialization authority

G5 must preserve:

```text
access/hazard envelope
!=
exact initialization requirement
!=
operation-guaranteed initialization effect
```

Upload/readback/copy/clear/resolve/attachment work participates in the same G3R model. No execution
convenience may bypass it.

## 1.4 Accepted is a RunenGPU fact

The portable submission lifecycle is:

```text
Accepted
    -> Completed
    -> Failed
```

`Accepted` means RunenGPU irreversibly admitted the submission. It does not claim that the private
backend queue has already completed—or even necessarily begun—the physical submit operation.

A `GpuSubmissionId` is allocated only at this irreversible acceptance point. Once published, every
later synchronous/asynchronous failure terminalizes that accepted ID exactly once.

# 2. G5A — executable logical work closure

G5A completes logical operation meaning. It does not submit work, map results, own progress, or own
surfaces.

## 2.1 Compute operation

Directionally:

```text
GpuComputeOperation
  pipeline: GpuComputePipelineDescriptor
  bindings: GpuRuntimeBindingSet
  dispatch: GpuDispatchIntent
  timestamp_writes
```

Exact type decomposition may follow current source, but the operation must contain all backend-neutral
meaning required for later private execution.

### Direct and indirect dispatch

```text
GpuDispatchIntent
  Direct { x: u32, y: u32, z: u32 }
  Indirect { arguments: GpuBufferHandle, offset: u64 }
```

Direct rules:

- each dimension is admitted against the normalized maximum compute-workgroups-per-dimension;
- zero is valid;
- any zero dimension means an empty shader invocation grid;
- the dispatch still retains interface-derived binding accesses/hazards because portable dispatch
  usage validation is defined from the active pipeline/bind groups rather than actual invocation
  count;
- zero invocations establish no shader execution effect and no definite shader initialization
  effect;
- explicit operation-owned timestamp semantics remain valid;
- private encoding may elide an empty shader dispatch only when validation/diagnostic equivalence is
  preserved; logical access/hazard semantics remain unchanged.

Indirect rules:

- the argument buffer contributes one exact 12-byte read beginning at the checked offset;
- offset/alignment are structurally validated;
- runtime GPU argument values are queue-timeline data and are not host planning truth;
- the operation requires `GpuCapabilityFeature::IndirectExecution`;
- runtime arguments invalid under the portable indirect-execution contract make the indirect
  dispatch non-executing rather than turning runtime GPU data into a host-side planning error;
- a non-executing indirect dispatch retains its argument-buffer read and conservative
  interface-derived usage/hazard scope, but establishes no shader execution or definite shader
  initialization effect;
- boundary validity is defined by the accepted portable/WebGPU contract for the admitted device, not
  by an independently invented RunenGPU off-by-one rule.

### Compute usage scope and writable aliasing

Each direct or indirect compute dispatch is one logical usage scope.

For that dispatch's effective pipeline/binding set:

- every binding potentially accessible through the active pipeline layout participates in usage
  validation even when a direct dispatch has a zero dimension or an indirect dispatch later becomes
  non-executing from runtime arguments;
- overlapping effective buffer ranges reject if either binding is writable;
- overlapping effective texture subresources reject if either binding is writable;
- disjoint ranges/subresources remain valid;
- dynamic offsets are applied before effective-range alias validation;
- diagnostics retain exact range/subresource evidence rather than collapsing to whole-resource
  booleans.

This is the compute analogue of draw-local writable-binding validation. It is distinct from G3's
coarser inter-node hazard ordering.

## 2.2 One indirect-execution capability and one semantic guarantee

Clean-cut:

```text
GpuCapabilityFeature::IndirectDraw
    -> GpuCapabilityFeature::IndirectExecution
```

The admitted WGPU fact already represents indirect draw and indirect dispatch capability together.
Existing indirect draw requirements migrate to `IndirectExecution`; indirect compute uses the same
fact.

Do not keep `IndirectDraw` as a deprecated alias and do not create a parallel `IndirectDispatch`
capability.

The public capability additionally means RunenGPU can preserve the portable runtime-validity
semantics of indirect execution. An implementation/debug/environment switch may not weaken those
semantics after `IndirectExecution` has been admitted.

For the WGPU 30 backend specifically:

- `InstanceFlags::VALIDATION_INDIRECT_CALL` (or a future proven-equivalent mechanism) is a required
  private realization invariant for indirect execution;
- RunenGPU-owned instance construction must restore/force that invariant after environment-derived
  WGPU options are resolved, so `WGPU_VALIDATION_INDIRECT_CALL=0` cannot silently weaken public
  RunenGPU behavior;
- if a backend cannot prove equivalent invalid-indirect-call behavior, it must not admit
  `IndirectExecution`;
- the validation mechanism remains private and is not promoted into RunenGPU capability vocabulary or
  user configuration.

The same invariant applies to accepted indirect draw semantics: invalid runtime indirect arguments
become non-executing according to the portable contract rather than entering undefined backend
behavior.

## 2.3 Render operation and draws

One `GpuRenderOperation` remains one logical render pass:

```text
GpuRenderOperation
  color attachments
  optional depth/stencil attachment
  draws: [GpuRenderDraw]
  timestamp writes
```

Each draw owns complete effective execution state:

```text
GpuRenderDraw
  pipeline: GpuRenderPipelineDescriptor
  bindings: GpuRuntimeBindingSet
  vertex buffers
  optional index buffer
  draw intent
  viewport
  scissor
  blend constant
  stencil reference
```

This permits multiple compatible pipelines/bindings/draws inside one pass without fabricating extra
attachment load/store boundaries.

### Pass signature

Preparation derives:

- effective extent;
- sample count;
- ordered color formats;
- optional depth/stencil format.

Every draw pipeline must match the pass signature before backend encoding. Pipeline blend/primitive/
write state remains draw-pipeline state and may differ where the pass signature permits.

## 2.4 Render usage scope versus draw-local aliasing

G5A explicitly separates two rules.

### Pass-wide usage compatibility

One render pass is one usage scope. Across all relevant pass commands:

- compatible storage/storage usage may coexist across distinct draws;
- attachment/attachment combinations remain subject to accepted attachment-region rules;
- incompatible read-like/write-like combinations reject;
- writable attachment usage cannot alias an incompatible sampled/storage use in the pass;
- vertex/index/indirect/bind-group state participates at the pass scope according to the portable
  execution contract.

### Draw-local writable binding aliasing

For each draw's effective pipeline/binding set:

- overlapping buffer ranges reject if either effective binding is writable;
- overlapping texture subresources reject if either effective binding is writable;
- disjoint ranges/subresources remain valid;
- dynamic offsets are applied before overlap analysis.

This replaces the current broader model that can reject any overlapping write-capable render access
without distinguishing pass compatibility from one draw's writable aliasing.

## 2.5 Vertex/index bindings

`GpuVertexBufferBinding` owns:

- vertex slot;
- logical buffer;
- checked byte range.

Pipeline stride/step/attributes remain G4 pipeline authority.

`GpuIndexBufferBinding` owns:

- logical buffer;
- checked byte range;
- accepted `GpuIndexFormat`.

No duplicate index-format type is introduced.

## 2.6 Dynamic render state

Each draw carries complete effective state rather than inheriting semantic meaning from prior backend
commands:

- viewport with finite canonical values and `0 <= min_depth <= max_depth <= 1`;
- viewport coordinate/extent validity against the portable bounds derived from the admitted
  `max_texture_dimension_2d` fact;
- scissor checked against effective render extent;
- finite canonical blend constant;
- stencil reference `u32`.

Zero-area viewport/scissor are valid no-rasterization cases. Private encoding may elide redundant
state setters; the logical value remains explicit. RunenGPU rejects invalid viewport state before
private command encoding rather than relying on backend validation as ordinary control flow.

## 2.7 Color clear semantics

Clean-cut `GpuColorClearValue` from a normalized-color-only policy to generic color-attachment clear
semantics:

- four finite canonical `f64` components;
- reject non-finite values;
- canonicalize signed zero consistently;
- no generic `[0, 1]` restriction;
- target attachment format owns conversion/admission;
- depth clear remains a separate depth-domain value with its own normalized constraint.

This removes an invalid ceiling from an existing generic concept. It does not authorize speculative
new texture formats.

## 2.8 One runtime binding-use model

`GpuRuntimeBindingSet` is the complete logical binding use for one compute invocation or render draw.
It preserves accepted G4 typed runtime binding values and deterministic pipeline-layout slot order.

It owns semantic facts including:

- logical resource identity;
- static buffer offset/size;
- optional logical `u64` dynamic offset;
- texture/view/sampler identity;
- interface-derived access intent.

## 2.9 Dynamic offsets are execution-use state

Physical bind-group identity must exclude dynamic offsets.

```text
GpuRuntimeBindingSet
      |
      +--> effective logical access range
      |       static offset + dynamic offset + size
      |
      +--> static physical binding projection
              layout
              resource identities
              static buffer offsets/sizes
              texture/view/sampler facts
              NO dynamic offset
                      |
                      v
              GpuRealizedBindGroup
```

G5A changes the private G4C2 key/record accordingly. Uses differing only by dynamic offsets reuse one
physical bind-group realization.

Remove physical-record APIs that claim a single invocation's complete runtime values are properties
of the realized bind group. Logical values remain inspectable from logical work.

G5B later owns ordered backend dynamic-offset slices and checked private narrowing from logical `u64`
to the backend domain.

## 2.10 Complete operation-derived access truth

After G5A:

- Compute derives runtime-binding accesses, indirect argument access, and timestamps. Zero direct
  dispatch and runtime-invalid indirect dispatch retain conservative interface-derived
  access/hazard evidence while producing no shader execution/definite initialization effect.
- Render derives attachment/resolve/binding/vertex/index/indirect/timestamp accesses.
- Copy/Clear/Resolve/Present retain accepted G3/G3R semantics unless a directly proven defect requires
  correction.
- Upload derives one exact destination write and initialization effect.
- Readback derives one exact source read and no initialization effect.

Renderer-authored duplicate generic GPU access truth is deleted as generic operations become complete.
Renderer/domain provenance may remain as non-authoritative metadata.

## 2.11 Upload

Add graph-visible immutable Upload work:

```text
GpuUploadOperation
  destination range/region
  immutable checked payload
```

It:

- participates in G3 hazards and exact initialization effects;
- owns payload value independently of physical staging;
- may have process-local correlation/record identity distinct from semantic payload equality;
- does not define persistence/content/cache identity;
- leaves physical transfer strategy to G5B.

## 2.12 Readback

Add graph-visible Readback work:

```text
GpuReadbackOperation
  source range/region
  GpuReadbackId
```

It:

- participates in exact read hazards/requirements;
- cannot feed CPU data into a later node in the same submission;
- exposes normalized logical result data, never mapped backend ranges;
- keeps submission completion and readback materialization distinct.

## 2.13 Execution-required normalized limits

G5A closes the limit vocabulary required by the operations it introduces or materially corrects. The
exact new normalized device/workload fields authorized by this slice are:

```text
max_texture_dimension_2d
max_bind_groups
max_bind_groups_plus_vertex_buffers
max_dynamic_uniform_buffers_per_pipeline_layout
max_dynamic_storage_buffers_per_pipeline_layout
max_compute_workgroups_per_dimension
```

Ownership/use is explicit:

- `max_texture_dimension_2d` supplies the portable bound needed by explicit viewport admission and
  closes the already-adjacent 2D resource/device fact gap;
- `max_bind_groups` admits complete runtime binding-set/pipeline-layout use;
- `max_bind_groups_plus_vertex_buffers` admits simultaneous render bind-group + vertex-buffer slots;
- the two dynamic-buffer limits admit the dynamic declarations that G5A makes executable per use;
- `max_compute_workgroups_per_dimension` admits direct dispatch and defines the device bound used by
  portable indirect-dispatch runtime validity.

Existing binding-size/count, vertex-buffer, color-attachment and dynamic-alignment facts remain
authoritative. G5A does not add the other WGPU `Limits` fields: shader workgroup shape/storage,
per-stage resource counts, vertex attributes/stride, texture dimensions other than the newly required
2D fact, buffer-size ceilings, and specialized feature limits remain with their existing
resource/program/pipeline owners or future evidence gates.

If implementation evidence proves that one of G5A's accepted new public operations can still be
rejected before encoding by a device-dependent limit absent from this closed set, that is a planning
defect: stop and amend the owning design rather than silently mirror another WGPU field or defer
normal validation to backend failure.

Do not mirror the entire WGPU limits structure mechanically.

## 2.14 G5A renderer transition

Current renderer structures may retain RunenRender planning/provenance. Once a generic operation is
execution-complete, the renderer must not retain an independent pipeline/binding/draw/access
representation for the same GPU operation.

Temporary pre-G5B raw execution may consume/reference generic prepared operations. It may not
reconcile parallel old/new GPU semantics.

G5A does not broaden `CurrentRenderDeviceQueue` or `CurrentRenderExecutionBridge`; their guaranteed
final deletion owner remains G5C after G7A.

# 3. G5B — reusable surface-independent execution lifecycle

G5B consumes accepted G5A and gives `GpuContext` bounded execution ownership.

## 3.1 Independent capacities

One context-local execution policy initially distinguishes:

```text
max_prepared_submissions
max_in_flight_submissions
max_upload_bytes_in_flight
max_readback_bytes_in_flight
max_pending_readbacks
```

These remain distinct from:

- G4 realization-record capacity;
- logical resource byte size;
- physical GPU allocation/residency;
- renderer/product budgets.

Pressure is structured and inspectable. There is no hidden unbounded submission queue.

## 3.2 Preparation

Directionally:

```text
GpuPreparedWorkGraph
        |
        v
GpuContext::prepare_submission(...).await
        |
        v
GpuPreparedSubmission
```

Preparation:

1. reserves prepared capacity through cancellation-safe RAII;
2. validates context/device-generation affinity;
3. validates G5A execution limits/dynamic offsets;
4. obtains/retains required G4 realizations;
5. plans immutable upload/readback staging;
6. builds ordered private dynamic-offset slices;
7. publishes one single-use prepared ticket only after the record is complete.

No submission ID exists during preparation.

## 3.3 Prepared ownership and revocation

A caller-held `GpuPreparedSubmission` is a single-use ticket, not the owner of context execution
state. The context owns the prepared record and its G4/staging reservations.

Holding a ticket alone must not keep the context/device/queue or prepared capacity alive forever.

- Dropping an active ticket releases its record while the context exists.
- `begin_shutdown()` can revoke every still-prepared record even if callers retain ticket values.
- A revoked ticket remains diagnosable but cannot execute.

## 3.4 Submit rejection preserves ownership

Pre-acceptance rejection returns ownership of the prepared value:

```text
submit_prepared(prepared)
  -> Accepted(GpuSubmission)
  -> Rejected(GpuSubmitRejection { prepared, reason })
```

Exact Rust spelling may differ; ownership semantics may not.

Retryable pressure rejection does not force reconstruction/re-realization. Stale/revoked reasons are
explicit on the returned prepared value.

## 3.5 Irreversible acceptance

Submit admission is ordered:

```text
validate ticket/context/generation/state
 -> reserve/convert in-flight + staging capacity
 -> remove prepared record from retryable prepared authority
 -> allocate/publish GpuSubmissionId
 -> state = Accepted
 -> private encode/queue work
```

Before ID publication:

- no queue action;
- no logical Upload side effect;
- no accepted submission record.

After publication:

- the call cannot report a pre-acceptance rejection;
- encode/queue/device-health/callback failure terminalizes the accepted ID as `Failed` exactly once;
- success eventually terminalizes as `Completed` exactly once.

## 3.6 Private encoding

G5B privately encodes accepted G5A operations through accepted G4 realizations.

Each compute/render binding use supplies:

```text
GpuRealizedBindGroup
+ exact ordered checked dynamic-offset slice
```

Logical offsets remain `u64`; backend narrowing is checked privately before encoding.

No raw backend object becomes public or renderer-owned.

Indirect execution uses only a backend path whose private realization proves the accepted runtime
validity/no-op semantics. WGPU instance/debug/environment configuration is never allowed to downgrade
that guarantee after `IndirectExecution` admission.

## 3.7 Transactional Upload baseline

Initial Upload lowering uses encoded staging copies.

WGPU queue writes are staged for a later queue submit. If they were used as the default logical Upload
path, a later failure before the intended submission could leave physical writes to be flushed by
subsequent logical work. That would break one-submission ownership and G3R initialization/hazard
truth.

A later private queue-write optimization is allowed only when evidence proves all staged writes are
transactionally coupled to the same accepted submission and cannot survive its failure/rejection
boundary. Public Upload semantics remain unchanged.

## 3.8 Submission observation

```text
GpuSubmissionStatus
  Accepted
  Completed
  Failed(structured failure)
```

There is no public backend `Submitted`, fence, queue index, or poll state.

Terminal observation handles may outlive context cleanup after execution-owned capacity/G4 retention
has been detached.

## 3.9 Readback observation

```text
GpuReadbackStatus
  Pending
  Ready(GpuReadbackBytes)
  Failed(structured failure)
```

Submission completion and readback materialization are separate facts. A submission can be
`Completed` while mapping/result materialization is still `Pending`. Submission failure fails its
independent readbacks.

Mapped WGPU ranges never escape. Texture results remove physical row padding and expose requested
logical data/metadata.

## 3.10 Progress

`GpuContext::progress()` is nonblocking and backend-neutral. It may:

- privately poll native WGPU where useful;
- drain/observe completion/map callbacks;
- terminalize submission/readback state;
- release safe staging/G4 retention;
- advance graceful shutdown.

Browser WebGPU callback progress is event-loop driven; `progress()` must not promise a native-style
blocking wait there.

RunenGPU owns no mandatory Tokio/Futures runtime, worker thread, or immortal polling service.

## 3.11 Graceful shutdown

```text
Running
 -> ShuttingDown
 -> Closed
```

`begin_shutdown()`:

- is idempotent;
- rejects new preparation/acceptance;
- revokes all unaccepted prepared records;
- leaves accepted submissions/readbacks progressable/observable;
- reaches `Closed` only after accepted work is terminal and execution-owned resources detach.

Product recovery remains outside RunenGPU.

## 3.12 Last-context Drop

Last `GpuContext` Drop is abrupt owner loss, not implicit graceful shutdown.

Required consequences:

- no new execution;
- prepared records revoked/released;
- nonterminal accepted submission/readback observations terminalize with a structured context-drop
  failure exactly once;
- no public claim that issued hardware work was synchronously cancelled;
- Drop does not block waiting for GPU completion;
- private backend/driver lifetime rules remain responsible for already-issued physical work after
  RunenGPU execution authority is gone;
- detached terminal observations may survive without device/queue/G4 realization authority.

## 3.13 G4 retention

Prepared/in-flight state retains exact accepted G4 realization records only while required. G5 does
not create a second resource/program/pipeline retirement registry. Safe release drops existing `Arc`
retention and returns reclamation authority to existing G4 bounded registries.

## 3.14 G5B proof

Acceptance requires a genuine non-render headless proof:

```text
Upload input
 -> compute
 -> compute using a second dynamic offset but same physical bind group
 -> Readback
 -> progress to terminal submission/readback
 -> verify result
```

It must prove:

- two valid dynamic offsets reuse one physical bind-group record;
- no renderer API participates;
- G3R remains initialization/hazard authority;
- pressure rejection preserves prepared ownership;
- cancellation/drop releases prepared capacity;
- exactly-once terminal outcomes;
- readback completion distinct from submission completion.

Native exact-head runtime proof is required. Wasm compilation/conformance is mandatory. Real browser
WebGPU lifecycle evidence is required where repository infrastructure can execute it; otherwise the
limitation is recorded and no stronger runtime claim is made.

# 4. G7A — minimal durable surface foundation

G7A remains deliberately between G5B and G5C.

## 4.1 Purpose

Current Runenwerk still owns raw WGPU surfaces/current-host integration. Migrating final renderer
execution before reusable surface identity/acquire/present semantics exist would require either a
disposable G5-only surface API, raw-surface escape hatch, or Winit/product policy inside RunenGPU.
All are rejected.

## 4.2 Minimum G7A facts

G7A owns only generic facts needed for reusable presentation execution:

- opaque `GpuSurfaceId`;
- context/device-generation affinity;
- surface/acquired-image generation or lease identity sufficient to reject stale use;
- normalized capabilities and admitted configuration;
- acquisition outcomes;
- acquired-image identity/allowed role facts;
- presentation acceptance/outcomes.

Physical WGPU surface objects remain private.

## 4.3 Exclusions

G7A does not own:

- Winit windows/event loops;
- application resize/minimize/visibility policy;
- product retry/fallback/recovery choice;
- complete device replacement/reconstruction;
- renderer image-formation semantics;
- persisted surface identity.

Full loss/generation/reconstruction conformance remains G7B.

# 5. G5C — final renderer execution cutover

G5C consumes accepted G5B + G7A.

## 5.1 Migration

Migrate renderer/UI/timing/capture paths so generic GPU work uses the same G5 logical/execution model.

Renderer/Runenwerk may retain:

- scene/view/material/image-formation decisions;
- render-plan provenance;
- feature/fallback/product policy;
- capture selection and product artifact policy.

They no longer own:

- generic uploads;
- raw command encoding;
- generic compute/render/copy execution;
- queue submission;
- GPU completion/progress;
- mapped readback mechanics;
- generic surface acquire/present mechanics.

## 5.2 Required clean deletion

Final G5C accepted source has zero definitions/call sites for:

```text
CurrentRenderDeviceQueue
current_render_device_queue()
CurrentRenderExecutionBridge
current_render_execution_bridge()
```

Purpose-typed terminals existing only to lend G4 realized WGPU objects into renderer execution are
deleted with the bridge.

No compatibility alias, broad closure bridge, `raw_device()`, `raw_queue()`, `as_wgpu_*`, forwarding
module, or parallel backend path replaces them.

## 5.3 Surface cutover

Current-host surface physical ownership/configuration/acquire/present moves behind accepted G7A
generic contracts. Runenwerk retains Winit/window/event-loop/product recovery policy. G5C does not
create a renderer-private second surface model.

# 6. Failure ownership

Failures are structured by semantic owner/lifecycle stage. Initial categories include:

```text
logical work invalid
context/generation mismatch
execution-limit rejection
prepared revoked/consumed
pre-acceptance pressure
shutdown/closed
backend encode/validation-contract failure
backend queue/device failure
device/context loss
readback mapping/result failure
surface acquisition/present failure (after G7A)
context dropped
```

Backend strings may accompany diagnostics but never determine semantic category, retry policy,
persistence identity, or equality.

# 7. Identity and determinism

- G3 prepared order remains deterministic for identical logical input.
- Physical realization/cache order is not semantic order.
- `GpuSubmissionId`, `GpuReadbackId`, prepared-ticket IDs, and transfer-record IDs are process-local
  operational identities.
- They are not wire/persistence/replay/content/cache/cross-process identity.
- Submission acceptance order is owner-local execution order, not a universal Runenwerk logical clock.

# 8. Pressure and memory dimensions

Keep separate:

```text
logical resource size
G4 realization-record count
G5 prepared count
G5 in-flight count
G5 upload staging bytes
G5 readback staging bytes
physical backend allocation/residency
```

RunenGPU may report facts it owns. It must not infer exact physical GPU residency from logical or
staging counts.

# 9. Portability

WGPU is the first private backend, not the public capability ceiling.

G5 uses:

- normalized capability/limit facts;
- backend-neutral lifecycle;
- backend-neutral progress/observation;
- normalized readback data;
- explicit unsupported capability rejection.

Do not expose native-only transition/barrier APIs, raw HAL objects, backend fences, browser promise
objects, WGPU instance flags, or backend validation toggles through G5.

Specialized/native interoperability requires separate accepted evidence and cannot be a generic raw
escape hatch.

# 10. Validation and conformance

## G5A

At one unchanged reviewed head require:

- complete executable compute/render/Upload/Readback operation tests;
- zero-dispatch proof preserving binding access/hazard evidence while producing no shader execution/
  definite initialization effect;
- compute-dispatch usage-scope and writable-alias proofs with dynamic effective ranges;
- indirect compute/draw capability/access proofs;
- invalid runtime indirect arguments proven non-executing without losing conservative usage/hazard
  evidence;
- a WGPU proof that environment/debug configuration cannot disable the accepted indirect-call
  validity guarantee;
- pass-wide render usage versus draw-local alias proofs;
- target-format-aware color clear proofs;
- dynamic-offset effective-range and static-realization-key proofs;
- exact normalized execution-limit mapping/admission proofs for the six newly authorized fields;
- viewport admission proof derived from normalized `max_texture_dimension_2d`;
- renderer duplicate GPU-semantic deletion guards;
- deterministic preparation/diagnostics;
- `cargo validate`;
- `git diff --check`;
- Documentation Build;
- exact-head CI and complete diff review.

## G5B

Require:

- headless non-render compute/upload/readback runtime proof;
- cancellation-safe preparation tests;
- retryable submit rejection returning prepared ownership;
- acceptance/exactly-once terminalization tests;
- shutdown/revocation/last-context-drop tests;
- staging/readback pressure tests;
- native progress/runtime evidence;
- wasm/conformance and browser runtime evidence where infrastructure permits;
- no raw backend public API;
- canonical exact-head validation/review.

## G7A

Require independent surface capability/identity/generation/acquire/present design and proof before
G5C activation.

## G5C

Require representative renderer/UI/timing/capture paths through accepted G5/G7A plus structural zero
counts for every deleted raw execution seam.

# 11. Explicit non-goals

G5 does not authorize:

- mesh/task shaders;
- ray tracing/query APIs;
- sparse resource APIs;
- multi-queue/queue-family public policy;
- raw native backend passthrough;
- broad placed-resource aliasing;
- persisted backend/pipeline caches;
- video/tensor/distributed-GPU systems;
- RunenRender image-formation semantics;
- ECS/world/SDF semantics;
- application scheduling;
- complete G7B reconstruction;
- standalone repository extraction.

Those require their own evidence gates.

# 12. Next activation

This planning result is decision-complete only if owner review confirms G5A implementation need not
invent operation, execution ownership, lifecycle, pressure, progress, transfer, surface-sequencing,
or portability policy mid-slice.

After planning acceptance, activate exactly one bounded implementation issue:

```text
G5A — executable logical work closure
```

Do not activate G5B, G7A, G5C, or GX simultaneously. Each consumes accepted predecessor authority
and begins from a fresh exact-main census.
