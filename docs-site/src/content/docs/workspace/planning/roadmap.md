---
title: Roadmap
description: Durable high-level sequence and dependency direction for Runenwerk and its peer frameworks.
status: active
owner: workspace
layer: workspace
canonical: true
last_reviewed: 2026-08-24
related_docs:
  - ../engineering-workflow.md
  - ../../architecture/runenwerk-platform-architecture.md
  - ../../architecture/repository-family-architecture.md
  - ../../adr/accepted/0014-repository-family-extraction-boundaries.md
  - ../../adr/accepted/0015-separate-gpu-execution-from-rendering.md
  - ../../design/active/runengpu-architecture-design.md
  - ../../design/active/runengpu-post-g5c-hardening-design.md
  - ../../design/active/runengpu-phase-requirements-proof-matrix.md
  - ../../design/active/runenrender-decomposition-design.md
---

# Roadmap

This page owns **durable sequence and dependency direction only**.

GitHub issues and the Engineering Portfolio own live work, priority, activation, owners,
blockers, and current status. Pull requests own delivery, reviewed feature heads,
validation, and merge evidence. Accepted ADRs and designs own durable architecture.

The roadmap therefore does not record exact accepted revisions, active branches, current
PR/check state, completion tables, or which implementation issue is active today.

## Repository family direction

```text
RunenSDF --------+
RunenECS --------+--> Runenwerk adapters and applications
RunenUI ---------+
                       |
                       +--> RunenRender --> RunenGPU
                       +--> non-render RunenGPU consumers
```

Runenwerk remains the integration and product repository. Framework repositories do not
depend on Runenwerk.

Accepted GPU/render dependency:

```text
RunenRender -> RunenGPU
```

Target layering:

```text
Runenwerk host, source, and product policy
    -> RunenRender semantic image planning
        -> RunenGPU generic resources, programs, work, realization, and execution
            -> private WGPU backend
```

Non-render consumers may lower directly into RunenGPU without depending on RunenRender.

## RunenSDF extraction direction

RunenSDF owns standalone field mathematics at its public boundary. Runenwerk retains
product and domain integration above that boundary.

Runenwerk now contains no `domain/sdf` package, source mirror, forwarding namespace, submodule, source include, or unused external dependency.

## RunenGPU sequence

The durable RunenGPU program is ordered as:

```text
G1A logical resource identity
    -> G2 capabilities and logical resources
        -> G3 checked access and work graph
            -> G4 backend realization and ownership cutover
                -> G3R initialization-semantics correction
                    -> G4R backend-baseline refresh
                        -> G5A/B executable work and surface-independent execution lifecycle
                            -> G7A minimal durable surface foundation
                                -> G5C final renderer execution cutover
                                    -> G5R initialization-materialization correction
                                        -> G6 representative breadth, scale, ergonomics, offscreen proof, and cost characterization
                                            -> G7B complete surface/device loss, generations, reconstruction, and retained-state continuity
                                                -> G8 operational, diagnostics, browser, backend-neutrality, extension, and no-reach-through conformance
                                                    -> GX standalone release and transfer to dornglut/runen-gpu (complete; accepted revision `77c7c8d5ad6922b6f46c6b25e31b1a224c1314a4`)
```

`G3R` and `G4R` are corrective predecessor phases discovered by G5 owner review. They are
ordered deliberately: backend-independent semantic correctness is repaired first, then the
corrected G1-G4 authority is re-proven against the refreshed private WGPU/Naga baseline.

`G7A` is intentionally narrower than complete G7. It establishes only the durable generic
surface identity/generation/capability/acquisition/presentation foundation required so the final
G5C renderer cutover does not create a disposable pre-G7 surface execution architecture. Full
loss/reconstruction policy remains G7B.

`G5R` is a bounded correctness gate between final execution cutover and representative proof. It
requires graph initialization truth and physical content materialization to agree before G6 may
characterize performance, ergonomics, or application breadth. The focused post-G5C hardening
design owns the detailed G5R/G6/G7B/G8/GX semantic gates; the RunenGPU phase requirements and
proof matrix owns the corresponding retained proof roles and observable evidence/artifacts.

G4 is itself ordered:

```text
G4A context admission
    -> G4B program/interface/pipeline contracts
        -> G4C1 private resource realization
            -> G4C2 private program/layout/binding realization
                -> G4C3 private pipeline realization and final G4 cutover
```

Each phase consumes accepted predecessor authority, not an unmerged implementation
branch. The owning GitHub issue determines whether a phase is proposed, active, blocked,
or complete at any particular time.

Detailed semantic contracts belong to accepted RunenGPU designs. Proof roles, observable evidence,
and retained artifact requirements belong to the RunenGPU proof matrix. A workspace RON spec is
subordinate implementation-handoff detail created only when an activated bounded slice benefits
from it; this roadmap does not duplicate any of those requirements.

## RunenRender sequence

RunenRender remains downstream of RunenGPU and preserves the accepted semantic spine:

```text
RenderSceneStore
    -> RenderSceneCommit(RenderSceneSnapshot + RenderChangeSet)

RenderSceneSnapshot + RenderRequest + RenderInputSet
    -> RenderMethod
        -> RenderPlan
            -> AdmittedRenderPlan
                -> RenderWorkSet
                    -> RunenGPU
```

Its durable sequence is:

```text
R1  scene revisions, identities, relationships, and cheap snapshots
R2  space, time, typed dynamic inputs, and availability
R3  representation offers, sampling footprints, protocols, and narrow results
R4  views, outputs, measurement, materials, methods, and planning
R5  founding analytic and field/SDF method through RunenGPU
R6  derived state, residency, sessions, and invalidation
R7  multiview, multi-output, surface, readback, and merge integration
R8  scalability, extension, operational, and extraction conformance
RX  standalone RunenRender transfer and clean cutover
```

The canonical RunenRender architecture owns the detailed semantics and conformance model.
The owning issues determine activation and current status.

## Other repository-family programs

RunenSDF remains the accepted standalone field-mathematics authority. RunenECS,
RunenSpatial, and RunenUI continue through separately owned programs and
may proceed in parallel only when repository, branch, workspace, files, authority, and
dependencies do not conflict.

A cross-family dependency belongs here only when it is durable architecture or sequence,
not merely because one current implementation happens to be waiting for another.

## Roadmap rules

1. Record only durable phase ordering, dependency direction, extraction direction, and
   cross-family sequencing constraints.
2. Do not copy live issue state, assignees, blockers, branch heads, PR state, check runs,
   exact accepted revisions, or completion tables into this page.
3. Do not use the roadmap to authorize implementation. An owning GitHub issue activates
   work and a reviewed pull request delivers it.
4. Detailed architecture and public contracts belong in accepted ADRs/designs, not in
   roadmap prose.
5. Historical chronology belongs in reports, closeouts, pull requests, and Git history.
6. Change this roadmap only when durable sequence or dependency truth changes.
