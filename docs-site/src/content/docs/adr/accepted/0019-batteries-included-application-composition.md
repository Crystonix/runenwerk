---
title: Batteries-Included Application Composition
description: Accepted product-composition and usability laws that preserve Runenwerk's explicit internal ownership while providing simple, transparent ordinary application paths.
status: accepted
owner: workspace
layer: architecture
canonical: true
last_reviewed: 2026-08-12
related_adrs:
  - ./0014-repository-family-extraction-boundaries.md
  - ./0017-cross-authority-consistency-and-graph-semantics.md
  - ./0018-semantic-federation-and-physical-realization.md
related_docs:
  - ../../architecture/repository-family-architecture.md
  - ../../design/active/runenwerk-domain-workbench-north-star.md
  - ../../design/active/net-declarative-replication-authoring.md
  - ../../design/active/net-plugin-runtime-bridge.md
  - ../../net/multiplayer-replication-implementation-roadmap.md
  - ../../reports/investigations/2026-08-12-application-composition-and-networking-ergonomics.md
---

# ADR 0019: Batteries-Included Application Composition

## Decision

Runenwerk adopts a product-composition doctrine that keeps the internal architecture
explicit and decomposed while making supported ordinary application paths deliberately
simple.

The short form is:

```text
Inside:
  explicit semantic authorities
  typed owner contracts
  owner plugins
  specialized realizations

Outside:
  one App composition root
  transparent batteries-included product groups
  visible product/domain intent
  progressive disclosure into direct expert control
```

The governing usability law is:

> **Internal decomposition must not determine application complexity.**

A correct internal split among RunenUI, RunenRender, RunenGPU, RunenSpatial, RunenSDF,
RunenECS, networking, world systems, and other owners must not force routine application
code to manually reconstruct that split.

This decision complements ADR 0017 and ADR 0018. It does not weaken their ownership,
admission, realization, or extraction laws. It defines how those laws are presented at
the product/application boundary.

## Existing implementation facts

This ADR does not claim the target is fully implemented.

At the accepted evidence baseline, Runenwerk already has the correct mechanical
composition root:

```text
App::new()
App::headless()
App::add_plugin(...)
App::add_plugins(...)
```

`IntoPlugins` already accepts individual plugins, boxed plugins, vectors, and tuples.
`engine::plugins::default_plugins()` already provides a narrow runtime baseline.

Current products nevertheless repeat portions of common engine capability assembly.
Runenwerk Draw and Runenwerk Editor both compose the runtime baseline, diagnostics,
scene, and rendering before adding product-specific behavior. Runtime Preview shows
that a genuinely small product can already remain small where its capability set is
small.

`App` construction also currently installs a broader builtin resource set than the
plugin baseline. The `engine` package currently carries a broad compile-time dependency
surface even when a product omits those capabilities at runtime.

Those are current implementation facts. This ADR establishes future ownership and
usability laws; it does not silently move resources, alter dependencies, or claim that
current headless builds are already dependency-minimal.

## One application composition root

Runenwerk uses one runtime composition root:

```text
App
```

Product convenience must lower into the existing `App`, ordinary plugins, typed
resources/configuration, and owner contracts.

Do not create another live application runtime merely to make setup shorter.

Rejected as the normal convenience mechanism:

```text
parallel RunenApp runtime
persistent meta-application configuration authority
untyped dependency-injection container
universal service locator
meta-executor that interprets product intent beside App
```

A future convenience builder or preset may exist only as **ephemeral construction
syntax** when useful:

```text
product intent / helper
    -> constructs or configures ordinary product groups/plugins/resources
    -> disappears as an independent source of truth
```

It must not remain a second runtime configuration database.

## Product/plugin groups

Runenwerk accepts the architectural concept of a **product/plugin group**:

> **A product/plugin group is an ordered, inspectable composition recipe over existing
> plugins and owner configuration. It is not semantic authority.**

The group answers a product-composition question:

```text
Which already-owned capabilities form this supported application path,
and with which default configuration/order?
```

It does not answer:

```text
Who owns renderer semantics?
Who owns GPU resources?
Who owns world state?
Who owns network replication truth?
```

Those questions remain with their existing authorities.

### Required qualities of a future group mechanism

A concrete implementation should provide only the machinery proven necessary for:

```text
named/debuggable membership
stable deterministic ordering
composition of groups where meaningful
member configuration/replacement
member removal/disable where legal
explicit duplicate/incompatibility diagnostics
inspection of the effective selected stack
```

Removal, replacement, and reordering are conditional on owner invariants. Invalid
composition must reject explicitly rather than silently producing a partially broken
product.

This ADR does not select the Rust type names, public constructors, group memberships,
or exact customization API.

## Avoid product-preset combinatorics

Do not encode every capability cross-product as a separate product preset.

Rejected direction:

```text
DesktopMultiplayerEditorSdfGameWith...
```

Prefer a small orthogonal set of composable groups/capabilities whose combinations are
explicitly validated.

For example, the architecture may eventually distinguish concepts directionally such
as:

```text
minimal/core runtime
windowed/local-game capabilities
tool/workbench capabilities
dedicated-server capabilities
client/server networking role capabilities
```

These names and memberships are illustrative only. Implementation authority must prove
the smallest useful set from current product examples and owner dependencies.

## Batteries-included means transparent defaults

Runenwerk adopts:

> **A supported batteries-included default must be explainable as explicit selected
> owner plugins and configuration even when the user does not spell them individually.**

Consequences:

- defaults are documented and inspectable;
- effective group membership/order can be diagnosed;
- selected owner configuration remains explicit and typed;
- advanced users can replace or remove members where the relevant invariants permit;
- invalid subtraction or reordering rejects with useful diagnostics;
- the ordinary path and expert path use the same owner implementations;
- convenience does not create hidden compatibility layers or mirrored state.

A default may be opinionated. It must not be mysterious.

## Product intent remains explicit

Product groups remove **generic Runen integration ceremony**. They do not erase
application/domain intent.

The following normally remain product-owned or domain-owned:

```text
game systems and rules
product-specific render methods/flows
input actions and bindings
world/procedural generation policy
editor/workbench behavior
native specialist adapters such as tablet input
network game protocol/components/input/ownership intent
custom compression/packing/streaming policy
application-specific diagnostics or quality policy
```

Candidate generic Runenwerk integration includes wiring that every selected supported
product would otherwise reproduce, such as:

```text
standard owner-plugin combination/order
routine cross-framework adapters
network role -> simulation/world authority integration
standard registered ECS replication extraction/application once implemented
common product lifecycle wiring
```

The test is ownership, not line count. Runenwerk must not steal domain semantics merely
to shorten application examples.

## Progressive disclosure is a whole-product law

Runenwerk applies progressive disclosure beyond individual subsystems.

### Ordinary path

The ordinary path should emphasize product concepts:

```text
supported product group(s)
+ product plugin / domain declarations
+ run
```

A routine application author should not need to understand cross-authority admission,
realization envelopes, provenance joins, framework adapter chains, or inspection IDs
merely to obtain the supported default integration.

### Configuration path

Users who need different product choices configure the selected groups and owner
plugins explicitly:

```text
replace/configure selected members
add/remove supported optional capabilities
set owner-specific policy
inspect effective composition
```

### Expert path

Direct lower-level control remains available for proven advanced consumers:

```text
direct owner/framework plugins
custom render flows and methods
custom networking driver
custom transport adapter
custom world-streaming representation
specialized framework adapters
explicit lower-level owner contracts
```

The expert path is not a different runtime. It exposes more of the same underlying
architecture.

## Common integration wiring belongs to Runenwerk

Runenwerk owns integration-specific composition among selected frameworks where the
wiring is genuinely product-generic.

The law is:

> **When several supported applications would otherwise reproduce the same
> cross-framework integration wiring, Runenwerk should own that wiring once. The
> application keeps the semantic choices that are actually application-specific.**

This follows ADR 0014's adapter direction: peer frameworks stay independently useful;
Runenwerk owns product integration without forcing peer frameworks to depend back on
Runenwerk.

Where canonical typed owner APIs already express the needed facts, product composition
and tooling should reuse or derive from those contracts rather than maintaining a
parallel hand-authored metadata model, consistent with ADR 0018.

## App builtin-state discipline

Runenwerk adopts this target ownership law:

> **`App` construction should own only genuine application-runtime invariants and
> cheap state required across supported App modes. Domain/product capability state
> should normally be installed by its owning plugin/group unless a separately proven
> universal invariant requires it at App construction.**

This law is intentionally stronger than the current implementation shape.

Current `App` construction installs resources spanning time/input/window scaffolding,
scene/runtime state, UI overlay state, gameplay configuration, product/query
publication, runtime jobs/cache, and simulation state.

This ADR does **not** classify or move those resources. Doing so could change runtime
behavior and requires a separate current-source design/cleanup issue. Until that work is
accepted, documentation and tests must continue to describe current behavior honestly.

The law exists to prevent future batteries-included work from solving convenience by
quietly adding more unrelated global builtin state.

## Runtime composition is not compile-time modularity

Runenwerk explicitly distinguishes:

```text
runtime/product composition ergonomics
!=
Cargo dependency / compile-time / binary-size modularity
```

A product group can avoid activating rendering, UI, networking, or another subsystem at
runtime without proving that the corresponding dependencies are absent from the engine
package, build graph, or final binary.

The current `engine` package has a broad dependency surface. This ADR does not redesign
Cargo features or package topology and must not claim that a future `DedicatedServer`
product group automatically removes WGPU/Winit or other dependencies from compilation.

If compile time, binary size, platform availability, or package publication creates
real pressure, those concerns require a separate dependency/feature-topology design.

## Built-in Runen networking

Runenwerk preserves its existing custom networking architecture.

The product-level law is:

> **Built-in Runen networking means Runen owns the game-network semantic contracts and
> supplies a low-boilerplate ordinary path for supported gameplay cases. It does not
> mean Runen must reimplement transport, TLS/crypto, or OS networking primitives.**

Runen-owned networking semantics include, according to their current owners:

```text
authoritative session and replication contracts
snapshots / deltas / ACK / baseline / resynchronization
prediction / correction / reconciliation
interest and streaming integration
simulation / history / replay integration
diagnostics and inspection
typed/declarative component/entity/input/protocol authoring
product-level client/server/host role composition
```

Contained realizations may own lower-level mechanics such as:

```text
QUIC/socket implementation
TLS/crypto primitives
OS networking backend behavior
future transport adapters
```

The current Quinn-based `engine_net_quic` realization does not make Quinn the owner of
Runen replication semantics.

## Ordinary networking path

The accepted declarative networking direction remains the normal target:

```text
mark/register replicated entities and components
register input streams and ownership routing
write ordinary authoritative ECS/game systems
Runen integration handles standard extraction/snapshots/deltas/apply/ACK/replay plumbing
```

At the evidence baseline this path is incomplete: declarative macros primarily provide
metadata, and ordinary gameplay still often requires a custom `ReplicationDriver`.

That gap is already owned by the networking design and implementation roadmap. In
particular, standard ECS component extraction/application and the later public usage
path must complete before documentation can claim custom-driver-free ordinary
multiplayer authoring.

This ADR does not implement or duplicate that roadmap.

### Expert networking path

Custom drivers remain legitimate and supported for semantically different workloads,
including:

```text
aggregate snapshots
custom compression
external non-ECS state
large-scale world-streaming representations
rollback-optimized packing
unusual delta formats
```

The existence of an expert path is not justification for making every ordinary game
write one.

## Managed backend services are a separate concern

Runenwerk distinguishes authoritative game-state networking from managed application
backend services.

```text
Runen game networking
  simulation/replication/prediction/interest/history semantics

managed backend services
  authentication/cloud persistence/object storage/lobbies/presence/
  backend functions/payments/etc.

low-level realization
  transport/storage/HTTP/SDK/OS primitives
```

Current evidence does not authorize a generic `OnlineServices` framework or any
provider dependency.

Provider-specific integrations may later be useful. A provider-neutral abstraction may
also eventually emerge. Either requires concrete product/provider proof defining
security, persistence, operational, identity, and ownership contracts.

The convenience offered by a hosted backend must not be used as evidence that Runen's
authoritative replication model should be replaced by that provider's realtime API.

## No hidden global lookup as convenience

Runenwerk rejects these as the primary product-composition mechanism:

```text
untyped service locator
universal mutable registry
autodiscovery that silently changes runtime composition
implicit global/latest authority lookup
```

Typed owner resources/contracts and explicitly selected product groups remain the
normal direction.

Convention, project templates, or scaffolding may later reduce project-creation work,
but generated or implicit setup must not become a second runtime authority.

## User-path fitness functions

Future implementation and documentation should be evaluated against these durable
product paths.

### Minimal / headless

- one obvious construction path;
- no requirement to install a broad windowed/game product group;
- rendering/UI behavior is not activated merely because those systems exist elsewhere;
- runtime simplicity does not falsely imply compile-time dependency minimality.

### Ordinary local game

- one obvious supported product composition path;
- common framework/integration wiring is not repeated in every game;
- product-specific game/render/input/world intent remains visible;
- routine spawning/game systems do not manually wire renderer/GPU authority boundaries.

### Dedicated authoritative server

- headless authoritative composition does not activate window/render/UI product
  behavior;
- network/simulation/world authority defaults are explicit and inspectable;
- generic role integration is not reconstructed by each game;
- compile-time dependency subtraction is claimed only when separately proven.

### Multiplayer client/server

- a normal registered ECS component/input path should eventually require no custom
  replication driver or transport glue;
- application protocol and game semantics remain explicit;
- custom drivers remain available for specialized representations.

### Tool / editor

- UI/render/diagnostics-oriented composition does not inherit gameplay/networking
  defaults merely because both are engine capabilities;
- product-specific flows, workbench policy, native adapters, and commands remain
  explicit.

### Managed backend services

- remain conceptually separate from game-state replication;
- no generic provider abstraction is created without independent proof.

## Inspiration and provenance

The detailed current-main census and primary-source comparison are recorded in:

[`2026-08-12-application-composition-and-networking-ergonomics.md`](../../reports/investigations/2026-08-12-application-composition-and-networking-ergonomics.md)

The decision borrows selected architectural lessons rather than adopting external
systems wholesale.

### Bevy

Useful pressure:

```text
one App composition root
DefaultPlugins vs MinimalPlugins
modifiable PluginGroup/PluginGroupBuilder
explicit effective plugin composition
```

Runen does not adopt Bevy's ECS-centered ownership model or copy its default group
membership.

### Lightyear

Useful pressure:

```text
complex networking implementation
+ client/server plugin groups
+ application-owned protocol/registration
= simple normal multiplayer setup
```

Runen does not replace its networking model with Lightyear or adopt its Bevy-specific
runtime ontology.

### Supabase

Useful pressure:

```text
independently meaningful services
+ strong integration/product tooling
+ progressive disclosure to underlying systems
```

Supabase additionally demonstrates that hosted convenience includes operational
ownership, not merely a local API wrapper.

Runen does not equate hosted realtime messaging with authoritative simulation
replication and does not infer a generic managed-service abstraction from analogy.

## Rejected alternatives

### Explicit low-level composition only

Retain as expert control; reject as the sole product experience because routine apps
would repeat internal integration knowledge.

### Monolithic default-everything App

Reject. Servers, tools, headless workloads, local games, and multiplayer clients have
materially different capability sets. Making subtraction the exception would hide
behavior and increase accidental coupling.

### Persistent capability/preset configuration runtime

Reject without new evidence. It risks duplicating plugin/resource truth and creating a
second application ontology. Thin ephemeral construction helpers remain possible.

### Convention/autodiscovery as composition authority

Reject. Hidden discovery weakens reproducibility, ownership reasoning, and inspection.

### Generated/scaffolded setup as runtime architecture

Reject. Scaffolding may help project creation but generated boilerplate does not replace
a clear runtime composition model.

### Service locator / universal registry

Reject as the convenience architecture because it hides dependencies and weakens typed
ownership.

### Replace Runen networking with a third-party or hosted realtime layer

Reject. Third-party libraries/services may provide contained realizations or separate
backend services, but Runen's game-network semantic layer remains independently useful
and already owns substantial behavior.

## Consequences

- Runenwerk keeps its explicit authority architecture while gaining a durable product
  usability doctrine.
- `App` remains the one composition root.
- Future product groups can provide Bevy-like convenience without becoming authority.
- Product-specific behavior remains visible instead of disappearing into one giant
  default stack.
- Dedicated servers and tools can have intentionally different product compositions.
- Existing networking architecture remains custom Runen technology while the ordinary
  ECS authoring path is expected to become substantially less boilerplate.
- Managed backend integrations remain available as future product work without being
  conflated with game replication.
- App builtin-resource cleanup and compile-time modularity remain separate future work.
- #205 can later write a North Star that explains both the internal semantic
  architecture and the external batteries-included experience.

## Non-goals

This decision does not create or authorize:

```text
new App runtime
RunenAppConfig runtime authority
final product-group type/API names
final group memberships
Rust implementation
builtin-resource movement
dependency/Cargo feature redesign
networking Phase 4 implementation
new replication semantics
Runen networking replacement
Supabase/Lightyear/Bevy dependency
OnlineServices framework
service locator
autodiscovery runtime
project scaffolding system
broad #205 documentation rewrite
```

## Delivery and follow-up

Issue #247 owns this bounded architecture decision. Issue #246 owns the current-main
census, external comparison, alternatives review, and six user-path pressure tests.

After this ADR is accepted and accepted-main validation succeeds, #205 may use ADR 0017,
ADR 0018, and ADR 0019 together when rewriting the positive Runenwerk North Star.

Implementation remains separately gated. In particular:

- exact product-group API design requires current-source implementation proof;
- moving `App` builtin resources requires a separate ownership/cutover review;
- declarative networking implementation remains governed by the existing networking
  design and roadmap;
- compile-time feature/dependency topology requires separate evidence;
- managed backend integrations require separate product/provider designs.