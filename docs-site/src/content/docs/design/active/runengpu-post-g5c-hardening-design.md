---
title: RunenGPU Post-G5C Hardening Design
description: Durable correctness, representative-workload, retained-state, diagnostics, browser, backend-neutrality, and standalone-release gates between G5C and GX.
status: active
owner: gpu
layer: framework/gpu
canonical: true
last_reviewed: 2026-08-24
related_docs:
  - ./runengpu-architecture-design.md
  - ./runengpu-g3-access-work-graph-design.md
  - ./runengpu-g3r-definite-initialization-correction.md
  - ./runengpu-phase-requirements-proof-matrix.md
  - ./runengpu-shader-authoring-artifact-boundary.md
  - ../../architecture/repository-family-architecture.md
  - ../../reports/investigations/runengpu-public-api-ergonomics-review.md
  - ../../reports/investigations/runengpu-proof-workload-strategy.md
  - ../../reports/investigations/runengpu-industry-comparison.md
  - ../../workspace/planning/roadmap.md
---

# RunenGPU Post-G5C Hardening Design

## Purpose and authority

This design owns the durable RunenGPU semantic hardening gates after final G5C renderer execution cutover and before standalone extraction.

It does not own live work state or proof-artifact detail:

```text
roadmap
    durable phase order only

this design and other focused phase designs
    semantic contracts and ownership boundaries

RunenGPU Phase Requirements and Proof Matrix
    required proof roles and observable evidence

GitHub issue
    activation and bounded implementation state

workspace RON spec, only when useful after activation
    subordinate implementation handoff

pull request + exact-head CI
    delivery, review, and validation evidence
```

The durable tail is:

```text
G5C final renderer execution cutover
    -> G5R initialization-materialization correction
        -> G6 representative breadth, scale, ergonomics, and direct-backend cost proof
            -> G7B loss, generations, reconstruction, and retained-state continuity
                -> G8 operational, diagnostics, browser, backend-neutrality, and extension conformance
                    -> GX standalone release and extraction
```

No later phase may activate from an unmerged predecessor branch.

## G5R — initialization materialization

### Problem

G3/G3R logical initialization truth must agree with physical content effects.

A logical resource descriptor may carry prepared source bytes while private physical realization creates only the backend object. Source metadata cannot make bytes physically exist or grant graph-entry initialization by itself.

### Narrow supersession of G3R

G5R intentionally revises exactly one G3R descriptor-entry rule:

```text
G3R accepted pre-G5R rule
    Prepared descriptor -> checked prepared initialized coverage

G5R corrected rule
    Prepared descriptor -> declarative initial-content request + retained source material
    descriptor metadata alone -> no initialized coverage
    completed canonical materialization -> initialized coverage
```

The existing public `GpuBufferInitialization::Prepared(data)` and `GpuTextureInitialization::Prepared(data)` vocabulary remains valid as **authoring intent**: it requests initial contents from prepared source data. The variant does not certify that those contents already exist physically. G5R must lower that request through canonical graph-visible materialization before graph-entry initialization may rely on it. A rename or parallel source enum is not required merely to correct this semantic mismatch.

All other G3R distinctions remain intact unless a later explicit correction says otherwise:

```text
access/hazard envelope
    != initialization requirement
    != definite initialization effect
```

In particular, generic shader write access still establishes no new definite initialized coverage by itself.

### Required semantic authority

RunenGPU has one initialization story:

```text
Uninitialized
    no initial-content promise

Zeroed
    creation/postcondition only where the accepted backend contract proves zero content

Prepared(data)
    declarative request for initial content from retained source/reconstruction material
    no physical-content or initialized-coverage claim by metadata alone
```

The `Prepared(data)` request must lower to the same authority as:

```text
create logical resource
    + canonical graph-visible GpuUploadOperation
```

Only completed canonical materialization, or a separately accepted equivalent with the same ordering/effect authority, establishes initialized coverage. If prepared bytes remain retained after materialization, they remain **source authority**, not current initialized-content evidence and not proof that current GPU-mutated contents equal those bytes.

A different implementation is acceptable only if an explicit design revision proves equivalent ordering, initialization, lifecycle, and observability without hidden queue effects or a second transfer authority.

### G5R acceptance boundary

G5R must leave these invariants true:

- public `Prepared(data)` descriptor state denotes requested initialization from source data, not completed initialization evidence;
- prepared payload metadata alone never grants graph-entry initialized coverage;
- physical materialization participates in the same G3/G5 work authority as other transfers;
- no resource-constructor write, hidden queue write, or compatibility transfer path bypasses canonical work ordering;
- zero initialization is claimed only where the backend contract guarantees it;
- convenience creation-with-data lowers to the same authority as explicit resource plus upload;
- retained source bytes, if any, are not confused with completed physical-content continuity;
- generic shader writes do not gain definite initialization authority as a side effect of this correction.

`G5R-C01` in the proof matrix owns the required observable evidence. G6 is blocked until G5R is accepted.

### G5C predecessor condition

G5R remains ordered after G5C because the accepted G5C renderer path uses canonical `GpuUploadOperation` for ordinary and dynamic uploads and does not need prepared descriptor metadata as a physical transfer.

G5C acceptance required any live normal-frame, timing, capture, or readback dependency on `Prepared` metadata being physically materialized without canonical work to be corrected before closure. That condition is sequencing rationale for G5R, not a second live G5C authority.

## G6 — prove the framework

G6 is a proof and correction phase, not an open-ended feature phase. It must establish that the accepted abstraction is useful, understandable, scalable enough for representative work, and acceptably expensive relative to direct WGPU.

The proof matrix owns the exact retained workload/evidence portfolio. This design owns the semantic gates below.

### Representative persistent workload

Reaction diffusion is the required representative persistent compute-to-render workload. It must exercise retained GPU state, repeated compute, compute-to-render sharing, offscreen execution, and the already-accepted surface path through one RunenGPU execution authority.

Persistent compute state must remain valid under the G3R/G5R initialization model. A generic shader write may mutate already-initialized retained state without expanding initialized coverage; any previously uninitialized region still needs an operation-guaranteed initialization effect before later readable use.

`G6-I01` owns the concrete proof envelope and artifact requirements. It complements rather than replaces the exact Game of Life oracle and the more complex boids integration proof.

### Public API ergonomics

A standalone non-render consumer must be able to author and submit ordinary compute work without mandatory knowledge of framework-internal administration.

The advanced descriptor/prepared-graph path may remain public and inspectable, but the ordinary path must not require unrelated knowledge of:

```text
graph-internal identities
backend realization records
manual retirement mechanics
raw backend objects
nested finish ladders
internal cache keys
```

Shader-interface declarations are evaluated under the separately accepted shader-interface authority decision; an explicit interface is not automatically an ergonomics failure merely because reflection could exist.

The representative G6 workload must expose whether application semantics dominate the example or framework ceremony does. `G6-E01` owns the evidence.

### Shader-interface authority decision

Before extraction, G6 must explicitly resolve one question deliberately deferred to G6:

> Are compiler-known shader interface facts manually declared as the accepted RunenGPU authority, or admitted through one immutable compiler/reflection-derived artifact?

The accepted canonical shader-authoring design remains authoritative unless an accepted G6 revision changes it. Planning language does not supersede that rule.

A reflection-derived authority, if accepted, may own only facts inherent in canonical WGSL, such as entry points, stages, binding coordinates, resource kinds, cardinality/layout facts, and workgroup size where applicable.

It must never infer or own:

- G3 resource hazards or causality;
- retained-state continuity;
- application semantics;
- runtime resource selection;
- capability fallback policy;
- filesystem, package, reload, or last-known-good product policy.

`G6-SH01` owns conformance evidence after the decision is accepted.

### Authoring frontend boundary

Canonical WGSL remains the RunenGPU runtime artifact.

Shader authoring languages and their toolchains are deliberately outside RunenGPU. The canonical shader-authoring design assigns source discovery, module/package resolution, compiler selection, translation, source maps, reload, and frontend promotion to Runenwerk/tooling policy.

Therefore **WESL, Slang, or another authoring frontend is not a RunenGPU G6, G8, or GX acceptance requirement**. Requiring one would make standalone RunenGPU extraction depend on a layer the framework explicitly does not own.

A Runenwerk integration proof may still use WESL or another frontend after the RunenGPU boundary is stable:

```text
authoring frontend
    -> canonical WGSL
        -> ordinary RunenGPU program admission
```

That proof is useful evidence of integration and authoring-toolchain independence, but frontend support/promotion remains governed by the shader-authoring design and does not create another RunenGPU source kind, cache, interface, resource, or submission authority.

### Capability aperture

G6 workloads, not backend enum breadth, justify normalized capability and format additions.

Every new feature/format must identify the retained workload that needs it and the semantic, portability, correctness, or measured performance reason. Reaction diffusion should compare a portable storage-buffer representation with a useful storage-texture representation only where that comparison supplies real evidence.

`G6-A01` owns the admission evidence. Do not mechanically mirror WGPU.

### Reusable graph plans remain evidence-gated

Measure repeated-topology graph preparation through `G6-S01` before designing reusable or compiled graph plans.

Only measured material cost may justify a later bounded design. Any such design must preserve one logical graph authority and correct invalidation across resources, programs, capabilities, and device generations.

## G7B — retained-state continuity and reconstruction

G7B completes surface/device loss, generations, and reconstruction and adds lifecycle authority for retained GPU state.

Three facts must remain separate:

```text
initialized-coverage continuity
    which regions are safe to read under G3R/G5R

opaque retained-content continuity
    whether current GPU contents belong to a successfully completed retained-state history

reconstructability
    whether those current contents can be recreated after generation loss
```

None implies either of the others automatically.

### Initialized-coverage continuity

G7B must carry forward accepted G3R/G5R initialized coverage between submissions for retained resources.

A successful generic shader write does not expand initialized coverage merely because it wrote. It may mutate bytes that were already initialized while those bytes remain readable. Exact operation-guaranteed effects such as upload/zero/copy may expand initialized coverage according to G3R/G5R.

This continuity is read-safety authority only. It does not claim that RunenGPU knows the semantic value of shader-produced bytes.

### Opaque retained-content continuity

For a resource explicitly participating in retained state, a successfully completed submission may advance an opaque content-continuity generation even when RunenGPU cannot inspect or reproduce shader-produced values.

```text
submission accepted
    -> execution completes successfully
        -> affected retained state has a new established opaque content generation

submission accepted
    -> failed / lost after a possible write
        -> affected opaque content continuity becomes unknown/revoked
```

This content-continuity fact must not be used to manufacture initialized coverage that G3R/G5R does not establish.

If failure occurs before any relevant write and that no-write fact is provable, previous content continuity may remain valid. Otherwise callers requiring coherent retained state must reconstruct/reset/reimport rather than silently continue from indeterminate contents.

This authority is not a second hazard graph or application-state database.

### Reconstruction sharpens the existing `GpuReconstruction` contract

G7B does **not** add a parallel reconstruction enum. It sharpens the meaning of the existing accepted descriptor vocabulary:

```text
GpuReconstruction::SourceBacked
GpuReconstruction::ExternallyReconstructed
GpuReconstruction::NonReconstructable
```

Those values describe the resource owner's durable reconstruction contract, not merely the resource's creation origin.

`SourceBacked` means the owner commits that the **current required contents** can be recreated after generation loss from retained source/materialization facts or another accepted deterministic reconstruction/replay recipe. Initial source bytes alone do not satisfy that promise after later GPU work has changed the state unless replay from those bytes can reconstruct the required current state.

`ExternallyReconstructed` means a new-generation external owner must reconstruct or reimport the required current state and provide the corresponding evidence.

`NonReconstructable` means current state loss is an accepted possibility and must be reported, never silently replaced.

A buffer initialized from `Prepared(seed)` and then evolved by GPU simulation therefore must not be treated as recoverable to its latest state merely because the seed still exists. It is `SourceBacked` only if the owner maintains a valid replay/checkpoint/source mechanism for the current state; otherwise its declared reconstruction contract must reflect external or non-reconstructable recovery semantics.

A device-generation change invalidates all old physical continuity. Reconstruction establishes new-generation initialized/content continuity only after the required materialization/replay/reimport work completes. Source metadata alone is not initialized-content evidence.

RunenGPU reports lifecycle/reconstruction facts. Runenwerk/application policy decides whether to reset, replay, restore a checkpoint, degrade, or abort.

`G7B-L01` and `G7B-L02` own the required lifecycle evidence.

## G8 — operational and explainable conformance

G8 closes residual operational and portability uncertainty before extraction.

### Explainable diagnostics

Typed diagnostics must be able to explain, where applicable:

- dependency cause and exact overlap;
- initialized/uninitialized reason;
- retained initialized-coverage origin;
- opaque content-continuity generation or revocation;
- reconstruction contract/outcome;
- capability/limit rejection;
- pressure rejection;
- cache incompatibility;
- stale context/device generation.

Human labels and provenance improve usability but do not replace typed facts.

### CPU and GPU observability

Framework CPU cost and GPU execution timing remain separate evidence classes.

Record CPU preparation/validation/realization/execution-preparation/submission cost independently from supported GPU timestamp intervals. Where supported, propagate semantic work labels into private backend debug markers/groups without exposing backend objects publicly.

### Browser support claims

Wasm compilation is not browser WebGPU execution evidence.

Before GX, either execute the declared browser baseline in an actual browser WebGPU environment or document browser execution precisely as compile-only/experimental. `G8-B01` owns the evidence.

### Authoring-toolchain independence

RunenGPU public/runtime semantics must depend on admitted canonical program artifacts, not on filesystem layout, WESL/Slang packages, compiler processes, source watching, or product reload state.

`G8-N01` owns this boundary audit. Frontend promotion itself remains Runenwerk/tooling policy.

### Backend-neutrality audit

Audit each public concept as:

```text
genuinely generic
portable baseline
WGPU-derived but independently implementable
WGPU-specific and therefore private
future extension requiring separate evidence
```

A second backend implementation is not required merely for this audit. If the public model is substantially a renamed WGPU surface, simplify or reconsider the split before GX.

### Reproducibility facts

RunenGPU must expose sufficient typed facts for external reproducibility/capture tooling without owning a persisted capture schema, artifact policy, image/video codec, or Runenwerk bundle format.

`G8-R01` proves the provider-side facts. Runenwerk remains owner of any cross-framework reproducibility bundle.

## GX — standalone release, not source movement

GX transfers accepted RunenGPU authority to `dornglut/runen-gpu` only when the result is an independently usable Rust framework.

The release must establish:

- one public package and private backend implementation;
- public API documentation and runnable headless/offscreen/surface examples;
- independent validation and supported native/web target matrix;
- SemVer, edition/MSRV/toolchain, Cargo feature, license, and provenance policy;
- benchmark baselines and reproducible direct-backend comparison procedure;
- at least one independent non-render consumer using only public RunenGPU APIs;
- Runenwerk and RunenRender consuming only the external package;
- deletion of original internal GPU authority and temporary seams;
- no source mirror, forwarding authority, compatibility facade, submodule/source include, moving-branch dependency, or raw backend reach-through.

The proof matrix owns release evidence and retained proof portfolio. Authoring-frontend support is not part of the RunenGPU package contract; canonical WGSL remains the runtime-source boundary.

## Explicit deferred scope

The following remain evidence-gated and are not implied by this design:

- transient memory aliasing;
- multi-queue scheduling;
- pass fusion;
- broad bindless expansion;
- native Vulkan/D3D12/Metal backends;
- kernel JIT/autotuning;
- backend-independent shader IR;
- custom shader language;
- large mechanical format/feature mirrors;
- reusable graph-plan caching without measured need.

## Activation discipline

1. activate one bounded G5R issue only from accepted G5C authority;
2. accept G5R before activating G6;
3. activate G6 against this design and stable evidence IDs from the proof matrix;
4. use G6 evidence to decide optional capability/performance changes and the explicit shader-interface authority decision;
5. activate G7B, then G8, then GX only from accepted predecessor authority.

Do not pre-create future RON specs merely to mirror this plan. Create one only when an active bounded implementation slice materially benefits from structured subordinate handoff detail.

No phase may preserve temporary compatibility paths merely to reduce cutover cost.
