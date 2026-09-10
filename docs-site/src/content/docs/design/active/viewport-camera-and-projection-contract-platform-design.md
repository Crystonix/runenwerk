---
title: Viewport Camera And Projection Contract Platform
description: Long-term platform design for renderer procedural projection, editor viewport camera semantics, viewport surface presentation, and example evidence.
status: active
owner: workspace
layer: workspace
canonical: true
last_reviewed: 2026-09-10
related_designs:
  - workspace-viewport-expression-upgrade-design.md
  - ../implemented/renderer-procedural-population-hardening-platform-design.md
  - ../implemented/render-product-surface-foundation-bundle-design.md
  - ../deferred/game-runtime-ui-projection-and-hud-platform-design.md
---

# Viewport Camera And Projection Contract Platform

## Decision

This design owns the long-term camera, projection, viewport presentation, and
surface-fit contract boundary historically decomposed as
`PT-VIEWPORT-PROJECTION`. It crosses renderer, editor viewport, app adapter,
example, and UI embedding boundaries.

This is not a renderer-only cleanup and not a UI feature target. The platform
must preserve these source-truth rules:

- camera intent belongs to the producer or editor viewport context that owns the
  interaction semantics;
- renderer code may own derived projection math, uniforms, surface-fit
  transforms, diagnostics, and example evidence;
- `PreparedViewFrame` carries view identity, target size, history identity, and
  render preparation data, not camera intent;
- UI primitives embed product or viewport surfaces and must not own camera or
  projection semantics;
- app/runtime adapters pack uniforms, route input, and bridge CPU picking to GPU
  products without becoming a new semantic owner.

The design exists because the current codebase has correct pieces in separate
owners, but the overall contract is not yet production-complete.

`PT-VIEWPORT-PROJECTION`, `PM-VIEWPORT-PROJECTION-*`, `WR-101`, `WR-102`,
`WR-104`, and `WR-106` are retained as historical decomposition/provenance
vocabulary. They are not current work authority. Any implementation requires an
owning GitHub issue, canonical roadmap sequencing when a maintained roadmap is
relevant, and a reviewed pull request with exact-head validation evidence.

## Current Evidence And Gaps

Historical `WR-101` completed reusable renderer procedural 2D camera projection
for the boids proof. That work is evidence for aspect-correct fill-viewport
projection, equal projected world x/y scale, and producer-owned camera intent.

The remaining platform gaps are broader:

- renderer examples still contain one-off surface/aspect decisions that should
  be reviewed against a typed presentation contract before final hardening;
- editor viewport camera state exposes a supported-looking orthographic
  projection variant without end-to-end runtime, rendering, and picking
  evidence;
- editor CPU picking and GPU projection derivation are close but not yet backed
  by one explicit drift-guard contract;
- viewport surface embedding still needs to stay camera-free while future
  identity typing and product-surface evidence improve.

These gaps should not be patched as local example fixes. They need explicit
owning issues, accepted design gates where ownership changes, disjoint write
scopes, and reviewable evidence.

## Ownership

Renderer ownership is limited to derived projection and presentation data:

- `engine/src/plugins/render/procedural/camera.rs`;
- module `engine::plugins::render::procedural::camera`;
- future renderer-owned surface-fit or procedural-ray helpers after accepted
  design.

Editor viewport ownership covers semantic camera and projection behavior:

- `domain/editor/editor_viewport/src/camera.rs`;
- module `domain::editor::editor_viewport::camera`;
- perspective, orthographic, orbit, pan, zoom, ray, and screen projection
  vocabulary once accepted.

Editor app adapter ownership covers runtime packing and input/output routing:

- `apps/runenwerk_editor/src/runtime/resources.rs`;
- `apps/runenwerk_editor/src/runtime/systems/picking.rs`;
- target-local uniform packing, viewport-local picking, and CPU/GPU drift tests.

UI ownership remains camera-free:

- `domain/ui`;
- surface embedding primitives, slots, and product or viewport surface identity;
- no camera intent, projection policy, world ray, or gameplay/editor camera
  semantics.

## Historical Milestone Decomposition

The former production milestone sequence remains useful as architecture
planning, but it is not live lifecycle state:

- `PM-VIEWPORT-PROJECTION-001`: governance, architecture, ADR triage, and
  follow-on decomposition.
- `PM-VIEWPORT-PROJECTION-002`: renderer surface-fit and procedural projection
  contracts.
- `PM-VIEWPORT-PROJECTION-003`: editor viewport camera and projection hardening.
- `PM-VIEWPORT-PROJECTION-004`: example migration and product-surface evidence.
- `PM-VIEWPORT-PROJECTION-005`: runtime-proven closeout and renderer-only
  no-gap-audit handoff.

The historical `WR-106` governance intake did not implement renderer, editor,
UI, or example behavior. A current owning issue may adopt, refine, split, or
reject this decomposition when work is activated.

## Bounded Follow-On Candidates

Useful follow-on slices remain:

- renderer surface-fit and procedural projection contracts;
- editor viewport camera and projection hardening;
- example and product-surface evidence migration;
- runtime-proven closeout and renderer perfection handoff.

Each activated slice must name exact ownership, write scope, focused tests,
runtime evidence, accepted base, and stop conditions in its owning GitHub issue.
The delivery pull request owns the complete diff, reviewed feature head, and
acceptance evidence.

## ADR Triggers

No ADR is required while future work preserves producer-owned camera intent,
renderer-derived uniforms, and camera-free UI embedding.

An ADR or accepted design update is required before:

- camera source truth moves into `PreparedViewFrame`;
- UI owns camera or projection semantics;
- a shared cross-domain camera ABI is introduced;
- renderer APIs start carrying editor-specific viewport concepts;
- editor camera contracts become canonical outside editor viewport ownership.

## Exclusions

This design does not own richer boid/flock behavior. Flock identity, overlap
policy, multiple flocks, affinity groups, split/merge behavior, attractors, and
semantic population dynamics remain separate behavior-authoring work; the old
`WR-102` label is provenance for that direction only.

This design does not own game runtime HUD, world-space attachment UI,
screen-space nameplates, damage numbers, or gameplay UI projection. Screen-space
game HUD remains owned by the game-runtime UI design; world-space and
entity-attached game UI remains a separate deferred boundary.

This design does not reopen the historical `WR-101` delivery. Its completed
renderer procedural camera evidence remains an input dependency.

## Fitness Functions

Before any implementation is accepted, the owning issue and pull request must
prove the relevant focused checks for:

- renderer landscape, portrait, square, and extreme aspect projection;
- equal projected world x/y scale where a world projection contract requires it;
- surface-fit behavior that does not hide camera source-truth movement;
- editor CPU picking and GPU projection drift;
- orthographic projection support or explicit removal/deferment;
- examples proving migration without boids-only shortcuts;
- guards proving no viewport-specific camera truth enters `PreparedViewFrame`;
- guards proving generic UI primitives remain camera-free.

Broad repository validation at one unchanged reviewed head includes focused
crate tests plus `cargo validate`, `git diff --check`, and the documentation
build. Repository-owned exact-head CI and Documentation Build are the acceptance
evidence for the reviewed revision.
