# Testing and Validation

Tests belong with the owner of the invariant they prove. Prefer behavior tests, typed APIs, dependency checks, and conformance cases over source-string guards.

## Focused checks

Use the smallest checks that exercise the changed package while implementing, for example:

```text
cargo test -p <package>
cargo clippy -p <package> --all-targets --locked -- -D warnings
python tools/docs/validate_docs.py
```

Focused checks support iteration. They do not replace the merge baseline.

## Integration-test boundaries

An integration-test executable represents an actual isolation or external-boundary requirement, not an individual issue, milestone, phase, or small contract. Related contract tests should normally be modules inside a boundary-owned integration target.

Keep separate executables where platform behavior, process or environment isolation, compile-fail behavior, crash or panic isolation, native GPU conformance, or explicit backend/runtime evidence requires them. Target consolidation must not introduce shared mutable runtime state or weaken test isolation.

## Required baseline

Before merge:

```text
cargo validate
git diff --check
```

`cargo validate` is read-only and lockfile-safe. It validates the repository tooling, formats the workspace, runs locked workspace tests, runs strict Clippy, validates documentation, and checks durable repository invariants.

Rust CI resolves, explicitly checks out, and validates the reviewed feature head for pull requests; push and dispatch use `github.sha`. That exact-head result is the merge authority for the baseline.

## Documentation build

Pull requests and pushes that change `docs-site/**` or `.github/workflows/docs-validation.yml` also run the Astro/Starlight production build through the path-scoped documentation workflow. Root-only documentation changes outside that path scope do not automatically trigger Documentation Build. The workflow independently selects and proves the same event-derived repository revision; its workflow-definition ref may be a synthetic merge ref, distinct from the checked-out contents.

## Supplemental RunenRender GPU execution proof

Current Runenwerk CI also runs the `RunenRender R6 Vulkan execution proof`. It resolves and proves the selected repository revision, restricts Vulkan loading to the installed Mesa software Vulkan implementation, and runs the complete R6 public-RunenGPU execution proof module.

This is Runenwerk-owned consumer and render-integration evidence. It does not make Runenwerk the owner of RunenGPU implementation or standalone framework conformance. RunenGPU implementation and conformance are owned by the external `dornglut/runen-gpu` repository and its repository-owned validation/conformance workflows.

Runenwerk validates its exact-revision RunenGPU dependency, public-API consumption, and product/runtime integration through its own locked baseline and focused integration proofs.

## Evidence

Report focused checks, `cargo validate`, exact-head CI, and anything not run. Do not convert source inspection or user-reported output into a stronger validation claim.

The accepted base, reviewed feature head, synthetic merge result, squash commit, and accepted-main push result are separate evidence objects.

Long-form workflow authority lives in [`docs-site/src/content/docs/workspace/engineering-workflow.md`](docs-site/src/content/docs/workspace/engineering-workflow.md).
