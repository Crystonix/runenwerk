---
title: SDF Procedural Animation and Animated Models Design
description: Active cross-domain design for procedural animation, animated SDF models, semantic regions, purpose-specific field products, and scheduler-aware runtime lowering.
status: active
owner: workspace
layer: cross-domain
canonical: true
last_reviewed: 2026-09-11
related_adrs:
  - ../../adr/accepted/0004-separate-description-from-execution.md
  - ../../adr/accepted/0005-projections-are-derived-state.md
  - ../../adr/accepted/0008-adopt-sdf-first-field-product-architecture.md
  - ../../adr/accepted/0011-animated-sdf-authoring-graphs-lower-before-runtime.md
  - ../../adr/proposed/animated-sdf-lowering-and-purpose-specific-products.md
related_designs:
  - ../accepted/sdf-first-field-world-platform-design.md
  - ../accepted/field-product-contracts-diagnostics-and-residency-design.md
  - ../accepted/execution-fabric-and-product-jobs-design.md
  - ../accepted/sdf-product-renderer-and-gpu-residency-design.md
  - ../accepted/sdf-first-production-capability-map.md
  - ./sdf-prefab-composition-system-design.md
  - ../deferred/sdf-physics-collision-system-design.md
  - ../deferred/sdf-world-production-slice-design.md
supersedes:
  - ../superseded/sdf-character-animation-system-design.md
---

# SDF Procedural Animation and Animated Models Design

## Status

Active cross-domain architecture design.

This document supersedes the older deferred SDF character animation draft and refines it into a broader procedural-animation, semantic-SDF, product-lowering, and scheduler-aware runtime design. It does not create a new crate or declare animation/physics/rendering domains implemented.

Implementation remains gated behind accepted ownership, product, scheduler, and validation decisions. ADR 0011 accepts the narrow rule that animated SDF authoring graphs lower before runtime hot paths consume them; the umbrella proposed ADR remains preserved context for broader product-family decisions.

## 0. Five-Pass Revision Summary

This revision turns the original brainstorm into an implementation-guiding architecture document.

```text
Pass 1: Architecture contract
  Added hard invariants, purpose-specific field guarantees, and non-negotiable runtime rules.

Pass 2: Ownership and execution
  Added domain ownership, mutation authority, compiler-style lowering, scheduling, and deferred mutation rules.

Pass 3: SDF, rendering, and physics correctness
  Added field correctness policy, deformation safety, render LOD policy, physics proxy policy, and cache invalidation doctrine.

Pass 4: Asset, editor, and adjacent-domain synthesis
  Added asset composition, procedural node workflow ideas, robotics/control inspiration, VFX interaction fields, and ECS relationship thinking.

Pass 5: Coherence and delivery
  Reduced ambiguity, converted open ideas into ADR candidates, added validation tests, measurable acceptance criteria, and phased implementation work.
```

The most important architectural change:

```text
Animated SDF assets should not execute directly from authoring graphs.
They should lower through validated semantic IR into runtime field plans, scheduled product jobs, and purpose-specific products/proxies.
```

## 1. Purpose

This document defines the long-term architecture for procedural animation of SDF-based models in Runenwerk.

The goal is not to copy skeletal mesh animation and replace vertices with distance fields. The goal is to treat animated SDF models as semantic, queryable spatial entities that participate in rendering, physics, gameplay, VFX, AI, world simulation, and editor tooling.

Core position:

```text
An animated SDF model is a living spatial function:
shape + semantics + motion intent + constraints + deformation + simulation feedback + runtime caches.
```

## 2. Context

Traditional mesh animation usually starts from:

```text
vertices + bones + skin weights + clips
```

That is a surface-first model.

Runenwerk should use an SDF-first model:

```text
field recipe + semantic parts + control graph + deformation operators + interaction fields + runtime caches
```

This supports:

```text
procedural creatures
terrain-adaptive motion
dynamic bosses
soft/hard hybrid bodies
semantic weak spots
field-based collision
snow/mud/water interaction
procedural damage
queryable gameplay regions
scalable SDF rendering and physics
```

This design should eventually be reflected in its owning architecture and domain documentation:

```text
ARCHITECTURE.md
docs-site/src/content/docs/guidelines/architecture.md
docs-site/src/content/docs/workspace/crate-inventory.md
domain/animation/
standalone dornglut/runen-sdf
domain/editor/
future domain/physics/
future domain/rendering/
future domain/runtime/
```

## 2.1 Relationship to Existing SDF-First Architecture

This design refines the accepted SDF-first field-product architecture for animated spatial entities. It must align with:

- [`../accepted/sdf-first-field-world-platform-design.md`](../accepted/sdf-first-field-world-platform-design.md)
- [`../accepted/field-product-contracts-diagnostics-and-residency-design.md`](../accepted/field-product-contracts-diagnostics-and-residency-design.md)
- [`../accepted/execution-fabric-and-product-jobs-design.md`](../accepted/execution-fabric-and-product-jobs-design.md)
- [`../accepted/sdf-product-renderer-and-gpu-residency-design.md`](../accepted/sdf-product-renderer-and-gpu-residency-design.md)
- [`../../adr/accepted/0008-adopt-sdf-first-field-product-architecture.md`](../../adr/accepted/0008-adopt-sdf-first-field-product-architecture.md)

The existing SDF-first doctrine remains the parent rule:

```text
Authoritative domain state -> ratified formed products -> derived runtime caches and expression products.
```

For animated SDF assets, this means authoring graphs, semantic part graphs, rig/control graphs, and deformation graphs are descriptions. They must be ratified and lowered before runtime systems consume them. Runtime caches, render proxies, editor overlays, and debug products remain derived state unless an owning product contract explicitly certifies them as strict consumer truth.

This document therefore uses "runtime field plan" as a planning and scheduling product concept, not as a private runtime authority that bypasses product lineage, diagnostics, or domain-owned validation.
## 3. Non-Goals

This document does not define final Rust APIs, exact GPU kernels, serialization syntax, or editor UI layouts.

It also does not ban mesh support, imported rigs, animation clips, motion matching, generated mesh proxies, or neural/compressed SDF representations.

Those may exist as adapters, authoring inputs, fallback representations, or runtime proxies.

They must not define the core architecture.

## 4. Primary Decision

Runenwerk animation should be:

```text
procedural-first
SDF-aware
semantic
simulation-coupled
cache-conscious
compiler-lowered
scheduler-friendly
editor-debuggable
```

The primary abstraction should not be:

```text
AnimatedMesh
```

The primary abstraction should be:

```text
AnimatedSpatialEntity
```

An `AnimatedSpatialEntity` exposes purpose-specific spatial queries:

```text
query_distance(point, time, purpose)
query_material(point, time, purpose)
query_semantic_region(point, time, purpose)
query_velocity(point, time, purpose)
query_interaction(point, time, purpose)
query_bounds(time, purpose)
update_motion(intent, world_feedback, dt)
```

The `purpose` parameter matters because render, physics, gameplay, AI, navigation, VFX, and editor preview systems do not require identical field representations.

Example purposes:

```text
Render
Physics
Gameplay
Navigation
VFX
EditorPreview
Debug
```

## 5. Core Runtime Contract

Every implementation of animated SDF models must preserve these invariants.

### 5.1 Determinism Contract

Gameplay-facing fields must be deterministic for the same input state.

```text
same asset
same animation state
same world feedback
same dt sequence
same query
= same gameplay answer
```

This applies to:

```text
damage regions
weak spots
interaction masks
ability sockets
grab regions
hit classification
AI/navigation affordances
```

Render-only approximation is allowed, but gameplay behavior must not depend on non-deterministic render artifacts.

### 5.2 Semantic Stability Contract

Semantic regions must maintain stable identity across frames.

```text
left_front_leg remains left_front_leg
head_weak_spot remains head_weak_spot
claw_sweep remains claw_sweep
```

Stable IDs are required for:

```text
hit reactions
damage accumulation
network replication
save/load
editor debugging
analytics
tests
```

### 5.3 Physics Conservativeness Contract

Physics fields must be conservative.

A physics field may be simpler than the visual render field, but it must not under-report collision in a way that allows obvious penetration or invalid contact behavior.

```text
Render field:
  may be visually detailed and approximate

Physics field:
  must be stable and conservative

Gameplay field:
  must be deterministic and semantically stable

Editor field:
  may be expensive and diagnostic
```

### 5.4 Bounded Query Cost Contract

No frame system may accidentally query every animated SDF for every ray, sample, contact, or gameplay check.

All high-volume queries must pass through broad-phase filtering:

```text
actor bounds
part bounds
cluster bounds
tile candidate lists
empty-space hierarchy
distance mips
runtime field plan
```

### 5.5 Explicit Approximation Contract

Any approximation must declare its purpose and safety level.

```text
Exact authoring field
Conservative physics proxy
Approximate render proxy
Semantic gameplay field
Debug-only diagnostic field
Far-distance impostor
```

A subsystem must not silently consume the wrong field type.

## 6. Architectural Principles

### 6.1 Do not treat SDFs as prettier meshes

```text
Bad:    SDF model = mesh replacement
Better: SDF model = semantic spatial function with behavior, material, interaction, and simulation meaning
```

### 6.2 Separate intent, control, deformation, simulation, and output

```text
Intent decides what the entity wants.
Control decides where parts should go.
Deformation decides how the field changes.
Simulation decides how the world pushes back.
Compilation decides how editable graphs become runtime plans.
Scheduling decides when jobs execute.
Output systems consume purpose-specific fields.
```

### 6.3 Use hybrid representation

Do not force all animated SDF models into one representation.

Use a hybrid asset model:

```text
Character / Actor
├── analytic or procedural core
├── sculpted SDF details
├── optional compressed or neural detail
├── semantic rig handles
├── deformation zones
├── material regions
├── gameplay regions
├── interaction fields
└── runtime field caches
```

### 6.4 Preserve semantic regions through composition

SDF composition must preserve semantic meaning.

A spider leg is not only geometry. It may also be:

```text
collision volume
damage source
foot contact source
sound source
weak point
material region
animation control target
VFX emitter
AI affordance
```

### 6.5 Lower authoring graphs before runtime execution

Editable authoring graphs should not be executed directly in the runtime hot path.

They should be lowered into:

```text
validated semantic IR
runtime field plan
scheduled job graph
purpose-specific proxies
```

## 7. Domain Ownership

The system crosses multiple domains. Ownership must be explicit to avoid an unmaintainable monolith.

| Artifact | Owner Domain | May Read | May Mutate | Notes |
|---|---|---|---|---|
| Authoring Shape Graph | SDF / Editor | Editor, Asset Compiler | Editor tools | Human-editable source of truth |
| Semantic Part Graph | SDF / Gameplay | Animation, Physics, Rendering, Editor, Gameplay | Asset compiler / editor | Stable region IDs must be preserved |
| Rig / Control Graph | Animation | Editor, Runtime Compiler | Animation authoring tools | Bones, handles, sockets, constraints |
| Motion Graph | Animation | Gameplay, AI, Scheduler | Animation tools | Procedural motors and adapters |
| Field Deformation Graph | SDF + Animation | Runtime compiler, Debugger | Animation/SDF authoring | Warps, transforms, safety metadata |
| Validated Semantic IR | Asset Compiler | Runtime, Editor, Tests | Asset compiler only | Immutable after compilation |
| Runtime Field Plan | SDF Runtime | Scheduler, Rendering, Physics, Gameplay | SDF runtime compiler | Jobs, dependencies, caches, proxies |
| Runtime Field Cache | SDF Runtime | Rendering, Physics, Gameplay, VFX | Scheduled SDF jobs only | Double-buffered where needed |
| Render Proxy | Rendering | Renderer, Debugger | Rendering jobs | May be approximate |
| Physics Proxy | Physics | Physics solver, Debugger | Physics/SDF jobs | Must be conservative |
| Gameplay Interaction Field | Gameplay + SDF | Gameplay, AI, VFX | Scheduled gameplay/SDF jobs | Must be deterministic |
| Editor Diagnostics | Editor | Humans, tests | Editor tools | May be expensive |

## 8. Recommended Documentation Split

```text
domain/animation/
├── procedural-animation.md
├── motion-intent.md
├── control-strategy-hierarchy.md
├── rig-control-graph.md
├── gait-and-locomotion.md
├── ik-and-constraints.md
├── animation-events.md
└── procedural-animation-and-sdf-models.md

dornglut/runen-sdf/docs/
├── sdf-models.md
├── sdf-shape-graph.md
├── semantic-sdf-regions.md
├── sdf-deformation.md
├── sdf-character-authoring.md
├── sdf-runtime-caching.md
└── sdf-render-physics-proxies.md

domain/runtime/
├── sdf-animation-compilation-pipeline.md
├── runtime-field-plan.md
├── deferred-field-mutation.md
├── job-graph-lowering.md
└── cache-invalidation-doctrine.md

domain/physics/
├── sdf-collision.md
├── contact-fields.md
├── physics-proxy-policy.md
├── simulation-islands.md
├── interaction-fields.md
└── softbody-sdf-candidates.md

domain/rendering/
├── animated-sdf-rendering.md
├── deformed-field-tracing.md
├── render-field-selection-policy.md
├── distance-mips.md
└── temporal-reprojection.md

domain/editor/
├── sdf-model-editor.md
├── procedural-animation-editor.md
├── debug-visualizers.md
├── asset-composition-workflows.md
└── authoring-validation.md
```

## 9. Compiler-Style Lowering Pipeline

The runtime should not consume authoring graphs directly.

Canonical PlantUML source: [diagrams/animated-sdf-compilation-pipeline.puml](diagrams/animated-sdf-compilation-pipeline.puml).

Pipeline stages:

```text
1. Authoring graphs are edited by humans or generated tools.
2. Asset compiler validates semantics, deformation safety, and proxy requirements.
3. Graphs lower into an immutable semantic IR.
4. Runtime compiler produces field plans and dirty-region rules.
5. Scheduler executes field jobs in dependency order.
6. Output systems consume purpose-specific fields and proxies.
```

## 10. High-Level Runtime Architecture

Canonical PlantUML source: [diagrams/animated-sdf-runtime-architecture.puml](diagrams/animated-sdf-runtime-architecture.puml).

## 11. Frame Execution Policy

Animated SDF updates must be scheduler-aware.

Recommended frame order:

```text
1. Read gameplay/AI intent.
2. Read previous-frame physics and world feedback.
3. Run motion planning and motor controllers.
4. Solve constraints, IK, contacts, and procedural controllers.
5. Emit deferred animation and field mutations.
6. Apply mutation sync point.
7. Update runtime field plan and dirty regions.
8. Execute field deformation/cache jobs.
9. Generate or refresh render/physics/gameplay/VFX proxies.
10. Run physics contacts and simulation islands.
11. Run gameplay interaction queries.
12. Render using tile candidate lists, distance mips, reprojection, or impostors.
13. Publish debug data for editor and tests.
```

Rules:

```text
Structural ECS changes are deferred.
Field cache writes are scheduled jobs.
Purpose-specific proxies are published at sync points.
Readers consume stable snapshots.
Long-running rebuilds must be incremental or double-buffered.
```

## 12. Core Asset Model

```text
Animated SDF Asset
├── Asset Composition Layers
│   ├── base creature
│   ├── biome variant
│   ├── material variant
│   ├── equipment/module variant
│   ├── boss phase override
│   ├── damage layer
│   └── runtime mutation layer
├── Shape Graph
│   ├── primitives
│   ├── blends
│   ├── booleans
│   ├── trims
│   ├── sculpted patches
│   └── material regions
├── Semantic Part Graph
│   ├── body parts
│   ├── weak spots
│   ├── sockets
│   ├── contact regions
│   ├── damage regions
│   ├── affordance regions
│   └── VFX emission regions
├── Rig / Control Graph
│   ├── bones
│   ├── handles
│   ├── pivots
│   ├── sockets
│   ├── IK goals
│   └── constraints
├── Motion Graph
│   ├── procedural locomotion
│   ├── gait phases
│   ├── steering intent
│   ├── reactive controllers
│   ├── optional clips
│   ├── optional motion warping
│   ├── optional motion matching
│   └── optional learned motion
├── Field Deformation Graph
│   ├── coordinate warps
│   ├── articulated part transforms
│   ├── muscle bulges
│   ├── squash and stretch
│   ├── impact dents
│   ├── breathing
│   ├── procedural noise
│   └── deformation safety metadata
└── Runtime Products
    ├── semantic IR
    ├── runtime field plan
    ├── local SDF bricks
    ├── distance mips
    ├── dirty regions
    ├── aggregate bounds
    ├── previous-frame field
    ├── velocity field
    └── render/physics/gameplay/VFX proxies
```

## 13. Asset Composition Model

Runenwerk should support layered animated SDF assets.

This is needed for:

```text
creature variants
biome adaptations
material overrides
boss phases
damage states
modular limbs
equipment
mutations
editor collaboration
runtime procedural generation
```

Recommended composition layers:

```text
Base Layer
  canonical creature shape, parts, rig, default materials

Variant Layer
  biome, sex, age, mutation, faction, rarity, scale, silhouette changes

Material Layer
  skin, armor, fur, shell, slime, fire, ice, wetness, emissive rules

Animation Layer
  gait sets, posture rules, attack controllers, personality modifiers

Gameplay Layer
  weak spots, damage regions, sockets, abilities, affordances

Damage Layer
  wounds, broken armor, severed parts, deformation scars

Runtime Override Layer
  temporary effects, procedural growth, buffs, debuffs, status effects
```

Composition must resolve into one immutable semantic IR before runtime execution.

## 14. Purpose-Specific Field Model

Do not force every subsystem to consume the same field.

```text
Authoring Field
  high quality, editable, semantic, expensive allowed

Render Field
  visually detailed, may be approximate, optimized for raymarch/raster/impostor use

Physics Field
  conservative, stable, simplified, contact-safe

Gameplay Field
  deterministic, semantic, stable across frames

Navigation Field
  coarse, pathfinding-friendly, affordance-aware

VFX Field
  emission/contact/material influence oriented

Debug Field
  diagnostic, explainable, may be expensive
```

Subsystem consumption rules:

| Subsystem | Field Type | Hard Requirement |
|---|---|---|
| Renderer | Render Field | Fast, LOD-aware, visually acceptable |
| Physics | Physics Field | Conservative, stable normal/contact behavior |
| Gameplay | Gameplay Field | Deterministic semantic answers |
| AI / Navigation | Navigation Field | Coarse, stable, affordance-aware |
| VFX | VFX Field | Contact/emission/material influence data |
| Editor | Authoring + Debug Fields | Explainable and inspectable |

## 15. Procedural Animation Control Stack

Procedural animation should be first-class.

```text
Intent Layer
  goal, target, urgency, emotion/personality, tactical state

Planner Layer
  choose action, path, attack family, body posture, approach vector

Motor Layer
  gait oscillator, procedural attack, steering, turning, aiming, balance

Constraint Layer
  IK, foot locking, look-at, joint limits, self-collision, contact constraints

Motion Adapter Layer
  optional clips, motion warping, motion matching, learned motion provider

Deformation Layer
  part transforms, coordinate warps, muscle response, squash/stretch, impact dents

Field Output Layer
  render, physics, gameplay, VFX, navigation, debug fields
```

Important rule:

```text
Clips, motion matching, and learned motion are motor providers.
They do not own the architecture.
```

## 16. Animation Strategies

### 16.1 Articulated SDF Parts

Each body part is represented as an SDF subtree.

Good for:

```text
robots
insects
monsters
skeletons
armor plates
hard-surface creatures
modular creatures
```

Example:

```text
Spider
├── abdomen SDF
├── thorax SDF
├── head SDF
├── leg_01_upper SDF
├── leg_01_lower SDF
├── leg_01_claw SDF
└── ...
```

Important rule:

```text
Use semantic blend zones, not global smooth unions.
```

A blend zone should define:

```text
joint attachment
blend radius
blend hardness
material transition
volume preservation rule
animation phase behavior
damage propagation rule
semantic propagation rule
```

### 16.2 Coordinate-Space Deformation

The query point is warped before evaluating the SDF:

```text
distance = sdf(inverse_warp(point))
```

Good for:

```text
tails
tentacles
breathing
muscle flex
squash/stretch
jelly creatures
soft monsters
stylized motion
```

Risk:

```text
Nonlinear deformation can break strict signed-distance correctness.
```

Mitigation:

```text
track deformation bounds
use conservative distance estimates
estimate local scale / Lipschitz factors
cap raymarch steps
fallback to cached corrected fields
debug gradient quality
separate render approximation from physics conservativeness
```

### 16.3 Procedural Primitive Rigs

Some creatures should not be bone-first.

```text
Slime
├── center mass
├── volume preservation
├── pseudopod emitters
├── surface waves
├── contact pressure fields
└── material response

Snake
├── path-following spine
├── traveling body wave
├── terrain contacts
├── muscle bulge wave
├── head-look controller
└── surface detail flow

Spider
├── body balance solver
├── 8 leg phase oscillators
├── terrain foot probes
├── foot locking
├── abdomen breathing
├── mandible controller
└── hair/noise field
```

### 16.4 Cached Dynamic Fields

For expensive deformation or high-detail bosses, build local temporary fields.

```text
Animated Actor Cache
├── static base SDF
├── current pose field
├── previous pose field
├── dirty bricks
├── velocity field
├── normal cache
├── semantic region cache
└── contact cache
```

Use for:

```text
large bosses
destructible creatures
softbody monsters
high-detail faces
localized wounds
mud/snow/water interaction
```

## 17. SDF Correctness Policy

SDF animation must track field quality explicitly.

```text
ExactSdf
  true or near-true signed distance field

ConservativeDistanceBound
  safe for physics/ray stepping but may overestimate distance behavior conservatively

ApproximateVisualField
  visually useful but not safe for gameplay or physics

SemanticMaskField
  stable region classification, not necessarily distance-accurate

CachedCorrectedField
  rebuilt or corrected local field used when deformation makes direct evaluation unsafe
```

Required metadata:

```text
local bounds
part bounds
deformation bounds
gradient quality estimate
semantic region ownership
material region ownership
velocity estimate
cache invalidation dependencies
fallback policy
```

## 18. Rendering Policy

Animated SDF rendering must not evaluate every actor against every ray or tile.

Required acceleration layers:

```text
actor bounds
part bounds
dynamic actor clusters
per-tile candidate lists
empty-space hierarchy
distance mips
temporal reprojection
impostors / generated proxies
dirty-region field rebuilding
```

Render field selection policy:

```text
Near camera:
  full animated render field or high-quality cached field

Medium distance:
  cached field + distance mips + simplified material regions

Far distance:
  impostor, generated mesh/proxy, or aggregate field

Occluded:
  bounds and temporal state only

Physics-only actor:
  no render field required

Editor debug:
  selectable overlays for distance, normals, gradients, semantics, caches, and proxies
```

Renderer may consume approximate fields, but it must not feed those approximate answers back into deterministic gameplay or conservative physics.

## 19. Physics Policy

Physics should consume conservative fields, not the full render SDF by default.

```text
Physics Field
├── stable distance query
├── conservative bounds
├── contact normal
├── velocity field
├── material response
├── semantic body-part ID
├── contact manifold support
└── simplified collision proxy
```

Physics should support:

```text
signed-distance collision gradients
contact manifolds
simulation islands
sleep/wake behavior
conservative proxy generation
softbody candidate fields
rigid/soft hybrid actors
```

Physics feedback should feed animation and deformation:

```text
contact pressure
ground normal
sliding state
impact impulse
penetration correction
support polygon
stability
sleep/wake state
```

Important rule:

```text
The physics proxy is allowed to be simpler than the render field.
It is not allowed to be less safe than the render field in a way that breaks collision correctness.
```

## 20. Interaction and Material Fields

Animated SDF entities should expose more than collision.

Recommended interaction fields:

```text
collision distance
damage distance
grab distance
bite / claw / weapon distance
heat distance
sound influence
smell influence
affordance masks
weak spot masks
```

Recommended material fields:

```text
contact pressure field
wetness field
heat field
snow compression field
mud adhesion field
dust emission field
fluid displacement field
damage residue field
blood / slime / acid emission field
```

Field lifecycles:

```text
Persistent
  stored across frames or saved as actor/world state

FrameLocal
  valid only for one frame

Accumulated
  integrates over time, such as wetness, heat, damage, snow compression

Predicted
  used for anticipatory VFX, AI, animation, or networking

Replicated
  deterministic/network-relevant gameplay field

EditorOnly
  diagnostic or visualization field
```

Example queries:

```text
Is this point inside the claw sweep?
How far is this snow surface from the foot?
Where should mud stick?
Where can the player grab?
Where does fire emit?
Where does armor block damage?
Which semantic region was hit?
```

## 21. Caching and Invalidation Doctrine

Caching is mandatory. Invalidation rules must be explicit.

Per actor cache:

```text
AnimatedSdfActorCache
├── part bounds
├── deformation bounds
├── semantic dirty flags
├── pose hash
├── previous pose hash
├── local brick cache
├── distance mip cache
├── velocity cache
├── semantic region cache
├── interaction field cache
└── aggregate bounds
```

World-scale cache:

```text
SdfWorldRuntime
├── sparse world bricks
├── dynamic cluster hierarchy
├── per-tile candidate lists
├── empty-space hierarchy
├── distance mips
├── temporal reprojection
├── dirty-region field rebuilding
├── actor aggregate bounds
└── scheduled rebuild queues
```

Invalidation triggers:

| Change | Invalidates |
|---|---|
| Actor transform | actor bounds, tile candidate lists, aggregate bounds |
| Part transform | part bounds, local field cache, velocity field, render/physics proxies |
| Deformation parameter | deformation bounds, local bricks, gradient quality, distance mips |
| Semantic region edit | semantic cache, gameplay field, debug overlays, tests |
| Material edit | render field, VFX field, material interaction cache |
| Damage event | semantic region state, damage layer, physics proxy, render proxy, gameplay field |
| Terrain contact | contact cache, material interaction fields, physics feedback |
| LOD transition | render proxy, impostor, distance mip selection |
| Asset variant swap | semantic IR, runtime field plan, all derived proxies |

Key rule:

```text
Do not ask every SDF about every ray, sample, or contact.
Ask spatial indexes which SDFs could matter.
```

## 22. Editor and Authoring

The editor should author semantic SDF animation, not only poses.

Required editor concepts:

```text
SDF Model Editor
├── Shape Graph
├── Semantic Parts
├── Blend Zones
├── Material Regions
├── Rig Handles
├── Procedural Controllers
├── Interaction Fields
├── Asset Composition Layers
├── Runtime Cache Preview
└── Debug Field Views
```

Procedural animation editor:

```text
Procedural Animation Editor
├── motion intent preview
├── gait phase editor
├── foot/contact probes
├── IK and constraint graph
├── balance visualization
├── body-part semantic overlays
├── terrain adaptation preview
├── motion adapter preview
└── event timeline
```

Mandatory debug views:

```text
distance field
normal field
gradient quality
semantic regions
blend zones
deformation bounds
dirty bricks
contact points
IK targets
gait phases
collision proxy
render proxy
gameplay masks
query-cost heatmap
cache invalidation visualization
```

Validation workflows:

```text
field correctness heatmaps
semantic-region stability tests
deformation stress tests
gait playback tests
cache invalidation replay
query-cost profiling
physics proxy conservativeness checks
deterministic playback capture
```

## 23. Example: Procedural Spider

Canonical PlantUML source: [diagrams/procedural-spider-animated-sdf.puml](diagrams/procedural-spider-animated-sdf.puml).

A procedural spider should be able to walk on caves, ceilings, trees, rocks, webs, and uneven generated terrain without handcrafted clips for every case.

## 24. Example: Boss Monster

```text
Boss Animation
├── Combat Intent
│   ├── attack target
│   ├── threat direction
│   ├── desired arena control
│   └── rage phase
├── Planner / Motor
│   ├── root motion
│   ├── weight shift
│   ├── turn-in-place
│   ├── impact anticipation
│   ├── recovery
│   └── phase transition controller
├── Procedural Attacks
│   ├── claw arc solver
│   ├── tail sweep solver
│   ├── stomp solver
│   ├── roar pose solver
│   └── projectile socket solver
├── SDF Response
│   ├── muscle bulge
│   ├── skin tension
│   ├── scars opening
│   ├── armor plates shifting
│   ├── impact dents
│   └── weak spot exposure
├── Runtime Fields
│   ├── render field
│   ├── conservative physics proxy
│   ├── deterministic gameplay regions
│   └── VFX emission/contact fields
└── World Coupling
    ├── terrain cracks
    ├── dust / snow displacement
    ├── influence map update
    └── camera shake event
```

Bosses are a strong validation target because they combine:

```text
semantic weak spots
animated armor plates
large contact fields
deformation from impacts
terrain interaction
VFX emission regions
procedural attacks
LOD pressure
cache invalidation pressure
```

## 25. Alternative Designs Considered

### Alternative A: Mesh-like skeletal SDF animation

```text
Use bones, skin weights, and clips as the primary model.
Adapt SDFs to behave like skinned meshes.
```

Rejected as primary architecture.

Reason:

```text
It imports mesh-era assumptions and underuses SDF strengths.
Acceptable only as compatibility or import support.
```

### Alternative B: Pure analytic SDF animation

```text
All models are procedural primitives and equations.
```

Rejected as the only representation.

Reason:

```text
Excellent for procedural creatures and stylized forms,
but too restrictive for sculpted characters, detailed bosses, and artist-driven assets.
```

### Alternative C: Pure voxel or brick SDF animation

```text
All animated models are stored as editable SDF volumes.
```

Rejected as the only representation.

Reason:

```text
Good for sculpting and destructive deformation,
but expensive and less semantically expressive unless paired with higher-level graphs.
```

### Alternative D: Pure neural SDF animation

```text
Use learned fields as the primary shape representation.
```

Rejected for the core engine layer.

Reason:

```text
Potentially useful later for compression, completion, or high-detail reconstruction,
but hard to make deterministic, debuggable, editable, and simulation-stable.
```

### Alternative E: Runtime-only procedural graph execution

```text
Execute editor-authored procedural graphs directly at runtime.
```

Rejected.

Reason:

```text
Too hard to validate, schedule, cache, and optimize.
Use compiler-style lowering instead.
```

### Alternative F: One universal field for all systems

```text
Rendering, physics, gameplay, VFX, and navigation all query one field.
```

Rejected.

Reason:

```text
Different systems need different correctness/performance guarantees.
Purpose-specific fields are safer and more scalable.
```

### Chosen approach: Hybrid semantic SDF animation

```text
Use analytic/procedural fields where possible,
sculpted SDF patches where needed,
cached runtime fields for expensive deformation,
purpose-specific proxies for runtime systems,
and optional learned/compressed representations only as specialized layers.
```

Accepted.

Reason:

```text
Best long-term fit for procedural animation, editor tooling, simulation, scale, and debugging.
```

## 26. External Domain Inspirations

The architecture intentionally borrows from adjacent domains:

```text
Compiler design
  high-level authoring graphs lower into validated IR and runtime plans

USD-style asset composition
  layered assets, variants, payload-like heavy data, overrides, resolved runtime assets

Robotics/control
  motion intent, trajectory refinement, collision gradients, control hierarchy

Game animation systems
  motion warping, motion matching, procedural motors, animation events

Physics engines
  contact manifolds, simulation islands, conservative proxies, sleep/wake policy

VFX simulation
  material interaction fields, particle/constraint systems, MPM-like snow/sand/mud thinking

ECS/data-oriented design
  relationships for semantic parts, deferred mutation, scheduled sync points, job graphs

Rendering acceleration
  tile candidate lists, distance mips, impostors, temporal reprojection, dirty-region rebuilding
```

These are inspirations, not dependencies.

## 27. Risks and Mitigations

| Risk | Impact | Mitigation |
|---|---:|---|
| Nonlinear deformation breaks true distance quality | High | Track deformation bounds, conservative stepping, gradient debug views, cached corrected fields |
| Runtime SDF queries become too expensive | High | Actor bounds, clusters, per-tile candidate lists, distance mips, dirty-region rebuilding |
| Semantic regions get lost during blending | High | Semantic blend zones, region propagation rules, debug overlays, semantic stability tests |
| Editor becomes too complex | High | Separate shape, semantics, rig, deformation, composition, and debug panels |
| Physics/render fields diverge unpredictably | High | Purpose-specific field contracts and validation tests |
| Procedural animation becomes hard to author | Medium | Gait editors, constraint visualizers, contact probes, reusable controller templates |
| Imported assets do not map cleanly | Medium | Treat imports as adapters and lower them into semantic IR |
| Too many one-off creature controllers | Medium | Standardize motor/constraint/deformation graph interfaces |
| Cache invalidation becomes correctness debt | High | Invalidation doctrine, replay tests, debug visualization |
| Runtime graph becomes unschedulable | High | Lower to explicit job dependency graph with sync points |

## 28. ADR Candidates

The following decisions should become ADRs before implementation:

```text
ADR: AnimatedSpatialEntity ownership and crate location
ADR: Semantic region ID model
ADR: Purpose-specific field contract
ADR: SDF animation lowering pipeline
ADR: Runtime field plan ownership
ADR: Cache invalidation doctrine
ADR: Physics proxy conservativeness policy
ADR: Asset composition model
ADR: Deferred mutation and scheduler sync points
ADR: First validation creature
```

## 29. Implementation Phases

### Phase 0: Documentation and Doctrine

Create/update:

```text
docs-site/src/content/docs/design/active/sdf-procedural-animation-and-animated-models-design.md
standalone dornglut/runen-sdf docs for sdf-deformation
standalone dornglut/runen-sdf docs for semantic-sdf-regions
domain/editor/procedural-animation-editor.md
future domain/physics/interaction-fields.md
future domain/rendering/animated-sdf-rendering.md
future domain/runtime/sdf-animation-compilation-pipeline.md
future domain/runtime/cache-invalidation-doctrine.md
```

Update current repository authority only when the corresponding facts change:

```text
ARCHITECTURE.md
docs-site/src/content/docs/guidelines/architecture.md
docs-site/src/content/docs/workspace/crate-inventory.md  # only when workspace membership changes
```

### Phase 0.5: Contracts and Test Fixtures

Before a visual prototype, create tests and fixtures:

```text
golden SDF query samples
deterministic playback tests
semantic-region stability tests
cache invalidation replay tests
deformation stress tests
query budget tests
physics conservativeness tests
debug view requirements
```

### Phase 1: Minimal Semantic SDF Actor

Build a non-final prototype model:

```text
SdfAnimatedModel
├── shape graph
├── semantic parts
├── simple articulated transforms
├── semantic IR
├── render field query
├── physics field query
└── gameplay field query
```

Target validation asset:

```text
procedural spider leg or tentacle
```

### Phase 2: Runtime Field Plan and Scheduler Integration

Implement:

```text
runtime field plan
dirty-region tracking
job dependency graph
deferred mutation sync point
field cache snapshots
```

### Phase 3: Procedural Motor and Constraint Prototype

Implement:

```text
motion intent
simple gait oscillator
IK/contact probes
foot locking
terrain adaptation
animation events
```

### Phase 4: Field Deformation Graph

Implement:

```text
coordinate warp
part transform
joint blend zone
breathing pulse
impact dent
deformation bounds
gradient quality debug
```

### Phase 5: Runtime Caching and Proxies

Implement:

```text
actor aggregate bounds
part bounds
local brick cache
distance mip generation
previous-frame field
velocity field
render proxy
physics proxy
gameplay field cache
```

### Phase 6: Editor Debugging

Implement debug views before expanding feature scope:

```text
distance
normal
gradient quality
semantic parts
blend zones
dirty bricks
contact points
gait phase
physics proxy
render proxy
query cost
cache invalidation
```

### Phase 7: Creature Validation

Validate against multiple morphology types:

```text
spider
slime
snake
boss monster
hard-surface robot
softbody creature
```

## 30. Acceptance Criteria

The architecture is acceptable when Runenwerk can express all of the following without changing the core design:

```text
a spider that walks on walls and ceilings
a slime that deforms from contacts
a boss with animated weak spots and armor plates
a snake/tentacle driven by procedural waves
a creature whose feet displace snow/mud/water
a physics-safe simplified field and a detailed render field
a deterministic gameplay field separate from visual approximation
a debug view explaining why a ray/contact/interaction query behaved as it did
```

Measurable acceptance criteria:

```text
Semantic IDs remain stable across deterministic playback.
Gameplay queries produce deterministic results for identical inputs.
Physics proxies are conservative relative to declared collision requirements.
Dirty-region rebuilds are bounded and explainable.
No high-volume renderer/physics/gameplay query bypasses broad-phase filtering.
Cache invalidation can be replayed and inspected.
Every purpose-specific field declares its correctness level.
Every generated proxy can be traced back to source asset layers and semantic IR.
Editor can visualize distance, gradient, semantic region, cache, and proxy state.
```

## 31. Final Design Summary

Runenwerk should treat procedural animation and SDF models as a unified spatial architecture.

The core should be:

```text
Procedural Animation
+ Semantic SDF Parts
+ Field Deformation
+ Interaction Fields
+ Purpose-Specific Runtime Caches
+ Compiler-Style Lowering
+ Scheduler-Aware Execution
```

Meshes, clips, motion matching, learned motion, generated meshes, and neural SDFs may be supported as adapters, sources, or proxies.

They should not define the architecture.

The long-term advantage is a system where animated entities are not merely rendered shapes, but queryable, semantic, simulated spatial participants in the world.
