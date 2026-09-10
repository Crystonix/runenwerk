---
title: Feature Map
description: Current capability map for the standalone RunenECS package.
status: active
owner: runen-ecs
layer: domain
canonical: true
last_reviewed: 2026-09-10
---

| Area | Capability | Status | Notes |
| --- | --- | --- | --- |
| Derives | `Component`, `Resource`, `Bundle` | ✅ | Supported for normal world lifecycle operations. |
| Derives | `Reflect` | ✅ | Standalone type metadata and reflected value access. |
| World | Entities and resources | ✅ | Opaque world-local entities and typed world resources. |
| World | `ChangeCursor` | ✅ | Lightweight ordered ECS observation position; absolute exhaustion fails before reuse. |
| Queries | `Query`, `QueryState`, tuples, optional forms | ✅ | Storage-independent typed component access. |
| Queries | `With`, `Without`, `Added`, `Changed` | ✅ | ECS-local filter semantics. |
| Queries | `RemovedQuery`, `RemovedState` | ✅ | Current removed-component observation window after deferred structural changes. |
| System params | `Res`, `ResMut`, `WorldMut`, `Commands` | ✅ | `WorldMut` is the built-in exclusive whole-world parameter. |
| Commands | `queue`, `spawn`, `despawn`, `insert`, `remove`, `batch`, `apply` | ✅ | Deferred structural mutations with deterministic serial application. |
| Runtime | Schedule labels and system sets | ✅ | Explicit `before` / `after` ordering with cycle validation. |
| Runtime | Deferred-apply boundaries | ✅ | ECS-neutral visibility points reported after successful flushes. |
| Runtime | Structured errors | ✅ | ECS-owned setup, schedule, parameter, command, system, boundary, and invariant categories. |
| Runtime | Deterministic serial execution | ✅ | Registration order is the tie-break for otherwise unordered systems. |
| Extension boundary | Manual `SystemParam` implementation | ⚠ | Low-level unsafe contract is doc-hidden and unsupported for downstream code; use derives or built-ins. |
| Events / telemetry / history | Generic channels, process-global telemetry, unbounded change journals | ❌ | Outside the retained RunenECS public contract. |

RunenECS has no application lifecycle, rendering, networking, replay, product,
or host-frame policy. Access compatibility is separate from semantic ordering;
it does not create ordering edges or visibility boundaries.

The public package identity is `runen-ecs` and the Rust crate identity is
`runen_ecs`. The companion derive package is `runen-ecs-macros`.
