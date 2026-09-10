---
title: Game Runtime, Editor, ECS, Scripting, and Hot Reload Design
description: Current-vs-target architecture map for runtime/editor/ECS boundaries, linked to a deferred preserved target draft.
status: active
owner: engine
layer: engine-runtime
canonical: true
last_reviewed: 2026-05-09
related_designs:
  - ./editor-ui-workspace-tool-surface-architecture.md
  - ../accepted/sdf-first-field-world-platform-design.md
  - ../deferred/engine-game-runtime-editor-ecs-scripting-hot-reload-preserved-target-draft.md
related_adrs:
  - ../../adr/accepted/0007-external-runtime-preview-process.md
related_roadmaps:
  - ../../apps/runenwerk-editor/roadmap.md
related_reports:
  - ../../reports/closeouts/m5-runtime-preview/closeout.md
---

# Game Runtime, Editor, ECS, Scripting, and Hot Reload Design

## Read This First

This page is the current-state grounded architecture/gap document.

The prior long-form target draft is preserved in a separate deferred document so detail is not lost while this page remains implementation-grounded.

## Purpose and Scope

This design defines boundaries between code, editor-authored content, runtime execution, ECS world state, scripting, instantiation, and reload workflows in Runenwerk.

This document is architecture-focused. It does **not** commit a final implementation sequence; sequencing belongs in roadmap docs.

M5 adopts an external runtime preview process for preview, simulate, and play execution. The accepted decision is recorded in [`../../adr/accepted/0007-external-runtime-preview-process.md`](../../adr/accepted/0007-external-runtime-preview-process.md). This design remains active as the current boundary reference for later runtime-preview and formed-product expansion.

## Current Repository Anchors (Implemented Today)

- Workspace membership is defined by current members in [`Cargo.toml`](../../../../../../Cargo.toml); current local package locations live in the [`workspace crate inventory`](../../workspace/crate-inventory.md), and Runenwerk placement/ownership guidance lives in [`guidelines/architecture.md`](../../guidelines/architecture.md).
- Runtime app composition currently flows through `engine::App` (`add_plugin`, `add_systems`, `add_scene`, `add_scene_template`, `run`) in [`engine/src/app/domain/app.rs`](../../../../../../engine/src/app/domain/app.rs).
- Runnable editor composition is in [`apps/runenwerk_editor/src/runtime/app.rs`](../../../../../../apps/runenwerk_editor/src/runtime/app.rs).
- M5 external runtime preview protocol contracts live in [`domain/editor/editor_preview/src/lib.rs`](../../../../../../domain/editor/editor_preview/src/lib.rs).
- The runtime preview child app lives in [`apps/runenwerk_runtime_preview/src/lib.rs`](../../../../../../apps/runenwerk_runtime_preview/src/lib.rs) and has a headless command/event loop plus child-process heartbeat/shutdown coverage.
- Editor-side preview process management lives in [`apps/runenwerk_editor/src/runtime/preview_process/mod.rs`](../../../../../../apps/runenwerk_editor/src/runtime/preview_process/mod.rs), including bounded command queueing, event ingestion, heartbeat state, bootstrap timeout, and graceful shutdown fallback.
- Project-owned reload classification and status projection live in [`apps/runenwerk_editor/src/asset_pipeline/catalog_runtime.rs`](../../../../../../apps/runenwerk_editor/src/asset_pipeline/catalog_runtime.rs).
- Ratified `world_sdf` runtime payload intake flows through [`engine/src/plugins/world/build/integration.rs::enqueue_ratified_world_sdf_payload_package`](../../../../../../engine/src/plugins/world/build/integration.rs).
- Editor runtime already exposes explicit authored/instantiated/simulated/session reality views in [`apps/runenwerk_editor/src/editor_runtime/runtime.rs`](../../../../../../apps/runenwerk_editor/src/editor_runtime/runtime.rs).
- Scene persistence format today is `SceneFileV2` RON records in [`domain/editor/editor_persistence/src/scene_file.rs`](../../../../../../domain/editor/editor_persistence/src/scene_file.rs), not `.rwscene/.rwprefab/.rwinput` assets.
- Scene manifest discovery currently reads `.ron` files from `assets/scenes` and `game/assets/scenes` in [`engine/src/plugins/scene/manifest/catalog.rs`](../../../../../../engine/src/plugins/scene/manifest/catalog.rs).
- Current app-level editor MVP scope intentionally excludes prefab/material/scripting tooling for the first milestone, per [`docs-site/src/content/docs/apps/runenwerk-editor/mvp/first-3d-editor-mvp.md`](../../apps/runenwerk-editor/mvp/first-3d-editor-mvp.md).

## Current-to-Target Gap Matrix

| Capability Area | Current State | Target Direction | Owning Areas |
|---|---|---|---|
| Runtime/editor boundary | M5 external loopback runtime preview process, protocol DTOs, and editor process manager exist; editor authoring composition remains single-process. | Later phases may deepen preview/play product execution after new formed-product domains exist. | `domain/editor/editor_preview`, `net/engine_net`, `net/engine_net_quic`, `engine`, `apps/runenwerk_editor`, `apps/runenwerk_runtime_preview` |
| Scene authoring/persistence | Scene document + migration/normalization/formation exists. | Broader authored asset families (prefab/input/ability/etc.) with explicit contracts. | `domain/editor/editor_persistence`, future domain crates |
| ECS state ownership | Live runtime world + reflected component registration are active. | Keep ECS as live state only; expand contract catalog without ownership leakage. | `domain/ecs`, `apps/runenwerk_editor` |
| Plugin/type registration | Concrete registration points exist in runtime/editor app code. | Stronger registry-driven discoverability across more gameplay domains. | `engine`, `apps/runenwerk_editor` |
| Hot reload model | M5 reload decisions/statuses exist for current scene/asset/field-product/`world_sdf`/shader/UI-definition contracts; future domains fail closed as unsupported or restart-required. | Later domains add their own live-product reload semantics after formed-product contracts exist. | `domain/asset`, `engine`, `apps/runenwerk_editor` |
| Gameplay scripting boundary | No script runtime crate is implemented in workspace yet. | Language-neutral script contract boundary + adapter implementation later; Rhai is the first concrete adapter candidate. | future domain/runtime/adapter crates |
| Runtime UI attachment binding | Overlay UI is currently scene-template driven (`overlay_ui` + `ui_template`), not entity-attachment driven. | World-space/screen-projected UI attachment binding is deferred post-MVP and should be added only through explicit authored binding contracts and runtime formation seams. | `engine` scene runtime + future domain/runtime contracts |
| Gameplay content breadth | Current editor MVP is 3D graybox scene authoring focused. | Post-MVP expansion for richer authored domains when contracts exist. | `apps/runenwerk_editor`, future domain crates |
| Architectural reality model | Doctrine exists in guideline docs and editor runtime reality views. | Deeper consistency across authored/normalized/formed/instantiated/simulated pipelines. | guidelines + editor/runtime domains |

## Canonical Boundary Rules for This Topic

- ECS stores **live simulated state**, not arbitrary authored source documents.
- Authored data should pass explicit migration/normalization/formation boundaries before runtime instantiation.
- Runtime may mutate entity/component instances, but mutation authority must remain explicit and domain-owned.
- Editor composition must stay capability-driven; the editor should not invent runtime semantics absent engine/domain contracts.
- Preview, simulate, and play execution use an app-owned external runtime child process in M5; domain crates own the preview protocol vocabulary and runtime/app crates own execution and transport wiring.
- Network crates may carry generic typed payload envelopes, but editor preview semantics must stay out of generic net protocol enums.
- Any future scripting integration must preserve Rust/domain ownership of correctness and invariants.
- Scripting boundaries remain language-neutral even when Rhai is used as the first adapter; adapter-specific types must not leak into domain/runtime contracts.
- World-space/screen-projected UI attachment flows are post-MVP and should not be treated as implied by current scene-overlay UI template support.

## M5 External Runtime Preview Boundary

The M5 preview boundary is intentionally concrete and narrow:

- `domain/editor/editor_preview` owns engine-agnostic preview protocol DTOs such as `PreviewSessionId`, `PreviewMode`, `PreviewCommand`, `PreviewEvent`, `ReloadDecision`, `ReloadStatus`, `RuntimeProductRef`, ratified payload references, checked preview payload metadata, and serialized bootstrap stdout format.
- `net/engine_net` owns generic bidirectional typed payload messages that can carry preview DTOs without knowing their semantics.
- `net/engine_net_quic` owns loopback QUIC transport for those generic payload messages.
- `apps/runenwerk_runtime_preview` owns the child process runtime host, separate preview/play window, bootstrap connection output, headless command loop, and command/event handling for start, mode, heartbeat, product, reload, and shutdown requests.
- `apps/runenwerk_editor/src/runtime/preview_process/` owns editor-side process spawning, connection management, heartbeat/shutdown, mode requests, status ingestion, bounded pending command queueing, and child lifecycle fallback.
- `engine` owns generic runtime loading, shader reload status vocabulary, and `world_sdf` runtime intake helpers.

The editor app may request mode transitions and publish ratified runtime product references. It must not write runtime ECS state, `SdfChunkStore` internals, shader registries, or renderer resources directly.

## Data Reload And Restart Boundary

M5 completes only the existing product families that already have owning contracts:

- scenes and scene templates through existing editor persistence and engine scene seams;
- asset catalog revisions and import/field-product outcomes through `domain/asset` and app-owned catalog runtime;
- field products and `world_sdf` payload packages through `domain/world_sdf` ratification and engine intake;
- shaders through engine reload payloads mapped to editor preview reload status at the app boundary;
- UI/editor definitions through existing `ui_definition` and `editor_definition` activation contracts.

Future material graph, texture, procgen, particle, physics, animation, gameplay graph, script, and graph execution families must emit typed `unsupported` or `restart_required` statuses until their owning domains and formed product contracts exist. M5 must not invent those domains inside the preview bridge.

Reload status must preserve the last valid runtime product when a new import, field product, shader revision, or payload package fails validation. Unsafe structural changes, including Rust component layout changes, new systems, plugin graph changes, renderer backend changes, ECS internals, and network schema changes, require a preview-session or runtime-process restart status instead of attempting a live swap.

## Validation Expectations

M5 closeout validation covers:

- invalid edit, preview, simulate, and play transitions reject through scoped mode contracts;
- a headless runtime preview child can spawn, connect over loopback QUIC, round-trip heartbeat, acknowledge shutdown, and exit;
- reload classification is deterministic for live reload, preview-session restart, runtime-process restart, unsupported, failed-preserved, and rejected outcomes;
- failed reloads preserve the prior valid runtime product;
- ratified `world_sdf` payload packages reach runtime stores through an engine-owned intake path, not direct editor mutation;
- shader registry reload status reaches existing editor console, asset/import, and viewport/product diagnostics surfaces through app-boundary mapping.

Closeout evidence is recorded in [`../../reports/closeouts/m5-runtime-preview/closeout.md`](../../reports/closeouts/m5-runtime-preview/closeout.md).

## Deferred Detailed Draft

The prior long-form target draft has been moved (verbatim) to keep this active document concise and implementation-grounded:

- [`../deferred/engine-game-runtime-editor-ecs-scripting-hot-reload-preserved-target-draft.md`](../deferred/engine-game-runtime-editor-ecs-scripting-hot-reload-preserved-target-draft.md)

Use this active doc for current boundaries and gap analysis; use the deferred preserved draft for deeper aspirational details.
