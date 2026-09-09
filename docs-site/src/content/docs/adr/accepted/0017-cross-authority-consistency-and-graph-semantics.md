---
title: Cross-Authority Consistency and Graph Semantics
description: Accepted cross-domain laws for semantic authority, foreign reads, compatible input sets, incremental correctness, graph meanings, capability vocabulary, shared extraction, and progressive disclosure.
status: accepted
owner: workspace
layer: architecture
canonical: true
last_reviewed: 2026-08-11
related_adrs:
  - ./0014-repository-family-extraction-boundaries.md
  - ./0015-separate-gpu-execution-from-rendering.md
related_docs:
  - ../../architecture/repository-family-architecture.md
  - ../../guidelines/authority-centered-boundary-architecture.md
  - ../../guidelines/domain-program-architecture-pattern.md
  - ../../guidelines/runenwerk-architecture.md
  - ../../design/active/runenwerk-domain-workbench-north-star.md
  - ../../design/active/semantic-graph-ir-and-compilation-design.md
  - ../../design/active/runengpu-architecture-design.md
  - ../../design/accepted/runenrender-decomposition-design.md
  - ../../design/active/runenecs-extraction-boundary-design.md
  - ../../design/accepted/sdf-first-field-world-platform-design.md
---

# ADR 0017: Cross-Authority Consistency and Graph Semantics

## Decision

Runenwerk and the Runen framework family use the following cross-domain law:

```text
One semantic invariant set has one authority.
```

This replaces weaker wording such as "one concept has one owner" where that wording
would incorrectly forbid legitimate layered representations with different invariants.
For example, all of these may coexist without sharing identity or authority:

```text
world-authoritative pose
renderer-local transform
GPU-packed transform
```

Correspondence between such values is explicit through owner-defined source maps,
provenance, adapters, or other typed relationships. It is not inferred from a
universal object ID.

This ADR also establishes the family-wide rules for cross-authority reads and
consistency, incremental correctness, graph semantics, capability vocabulary,
shared extraction, and progressive disclosure.

It does not create a new runtime, crate, repository, graph interpreter, store,
transaction model, identity system, or public generic wrapper family.

## Existing authority retained

ADR 0014 remains authoritative for independent peer-framework ownership, explicit
adapters, clean cutover, framework-owned identities, and the rejection of a universal
`RunenCore` or shared meta-framework.

ADR 0015 remains authoritative for:

```text
Runenwerk integration and host policy
    -> RunenRender semantic image formation
        -> RunenGPU generic GPU execution
```

This decision does not change accepted RunenGPU, RunenRender, RunenECS, RunenUI,
RunenSpatial, RunenSDF, field/product, scheduler, or graph owner contracts.

## Cross-authority read law

A foreign authority must not reach through another authority's private mutable state.
It consumes an explicit owner contract.

The concrete contract remains owner-defined and may be a:

```text
query result
immutable snapshot
formed product
prepared input
renderer-neutral paint output
typed GPU export/resource relationship
stream
status or diagnostic product
another explicit owner-defined value
```

`Observation` is therefore a conceptual relationship: one authority is reading an
explicit contract published by another authority. It is not a universal wrapper type,
a mandatory materialized snapshot, or a replacement for Query, Product, Projection,
Status, or another useful contract role.

Direct reads inside one authority are normal and do not require an observation wrapper.

## Cross-authority consistency law

There is no global Runenwerk transaction, world revision, or universal snapshot that
makes every framework value mutually current.

Each producer retains its own applicable consistency facts. A consumer or integration
boundary that combines several authorities must admit a semantically compatible input
set using only the facts required by that consumer.

Applicable facts may include:

```text
owner-local revision or generation
time, tick, or interval
scope and coverage
completeness
freshness or bounded staleness
availability or residency
provenance and source lineage
identity/correspondence mapping
legal fallback
```

These are dimensions, not a mandatory common record or enum.

Useful policy families include:

```text
exact revision/time
explicit compatible cut or set
bounded-stale admission
intentionally independent latest
```

A concrete owner may define a typed policy when a real consumer needs one. This ADR
does not pre-create a universal policy type.

If required inputs are incompatible, the responsible boundary must do one of the
following according to explicit policy:

```text
wait within a bound
request or rebuild fresher data
retry
perform a clean/full resynchronization
use an explicitly legal fallback
reject with structured facts
```

Silently taking the latest independently available value from every producer is not a
consistency model.

## Flow roles remain distinct

The following roles are related but not interchangeable:

```text
Authority
  owns a semantic invariant set and decides validity

Command
  requests a state change

Query
  requests an owner-defined read

Event
  reports an accepted fact that happened

Observation
  conceptual foreign-authority read relationship

Projection
  derived read model, optionally retained or cached

Snapshot
  immutable view of one owner state/revision; it may be an authoritative read

Product
  owner-defined formed or derived semantic output with explicit validity/lineage

Cache
  discardable derived state whose reuse changes cost, not meaning

Contribution
  producer input to another owner's composition process

Descriptor
  normalized inert declaration

Catalog
  read-only inspected set of validated descriptors

Plan
  owner-specific declaration of intended work or selection

Work
  executable unit admitted to an execution authority

Status
  observed lifecycle or current condition

Diagnostic / Report
  explanation and evidence; not hidden control flow
```

Additional owner-specific terms remain valid where they have a distinct purpose.
Do not introduce universal `Observation<T>`, `Product<T>`, `Plan<T>`, or similar
wrappers merely to mirror this taxonomy.

## Ratification, admission, and reconciliation

These words are also distinct:

```text
ratification
  semantic owner acceptance of a proposed meaning/state transition where that domain
  requires an explicit acceptance boundary

admission
  acceptance that already meaningful work/data is compatible with a particular
  execution or consumption environment

reconciliation
  convergence between independently changing desired and observed state
```

Not every operation requires ratification. Deterministic pipelines should not be
renamed reconciliation merely because they transform data.

## Incremental correctness law

For the same admitted semantic input set, incremental evaluation must be
observationally equivalent to clean/full evaluation under the owning domain's declared
equality or tolerance semantics.

Consequences:

1. Missing, unknown, or untrusted narrow change evidence must broaden invalidation or
   force clean recomputation/full resynchronization.
2. Change evidence is contextual to its owner revision, generation, scope, or another
   explicit validity boundary.
3. Stale cache reuse is not a quality-degradation mechanism unless an owning semantic
   contract explicitly defines bounded-stale input as legal.
4. Cache hits change cost, not semantic meaning.
5. Retained snapshots, histories, revisions, and incremental journals require explicit
   lifetime, pressure, and pruning policy.
6. Losing incremental history must have an explicit fallback such as clean rebuild or
   full resynchronization when correctness requires it.
7. This law does not authorize a universal incremental database, query engine, or
   retained dependency runtime.

Current concrete evidence includes RunenRender's requirement that full and incremental
scene construction produce the same semantic snapshot and networking's full-resync
fallback when required history is missing or pruned.

## Graph taxonomy

The family uses multiple graph-shaped structures because their semantics differ.
Useful graph/topology classes include:

```text
structural/source graph
  authored or neutral structure and connectivity

semantic/program IR graph
  domain-owned semantic relationships or constrained intent

product dependency graph
  semantic product prerequisites and lineage

incremental dependency graph
  invalidation/recomputation dependencies

scheduler/readiness graph
  execution eligibility and ordering constraints

render planning/execution graph
  renderer-owned planning/lowering structure

RunenGPU work graph
  GPU resource access, hazards, and generic execution work

retained runtime topology
  persistent containment/identity structures such as a UI mounted tree or spatial
  hierarchy/index; often not a dependency graph

reconciliation loop
  desired/observed control loop; not a dependency graph merely because relationships
  can be drawn as edges
```

The governing law is:

```text
semantic dependency
!= execution ordering
!= resource hazard
!= invalidation dependency
!= containment/hierarchy
```

One implementation may derive one class from another, but that derivation is an
explicit owner contract. Structural similarity alone does not authorize a shared
runtime.

`domain/graph` therefore remains a structural graph substrate. Semantic graph meaning
stays in the owning domain. RunenGPU G3 remains the owner of its GPU work/access graph.
Schedulers, RunenRender, RunenUI, and RunenSpatial retain their existing graph or
topology semantics.

## Feedback law

An acyclic graph must not hide semantic feedback as an undeclared back-edge.
Feedback crosses an explicit temporal, iterative, reconciliation, interaction, or
distributed boundary.

Useful feedback classes include:

```text
temporal feedback
  tick/frame/revision N produces input for N+1

bounded or fixed-point iteration
  an owner explicitly defines convergence, iteration budget, and failure outcome

reconciliation
  desired state is compared with independently changing observed state

interaction loop
  input changes state, state changes later expression, later input observes that result

distributed prediction/correction
  local prediction is later compared with accepted remote/authority state
```

Cycle policy belongs to the owner that defines the graph's meaning.

## Capability, requirement, policy, and authority

The family must not use one overloaded `Capability` concept for unrelated concerns.
Distinguish:

```text
support/capability fact
  the implementation or environment can support X

requirement
  a consumer needs X for the requested operation

host/security policy decision
  X is allowed in this context

authority
  the holder may control or mutate X
```

A declaration of support is not permission. A requirement is not authority.
Concrete frameworks may continue to define their own capability types; no universal
family capability enum or registry is authorized.

## Shared-extraction gate

Shared infrastructure is extracted only after concrete repetition proves the boundary.
Before extraction, require all of the following:

1. At least two structurally different domains prove the same invariant or operation
   shape.
2. Repeated implementation or maintenance burden is concrete rather than hypothetical.
3. The proposed shared contract contains no proving-domain semantic branches, enum
   cases, or vocabulary.
4. The contract remains meaningful if either proving domain disappears.
5. Dependency direction remains valid; independent peer frameworks are not forced onto
   Runenwerk-owned meta-infrastructure.
6. The ordinary owner API remains understandable without first learning the substrate.
7. Runtime, serialization, versioning, memory, and cognitive cost are characterized
   where relevant.
8. A separate accepted extraction design authorizes exactly the repeated primitive.

The sequence remains:

```text
design locally
-> prove one domain
-> prove a structurally different second domain
-> characterize repeated cost and ownership
-> extract only the repeated neutral primitive through a separate decision
```

This gate explicitly rejects pre-authorizing a `foundation/meta`, universal identity
model, generic DomainProgram runtime, universal registry, universal graph interpreter,
generic compiler/evaluator framework, or another shared substrate inventory before
proof.

## Progressive disclosure

Common-path usability is a family-wide architecture requirement:

```text
ordinary path
  direct typed high-level API

inspection/tooling path
  normalized forms, source maps, graphs/plans, diagnostics, provenance

expert path
  explicit lower-level control only for proven consumers
```

Intermediate representations should remain inspectable without becoming mandatory
common-path ceremony.

This principle is already exemplified by RunenGPU's direct submission path versus its
explicit preparation/inspection path and should be applied by each owner where useful.
It does not require identical APIs across frameworks.

## Superseded conflicting claims

Until the broader documentation-authority cleanup in issue #205 performs structural
rewrites, this ADR has precedence over the following older generic claims where they
conflict:

| Older claim | Decision here |
|---|---|
| The 9-layer/multi-reality doctrine treats `Reality` categories as the primary universal platform ontology. | Formation, validity, realization, derivation, consumption, lifetime, and distribution are independent concerns owned by concrete domains. `Reality` may remain explanatory prose but is not a required family-wide type or ontology. |
| "No consumer observes authority directly" requires every consumer to use a declared observation frame. | Foreign authorities consume explicit owner contracts. A query, immutable snapshot, product, paint output, typed GPU relation, or stream may be that contract. No universal observation-frame type is required. |
| Runenwerk-local `UiProgram` is a settled current proving-domain implementation for shared Domain Program infrastructure. | Standalone RunenUI owns current UI architecture. Historical Runenwerk UI work is not sufficient cross-domain extraction proof. Issue #205 owns its broader legacy/adoption cleanup. |
| `RenderPlan` is a generic example name for a durable Domain Program. | Canonical RunenRender owns `RenderPlan` as the per-request device-independent plan produced by `RenderMethod` before `AdmittedRenderPlan`. Generic guidance must not assign a conflicting meaning. |
| A generic `RenderAuthority` owns GPU execution and render resource lifetime. | RunenRender owns semantic image formation; RunenGPU owns generic GPU execution and GPU resource lifetime according to ADR 0015. |
| The Domain Workbench Meta Kernel inventory is a pre-approved shared platform implementation target. | The vision may retain domain ownership, compiler-like formation, inspectability, and source lineage; every shared primitive remains conditional on the extraction gate in this ADR and ADR 0014. |

These precedence corrections are intentionally narrow. Issue #205 owns later rewrite,
merging, deletion, navigation, and RunenUI historical-material cleanup. This ADR does
not perform that documentation-spine work.

## Consequences

- Peer frameworks can maintain independent identities and revisions without inventing a
  global world transaction.
- Integrators must state what makes a multi-owner input set compatible rather than
  silently combining unrelated latest values.
- Snapshots and products can be authoritative reads or strict consumer truth where
  their owner defines that role; they are not automatically disposable caches.
- Incremental systems require a clean/full correctness path and explicit history
  pressure policy.
- Graph-shaped systems may share structural utilities only after repeated neutral
  pressure is proven; graph shape does not imply shared semantics or execution.
- Support, requirement, permission, and mutation authority remain distinguishable.
- High-level APIs remain the normal path even when lower-level plans/graphs are
  inspectable.
- RunenUI provides a concrete pressure test: transient authored elements, retained
  mounted runtime authority, semantic products, hit-test products, paint products, and
  trace are distinct even though they describe one application experience.
- RunenSpatial provides a concrete pressure test: neutral spatial availability does not
  become world-product readiness, renderer residency, or GPU realization.

## Rejected alternatives

Rejected:

- one global Runenwerk/world transaction or revision;
- universal `ObservationFrame<T>` or mandatory materialized snapshots;
- making Observation replace Query, Product, Projection, Status, or Diagnostic;
- universal object IDs to align peer frameworks;
- one shared product/residency/readiness state;
- one universal graph/runtime/dataflow engine;
- treating all cycles as ordinary graph back-edges;
- making ECS the platform ontology;
- treating capability declarations as permissions;
- creating shared infrastructure from one proving domain;
- forcing advanced inspection IR into common-path APIs.

## Fitness functions

This decision is satisfied when:

- every semantic invariant set has one identifiable owner;
- cross-authority consumers use explicit owner contracts rather than private mutable
  reach-through;
- multi-owner consumers document sufficient compatibility/admission facts;
- incremental paths have clean/full-equivalence and resynchronization behavior;
- graph meanings remain distinguishable in architecture and code;
- capability/support, requirements, policy, and authority are not conflated;
- no new shared framework exists without the full extraction gate;
- ordinary APIs remain understandable without mandatory knowledge of internal graphs or
  plans;
- accepted RunenGPU, RunenRender, RunenECS, RunenUI, RunenSpatial, and SDF/field owner
  contracts remain unchanged.

## Delivery and follow-up

Issue #228 owns this bounded architecture decision. Its investigation evidence is issue
#227.

After this ADR is accepted, issue #205 may create a separately reviewable documentation
slice that restructures or supersedes the old multi-reality doctrine, simplifies the
Domain Workbench north star, reconciles historical Runenwerk-local UI authority with
standalone RunenUI, merges or retires overlapping guidance, and normalizes the
cold-start architecture reading path.

Issue #220 remains separately responsible for production-track, phase-spec,
track-execution, context-profile, and executable-tooling authority cleanup.
