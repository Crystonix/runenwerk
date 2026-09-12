---
title: Runenwerk-Owned Product and Query Publication Phases
description: Accepted lifecycle semantics for placing product and query-snapshot publication independently of RunenECS execution grouping.
status: accepted
owner: engine
layer: architecture
canonical: true
last_reviewed: 2026-09-12
related_adrs:
  - ./0018-semantic-federation-and-physical-realization.md
  - ./0019-batteries-included-application-composition.md
related_docs:
  - ../../design/accepted/execution-fabric-and-product-jobs-design.md
  - ../../engine/reference/architecture.md
---

# ADR 0022: Runenwerk-Owned Product and Query Publication Phases

## Context

Runenwerk's accepted execution-fabric design already assigns application lifecycle,
product publication, query-snapshot publication, and application barriers to Runenwerk.
RunenECS owns ECS schedule semantics and deferred ECS structural mutation.

The current implementation does not preserve that boundary cleanly. Engine publication
handlers are dispatched from every RunenECS deferred-apply callback, so the number and
identity of Runenwerk publication events currently inherit the ECS executor's physical
topological grouping.

That coupling is not only an ownership problem. Exact maintained consumers require
publication at different semantic points:

- Runenwerk Draw must accept committed products and then publish drawing query snapshots
  before its frame-submission system consumes the newly visible product state;
- Runenwerk Editor can queue product work through both primary input dispatch and
  secondary-window target input;
- Editor viewport query snapshots derive from `ViewportArtifactObservationResource`,
  which is refreshed by `ViewportPresentationSync` only after Editor frame submission;
- procgen overlay and later render preparation consume state that must already have the
  appropriate product/query publication applied.

A single schedule-completion hook would therefore publish too late for Draw. A universal
paired Product-then-Query hook at one position would publish Editor viewport snapshots
too early. Preserving one callback per ECS stage/frontier would keep physical execution
layout as hidden application authority.

## Decision

### 1. Product and query publication are independent Runenwerk lifecycle classes

Runenwerk defines two distinct publication phase classes:

```text
ProductPublication
QuerySnapshotPublication
```

They are Runenwerk application/runtime lifecycle semantics. They are not aliases for:

- RunenECS deferred-publication frontiers;
- RunenECS physical execution stages;
- worker cohorts or task batches;
- application frame boundaries unless the owning application explicitly places them
  there.

The two classes do not imply one universal cadence or one universal paired location.
An application/plugin places each class where its actual producer/consumer invariants
require it.

### 2. Publication placement is ordinary semantic schedule ordering

The normalized placement rule is:

```text
producer systems / workflow mutations
    -> owning publication phase
    -> consumers that require accepted publication state
```

Publication phases participate in the same explicit semantic ordering model as other
Runenwerk systems. Only proven producer/consumer dependencies receive ordering edges.
Historical stage adjacency is not a reason to add order.

If an ordered predecessor used RunenECS deferred structural mutation, the RunenECS
schedule contract owns the corresponding deferred-visibility guarantee before the
publication system executes. Runenwerk must not subscribe to every ECS publication
frontier to manufacture an application lifecycle clock.

Changing worker count, physical stage shape, cohort width, or the number of ECS semantic
publication frontiers must not change Runenwerk publication occurrence cardinality for
the same application lifecycle path.

A publication phase is installed only where a maintained semantic producer/consumer
boundary requires it. The predecessor adapter's ability to emit callbacks from Startup,
PreUpdate, FixedUpdate, RenderPrepare, RenderSubmit, FrameEnd, or any other schedule is
not evidence that those schedules require a Runenwerk publication phase. Exact-current
maintained Draw/Editor consumers do not require a separate Startup/PreUpdate publication
occurrence before the explicit Update phases defined below, so no such compatibility
phase is authorized merely to preserve callback cadence.

### 3. Preserve a narrow publication-handler registry

The existing distinction between product handlers and query-snapshot handlers is a
useful narrow Runenwerk boundary. Plugin-owned handlers perform domain/application work
that is intentionally richer than one generic queue flush, including ratification,
formation, catalog updates, cache updates, journals, and app-owned projection changes.

Implementation may retain or refine these two registries, including deterministic
registration order. What changes is their dispatch authority:

- product handlers are dispatched only by an explicitly scheduled
  `ProductPublication` occurrence;
- query-snapshot handlers are dispatched only by an explicitly scheduled
  `QuerySnapshotPublication` occurrence.

Do not replace this with a universal event bus, generic hook registry, second scheduler,
or product-agnostic lifecycle framework.

A publication dispatcher may use the supported exclusive-World system capability because
the current handler contract coordinates heterogeneous app-owned resources. That makes
the dispatcher invoker-thread-only under the accepted RunenECS mobility model. Thread
affinity is an execution capability fact, not a new semantic ordering edge.

### 4. Normalize publication occurrence identity

Each explicit publication phase entry creates a Runenwerk-owned **publication
occurrence** with these semantic fields:

```text
class: ProductPublication | QuerySnapshotPublication
sequence: monotonically increasing within that class for one runtime instance
schedule: Runenwerk schedule/lifecycle label
```

Each class owns an independent sequence. Product sequence `N` and query sequence `N`
have no equality, pairing, or cross-class correlation meaning.

The sequence advances when an installed publication phase is entered, before its
handlers are dispatched. It therefore also identifies an attempted occurrence whose
handler later returns an error or panics.

Publication occurrence identity is diagnostic/provenance state only. It is not:

- semantic precedence;
- RunenECS frontier/stage/cohort identity;
- worker identity;
- persistence identity;
- network identity.

The clean cutover removes the borrowed ECS `deferred_apply_index`. The predecessor
single global `publication_boundary_index` is also not retained as authority. Product,
query, and app-local derivative journals may record their owning class-local publication
sequence and schedule label where useful.

No compatibility aliases, forwarding fields, or mirrored old indices are authorized.

### 5. Empty queues do not create or remove lifecycle semantics

Publication phase occurrence is determined by explicit schedule placement, not by queue
contents.

An installed phase still has one occurrence when all of its handlers have no staged work.
Individual handlers may return a no-op result in that case. Conversely, an application
that does not install a publication phase receives no synthetic publication callbacks to
preserve predecessor cadence.

Runtime queue non-emptiness is derived data and never becomes lifecycle authority.

### 6. Failure is ordered fail-stop, not transactional rollback

A publication occurrence dispatches its handlers in accepted deterministic registration
order on the schedule-invoking thread.

If a handler returns an error or panics:

- the occurrence retains the class-local sequence assigned on entry;
- effects already committed by earlier handlers/earlier publication occurrences remain
  committed;
- later handlers and semantically later dependent systems do not successfully advance
  past the failure;
- later unpublished work remains unpublished;
- no generic product, query, application, or World rollback is implied;
- panic/error propagation follows the ordinary Runenwerk/RunenECS system invocation
  contract rather than a special ECS-boundary adapter contract.

This decision does not change product ratification or query freshness/invalidation rules.
It changes when their existing Runenwerk-owned publication machinery is invoked.

## Maintained application placement

These are current maintained semantic constraints for the implementation cutover. They
express required edges, not physical stage layouts.

### Runenwerk Draw `Update`

The minimal publication graph is:

```text
InputRoute
    -> PreviewJobs
        -> ProductPublication
            -> QuerySnapshotPublication
                -> FrameSubmit

PreviewJobs
    -> GpuValidation
        -> FrameSubmit
```

`ProductPublication` is not ordered against `GpuValidation` unless a separate semantic
dependency is later proven. Both branches converge at `FrameSubmit`.

Drawing product publication must precede drawing query publication because drawing query
descriptors are derived from accepted published product descriptors. Both must precede
`FrameSubmit`, which consumes the newly accepted visible product state.

### Runenwerk Editor `Update`

The maintained dependency skeleton is:

```text
InputBridge
    -> CompositionTransitions
        -> TargetInput
            -> ProductPublication
                -> FrameSubmit
                    -> ViewportPresentationSync
                        -> QuerySnapshotPublication
                            -> ProcgenViewportOverlay
                                -> ViewportProductTargets
                                    -> ViewportRenderJobs
```

Only the shown semantic dependencies are normative. Existing independent branches retain
their own ordering and must not be serialized against publication merely because they
shared predecessor execution stages.

`ProductPublication` is after `TargetInput` because both primary and secondary-window
input paths can dispatch app workflow commands that queue product work. It is before
`FrameSubmit` so same-frame app/UI consumers may observe accepted product state. Existing
`FrameSubmit -> ViewportPresentationSync` then ensures the current accepted product state
participates in the viewport projection.

`QuerySnapshotPublication` is after `ViewportPresentationSync` because viewport snapshots
are built from the observation projection produced there. It is before
`ProcgenViewportOverlay`: viewport observation snapshots ratify the projection just
produced by `ViewportPresentationSync`, while the procgen query handler in the same query
phase publishes snapshots for procgen descriptors that were accepted by the earlier
product phase. The later overlay/target/job/render-selection path therefore sees the
required query state without a second procgen-specific lifecycle clock.

Render preparation remains a separate Runenwerk schedule after `Update`; its viewport
product-selection system consumes `QuerySnapshotRuntimeResource` only after the Update
query-publication phase has completed successfully.

## Consequences

Runenwerk publication semantics no longer depend on how RunenECS groups or flushes ECS
work. The application owns the exact product/query acceptance points through explicit
schedule relations.

Product and query publication can evolve independently when their real producers and
consumers differ. This avoids both over-publication on executor boundaries and a false
single global barrier model.

The handler registries remain narrow and useful; the accidental ECS callback adapter and
borrowed ECS identity do not.

The new publication occurrence sequence provides deterministic local provenance without
claiming cross-class identity or portable meaning.

Because the explicit dispatch system requires exclusive World access, the first
implementation remains intentionally conservative and invoker-thread-only. Future
execution optimization may change physical realization only if these publication
semantics and handler ordering remain unchanged.

## Non-goals

This ADR does not:

- change RunenECS deferred-publication semantics;
- change product ratification rules;
- change query snapshot freshness, preservation, or invalidation rules;
- create a generic lifecycle/event framework;
- create a second scheduler;
- define worker execution or parallel publication;
- require product and query phases to occur once per frame;
- retain physical-stage/frontier cadence as compatibility behavior.

## Implementation gate

Issue #591 owns the Rust/runtime cutover.

That implementation must:

- remove publication lifecycle authority from `run_schedule_with_deferred_apply_boundary`;
- install explicit maintained Draw and Editor publication systems at the semantic positions
  above;
- remove `deferred_apply_index` and replace shared boundary indices with class-local
  publication occurrence sequences;
- preserve deterministic handler registration order and current ratification/freshness
  behavior;
- add tests proving publication count/identity is independent of ECS stage/frontier shape
  and queue emptiness;
- prove same-schedule Draw and Editor consumer visibility;
- be exercised against the exact RunenECS #33 candidate/accepted revision before
  downstream compatibility is claimed;
- pass canonical Runenwerk validation on an exact immutable candidate head.
