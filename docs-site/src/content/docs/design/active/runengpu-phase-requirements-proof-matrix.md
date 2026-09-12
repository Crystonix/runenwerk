---
title: RunenGPU Phase Requirements and Proof Matrix
description: Canonical proof roles, observable evidence, and retained artifact requirements for RunenGPU from accepted G5 execution through standalone extraction.
status: active
owner: gpu
layer: framework/gpu
canonical: true
last_reviewed: 2026-09-05
related_docs:
  - ./runengpu-architecture-design.md
  - ./runengpu-g3-access-work-graph-design.md
  - ./runengpu-g3r-definite-initialization-correction.md
  - ./runengpu-g5-execution-lifecycle-design.md
  - ./runengpu-post-g5c-hardening-design.md
  - ./runengpu-shader-authoring-artifact-boundary.md
  - ../../architecture/repository-family-architecture.md
  - ../../reports/investigations/runengpu-proof-workload-strategy.md
  - ../../reports/investigations/runengpu-public-api-ergonomics-review.md
  - ../../workspace/planning/roadmap.md
---

# RunenGPU Phase Requirements and Proof Matrix

## Purpose and authority

This document owns **proof role, observable evidence, and retained artifact requirements**. It does not own the semantic contract being proved.

```text
phase / focused design
    what RunenGPU must mean or guarantee

this matrix
    how acceptance demonstrates that guarantee
```

It also does not own:

- durable phase order — the roadmap owns sequence;
- activation, blockers, current branch, or completion state — GitHub issues own live work;
- implementation handoff detail — a workspace RON spec may be created only for an active bounded slice and remains subordinate;
- source-language/toolchain promotion — the shader-authoring design and Runenwerk tooling policy own that;
- image/video codecs or persisted artifact policy — Runenwerk proof/batch tooling owns those.

Requirement IDs are stable references for future issues, tests, CI descriptions, and closeouts. Future issues should reference the IDs and the semantic owner instead of copying this document.

Historical accepted phases are not reopened merely because their proof evidence is retained here. New retained-evidence requirements apply when a later phase deliberately reuses that workload.

## Evidence taxonomy

### Exact conformance

Machine-checkable contract evidence: exact bytes/values/counts/checksums, exact selected pixels where portable, exact structured outcomes, or exact graph/dependency/initialization facts.

A screenshot or plausible visual result never replaces exact conformance.

### Structural integration

Machine-checkable evidence that accepted owners compose through one authority: inferred dependencies, shared context, canonical execution, no raw-backend reach-through, or offscreen/surface paths using the same semantics.

### Operational evidence

Structured evidence for progress, pressure, cancellation, shutdown, cache behavior, loss, reconstruction, continuity, and bounded resource use.

### Performance evidence

Measured CPU/GPU cost under a documented equivalent comparison envelope. Performance is diagnostic unless a separately accepted controlled budget makes a threshold normative.

### Visual showcase artifact

Human-inspectable GPU output demonstrating representative value. Tolerant image comparison may support workloads whose floating-point behavior is not bit-stable, but visual output is never the sole correctness oracle.

### Offline sequence artifact

A reproducible ordered sequence:

```text
fixed seed / parameters / dimensions / timestep or iteration schedule
    -> ordered logical frames
        -> per-frame state or image evidence
            -> versioned manifest
```

For GPU image-producing proofs, the baseline human-facing artifact is an ordered lossless image sequence encoded by Runenwerk proof/batch tooling. PNG is the default. EXR is used only when a proof explicitly requires higher precision or HDR semantics.

GIF/MP4/WebM are optional derivatives and never RunenGPU correctness authority.

## Artifact ownership

```text
RunenGPU
    GPU work semantics
    execution / completion
    offscreen targets where required
    readback facts and bytes

Runenwerk proof / batch tooling
    fixed jobs, seeds, frame numbering
    manifests and checksums
    lossless image encoding
    artifact directories
    retry/failure policy

optional external encoder
    GIF / MP4 / WebM or equivalent media
```

RunenGPU does not acquire filesystem, codec, filename, media, or persisted product-artifact policy merely because its proofs generate images.

# Retained G5 evidence

## G5-C01 — prefix scan/readback

**Role:** exact conformance.

**Semantic owner:** accepted G5 work/execution contracts.

**Evidence:** retain the fixed 4,097-element `u32` prefix-scan proof with exact inclusive/exclusive full-output comparison, exact total-count verification, hierarchy-crossing temporary storage, asynchronous completion/readback, and no renderer/window/product dependency.

No image artifact is required.

## G5-C02 — headless Game of Life

**Role:** exact stateful conformance.

**Semantic owner:** accepted G5 work/execution contracts.

**Evidence:** retain the accepted fixed job:

- 160 x 90 cells;
- fixed seed, with source state prepared outside RunenGPU and canonically uploaded;
- 16 steps;
- ping-pong storage;
- exact full-grid GPU equality with the CPU oracle;
- final live-cell count `2,063`;
- final FNV-1a-64 `0xBD710B88594CD584`;
- optional selected-cell diagnostics, if retained, derive expected values from the CPU oracle and are not an independent truth source;
- asynchronous completion/readback.

The phrase **offline sequence** means deterministic state progression, not graphics support.

When G6 or GX retains a state sequence, each retained step digest must be derived from or compared with the CPU oracle rather than merely recorded from the GPU result. The manifest records job identity, dimensions, seed, step indices, and expected/observed sequence identity.

Runenwerk tooling may optionally rasterize validated states to PNG frames for review. Those frames are not RunenGPU graphics evidence and do not retroactively change G5 acceptance.

## G5-C03 — known integer compute-to-texture

**Role:** exact texture/transfer conformance.

**Semantic owner:** accepted G5 transfer/resource contracts.

Where this conditional proof is retained, require exact/documented integer output, texture-to-buffer readback, and row-padding normalization.

A single PNG may accompany validated bytes as supporting evidence; the image is not the oracle.

## G5-O01 — progress/pressure lifecycle

**Role:** operational evidence.

**Semantic owner:** G5 execution lifecycle design and RunenGPU architecture design.

Retain structured tests for bounded in-flight work, staging/upload/readback pressure, callback/reentrancy rules, cancellation, shutdown with pending work, and exactly-once terminal outcomes.

No visual artifact is required.

# G5C — final execution cutover

G5C semantics are owned by the accepted G5 design and historical G5C implementation authority; this matrix retains the required integration and ownership evidence only.

## G5C-I01 — normal-frame authority

**Role:** structural integration.

**Evidence:** demonstrate that one normal frame uses accepted RunenGPU surface acquisition, one frame-level G3/G3R work authority, G5 preparation/submission, render/UI/copy work, ordered `Present`, and terminal observation.

The proof must show that the normal path no longer owns raw renderer command encoding, queue submission, raw surface acquisition, or raw presentation.

## G5C-I02 — residual bridge deletion

**Role:** ownership cutover.

**Evidence:** after timing/capture/readback migration, structural guards and exact-head review show zero definitions/call sites for the temporary G5C execution/surface bridges named by the owning issue and no replacement raw-device/raw-queue/renderer-encoder/surface-view escape hatch or second execution DAG.

# G5R — initialization/materialization correction

## G5R-C01 — logical/physical agreement

**Role:** exact semantic correction evidence.

**Semantic owner:** post-G5C hardening design, G5R section. That design narrowly supersedes G3R's accepted `Prepared descriptor -> checked prepared initialized coverage` rule while retaining the rest of G3R's requirement/effect separation.

**Evidence cases:**

1. uninitialized content is rejected when a read requires prior initialization;
2. zero-initialized content is accepted only under a backend contract that establishes zero content;
3. a resource carrying prepared source bytes is not readable merely because source metadata exists;
4. canonical upload establishes the exact expected initialized coverage and physical bytes before dependent work;
5. convenience creation-with-data produces the same canonical work/effect authority as explicit resource plus upload;
6. retained source data remains distinguishable from completed physical-content continuity;
7. a generic shader write to previously uninitialized coverage still does not establish definite initialization;
8. structural review finds no hidden constructor/queue transfer authority.

No visual artifact is required. G6 remains blocked until this requirement is accepted.

# G6 — framework proof

G6 combines retained narrow correctness with representative integration, ergonomics, scale, and direct-backend cost evidence. Visual output supplements rather than replaces exact or structural proof.

## G6-C01 — known-pattern offscreen draw

**Role:** exact graphics conformance.

**Evidence:** no surface requirement; one graphics pipeline; known clear/draw behavior; exact attachment/load/store facts; selected-pixel readback assertions.

**Artifact:** one deterministic offscreen PNG after exact readback validation.

## G6-C02 — compute-generated indirect draw

**Role:** structural GPU-driven composition.

**Evidence:** compute generates draw arguments/source data; graphics consumes them; dependency arises from accepted G3 access rather than duplicate manual ordering; bounded render assertions establish the result.

**Artifact:** one offscreen image or equivalent exact bounded render artifact.

## G6-I01 — reaction diffusion

**Role:** representative persistent integration + visual showcase.

**Semantic owner:** post-G5C hardening design, G6 representative workload.

**Proof job:** fixed recorded dimensions and parameters, retained ping-pong state, parameter upload, repeated compute iterations per logical output frame, compute-to-render sharing without CPU state round-trip, offscreen output, and presentation through the same semantic work path.

The initial readable coverage of every ping-pong region used by generic shader reads must be established through accepted initialization effects before the first iteration. Later generic shader writes may mutate already-initialized coverage but do not expand initialized coverage by themselves.

At least two representative workload sizes or iteration envelopes must be recorded so the proof is not valid only at a toy configuration.

**Machine evidence:**

- initial ping-pong readable coverage established through canonical initialization effects;
- finite/bounded resource and iteration invariants;
- correct dependency and initialization facts;
- successful offscreen completion/readback;
- successful surface presentation using the same semantic work construction;
- canonical program identity and relevant capability facts in the manifest;
- direct-WGPU comparison under `G6-P01`.

**Required human-facing artifact:** a bounded GPU-generated PNG frame sequence plus manifest. GIF/video may be derived externally.

## G6-I02 — offscreen boids

**Role:** representative GPU-driven integration + visual showcase.

**Semantic owner:** post-G5C hardening design, G6 representative workload.

**Evidence:** fixed-step simulation/draw behavior, double-buffered agents, the retained spatial-neighborhood pipeline where it is part of the proof job, compute-to-graphics sharing, instanced drawing, one RunenGPU context, offscreen completion, and bounded state/resource invariants.

Persistent readable state follows the same G3R/G5R rule as `G6-I01`: initial readable coverage is established canonically; generic shader writes preserve already-established initialized coverage but do not create new definite coverage.

Do not promise exact cross-backend floating-point pixels where floating/atomic behavior does not justify it.

**Required human-facing artifact:** a bounded GPU-generated PNG frame sequence plus manifest under fixed proof-job inputs. Image acceptance uses explicitly bounded/tolerant evidence where bit-identical output is not portable.

## G6-S01 — synthetic graph scale

**Role:** scalability characterization.

**Evidence:** exercise at least three increasing graph-size tiers and more than one topology shape, including a dependency-heavy shape and a largely independent shape. The largest tier should be at least 100x the smallest tier unless a documented implementation/environment limit is reached first.

Record:

- graph node/resource/access counts;
- preparation and validation CPU time separately where measurable;
- allocation or memory high-water evidence where available;
- correctness-preserving completion;
- scaling trend across tiers.

This requirement supplies evidence for or against reusable/compiled graph-plan work; it does not pre-authorize that feature.

## G6-P01 — direct-WGPU cost comparison

**Role:** performance characterization.

**Evidence:** retain equivalent direct-WGPU baselines for:

- `G5-C01` prefix scan;
- `G6-C01` known-pattern offscreen draw;
- `G6-I01` reaction diffusion.

A fourth workload may be added when it answers a distinct cost question; it is not required merely for breadth.

The compared paths must use equivalent shader artifacts, resource sizes/formats, dispatch/draw counts, adapter/backend selection, correctness checks, and warm-up policy.

Record separately:

- CPU preparation/validation;
- allocations/bytes where measurable;
- command/execution preparation;
- submission overhead;
- staging/readback bytes;
- pipeline cold/warm behavior;
- GPU timestamps where supported.

No universal numeric threshold is implied without separately accepted controlled evidence. G6 acceptance must nevertheless record an explicit disposition for each material regression: **accepted with rationale**, **correction required**, or **architecture/split reconsideration**. An unexplained material regression cannot be waved through as “diagnostic only.”

## G6-E01 — public API ergonomics

**Role:** API acceptance.

**Semantic owner:** post-G5C hardening design and accepted public-experience architecture.

Use at least the reaction-diffusion implementation to inspect the ordinary public path.

**Evidence:** record:

- framework concepts a caller must understand;
- application-semantic code versus RunenGPU ceremony;
- graph-internal terminology exposed to the ordinary path;
- backend-realization or retirement knowledge required by the caller;
- nested builder/finish ceremony;
- fallible construction layers;
- how shader-interface facts are supplied under the accepted `G6-SH01` decision;
- whether simple and advanced paths lower to one canonical preparation authority.

An explicit shader interface is not itself a failure if that remains the accepted authority. The gate fails when ordinary use requires unrelated internal administration or parallel authorities.

## G6-SH01 — shader-interface authority conformance

**Role:** architecture/API conformance.

**Semantic owner:** shader-authoring design unless explicitly revised by an accepted G6 decision.

**Evidence:**

- exactly one accepted authority for compiler-known interface facts;
- deterministic agreement/rejection tests between canonical WGSL and that authority;
- precise mismatch diagnostics;
- no inferred G3 hazards, retained-state semantics, application meaning, or runtime-resource selection from shader reflection;
- no second pipeline/interface path for the ordinary API.

If G6 changes interface authority, the owning shader-authoring design must be revised in the same accepted decision; this matrix alone cannot change it.

## G6-A01 — capability aperture

**Role:** capability-admission evidence.

Every G6 feature/format addition must identify:

- the retained workload requiring it;
- semantic/correctness/performance need;
- portability effect;
- observed evidence that justifies permanent public vocabulary.

Backend enum breadth alone is not evidence. Reaction diffusion may compare storage-buffer and storage-texture representations only when that comparison answers a concrete question.

# G7B — loss, generations, reconstruction, continuity

## G7B-L01 — initialized coverage and opaque content continuity

**Role:** lifecycle correctness.

**Semantic owner:** post-G5C hardening design, G7B continuity model.

**Evidence scenarios:**

- completed work preserves previously established initialized coverage across retained submissions;
- exact operation-guaranteed effects expand initialized coverage only as allowed by G3R/G5R;
- successful generic shader writes do not expand previously uninitialized coverage;
- a successfully completed retained-state submission advances opaque content continuity for the affected state without claiming RunenGPU knows shader-produced values;
- failure provably before any relevant write preserves prior opaque content continuity;
- failed/lost work after a possible write makes affected opaque content continuity unknown/revoked;
- revoking opaque content continuity does not falsely erase initialized coverage that remains provably established, and initialized coverage never manufactures coherent content continuity;
- callers requiring coherent retained state after revocation must reconstruct/reset/reimport before continuing.

## G7B-L02 — current-state reconstruction matrix

**Role:** recovery evidence.

**Semantic owner:** post-G5C hardening design plus accepted G7 lifecycle/reconstruction ownership.

Retain explicit scenarios for:

- current contents reproducible from retained source/materialization facts;
- current contents reproducible through an accepted deterministic reconstruction/replay recipe;
- external owner reimport/reconstruction;
- GPU-mutated state whose original seed/source still exists but whose **latest contents are not reconstructable from that source alone**;
- non-reconstructable permanent current-state loss;
- stale-generation rejection;
- surface outdated/lost/out-of-memory;
- device loss/out-of-memory/backend failure classification.

Every successful reconstruction must establish new-generation initialized coverage through canonical materialization/replay/reimport effects; source metadata alone is insufficient.

No visual artifact is required.

# G8 — operational and explainable conformance

## G8-D01 — explainable diagnostics

**Role:** diagnostics conformance.

**Evidence:** representative typed diagnostics explain dependency cause, exact overlapping ranges/subresources, initialization reason, initialized-coverage continuity, opaque content-continuity generation/revocation, current-state reconstructability classification, capability/limit rejection, pressure rejection, cache incompatibility, and stale generation where applicable.

## G8-O01 — CPU/GPU observability

**Role:** operational/performance evidence.

**Evidence:** CPU framework timing and GPU timestamp evidence are reported separately. Semantic labels/provenance correlate with private backend debug markers/groups where supported.

## G8-B01 — actual browser WebGPU

**Role:** environment conformance.

**Evidence:** before browser runtime support is claimed, execute at least one retained headless compute proof and one retained offscreen graphics proof in an actual browser WebGPU environment. Include the surface path as well if browser surface presentation is part of the declared support profile.

Wasm compilation alone is insufficient. If runtime proof is absent, documentation must say compile-only/experimental.

## G8-N01 — authoring-toolchain independence

**Role:** architecture conformance.

**Evidence:** public RunenGPU runtime/package dependencies and APIs contain no filesystem/module/package/compiler-process/WESL/Slang/reload authority. Already-produced canonical WGSL executes without any authoring frontend present.

A separate Runenwerk frontend proof may consume RunenGPU, but frontend promotion is not a RunenGPU phase gate.

## G8-N02 — backend-neutrality audit

**Role:** architecture conformance.

Classify every public RunenGPU concept as genuinely generic, portable baseline, WGPU-derived but independently implementable, WGPU-specific/private, or future extension. A second backend is not required solely for this audit.

Any public concept that is merely renamed WGPU without independent semantic value is a review finding, not automatically accepted vocabulary.

## G8-R01 — reproducibility fact provider

**Role:** reproducibility integration evidence.

**Evidence:** RunenGPU exposes the typed facts external tooling needs to identify and correlate a run: normalized capability/limit facts, allowed adapter/backend facts, prepared-work inspection, workload/source/program identities, relevant generations, and stable process-local correlation/provenance facts where applicable.

RunenGPU does not own a persisted bundle schema, redaction policy, filesystem layout, artifact naming/checksum policy, or image/video encoding. A Runenwerk-owned bundle may consume these facts as separate integration evidence.

## G8-O02 — residual operational matrix

**Role:** operational conformance.

Retain shutdown, pending work, quota saturation, cache reuse/rejection, bounded RunenGPU diagnostic/readback/profiling state, loss/reconstruction, exactly-once outcomes, and zero raw-WGPU reach-through evidence required by the RunenGPU architecture and post-G5C hardening designs.

Persisted capture growth remains a Runenwerk/product concern unless a future accepted RunenGPU feature explicitly owns part of it.

# GX — standalone release/extraction

## GX-X01 — standalone package

**Role:** release conformance.

**Semantic owner:** RunenGPU architecture design, external cutover and definition of done.

**Evidence:** establish:

- one independently validated `runen-gpu` package;
- README/public rustdoc and runnable headless/offscreen/surface examples;
- declared edition, MSRV/toolchain, SemVer, Cargo feature, license, and provenance policy;
- supported native/web validation matrix with honest environment skips;
- benchmark baselines and reproducible direct-WGPU comparison procedure;
- at least one independent non-render downstream consumer using only public APIs;
- exact-revision Runenwerk integration through the external package;
- migration of all active consumers;
- deletion of original internal GPU authority and temporary seams;
- no mirror, forwarding package/namespace, compatibility facade, submodule/source include, moving-branch dependency, duplicate context/descriptor/execution authority, or raw backend reach-through.

## GX-X02 — retained proof portfolio

**Role:** release conformance.

The standalone package retains at minimum:

- `G5-C01` prefix scan;
- `G5-C02` Game of Life;
- `G5-O01` lifecycle evidence;
- `G5R-C01` initialization/materialization correction;
- `G6-C01` known offscreen draw;
- `G6-C02` indirect composition;
- `G6-I01` reaction diffusion + frame-sequence manifest;
- `G6-I02` boids + frame-sequence manifest;
- `G6-S01` graph-scale characterization;
- `G6-P01` direct-WGPU evidence and accepted regression dispositions;
- `G6-E01` public API ergonomics evidence;
- `G6-SH01` shader-interface authority conformance;
- applicable `G6-A01` evidence for public capabilities added during G6;
- `G7B-L01/L02` initialized/content continuity and reconstruction evidence;
- applicable G8 diagnostics, operational, browser-claim, neutrality, and reproducibility-fact evidence.

G5C renderer-cutover evidence remains Runenwerk integration history and is not falsely reclassified as a standalone package test. `GX-X01` independently proves the final external migration/deletion boundary.

Unsupported hardware/environment paths produce structured skip/environment evidence rather than false success.

# Phase-to-artifact summary

| Requirement | Primary role | Required machine evidence | Human-facing artifact |
| --- | --- | --- | --- |
| G5-C01 | exact compute | exact readback | none |
| G5-C02 | exact stateful compute | CPU oracle + exact state/sequence identity | optional host-derived frames |
| G5-C03 | exact texture/transfer | exact/documented bytes | optional PNG |
| G5-O01 | operational | structured lifecycle outcomes | none |
| G5C-I01/I02 | ownership/integration | frame path + bridge/no-reach-through guards | optional capture |
| G5R-C01 | semantic correction | initialization/materialization cases | none |
| G6-C01 | exact graphics | selected pixels + attachment facts | PNG |
| G6-C02 | structural composition | G3 dependency + bounded render checks | PNG/artifact |
| G6-I01 | representative integration | invariants + initialized-state evidence + manifest | **GPU PNG sequence** |
| G6-I02 | representative integration | invariants + tolerant evidence | **GPU PNG sequence** |
| G6-S01 | scalability | CPU/memory trend across tiers/topologies | none |
| G6-P01 | performance | equivalent-envelope measurements + disposition | comparison report |
| G6-E01 | API acceptance | public-path review | representative source/example |
| G6-SH01 | shader-interface conformance | one authority + agreement/rejection tests | diagnostics/example |
| G6-A01 | capability admission | workload-backed justification | none |
| G7B-L01/L02 | lifecycle/recovery | initialized/content continuity + reconstruction tests | optional report |
| G8-* | operational/architecture | diagnostics/profiling/browser/audits/facts | reports/captures as applicable |
| GX-* | release | package/support/downstream/retained proof | runnable documented examples |

# Normalized ownership stack

```text
roadmap
    phase order and dependencies only

phase / focused designs
    semantic contracts and boundaries

this matrix
    proof role, evidence, and retained artifact requirements only

GitHub issue
    one active bounded implementation slice

workspace RON spec, only when useful after activation
    subordinate implementation constraints

pull request + exact-head CI
    delivery/review/validation evidence

investigation reports
    rationale/history, not acceptance authority
```

Do not pre-create G5R/G6/G7B/G8/GX RON specs. Create one only after the corresponding bounded issue activates and structured handoff detail materially helps implementation.

# Interpretation of older material

1. Live/candidate status embedded in older RunenGPU designs is historical context; GitHub owns current state.
2. The roadmap owns the post-G5C sequence; older future-tail text does not override it.
3. The proof-workload and ergonomics investigations remain rationale, not parallel acceptance authorities.
4. The shader-authoring design remains the semantic owner for interface authority and frontend/toolchain policy until explicitly revised.
5. G5R narrowly supersedes G3R's prepared-descriptor initialization rule; all other G3R requirement/effect distinctions remain unless explicitly revised.
6. Historical RON lifecycle/base/issue fields are snapshots and never determine current activation/completion.

This interpretation is a compatibility rule for existing documentation, not permission to add new stale live-state prose to canonical designs.
