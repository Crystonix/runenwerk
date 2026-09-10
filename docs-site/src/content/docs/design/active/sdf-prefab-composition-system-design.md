---
title: SDF Prefab Composition System Design
description: Active V2-gated design for reusable SDF-first prefab composition, placement, products, and diagnostics.
status: active
owner: workspace
layer: domain / engine-runtime
canonical: true
last_reviewed: 2026-09-10
related_designs:
  - ../accepted/sdf-first-production-capability-map.md
  - ../accepted/sdf-first-field-world-platform-design.md
  - ./sdf-procedural-animation-and-animated-models-design.md
  - ../implemented/editor-rendered-world-and-multi-entity-viewport-design.md
---

# SDF Prefab Composition System Design

## Status

Active V2-gated design.

The accepted long-term capability map is
`../accepted/sdf-first-production-capability-map.md`.

Implementation remains gated behind:

- rendered-world V1 in `../implemented/editor-rendered-world-and-multi-entity-viewport-design.md`;
- source-backed asset identity for prefab descriptors;
- product ownership for render, field, material, collision, and diagnostic outputs.

This document is active so prefab identity and product boundaries can shape the
roadmap, but runtime prefab instancing is not part of the rendered-world V1
implementation slice.

This document defines prefab composition for an SDF-first engine.

A prefab is not a mesh object. A prefab is a reusable field composition that can produce render, collision, material, interaction, simulation, and diagnostic products.

---

# Purpose

Runenwerk needs a production prefab system for:

- trees
- rocks
- ruins
- player and enemy bodies
- riverbanks
- vegetation clumps
- gameplay props
- interactable objects
- future caves, doors, roots, bridges, and organic structures

The prefab system must support SDF-first world modelling and product-driven runtime use.

---

# Core Concept

An **SDF prefab** is an authored composition that describes:

- field shape
- material fields
- placement constraints
- interaction emitters
- collision/query products
- render products
- simulation hooks
- LOD/fallback rules
- diagnostics
- lineage and product generation

A placed prefab instance contributes to one or more products.

```text
Prefab Definition
  -> Prefab Instance
  -> SDF contribution
  -> material contribution
  -> render product
  -> collision/query product
  -> interaction/influence emitters
  -> diagnostics
```

---

# Design Goals

1. Make prefabs SDF-first.
2. Avoid mesh-centric prefab assumptions.
3. Keep prefab definitions serializable, inspectable, ratifiable, and diffable.
4. Allow prefab instances to generate multiple product families.
5. Support deterministic placement from procgen and authored placement.
6. Support streaming and LOD by product scope.
7. Support reusable material field rules.
8. Support interaction emitters for grass, wetness, influence, and gameplay.
9. Support collision/query products separate from visual render products.
10. Make prefab diagnostics first-class.

---

# Non-Goals

This design is not:

- a mesh prefab system
- a final editor prefab UI design
- a physics solver
- an animation system
- a material graph design
- a renderer implementation
- an ECS archetype design

It defines the prefab product model and ownership boundaries.

---

# Prefab Types

## Static SDF Prefabs

Examples:

- rock
- stone pillar
- ruin wall
- arch
- tree stump
- fallen log

Characteristics:

- stable shape
- static material fields
- stable collision products
- streamable by placement scope

## Procedural SDF Prefabs

Examples:

- twisted tree
- mossy boulder
- root cluster
- ruin fragment
- fairy-ring stones

Characteristics:

- seed-driven variation
- generated details
- deterministic from prefab seed + placement seed
- authored high-level rules

## Animated SDF Prefabs

Examples:

- player
- enemy
- creature
- swaying tree
- animated magical object

Characteristics:

- pose/deformation state
- runtime products
- interaction emitters
- changing bounds

Animated character prefabs are further specified in the active
[SDF Procedural Animation and Animated Models Design](sdf-procedural-animation-and-animated-models-design.md).

## Field Emitter Prefabs

Examples:

- lantern
- glowing mushroom
- poison flower
- cursed stone
- wet spring
- scent source
- heat source

Characteristics:

- may have little or no visible mass
- emits influence/material/radiance/substance products
- can be gameplay-relevant

---

# Prefab Definition Model

A prefab definition should include:

| Field | Meaning |
|---|---|
| Prefab identity | Stable ID for authored prefab. |
| Display name | Human-readable name. |
| Version | Schema/content revision. |
| SDF composition | Shape graph or field recipe. |
| Material rules | Material/substance channel outputs. |
| Bounds policy | Conservative bounds and dynamic bounds rules. |
| Placement rules | Slope, biome, moisture, water, spacing, scale. |
| Product outputs | Render, collision, influence, diagnostics, etc. |
| LOD policy | Scale bands, transitions, fallback products. |
| Interaction emitters | Footstep, bend, threat, glow, wetness, etc. |
| Streaming behavior | Residency class and scope rules. |
| Ratification rules | Validity constraints. |
| Diagnostics | Warnings/errors for invalid definition. |

---

# SDF Composition Model

Prefab SDF composition may include:

- primitives
- procedural noise
- smooth union
- subtraction
- intersection
- deformation
- twist/bend/taper
- repetition
- masks
- local material fields
- anchors/attachment points
- optional child-prefab references

Examples:

```text
TreePrefab
  trunk field
  root fields
  branch fields
  bark material mask
  moss mask
  foliage density field
  wind response field
```

```text
RockPrefab
  base noisy volume
  erosion mask
  crack fields
  moss/wetness material channels
  collision support field
```

```text
RuinArchPrefab
  block SDF fields
  erosion subtraction
  moss material field
  stability/collision field
  placement anchors
```

---

# Prefab Instance Model

A prefab instance should include:

| Field | Meaning |
|---|---|
| Instance identity | Stable ID for this placement. |
| Prefab identity | Referenced prefab definition. |
| Transform | Position, rotation, scale. |
| Seed | Deterministic variation seed. |
| Scope | Chunk, region, sector, or authored scope. |
| Overrides | Material, scale, product, gameplay overrides. |
| Runtime state | Optional state for interactive/animated prefabs. |
| Generation | Instance/product generation. |
| Diagnostics | Instance-specific issues. |

Instances may be authored, generated, spawned, or replicated.

---

# Product Outputs

A prefab may produce several products.

| Product | Purpose |
|---|---|
| SDF render product | Renderer-visible field contribution. |
| Material field product | Surface/material/substance contribution. |
| Collision/query product | Physics/gameplay interactions. |
| Navigation product | Walkability/blocking/cost contribution. |
| Influence product | Threat, scent, magic, glow, danger, etc. |
| Vegetation product | Foliage/grass interaction or local density. |
| Water/wetness product | Wetness, flow obstruction, shoreline influence. |
| Diagnostic product | Bounds, lineage, freshness, invalid placement. |

Not every prefab produces every product.

---

# Placement Rules

Placement rules allow authored and procedural placement to use the same contract.

Examples:

- allowed biome
- allowed material surface
- slope range
- altitude range
- water distance
- moisture range
- minimum spacing
- maximum density
- local avoidance radius
- alignment to surface normal
- preferred orientation
- cave/interior eligibility
- gameplay relevance class

Placement products should be deterministic and inspectable.

---

# LOD and Fallback

Prefab LOD should use product bands.

Example tree:

```text
near:
  full SDF trunk/root/branch + foliage density
mid:
  simplified SDF trunk/branch + clumped foliage
far:
  silhouette/density summary
summary:
  biome/material contribution only
```

Transition rules:

- no hard pop
- stable seeded variation
- cross-fade or field morph
- preserve gameplay silhouettes
- preserve collision authority near player
- visual fallback must not replace strict collision

---

# Streaming

Prefab products stream by scope.

A prefab instance may have:

- definition resident
- instance record resident
- render product resident
- collision product resident
- influence product resident
- material product resident
- fallback summary resident

Different products can stream independently.

Example:

```text
far tree:
  instance summary resident
  full SDF render product non-resident
  collision product non-resident
  biome contribution resident
```

---

# Interaction Emitters

Prefab instances can emit field interactions.

Examples:

| Emitter | Use |
|---|---|
| Glow | lighting/radiance/material response |
| Scent | AI perception |
| Threat | enemy influence |
| Wetness | water/shoreline |
| Heat | fire/temperature |
| Bend | vegetation interaction |
| Obstruction | wind/water flow |
| Sound | acoustic/perception field |

Emitters should be declared and product-scoped.

---

# Renderer Integration

The renderer receives prepared prefab render products:

- SDF composition references
- instance transforms
- material masks
- bounds
- LOD/fade state
- product generation
- diagnostic flags

The renderer should not own prefab truth.

---

# Physics and Query Integration

Collision/query products may differ from visual products.

Rules:

1. Strict collision cannot use visual-only fallback.
2. Collision LOD changes must be safe near active bodies.
3. Dynamic/animated prefabs must expose conservative bounds.
4. Interaction products must declare authority class.

---

# Multiplayer

Replicate:

- authoritative prefab placement
- instance creation/destruction
- gameplay-relevant state
- generation changes
- operation logs

Do not replicate:

- renderer cache
- local fallback products
- editor-only diagnostics unless collaborative tooling requires it

---

# Diagnostics

Prefab diagnostics should expose:

- invalid definition
- missing child prefab
- invalid SDF composition
- invalid bounds
- invalid material rules
- invalid placement
- missing output product
- stale instance products
- fallback active
- collision/render mismatch
- unsupported consumer request

---

# Open Questions

1. What is the minimum SDF composition graph needed for first production prefabs?
2. Should prefab composition reuse `domain/graph` directly or define a prefab-specific graph over it?
3. How are child prefabs referenced without creating cyclic dependencies?
4. What is the first accepted material channel set?
5. How are animated prefab bounds updated?
6. What prefab outputs are mandatory versus optional?
7. How are authored and procedural placements unified?
8. What is the first prefab ratification report shape?
9. How much prefab state is multiplayer-authoritative?
10. How do prefab product generations map to asset revisions?

---

# Design Decisions

1. Prefabs are SDF/field compositions.
2. Meshes are not the primary prefab representation.
3. Prefab definitions are separate from prefab instances.
4. Prefabs produce multiple product families.
5. Visual and collision products are separate.
6. Placement is product/contract-driven.
7. Fallbacks are explicit and diagnosable.
8. Procedural variation is deterministic.
9. Prefab diagnostics are required.
10. Character prefabs specialize this model rather than replacing it.

---

# Implementation Phases

## Phase 1: Prefab Descriptor Contract

Deliver:

- prefab definition descriptor
- prefab instance descriptor
- product output declaration
- placement rule descriptor
- diagnostics vocabulary

## Phase 2: Static SDF Prefabs

Deliver:

- rock/tree/ruin SDF definitions
- placement records
- render products
- collision products
- diagnostics

## Phase 3: Product Integration

Deliver:

- material field output
- collision/query output
- render product resolver integration
- streaming/residency integration

## Phase 4: Procedural Variation

Deliver:

- seed-driven variation
- deterministic product generation
- generated placement products
- debug inspection

## Phase 5: Interaction Emitters

Deliver:

- glow/wetness/threat/bend emitters
- field-product output hooks
- diagnostics

## Phase 6: Animated Prefab Bridge

Deliver:

- animated prefab product hooks
- character system handoff
- dynamic bounds

---

# Acceptance Criteria

This design is accepted when:

1. A rock/tree/ruin can be defined as an SDF prefab.
2. A prefab instance can produce render and collision products.
3. Placement is deterministic and inspectable.
4. Prefab products have lineage/freshness/residency.
5. Visual fallback and collision fallback are separate.
6. Prefab diagnostics expose invalid or stale state.
7. Character prefabs can build on the same model.
