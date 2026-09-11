---
title: Runenwerk Platform Architecture
description: Canonical top-down architecture spine for the Runenwerk integration platform, semantic federation, specialized execution, Workbench inspection, and batteries-included application composition.
status: accepted
owner: workspace
layer: architecture
canonical: true
last_reviewed: 2026-09-11
related_adrs:
  - ../adr/accepted/0014-repository-family-extraction-boundaries.md
  - ../adr/accepted/0015-separate-gpu-execution-from-rendering.md
  - ../adr/accepted/0017-cross-authority-consistency-and-graph-semantics.md
  - ../adr/accepted/0018-semantic-federation-and-physical-realization.md
  - ../adr/accepted/0019-batteries-included-application-composition.md
related_docs:
  - ./repository-family-architecture.md
  - ../guidelines/authority-centered-boundary-architecture.md
  - ../guidelines/domain-program-architecture-pattern.md
  - ../reports/investigations/2026-08-12-semantic-federation-and-inspection-provenance.md
  - ../reports/investigations/2026-08-12-application-composition-and-networking-ergonomics.md
---

# Runenwerk Platform Architecture

## Purpose

This is the canonical top-down architecture spine for Runenwerk.

Use it to understand the whole platform before reading subsystem-specific designs. The
owning ADRs contain the full decision detail; this page exists so a cold-start human or
agent does not have to reconstruct the architecture from many parallel documents.

## North star

Runenwerk is a **custom integration, application, and Workbench platform for
independently owned Runen systems**.

Its long-term target combines:

- independently useful typed frameworks and domain owners;
- explicit semantic ownership and cross-owner contracts;
- freedom to choose specialized CPU, GPU, spatial, field, network, persisted, or other
  physical representations;
- specialized execution rather than one universal runtime substrate;
- source lineage, diagnostics, intermediate inspection, and domain-specific tools;
- a federated Workbench that makes heterogeneous systems understandable without taking
  their authority;
- a batteries-included application experience that composes the internal architecture
  without exposing routine integration ceremony to every product author.

The compact form is:

```text
Independent owners.
One semantic grammar.
Explicit typed integration.
Owner-local versions; consumer-owned admission.
Many physical realizations.
Specialized execution.
One inspectable Workbench.
One batteries-included App experience.
```

Runenwerk is ambitious about product capability and conservative about universal
machinery.

## One picture

```text
                         PRODUCT / APPLICATION

                     one App composition root
                              |
                transparent product/plugin groups
                              |
                 product- and domain-specific intent
                              |
                              v
                     RUNENWERK INTEGRATION

             adapters / lifecycle / product policy / hosts
                 /          |          |          \
                v           v          v           v
             owner       owner      owner       owner
          / framework  / framework / framework / framework
                \          |          |          /
                 +---------+----------+---------+
                              |
                       typed contracts
                              |
                explicit compatibility/admission
                              |
                              v
                 SPECIALIZED REALIZATION / EXECUTION

          CPU / ECS / fields / spatial / GPU / persistence /
                   networking / external services / ...


                       OPTIONAL WORKBENCH

          owner-selected read-oriented inspection projections
          tables / trees / graphs / timelines / fields /
                    images / resources / reports
                              |
                    lineage and correlation

          inspection never becomes foreign semantic authority
```

This is an ownership/integration map, not one mandatory dataflow pipeline.

# 1. Framework ownership and Runenwerk integration

Current cross-repository family membership, repository roles, and source-authority
handoffs are owned by [Dornglut Engineering](https://github.com/dornglut/engineering/blob/main/architecture/runen-family.md)
and [Engineering ADR 0008](https://github.com/dornglut/engineering/blob/main/adrs/0008-adopt-bounded-source-authority-handoffs.md).

Within Runenwerk, ADR 0014 establishes the local integration consequence:

> Independently useful frameworks own their reusable semantics. Runenwerk owns
> application lifecycle, cross-framework integration, product policy, and adapters.

The Runenwerk integration direction is:

```text
peer frameworks / domain owners
        |
        v
Runenwerk adapters and integration
        |
        v
applications / tools / product hosts
```

A peer framework must not depend on Runenwerk merely to obtain a global identity,
participate in inspection, register into a universal database, or make application
composition convenient.

Current accepted peer-framework relationships are recorded by Engineering's Runen-family
architecture. ADR 0015 is the Runenwerk-local ownership-separation decision underlying
the accepted `RunenRender -> RunenGPU` relationship: semantic image formation needs GPU
execution while RunenGPU remains independently useful.

Current source location is implementation evidence, not permanent ownership.

# 2. Semantic ownership and contracts

The foundational ownership law is:

```text
One semantic invariant set has one authority.
```

Layered values may coexist when they own different invariants:

```text
world-authoritative pose
renderer-local transform
GPU-packed transform
```

Correspondence does not imply shared identity or authority.

A cache, database row, GPU buffer, persisted artifact, index, or expensive derived value
does not become semantic authority merely because runtime behavior relies on it.

## Shared reasoning grammar

ADR 0018 gives Runenwerk one common way to reason across heterogeneous owners without
forcing them onto one implementation substrate:

| Question | Meaning |
| --- | --- |
| **Authority** | Who owns the semantic invariant and decides validity? |
| **Contract** | What explicit typed semantic boundary is exposed or accepted? |
| **Operation / Relationship** | How do contracts, values, or authorities relate or transition? |
| **Validity / Provenance** | Under what owner-local context is this meaningful, compatible, and traceable? |
| **Realization** | How is semantic meaning represented, retained, located, or accelerated physically? |
| **Effect** | Where does authoritative or external state actually change? |

These are architecture questions, not mandatory runtime wrappers such as
`MetaContract<T>`, `MetaNode`, or `RunenObject`.

Useful owner-specific roles remain distinct, including:

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

Cross-domain explanatory verbs such as `Observe`, `Propose`, `Derive`, `Adapt`,
`Admit`, `Realize`, `Execute`, and `Commit` are open vocabulary, not a closed enum or
universal execution pipeline.

# 3. Versions and compatibility

There is no global Runenwerk revision, transaction, frame snapshot, or clock that makes
every owner value mutually current.

The compact law is:

```text
Versions are owner-local.
Compatibility is contextual.
Admission is consumer-owned.
```

A consumer combining several authorities admits only the facts its operation actually
needs, such as:

```text
revision / generation
time / tick / interval
scope / coverage
completeness
freshness / bounded staleness
availability / residency
capability / accuracy
source lineage
identity correspondence
legal fallback
```

A renderer invocation, network publication, editor inspector, and offline proof may all
need different legal cuts over the same owners.

A Workbench can correlate values for inspection without certifying them for runtime
use:

```text
inspection correlation / join
!=
semantic runtime admission
```

# 4. Meaning versus realization

ADR 0018 establishes:

> **A semantic contract is not defined by its physical realization. A realization may
> constrain capability, precision, validity, locality, residency, lifetime, or cost,
> but it must not silently redefine the semantic contract it claims to realize.**

This protects correctness while leaving implementation freedom.

Possible realizations include:

```text
ECS archetype columns
sparse sets
persistent tables / COW pages
BVHs / spatial indices
analytic fields / sampled grids
CPU vectors / SoA / SIMD
GPU buffers / textures
compiled programs
persisted artifacts
network payloads
remote services
```

If a realization cannot preserve a claimed semantic property, it must expose a weaker
or different capability, declare its approximation/tolerance, adapt explicitly, or
reject the request according to the owner contract.

No physical layout becomes the platform ontology.

# 5. Views, products, retention, and change

Database-derived distinctions are useful when they remain orthogonal:

| Concept | Question |
| --- | --- |
| **Projection / View** | How can derived information be read for this purpose? |
| **Snapshot** | Which immutable owner-local state/revision is observed? |
| **Product** | What semantic result has an owner intentionally published? |
| **Materialization** | Is derived information retained rather than recomputed? |
| **Index** | What retained structure accelerates access? |
| **Cache** | What discardable/reconstructable state avoids repeated work? |
| **Realization** | What physical representation satisfies the semantic contract? |

A value can legitimately carry several of these roles at once. Retention or
acceleration does not grant source authority.

Incremental mechanisms remain owner-specific. The shared correctness law is:

> For the same admitted semantic inputs, incremental evaluation is observationally
> equivalent to clean/full evaluation under the owner's declared equality/tolerance.

Missing or untrusted narrow change evidence widens invalidation, causes clean rebuild,
or triggers full resynchronization as required. A cache hit changes cost, not meaning.

# 6. Specialized execution and graphs

Runenwerk deliberately does not use one execution model.

Owners may need direct functions, ECS systems, parallel CPU scheduling, async IO,
streams, GPU work, incremental evaluation, fixed-point iteration, network sessions,
controllers, or external processes.

Graph shape also does not imply one graph runtime:

```text
semantic dependency
!= execution ordering
!= resource hazard
!= invalidation dependency
!= containment / hierarchy
```

Source graphs, program graphs, product dependencies, scheduler readiness, render plans,
RunenGPU work/access graphs, and retained runtime topology keep their actual edge
semantics.

Feedback is explicit. Temporal feedback, fixed-point iteration, desired/observed
reconciliation, interaction loops, and distributed prediction/correction are different
patterns.

# 7. Shared extraction

Shared implementation is earned by repeated neutral proof:

```text
design locally
-> prove one real domain
-> prove a structurally different second domain
-> identify concrete repeated implementation/maintenance burden
-> characterize dependency/runtime/memory/version/cognitive cost
-> accept a separate extraction decision
-> extract only the repeated neutral primitive
```

A shared contract must remain meaningful if either proving domain disappears and must
not contain proving-domain semantic branches.

The acceptance bar rises with commitment:

```text
Level A
  conceptual law / vocabulary

Level B
  interoperability / reflection contract

Level C
  shared runtime mechanism
```

ADR 0018 accepts Level-A semantic federation. It does not pre-authorize a shared
Workbench protocol, query engine, dataflow runtime, global store, generic optimizer, or
meta-executor.

# 8. Workbench

Runenwerk is more than a runtime. Its long-term product includes games, focused tools,
editor/workbench applications, procedural and field workflows, simulation/render/GPU
inspection, asset/content formation, networking inspection, diagnostics, and headless
automation.

The Workbench obtains coherence through **federation**, not ownership centralization:

```text
owners / peer frameworks
        |
        v
explicit Runenwerk integration / inspection adapters
        |
        v
optional Workbench inspection session
```

Owners may expose zero or more read-oriented inspection projections, directionally:

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

These are tooling views, not semantic storage classes.

The Workbench must not become a global semantic ID/revision authority, domain mutation
authority, runtime admission authority, universal payload store, universal serialization
format, or generic executor over private owner state.

Inspection should remain optional, lazy, bounded, pressure-aware, and device/location
aware. Inspecting GPU metadata must not imply payload readback; inspecting a field must
not imply eager dense materialization.

Any peer-neutral inspection/reflection API requires a separate Level-B decision.

# 9. Application and product experience

ADR 0019 establishes:

> **Internal decomposition must not determine application complexity.**

`App` remains Runenwerk's one runtime composition root.

The product-facing model is progressive disclosure:

```text
ordinary path
  supported batteries-included product/plugin group(s)
  + product/domain declarations

configuration path
  inspect/configure/replace selected members
  + owner-specific configuration

expert path
  direct owner/framework plugins
  custom render flows
  custom networking drivers/transports
  specialized adapters / lower-level contracts
```

All three paths use the same underlying owners. Convenience must not create a second
runtime, service locator, or persistent mirror of plugin/resource truth.

## Product/plugin groups

A future product/plugin group is an ordered, inspectable **composition recipe** over
existing plugins/configuration. It is not semantic authority.

The final Rust API, memberships, and names are not yet implemented or accepted. A
concrete design should prove only the machinery needed for deterministic membership,
composition, configuration/replacement, legal removal, and useful incompatibility
reporting.

Do not create one preset for every capability cross-product.

## Product intent remains visible

The convenience layer removes repeated generic Runen integration wiring. It does not
hide real application semantics such as game rules, product-specific render flows,
input bindings, procedural/world policy, editor behavior, specialist devices, or game
network protocol intent.

Runenwerk owns common integration wiring only when it is genuinely reusable across
supported products.

## Current implementation honesty

The target is not fully implemented.

Current `App` construction still installs broader builtin state than the long-term
product-capability ownership rule, and the `engine` package has a broader compile-time
dependency surface than runtime product selection implies.

Therefore:

```text
runtime composition simplicity
!=
compile-time / binary-size modularity
```

Product groups, App builtin cleanup, and compile-time capability subtraction require
separate implementation/design proof.

# 10. Networking

Runen networking is custom engine technology, not merely transport glue.

Directionally:

```text
engine_net
  protocol / session / replication semantics

engine_net_quic
  Quinn-based QUIC realization

engine integration
  scheduling / simulation authority / history / diagnostics

application/gameplay
  protocol declarations and game-specific policy
```

Runen owns authoritative replication, snapshots/deltas, ACK/baseline/resync,
prediction/correction, interest/streaming, simulation/history/replay integration,
diagnostics, and typed/declarative game-network authoring.

Lower-level libraries may own QUIC, TLS/crypto, sockets, or OS networking mechanics
without owning Runen replication semantics.

The target ordinary gameplay path is registration-driven:

```text
register replicated entities/components
register inputs and ownership routing
write ordinary authoritative ECS/game systems
Runen handles standard extract/snapshot/delta/apply/ACK/replay plumbing
```

That target is not fully implemented yet; existing networking designs/roadmap own the
remaining standard-ECS bridge work. Custom replication drivers remain an expert path for
genuinely specialized representations.

Authentication, cloud persistence, object storage, lobbies/presence, hosted functions,
payments, and similar managed services are a separate product concern and do not replace
authoritative game-state networking.

# 11. Domain Program and other specialized patterns

A Domain Program is an **optional specialized pattern** for domains that genuinely need
several concerns such as durable authored intent, versioning, inspection, migration,
fixtures/conformance, multiple hosts, compiled hot-path artifacts, source maps, or
reproducibility.

Typical shape:

```text
authoring source/model
-> normalized domain model
-> Domain Program
-> domain-owned graph(s) if useful
-> compiler and/or evaluator
-> runtime artifact / product / output facts
-> host integration
```

It is not the universal root of Runenwerk. GPU resources, mounted UI runtime state,
input events, spatial availability, render scene snapshots, network streams, scheduler
readiness, and query results need not become Domain Programs.

Use the [Domain Program Architecture Pattern](../guidelines/domain-program-architecture-pattern.md)
for full guidance.

Other powerful patterns remain local where their semantics fit:

- relational/views/materialization/indexing for genuinely relational data;
- incremental/dataflow mechanisms where their time/progress/state costs are justified;
- compiler/lowering systems for authored semantic programs;
- reconciliation only where desired and observed state can change independently;
- event/history models only where history itself is required authority/evidence.

# 12. What is deliberately not universal

Current Runenwerk architecture does not authorize one universal implementation for:

```text
object identity
revision / transaction / global snapshot
storage engine
ECS
query language / query engine
graph runtime
dataflow runtime
incremental database
logical clock
scheduler / executor
event bus
serialization / wire format
memory layout
GPU representation
compiler / evaluator framework
reconciliation runtime
managed backend services
Workbench reflection ABI
```

These may exist locally. Shared extraction remains possible after real repeated proof.

# 13. Accepted architecture versus future implementation

Accepted durable Runenwerk authority includes:

- framework ownership separation and one-way Runenwerk integration direction;
- one semantic invariant set per authority;
- explicit foreign-owner contracts;
- owner-local versions and consumer-owned admission;
- graph/feedback separation and incremental/full correctness;
- semantic/physical realization separation;
- semantic-federation reasoning vocabulary;
- no pre-authorized shared meta runtime;
- `App` as the one runtime composition root;
- product/plugin groups as the accepted future composition concept;
- custom Runen networking semantic ownership with contained transport realization.

Still separately gated:

- concrete product/plugin-group API and memberships;
- moving current `App` builtin resources;
- Cargo feature/dependency topology for smaller builds;
- peer-neutral Workbench inspection/reflection protocol;
- Workbench query capabilities;
- low-boilerplate standard ECS networking completion;
- managed backend/provider integrations;
- any new shared runtime mechanism.

Documentation must not describe these future items as already implemented.

# 14. Fitness functions

The architecture remains healthy when:

1. semantic invariants have identifiable owners;
2. peer frameworks remain independently useful;
3. cross-owner reads use explicit contracts instead of private mutable reach-through;
4. multi-owner consumers define enough compatibility/admission facts;
5. physical optimization can change without silently changing semantic meaning;
6. graph species retain their real edge semantics;
7. incremental systems retain clean/full or full-resynchronization safety behavior;
8. Workbench inspection stays optional and non-authoritative;
9. shared infrastructure is extracted only after structurally different real proofs;
10. routine application paths expose product/domain concepts instead of internal wiring;
11. defaults remain inspectable and lower to the same owners used by expert paths;
12. custom Runen systems can use contained lower-level libraries without giving away
    their higher-level semantic ownership;
13. docs distinguish accepted architecture, current implementation, future target, and
    historical evidence.

# 15. Cold-start reading path

For cross-domain architecture work:

1. **This page** — current Runenwerk-wide synthesis.
2. [Dornglut Engineering Runen-family architecture](https://github.com/dornglut/engineering/blob/main/architecture/runen-family.md) — current cross-repository family membership and repository relationships.
3. [ADR 0014](../adr/accepted/0014-repository-family-extraction-boundaries.md) —
   Runenwerk-local framework extraction and integration decision basis.
4. [ADR 0017](../adr/accepted/0017-cross-authority-consistency-and-graph-semantics.md) —
   authority, consistency, graph, incremental, capability, extraction, and safety laws.
5. [ADR 0018](../adr/accepted/0018-semantic-federation-and-physical-realization.md) —
   positive semantic-federation, physical-realization, and Workbench direction.
6. [ADR 0019](../adr/accepted/0019-batteries-included-application-composition.md) —
   product-facing `App` and usability doctrine.
7. [Framework Integration Architecture](./repository-family-architecture.md) and the owning
   subsystem/framework design for the work at hand.
8. [Authority-Centered Boundary Architecture](../guidelines/authority-centered-boundary-architecture.md)
   or [Domain Program Architecture Pattern](../guidelines/domain-program-architecture-pattern.md)
   when those specialized guidelines apply.

Investigation reports explain research ancestry; they do not override accepted ADRs.

## Final position

Runenwerk is not one database, ECS, graph, compiler, meta-framework, editor shell, or
renderer.

It is a custom integration and Workbench platform whose owners can stay specialized
while participating in one coherent product:

```text
owners keep meaning
contracts make boundaries explicit
admission makes multi-owner use deliberate
realizations stay free to optimize
execution stays specialized
inspection makes the whole system understandable
App composition makes the ordinary product path simple
```

That combination—not one universal substrate—is the current Runenwerk North Star.
