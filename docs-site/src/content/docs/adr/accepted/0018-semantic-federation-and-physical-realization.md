---
title: Semantic Federation and Physical Realization
description: Accepted positive meta-architecture for reasoning across independent Runen authorities without imposing a universal runtime, storage model, executable IR, identity system, or physical representation.
status: accepted
owner: workspace
layer: architecture
canonical: true
last_reviewed: 2026-08-12
related_adrs:
  - ./0014-repository-family-extraction-boundaries.md
  - ./0015-separate-gpu-execution-from-rendering.md
  - ./0017-cross-authority-consistency-and-graph-semantics.md
related_docs:
  - ../../architecture/repository-family-architecture.md
  - ../../guidelines/authority-centered-boundary-architecture.md
  - ../../guidelines/domain-program-architecture-pattern.md
  - ../../design/active/runenwerk-domain-workbench-north-star.md
  - ../../design/active/runengpu-architecture-design.md
  - ../../design/accepted/runenrender-decomposition-design.md
  - ../../design/active/runenecs-extraction-boundary-design.md
  - ../../reports/investigations/2026-08-12-semantic-federation-and-inspection-provenance.md
---

# ADR 0018: Semantic Federation and Physical Realization

## Decision

Runenwerk adopts a positive cross-domain meta-architecture based on **semantic
federation rather than runtime centralization**.

The short form is:

```text
One semantic grammar.
Many independent authorities.
Explicit typed contracts and relationships.
Owner-local versions; consumer-owned admission.
Semantic meaning independent of physical realization.
Specialized execution.
Optional federated inspection.
```

The shared semantic grammar exists to make heterogeneous systems easier to reason
about, integrate, inspect, explain, and evolve. It is **not** a mandatory runtime
object model.

Runenwerk and the Runen framework family use six recurring architecture questions:

```text
AUTHORITY
  Who owns the semantic invariant?

CONTRACT
  What typed semantic boundary is exposed or accepted?

OPERATION / RELATIONSHIP
  How do contracts, values, or authorities relate or transition?

VALIDITY / PROVENANCE
  Under what owner-local context is the contract meaningful,
  compatible, and traceable?

REALIZATION
  How is semantic meaning represented, retained, located,
  or accelerated physically?

EFFECT
  Where does authoritative or external state actually change?
```

These are questions and architectural dimensions. They do not authorize a
`MetaContract`, `MetaOperation`, `MetaNode`, `RunenObject`, universal registry,
universal graph, universal database, or another generic runtime hierarchy.

ADR 0017 remains authoritative for ownership, cross-authority consistency,
incremental correctness, graph semantics, capability terminology, shared extraction,
and progressive disclosure. ADR 0018 supplies the **positive reasoning and inspection
model** that the old Domain Workbench Meta Kernel was trying to provide without
reviving its pre-authorized shared implementation inventory.

## Why this decision exists

ADR 0014 and ADR 0017 intentionally keep independently useful frameworks autonomous:

```text
RunenUI
RunenSDF
RunenSpatial
RunenECS
RunenRender
RunenGPU
other domain owners
```

Each authority owns the identities, invariants, revisions, data structures,
algorithms, and execution semantics that belong to its domain. That independence is
necessary for clean ownership and long-term framework reuse.

Independence alone, however, does not answer the Workbench-level question:

> How can a human or tool reason coherently about information moving through several
> authorities without forcing those authorities onto one common runtime substrate?

The older Domain Workbench direction identified this real problem but proposed a
broad Meta Kernel containing generic identities, registries, schema/program/graph
infrastructure, compiler/evaluator/host contracts, artifact envelopes, fixtures,
proofs, and other shared mechanisms before structurally different domains had proven
that machinery.

That mechanism is rejected by ADR 0017's extraction gate. The useful goal remains.

The accepted answer is to standardize **semantic questions and boundary laws first**.
Any shared machine-readable inspection protocol or runtime mechanism remains a later,
separately justified extraction.

## Existing authority retained

This ADR does not change accepted owner contracts.

### ADR 0014 remains authoritative for repository-family ownership

Peer frameworks remain independently usable. They do not depend on Runenwerk merely
so Runenwerk can inspect them. Cross-framework translation and integration remain
Runenwerk-owned until an independently reusable adapter or protocol is separately
proven.

### ADR 0015 remains authoritative for rendering and GPU ownership

```text
Runenwerk integration and host policy
    -> RunenRender semantic image formation
        -> RunenGPU generic GPU execution
```

The semantic-federation model does not move renderer meaning into RunenGPU or GPU
execution authority into RunenRender.

### ADR 0017 remains the safety foundation

In particular:

```text
One semantic invariant set has one authority.
```

remains the core ownership rule. The new grammar helps describe boundaries around that
rule; it does not weaken it.

## The six-question semantic grammar

### Authority

Authority answers:

```text
Who decides whether this semantic state or value is valid?
Who owns the invariants that must hold?
```

Authority is semantic ownership, not physical possession.

Examples:

```text
RunenSDF
  owns signed-field mathematics and reference-query semantics

RunenUI mounted runtime
  owns mounted identity, lifecycle, focus, and interaction state

RunenRender scene store
  owns renderer-local semantic scene revisions

RunenGPU context
  owns admitted GPU realization and execution invariants
```

A database, file, GPU buffer, cache, thread, or process does not become semantic
authority merely because it physically contains a representation.

### Contract

A contract is an explicit typed semantic boundary that another component or authority
may consume, propose, or inspect.

Existing useful contract roles remain distinct, including:

```text
Command
Query
Event
Projection
Snapshot
Product
Contribution
Descriptor
Catalog
Plan
Work
Status
Diagnostic / Report
```

The contract may be an ordinary Rust value, immutable snapshot, query result, opaque
handle, stream, GPU-resident export, field evaluator, renderer paint product, or
another owner-defined representation.

No universal `Contract<T>` wrapper is required.

### Operation / relationship

An operation or relationship answers:

```text
What does this boundary do with its contracts?
How do two semantic values or authorities correspond?
```

Useful open architecture vocabulary includes:

```text
Observe
  read or consume an explicit contract without changing its owner

Propose
  ask an owning authority to consider a state change

Derive
  form new non-authoritative semantic meaning from admitted inputs

Adapt
  explicitly translate between different owner contracts

Admit
  decide whether already meaningful values, capabilities, and context
  are legal together for a particular consumer or execution

Realize
  produce a physical or execution representation that satisfies a
  semantic contract

Execute
  perform admitted work under an execution authority

Commit
  accept and publish an authoritative state transition
```

This is an **open vocabulary**, not a closed enum and not a universal pipeline. An
owner may use more precise domain terminology where it matters.

`Materialize` and `Retain` are not required to be semantic relationship kinds. They
primarily answer how long a derived result is retained and in what realization form.

`Reconcile` is not one edge. It is a temporal control pattern in which desired and
observed state can evolve independently:

```text
observe
  -> compare/decide
  -> propose/execute/effect
  -> later observe again
```

Pure deterministic formation must not be renamed reconciliation merely because one
value becomes another.

### Validity / provenance

Validity and provenance are applicable facts that explain why a contract can be used
or trusted for a purpose.

Depending on the owner and consumer, they may include:

```text
owner-local revision or generation
time, tick, sample, or interval
scope and coverage
completeness
freshness or bounded staleness
availability or residency
capability and accuracy
source lineage
identity correspondence
legal fallback
```

They are dimensions, not a universal envelope.

A field query may need capability and bounds. A renderer input may need source revision,
time interval, representation coverage, and accuracy. A GPU realization may need exact
context and device generation. A UI semantic publication may need surface identity and
semantic revision.

Each owner exposes only facts that are meaningful to its contracts.

### Realization

Realization answers:

```text
How is this semantic meaning physically represented or made executable here?
```

Examples include:

```text
ECS archetype columns
sparse sets
persistent tables
copy-on-write pages
BVHs or spatial indices
analytic SDF evaluators
sampled field grids
CPU vectors or SoA layouts
GPU buffers and textures
compiled programs
persisted artifacts
network payloads
remote services
```

Realization is not a synonym for semantic meaning and is not automatically public.

### Effect

Effect answers:

```text
Where does authoritative state or the external environment actually change?
```

Examples include:

```text
committing an authority transition
submitting GPU work
writing a file
starting an external import process
sending network data
creating or destroying native host resources
performing an application-owned state update
```

Effects remain explicit. `Derive`, `View`, `Projection`, and `Materialize` must not
become euphemisms for arbitrary hidden side effects.

## Semantic meaning is independent of physical realization

Runenwerk and peer frameworks use the following family-level law:

> **A semantic contract is not defined by its physical realization. A realization may
> constrain capability, precision, validity, locality, residency, lifetime, or cost,
> but it must not silently redefine the semantic contract it claims to realize.**

This law protects both correctness and optimization freedom.

### Consequences

A world-authoritative pose may correspond to a renderer-local transform and a
GPU-packed transform without sharing identity or authority.

An ECS implementation may use archetypes, dense columns, sparse sets, or another
storage strategy without making that physical organization the public component model.

A RunenRender scene snapshot may be backed by persistent tables, arenas, copy-on-write
pages, lazy views, or compiled dense tables without those choices redefining renderer
scene semantics.

A RunenGPU logical resource or program may have a context/device-generation-bound WGPU
realization without exposing WGPU as the semantic contract.

A signed field may be realized analytically, as a sampled grid, or through GPU code
only when the realization preserves or explicitly narrows the required numerical
capabilities.

A persisted artifact, materialized result, or GPU-resident representation does not
become source authority merely because it survives longer or is expensive to rebuild.

### Realization mismatch must be explicit

If a realization cannot preserve a claimed semantic property, it must do one of the
following according to the owner contract:

```text
expose a weaker or different capability
expose declared approximation or tolerance
perform an explicit semantics-preserving adaptation
reject the realization or consumer request
```

It must not silently continue under the stronger semantic label.

This applies particularly to:

```text
SDF exact-distance vs conservative/approximate behavior
renderer representation accuracy/error contracts
numerical precision modes
GPU feature/format/alignment support
partial or stale residency
lossy serialization or transport
```

## Versions are local; compatibility is contextual

ADR 0017's cross-authority consistency decision is retained and compressed into three
rules:

```text
Versions are owner-local.
Compatibility is contextual.
Admission is consumer-owned.
```

There is no universal Runen revision, frame transaction, world snapshot, or clock that
makes every framework value mutually current.

A consumer combining several authorities admits only the facts it needs for its own
purpose.

For example, one render invocation might consume:

```text
world/source revision
renderer scene revision
simulation interval
representation generation
GPU device generation
```

while network publication may admit a different cut over world, ECS, interest, and
history state.

The fact that both consumers can be drawn as joins over related values does not make
their compatibility policies identical.

## Inspection correlation is not semantic admission

Runenwerk adopts the following explicit safety law:

> **Inspection correlation or join is not semantic runtime admission.**

A Workbench may correlate:

```text
WorldEntity
  -> RenderObject
      -> GpuResource
```

through explicit lineage or correspondence facts for debugging.

That relationship does not prove that the *currently displayed* revisions,
generations, time intervals, coverage, capabilities, or freshness facts form a legal
runtime input set.

Only the owning consumer or integration boundary can make that admission decision.

This rule is required so future table/query ergonomics do not accidentally introduce a
global consistency model.

## View, snapshot, product, materialization, index, cache, and realization

These concepts answer different questions and may legitimately compose.

### Projection / View

A projection or view is a derived consumer-oriented read model.

It answers:

```text
How can this information be read for this purpose?
```

It does not imply physical retention.

### Snapshot

A snapshot is an immutable view of one owner-local state or revision cut.

It answers:

```text
Which owner state am I observing immutably?
```

A snapshot may be an authoritative read contract when its owner defines it that way.
It is not automatically a cache.

### Product

A product is an owner-defined semantic publication.

It answers:

```text
What formed result has this owner intentionally published for consumers?
```

A product may carry lineage, scope, freshness, capability, consumer-class, fallback,
certification, or rebuild semantics according to its owner.

A product is not automatically discardable and is not defined by whether it is
materialized.

### Materialization

Materialization answers:

```text
Is a derived result retained rather than recomputed on demand?
```

Materialization may be transient, memory-resident, GPU-resident, persisted, or another
owner-defined form.

Materializing a value does not imply `Apply`, `Commit`, or transfer of semantic
authority to another domain.

### Index

An index answers:

```text
What retained structure accelerates access to a contract, view, or product?
```

An index may be expensive and correctness-sensitive, but it does not become semantic
source authority merely because consumers rely on its speed.

### Cache

A cache is a discardable or reconstructable reuse optimization.

A cache hit may change cost. It must not silently change semantic meaning. ADR 0017's
incremental and stale-data rules continue to apply.

### Realization

A realization answers how semantic meaning is physically represented or made
executable. A materialization, index, or cache may themselves have one or more
realizations.

### Runtime Artifact

`Runtime Artifact` remains an owner-specific Domain Program term where useful. It is
not promoted into a new universal authority layer. Depending on the owner, a runtime
artifact may be a Product or another optimized formed output.

### Orthogonal composition

A single value may legitimately be several things at once, for example:

```text
renderer-owned Product
+ immutable Snapshot
+ materialized
+ indexed
+ GPU-resident realization
```

Architecture documentation should state the relevant roles instead of inventing a
new universal stage name for every combination.

## Change evidence and incremental correctness

ADR 0017's incremental correctness law remains authoritative:

> For the same admitted semantic input set, incremental evaluation must be
> observationally equivalent to clean/full evaluation under the owning domain's
> declared equality or tolerance semantics.

ADR 0018 adds the following operational formulation:

> **Change evidence is a correctness-relevant optimization contract; clean semantic
> reconstruction or full resynchronization remains the safety reference.**

Therefore:

```text
valid narrow change evidence
  -> narrow incremental work may be performed

missing, pruned, incompatible, unknown, or untrusted evidence
  -> widen invalidation, clean rebuild, or full resynchronization as required
```

This does not require one family-wide `Delta<T>`, dependency database, retained journal,
or incremental evaluator.

Owner-local mechanisms may include:

```text
dirty sets
change sets
revision checks
memoized queries
tracked dependency graphs
incremental dataflow
GPU-side incremental algorithms
full replacement
```

provided the owner proves the required semantic behavior.

## Inspection projections are not semantic storage shapes

Runenwerk adopts the following Workbench-level principle:

> **An owner contract may expose zero or more optional inspection projections. The
> projection shape does not define the owner's semantic or storage ontology.**

Potential future projection classes include:

```text
Record
Table
Tree
Graph
Timeline
Field
Image
Resource
Text / Report
Opaque / Custom
```

These names are directional tooling vocabulary only. This ADR does not create Rust
types or a stable schema for them.

### Multiple projections may describe one contract

For example, one renderer scene snapshot may eventually be inspectable as:

```text
object/property tables
relationship graphs
spatial/custom scene views
revision/change timelines
```

One UI publication may be inspectable as:

```text
mounted or semantic trees
node tables
focus/relationship graphs
revision timelines
```

One signed-field contract may offer:

```text
field sampling view
capability record
query diagnostics report
```

One GPU context may offer:

```text
resource table
work/access graph
submission timeline
pressure/memory view
```

The owner does not have to store its data in the inspector's projection format.

## Workbench semantic federation boundary

Runenwerk may target a federated Workbench inspection experience without creating a
shared runtime authority.

The conceptual dependency direction is:

```text
peer frameworks and domain owners
    -> explicit Runenwerk integration/inspection adapters
        -> optional Workbench inspection session
```

Initially, any concrete inspection adapters should remain Runenwerk-local unless a
separate Level-B extraction decision proves a peer-neutral reusable protocol.

### The Workbench may eventually own

Subject to later implementation authority, the product-level Workbench may own:

```text
inspection-session-local handles
composition/discovery of explicitly supplied adapters
projection selection and UI presentation
filter/sort/group/diff over projections that support those operations
provenance and correspondence navigation
cross-owner correlation display
revision/time/pressure visualization
bounded paging and inspection scheduling
```

These are tooling/product responsibilities.

### The Workbench must not own

```text
domain semantic identities
domain revisions or clocks
domain state or mutation authority
runtime semantic admission
GPU resource authority
owner payload storage
universal serialization
universal query execution over private owner state
```

### Inspection identity

Workbench navigation needs identities, but they are not a new global object system.

The law is:

```text
Workbench identity is inspection-session-local.
Owner identity remains owner-local.
Cross-owner correspondence is explicit lineage or provenance.
```

When the Workbench displays an owner identity, it conceptually identifies:

```text
(owner, owner-local identity)
```

without promoting that pair into a stable family-wide persistence or wire identity.

## Inspection must remain optional, lazy, and bounded

Inspectability is valuable only if it does not force every runtime onto one costly
representation.

When inspection is disabled, the target is:

```text
no mandatory payload copies
no mandatory global graph construction
no mandatory per-frame registration walk
no mandatory CPU readback of GPU-resident data
no eager field or large-table materialization
no unbounded history retention
```

When enabled, inspection should be able to remain:

```text
lazy
paged or ranged
bounded
pressure-aware
copy-aware
location/device-aware
explicit about dropped or incomplete evidence
```

Examples:

- A field inspector samples a requested scope/resolution rather than requiring a full
  dense grid.
- A GPU inspector may expose descriptor, lineage, affinity, memory, pressure, and
  execution facts without reading resource payload bytes back to the CPU.
- Payload readback is a separate explicit owner capability/request.
- Large table-like projections are paged or ranged.
- Timelines and traces have bounded retention and explicit dropped-prefix evidence.

This ADR defines the design target, not measured performance numbers. Implementation
benchmarks require separate authority.

## Metadata must not become parallel truth

Runenwerk adopts the following doctrine:

> **Do not maintain a second hand-authored description of facts already canonical in
> the typed owner API when those facts can be reused, derived, or generated safely.**

Preferred order:

```text
1. reuse canonical owner contracts and descriptors
2. derive or generate inspection facts where justified
3. use Runenwerk sidecar adapters for cross-framework interpretation
4. duplicate semantic metadata manually only when no canonical source can express it
```

A manually duplicated fact requires an explicit consistency owner and validation path.

This decision does not mandate macros, reflection, proc-macros, code generation, or a
particular Rust mechanism.

## Extraction levels

ADR 0017's shared-extraction gate remains authoritative. ADR 0018 makes the acceptance
bar more explicit by distinguishing three levels.

### Level A — conceptual law and vocabulary

Level A creates no runtime or API dependency.

A concept may be accepted when:

1. several structurally different domains benefit from it;
2. it answers a recurring architecture question;
3. it does not steal semantic meaning from the proving domains;
4. it does not imply one runtime representation;
5. the terminology reduces ambiguity or materially improves cross-domain reasoning.

This ADR is primarily a Level-A decision.

### Level B — interoperability, reflection, or inspection contract

Level B creates API, versioning, dependency, and compatibility cost.

Before extraction require at least:

1. two structurally different concrete proving adapters or owners;
2. concrete repeated adapter/inspection machinery or maintenance burden;
3. no proving-domain semantic branches in the shared contract;
4. optional/progressive-disclosure participation;
5. explicit identity, lifetime, pressure, and evolution semantics;
6. characterized allocation, copy, locking, serialization, and cognitive costs where
   relevant;
7. independence if either proving domain disappears;
8. preserved repository-family dependency direction;
9. a separate accepted extraction design.

A possible future peer-neutral inspection projection protocol would be Level B.

This ADR does not authorize one.

### Level C — shared runtime mechanism

Examples include:

```text
query engine
incremental database
dataflow runtime
shared relation store
generic optimizer
shared scheduler/executor
```

Level C requires stronger proof than Level B:

1. multiple real domains already implement materially identical machinery;
2. measured runtime or maintenance burden justifies extraction;
3. time, storage, scheduling, retention, failure, and pressure semantics are genuinely
   shared;
4. hot-path cost is benchmarked where relevant;
5. no owner's semantics are weakened or distorted to fit;
6. ordinary owner APIs remain understandable without the runtime;
7. a separate accepted runtime/extraction decision authorizes the exact mechanism.

Current evidence authorizes **no Level-C semantic-federation runtime**.

## Domain Program remains a specialized pattern

Domain Program remains a strong pattern for domains that need durable, authored,
versioned, inspectable, migratable, compilable, or evaluable semantic intent.

Typical shape:

```text
authored source
  -> normalization / validation
  -> Domain Program
  -> compiler or evaluator
  -> products / runtime artifacts
  -> host or execution integration
```

ADR 0018 clarifies that Domain Program is **not the universal platform ontology**.

The following need not become Domain Programs merely to participate in Runenwerk:

```text
GPU resources
input events
mounted UI runtime state
spatial availability
renderer scene snapshots
network streams
query results
scheduler readiness state
```

Owners continue to use Domain Program where its actual lifecycle and semantics justify
it.

## Inspiration and provenance

The detailed primary-source research behind this decision is recorded in:

[`2026-08-12-semantic-federation-and-inspection-provenance.md`](../../reports/investigations/2026-08-12-semantic-federation-and-inspection-provenance.md)

The decision is a synthesis rather than an assertion that the individual mechanisms
are novel.

Key external pressures include:

- MLIR for domain-specific semantics, generic interfaces, and explicit lowering;
- PostgreSQL for MVCC/snapshot thinking and the logical-view/materialization split;
- Materialize for the separation of views, maintained materializations, and indexes;
- OpenUSD/Hydra Scene Index for queryable views, change notices, filtering chains, and
  inspection;
- Salsa and Bazel Skyframe for tracked dependencies and incremental correctness;
- Differential Dataflow/Timely for explicit change/time semantics and maintained
  arrangements;
- Apache Arrow for explicit physical-layout tradeoffs and narrow zero-copy CPU/device
  interchange;
- Substrait for compute semantics separated from physical representation and cautious
  extension points;
- Bevy ECS for typed APIs that carry access information without requiring a separately
  authored graph;
- Unreal RDG for the optimization power of deliberately specialized graph semantics;
- Kubernetes controllers for genuine desired/observed reconciliation;
- Reactive Streams for narrow bounded asynchronous interoperability;
- seL4 for the warning that an authority-bearing capability is stronger than a support
  fact.

Runenwerk adopts selected architectural lessons from these systems. It does not adopt
their full runtime models.

## Rejected alternatives

### ADR 0017 only, with no positive meta-model

Rejected as the final north star.

ADR 0017 is an effective safety doctrine but intentionally does not provide a complete
positive Workbench reasoning model. Leaving the architecture there would preserve low
abstraction risk but leave cross-domain inspection and human reasoning unnecessarily
owner-by-owner.

### Resurrect the old broad Meta Kernel

Rejected.

It would pre-authorize shared IDs, registries, schemas, graph/program infrastructure,
compilers/evaluators, artifacts, and other machinery before structurally different
domains prove common implementation burden. It would also create gravity toward a
RunenCore-like dependency.

### Universal relational/database ontology

Rejected as universal.

Relations, views, joins, indexes, and materialization are highly useful for some
systems and Workbench projections, but fields, retained UI hierarchy, GPU work/access
semantics, effects, controllers, and specialized runtime state are not naturally one
relational ontology.

Database techniques remain valid owner-local implementation choices and Workbench
inspection techniques.

### Universal dataflow runtime

Rejected as universal.

Serious incremental/iterative dataflow introduces logical time, progress, retained
indices, compaction, scheduling, and pressure semantics. Domains that need those
semantics may adopt suitable mechanisms locally; ordinary Runen operations should not
pay for them.

### Universal MLIR-like executable IR

Rejected as universal.

MLIR succeeds because its participants are compiler representations. Runenwerk also
contains mutable authorities, retained runtime state, interaction loops, external IO,
controllers, streaming, and GPU execution. The family borrows dialect/interface/lowering
principles without forcing all semantics into one compiler IR.

### Universal ECS ontology

Rejected.

ECS is appropriate for data-oriented mutable world/simulation state. It does not become
the semantic owner of assets, render image formation, GPU resources, UI hierarchy,
compiler/source programs, or networking merely because those systems can be encoded in
components.

### Universal scene graph or scene index

Rejected.

Scene-index architecture is powerful for graphics and inspection but too semantically
narrow to own fields, network publication, schedulers, arbitrary assets, GPU execution,
or application interaction.

### Universal event sourcing

Rejected.

Append-only accepted-fact history is valuable for selected audit/replay domains but is
inappropriate as a mandatory model for transient high-frequency runtime state and pure
derived computation.

## Fitness functions

This decision remains healthy when:

- every semantic invariant set still has one identifiable owner;
- peer frameworks remain independently usable;
- the six-question grammar improves explanation without creating mandatory runtime
  wrappers;
- local domain terminology remains primary;
- semantic contracts can change physical realization without silently changing
  meaning;
- realization mismatches expose capability/tolerance/adaptation/rejection explicitly;
- owner-local revisions remain independent;
- runtime compatibility is admitted by the actual consumer rather than inferred from
  Workbench joins;
- Product, Snapshot, View, Materialization, Index, Cache, and Realization remain
  distinguishable;
- incremental paths retain clean/full or full-resync safety behavior;
- optional inspection does not require universal storage, identity, CPU readback, or
  eager materialization;
- metadata remains derived from or tied to canonical owner contracts where possible;
- Level B or C infrastructure is not created without the full extraction evidence;
- Domain Program remains useful where justified without becoming mandatory for all
  domains;
- ordinary typed APIs remain the common path.

## Consequences

- Runenwerk gains a positive meta-architecture without a new shared runtime.
- Database/table reasoning becomes first-class where it clarifies a domain or
  Workbench projection, without making the platform a database.
- A future Workbench can present tables, trees, graphs, timelines, fields, images,
  resources, provenance, revisions, and pressure as different projections over owner
  contracts.
- Physical optimization remains unconstrained by one universal data layout.
- GPU-resident and field-native representations can stay native when inspection does
  not require payload transfer.
- Cross-authority correlation becomes more inspectable while remaining separate from
  semantic consistency/admission.
- The old Domain Workbench Meta Kernel can be retired in the later #205 architecture
  spine rewrite without losing its useful goal of one coherent platform mental model.

## Non-goals

This decision does not create or authorize:

```text
runen-core
runen-meta
runen-db
runen-flow
runen-dataflow
universal Meta IR
universal Relation<T>
universal Transform<I, O>
universal ObjectId
universal Revision
universal transaction or snapshot
universal query language
universal graph runtime
universal scheduler or executor
universal event bus
universal serialization format
universal incremental evaluator
Arrow or Substrait as a required dependency or format
```

It does not change current RunenUI, RunenSpatial, RunenSDF, RunenRender, RunenGPU,
RunenECS, asset, networking, scheduler, or product semantics.

It does not perform the broad documentation-spine cleanup owned by issue #205.

## Delivery and follow-up

Issue #244 owns this bounded architecture decision. Issue #243 contains the detailed
owner pressure tests, counterexamples, risks, and research synthesis.

After this ADR is accepted and accepted-main validation succeeds, issue #205 may use it
to perform a separately reviewable rewrite of the old Domain Workbench north star and
architecture spine.

Any concrete Workbench inspection protocol or adapter proof remains separately
unauthorized. The first implementation investigation, if later justified, should start
as a Runenwerk-local, read-only experiment over several deliberately different owners
rather than as a new framework or shared runtime.
