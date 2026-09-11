---
title: Runtime Product Job Executor Roadmap
description: Implementation sequence for turning the accepted execution fabric and product jobs design into engine runtime execution.
status: active
owner: engine
layer: engine-runtime
canonical: true
last_reviewed: 2026-05-14
related_designs:
  - ../../design/accepted/execution-fabric-and-product-jobs-design.md
  - ../../design/accepted/sdf-first-field-world-platform-design.md
  - ../../design/accepted/field-product-contracts-diagnostics-and-residency-design.md
related_roadmaps:
  - ../../workspace/sdf-first-execution-roadmap.md
related_reports:
  - ../../reports/closeouts/runtime-product-job-rpj4-rpj6/closeout.md
  - ../../reports/closeouts/runtime-product-job-rpj7a-cache-policy/closeout.md
---

# Runtime Product Job Executor Roadmap

## Goal

Turn the accepted execution fabric and product jobs design into a reusable
engine runtime subsystem without making scheduler, ECS, product, or drawing
domains own runtime execution.

The long-term program includes runtime product jobs, persistent product caches
and package sidecars, work stealing, GPU jobs, full ECS parallel execution, and
cross-process or distributed jobs. Deferred below means deferred from the first
implementation batch, not excluded from the program.

## Current Baseline

- `domain/product` owns `ProductJobDescriptor`, publication outcomes, query
  snapshots, render selection, diagnostics, and ratification.
- standalone `runen-ecs` owns ECS scheduling and deferred-command visibility;
  Runenwerk Engine owns product-job sequencing and publication barriers.
- ECS-owned command queues are not the product job executor.
- `engine/src/runtime/jobs` owns the serial and bounded worker runtime job
  executor, typed handles, generations, stale suppression, panic capture,
  queue backpressure diagnostics, and clean shutdown.
- Runtime product publication helpers and Draw committed tile jobs publish
  through existing product publication and query snapshot barriers.
- Draw preview-quality CPU ink now splits immediate UI feedback from visual
  tile catch-up: `StrokePrimitive` is screen-space feedback, while preview tile
  products are asynchronous catch-up output.
- RPJ1-RPJ7A are implemented for the local runtime substrate. Draw
  responsiveness, backend-neutral cache identity, preview/final tile identity
  separation, fixed worker-pool execution, work-stealing execution, and runtime
  job inspection diagnostics exist. Engine now owns metadata-only runtime cache
  decisions, while Draw owns the in-memory tile payload cache proof.
- Persistent disk caches/package sidecars, GPU jobs, ECS parallel waves, and
  cross-process/distributed jobs remain later phases.

## Phase RPJ0 - Roadmap And Status Alignment

Owner: documentation under `docs-site/src/content/docs/engine/roadmaps`.

Status: active.

Requirements:

- link this roadmap from workspace and execution-fabric indexes;
- keep the accepted design as architecture, not a duplicated phase list;
- make clear that later executor families are planned program phases.

Validation:

```text
python tools/docs/validate_docs.py
```

## Phase RPJ1 - Serial Runtime Product Job Executor

Owner: `engine/src/runtime/jobs`.

Status: implemented.

Requirements:

- provide `RuntimeJobExecutorResource`, `RuntimeJob`, typed handles,
  generations, statuses, completions, submission errors, and diagnostics;
- default to serial execution so product-job behavior stays deterministic and
  easy to test;
- install the executor resource from app bootstrap;
- keep product visibility behind existing product publication and query snapshot
  barriers.

Validation:

```text
cargo test -p engine runtime_job
cargo test -p product
cargo test -p scheduler
cargo test -p ecs runtime_phase3
```

## Phase RPJ2 - Worker-Backed CPU Jobs

Owner: `engine/src/runtime/jobs`.

Status: implemented.

Requirements:

- add a bounded worker backend behind the same executor API;
- use owned input snapshots only;
- reject live `World`, renderer handles, windows, UI state, and backend objects
  from worker jobs by API shape and review;
- support queue capacity, worker count, per-frame completion drain budget,
  stale generation suppression, panic capture, and clean shutdown.

Validation:

```text
cargo test -p engine runtime_job
```

## Phase RPJ3 - Product Publication Integration

Owner: `engine/src/runtime/jobs/product.rs`.

Status: implemented.

Requirements:

- turn completed runtime product jobs into staged product publications and query
  snapshots;
- preserve deterministic staging order by generation, stage sequence, and
  `ProductJobId`;
- do not add a global product registry.

Validation:

```text
cargo test -p engine runtime_job
cargo test -p product
```

## Phase RPJ4 - Draw Tile Proof

Owner: `apps/runenwerk_draw`.

Status: implemented for the current Draw responsiveness proof.

Requirements:

- move CPU preview and committed ink tile formation off the input hot path;
- keep immediate `StrokePrimitive` feedback separate from preview tile
  catch-up;
- submit owned drawing document, stroke, and tile snapshots as runtime jobs;
- drain completions on the main thread and stage outputs through existing
  product/query barriers.

Implementation Notes:

- committed tile jobs form authoritative preview-quality CPU tiles behind the
  product publication barrier;
- preview tile jobs form visual catch-up products behind the runtime job
  executor and update app-owned preview products on the main thread;
- Draw installs a bounded worker executor by default while the engine default
  remains serial;
- preview tiles are not authoritative drawing state, and `StrokePrimitive` is
  not a product cache entry.

Validation:

```text
cargo test -p runenwerk_draw --test app_shell
```

Closeout:

- [Runtime Product Job RPJ4-RPJ6 Closeout](../../reports/closeouts/runtime-product-job-rpj4-rpj6/closeout.md)

## Phase RPJ5 - Cache Identity Before Persistence

Owner: `domain/product`, `domain/drawing`, and owning apps.

Status: implemented for backend-neutral cache identity; persistent caches and
package sidecars remain deferred.

Requirements:

- expose a typed product cache identity derived from product descriptors and
  lineage;
- make Draw preview/final quality participate in deterministic tile identity,
  product scale band, descriptor generation, cache key, and render selection;
- add persistent cache only after product job identity, descriptor generation,
  lineage, and stale-result behavior are stable;
- keep generic cache primitives backend-neutral in engine/runtime;
- keep package and sidecar policy in owning apps or domains;
- include product id, kind, quality or scale band, descriptor generation, source
  revision, formation version, and producer lineage in cache identity.

Validation:

```text
cargo test -p product
cargo test -p drawing --test ink_tile
```

## Phase RPJ6 - Work-Stealing Executor

Owner: `engine/src/runtime/jobs`.

Status: implemented for local runtime jobs.

Requirements:

- add work stealing as an executor backend, not a new product-job API;
- preserve serial and fixed-worker modes for debugging and deterministic tests;
- expose runtime job inspection diagnostics for executor mode, worker
  configuration, pending jobs, latest generations, recent issues, and drain
  activity;
- prove serial, fixed-worker, and work-stealing execution publish equivalent
  authoritative products.

Validation:

```text
cargo test -p engine runtime_job
```

## Phase RPJ7A - Cache Policy And Last-Good Semantics

Owner: `domain/product`, `engine/src/runtime/product_cache.rs`, and owning apps.

Status: implemented for backend-neutral runtime cache policy and Draw
in-memory cache proof; persistent storage remains deferred.

Requirements:

- derive typed `ProductCacheKey` values from `ProductCacheIdentity`;
- expose backend-neutral cache decisions for hit, miss, stale, rejected,
  write-failed, and preserved last-good states;
- keep engine runtime cache state metadata-only, with no Draw tile bytes,
  package paths, or renderer handles;
- let Draw check an app-owned source-key-to-payload cache before submitting a
  committed tile job;
- update runtime cache metadata only after product publication accepts formed
  descriptors;
- preserve last-good committed products when cache lookup, formation,
  publication, or query snapshot handling fails.

Validation:

```text
cargo test -p product
cargo test -p engine runtime_product_cache
cargo test -p runenwerk_draw --test app_shell committed_ink_cache_hit
```

Closeout:

- [Runtime Product Job RPJ7A Cache Policy Closeout](../../reports/closeouts/runtime-product-job-rpj7a-cache-policy/closeout.md)

## Phase RPJ7B - Persistent Product Caches And Package Sidecars

Status: deferred.

Requirements:

- add disk persistence only after cache acceptance/rejection, last-good
  behavior, and app-owned payload lookup are stable;
- keep Draw sidecar manifests, pruning, package migration, and artifact IO
  app/package-owned;
- reject corrupt, stale, incompatible, or untrusted entries with diagnostics
  and without clearing last-good visible products.

## Phase RPJ8 - GPU Job Execution

Status: deferred.

Requirements:

- add GPU jobs only after CPU product jobs and render product selection are
  stable;
- require explicit render-prepare, render-submit, GPU-adjacent prepare, or
  readback affinity;
- publish GPU output through the same product/query barriers;
- never bypass CPU reference or fallback policy where an owning domain requires
  it.

## Phase RPJ9 - Full ECS Parallel Execution

Status: deferred.

Requirements:

- parallelize ECS waves only after product jobs no longer depend on this step;
- define `Send`/`Sync` policy for system params, resources, components, command
  buffers, and deferred merge order;
- preserve serial fallback and prove serial/parallel equivalence before default
  enablement.

## Phase RPJ10 - Cross-Process And Distributed Jobs

Status: deferred.

Requirements:

- require serialized job inputs and outputs, version compatibility, process
  lifecycle, trust policy, artifact transfer, timeout/retry policy, and
  preserved last-good products;
- do not start this phase before local job execution, cache identity, and
  failure diagnostics are stable.

## Invariants

- `domain/product` owns product descriptors and ratification, not execution.
- standalone `runen-ecs` owns ECS plans and deferred-command boundaries, not
  worker threads or product publication.
- `engine/src/runtime/jobs` owns execution, queues, workers, completions, and
  runtime diagnostics.
- Product visibility changes only through publication/query barriers.
- Serial fallback remains permanent.
- Failed, stale, panicked, rejected, or timed-out jobs are diagnosable and must
  not silently clear last-good products.

## Stop Conditions

- Stop if a phase requires runtime concepts inside `domain/product`, the
  standalone ECS framework, or another pure domain crate.
- Stop if a worker job needs live `World` access.
- Stop if GPU work requires renderer-private app semantics.
- Stop if cache work needs package-format decisions before cache identity is
  stable.
- Stop if full ECS parallelism becomes a prerequisite for product jobs.
- Stop if distributed execution requires an unaccepted trust, security, or
  artifact format decision.
