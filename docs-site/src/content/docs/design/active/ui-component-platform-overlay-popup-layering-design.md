---
title: UI Component Platform Overlay / Popup / Layering Design
description: Completed owner-first overlay, popup, dropdown, tooltip, focus-containing, placement, package, catalog, inspection, runtime proof, static proof, and no-bypass design reference.
status: active
owner: ui
layer: domain
canonical: true
last_reviewed: 2026-09-12
related_docs:
  - ../../workspace/planning/roadmap.md
  - ../../workspace/planning/decision-register.md
  - ./ui-component-platform-generic-interaction-design.md
  - ./ui-component-platform-executable-interaction-story-design.md
  - ../../reports/closeouts/pt-ui-component-platform-012a-executable-interaction-story-implementation-scope-closeout.md
  - ./ui-component-platform-text-editing-design.md
---

# UI Component Platform Overlay / Popup / Layering Design

Lifecycle state: `completed`.

Planning ID: `PT-UI-COMPONENT-PLATFORM-013`.

Phase 13 completed through merged PR #44 on 2026-07-02 at merge commit `6f2d3827f315191d7aeaf68a64f523627197cad8`. Completion evidence covers package-backed overlay declarations, base-control overlay lowering, main-path package validation, catalog projection, inspection projection, normalized input fact consumption, `ui_runtime::overlay` replay/report/stack/placement/focus/dismissal/suppression proof, proof-frame projection, static mount proof, no-bypass evidence, and the full local validation gate reported green on 2026-07-02.

## Completed proof chain

```text
ui_controls overlay vocabulary
  -> base-control overlay lowering
  -> ControlPackageDescriptor.overlay_descriptors
  -> package validation
  -> catalog projection
  -> inspection projection
  -> ui_input normalized facts
  -> ui_runtime::overlay replay/report/stack/placement/focus/dismissal/suppression evidence
  -> OverlayLayeringVisualProof
  -> OverlayLayeringProofRenderFrame / UiFrame
  -> ui_static_mount validation
```

## Owner boundaries

`ui_controls` owns reusable overlay declarations, ergonomic builders, base-control lowering, package descriptors, package validation, catalog projection, and inspection projection. It must not own runtime stack state, raw input, product commands, product/editor/game mutation, app-specific modal state, authored editing, renderer backend behavior, or text editing.

`ui_input` owns normalized input facts only. It must not interpret overlay kind, placement, focus containment, dismissal, product behavior, story execution, or layer rendering.

`ui_runtime` owns renderer-neutral overlay proof under `domain/ui/ui_runtime/src/overlay/`. It consumes package-backed declarations and normalized facts to form open intents, stack entries, placement, layer, focus, dismissal, suppression, keyboard, pointer-capture, viewport, proof-frame, and no-bypass evidence. It must not execute product commands, mutate product/editor/game state, own authored editing, own app modal lifecycle, own text editing, own renderer backend behavior, or extract a plugin framework.

`ui_static_mount` validates renderer-neutral proof frames only.

Product/editor/game layers own command execution, state mutation, route authorization, persistence, authored editing, app-specific modal lifecycle, and product policy.

## Delivered scope

Completed implementation includes:

- `ControlOverlayDescriptor`, requirements, kinds, triggers, placement, layer, dismissal, focus, and support summaries;
- base-control lowering for popup, menu/submenu, dropdown, tooltip hover/focus, picker popup, and focus-containing overlay support;
- package-level overlay descriptors for all base controls;
- package validation for duplicate, unresolved, and invalid overlay descriptors;
- catalog projection of overlay kinds, triggers, layers, dismissal policies, focus policies, and no-command/no-mutation flags;
- inspection projection of overlay support facts;
- runtime package-backed fixture construction;
- runtime open intent, stack, placement, layer, focus, dismissal, suppression, keyboard, pointer-capture, viewport, and no-bypass evidence;
- deterministic replay evidence;
- runtime report to visual proof frame to static mount validation;
- tests for package, catalog, inspection, runtime package-backed consumption, runtime behavior, static mount, and no-bypass boundaries.

## Completed implementation files

```text
domain/ui/ui_controls/src/overlay.rs
domain/ui/ui_controls/src/package/descriptor.rs
domain/ui/ui_controls/src/package/validation.rs
domain/ui/ui_controls/src/package/overlay_validation.rs
domain/ui/ui_controls/src/base_control/lowering/layering_support.rs
domain/ui/ui_controls/src/base_control/lowering/mod.rs
domain/ui/ui_controls/src/base_control/compiler.rs
domain/ui/ui_controls/src/base_control/lowering/inspection.rs
domain/ui/ui_controls/src/catalog/entry.rs
domain/ui/ui_controls/src/catalog/inspection.rs
domain/ui/ui_controls/tests/overlay.rs
domain/ui/ui_controls/tests/overlay/package.rs
domain/ui/ui_controls/tests/overlay/catalog.rs
domain/ui/ui_controls/tests/overlay/inspection.rs

domain/ui/ui_input/tests/overlay_normalized_facts.rs

domain/ui/ui_runtime/src/overlay/mod.rs
domain/ui/ui_runtime/src/overlay/report.rs
domain/ui/ui_runtime/src/overlay/fixture.rs
domain/ui/ui_runtime/src/overlay/placement.rs
domain/ui/ui_runtime/src/overlay/stack.rs
domain/ui/ui_runtime/src/overlay/layering.rs
domain/ui/ui_runtime/src/overlay/proof_frame.rs
domain/ui/ui_runtime/tests/overlay.rs
domain/ui/ui_runtime/tests/overlay/layering_report.rs
domain/ui/ui_runtime/tests/overlay/executable_story.rs
domain/ui/ui_runtime/tests/overlay/package_backed.rs

domain/ui/ui_static_mount/tests/base_controls_overlay_layering_static_mount.rs
```

Overlay runtime semantics belong under `domain/ui/ui_runtime/src/overlay/`, not under `domain/ui/ui_runtime/src/input/overlay_*`.

## Completed proof scenarios

The merged implementation proves package-backed declarations, catalog projection, inspection projection, runtime package-backed consumption, Button popup, ActionPrompt menu/submenu, Dropdown, tooltip hover/focus, picker popup, focus-containing overlay, topmost Escape dismissal, outside pointer dismissal, inside-active-overlay no dismissal, pointer capture, keyboard navigation without product command execution, scroll and viewport placement recomputation, anchor invalidation, runtime report to static proof frame, and deterministic replay.

## No-bypass counters

`OverlayBoundaryAssertions` proves zero host command execution, zero product mutation, zero text-edit transaction, zero app-specific modal operation, zero authored UI edit, and zero plugin-framework operation.

## Validated gate

The current boundary-owned focused targets are:

```text
cargo test -p ui_controls --test overlay
cargo test -p ui_runtime --test overlay
```

The commands below are retained as the contemporaneous Phase 13 completion evidence from before the later test-topology cutover.

The full Phase 13 gate passed locally on 2026-07-02 before merge:

```text
cargo fmt --all --check
cargo check -p ui_controls
cargo check -p ui_input
cargo check -p ui_runtime
cargo check -p ui_static_mount
cargo check -p ui_story
cargo check -p runenwerk_editor
cargo test -p ui_controls overlay
cargo test -p ui_controls --test base_controls_overlay_package
cargo test -p ui_controls --test base_controls_overlay_catalog
cargo test -p ui_controls --test base_controls_overlay_inspection
cargo test -p ui_input input
cargo test -p ui_runtime overlay_layering
cargo test -p ui_runtime --test overlay_layering_report
cargo test -p ui_runtime --test executable_overlay_layering_story
cargo test -p ui_runtime --test overlay_package_backed
cargo test -p ui_static_mount base_controls_overlay_layering
python tools/docs/validate_docs.py
git diff --check
```

## Historical stop conditions

The completed scope stayed inside the accepted boundaries. Any follow-up must stop and redesign if it requires overlay runtime under `ui_runtime::input`, command execution in generic UI, product/editor/game mutation in generic UI, app-specific modal lifecycle in generic UI, runtime behavior in `ui_controls`, input semantics in `ui_input` beyond facts, story registry ownership in `ui_runtime`, editor shell registration, Workbench provider redesign, UI Gallery, UI Designer, full text editing, dynamic plugin framework, `foundation/meta`, shared plugin primitives, phase-shaped public API names, or compatibility-only aliases/shims.
