---
title: Gameplay Graph ATR IR and ECS Lowering Design
description: Active design for declarative gameplay graph authoring, compiler-style validation, Action/Trigger/Rule IR, and lowering into ECS queries, events, schedules, and runtime products.
status: active
owner: gameplay
layer: cross-domain
canonical: true
last_reviewed: 2026-07-24
related_designs:
  - ./semantic-graph-ir-and-compilation-design.md
  - ./engine-game-runtime-editor-ecs-scripting-hot-reload-design.md
  - ../accepted/sdf-first-field-world-platform-design.md
  - ./editor-procedural-content-and-simulation-workflow-plan.md
  - ../deferred/domain-gameplay-actions-design.md
  - ../deferred/domain-gameplay-powers-design.md
  - ../deferred/engine-gameplay-action-runtime-design.md
  - ../deferred/engine-gameplay-power-runtime-design.md
related:
  - ../../domain/graph/README.md
  - ../../domain/ecs/README.md
  - ../../domain/ecs/03-queries.md
  - ../../domain/world-sdf/README.md
---

# Gameplay Graph ATR IR and ECS Lowering Design

## Purpose

Define Runenwerk's compiler-style gameplay authoring model.

The goal is declarative, analyzable gameplay authoring:

- designers and tools declare domain concepts, rules, interactions, events, abilities, quests, state machines, triggers, ECS relations, and SDF/world-field dependencies;
- a compiler builds an abstract gameplay graph;
- compiler passes validate and normalize the graph;
- the graph lowers into Action/Trigger/Rule IR;
- formed products become ECS query descriptors, event subscriptions, system descriptors, schedule edges, generated ids, schemas, diagnostics metadata, runtime registries, and multiplayer authority metadata.

Runtime must execute formed products. It must not interpret editor-authored gameplay graphs every frame.

## Repo Truth Baseline

Implemented today:

- `domain/graph` owns neutral graph structure and validation.
- `docs-site/src/content/docs/design/active/semantic-graph-ir-and-compilation-design.md` defines the generic semantic graph pipeline and the `SELECT`, `RELATE`, `TRANSFORM` primitive family.
- Standalone `dornglut/runen-ecs` owns live ECS state, queries, systems, events, schedules, and deferred ECS commands.
- Standalone RunenECS owns deterministic ECS execution ordering contracts; Runenwerk owns only host lifecycle and adapter policy.
- Standalone `dornglut/runen-sdf` owns reusable SDF field queries, including raymarch, projection, classification, and sweep foundations.
- `domain/world_sdf` owns world-scale SDF payloads and collision query readiness contracts.
- Deferred gameplay action/power designs exist, but they do not define an active gameplay graph compiler.

Missing today:

- no `domain/gameplay_graph`;
- no accepted gameplay event/action/state/quest contract set for gameplay graph lowering targets;
- no Action/Trigger/Rule IR crate or module;
- no gameplay graph ratifier, compiler passes, source maps, or formed gameplay products;
- no editor provider for gameplay graph authoring;
- no gameplay graph to ECS query/event/schedule lowering;
- no gameplay graph authority/network validation;
- no SDF physics relation lowering from collision/query products into gameplay events.

The repository has no current `ATR` acronym in code or docs. This design uses `ATR IR` to mean `Action/Trigger/Rule IR`.

## Core Pipeline

```text
Authored gameplay input
  -> abstract gameplay graph
  -> compiler passes
  -> ratified Action/Trigger/Rule IR
  -> formed runtime products
  -> ECS/scheduler/runtime instantiation
  -> observed diagnostics and editor/debug products
```

## Required Prerequisite Contracts

Gameplay graph is an orchestration/compiler domain. It must not become the owner of all gameplay meaning.

Before first-slice gameplay graph lowering is implementation-ready, these narrower contracts must exist:

- `domain/gameplay/events`
  - event ids, payload schemas, channel descriptors, authority class, retention policy, and source-map subjects.
- `domain/gameplay/actions`
  - action requests, action plans, action results, effect vocabulary, and validation reports.
- `domain/gameplay/state`
  - state machine ids, state membership, transitions, conditions, and transition diagnostics.
- `domain/gameplay/quests`
  - quest ids, objective ids, progress semantics, completion/failure rules, and persistence policy.

`domain/gameplay_graph` may select, relate, and transform those contracts. It does not define their core semantics.

### Authored Gameplay Input

Authoring inputs may include:

- gameplay concept definitions;
- actions and abilities;
- triggers and conditions;
- events and event payload schemas;
- quests and objectives;
- state machines and transitions;
- relations between ECS component families;
- SDF/field-world query requirements;
- authority, prediction, replication, and persistence policy.

Authored input may be graph-shaped, form-shaped, table-shaped, or document-shaped. The compiler owns the abstract gameplay graph after normalization.

### Abstract Gameplay Graph

Nodes represent gameplay meaning:

- concept;
- action;
- trigger;
- rule;
- state;
- event;
- quest objective;
- ECS relation;
- query scope;
- SDF/physics interaction;
- authority boundary;
- runtime product.

Edges represent semantic dependencies:

- ownership;
- containment;
- triggering;
- data flow;
- event flow;
- ordering;
- read dependency;
- write dependency;
- authority dependency;
- network replication dependency;
- schedule dependency;
- source lineage.

Canvas layout, node positions, comments, editor selection, and panel state are authored/editor metadata. They do not enter formed runtime products.

## ATR IR Primitive Model

The first gameplay IR should preserve the generic semantic graph shape:

```text
SELECT     what gameplay entities, ECS facts, events, resources, products, or values are in scope
RELATE     what semantic relationship exists between selected facts
TRANSFORM  what action request, event, state transition, product, command, or schedule result is produced
```

These primitives are compiler categories, not a visual scripting node taxonomy.

### SELECT

`SELECT` declares a typed scope. It may lower to an ECS query descriptor, event subscription, resource read, field-product read, asset reference, or formed product lookup.

Examples:

```text
SELECT EventStream<PhysicsHit>
SELECT EntitySet<Damageable> where Has<Health> and Has<DamageReceiver>
SELECT FieldProduct<WorldCollisionReadiness>
SELECT QuestState<FindRelic>
SELECT Resource<GameplayClock>
```

Rules:

- every select has an owner, type, revision policy, and read capability;
- every ECS-facing select lowers to explicit query/access metadata;
- every field/SDF select declares product freshness and readiness requirements;
- hidden reads of runtime internals are invalid.

### RELATE

`RELATE` declares a semantic relationship between selected scopes. It does not mutate state.

Initial relation families:

- `HIT`: contact, sweep, ray, overlap, projectile, or ability hit relation;
- `IN_RANGE`: distance or field-aware range relation;
- `OWNS`: ownership or parent relation;
- `HAS_STATE`: state machine or gameplay state membership;
- `TRIGGERS`: event-to-rule or condition-to-action relation;
- `DEPENDS_ON`: ordering or data dependency;
- `AUTHORITY_OWNS`: server/client/local authority dependency;
- `REPLICATES_TO`: network visibility or replication target relation.

SDF physics integration belongs here. A `HIT` relation may be sourced from:

- standalone RunenSDF sweep/query foundations;
- `domain/world_sdf` collision query products;
- future `domain/physics` collision products;
- engine physics runtime events derived from SDF/world-field products.

The gameplay graph never recomputes private physics internals. It declares the relation it needs and lowers to formed product dependencies and event/query contracts.

### TRANSFORM

`TRANSFORM` declares the produced effect or product. It may lower to:

- emitted gameplay event;
- domain action request;
- state transition;
- quest/objective update request;
- ECS command proposal;
- schedule edge;
- generated registry entry;
- runtime product descriptor;
- diagnostics or editor metadata product.

Rules:

- transforms must declare write ownership;
- transforms that mutate live ECS must go through ECS-owned mechanisms, domain commands, or controlled runtime application contracts;
- transforms that mutate authored world state must go through editor/domain commands and ratification;
- transforms that affect networked state must declare authority and replication policy.

## Example

Authored intent:

```text
When a sword hit contacts a damageable target, emit a damage request event.
```

ATR IR shape:

```text
SELECT hit_events: EventStream<PhysicsHit>
SELECT targets: EntitySet<Damageable> where Has<Health> and Has<DamageReceiver>
RELATE hit_events HIT targets using ContactPair(source, target)
TRANSFORM emit DamageRequested {
  source = hit_events.source,
  target = targets.entity,
  amount = hit_events.impulse * weapon.damage_scale
}
```

Formed products:

- ECS query descriptor for `Damageable` targets;
- event subscription descriptor for `PhysicsHit`;
- relation matcher descriptor for contact-pair source/target matching;
- `DamageRequested` event channel descriptor;
- system descriptor such as `gameplay.damage.emit_from_hit`;
- schedule edge after physics collision publication and before health resolution;
- source map from formed products back to source graph nodes;
- diagnostics for missing `Health`, missing authority policy, illegal client-side write, stale collision product, or unsupported network replication.

## Compiler Passes

The gameplay graph compiler must run explicit passes:

1. Structural graph validation through `domain/graph`.
2. Name, id, and reference resolution.
3. Schema/type validation through domain-owned gameplay schemas.
4. Concept ratification for actions, events, states, quests, and relations.
5. Relation normalization, including expansion of shorthand rules into explicit `SELECT`, `RELATE`, and `TRANSFORM` records.
6. Dependency validation and deterministic ordering.
7. Cycle detection with separate rules for data cycles, event cycles, state-machine cycles, and schedule cycles.
8. Reachability checks for dead triggers, orphan states, unhandled events, impossible quest objectives, and unused outputs.
9. Read/write ownership validation.
10. SDF/field readiness validation for any field-dependent relation.
11. Authority, prediction, replication, and persistence validation.
12. Runtime scheduling and ECS access planning.
13. Source-map and diagnostics generation.
14. Formed product emission.

No pass may depend on graph canvas layout or editor session state.

## Lowering Targets

Initial formed product families:

- `GameplayQueryProduct`
  - ECS query descriptors, filter descriptors, component/resource access metadata, and query source maps.
- `GameplayEventProduct`
  - event channel definitions, payload schemas, subscriptions, reliability/authority class, and source maps.
- `GameplaySystemProduct`
  - system descriptor, fixed executor kind, input products, output products, and schedule dependencies.
- `GameplayScheduleProduct`
  - stage labels, ordering edges, conflict metadata, and scheduler input contracts.
- `GameplayRegistryProduct`
  - generated ids, action ids, event ids, state ids, quest ids, and runtime registry metadata.
- `GameplayNetworkProduct`
  - authority class, replication extraction/apply descriptors, prediction/reconciliation metadata, and multiplayer diagnostics.
- `GameplayDebugProduct`
  - graph-to-query map, graph-to-system map, diagnostics, runtime counters, and replay/source-lineage metadata.

The first implementation should form descriptors and fixed executor products, not generated Rust code. Rust code generation can be considered later only if descriptor execution is not sufficient and source maps stay intact.

## Boundary Rules

### Boundary With ECS

Standalone `runen-ecs` owns live state and execution.

Gameplay graph lowering may emit ECS query descriptors, event descriptors, system descriptors, access metadata, and schedule edges. It must not own ECS storage internals, bypass ECS command queues, or mutate ECS state from graph nodes.

### Boundary With SDF Physics

Reusable SDF query truth belongs to standalone RunenSDF; Runenwerk world-field collision and product truth belongs to `domain/world_sdf`, future `domain/physics`, and engine runtime adapters.

Gameplay graph may declare `HIT`, `IN_RANGE`, `ON_SURFACE`, or `INSIDE_FIELD` relations. Lowering must bind those relations to formed collision/query products or runtime events with readiness diagnostics.

### Boundary With Gameplay Actions And Powers

Deferred action/power designs remain useful lower-level domains.

`domain/gameplay_graph` may compile accepted gameplay graph transforms into action requests, event products, or power-runtime inputs when those contracts exist. It must not become the dumping ground for all action or power semantics.

### Boundary With Networking

Network transport does not own gameplay meaning.

Gameplay graph products may declare authority, prediction, replication, and reconciliation metadata so later multiplayer can use explicit descriptors. Transport lanes, packet formats, and runtime connection policy remain net/engine concerns.

## Not Blueprints 2.0

This design is not a visual scripting runtime.

Do not:

- add arbitrary imperative node execution;
- add per-frame graph interpretation;
- add hidden node callbacks;
- expose direct ECS mutation from graph nodes;
- use graph wires as dynamically typed value spaghetti;
- let graph canvas state affect runtime behavior;
- add loops, unbounded recursion, or arbitrary user code as graph semantics;
- make one universal graph for gameplay, materials, UI, physics, particles, animation, and procgen;
- hide scheduler, authority, query, or source-map products behind opaque graph execution.

Allowed graph expressiveness is constrained gameplay declaration. The compiler may reject awkward or invalid authoring if it cannot form deterministic runtime products.

## Editor And Tooling

Required editor surfaces:

- Gameplay Graph Canvas;
- Gameplay Concept Browser;
- Action/Trigger/Rule Inspector;
- Gameplay Compiler Diagnostics;
- Query/Event/Schedule Lowering View;
- Authority/Replication View;
- SDF Physics Relation Debug View;
- Runtime Gameplay Graph Debug View.

Editor providers must show source-mapped products and diagnostics. They must not execute graph nodes.

Implementation targets:

- future `domain/gameplay_graph/src/authored/document.rs::GameplayGraphDocument`
  - authored graph/document contracts.
- future `domain/gameplay_graph/src/ir/atr.rs::GameplayAtrIr`
  - Action/Trigger/Rule IR records for select, relate, transform, source maps, and diagnostics.
- future `domain/gameplay_graph/src/catalog/node_catalog.rs`
  - constrained gameplay concept, relation, and transform descriptors.
- future `domain/gameplay_graph/src/passes/`
  - compiler passes listed above.
- future `domain/gameplay_graph/src/lowering/ecs.rs`
  - ECS query/event/system/schedule product lowering.
- future `domain/gameplay_graph/src/lowering/network.rs`
  - authority and replication product lowering.
- future `domain/gameplay_graph/src/lowering/sdf_physics.rs`
  - SDF/world-field relation product binding.
- future `domain/gameplay_graph/src/formed/product.rs`
  - formed gameplay products, source maps, and issue codes.
- future `engine/src/plugins/gameplay_graph/`
  - fixed runtime adapters that instantiate formed products into ECS/scheduler/runtime resources.
- future `apps/runenwerk_editor/src/shell/providers/gameplay_graph_canvas.rs::GameplayGraphCanvasProvider`
  - graph authoring without making canvas state authoritative.
- future `apps/runenwerk_editor/src/shell/providers/gameplay_compiler_diagnostics.rs::GameplayCompilerDiagnosticsProvider`
  - diagnostics and source-map inspection.
- future `apps/runenwerk_editor/src/shell/providers/gameplay_runtime_debug.rs::GameplayRuntimeDebugProvider`
  - runtime query/event/schedule/debug products.

## First Implementation Slice

The first slice should support:

- authored gameplay graph document;
- `SELECT EntitySet`, `SELECT EventStream`, `SELECT Resource`, and `SELECT FieldProduct`;
- `RELATE HIT`, `RELATE IN_RANGE`, `RELATE TRIGGERS`, and `RELATE DEPENDS_ON`;
- `TRANSFORM EmitEvent`, `TRANSFORM RequestAction`, and `TRANSFORM StateTransition`;
- source maps for graph node/edge/port to formed query/event/system products;
- ECS query descriptor lowering;
- event subscription and event emission product lowering;
- schedule edge lowering;
- SDF/world-field readiness diagnostics for `HIT`;
- authority class diagnostics for every state-changing transform;
- editor diagnostics and lowering view.

Out of first slice but required:

- quest graph breadth;
- broad ability/power authoring;
- code generation;
- client prediction and rollback execution;
- visual debugging over recorded multiplayer sessions;
- automatic migration of old graph schema versions.

## Milestones

### G0 - Contract Design

Exit criteria:

- this design is linked from the active design index and editor roadmap;
- `semantic-graph-ir-and-compilation-design.md` points to this document as the gameplay specialization;
- docs validation passes.

### G1 - Gameplay IR Domain

Exit criteria:

- `domain/gameplay_graph` exists;
- authored, normalized, ratified, and formed product types exist;
- first-slice issue codes and source-map subjects are defined.

### G2 - ECS Lowering

Exit criteria:

- first-slice `SELECT` records lower to ECS query/event/resource descriptors;
- first-slice `TRANSFORM` records lower to event/action/state products;
- schedule edges and ECS access metadata are deterministic.

### G3 - SDF Physics Relations

Exit criteria:

- `RELATE HIT` can bind to SDF/world-field collision or physics products through explicit readiness contracts;
- stale or missing collision/query products produce diagnostics;
- gameplay graph compiler does not compute private physics internals.

### G4 - Editor Tooling

Exit criteria:

- gameplay graph canvas, compiler diagnostics, and lowering view exist;
- invalid graphs preserve the last valid formed products;
- source maps connect runtime products back to graph nodes.

### G5 - Authority And Multiplayer Readiness

Exit criteria:

- every state-changing transform declares authority class;
- network product descriptors exist for replication extraction/apply and prediction/reconciliation metadata;
- transport remains gameplay-agnostic.

## Validation Strategy

Required tests:

- structural graph validation rejects invalid gameplay connections;
- unknown gameplay node kinds are rejected;
- unresolved concept ids are rejected;
- `SELECT EntitySet` lowers to stable ECS query descriptors;
- `RELATE HIT` rejects missing collision product readiness;
- `TRANSFORM EmitEvent` produces an event channel descriptor and source map;
- state-changing transforms reject missing authority policy;
- dependency ordering produces deterministic schedule edges;
- formed products omit graph canvas/editor session state;
- repeated lowering from the same ratified IR is stable;
- invalid lowering preserves the previous valid formed product.

Representative names:

```text
gameplay_graph_select_entity_set_lowers_to_ecs_query_descriptor
gameplay_graph_hit_relation_requires_collision_product_readiness
gameplay_graph_emit_event_preserves_source_node_lineage
gameplay_graph_state_transform_requires_authority_policy
gameplay_graph_lowering_omits_canvas_state
gameplay_graph_same_ir_forms_same_schedule_edges
```
