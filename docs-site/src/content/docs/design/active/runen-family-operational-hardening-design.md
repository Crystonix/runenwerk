---
title: Runen Family Operational Hardening Design
description: Cross-framework pressure, progress, cache, recovery, compatibility, reproducibility, and performance contracts for RunenGPU, RunenRender, and Runenwerk integration.
status: active
owner: workspace
layer: architecture/operations
canonical: true
last_reviewed: 2026-07-27
related_docs:
  - ../../architecture/repository-family-architecture.md
  - ../../adr/accepted/0015-separate-gpu-execution-from-rendering.md
  - ./runengpu-architecture-design.md
  - ./runengpu-g3-access-work-graph-design.md
  - ../accepted/runenrender-decomposition-design.md
  - ./runenrender-internal-decomposition-execution-plan.md
  - ../../reports/investigations/runengpu-industry-comparison.md
  - ../../reports/investigations/runengpu-runenrender-application-domain-fit.md
  - ../../reports/investigations/runengpu-proof-workload-strategy.md
  - ../../workspace/planning/roadmap.md
---

# Runen Family Operational Hardening Design

## Status and scope

This design binds cross-cutting operational requirements discovered after accepted
RunenGPU G3 planning. It does not reopen or alter accepted G3 resource-access,
initialization, hazard, causality, operation, or graph-preparation semantics.

The accepted baseline is Runenwerk merge
`5c82cc54d5ac51aeb2fd8e3da916ed895f8058e8` from PR `#175`.

This design does not authorize Rust implementation. Existing G and R phases remain
the only implementation sequence.

## Current-source findings

Current source contains useful behavior but no single reusable operational authority.
The timing path in
`engine/src/plugins/render/renderer/render_flow/gpu_timing.rs` currently:

```text
creates a timestamp query set
creates QUERY_RESOLVE and MAP_READ buffers
writes pass timestamps
resolves the query set
copies the result to readback
calls map_async
calls device.poll(wait_indefinitely)
waits on a channel
publishes timing evidence
```

This proves the need for typed query resolution accepted by G3. It also exposes
later-phase concerns that G3 must not absorb:

- progress is driven synchronously by the current caller;
- the wait is unbounded;
- callback delivery is tied to polling/backend behavior;
- the path owns no shared pending-readback quota;
- cancellation and shutdown behavior are local rather than framework-wide;
- WebGPU polling semantics differ from native WGPU-core backends.

WGPU's current contracts further establish:

- `Device::poll(PollType::Wait)` can block and invokes mapping callbacks on native
  backends;
- WebGPU devices are polled by the browser event loop and explicit polling is a
  no-op;
- queue-empty observations are inherently racy when other threads can submit;
- pipeline-cache data is conditional on WGPU version, adapter identity, driver,
  and backend acceptance;
- device loss and out-of-memory remain backend/runtime outcomes, not graph facts.

RunenGPU must normalize these facts without promising portability the backend cannot
provide.

## Family-wide operational doctrines

### Accepted-work integrity

Accepted work is never silently discarded.

Every bounded producer/consumer boundary returns a structured result:

```text
accepted
accepted_after_wait
rejected_unsupported
rejected_pressure
cancelled_before_submission
cancelled_after_submission
failed_backend
failed_device_lost
failed_shutdown
```

Exact public names remain phase decisions. Logging is not acceptance authority.

### Bounded pressure

Every queue, staging arena, readback pool, retained cache, history set, capture
buffer, and pending-completion collection is bounded or exposes an explicit growth
policy.

Pressure handling must be one of:

```text
reject with structured evidence
wait with an explicit bound
shed only discardable derived work
reduce quality through caller-owned policy
```

Frameworks do not silently drop authoritative work, source state, completion
notifications, or requested artifacts.

### Derived-cache doctrine

Derived caches are:

- non-authoritative;
- discardable and reconstructable;
- keyed by all facts required for correctness;
- source-generation-bound;
- validated before reuse;
- versioned when persisted;
- rejected rather than guessed when compatibility facts are incomplete.

A cache hit may change cost, never semantics.

### Compatibility manifest

Runenwerk owns the tested cross-framework compatibility manifest because Runenwerk
selects exact framework revisions and integrates them.

The manifest records namespaced facts such as:

```text
runengpu API/contract revision
runenrender API/contract revision
runensdf revision
runenecs revision
runenui revision
Runenwerk adapter revision
WGPU version and enabled backend family
artifact/capture schema revisions
```

This is not a shared RunenCore package and does not create dependency cycles.

### Recovery ownership

Frameworks report lifecycle and reconstruction facts. Runenwerk owns product recovery
policy.

```text
framework
    classify outcome
    invalidate generations
    report reconstructability and affected values

Runenwerk
    pause/retry/recreate/degrade/exit
    rebuild adapters and product state
    present diagnostics to the user
```

RunenGPU does not decide application restart policy. RunenRender does not decide
world reload policy.

### Reproducibility bundle

Runenwerk may assemble a versioned, namespaced reproducibility bundle containing:

- compatibility manifest;
- normalized capability and limit facts;
- device/backend/driver identity where disclosure is permitted;
- prepared-work graph facts and diagnostics;
- RunenRender scene/view/quality generations;
- seeds, fixed-time inputs, and product configuration;
- source and artifact provenance;
- privacy/redaction policy;
- stable artifact schema versions.

Runtime object handles, raw backend pointers, memory addresses, and unversioned debug
strings are never persisted as authority.

## RunenGPU operational requirements

### G4 — admission, portability, and caches

G4 must bind:

- a portability class for each admitted context;
- normalized capabilities plus explicit backend-specific limitations;
- context and device generations;
- all cache compatibility facts;
- a contained internal WGPU realization boundary;
- callback/reentrancy invariants used later by progress delivery;
- structured unsupported and degraded admission outcomes.

Suggested portability classes:

```text
portable_baseline
portable_with_declared_extensions
backend_specialized_internal
unsupported
```

`backend_specialized_internal` never implies a stable public native-handle escape.

Pipeline-cache compatibility must include at least:

```text
RunenGPU cache schema
WGPU version/cache key
backend family
adapter identity
relevant driver identity/version
shader/program/interface generation
pipeline descriptor hash
enabled features and limits
```

An incompatible cache is ignored with structured evidence. It is not a fatal
correctness failure.

### G5 — progress, pressure, completion, and shutdown

G5 must own one explicit progress model for native and web environments.

It must specify:

- who drives native device polling;
- which thread or executor may drive it;
- whether progress is automatic, caller-driven, or runtime-driven;
- callback execution and reentrancy rules;
- prohibition on invoking consumer callbacks while holding internal registry,
  queue, staging, or completion locks;
- bounded pending submissions, uploads, map operations, and readbacks;
- completion delivery exactly once;
- cancellation meaning before and after backend submission;
- shutdown behavior with pending work;
- time-bounded waits where the product requires responsiveness.

A submission accepted by RunenGPU receives a terminal outcome even when shutdown or
loss occurs.

### G6 — narrow overhead and capture proof

G6 adds offscreen graphics and must compare the framework path with a narrow direct
WGPU baseline for the same workload.

Required dimensions include:

- preparation CPU time;
- graph-validation CPU time;
- allocation counts and bytes;
- pipeline cold/warm time;
- command-recording time;
- upload/readback staging bytes;
- pending-work high-water marks;
- GPU timestamp evidence where supported;
- final artifact equality/tolerance.

Performance evidence is diagnostic. It does not weaken correctness checks.

### G7 — device loss and generations

G7 must classify at least:

```text
surface_outdated
surface_lost
surface_out_of_memory
device_lost
device_out_of_memory
backend_failure
```

On device replacement:

- the context generation changes;
- all backend realizations from the old generation become invalid;
- logical source-backed resources can be reported as reconstructable;
- imported resources require the external owner to reconstruct or reimport;
- non-reconstructable resources are reported explicitly;
- retained handles do not silently bind to a new device generation.

### G8 — operational conformance

G8 must prove:

- clean shutdown with no work;
- shutdown with pending submissions and readbacks;
- no lost completion notifications;
- quota saturation outcomes;
- device-loss invalidation and reconstruction reporting;
- reproducibility-bundle generation;
- cache rejection/reuse behavior;
- no raw WGPU reach-through outside the backend boundary;
- bounded diagnostic and capture growth;
- direct-WGPU comparison evidence for representative narrow workloads.

## RunenRender operational requirements

### Provider maturity

Provider families are planning categories, not a promise to implement every family.

```text
near-term proof
    Procedural
    Analytic
    Solid/field adapter sufficient for the first SDF terrain proof
    Overlay

research candidate
    Volume
    Population
    RegionalSummary
    Liquid

fully deferred pending accepted consumer evidence
    Fiber
    broad hardware-accelerated provider variants
    universal provider unification
```

A provider exposes narrow capabilities. No universal provider trait may require every
provider to implement surface intersection, interval traversal, transmittance,
raster visibility, velocity, streaming, and hardware acceleration.

Capability families may be separate interfaces or validated capability records:

```text
surface_query
visibility_query
interval_query
transmittance_query
raster_visibility
material_attributes
motion
refinement
streaming
```

### R1/R2 — incremental prepared scenes

Prepared-scene and contribution authority must support deterministic:

```text
insert
replace
remove
retire_producer
```

Each change identifies affected generations and changed regions. Unrelated providers,
materials, views, targets, and overlays must not require full-scene reconstruction.

Required evidence includes:

- identical final prepared scene from equivalent full and incremental construction;
- deterministic replacement/removal behavior;
- bounded update cost proportional to changed contributions where feasible;
- explicit fallback to full rebuild when an adapter cannot provide narrower facts.

### R3 — provider proof and anti-universal-trait rule

R3 accepts only provider capabilities required by current proofs. New provider
families require:

- a concrete consumer;
- owned numerical/semantic contracts;
- a representative proof;
- explicit non-goals;
- no forced expansion of unrelated provider interfaces.

### R6 — cache and history invalidation

Every renderer-derived cache/history entry records:

- scene/view/provider/material/quality generations;
- relevant changed regions;
- device/context generation where GPU-realized;
- algorithm/schema revision;
- reconstruction source or explicit non-reconstructability.

History and caches are invalidated narrowly when correctness facts permit and fully
when they do not. Stale cache reuse is never a quality degradation mechanism.

### R8 — renderer operational conformance

R8 must characterize:

- full-scene versus incremental preparation cost;
- provider query counts and divergence evidence;
- cache hit/miss and invalidation behavior;
- current-frame and history-dependent quality paths;
- CPU/GPU memory high-water marks;
- cold/warm pipeline and shader cost inherited through RunenGPU;
- artifact/capture reproducibility;
- direct narrow alternatives where a simpler existing renderer could satisfy the
  same proof.

## Application-domain rule

Application-domain reports identify where the architecture is useful. They do not
move domain systems into the frameworks.

Examples:

- CAD owns constraints, topology, manufacturing rules, and document formats;
- medical tools own clinical data governance and regulated validation;
- robotics owns sensor models, calibration, datasets, and simulation policy;
- geospatial systems own coordinate reference systems and data streaming;
- VFX tools own asset, timeline, farm, and color-management workflows.

RunenGPU and RunenRender provide reusable execution and image-formation contracts,
not complete vertical products.

## Phase mapping

```text
G3  accepted graph facts only; unchanged
G4  portability, backend containment, cache compatibility
G5  progress, pressure, completion, cancellation, pending-work shutdown
G6  offscreen capture and direct-WGPU narrow comparisons
G7  device generations, loss classification, reconstruction facts
G8  recovery proof, reproducibility bundle, performance and interop audit

R1/R2 incremental prepared scenes and contributions
R3    provider maturity and narrow capabilities
R6    derived-cache and history invalidation
R8    renderer performance, capture, diagnostics, and anti-cheating proof
```

No new G or R phase is created.

## Strategic reevaluation gates

The RunenGPU/RunenRender split must be reconsidered if accepted evidence shows any of
the following:

- RunenGPU has no independent non-render consumer;
- public callers require raw WGPU for ordinary work;
- the framework path adds material overhead without reusable correctness value;
- a simpler WGPU/rend3/Filament-style renderer satisfies all accepted RunenRender
  proofs with less ownership;
- provider abstractions force unrelated implementations into one interface;
- incremental scene preparation cannot avoid systematic full rebuilds;
- backend-neutral contracts are repeatedly broken by first-backend leakage;
- operational pressure or recovery cannot be expressed without product-specific
  policy inside the frameworks.

Reevaluation is an architecture decision, not an implementation shortcut.

## Non-goals

This design does not authorize:

- G3 API changes;
- implementation before the owning phase;
- a RunenCore/shared diagnostics package;
- a public raw WGPU escape hatch;
- automatic multi-queue scheduling;
- aggressive aliasing or pass fusion;
- graph visualization UI;
- a universal shader IR;
- domain applications inside RunenGPU or RunenRender;
- persisted runtime handles or unversioned capture formats.
