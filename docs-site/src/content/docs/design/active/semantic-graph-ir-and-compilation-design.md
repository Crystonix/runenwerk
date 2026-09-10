---
title: Semantic Graph IR and Compilation Design
description: Active design for constrained semantic graph authoring, ratification, lowering, and ECS/scheduler compilation boundaries.
status: active
owner: workspace
layer: cross-domain
canonical: true
last_reviewed: 2026-09-11
related_docs:
  - ../../guidelines/runenwerk-architecture.md
  - ../../domain/scheduler/README.md
related_designs:
  - ./gameplay-graph-atr-ir-and-ecs-lowering-design.md
  - ./engine-game-runtime-editor-ecs-scripting-hot-reload-design.md
  - ../accepted/sdf-first-field-world-platform-design.md
  - ./workspace-viewport-expression-upgrade-design.md
related_adrs:
  - ../../adr/accepted/0010-graph-substrate-canvas-boundary.md
---

# Semantic Graph IR and Compilation Design

## Status

Active design. This document records the current target architecture for future semantic graph domains. It does not describe an implemented crate yet.

## Purpose

Runenwerk needs graph-shaped authoring for future material, simulation, gameplay, ability, field, and tooling workflows. Those graphs must not become arbitrary runtime programs, editor-owned scripts, or a universal engine object graph.

This design defines a constrained semantic graph model:

```text
authored semantic graph intent
  -> normalized graph intent
  -> ratified semantic IR
  -> formed ECS/scheduler products
  -> instantiated/simulated runtime execution
  -> observed/expression products for tools
```

The runtime should execute formed ECS systems, event channels, schedule plans, render products, or other domain-owned execution products. It should not walk editor-authored semantic graphs every frame.

## Repository Anchors

Current implemented anchors:

- `domain/graph` owns the domain-neutral graph substrate: typed graph, node, port, edge definitions, validation, traversal, and cycle policy.
- `domain/scheduler` owns deterministic scheduling contracts and execution ordering.
- `domain/ecs` owns live entity/component/resource state, system execution, events, queries, and deferred ECS commands.
- `foundation/schema` owns portable schema vocabulary.
- `foundation/commands` owns portable command descriptor/proposal vocabulary, not execution.
- `foundation/ratification` owns report vocabulary; concrete ratification meaning stays domain-owned.
- `foundation/diagnostics` owns diagnostic report vocabulary; diagnostic code families stay domain-owned.
- `domain/editor/*` owns editor-facing contracts, commands, shell/workspace projections, inspector models, and persistence boundaries.
- `apps/runenwerk_editor` wires concrete editor runtime, panels, persistence locations, and engine integration.

This design must build on those anchors instead of creating a parallel architecture.

## Non-Scope

This design does not:

- change `domain/graph` into a semantic graph language;
- define material, ability, gameplay, or simulation semantics directly;
- introduce a universal `GraphRuntime`;
- make editor graph canvas state authoritative;
- make ECS archetypes, component storage, or runtime systems editor-owned;
- add a scripting language integration;
- require every domain to expose an API trait;
- replace domain commands, ratifiers, diagnostics, schemas, or scheduler contracts.

## Architectural Position

Semantic graph meaning belongs in an owning domain crate or subsystem, not in the neutral graph substrate.

Examples:

```text
domain/graph                 -> structural graph substrate only
future domain/material_graph  -> material graph semantics and lowering
future domain/ability_graph   -> ability or skill graph semantics and lowering
future domain/simulation_graph -> simulation rule semantics and lowering
future domain/gameplay_graph  -> gameplay interaction semantics and lowering
```

The exact crate should be created only when a concrete semantic domain exists. Until then, this document defines the boundary policy.

## Core Doctrine

Semantic graphs are authored intent, not runtime authority.

The governing rule is:

```text
Graphs describe constrained intent.
Domains ratify intent.
Compilers lower intent.
Schedulers order formed products.
ECS/runtime executes formed products.
Diagnostics explain every rejection and derived mapping.
```

This is a specialization of the repository doctrine:

```text
AI proposes.
Domains validate.
Ratifiers check.
Diagnostics explain.
Tests protect.
Schemas describe.
Inspection views expose.
Commands mutate.
```

## Reality Mapping

| Reality | Semantic graph form | Owner |
|---|---|---|
| Authored | User-facing graph document, node positions, draft connections, labels, source references | semantic graph domain plus editor document/persistence owner |
| Normalized | Canonical graph references, resolved structural substrate, migrated schema versions | semantic graph domain |
| Ratified | Accepted semantic IR with domain-owned issue checks passed | semantic graph domain ratifier |
| Formed | ECS queries, event channel definitions, system descriptors, schedule edges, render/material products, or domain-specific execution packages | semantic graph domain plus target execution domain contracts |
| Instantiated | Loaded runtime resources and source-to-runtime mappings | engine/runtime or target domain runtime owner |
| Simulated | Hot ECS/world/runtime state | `domain/ecs`, engine runtime, or target runtime owner |
| Observed | Inspector reports, graph-to-system maps, diagnostics, provenance, debug views | owning observation/inspector domain |
| Expressed | Viewport/tool products derived from formed or simulated data | expression/product owner and presentation consumer |

No reality may be treated as interchangeable with another merely because it has graph-shaped data.

## Semantic IR Shape

The initial semantic IR should be intentionally small and compiler-friendly.

Preferred primitive family:

```text
SELECT     what entities, resources, values, or products are in scope
RELATE     what interaction, dependency, or correspondence exists
TRANSFORM  what effect, product, event, or derived result is requested
```

These are conceptual categories, not mandatory enum names. A future domain may use domain-specific names if they preserve the same restrictions.

The graph is a declaration of interaction intent. It is not a general-purpose function graph.

## Required Constraints

A semantic graph compiler must enforce four constraint classes before producing formed execution products.

### 1. Ownership Constraint

Every transform must declare what domain-owned state or product family it may write.

Examples:

```text
combat rule may write combat event products
animation rule may write animation intent products
AI rule may read combat observations but not mutate combat authority
material rule may write material expression products, not ECS gameplay components
```

### 2. Visibility Constraint

Every select or relate operation must declare what it may read and through which contract.

The compiler should reject hidden reads of runtime internals when the domain only exposes observation frames, schemas, descriptors, or formed products.

### 3. Dependency Constraint

Every formed product must have deterministic ordering.

The graph compiler may emit scheduler edges, stage labels, system access metadata, or target-domain ordering contracts, but it must not rely on implicit graph traversal order.

### 4. Capability Constraint

Graph-authored rules may only perform operations for which the current authoring context has an explicit capability.

Capability checks belong at semantic graph construction, command proposal adaptation, ratification, or formation boundaries. Descriptor visibility is not permission.

## Runtime Rule

Runtime execution must not depend on live editor graph traversal.

Allowed runtime products include:

- ECS systems;
- ECS event channels;
- ECS queries;
- scheduler plans;
- formed scene/runtime packages;
- material/render products;
- field/world products;
- explicit runtime resources with source lineage.

Forbidden runtime products include:

- arbitrary graph interpreter loops on editor-authored graphs;
- hidden mutable graph state as simulation authority;
- graph node callbacks that bypass domain commands or ECS scheduling;
- editor canvas state in runtime APIs;
- untraceable generated systems.

## Traceability Requirements

Formation must preserve enough source lineage for debugging, diagnostics, and editor feedback.

At minimum, formed products should be able to report:

- source graph id and version;
- source node/edge/port ids where applicable;
- normalized/ratified input version;
- produced system, query, event, schedule edge, product, or resource id;
- diagnostics generated during normalization, ratification, lowering, and scheduling;
- whether the formed product is rebuildable, retained, or authority-relevant.

Traceability is not optional. Without it, semantic graph compilation becomes hard to debug and unsafe for editor tooling.

## Boundary With `domain/graph`

`domain/graph` remains neutral.

It may own:

- graph identity;
- node identity;
- port identity;
- edge identity;
- port direction;
- port type compatibility;
- structural graph definitions;
- traversal helpers;
- cycle policy;
- structural graph validation.

It must not own:

- semantic node catalogs;
- gameplay meaning;
- material meaning;
- ability meaning;
- ECS query derivation;
- event generation;
- scheduler policy beyond generic traversal/order helpers;
- graph canvas layout;
- editor tool interaction;
- runtime graph execution.

Future semantic graph crates may depend on `domain/graph`. `domain/graph` must not depend on them.

## Boundary With ECS

`domain/ecs` owns live ECS state and ECS execution contracts.

Semantic graph domains may form ECS-facing products such as:

- query descriptors;
- event channel descriptors;
- system descriptors;
- access metadata;
- schedule constraints;
- source maps.

They must not own ECS storage internals or bypass ECS command/stage rules.

If a formed semantic graph product wants to mutate ECS state, the mutation must go through ECS-owned execution mechanisms, domain-owned commands, or controlled runtime application contracts.

## Boundary With Scheduler

`domain/scheduler` owns deterministic execution planning and ordering.

Semantic graph compilers may emit scheduler input contracts. They must not make scheduler behavior depend on editor concepts or semantic graph canvas state.

Execution order should be explicit in formed products:

```text
semantic relation -> formed dependency edge
semantic transform -> formed system descriptor
semantic read/write intent -> formed access metadata
```

## Boundary With Editor

The editor is an authoring and observation front door. It is not the authority for semantic meaning.

The editor may own:

- graph canvas interaction;
- node placement;
- selection;
- pan and zoom;
- command proposals;
- document/session state;
- diagnostics display;
- graph-to-formed-product inspection views.

The editor must not own:

- semantic type definitions;
- ECS component behavior;
- runtime execution order;
- generated system behavior;
- domain acceptance rules.

The editor should consume domain-published descriptors, schemas, capabilities, diagnostics, and observation frames.

## Public API Policy

A future semantic graph domain should default to direct domain-owned APIs and ratifiers.

Do not add a trait merely because the boundary is important.

Use traits only when:

- multiple implementations are expected;
- dependency inversion is needed across an adapter boundary;
- test substitution is required at the boundary level;
- an extension point is intentionally open.

Closed policy spaces should use enums or explicit value types, not traits.

## Command and Ratification Policy

Graph edits should enter through domain/editor commands or controlled document mutation APIs.

Formation must not silently accept invalid semantic graphs. It should return structured errors or ratification reports with domain-owned issue codes.

Recommended flow:

```text
editor command proposal
  -> semantic graph document mutation
  -> structural validation
  -> semantic ratification
  -> lowering/formation
  -> formed product diagnostics and source map
```

`foundation/commands` may describe requestable mutations. It must not execute them.

`foundation/ratification` may carry report vocabulary. It must not define concrete semantic graph issue meaning.

`foundation/diagnostics` may carry report vocabulary. Concrete diagnostic codes stay in the semantic graph domain.

## Invariants

Every future semantic graph domain should define at least these invariant families:

- structural graph references resolve;
- port directions and types are compatible;
- semantic node kinds are known for the targeted graph schema version;
- semantic type references resolve through the owning registry or descriptor set;
- write ownership is unique or explicitly reconciled;
- reads use declared visibility contracts;
- formed execution order is deterministic;
- generated products retain source lineage;
- graph canvas/session state cannot affect runtime semantics;
- runtime execution does not depend on editor graph traversal;
- descriptor visibility never grants mutation permission.

## Failure Modes

This design prevents the following failures:

- `domain/graph` becoming a universal semantic god abstraction;
- editor graph canvas state leaking into runtime authority;
- runtime systems depending on arbitrary per-frame graph interpretation;
- graph-authored rules causing hidden overlapping writes;
- semantic type definitions being mutated by the editor;
- generated ECS systems becoming untraceable;
- traits becoming artificial domain APIs;
- command descriptors being mistaken for execution permission;
- observation/projection products becoming authoritative state.

## First Implementation Candidate

The first implementation should not start by creating a broad semantic graph platform.

A good first candidate must have:

- one concrete owning domain;
- a small descriptor vocabulary;
- one authored graph document shape;
- structural validation through `domain/graph`;
- one domain-owned ratifier;
- one lowering target;
- one formed product shape;
- source map diagnostics;
- focused tests.

Candidate domains:

```text
material graph -> formed material/render expression product
simulation graph -> formed ECS/scheduler product
field graph -> formed field/world product
ability graph -> formed gameplay action or event product
```

The crate name should follow the owning domain, not the UI surface.

## Suggested Module Shape

For a future crate such as `domain/simulation_graph`, prefer subdomain folders:

```text
domain/simulation_graph/src/
|-- lib.rs
|-- authored/
|   |-- mod.rs
|   |-- document.rs
|   `-- commands.rs
|-- ir/
|   |-- mod.rs
|   |-- select.rs
|   |-- relate.rs
|   `-- transform.rs
|-- ratification/
|   |-- mod.rs
|   |-- issues.rs
|   `-- ratifier.rs
|-- lowering/
|   |-- mod.rs
|   |-- ecs.rs
|   `-- source_map.rs
|-- formed/
|   |-- mod.rs
|   `-- product.rs
`-- diagnostics/
    |-- mod.rs
    `-- codes.rs
```

This is an example ownership shape, not a mandate to create the crate before a concrete domain needs it.

## Testing Strategy

Required tests for the first semantic graph domain:

- structural graph validation rejects invalid port connections;
- semantic ratifier rejects unknown semantic node kinds;
- semantic ratifier rejects unresolved semantic type references;
- ownership constraint rejects hidden overlapping writes;
- visibility constraint rejects forbidden reads;
- dependency constraint produces deterministic order;
- capability constraint rejects unauthorized transforms;
- lowering preserves source graph lineage;
- formed products do not retain editor canvas/session state;
- repeated formation from the same normalized input is stable.

Use behavior names, for example:

```text
semantic_graph_rejects_hidden_cross_domain_write
semantic_graph_lowering_preserves_source_node_lineage
formed_simulation_product_omits_editor_canvas_state
dependency_constraints_produce_stable_schedule_edges
```

## Validation Plan

Before accepting this design:

1. Pick one concrete semantic graph domain.
2. Write that domain's contract document.
3. Define authored, normalized, ratified, and formed product names.
4. Define exact issue codes and diagnostic subjects.
5. Prove the lowering target does not require runtime graph interpretation.
6. Add focused ratification and lowering tests.
7. Update the canonical [`crate inventory`](../../workspace/crate-inventory.md) and owning crate/domain docs only when a new workspace crate is actually added.

## Remaining Questions

- Gameplay graph specialization is defined in `docs-site/src/content/docs/design/active/gameplay-graph-atr-ir-and-ecs-lowering-design.md`.
- Which non-gameplay semantic graph domain should be implemented first after material/procedural and gameplay contracts are sequenced?
- Should source maps use only domain-local ids, or also integrate with future retained expression graph lineage?
- Which formed product family should be the first target: ECS/scheduler, material/render expression, or field/world products?
- When should semantic graph descriptors become discoverable through `foundation/commands` and `foundation/schema`?
- Which graph schema/version contract should govern migration of authored semantic graphs?

## Decision Summary

Runenwerk should support semantic graph authoring by compiling constrained graph intent into formed domain products.

The important boundary is:

```text
domain/graph is structure
semantic graph domains own meaning
ratifiers decide acceptance
lowering creates formed products
runtime executes formed products
editor observes and authors through contracts
```

This keeps graph tooling powerful without turning the editor into the runtime brain or the neutral graph substrate into a universal programming language.
