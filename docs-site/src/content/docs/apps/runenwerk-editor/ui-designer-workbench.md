---
title: UI Designer Workbench
description: Practical usage guide for the standalone and embedded Runenwerk UI Designer Workbench.
status: active
owner: editor
layer: app
canonical: true
last_reviewed: 2026-09-10
related_designs:
  - ../../design/accepted/ui-designer-workbench-product-design.md
  - ../../design/deferred/game-runtime-ui-projection-and-hud-platform-design.md
related_reports:
  - ../../reports/closeouts/pm-ui-designer-wb-008-runtime-proven-closeout-and-handoff/closeout.md
  - ../../reports/closeouts/pm-ui-designer-wb-v1-closure-006-runtime-proven-product-closeout-and-handoff/closeout.md
  - ../../reports/closeouts/pm-ui-designer-evidence-correction-001-runtime-evidence-source-revision-and-catalog-correction/closeout.md
---

# UI Designer Workbench

## Launch Paths

The standalone UI Designer is the focused authoring host:

```text
cargo run -p runenwerk_editor --bin runenwerk_ui_designer
```

The embedded path is the `Editor Design` workspace in the full editor:

```text
cargo run -p runenwerk_editor
```

Both paths consume the same UI Designer package, document, session, provider,
and evidence contracts. The standalone app is the clearest product proof; the
embedded workspace proves that the same workbench contracts fit the full
editor host.

## Normal Workflow

1. Open the standalone UI Designer or the embedded `Editor Design` workspace.
2. Select a UI document from the package-backed Designer session.
3. Inspect the catalog for compatible recipes, disabled reasons, slot
   contracts, target profiles, token references, binding references, and
   accessibility requirements.
4. Use the hierarchy, canvas, inspector, properties, token/recipe preview,
   binding preview, diagnostics, scenario matrix, readiness, and native
   evidence panes to review the selected document.
5. Route edits through typed visual-layout operations. Insert, move, reorder,
   layout, token-reference, binding-reference, and accessibility changes must
   produce a typed operation or a typed rejection diagnostic.
6. Review the deterministic operation diff before accepting it.
7. Apply, reject, reload last applied, or roll back explicitly. Reload and
   rollback are snapshot-backed and do not turn session state into source
   truth.
8. Capture scenario evidence explicitly when product readiness needs evidence.
   `editor.workbench` runtime readiness requires fresh product-path evidence
   tied to the selected source revision. `game.runtime` is recorded only as
   descriptor compatibility until the game-runtime UI track provides a real
   runtime product path. Capture is not automatic every frame.

## Evidence Example

The runtime evidence path is intentionally explicit:

```text
cargo test -p runenwerk_editor ui_designer
cargo test -p runenwerk_editor scenario_evidence
```

These tests prove the app/headless UI Designer provider route, workbench panes,
operation/apply/rollback projection, source-revisioned runtime evidence
packets, diagnostics snapshots, retained product artifacts with digests,
freshness invalidation, and product-path performance baseline families.

The game-runtime compatibility seam is validated separately:

```text
cargo test -p ui_definition game -- --nocapture
cargo test -p editor_shell game -- --nocapture
```

Those checks, together with the scenario evidence tests, prove `game.runtime`
descriptor, fixture, binding, validated intent, and evidence compatibility
only. They do not implement game HUD runtime behavior and cannot satisfy
UI Designer Workbench runtime readiness.

## Ownership Boundaries

- `domain/ui` owns generic UI definition truth: Canonical UI IR,
  target-profile descriptors, preview fixtures, production readiness,
  view-model binding contracts, recipes, persistence activation, and visual
  layout validation.
- `domain/editor` owns editor/workbench contracts: app-neutral UI Designer
  view models, UX Lab scenario/evidence adapters, and editor vocabulary.
- `apps/runenwerk_editor` owns concrete host composition, standalone launch,
  embedded workspace projection, session state, provider routing, evidence
  capture, retained artifacts, and typed unsupported-platform reasons.
- `docs-site/src/content/docs/workspace` owns roadmap and production state.

Session state, provider projection, evidence packets, and generated reports are
derived output. They must not replace authored UI package or document truth.

## Downstream Handoff

The UI Designer Workbench runtime evidence standard was corrected on
2026-05-29. Earlier completed closeouts remain historical, but current
readiness depends on the corrected source-revision, runtime-product evidence,
catalog routing, and descriptor/runtime split recorded by
`PM-UI-DESIGNER-EVIDENCE-CORRECTION-001`.

The UI Designer Workbench is complete at `runtime_proven` quality for V1
authoring and editor-workbench runtime evidence. These remain out of scope for
this product track:

- game HUD runtime behavior;
- SDF HUD rendering or in-frame runtime UI projection;
- native runtime-window screenshot evidence;
- packaged release readiness;
- perfectionist no-gap certification.

`PT-GAME-RUNTIME-UI` owns concrete game-runtime UI projection and HUD behavior.
A separate no-gap audit or certification track must own any future
`perfectionist_verified` claim.
