---
title: Module Structure
description: Module Structure
status: active
owner: workspace
layer: workspace
canonical: true
last_reviewed: 2026-09-11
---

# Intra‑Crate Module Structure Guidelines

This document defines the **preferred module organization pattern inside
crates** for the `Runenwerk` workspace.

It complements:

-   `AGENTS.md`
-   `architecture.md`
-   [`../workspace/crate-inventory.md`](../workspace/crate-inventory.md)
-   [`dependency-rules.md`](dependency-rules.md)

These rules help both humans and AI coding agents consistently place
code and navigate subsystems.

------------------------------------------------------------------------

# Core Principle

Within a crate, organize code by **subdomain responsibility**, not by
technical layer or arbitrary utility buckets.

Good structure should answer:

-   *What concept owns this code?*
-   *Which subsystem is responsible for this behavior?*

------------------------------------------------------------------------

# Preferred Pattern

For medium or large subsystems, use **subdomain folders with explicit
`mod.rs` boundaries**.

Example:

``` text
render/
├── mod.rs
├── renderer/
│   ├── mod.rs
│   ├── config.rs
│   ├── runtime.rs
│   └── diagnostics.rs
├── frame_graph/
│   ├── mod.rs
│   ├── spec.rs
│   ├── builder.rs
│   └── registry.rs
└── shader_manager/
    ├── mod.rs
    ├── registry.rs
    ├── compiler.rs
    └── types.rs
```

### Why this works

-   Each folder represents a **real ownership boundary**
-   Navigation is predictable
-   Subsystems remain isolated
-   AI agents can determine the correct placement for new code

------------------------------------------------------------------------

# `mod.rs` Responsibilities

Every module root (`mod.rs`) should define the **public surface** of
that subsystem.

Example:

File: `engine/src/plugins/render/mod.rs`

``` rust
pub mod renderer;
pub mod frame_graph;
pub mod shader_manager;
```

Inside a subdomain:

File: `engine/src/plugins/render/renderer/mod.rs`

``` rust
pub mod config;
pub mod runtime;

mod diagnostics;
```

Rules:

-   Export only what should be externally visible.
-   Keep internal helpers private when possible.

------------------------------------------------------------------------

# When to Create a Subdomain Folder

Create a folder when:

-   The subsystem has **3 or more related files**
-   The files represent a **cohesive concept**
-   The subsystem is expected to grow

Example good subdomains:

-   `renderer`
-   `frame_graph`
-   `shader_manager`
-   `replication`
-   `session`
-   `prediction`

Avoid creating folders for a single trivial file.

------------------------------------------------------------------------

# Avoid These Patterns

The following patterns are **not allowed** in this repository.

## `include!` Module Composition

Do not use:

``` rust
include!("some_large_file.rs");
```

Reasons:

-   Breaks module boundaries
-   Makes navigation difficult
-   Confuses tooling and agents

Use normal Rust modules instead.

------------------------------------------------------------------------

## `_internal` Module Suffix

Do not create modules like:

``` text
renderer_internal
game_internal
handoff_internal
```

These are ambiguous and inconsistent.

Use proper module boundaries instead.

------------------------------------------------------------------------

## Catch‑All Utility Buckets

Avoid vague files like:

``` text
utils.rs
helpers.rs
misc.rs
common.rs
```

Instead use explicit names:

``` text
routing.rs
connection.rs
prediction.rs
metrics.rs
diagnostics.rs
```

Clear naming improves maintainability and code discovery.

------------------------------------------------------------------------

# Subdomain Coupling Rules

Subdomains may collaborate, but **ownership boundaries must remain
clear**.

Good:

-   `renderer` orchestrates rendering
-   `frame_graph` defines render graph structure
-   `shader_manager` manages shaders

Avoid:

-   deep cross‑imports of private internals
-   bypassing public APIs of sibling modules

Prefer exposing a small interface through the module root.

------------------------------------------------------------------------

# Small Subsystems

Not every module requires nested folders.

For small subsystems, a flat layout is fine:

``` text
scene/
├── mod.rs
├── manager.rs
├── lifecycle.rs
├── transitions.rs
└── tests.rs
```

Add subdomain folders **only when complexity justifies it**.

------------------------------------------------------------------------

# Agent Guidance

When adding new code, agents should:

1.  Identify the **owning domain**
2.  Locate the **owning subsystem folder**
3.  Follow the structure used by nearby modules
4.  Avoid creating new structural patterns unless necessary

Agents must prefer extending existing subsystems rather than creating
new ambiguous modules.

------------------------------------------------------------------------

# Summary

Preferred structure:

-   Domain‑oriented modules
-   Subdomain folders for larger systems
-   Explicit naming
-   Clear `mod.rs` boundaries

Forbidden patterns:

-   `include!`
-   `_internal` modules
-   catch‑all utility files

This approach keeps the repository predictable, navigable, and aligned
with the architecture defined in `architecture.md`.


---

# How to Decide Where New Code Belongs

When adding new code, decide placement in this order.

## 1. Choose the owning top-level domain first

Pick the owner before picking a file.

Use these rules:

- `domain/` for Runenwerk-owned engine-agnostic domain contracts and logic
- `engine/` for Runenwerk runtime composition, plugins, rendering integration, input, scene, UI, and time
- `net/` for the remaining Runenwerk simulation/history/network-authoring and migration surfaces; reusable realtime-networking semantics belong to standalone RunenNet
- `apps/` for process wiring, config loading, product policy, and external service integration
- `adapters/` for explicit external runtime/engine integration glue

If a reusable semantic already belongs to a peer framework, change that framework through its own repository rather than adding duplicate Runenwerk authority. If the logic is Runenwerk-specific runtime/product glue, place it in the appropriate local integration owner.

## 2. Choose the owning crate next

Examples:

- Reusable realtime-networking contract change  
  -> standalone `dornglut/runen-net`, not `net/engine_net`

- Runenwerk-specific retained replication migration behavior  
  -> `net/engine_net` only while that current RN8 migration surface exists

- QUIC transport realization  
  -> standalone `runen-net-quic`; Runenwerk keeps only maintained app/host integration

- ECS/plugin integration for networking  
  -> `engine/src/plugins/net`

- Gameplay replication/correction/smoothing policy  
  -> the owning gameplay domain module (or `apps/*` if app-local) plus `engine/src/plugins/net` for engine runtime wiring

- Gameplay rule or entity behavior  
  -> the owning module under `domain/`

- App bootstrap/config/runtime wiring  
  -> the relevant crate under `apps/`

## 3. Choose the owning subsystem inside the crate

Find the narrowest subsystem that already owns the concept.

Examples:

- Render graph registration  
  -> `engine/src/plugins/render/frame_graph/`

- Shader compilation  
  -> `engine/src/plugins/render/shader_manager/`

- Scene transition lifecycle  
  -> `engine/src/plugins/scene/lifecycle/`

- Client-side net prediction  
  -> the owning gameplay module (`domain/*` or `apps/*`) or `engine/src/plugins/net/` depending on whether it is game-specific or Runenwerk runtime integration; reusable prediction semantics remain RunenNet-owned

Prefer extending an existing subsystem over creating a new bucket.

## 4. Create a new subsystem only when ownership is real

Create a new folder/module only if all of the following are true:

- the concept is distinct
- it owns behavior, not just leftovers
- it is expected to grow
- existing nearby modules would become less clear if it were added there

Do not create new folders just to avoid a slightly larger file.

## 5. Use names that describe responsibility

Prefer:

```text
prediction.rs
routing.rs
connection.rs
diagnostics.rs
registry.rs
builder.rs
```

Avoid:

```text
utils.rs
helpers.rs
common.rs
misc.rs
temp.rs
```

## 6. Put code next to related code

If a helper is only used by one subsystem, keep it in that subsystem.

Do not promote local logic into a shared module too early.

Promote code upward only when it is truly reused across domains or subsystems.

## Decision examples

### Example A: change reusable snapshot/ack semantics
Owner:
- repository: standalone `dornglut/runen-net`
- package/subsystem: the owning RunenNet replication/delivery contract

If the change is only to Runenwerk's still-retained migration adapter/state, keep that narrower change in the current `net/engine_net` migration surface instead of redefining the reusable contract.

### Example B: change QUIC datagram MTU safety logic
Owner:
- repository: standalone `dornglut/runen-net`
- package: `runen-net-quic`
- Runenwerk changes only if its maintained consumer integration must adapt to the public contract

### Example C: add remote player smoothing for current gameplay module
Owner:
- top-level domain: `domain` or `apps` (depending on gameplay ownership)
- crate: the owning gameplay crate
- subsystem: the crate's net/gameplay integration module

### Example D: add generic engine-side network schedules
Owner:
- top-level domain: `engine`
- crate: `engine`
- subsystem: `src/plugins/net/`

## Placement rule summary

Choose placement in this order:

1. semantic owner/repository
2. top-level Runenwerk area when Runenwerk owns the change
3. crate
4. subsystem
5. file/module

Never choose the file first and invent ownership afterward.

---

# Module Size Limits and When to Split a Subsystem

This repo does not need rigid size limits, but it does need clear split triggers.

## Split a file when one or more of these is true

### 1. The file owns more than one responsibility

Bad:

- type definitions
- runtime orchestration
- diagnostics
- registry logic
- tests

all in one file

Better:

```text
renderer/
├── mod.rs
├── config.rs
├── runtime.rs
├── diagnostics.rs
└── tests.rs
```

### 2. The file becomes difficult to navigate

A file should still be easy to scan and reason about.

As a practical rule, strongly consider splitting when:

- it is consistently above roughly 300-500 lines, and
- the sections represent real concepts, not one continuous algorithm

This is not a hard rule. A cohesive 600-line file can be fine. A confused 180-line file can already need splitting.

### 3. Private sections start acting like submodules

If you find yourself writing comment blocks like:

- registry implementation
- runtime wiring
- diagnostics
- helper types

inside one file, that is usually a sign the file should become a folder or gain sibling modules.

### 4. Tests require heavy scrolling away from the code they validate

Prefer:

```text
frame_graph/
├── mod.rs
├── builder.rs
├── registry.rs
└── tests.rs
```

over burying large test sections deep in unrelated files.

### 5. A subsystem has multiple stable concepts

Split a subsystem into subdomain folders when it has distinct concepts with their own growth path.

Good split:

```text
render/
├── renderer/
├── frame_graph/
└── shader_manager/
```

Bad split:

```text
render/
├── part_a/
├── part_b/
└── other/
```

where the names do not represent real ownership.

## Do not split too early

Avoid creating folders when there is only one small file and no clear boundary yet.

Good:

```text
scene/
├── mod.rs
├── manager.rs
├── lifecycle.rs
└── transitions.rs
```

Only later, if one of those grows into a real subsystem, split it into a folder.

## Preferred split order

When a subsystem grows, split in this order:

### 1. Split by responsibility first
Example:

```text
runtime.rs
diagnostics.rs
config.rs
registry.rs
```

### 2. Split into a subdomain folder second
Only when a concept clearly owns several related files.

Example:

```text
shader_manager/
├── mod.rs
├── registry.rs
├── compiler.rs
└── types.rs
```

### 3. Extract shared code upward last
Only when multiple sibling subsystems truly reuse it.

Do not create shared abstractions prematurely.

## Warning signs of architecture drift

A subsystem likely needs restructuring if you see:

- growing `utils.rs`
- lots of `pub(crate)` escape hatches between unrelated modules
- comments separating several pseudo-modules inside one file
- new files named `misc`, `common`, or `helpers`
- repeated uncertainty about where code belongs

## Summary

Split when:
- responsibility is mixed
- navigation is poor
- stable concepts emerge

Do not split when:
- the boundary is artificial
- the folder would contain one tiny file
- the move is only aesthetic

The goal is not more folders. The goal is clearer ownership.
