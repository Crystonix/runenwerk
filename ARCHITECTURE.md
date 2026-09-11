# Runenwerk Architecture

Runenwerk is the integration and product repository for a family of focused Rust frameworks. Canonical long-form Runenwerk architecture lives under `docs-site/src/content/docs`; Dornglut Engineering owns cross-repository family architecture.

## Framework integration

Current family membership, repository roles, and cross-repository dependency direction are owned by [Dornglut Engineering's Runen family architecture](https://github.com/dornglut/engineering/blob/main/architecture/runen-family.md). Standalone framework repositories own their reusable semantics, public contracts, conformance, validation, and release policy.

Runenwerk consumes accepted public framework contracts through explicit adapters and product integration:

```text
standalone framework public contracts
              |
              v
      Runenwerk adapters/integration
              |
              v
       applications and tools
```

Runenwerk retains application lifecycle, windows/event-loop policy, cross-framework adapters, editor/runtime integration, product policy, diagnostics presentation, and applications. Cross-framework meaning is translated explicitly rather than by reaching into framework internals.

The render plugin remains Runenwerk-owned integration until the separately bounded RunenRender extraction. It consumes RunenGPU only through the external public API; the accepted direct dependency direction is `RunenRender -> RunenGPU`.

## Current source layout

The standalone RunenECS and RunenGPU successors and their Runenwerk consumer cutovers are accepted. The workspace consumes `runen-ecs` and `runen-gpu` through exact accepted Git revisions; their implementations and framework conformance belong to `dornglut/runen-ecs` and `dornglut/runen-gpu` rather than to Runenwerk.

```text
foundation -> domain -> engine/runtime -> apps/adapters/tools
                 |             |
                 |             +--> exact-SHA runen-ecs consumer
                 +----------------> exact-SHA runen-gpu consumer
```

Current workspace membership is implementation evidence, not cross-repository semantic ownership.

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

## Invariants

- Framework repositories do not depend on Runenwerk.
- Runenwerk integrates frameworks through accepted public contracts and explicit adapters.
- RunenRender depends on RunenGPU and does not own WGPU directly.
- RunenRender semantic meaning remains distinct from source-domain state, request-scoped foreign semantic inputs, current availability/residency, and physical GPU realization.
- After an accepted framework consumer cutover, Runenwerk does not retain a writable predecessor implementation, forwarding namespace, source mirror, submodule, or moving branch dependency as parallel authority.

## Authority

- [Dornglut Runen family architecture](https://github.com/dornglut/engineering/blob/main/architecture/runen-family.md) — cross-repository family membership and repository relationships.
- [Framework integration architecture](docs-site/src/content/docs/architecture/repository-family-architecture.md) — Runenwerk-local adapters, compatibility, recovery, and product integration.
- [Repository extraction ADR](docs-site/src/content/docs/adr/accepted/0014-repository-family-extraction-boundaries.md)
- [GPU/render ownership ADR](docs-site/src/content/docs/adr/accepted/0015-separate-gpu-execution-from-rendering.md)
- [Cross-authority consistency and graph-semantics ADR](docs-site/src/content/docs/adr/accepted/0017-cross-authority-consistency-and-graph-semantics.md)
- [Semantic federation and physical realization ADR](docs-site/src/content/docs/adr/accepted/0018-semantic-federation-and-physical-realization.md)
- [RunenRender semantic-rendering ADR](docs-site/src/content/docs/adr/accepted/0021-ratify-runenrender-semantic-rendering-architecture.md)
- [RunenRender canonical design](docs-site/src/content/docs/design/accepted/runenrender-decomposition-design.md)
- [Dependency rules](docs-site/src/content/docs/guidelines/dependency-rules.md)
