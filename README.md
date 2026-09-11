# Runenwerk

Runenwerk is an experimental Rust platform for building world-centric applications, editors, simulations, and rendering systems.

It is the integration and product repository for the wider Runen framework family. Dornglut Engineering owns cross-repository family membership and repository roles; Runenwerk consumes accepted peer-framework public contracts while retaining application lifecycle, cross-domain composition, editor and runtime integration, adapters, and product-level policy.

> **Status:** Active development. Architecture and package boundaries are still evolving. Runenwerk does not yet provide a stable public API or production-readiness guarantee.

## Design direction

- explicit domain ownership and one-way dependencies;
- field-first spatial and rendering systems without mandatory mesh authoring;
- deterministic, inspectable contracts and structured diagnostics;
- direct authoring, procedural generation, simulation, and persistent state as composable sources;
- reusable frameworks separated from application and product integration;
- headless validation and focused conformance tests.

## Framework integration

Standalone peer repositories own their reusable semantics and public contracts; family membership alone does not imply Runenwerk adoption or semantic ownership. Runenwerk composes accepted public framework capabilities through explicit adapters and product integration.

RunenRender remains Runenwerk-owned rendering integration until a separately accepted external cutover and consumes RunenGPU only through its accepted public API. Current cross-repository family architecture is owned by [Dornglut Engineering](https://github.com/dornglut/engineering/blob/main/architecture/runen-family.md).

## Repository layout

```text
foundation/  low-level shared contracts
domain/      engine-agnostic domain crates
engine/      runtime and integration
apps/        executable products and examples
adapters/    external-host and cross-system adapters
docs-site/   canonical documentation
```

## Documentation

- [Architecture overview](ARCHITECTURE.md)
- [Testing and validation](TESTING.md)
- [Canonical documentation](docs-site/src/content/docs/index.mdx)
- [Framework integration architecture](docs-site/src/content/docs/architecture/repository-family-architecture.md)
- [Dependency rules](docs-site/src/content/docs/guidelines/dependency-rules.md)
- [Crate inventory](docs-site/src/content/docs/workspace/crate-inventory.md)

## Contributions

Runenwerk is currently a solo-maintained research and development project.

Issue reports, design discussion, reviews, reproducible cases supplied through discussion, and other participation that does not add third-party repository content are welcome.

Until reviewed inbound contribution terms exist that preserve the intended commercial relicensing path, external pull requests that add tracked repository content are not accepted, including code, documentation, tests, examples, build scripts, or assets.

## License

Runenwerk is available under the GNU General Public License v3.0 only (`GPL-3.0-only`). A separate commercial license may be available from copyright holder(s) authorized to grant it. See [LICENSE](LICENSE) and [LICENSING.md](LICENSING.md).
