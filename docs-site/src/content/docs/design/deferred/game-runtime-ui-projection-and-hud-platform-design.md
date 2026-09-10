---
title: Game Runtime UI Projection And HUD Platform
description: Deferred architecture design for a runtime-proven game UI projection and HUD platform, starting with an SDF screen-HUD proof.
status: deferred
owner: workspace
layer: domain/ui-definition / engine-runtime
canonical: true
last_reviewed: 2026-09-10
related_adrs:
  - ../../adr/accepted/0001-use-domain-owned-commands.md
  - ../../adr/accepted/0004-separate-description-from-execution.md
  - ../../adr/accepted/0005-projections-are-derived-state.md
  - ../../adr/accepted/0009-ui-interaction-formation-v2.md
related_designs:
  - ../accepted/app-neutral-ui-composition-design.md
  - ../active/ui-designer-interface-lab-platform-design.md
  - ../accepted/ui-designer-target-projection-profiles-design.md
  - ../accepted/ui-designer-view-model-capability-and-intent-binding-design.md
  - ../accepted/ui-designer-live-preview-fixtures-scenarios-and-target-matrix-design.md
  - ../accepted/ui-designer-production-readiness-and-evidence-design.md
  - ../active/engine-game-runtime-editor-ecs-scripting-hot-reload-design.md
related_roadmaps:
  - ../../domain/ui/roadmap.md
---

# Game Runtime UI Projection And HUD Platform

## Deferred status

This architecture remains valid but is not active implementation work. The bounded-context owner is still unresolved. Reactivate only through an accepted owner-boundary/implementation issue that fixes dependency direction, source-of-truth boundaries, exact proof scope, and roadmap placement where cross-family sequencing matters. Do not create a game-runtime UI owner crate or treat the SDF proof as activation before that decision.

## Decision

This design defines the runtime-proven game-runtime UI projection and HUD target
historically decomposed as `PT-GAME-RUNTIME-UI`: HUD composition, read-only
view-model binding, validated game intents, runtime UI expression submission,
and an SDF screen-HUD proof.

It consumes the completed UI Designer contracts as design input. It does not
reopen the historical UI Designer or UI Lab sequences and does not claim
perfectionist verification. UI Lab evidence may be cited only as an
evidence-pattern reference. A separate no-gap audit must exist before any later
`perfectionist_verified` claim.

`PT-GAME-RUNTIME-UI`, `PM-GAME-RUNTIME-UI-*`, `WR-104`, and the old roadmap
intake are retained only as architecture/decomposition provenance. There is no
live production-track or WR lifecycle authority in this design. Future work
requires an owning GitHub issue and roadmap placement when sequence across
active families matters; the delivery pull request owns review and exact-head
validation.

## Architecture Governance

Architecture governance findings for this design:

- DDD bounded context owner is unresolved by implementation and must be decided
  by accepted design or ADR before a game UI owner crate exists.
- Current generic UI definition owner remains `domain/ui/ui_definition`.
- Future game-runtime UI target extensions may live in `domain/game_ui`,
  `domain/game/interface`, or another accepted owner only after governance
  records dependency direction, source-of-truth boundaries, and fitness
  functions.
- `engine` owns runtime composition, frame preparation, render-flow execution,
  and UI expression submission. It does not own game semantics.
- Examples and apps own concrete proof state, proof view models, and application
  of accepted intents. UI projections must not mutate state directly.
- ADR need is deferred to the owner-boundary milestone. A new ADR or accepted
  design update is required before adding a game-runtime UI owner crate,
  changing dependency direction, or making projection output authoritative.
- ATAM-lite priority order: ownership/correctness first, fail-closed diagnostics
  second, runtime evidence third, author/developer ergonomics fourth,
  performance fifth.
- Team Topologies label: stream-aligned game runtime product work with
  complicated-subsystem support from `domain/ui`, engine render/runtime, and
  example/app evidence owners.

## Governing Invariants

- UI definitions are source truth only for UI/interface structure.
- UI definitions may bind only to read-only domain-owned view-model packets.
- UI definitions may emit validated intent proposals only.
- UI definitions must not mutate gameplay, render, runtime, scene, simulation,
  save-game, network, or project truth.
- Game-runtime UI projection must not depend on `domain/editor/editor_shell`,
  Workbench host policy, editor command routing, or editor provider vocabulary.
- Runtime/app/provider layers consume derived projection output and must not
  become canonical UI/interface truth.
- Engine UI submission is expression infrastructure, not game semantic
  authority.
- Unsupported target-profile features, denied capabilities, missing view-model
  packages, stale data, invalid intents, and unsafe editor/game coupling fail
  closed with typed diagnostics.

### App-neutral composition boundary

The accepted composition design provides reusable structure without promoting
game HUD or world-space semantics into the current cutover:

```text
GameViewportTarget -> PresentationTarget profile
GameScreen / HudRoot -> future app-owned composition definition
HudRegion -> future region profile
HudInstance -> future MountedContentRef profile
```

This does not resolve the future game-domain owner and does not authorize
`domain/game_ui`, world-space UI, or runtime implementation. Concrete
game state, entity anchors, culling, occlusion, input policy, and gameplay
commands remain outside generic UI. Game runtime work must not consume editor
workspace/tool-surface vocabulary.

## Current Code Truth

- `engine/examples/sdf_render_flow/runtime/app.rs::update_sdf_view_and_animation_system`
  currently projects the selected SDF view mode into `WindowState` title text.
  That is app chrome, not a game UI surface.
- `engine/examples/sdf_render_flow/rendering/graph.rs::build_render_flow`
  currently ends in terminal `present_pass("sdf.present")`. A later UI proof
  must copy scene color into `surface.color` and run the built-in UI composite
  pass after the scene output.
- `engine/src/plugins/render/runtime/ui_submission.rs::collect_runtime_ui_frame_submissions_system`
  currently collects hardcoded scene overlay and debug metrics producers. A
  later runtime UI slice must provide typed, deterministic producer registration
  and ordering without adding game semantics to engine render code.
- `engine/src/plugins/debug_metrics/mod.rs::debug_metrics_overlay_system`
  is diagnostics UI. It may provide metrics source evidence, but game HUD must
  not be implemented by reusing the debug overlay frame.
- `domain/ui/ui_widgets/src/tabs.rs::tabs`,
  `domain/ui/ui_runtime/src/runtime/ui_runtime.rs::UiRuntime`, and
  `UiInteraction::TabSelected` already provide retained UI substrate that the
  SDF proof should consume through accepted target projection and intent
  boundaries.

## Target Runtime Flow

```text
Authored UI definition or accepted proof fixture
  -> Canonical UI IR
  -> game-runtime target projection plan
  -> read-only game/runtime view-model packet
  -> retained UI runtime frame
  -> engine UI expression submission
  -> validated game intent proposal
  -> owning domain/app/example applies mutation
```

The SDF proof is the first runtime evidence target, not the architecture owner.
It must prove screen HUD and control behavior without implementing
world-space/entity-attached UI.

## Proposed Decomposition

The historical PM sequence remains useful as a proposed implementation
decomposition, not current lifecycle state:

- `PM-GAME-RUNTIME-UI-001`: governance, owner boundary, code-truth matrix, and
  follow-on decomposition.
- `PM-GAME-RUNTIME-UI-002`: accepted game-runtime target extension contract and
  owner decision.
- `PM-GAME-RUNTIME-UI-003`: read-only view-model and validated intent contract
  activation.
- `PM-GAME-RUNTIME-UI-004`: generic runtime UI expression submission.
- `PM-GAME-RUNTIME-UI-005`: SDF screen HUD runtime proof.
- `PM-GAME-RUNTIME-UI-006`: evidence, docs, API ergonomics, and hardening.
- `PM-GAME-RUNTIME-UI-007`: runtime-proven closeout and perfectionist-audit
  intake.

An owning GitHub issue may adopt, refine, split, or reject this decomposition
when the work is activated.

World-space and screen-projected attachment UI remains a separate future target,
historically labelled `PT-GAME-WORLDSPACE-UI`. Nameplates, damage numbers, boss
frames, entity-attached widgets, and split-screen attachment contracts must wait
for explicit authored binding contracts, viewport/projection readiness,
renderer readiness where needed, and runtime formation seams.

## Evidence Rules

Descriptor-only, title-text-only, debug-overlay-only, or screenshot-free claims
are insufficient for runtime-proven game UI.

Runtime proof must show that:

- FPS/status and tab controls render in-frame through UI composition;
- window title no longer carries interactive game UI state;
- UI tab selection emits a validated intent proposal;
- the SDF/example state owner applies the accepted intent;
- UI producer ordering is deterministic across game HUD, scene overlay, and
  debug overlay submissions;
- render-flow pass shape includes scene color copy to `surface.color` followed
  by UI composite;
- diagnostics are typed for unsupported target features, denied capabilities,
  missing view-model data, stale data, invalid intents, and unsafe coupling.

## Current Activation Rule

This document is durable deferred architecture, not a work tracker. Before product code
changes, create or select an owning GitHub issue that resolves the still-open
owner boundary and names exact scope, accepted base, validation, and runtime
evidence. Put the work on the maintained roadmap when cross-family sequencing
matters. Deliver through a pull request with one unchanged reviewed head and
repository-owned exact-head CI/Documentation Build evidence.

## Stop Conditions

- A slice creates `domain/game_ui` or equivalent before accepted owner-boundary
  design or ADR.
- A UI projection mutates domain, runtime, render, scene, gameplay, save-game,
  network, or project truth directly.
- Engine render code starts owning game HUD semantics.
- SDF proof changes become the architecture instead of a bounded consumer proof.
- World-space/screen-projected attachment UI is folded into the first screen HUD
  proof instead of remaining a separate design slice.
- The work claims `perfectionist_verified` before a separate completed no-gap
  audit with empty known gaps.

## Validation

Any implementation must add focused crate tests for target-profile
compatibility, fail-closed diagnostics, view-model/intent behavior, engine UI
submission ordering, SDF render-flow UI composite shape, and in-frame HUD proof.
Broad validation at the reviewed head includes:

```text
cargo validate
git diff --check
CI=true pnpm --dir docs-site build
```
