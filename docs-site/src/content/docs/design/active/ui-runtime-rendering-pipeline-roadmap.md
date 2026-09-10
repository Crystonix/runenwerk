---
title: UI Runtime Rendering Pipeline Roadmap
description: Perfectionist roadmap for moving UiProgram artifacts from authored controls to evaluated runtime views, render primitives, visible gallery output, and production readiness without renderer-owned UI truth.
status: active
owner: ui
layer: domain
canonical: true
last_reviewed: 2026-09-10
related:
  - ./ui-program-architecture.md
  - ../deferred/game-runtime-ui-projection-and-hud-platform-design.md
  - ../../domain/ui/architecture.md
  - ../../domain/ui/roadmap.md
  - ../../workspace/production-track-planning-model.md
  - ../../workspace/planning-and-implementation-workflow.md
---

# UI Runtime Rendering Pipeline Roadmap

## Decision

The first visible button must be the end of a proven runtime pipeline, not a hand-written render shortcut.

The durable target is:

```text
Authored UI definition
  + explicit ControlPackageRegistrySnapshot
  + host data snapshot
  + theme/style tokens
  + viewport/layout constraints
  -> UiProgramFormationReport
  -> UiProgram
  -> UiCompilerReport
  -> UiRuntimeArtifact
  -> runtime view model
  -> layout/style/text/accessibility/interaction reports
  -> backend-neutral render primitives
  -> backend adapter
  -> visible gallery/editor/game UI
```

Rendering must consume runtime artifacts, runtime views, or render primitives derived from those artifacts. Rendering must not read authored `.ron` files directly, invent missing package truth, infer control semantics from strings, or bypass formation/compiler/evaluator diagnostics.

## Procedure Review

This roadmap follows the production planning rules:

- production tracks define strategic outcomes, while WR roadmap rows remain the dependency-checked execution graph;
- generated production docs are outputs and must not be edited directly;
- implementation must remain blocked behind accepted design gates, WR ownership, validation commands, evidence gates, and closeout evidence;
- design and roadmap documents may define the target and milestones, but they do not authorize code changes or crate creation by themselves.

This document is therefore an active design/roadmap source. It does not claim production-track completion, does not edit generated production indexes, and does not authorize renderer or app changes by itself.

## Story-First Supersession

`PT-UI-STORY-PLATFORM` is now the production growth path for UI. The former
standalone static gallery path has been re-run through `UiStoryManifest`,
`UiStoryRegistry`, `UiStoryRunner`, `UiStoryRunReport`, and
`UiStoryMountEligibility`.

Gallery preview, CLI inspection, and static mount proof may claim success only
from the story report, not from a hardcoded gallery path. Product host mounting
still requires its own future story-derived eligibility work.

## Current Code Truth

The pushed UI changes have moved the architecture in the right direction:

- `ui_program_lowering` exposes a registry-snapshot formation entrypoint: `form_ui_program_report_from_node_with_registry_snapshot(...)`.
- `UiProgramFormationReport` carries `program`, combined diagnostics, and a mandatory `catalog_report`.
- `UiProgramFormationCatalogReport` reports catalog diagnostics and skipped control kinds.
- `ui_compiler` integration can compile the `formation_report.program` into runtime artifacts.
- `UiRuntimeArtifactTables` already split controls, layout, style, state, interaction, binding snapshots, collection diffs, visual, text layout requests, accessibility, and inspection.
- `ui_controls` already models multiple control kinds through package modules, including button, label, inspector field, color picker, action prompt, list view, tree view, and table view.

Current runtime-rendering code truth:

- `ui_program_lowering::lower_control_nodes` now fails closed for unknown authored control kinds and emits no contract-derived graph rows.
- Catalog derivation checks schema/kernel package completeness and propagates diagnostics into `UiProgramFormationReport`.
- Authored button property validation and value carry-through are covered by checked-in `assets/ui_gallery/button/*.ron` fixtures.
- `ui_runtime_view::ButtonRuntimeViewReport` derives label, route, capability, selected/disabled state, accessibility label, style axes, source-map indexes, and diagnostics from runtime artifacts plus supplied host data.
- `ui_render_primitives` owns backend-neutral primitive generation, resolves `ThemeTokens`, shapes label text through `AtlasTextLayouter`, and emits `GlyphRunPrimitive` instead of placeholder text diagnostics.
- `runenwerk_editor --bin runenwerk_ui_gallery` hosts the static gallery path through the existing renderer UI composite pass only after checked-in stories report passing render primitive, render-data, static-mount, preview-frame, and mount-eligibility stages.

Remaining code truth:

- Interactivity, hover/pressed/focus loops, gallery inspection, additional control pipelines, and production-readiness evidence are still separate roadmap work.

## Governing Invariants

- Authored definitions are source intent, not runtime rendering truth.
- `UiProgram` is the typed semantic graph boundary.
- `UiRuntimeArtifact` is optimized runtime/evaluator/render data.
- Control package registries must be explicit inputs or explicit snapshot artifacts.
- Unknown control kinds fail formation.
- Broken package contracts fail catalog derivation.
- Invalid authored property values fail formation before runtime artifacts are treated as renderable.
- Runtime views are derived state, not new source truth.
- Render primitives are backend-neutral derived products.
- Renderer code owns rendering execution only. It must not own UI semantics, package semantics, host command truth, game truth, editor truth, ECS truth, or product truth.
- A visible button is not a success condition unless every upstream report in the chain passed.

## Roadmap

### M0 — Roadmap And Gate Doctrine

State: active.

Goal: make this document the design-level roadmap for artifact-backed rendering without claiming production execution authority.

Acceptance criteria:

- The roadmap names every required layer before visible rendering.
- The roadmap records stop conditions and forbidden shortcuts.
- The roadmap links to the UI domain roadmap and architecture docs.

### M1 — Unknown Control Kind Fail-Closed

State: implemented.

Goal: prevent unregistered authored controls from becoming plausible semantic program nodes.

Owning files:

```text
domain/ui/ui_program_lowering/src/lower.rs
domain/ui/ui_program_lowering/tests/ui_program_formation_report.rs
domain/ui/ui_program_lowering/tests/ui_program_button_formation.rs
```

Acceptance criteria:

- `catalog.control_kind(kind)` returning `None` emits `ui.program.control.unknown_kind`.
- The diagnostic message includes authored control id and control kind id.
- The diagnostic is source-mapped to the authored control path.
- Unknown controls do not produce package-contract-derived layout, style, state, interaction, binding, visual, accessibility, or inspection rows.
- A minimal source-mapped control placeholder is allowed only if the formation report fails.

### M2 — Catalog Schema And Kernel Completeness

Goal: make catalog derivation a complete package contract gate.

Owning file:

```text
domain/ui/ui_program_lowering/src/catalog.rs
```

Acceptance criteria:

- Missing property schema fails catalog derivation.
- Missing state schema fails catalog derivation.
- Missing event payload schema fails catalog derivation.
- Missing layout, interaction, visual, accessibility, or inspection kernel fails catalog derivation.
- Skipped control kinds are named and diagnostics are propagated into `UiProgramFormationReport`.

### M3 — Authored Property Validation And Value Carry-Through

Goal: prove authored control properties are valid and survive into semantic/runtime data.

Owning files:

```text
domain/ui/ui_program/src/*
domain/ui/ui_program_lowering/src/lower.rs
domain/ui/ui_artifacts/src/tables/control.rs
```

Acceptance criteria:

- Button `label`, `variant`, `tone`, `density`, `size`, icon, tooltip, and disabled fields validate against the package property schema.
- Unknown properties fail by default.
- Missing required fields fail.
- Valid property values are available from `UiProgram` and runtime artifact data without re-reading authored source.

### M4 — Source Map And Diagnostic Completeness

Goal: make every formation and runtime-derived row traceable.

Owning files:

```text
domain/ui/ui_program_lowering/src/source_map.rs
domain/ui/ui_artifacts/src/source_map.rs
```

Acceptance criteria:

- Every graph-family row has source-map attachment.
- Every formation diagnostic has a source-map attachment where source path exists.
- Runtime artifact source-map indexes resolve all artifact table rows that claim source identity.

### M5 — Runtime Artifact Snapshot Contract

Goal: make compiled runtime artifacts reproducible and reviewable.

Acceptance criteria:

- Button basic and selected fixtures produce stable artifact snapshots.
- Manifest rows include package ids, control kind ids, schema ids, route ids, kernel ids, capability ids, and source-map entries.
- Snapshot drift is explicit and reviewed.

### M6 — Artifact-Backed Runtime View Model

Goal: derive UI runtime views from artifacts and host data, not authored definitions.

Target owner:

```text
domain/ui/ui_runtime_view/
```

Crate creation requires separate WR/production authority. Until then, tests may prototype behind existing authorized modules only.

Acceptance criteria:

- `ButtonRuntimeView` is derived from `UiRuntimeArtifact` and evaluated host data.
- The view contains id, label, selected, disabled, route, capability, accessibility label, and style axes.
- Missing or stale host data reports diagnostics.

### M7 — Binding Evaluator Integration

Goal: make selected/disabled/host-fed control state runtime-real.

Owning areas:

```text
domain/ui/ui_binding
domain/ui/ui_runtime_view
```

Acceptance criteria:

- Host data can drive selected state deterministically.
- Authorization failures are diagnostic-bearing.
- Dirty propagation is deterministic and report-backed.

### M8 — Runtime Layout Resolution

Goal: derive stable layout boxes for runtime views.

Acceptance criteria:

- A button runtime view receives a stable `UiRect` from runtime layout data.
- Layout diagnostics fail missing constraints or unsupported layout kernels.
- Layout result source maps trace back to artifact rows.

### M9 — Style And Theme Token Resolution

Goal: resolve visual tokens without hardcoding renderer colors.

Acceptance criteria:

- Button variant, tone, density, size, disabled, hovered, focused, pressed, and selected states resolve through theme/style tokens.
- Editor and game target profiles can resolve differently without changing authored source.
- Renderer code does not hardcode product UI colors.

### M10 — Text Layout Request And Result Path

Goal: make labels measured through the text system.

Acceptance criteria:

- Button labels emit text layout requests.
- Text measurement affects button minimum size and render primitive placement.
- Text source is inspectable and source-mapped.

### M11 — Backend-Neutral Render Primitives

State: implemented for the static button gallery slice.

Goal: produce render primitives without a backend dependency.

Target owner:

```text
domain/ui/ui_render_primitives/
```

Standalone crate creation for the static button gallery slice is superseded by
WR-177 / PM-UI-STORY-004. Runtime rendering proof now lives inside
`UiStoryRunReport` stages after the story manifest, registry, runner, report,
and mount-eligibility contracts exist.

Acceptance criteria:

- Button primitive output includes background, border, text, icon, focus ring, clipping, and layer order where applicable.
- Render primitive snapshots are deterministic.
- Invalid runtime views produce diagnostics, not guessed primitives.

### M12 — Headless Render Frame Snapshots

Goal: prove render output before visible backend integration.

Acceptance criteria:

- Button basic, selected, disabled, focused, and invalid fixtures produce headless render-frame snapshots.
- Snapshot output records primitive ordering, bounds, token ids, text runs, and accessibility correlation.

### M13 — First Visible Gallery Button

State: implemented for checked-in basic and selected button fixtures.

Goal: render a visible button only after artifact-backed primitive proof.

Acceptance criteria:

- The visible button is generated from checked-in authored fixtures through the full pipeline.
- Formation report passes.
- Compiler report passes.
- Runtime view report passes.
- Layout/style/text/accessibility reports pass.
- Render primitive snapshot passes.
- Backend adapter consumes primitives only.

### M14 — Interaction Hit Testing And Route Proposal

Goal: make the visible button interactive without moving mutation authority into UI.

Acceptance criteria:

- Pointer click and keyboard activation resolve to the same semantic route proposal.
- Disabled controls do not emit activation.
- Route proposals carry payload schema and capability information.
- Host/app/domain owners apply or reject proposals.

### M15 — Button Visual State Loop

Goal: prove hover, pressed, focused, disabled, and selected visual states.

Acceptance criteria:

- Visual state changes are derived from runtime state and input evaluation.
- Pressed/hover/focus state does not mutate host truth directly.
- Selected state can be host-fed through binding evaluation.

### M16 — Accessibility Runtime Proof

Goal: make rendered/runtime button accessibility provable.

Acceptance criteria:

- Every interactive button has an accessibility node.
- Accessibility rows are source-mapped.
- Label, disabled, selected, pressed, and focusability state are inspectable.
- Invalid accessibility roles fail formation.

### M17 — Label Control Pipeline

Goal: prove the same pipeline for label controls.

Acceptance criteria:

- Label fixture passes formation, compilation, runtime view, text layout, primitive snapshots, and visible rendering.

### M18 — Inspector Field Pipeline

Goal: prove field editing without moving provider behavior into generic UI.

Acceptance criteria:

- Inspector field view derives from artifact + host data.
- Edit proposals are route/capability/payload checked.
- Provider/app owns mutation.

### M19 — Color Picker Pipeline

Goal: prove rich control package behavior without central enum expansion.

Acceptance criteria:

- Color picker runtime view carries committed and preview color state.
- Preview and commit emit distinct schema-valid route proposals.
- Rendering uses primitives and tokens, not renderer-owned semantics.

### M20 — Collection Controls Pipeline

Goal: extend the same proof chain to action prompt, list view, tree view, and table view.

Acceptance criteria:

- Each control has fixtures, formation, compilation, runtime view, layout, accessibility, interaction, primitive snapshots, and visible gallery coverage.

### M21 — UI Gallery Inspector

Goal: make every layer inspectable.

Acceptance criteria:

- Gallery shows authored source, formation report, UiProgram graph, runtime artifact tables, runtime view, layout boxes, style tokens, accessibility tree, render primitives, rendered preview, and interaction trace.
- When a visible result is wrong, the failing layer is observable.

### M22 — Production Readiness Evidence Integration

Goal: connect the pipeline to production readiness rather than isolated tests.

Acceptance criteria:

- Evidence includes formation, compiler, runtime view, binding, layout, style, text, accessibility, interaction, render primitive, backend render, performance, localization, and target-profile compatibility proof.
- No milestone can claim production readiness with stale, preview-only, unsupported, missing, or wrong-owner evidence.

## Render Permission Gates

Use these gates exactly:

```text
After M12: headless render output may be trusted.
After M13: visible static button may be shown.
After M15: interactive button may be claimed.
After M16: accessible button may be claimed.
After M22: production-ready UI rendering may be claimed.
```

## Stop Conditions

Stop and redesign if any slice:

- renders from authored `.ron` directly;
- renders from `ControlPackageDescriptor` alone;
- infers package truth from a control-kind string;
- lets unknown control kinds pass formation;
- lets invalid package schema/kernel references create usable contracts;
- hardcodes button colors or visual policy in the backend renderer;
- uses debug overlay or editor shell behavior as generic UI runtime proof;
- mutates editor/game/domain/app state directly from generic UI code;
- creates new crates without WR/production authority;
- updates generated production docs instead of their source files.

## Validation Expectations

Roadmap/design validation:

```text
task docs:validate
task production:validate
task production:check
task planning:validate
```

Implementation milestones must add focused crate tests before broad local CI. The expected implementation validation envelope is:

```text
cargo fmt --all
cargo test -p ui_program_lowering
cargo test -p ui_compiler
cargo test -p ui_artifacts
cargo test -p ui_binding
cargo test -p ui_accessibility
cargo test -p ui_definition
```

Later rendering milestones must add crate-specific validation for runtime view, layout/style/text resolution, render primitives, backend adapter, and gallery evidence when those owners exist.

<!-- BEGIN RUNENWERK:UI_COMPONENT_PLATFORM:render-surface-output -->
## Component Platform render/surface output requirements

Reusable controls and surfaces must lower to renderer-neutral output with provenance. Component output may include rect, border, clip, image, glyph run, stroke, graph/surface primitive batches, product surface primitives, viewport surface embeds, layer order, sort key, draw key, primitive order, surface order, expected primitive counts, and render diagnostics. Renderer code must not own authored UI semantics.
<!-- END RUNENWERK:UI_COMPONENT_PLATFORM:render-surface-output -->
