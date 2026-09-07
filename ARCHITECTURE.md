# Runenwerk Architecture

Runenwerk is the integration and product repository for a family of focused Rust frameworks. Canonical long-form architecture lives under `docs-site/src/content/docs`.

## Repository family

```text
RunenSDF -----+
RunenSpatial -+
RunenECS -----+--> Runenwerk adapters/integration --> applications
RunenUI ------+
                   |
                   +--> RunenRender --> RunenGPU
                   +--> non-render RunenGPU workloads
```

Framework repositories must not depend on Runenwerk. The accepted direct framework dependency is:

```text
RunenRender -> RunenGPU
```

- **RunenGPU** owns general GPU execution, WGPU realization, resources, workloads, submissions, uploads/readback, surfaces, and device outcomes.
- **RunenRender** owns image formation, prepared render scenes, views, materials/media, visibility, transport, reconstruction, overlays, presentation intent, and lowering into RunenGPU workloads.
- **RunenSDF** owns reusable signed-distance and implicit-field mathematics and reference queries.
- **RunenSpatial** owns host-neutral spatial identity, addressing and coordinate mechanics, bounded spatial-demand planning, and content-agnostic availability lifecycle mechanics.
- **RunenECS** owns reusable entity/component/resource, query, and scheduling semantics.
- **RunenUI** owns renderer-neutral semantic UI, state/actions, layout, text, focus, accessibility, and paint output.
- **Runenwerk** retains application lifecycle, windows/event-loop policy, cross-framework adapters, editor/runtime integration, product policy, diagnostics presentation, and applications.

## Current source layout

After the accepted RunenGPU cutover:

```text
foundation -> domain -> engine/runtime -> apps/adapters/tools
                               |
                               +--> exact-SHA runen-gpu consumer
```

The render plugin remains Runenwerk-owned integration until the separately bounded
RunenRender extraction. It must consume RunenGPU only through the external public API.

## Foundation crates

Foundation crates provide reusable vocabulary and low-level contracts with no domain, engine/runtime, application, or adapter dependencies.

```text
foundation/id             typed identity primitives and allocators
foundation/id_macros      attribute macro support for typed ID wrappers
foundation/diagnostics    structured diagnostic reporting vocabulary
foundation/ratification   shared ratification report vocabulary
foundation/schema         portable schema identity, value, shape, constraint, and descriptor vocabulary
foundation/commands       portable command contract vocabulary
foundation/resource_ref   portable external resource references
```

The canonical workspace membership and purpose descriptions live in the [crate inventory](docs-site/src/content/docs/workspace/crate-inventory.md). Keep this root summary aligned with that inventory and the workspace manifest. Foundation must not own domain-specific invariants, application or editor policy, runtime orchestration, backend integration, or cross-framework composition.

The GPU/render extraction sequence is:

```text
current-source inventory
-> internal RunenGPU proof
-> external RunenGPU clean cutover (complete for the accepted revision)
-> internal RunenRender proof on RunenGPU
-> external RunenRender clean cutover
```

## Invariants

- Framework repositories do not depend on Runenwerk.
- RunenGPU contains no renderer, ECS, UI, SDF, editor, or product meaning.
- RunenRender depends on RunenGPU and does not own WGPU directly.
- Cross-framework meaning is translated by explicit Runenwerk adapters.
- A completed extraction leaves one source authority, exact dependency pinning, all consumers migrated, and no forwarding namespace, source mirror, submodule, branch dependency, or duplicate implementation.

## Canonical authority

- [Repository-family architecture](docs-site/src/content/docs/architecture/repository-family-architecture.md)
- [Repository extraction ADR](docs-site/src/content/docs/adr/accepted/0014-repository-family-extraction-boundaries.md)
- [GPU/render ownership ADR](docs-site/src/content/docs/adr/accepted/0015-separate-gpu-execution-from-rendering.md)
- [Cross-authority consistency and graph-semantics ADR](docs-site/src/content/docs/adr/accepted/0017-cross-authority-consistency-and-graph-semantics.md)
- [Dependency rules](docs-site/src/content/docs/guidelines/dependency-rules.md)
