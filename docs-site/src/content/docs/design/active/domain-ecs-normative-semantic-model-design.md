---
title: RunenECS Normative Semantic Model Design
description: Normalized semantic contract for RunenECS runtime identity, world state, queries, change observation, mutation, systems, scheduling, deterministic reference execution, optimized execution, and external state adaptation.
status: active
owner: ecs
layer: domain/ecs
canonical: false
last_reviewed: 2026-09-09
related_docs:
  - ../accepted/runenecs-extraction-boundary-design.md
  - ../accepted/runenecs-boundary-repair-execution-plan.md
  - ../../architecture/repository-family-architecture.md
  - ../../adr/accepted/0014-repository-family-extraction-boundaries.md
  - ../../adr/accepted/0017-cross-authority-consistency-and-graph-semantics.md
---

# RunenECS Normative Semantic Model Design

## Status and authority

This document defines the proposed durable semantic model for standalone RunenECS.

It is an **active design**, not accepted authority and not an implementation issue.
The accepted RunenECS extraction boundary, accepted repair sequence, accepted ADRs,
and current owning GitHub issue remain higher implementation authority until this
design is explicitly accepted.

The accepted repair sequence remains:

```text
C0 -> C1 -> C2 -> C3 -> C4 -> C5 -> C6 -> C7 -> C8 -> C9
```

At the time of this review:

- C8 owns the active ECS-native schedule/access separation and removal of the
  `ecs -> scheduler` dependency;
- C9 remains the standalone conformance/performance gate;
- external population/cutover into `dornglut/runen-ecs` remains a separately
  accepted post-C9 boundary.

This design MUST NOT widen C8 or pre-authorize future capability implementation.

## Purpose

RunenECS should have one normalized semantic model from which storage,
scheduling, diagnostics, parallel execution, SIMD, and external integration can be
derived.

The governing rule is:

```text
one semantic model
      ↓
multiple valid physical realizations
```

RunenECS should be identifiable as:

> A storage-independent ECS with non-aliasing runtime identity, sound typed
> queries, explicit change and mutation semantics, normalized scheduling,
> deterministic serial reference execution, explainable execution constraints,
> and a mechanically testable path to optimized execution and external state
> adaptation without absorbing application, networking, persistence, rendering,
> spatial, physics, or other domain authority.

## Normative language

The terms **MUST**, **MUST NOT**, **SHALL**, **SHOULD**, **SHOULD NOT**, and
**MAY** are normative.

A future capability described here constrains that capability if later accepted.
It does not authorize implementation.

A **logical ECS observation** is behavior exposed by a supported RunenECS semantic
contract. Memory addresses, worker IDs, physical chunk boundaries, cache layout,
execution duration, SIMD width, and similar implementation details are not logical
observations unless an explicit expert API makes them so.

---

# 1. Ownership and dependency boundary

## RunenECS owns

RunenECS owns reusable ECS semantics for:

```text
World
Entity
Component
Resource
Bundle
runtime ECS type/reflection registration
entity/component/resource lifecycle
typed queries and filters
query safety
ECS-local change observation
component-removal observation where justified
structural mutation
Commands and deferred ECS mutation
System and SystemParam
runtime system identity
generic Schedule identity
SystemSet identity
semantic precedence
deferred ECS visibility
schedule validation
deterministic serial reference execution
ECS-neutral schedule/diagnostic facts
ECS-local derived indexes where independently justified
ECS-neutral cross-World Entity correspondence/remapping mechanics when proven
```

## RunenECS does not own

RunenECS MUST NOT own:

```text
application frame lifecycle
fixed timestep policy
startup/shutdown policy
render lifecycle
physics semantics
general spatial semantics
network sessions
network authority
replication policy
prediction/reconciliation
interest/relevancy
network transport or wire format
stable network object identity
save-file format
persistent object identity
portable schema identity/versioning
schema migration policy
editor persistence format
archival replay policy
cross-framework job scheduling
application worker lifetime
OS-main-thread meaning
GPU execution/submission
product publication
query-product publication
generic non-ECS workflow/DAG execution
```

A value does not become ECS-owned merely because it is stored in a component or
resource.

## Standalone dependency rule

Standalone RunenECS MUST remain independently useful and MUST NOT require
Runenwerk or peer Runen frameworks to define its core semantics.

Cross-domain integration follows:

```text
RunenECS owner API
      ↓
Runenwerk / consumer adapter
      ↓
other semantic authority
```

In particular, standalone RunenECS MUST NOT duplicate Runenwerk-local portable
schema, networking, publication, or host-execution ontology merely because
repository dependency direction prevents depending on those owners.

---

# 2. Semantic separation laws

The following distinctions are normative:

```text
runtime Entity            != portable object identity
runtime TypeId/token      != portable schema identity
query semantics           != storage layout
change revision           != application tick
direct access             != deferred effect
access incompatibility    != semantic precedence
semantic precedence       != physical serialization
semantic precedence       != necessarily deferred visibility
deferred visibility       != application publication
SystemSet                 != execution stage
ScheduleLabel             != application lifecycle phase
executor batch            != semantic dependency
worker completion order   != logical publication order
SIMD span                 != query semantics
ECS Commands              != portable application commands
state adaptation          != serialization format
```

Physical optimizations MUST derive from semantic facts rather than become hidden
second authorities.

---

# 3. Runtime identity

## General runtime-token law

Every correctness-relevant runtime token used to address owner-local semantic
state SHALL have a defined validity domain.

Examples include:

```text
Entity
runtime component/resource registration handles
SystemId
future runtime-local query/plan handles
```

A token originating from owner instance A MUST NOT silently resolve to unrelated
state in owner instance B merely because local numeric representations coincide.

A public owner-local addressing token therefore MUST satisfy at least one of:

1. carry sufficient owner/lineage identity to reject foreign use;
2. be lifetime/type bound so foreign use is impossible; or
3. not escape as independent addressing authority.

This does **not** authorize one universal Runen ID framework or global identity
registry. Each owner implements the invariant locally.

## Runtime-token exhaustion

A correctness-relevant identity allocator MUST NOT:

```text
wrap into a live identity
saturate into duplicate identity
reuse an identity while an older valid token may still exist
```

Valid exhaustion strategies include checked failure, permanent slot retirement,
representation widening, epoch transition, or terminal failure before reuse.

The invariant is:

```text
identity exhaustion never becomes aliasing
```

Telemetry counters and metrics that are not addressing identities may have weaker
overflow behavior when explicitly documented.

## Entity

Runtime `Entity` semantically consists of:

```text
Entity = WorldScopeId + Slot + Generation
```

`Entity` SHALL remain opaque, copyable, comparable, hashable, and world-local.

World operations SHALL validate World scope before local slot/generation meaning.
Applicable failures remain distinguishable where relevant:

```text
ForeignWorld
UnknownEntity
StaleGeneration
AlreadyFreed
IndexExhausted / allocation exhaustion
```

Rejected entity operations MUST NOT mutate World state.

Generation exhaustion permanently retires a slot rather than wrapping or
saturating into an older valid identity.

## WorldScopeId

`WorldScopeId` is opaque process-local runtime identity. It is not persistence,
network, editor, schema, or user-facing stable identity.

The accepted C1 contract remains authoritative for its theoretical exhaustion:
World-scope allocation is checked and non-reusing; exhaustion is a terminal ECS
identity-space condition and construction terminates before reuse. This design does
**not** require making the ordinary `World::new()` / `World::default()` surface
fallible solely for that theoretical process-global condition.

## Runtime registry tokens

Compact component/resource registration ordinals MAY exist internally.

If an ordinal escapes publicly and callers can pass it back for lookup, it MUST
obey the runtime-token law. A token from World/registry A MUST NOT silently address
another type in World/registry B.

If no maintained consumer requires public runtime-key addressing, compact ordinals
SHOULD remain private rather than become unnecessary scoped public IDs.

A forgeable `Default`/raw constructor is inappropriate for a public runtime token
unless that sentinel has a real safe semantic.

## SystemId

`SystemId` is runtime ECS identity, not portable schedule, persistence, network, or
cross-runtime identity.

If callers can feed a `SystemId` back into a runtime for lookup/inspection,
cross-runtime misuse MUST be rejected or structurally impossible. If the value is
only a report-local ordinal, its scope SHOULD remain report-local rather than claim
greater identity.

C8 SHOULD derive the smallest truthful `SystemId` representation rather than
mechanically transplant a forgeable/saturating migration-scheduler ID.

## Context-free serialization of runtime tokens

Core RunenECS SHOULD NOT provide context-free portable serialization of raw
runtime-local tokens such as:

```text
Entity
WorldScopeId
SystemId
registry-local ordinals
change cursors
```

Runtime-local diagnostic serialization MAY exist if clearly documented as
runtime-local. Portable transfer requires explicit correspondence/mapping context.

---

# 4. World, components, resources, reflection, and storage

## World

A `World` is one independent authoritative ECS state container.

Semantically:

```text
World =
    WorldIdentity
  + EntityState
  + ComponentState
  + ResourceState
  + RuntimeTypeState
  + ChangeState
  + DerivedEcsState
```

`DerivedEcsState` may include query caches, archetype metadata, secondary indexes,
and other rebuildable accelerators.

## Storage independence

RunenECS MAY physically use archetypes, tables, columns, sparse sets, pages,
chunks, pools, hybrid layouts, and secondary indexes.

The following MUST NOT become portable logical identity merely because the current
implementation uses them:

```text
archetype ID
table ID
chunk ID
row
memory address
allocation address
```

A storage redesign must preserve logical ECS behavior.

## Component and Resource

`Component` and `Resource` identify ECS storage/access roles.

They do not automatically imply:

```text
Serialize
NetworkReplicated
Persistent
Reflect
StableSchema
Send + Sync
```

These are separate capabilities.

RunenECS SHOULD NOT globally require `Component: Send + Sync` or
`Resource: Send + Sync` merely to simplify future parallel execution. Thread
eligibility depends on actual system access and captured state.

## Bundle

A `Bundle` is composition syntax for a finite component set participating in a
documented structural operation. It is not a permanent storage object, entity type,
or archetype identity.

## Runtime reflection

Runtime reflection authority SHALL remain explicit and instance-owned.

It MAY describe Rust `TypeId`, runtime shape, fields/variants, runtime
component/resource capabilities, value access, and presentation names.

It MUST NOT automatically establish portable schema identity/version, wire/save
encoding, or migration policy.

Stable portable identity MUST NOT be inferred from runtime registration order,
memory layout, display name, or uncontrolled Rust type paths.

---

# 5. Query semantics and safety

## Query model

A query semantically normalizes to:

```text
QueryDefinition {
    Selection,
    Fetch,
    Filter,
    DirectAccess,
}
```

`Selection` identifies candidate entities, `Fetch` defines logical returned
values/references, `Filter` determines matching, and `DirectAccess` describes
immediate ECS state access.

Storage layout is not part of this definition.

## Query safety

Safe query construction MUST prevent incompatible mutable aliasing.

Any low-level extension whose access metadata participates in Rust reference safety
MUST be framework-sealed or explicitly unsafe with a complete public safety
contract. Safe downstream code MUST NOT be able to under-declare access and cause
RunenECS to manufacture invalid references.

## QueryState lineage

Reusable query state MAY cache match/storage/change facts. Every World-relative
fact MUST be invalidated or explicitly rebound when reused with another World.

A change cursor or cache from World A never silently retains meaning in World B.

## Query ordering

Default portable query iteration order SHALL be unspecified.

Applications MUST NOT derive semantic order from archetype creation order, table
order, row order, chunk order, hash iteration, allocation address, or worker
assignment.

A repeatable current implementation is not automatically a public ordering
contract.

Order-sensitive algorithms SHOULD use an explicit stable application key, ordered
query/projection, deterministic reduction, or other explicit mechanism rather than
forcing ordinary query iteration to become globally ordered.

## Filter-aware disjointness

The access model MAY conservatively prove that two filtered accesses operate on
disjoint entity domains, for example:

```text
Write<Transform> With<Player>
Write<Transform> Without<Player>
```

If disjointness cannot be proven, overlap MUST be assumed. Filter-aware reasoning
changes concurrency legality, not query meaning.

## Query partitioning

Logical query results MAY be physically partitioned for parallel or vectorized
execution. Partition boundaries, counts, and worker assignment are non-semantic.

Every logical match must still obey ordinary query safety and exactly-once
semantics where applicable.

## Failed-access purity

A failed ordinary ECS access MUST NOT fabricate a successful mutation observation.

Examples include:

```text
get_mut<T>(entity without T)
require_mut<T>(entity without T)
resource_mut<R>() when R is absent
```

A successful mutable borrow MAY conservatively mark a value as changed even if the
caller writes the same value or does not ultimately mutate it.

The semantic distinction is:

```text
successful mutable access
    MAY conservatively count as changed

failed access
    MUST NOT manufacture changed state
```

Merely attempting to access an absent resource SHOULD NOT silently register a new
runtime type unless registration is explicitly part of that API contract.

---

# 6. Change observation

## Separation of concepts

RunenECS SHALL distinguish:

```text
ChangeRevision
ChangeCursor
latest current change stamps
optional retained ChangeJournal
component-removal observations
```

These concepts MUST NOT collapse into one overloaded raw integer or unbounded log.

## ChangeRevision

A `ChangeRevision` is World-local logical ordering evidence for accepted ECS
mutation. It is not an application frame, simulation tick, network tick, wall
clock, or portable revision identity.

Its representation is replaceable and may use a checked integer, epoch + counter,
bounded modular-age semantics, rebasing, or another proven mechanism.

## Revision exhaustion

Change tracking MUST NOT silently stop detecting future changes because a finite
counter saturated or wrapped incorrectly.

Invalid behavior is:

```text
revision == MAX
mutation occurs
revision remains MAX
changed_since(MAX) => false
```

When exact freshness can no longer be proven, RunenECS MUST fail conservatively.
For invalidation semantics:

```text
unknown freshness => changed / rebuild
```

is valid. False-negative reuse is not.

## ChangeCursor

A public change cursor is World-lineage-local and scoped to whatever age/retention
contract applies. Cross-World use must reject or explicitly reset/rebind according
to the API.

## Added and Changed

For an observer:

- `Added<T>` means `T` became associated with an entity after the relevant prior
  observation point;
- `Changed<T>` means `T` passed through the documented mutation/change-marking path
  after that point.

`Changed<T>` does not inherently mean `old_value != new_value`.

## Current freshness versus retained history

Current freshness queries SHOULD NOT require scanning an ever-growing history.
Latest relevant change metadata SHOULD remain independent from an optional retained
journal.

Pruning historical records MUST NOT break ordinary `Added`/`Changed` or
current-freshness semantics.

## Optional ChangeJournal

A retained full history journal is optional.

If retained, it MUST define retention, pressure policy, ordering, cursor validity,
pruning, and history-loss behavior. A bounded journal must distinguish complete
history from lost history rather than return an incomplete sequence as complete.

If no maintained independent consumer requires full retained history, deletion is
preferred over permanently retaining unbounded logs.

## Component removal

The durable semantic fact currently approximated by `Orphaned<T>` is component
removal:

```text
ComponentRemoved<T> {
    entity,
    revision,
}
```

It means `T` was removed from the entity. It does NOT mean the entity is dead,
currently lacks `T`, cannot have had `T` reinserted, or is semantically an
"orphan".

Removal observation MUST NOT permanently depend on stage/wave lifetime. Before
standalone external API stabilization, C9 SHALL explicitly decide whether to
remove the current capability or retain it under removal-oriented terminology and
cursor/retention semantics without compatibility aliases.

During C8, maintained current removal behavior must remain correct while scheduler
ownership changes. An extra physical deferred flush is not observationally free if
a surviving removal API can observe it through record clearing/lifetime.

---

# 7. Structural mutation and Commands

## Structural mutation

Structural mutation includes spawn, despawn, component insertion/removal, and
bundle structural changes.

Every safe framework-owned structural operation SHALL document its atomicity
boundary. The C2 baseline remains operation-level atomicity: a rejected documented
atomic operation does not partially commit that operation.

Whole multi-operation transactions are not baseline semantics and require separate
design.

## Direct access versus deferred effects

A system may simultaneously have:

```text
DirectAccess:
    Read<Transform>

DeferredEffects:
    MaySpawn
    MayInsert<Health>
```

Direct access answers whether live invocations can safely overlap. Deferred effects
answer what may become live later, when visibility is required, and whether
unordered deferred operations may conflict.

A `Commands` parameter MUST NOT permanently mean an exclusive live `World` borrow
merely because a conservative implementation currently serializes it.

## Deferred effect vocabulary

Where useful and framework-known, deferred effects MAY distinguish concepts such as:

```text
MaySpawn
MayDespawn
MayInsert<T>
MayRemove<T>
MayWriteResource<T>
UnknownStructuralEffect
UnknownWorldEffect
```

Precise metadata is truthful only where the framework knows the operation.

## Known versus opaque deferred commands

Framework-known commands such as spawn/despawn/insert/remove have precise operation
semantics and inherit the atomicity of the underlying framework World operation.

An opaque custom deferred callback receiving `&mut World` may perform arbitrary
World operations or external side effects. Its truthful effect is conservatively
`UnknownWorldEffect` unless a stronger separately accepted proof/declaration
mechanism exists.

An opaque custom callback is NOT automatically atomic. It may mutate A, mutate B,
and then return an error, leaving those earlier mutations committed.

## Commands ordering and failure

`Commands` is an ordered fail-stop deferred buffer, not a transaction.

Within one producer buffer, insertion order is canonical.

If the buffer contains:

```text
C0
C1
C2
C3
```

and `C2` fails during application:

```text
C0, C1
    remain committed

C2
    has exactly C2's own atomicity contract

C3
    does not execute
```

## Invocation-local staging

Each system invocation using Commands owns invocation-local staged work that is not
live World state until an ECS deferred-application boundary publishes it.

If parameter extraction or the system invocation returns recoverable failure before
publication, its unpublished staged work is discarded. Direct World writes already
performed by the invocation are not automatically rolled back.

## Panic / unwind hygiene

A Rust panic is not a recoverable schedule transaction and does not imply rollback
of direct writes.

However, if unwinding propagates and the Runtime/Schedule object remains usable,
unpublished deferred work from the aborted run MUST NOT later execute during an
unrelated schedule invocation. RAII/unwind cleanup SHOULD guarantee that invariant
without requiring catch-unwind or transactional rollback.

---

# 8. System semantic model and thread constraints

A system conceptually normalizes into:

```text
SystemSemantic {
    RuntimeIdentity,
    Executable,
    ParamState,
    DirectAccess,
    DeferredEffects,
    ExecutionConstraints,
    Configuration,
}
```

This is an ontology, not a required monolithic public struct.

## DirectAccess

At minimum the model distinguishes:

```text
ComponentRead<T>
ComponentWrite<T>
ResourceRead<T>
ResourceWrite<T>
ComponentRemovalRead<T> where retained
ExclusiveWorld
```

Access declarations MUST be conservative. Under-declaration is invalid where it
can affect Rust aliasing safety, parallel legality, or schedule correctness.
Over-declaration is safe but may reduce optimization.

## External side effects

ECS access metadata describes ECS authority. Arbitrary Rust systems may also affect
global state, filesystem, sockets, atomics, interior-mutability state, random
sources, wall-clock time, or foreign libraries.

Therefore baseline ECS access declarations are NOT proof of arbitrary program
purity. Strong deterministic or incremental-execution profiles require additional
constraints where external effects matter.

## Thread transferability

Access compatibility and worker-thread eligibility are separate facts.

A future normalized system execution constraint MAY distinguish:

```text
Transferable
InvokerThreadOnly
```

`InvokerThreadOnly` means the thread that invoked the ECS schedule, not inherently
the OS main/window/render thread.

Transferability should be derived conservatively from system callable/captured
state, SystemParam state, and accessed component/resource thread-safety.

For example, worker execution of shared `&T` requires sharing safety equivalent to
`T: Sync`, while mutable/exclusive transfer requires the relevant transfer safety
equivalent to `Send`.

RunenECS does not need to expose the entire `World` as freely `Send + Sync` to run
systems in parallel; a small framework-owned unsafe executor core may split
internally proven-disjoint access after adequate mechanical proof.

---

# 9. Schedule semantics

## Schedule identity

A schedule label is generic ECS identity. RunenECS MUST NOT infer application
meaning from its name.

A label named `FixedUpdate` is only a label to RunenECS; the application owns when
and why that schedule is invoked.

## Normalized relations

RunenECS SHALL keep conceptually separate:

```text
SemanticPrecedenceGraph
DirectAccessConflictGraph
DeferredVisibilityGraph
PhysicalExecutionPlan
```

They MUST NOT collapse into one generic dependency graph.

## Semantic precedence

Let `P(A, B)` mean A semantically precedes B.

`P` is a strict partial order: irreflexive, acyclic, and transitively meaningful.
Syntax such as `before`, `after`, `chain`, and set ordering normalizes into this
relation.

## Access incompatibility

Let `X(A, B)` mean A and B cannot safely overlap because of direct access hazards.
`X` is symmetric.

Critically:

```text
X(A, B) does not imply P(A, B)
X(A, B) does not imply P(B, A)
```

Physical serialization required by access safety does not create application
meaning.

## Deferred visibility

Let `V(A, B)` mean B is guaranteed to observe A's eligible deferred ECS effects
before B executes.

Semantic precedence and deferred visibility remain separately representable.

For the ordinary authoring path, `before` / `after` / `chain` SHOULD mean:

```text
semantic precedence
+
required deferred visibility
```

when the predecessor owns eligible deferred ECS effects.

A future expert order-only form MAY create semantic precedence while leaving
visibility unspecified from that edge. The expert form must be explicit.

## Hazard-induced serialization law

Serialization caused only by access conflict, thread restriction, worker capacity,
load balancing, or physical batching MUST NOT create semantic precedence or a
**guaranteed** deferred-visibility relationship.

An implementation may physically apply deferred work more often, but programs must
not rely on incidental visibility without declaring the semantic requirement.

## DeferredApplyBoundary

The normalized ECS concept that makes eligible deferred ECS effects live is:

```text
DeferredApplyBoundary
```

It replaces migration-era semantic dependence on stage end, wave end, or a generic
application barrier.

A DeferredApplyBoundary is ECS-owned. It is not ProductPublication,
QuerySnapshotPublication, RenderSubmit, FrameEnd, or ReplayNetworkCapture.

Derived boundaries SHOULD be justified by actual ECS semantics such as required
deferred visibility, an explicit expert boundary, or final successful schedule
completion.

Extra physical application is legal only if every surviving logical ECS
observation remains valid, including removal/change observation and failure
semantics.

## SystemSet

A `SystemSet` is grouping/configuration identity. Membership alone does not imply
ordering, serialization, worker affinity, or execution stage.

A system may belong to multiple sets. If nested set hierarchy is introduced, it
must be acyclic.

## Ordering target validation

At preparation time an ordering target should resolve as a known system, known
explicitly declared set, valid empty set, unresolved target, or cross-schedule
target.

A strict profile SHOULD reject unresolved/cross-schedule targets. A normal
development profile SHOULD at least diagnose them. A typo should not silently
become a successful no-op dependency.

## Ambiguity

A semantic ambiguity exists when semantically unordered operations can produce
order-dependent logical outcomes, for example conflicting direct writes,
read/write interaction, noncommutative deferred effects, or opaque
`UnknownWorldEffect` commands.

An ambiguity policy MAY support Error/Warn/Allow. A strict deterministic profile
SHOULD use Error. Acknowledging ambiguity means the unordered interaction is
intentional; it does not insert semantic order.

## PreparedSchedule

A prepared schedule is derived runtime execution information. It MAY contain
normalized systems, set expansion, semantic precedence, access conflicts, deferred
effects, visibility requirements, derived boundaries, execution constraints,
ambiguity diagnostics, a canonical serial linearization, and reason/provenance
facts.

It is not an independent semantic authority.

Changes to system registration/removal, set membership, ordering, conditions, or
relevant access/effect metadata invalidate preparation and take effect between
schedule invocations rather than silently mutating a running topology.

## Physical execution concepts

Stages, waves, batches, tranches, tasks, worker groups, and SIMD spans are executor
or compiler implementation concepts. They MAY exist internally but MUST NOT become
required schedule semantics merely because an executor uses them.

---

# 10. ECS-neutral host integration seam

Runenwerk currently has maintained host/product work that must occur around parts
of ECS schedule execution.

C8 may therefore expose the smallest ECS-neutral integration fact necessary to
communicate that an ECS deferred-application boundary was reached, or equivalent
schedule progress.

The exact API remains a C8 implementation decision.

RunenECS MUST NOT encode host concepts such as:

```text
ProductPublication
QuerySnapshotPublication
RenderSubmit
ReplayNetworkCapture
```

inside its scheduling ontology.

Runenwerk Engine owns what host/application work occurs when it observes an
ECS-neutral boundary.

Boundary identifiers, if exposed, are runtime execution-local facts and do not
make wave/stage/barrier numbers portable semantic identity.

---

# 11. Serial reference execution and failure

## Canonical serial reference

Every valid prepared schedule SHALL define one deterministic serial system
linearization consistent with semantic precedence.

A deterministic tie-break may order semantically unordered systems for executable
reference behavior. That tie-break does not create semantic precedence and
applications SHOULD NOT rely on it as hidden ordering authority.

## Reference executor scope

The serial reference executor defines:

```text
inter-system reference order
required deferred publication order
reference producer-buffer merge order
recoverable fail-stop progression
```

It does **not** by itself make arbitrary system code deterministic.

It does not automatically make randomness, wall-clock access, external I/O,
unspecified query iteration, floating-point reduction order, or hidden external
state deterministic.

## Reference execution

Conceptually:

```text
prepare schedule

for each system in canonical reference order:
    determine eligibility
    extract parameters
    execute system

    on success:
        make invocation-local deferred work eligible

    on recoverable failure:
        discard still-unpublished deferred work according to the current cut
        stop

    whenever required visibility demands:
        apply eligible deferred work in canonical order

on successful schedule completion:
    apply all remaining ordinary eligible deferred work

return
```

A normal complete successful schedule therefore returns with ordinary deferred ECS
work applied. A deliberately externally managed deferred remainder would require a
separate expert contract.

## Failure before deferred application

Suppose A and B succeed and stage unpublished deferred buffers, then C fails before
the next deferred application.

Baseline behavior is:

```text
direct writes already performed by A/B/C
    remain

eligible but unapplied A/B deferred buffers
    are discarded

C's unpublished buffer
    is discarded

later systems
    do not execute
```

This is fail-stop execution, not schedule transaction rollback.

## Failure during deferred application

Suppose canonical deferred command order is:

```text
A0
A1
B0
B1
```

and `B0` fails.

Then A0/A1 remain committed, B0 has B0's own atomicity contract, B1 and later
commands do not execute, and the schedule returns failure.

## Canonical deferred order

Inside one producer buffer, command insertion order is canonical. Across producer
buffers participating in one application boundary, canonical serial system order
provides the reference producer order.

Worker completion order MUST NOT define deterministic deferred publication.

---

# 12. Determinism model

RunenECS SHALL distinguish several different claims rather than use one vague
"deterministic" label.

## Reference scheduling determinism

For one prepared schedule, RunenECS deterministically derives:

```text
serial system reference order
required deferred-boundary semantics
producer-buffer merge order
```

This is the baseline C8 scheduler guarantee.

## Deterministic ECS-program profile

A stronger future deterministic profile constrains the program enough that logical
ECS observations are reproducible independently of permitted physical executor
choices.

Such a profile SHOULD require at least:

```text
no unresolved semantic ambiguity
no reliance on unspecified query order
order-sensitive aggregation uses explicit deterministic ordering/reduction
randomness is explicit deterministic input
external time is explicit input
opaque external side effects are excluded or outside compared state
parallel command publication uses scheduling-independent ordering
relevant state has explicit deterministic observation/equality projection
```

Ordinary unordered queries remain valid in a deterministic program when the
operation is order-independent.

## Portable numeric determinism

Cross-platform bit-identical floating-point results are a stronger separate
capability. CPU architecture, SIMD width, FMA, compiler/codegen, floating-point
environment, and third-party numerical implementations can all affect results.

Portable numeric determinism requires separate design and proof.

## Logical observations

Executor equivalence may compare, as applicable:

```text
success/failure class
entity liveness and reuse relationships
component membership
logical component values
resource presence/value
Added/Changed/removal observations
deferred mutation result
structured ECS failures
```

Process-local World scope IDs are mapped/ignored when comparing independently
created equivalent Worlds.

Whole-World equality/hash MUST NOT be assumed for arbitrary user components.
Equivalence testing should use explicit observation projections, registered stable
comparators/digests, or domain assertions rather than requiring every component to
implement `Eq + Hash + Serialize`.

---

# 13. Optimized execution, parallelism, and SIMD

## Equivalence levels

A future optimized executor should state its guarantee precisely.

### Successful reference equivalence

For admitted deterministic/well-specified executions where serial reference
execution succeeds, optimized execution also succeeds and produces equivalent
logical ECS observations.

### Failure-prefix equivalence

A stronger optional contract means recoverable failure leaves an equivalent
committed semantic prefix and failure class to serial reference execution.

This is harder and MUST NOT be claimed merely because successful results match.

## Minimum failure containment

Even without failure-prefix equivalence, an optimized executor MUST preserve memory
safety, World structural invariants, Entity validity, framework operation-level
atomicity, structured failure reporting, and no later replay of abandoned deferred
buffers.

A World after parallel recoverable failure may be structurally valid but
semantically partially advanced if work beyond the serial failure point already
performed direct writes and the executor did not advertise failure-prefix
equivalence. Host/application recovery policy owns what happens next.

## Publication frontier

Optimized execution SHALL distinguish work completion from logical publication.
The publication frontier is the largest reference-consistent portion whose effects
may safely become observable under the executor's advertised equivalence level.

Worker completion timing alone cannot advance semantic publication.

## Thread transferability

A future executor may overlap systems only when semantic precedence, direct access,
required deferred visibility, conditions, thread transferability, and advertised
failure/publication semantics permit it.

Changing worker count MUST NOT change logical successful results for an executor
claiming deterministic successful-reference equivalence.

## Parallel query execution

Baseline parallel query semantics are:

```text
every logical match processed exactly once
iteration order unspecified
worker assignment unspecified
partition boundaries unspecified
```

Scalar query safety rules remain unchanged.

## Parallel command production

Deterministic parallel deferred command production cannot use arbitrary thread
completion order.

A deterministic implementation must either:

1. forbid deferred command production for that operation;
2. assign scheduling-independent canonical producer/source ordinals plus local
   command ordinals; or
3. prove the relevant effects commute under the advertised contract.

A deliberately nondeterministic expert API MAY exist but must be visibly outside
the deterministic profile.

## Deferred entity creation

If future deferred/parallel code must refer to an entity before it is live, it
SHOULD use a provisional concept such as `DeferredEntity`, not a live `Entity`.

Final live Entity assignment under a deterministic profile MUST NOT depend on
worker completion timing. Resolution may occur through canonical publication order
or another proven deterministic reservation mechanism.

## Deterministic reductions

A deterministic parallel reduction MUST define logical input domain, partition
semantics, local reduction, merge topology, and merge order. Worker completion
timing is not a deterministic merge order.

For non-associative arithmetic such as ordinary floating-point addition, merge
association is part of numerical behavior.

## SIMD

SIMD is an execution optimization, not ECS semantics.

A logical query may execute through scalar code, auto-vectorization, portable SIMD,
architecture-specific SIMD, or parallel + SIMD execution.

A future expert query capability MAY expose temporary contiguous component spans
when physical storage proves them. Such a capability can support SIMD, bulk
transforms, FFI/vector libraries, and cache-efficient kernels without making
archetype/chunk boundaries semantic.

Contiguous access is optional and may be unsupported for sparse storage,
incompatible filters/query shapes, or storage implementations that cannot prove
contiguity. Span count, size, order, storage origin, and SIMD width remain
non-semantic unless an expert API explicitly documents stronger guarantees.

---

# 14. External state adaptation, networking, persistence, cloning, and forks

## Principle

Ownership separation MUST NOT make networking, saving/loading, editor transfer, or
cloning require private ECS storage reach-through.

The normalized relationship is:

```text
RunenECS logical state
        ↓
ECS-neutral state access / Entity correspondence
        ↓
external adapter
        ↓
network / save / editor / schema authority
```

## ECS-neutral adaptation primitives

Where real consumers prove a reusable contract, RunenECS MAY own generic
mechanisms for:

```text
enumerating selected runtime entities
enumerating selected component/resource state
typed/reflected runtime state access
reporting selected ECS change facts
mapping source Entity correspondence to target Entity
remapping embedded Entity references
applying validated ECS mutations
```

These are ECS mechanics, not external format policy.

## Entity correspondence

Any transfer of entity-containing state between independent Worlds requires
explicit correspondence:

```text
Source Entity
      ↓
Entity mapping
      ↓
Target Entity
```

Raw Entity bits MUST NOT be treated as portable identity.

A component may contain direct, optional, collection, or nested Entity references.
Cross-World copy/load requires those references to be remapped or the operation to
reject that type.

A generic public remapping trait/derive SHOULD be introduced only after actual
save/network/clone/fork consumers prove the needed ECS-neutral shape.

## Persistence

RunenECS does not own serialization codec, save-file structure, persistent object
identity, portable schema/version, migration policy, compression, or unknown-field
behavior.

A clean persistence adapter may:

```text
select ECS state
map runtime ECS types to portable schema
map Entity -> persistent application identity
encode through persistence owner
```

A clean load path may:

```text
decode external schema
validate/migrate externally
allocate target entities
build external-ID -> Entity correspondence
remap embedded Entity references
apply validated ECS mutations
```

For large all-or-nothing loads, building and validating a fresh World before host
publication is preferred to requiring arbitrary rollback of a partially mutated
live World.

## Networking

RunenECS does not own network object IDs, protocol schema, authority, replication,
prediction, reconciliation, interest, transport, or session semantics.

Typical integration remains:

```text
network/application stable identity
             ↕
      explicit Entity map
             ↕
          RunenECS
```

RunenECS SHOULD expose enough neutral selected-state/change/remapping capability
that RunenNet/Runenwerk adapters do not require private storage reach-through.

## World cloning/forking

Future independent World clones/forks SHOULD receive distinct World scope identity.
Source Entity tokens remain invalid in the branch.

An optimized internal representation MAY preserve cheap slot/generation
correspondence while changing only branch scope, but source↔branch correspondence
remains explicit semantic state rather than raw Entity aliasing.

World patch/merge semantics are compatible with this model but require separate
design for base revision, correspondence, structural/component changes, conflict
rules, and merge policy.

---

# 15. Future extension constraints

The following are compatible future ECS capabilities but are NOT defined or
authorized by this core design:

```text
run conditions
component hooks
observers
relationships
required components
structural invariants
whole-buffer structural transactions
World forks/patches
incremental systems
dynamic/reflected runtime queries
schedule stepping
GPU-assisted ECS workloads
portable state projection
```

Each future capability must define its interaction with relevant existing
invariants such as operation atomicity, reentrancy, deferred mutation, entity
lifetime, change observation, remapping, parallel execution, and failure behavior.

No capability is added merely because another ECS exposes it.

---

# 16. Explainability and diagnostics

Important scheduling decisions SHOULD eventually be explainable through their true
semantic causes.

Useful questions include:

```text
Why must A precede B?
Why can A and B not overlap?
Why must deferred effects be applied here?
Why is this interaction ambiguous?
Why is this system InvokerThreadOnly?
Why can optimized publication not advance farther?
```

Reason categories should remain distinct, for example:

```text
SemanticPrecedence
DirectAccessConflict
RequiredDeferredVisibility
DeferredEffectConflict
UnknownDeferredEffect
InvokerThreadConstraint
RunCondition
ExclusiveWorld
PublicationFrontier
```

Durable schedule inspection SHOULD expose normalized systems, sets, semantic
precedence, access incompatibilities, deferred visibility, derived ECS boundaries,
ambiguities, execution constraints, and reason provenance.

Physical stage/wave/batch data may appear as executor-specific diagnostics but is
not the semantic schedule model.

A future schedule/plan fingerprint MAY support regression testing or cache
invalidation, but if it contains runtime IDs, Rust `TypeId`, or registry ordinals
it MUST remain scoped to the relevant build/runtime and MUST NOT masquerade as a
portable schema or network compatibility identifier.

Diagnostics/telemetry observe behavior and MUST NOT become hidden control
authority.

---

# 17. Public API minimality and C9 disposition

The normalized model does NOT automatically ratify every current public symbol.

Before external standalone stabilization, every surviving public concept should
have a distinct independently useful ECS semantic.

C9 SHALL explicitly review current surfaces such as:

```text
ComponentTypeKey / ResourceTypeKey
raw change ticks/cursors
full component/resource change journals
query_snapshot_source_generation
QueryOrphaned / Orphaned
StatefulComponent / ComponentState
ResView
secondary-index APIs
migration-era stage/wave/barrier plan reports
opaque custom DeferredCommand extension points
```

Each questionable surface should receive one explicit disposition:

```text
KEEP
    distinct useful ECS semantic with correct contract/evidence

REDESIGN
    useful semantic but current identity/safety/ownership contract is wrong

DELETE
    migration residue, alias, duplicate authority, implementation detail,
    or unjustified surface

FOLLOW-UP DESIGN
    useful capability proven, but semantics require separate acceptance
```

Source compatibility alone is not a sufficient reason to retain a concept because
the extraction architecture explicitly rejects long-lived compatibility residue.

## Current-source concerns that C9 should verify

The current-main audit leading to this design identified concrete conformance/public
surface risks that should be re-resolved against then-current accepted source rather
than treated as frozen findings:

```text
failed get_mut / require_mut modification side effects
failed resource_mut registration side effect
change revision saturation/exhaustion
runtime registry ordinal saturation and owner scope
StatefulComponent counter saturation/necessity if retained
QueryOrphaned terminology/lifetime/necessity
full retained change-log necessity/pressure policy
query_snapshot_source_generation necessity/naming
ResView alias necessity
secondary-index public value
custom DeferredCommand trust/atomicity documentation
final normalized schedule-report vocabulary
```

C9 must not merely benchmark whatever surface survives C8; it must establish the
standalone conformance/public contract before external stabilization.

---

# 18. C8 interpretation and stop conditions

C8 remains limited to its accepted scope:

```text
runtime ECS system identity
generic Schedule labels
SystemSet identity
in_set / before / after
ECS access facts
schedule validation
cycle rejection
access-conflict diagnostics
deterministic serial reference execution
ECS deferred-command application boundary
ECS-neutral diagnostics where maintained consumers prove them
Engine migration away from scheduler-owned product/lifecycle policy
```

This design MUST NOT cause C8 to implement parallel execution, parallel queries,
SIMD APIs, run conditions, relationships, transactions, serialization, World
forks, incremental systems, or portable schema.

## Access-derived visibility stop condition

The migration scheduler currently derives physical groups partly from access
compatibility. C8 MUST NOT preserve this accidental semantic chain:

```text
access conflict
    ↓
physical stage/wave separation
    ↓
automatic deferred flush
    ↓
consumer-visible semantic ordering/visibility
```

The mandatory C8 census must determine whether a maintained consumer intentionally
relies on that behavior.

If it does, STOP and classify the actual missing semantic requirement rather than
freezing access-induced stage behavior into standalone RunenECS.

## Engine publication migration

Current Engine/product consumers genuinely require publication behavior around ECS
execution. C8 must preserve their behavior through the narrowest ECS-neutral
boundary/progress integration seam while moving ProductPublication,
QuerySnapshotPublication, render/replay/network/application barriers back to their
correct Runenwerk owner.

Do not preserve a generic application barrier registry inside RunenECS merely to
retain migration shape.

## SystemId migration

Because C8 owns runtime system identity, it SHOULD avoid mechanically preserving a
forgeable/saturating owner-unsafe scheduler `SystemId` if that value remains
addressable public runtime identity.

## Unwind hygiene

Because C8 materially rewrites execution flow, it SHOULD ensure an aborted/unwound
schedule invocation cannot leak unpublished deferred buffers into a later unrelated
run. This requires cleanup correctness, not panic rollback.

---

# 19. Conformance and validation strategy

## C9 semantic evidence

Standalone conformance should prove at least:

### Runtime identity

```text
foreign-World Entity rejection
stale generation rejection
double free
generation exhaustion
World-scope no-reuse terminal behavior
owner-safe runtime tokens where public
SystemId owner safety where addressable
```

### Access and mutation

```text
failed access purity
atomic spawn
atomic insert/remove/despawn
bundle preflight
```

### Queries

```text
mutable alias rejection
resource/exclusive conflicts
QueryState cross-World reset/rebind
Added
Changed
unspecified portable iteration order
```

### Change observation

```text
revision progression
exhaustion/wrap/rebase behavior
no false-negative invalidation
cursor World/lineage validity
component-removal semantics if retained
history-loss behavior if retained journal exists
```

### Commands

```text
producer insertion order
invocation-local staging
discard on recoverable failed invocation
aborted-run deferred cleanup
deferred visibility
fail-stop application
framework-command operation atomicity
opaque-command partial-failure semantics
```

### Scheduling

```text
labels and sets
semantic precedence
cycle rejection
access conflict distinct from semantic order
required deferred visibility
unresolved-target diagnostics
canonical serial reference execution
ambiguity diagnostics
```

## Performance baseline

C9 SHOULD characterize at least spawn/despawn, component insert/remove/read/write,
resource access, query iteration/filtering, change detection, system dispatch,
schedule preparation, and deferred command production/application.

Where useful, also record memory footprint, allocation behavior, archetype
transition cost, and secondary-index cost.

Future optimized work should compare against this baseline.

## Mechanical evidence

Applicable safety/correctness evidence SHOULD include unit/integration tests,
compile-pass/fail tests, Miri, sanitizers where relevant, property tests, exhaustion
boundary tests, schedule permutation/fuzz tests, worker-count equivalence tests,
and downstream public conformance.

Claims MUST NOT exceed maintained evidence.

---

# 20. Recommended post-C9 evolution

After C8/C9 establish the clean standalone baseline, the strongest coherent
sequence is:

```text
E1  normalized reason-carrying schedule inspection
E2  strict ambiguity checking
E3  schedule permutation/fuzz testing
E4  deterministic observation projections
E5  thread-transferability model
E6  parallel system executor
E7  serial/parallel successful-reference equivalence
E8  parallel query execution
E9  deterministic parallel Commands where needed
E10 deterministic reductions
E11 contiguous query / SIMD expert capability
```

In parallel, consumer-proven external state work may proceed:

```text
S1  prove save/network/editor extraction needs
S2  define minimal ECS-neutral state traversal
S3  define Entity correspondence/remapping
S4  prove typed/reflected adapters
S5  add codec/schema conveniences only in the correct owner
S6  consider optional ergonomic serialization integrations
```

More speculative World forks/patches, structural invariant engines, relationships,
or incremental systems should follow only after the normalized core and execution
proofs are stable.

The intended differentiator is not feature count. It is:

> simulation correctness and optimized execution equivalence are mechanically
> inspectable.

---

# 21. Core invariants

A conformant RunenECS ultimately preserves:

```text
I1
A foreign Entity cannot silently alias local World state.

I2
Correctness-relevant runtime identity exhaustion cannot produce aliasing.

I3
Runtime-local identity is not portable identity.

I4
Storage representation is not public ECS meaning.

I5
Failed access does not fabricate successful mutation observation.

I6
Loss of incremental evidence cannot produce a false "unchanged" result.

I7
A documented atomic framework structural operation does not partially commit.

I8
Commands are ordered fail-stop deferred work, not implicit transactions.

I9
Opaque custom deferred commands do not receive false atomicity/effect guarantees.

I10
Direct access and deferred effects are distinct facts.

I11
Access incompatibility never creates semantic precedence.

I12
Physical serialization does not create semantic precedence.

I13
Ordinary explicit semantic ordering provides required deferred visibility.

I14
An explicit expert order-only form may remain separately expressible.

I15
Stages/waves/batches are derived executor facts, not semantic authority.

I16
The serial executor defines inter-system/deferred reference behavior.

I17
Serial execution alone does not promise determinism of arbitrary system code.

I18
Default query iteration order is unspecified.

I19
Parallel/SIMD partition boundaries are non-semantic.

I20
Worker completion timing cannot define deterministic command publication.

I21
Thread transferability is distinct from access compatibility.

I22
Component/Resource are not globally forced to Send + Sync solely for parallelism.

I23
Aborted deferred state cannot leak into a later schedule run.

I24
Runtime reflection is not portable schema.

I25
Cross-World state transfer requires explicit Entity correspondence.

I26
Context-free raw Entity serialization is not portable state transfer.

I27
Networking/save/editor adapters can consume ECS state without private storage
reach-through while their policy remains separately owned.

I28
Important derived scheduler decisions are explainable by their true semantic
cause.
```

---

# 22. Acceptance gate

Before this design moves to `accepted/`, the ECS owner must explicitly ratify the
following decisions:

1. storage layout is not semantic authority;
2. `Entity` remains World-scoped runtime identity;
3. World-scope exhaustion follows the accepted C1 terminal non-reuse contract;
4. other correctness-relevant runtime identities do not silently saturate/alias;
5. public registry ordinals survive only with proven need and owner-safe semantics;
6. failed mutable access cannot emit fabricated change observations;
7. failed resource lookup does not silently mutate registration unless documented;
8. current freshness metadata and retained full history remain separate concerns;
9. lost incremental evidence broadens invalidation rather than producing false
   negatives;
10. full journals require explicit retention/history-loss semantics and real
    justification;
11. component removal is not semantically "orphaning";
12. removal observation will not permanently depend on physical stage/wave lifetime;
13. framework-known and opaque deferred commands have different knowledge and
    atomicity guarantees;
14. Commands remain ordered fail-stop buffers rather than implicit transactions;
15. semantic precedence and access incompatibility remain distinct;
16. ordinary `before`/`after`/`chain` ordering guarantees deferred visibility;
17. an explicit expert order-only form may later exist;
18. access-hazard serialization cannot invent semantic order/visibility guarantees;
19. `DeferredApplyBoundary` is ECS-owned while product publication is not;
20. stages/waves/batches remain executor details;
21. serial execution remains the inter-system/deferred correctness oracle;
22. serial execution alone does not establish whole-program determinism;
23. strict determinism additionally constrains order-sensitive iteration,
    reductions, explicit inputs, and relevant external effects;
24. default query order remains unspecified;
25. Component/Resource are not globally made `Send + Sync` solely for parallelism;
26. thread transferability is modeled separately;
27. optimized execution distinguishes successful-reference equivalence from the
    stronger failure-prefix equivalence;
28. parallel command order cannot depend on worker completion;
29. SIMD/contiguous spans are explicit capabilities without making physical chunks
    semantic;
30. cross-World Entity correspondence/remapping is valid ECS-neutral integration
    functionality when consumer-proven;
31. portable schema, wire/save formats, and stable external IDs remain outside ECS;
32. RunenECS exposes enough neutral state access for network/save adapters without
    private reach-through;
33. independent World forks preserve distinct World identity and explicit Entity
    correspondence;
34. C8 remains a bounded ownership repair;
35. C9 explicitly resolves current semantic/public-surface defects before external
    stabilization;
36. current public conveniences are not automatically grandfathered into the
    standalone API;
37. the active wave-centric parallel design is superseded or rewritten against
    this normalized model before parallel implementation becomes authoritative;
38. stale ECS documentation is reconciled before standalone release.

## Acceptance stop conditions

Do not promote this design if C8's completed census proves that a maintained
consumer intentionally requires access-conflict-generated physical stage ordering
as semantic behavior without an explicit semantic dependency.

Do not promote it if Engine publication cannot be separated from ECS semantics
without inventing a new generic cross-domain scheduler.

Do not weaken World/Entity identity safety to simplify persistence, networking, or
forking; use explicit correspondence/remapping instead.

---

# 23. Post-extraction audit

After C8, C9, and the separately accepted clean extraction/cutover make
`dornglut/runen-ecs` the sole framework source authority, the standalone repository
should run a fresh semantic conformance, public-surface, capability-readiness, and
performance audit against then-current accepted source rather than trusting this
pre-extraction review indefinitely.

The currently recorded downstream audit is:

- [`dornglut/runen-ecs#1 — Post-extraction semantic conformance, capability, and public-surface audit`](https://github.com/dornglut/runen-ecs/issues/1)

That issue is evidence/planning for a future read-only audit. It MUST NOT be used to
defer C8/C9 correctness work that belongs before extraction.
