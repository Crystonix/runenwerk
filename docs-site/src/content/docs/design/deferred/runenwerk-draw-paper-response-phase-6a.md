---
title: Runenwerk Draw Paper Response Phase 6A
description: Deferred design for the first deterministic domain-owned paper response slice in drawing CPU ink tile formation.
status: deferred
owner: drawing
layer: domain
canonical: true
last_reviewed: 2026-09-10
related_docs:
  - ../active/drawing-authoring-and-comic-layout-platform-design.md
  - ../../domain/drawing/README.md
  - ../../apps/runenwerk-draw/README.md
  - ../../apps/runenwerk-draw/roadmap.md
  - ../implemented/runenwerk-draw-stroke-fidelity-phase-1-preview-final-parity.md
---

# Runenwerk Draw Paper Response Phase 6A

## Deferred status

Phase 6A remains a valid drawing-quality direction but is not active work. Stroke-fidelity prerequisites being materially implemented do not activate this phase automatically. Reactivate only through a new accepted drawing-quality/paper-response implementation issue after a fresh source, ownership, product-identity, and formation-contract census confirms the design is still current.

## Summary

This document plans the smallest deterministic paper response slice where
existing `PaperDescriptor` and `PaperHeightSource` values affect CPU ink tile
formation in a product-safe, domain-owned way.

Phase 6A is a visible drawing-quality slice, not a new interaction or renderer
slice. `domain/drawing` owns paper semantics, deterministic paper sampling, CPU
tile formation, diagnostics, lineage, and product/cache identity. The Draw app
may choose a default paper preset, but it must not own paper response semantics.

The first slice should support `PaperHeightSource::None` as the unchanged
baseline and `PaperHeightSource::ProceduralNoise` as the only pixel-changing
paper height source. Imported height fields, formed-product references, and
SDF-derived height sources remain explicit diagnostic/no-op paths until their
product contracts exist.

## Investigation Findings

`domain/drawing/src/paper/descriptor.rs::PaperDescriptor` already defines
`paper_id`, `schema_version`, `revision`, `name`, `roughness`, `absorbency`,
and `height_source`.

`domain/drawing/src/paper/height_source.rs::PaperHeightSource` already defines
`None`, `ProceduralNoise { seed, scale, amplitude }`,
`FormedProductReference { product_ref }`,
`ImportedHeightField { asset_ref }`, and `SdfDerived { source_ref }`.

`domain/drawing/src/ratification/ratifier.rs::ratify_papers` already checks
unique paper ids, nonzero schema and revision, nonempty names, finite bounded
`roughness` and `absorbency`, valid procedural noise `scale` and `amplitude`,
and nonempty references for external height sources.

`domain/drawing/src/composition/source.rs::PaperSource` exists as a composition
source node descriptor with a `paper_id`, and
`domain/drawing/src/composition/node.rs::DrawingCompositeNode::PaperSource`
can hold that descriptor. Current ratification accepts paper source nodes but
does not yet resolve an active paper for ink formation.

`apps/runenwerk_draw/src/app/document_factory.rs::minimal_drawing_document`
creates one `PaperDescriptor` named `Smooth Paper` with
`PaperHeightSource::None`.

`domain/drawing/src/tile/formation.rs::rasterize_stroke` and
`domain/drawing/src/tile/formation.rs::rasterize_dab` currently form ink
payloads from strokes and brushes only. Paper roughness, absorbency, and height
source do not change pixels today.

`domain/drawing/src/tile/formation.rs::lineage_for_strokes` already records
`PaperLineageRef` values for every `document.papers` entry in
`DrawingProductLineage::paper_revisions`.

`domain/drawing/src/tile/formation.rs::hash_formation_inputs` currently hashes
document identity, canvas bounds, formation policy, brush descriptors, strokes,
and samples. It does not hash paper descriptor fields or height source values
directly.

`domain/drawing/src/tile/formation.rs::drawing_ink_tile_source_cache_key` and
the per-product `cache_key` currently include formation kind, quality class,
document id, document revision, output id, tile id, and formation version. They
do not include a paper response token beyond any document revision change.

`domain/drawing/tests/ink_tile.rs` already covers deterministic CPU ink tile
payloads, preview/final quality identity, brush-driven payload changes,
formation version identity, product descriptors, publication contracts, query
snapshot contracts, and paper descriptor presence with `PaperHeightSource::None`.
It does not yet cover paper response.

`docs-site/src/content/docs/design/active/drawing-authoring-and-comic-layout-platform-design.md`
defines Phase 6 as paper height and procedural surface inputs. Its completion
criteria include explicit product contracts for paper sources, deterministic
paper interaction in ink formation, CPU reference formation, GPU consumption of
the same descriptors later, and diagnostics for unsupported paper sources.

`docs-site/src/content/docs/domain/drawing/README.md` names paper response as a
next phase, while `docs-site/src/content/docs/apps/runenwerk-draw/README.md`
and `docs-site/src/content/docs/apps/runenwerk-draw/roadmap.md` still list
paper response as deferred after the rendering foundation.

## Problem Statement

The repository already has paper descriptors, ratification, and lineage
vocabulary, but paper is not yet a pixel-affecting input to CPU ink tile
formation. That creates a quality ceiling: Draw can present a paper shell, but
formed ink still behaves as if the surface is perfectly smooth.

The unsafe shortcut would be to add app-local visual texture or renderer-only
paper noise. That would make visible output diverge from CPU tile truth and
would bypass product identity, cache identity, and lineage. Phase 6A must
instead make paper response a deterministic `domain/drawing` formation input.

The current product identity gap is explicit: paper lineage records revisions,
but `hash_formation_inputs` and cache keys do not directly encode
pixel-changing paper response inputs. If paper response changes pixels, the
formation hash, descriptor generation, source cache key, and product cache key
must account for those inputs.

## Proposed Boundary

`domain/drawing` owns:

- paper response descriptor resolution;
- procedural paper height sampling;
- paper response diagnostics;
- CPU ink tile formation changes;
- product lineage and determinism identity for paper response inputs;
- tests proving deterministic outputs and identity changes.

`apps/runenwerk_draw` owns:

- selecting a default app fixture paper preset;
- routing formed products through the existing preview/final/product lifecycle;
- preserving CPU tile products as the correctness oracle.

`engine/render` owns no paper semantics in Phase 6A. GPU ink validation and
promotion remain derived proof/acceleration paths and must not become paper
truth.

`native_tablet_input`, `ToolSession`, and input routing are unrelated to this
slice.

## First-Slice Decision

Use only `PaperHeightSource::ProceduralNoise` for pixel-changing paper response
in Phase 6A.

`PaperHeightSource::None` must preserve the current payload behavior exactly.

`PaperHeightSource::FormedProductReference`,
`PaperHeightSource::ImportedHeightField`, and `PaperHeightSource::SdfDerived`
should produce explicit formation diagnostics and no paper response until their
formed-product, asset, or SDF product contracts exist. They should not silently
pretend to be supported.

Use a domain-owned active paper resolver with a conservative v1 policy:

- if an active `PaperSource` can be resolved from the drawing composition, use
  the referenced paper;
- otherwise, if the document has exactly one paper descriptor, use it as the
  legacy/default paper;
- if the document has no paper, use no paper response;
- if the document has multiple papers and no resolved active paper, disable
  paper response for formation and emit an explicit diagnostic.

This keeps the app fixture simple while avoiding arbitrary multi-paper behavior.

## Domain Model Plan

Add internal domain formation helpers rather than app-visible behavior first:

- `PaperResponseContext` or equivalent internal tile-formation context;
- `ActivePaperResponse` or equivalent resolver result for the active paper;
- deterministic procedural height sampling for
  `PaperHeightSource::ProceduralNoise`;
- explicit unsupported-source diagnostics for external/imported/SDF height
  sources.

The response should be dry ink surface interaction only. It may modulate dab
coverage, edge breakup, or alpha in a bounded deterministic way using:

- `PaperDescriptor::roughness`;
- `PaperDescriptor::absorbency`;
- `PaperHeightSource::ProceduralNoise { seed, scale, amplitude }`;
- existing brush fields such as `InkBrushDescriptor::viscosity` and
  `InkBrushDescriptor::absorption_response`.

The response must not introduce wetness state, diffusion, drying, pigment
movement, temporal simulation, or renderer-only effects.

## Tile Formation Plan

Thread the resolved paper response through
`domain/drawing/src/tile/formation.rs::form_drawing_ink_tile_records_inner`
into `rasterize_stroke`, `append_segment_dabs`, and `rasterize_dab` without
changing the public app runtime path.

For `PaperHeightSource::None`, keep the current rasterization path exactly
unchanged.

For `PaperHeightSource::ProceduralNoise`, sample deterministic paper height in
canvas space at pixel or dab sample positions. The sampling must be stable for
the same seed, scale, amplitude, canvas coordinate, tile policy, and formation
version on all platforms.

The first response should be visually small but testable. Prefer a bounded
coverage or alpha modulation over a broad simulation model. It should not
change tile extent logic unless tests prove the chosen response can affect
affected-tile bounds.

Unsupported height sources should add a diagnostic and fall back to no paper
response for pixels.

## Ratification Plan

Keep existing descriptor ratification in
`domain/drawing/src/ratification/ratifier.rs::ratify_papers` as the baseline.

Phase 6A implementation should add only missing validation that is required to
make active paper resolution safe. Likely candidates:

- if composition-based active paper resolution is implemented, ensure a
  `PaperSource` references an existing `PaperDescriptor`;
- if multiple active paper sources are possible, emit a deterministic diagnostic
  rather than picking one by traversal accident;
- keep unsupported source handling in formation diagnostics unless the document
  descriptor itself is invalid.

Do not reject valid existing documents merely because their external paper
height source is not yet supported for pixel response. Invalid references that
are already empty remain ratifier errors.

## Product Lineage / Determinism Plan

Any paper input that changes pixels must affect product and cache identity.

Phase 6A should update formation identity in
`domain/drawing/src/tile/formation.rs::hash_formation_inputs` to include the
resolved active paper response inputs:

- paper id;
- paper schema version and revision;
- roughness;
- absorbency;
- height source kind;
- procedural noise seed, scale, and amplitude for `ProceduralNoise`;
- an explicit unsupported/no-op token for unsupported height sources.

The per-product descriptor generation already hashes payload bytes, but the
formation input hash should still include paper response inputs so
`determinism_key` changes before payload comparison.

The source cache key and product `cache_key` should include a stable paper
response token or otherwise derive from the expanded formation input identity.
Document revision alone is not enough to distinguish same-id, same-revision test
documents with different paper response descriptors.

`DrawingProductLineage::paper_revisions` already records all paper revisions.
Phase 6A may keep that conservative lineage for the first slice, but tests must
prove the active paper revision is present. A later cleanup can narrow lineage
to resolved active paper only if broader lineage becomes too invalidating.

If Phase 6A changes the default rasterization behavior for any non-`None`
source, bump or explicitly test `DrawingTileFormationPolicy::formation_version`
usage so cache identity stays migration-safe.

## App Fixture / UX Plan

`apps/runenwerk_draw` should not own paper response semantics.

The eventual app change should be limited to choosing a default paper preset
that exercises Phase 6A once the domain behavior exists. The minimal document
can move from `PaperHeightSource::None` to a deterministic
`PaperHeightSource::ProceduralNoise` preset only after domain tests prove:

- `None` preserves old output;
- procedural paper changes output deterministically;
- product/cache identity changes with paper settings.

No paper UI, paper picker, package IO, layer UI, or Workbench integration is
part of Phase 6A.

## Test Plan

Add focused `domain/drawing/tests/ink_tile.rs` coverage:

- `PaperHeightSource::None` produces the same payloads as the current baseline;
- identical `ProceduralNoise` seed/settings produce identical products and
  determinism keys;
- changing procedural seed changes payload, product id, determinism key, and
  cache identity;
- changing procedural scale or amplitude changes determinism identity and, for
  a stable stroke fixture, changes payload;
- changing roughness or absorbency changes determinism identity and the chosen
  paper-response payload fixture;
- active paper revision appears in product lineage;
- unsupported `FormedProductReference`, `ImportedHeightField`, and `SdfDerived`
  produce explicit diagnostics and no pixel response;
- multiple papers without a resolved active paper produce the planned
  diagnostic/no-op behavior;
- invalid procedural scale or amplitude remains rejected by ratification.

Add focused `apps/runenwerk_draw/tests/app_shell.rs` coverage only after the
domain behavior is in place:

- the app default paper preset remains ratified;
- Draw preview/final publication still preserves CPU tile truth;
- product publication and query snapshot barriers are unchanged;
- GPU validation remains proof/promotion only and does not become paper truth.

Eventual implementation validation:

```text
cargo test -p drawing --test ink_tile
cargo test -p runenwerk_draw --test app_shell
cargo test -p runenwerk_draw
cargo check --workspace
task docs:validate
git diff --check
```

## Docs Plan

Update `docs-site/src/content/docs/domain/drawing/README.md` after
implementation to explain the first supported paper response behavior,
unsupported height source diagnostics, and product/cache identity guarantees.

Update `docs-site/src/content/docs/apps/runenwerk-draw/README.md` only after
the app fixture actually selects a procedural paper preset.

Update `docs-site/src/content/docs/apps/runenwerk-draw/roadmap.md` after the
slice is implemented and validated, moving paper response from fully deferred
to first-slice complete with remaining watercolor/procedural material work
still deferred.

If implementation changes product identity policy, add closeout evidence under
the appropriate report path instead of mixing raw test artifacts into prose
docs.

## Non-Goals

- watercolor or wet media simulation;
- diffusion, drying, pigment movement, granulation, or temporal paper state;
- export/package IO or native package sidecars;
- layer UI, paper picker UI, or Workbench integration;
- engine render changes;
- GPU paper response or GPU authority over drawing truth;
- ToolSession, input routing, radial menu, offhand input, or new tool behavior;
- external/imported height field product contracts;
- SDF-derived paper product formation;
- formed paper material product families;
- persistent cache storage or cache pruning policy;
- eraser compositing.

## Risks And Mitigations

Risk: paper response changes pixels without changing cache identity.
Mitigation: include active paper response inputs in formation hash,
determinism key, source cache key, product cache key, and tests.

Risk: the app owns paper semantics by choosing ad hoc visual noise.
Mitigation: keep sampling and response in `domain/drawing`; let the app choose
only a descriptor preset.

Risk: multiple paper descriptors produce arbitrary output.
Mitigation: use a deterministic active paper resolver and diagnostic/no-op
behavior when active paper is ambiguous.

Risk: unsupported height sources silently produce smooth paper.
Mitigation: emit explicit formation diagnostics for unsupported sources, while
keeping pixels as no-op until product contracts exist.

Risk: the first response becomes hidden watercolor work.
Mitigation: restrict Phase 6A to bounded dry ink coverage/alpha modulation and
defer wetness state, diffusion, drying, and pigment movement.

Risk: `PaperHeightSource::None` regresses existing Draw output.
Mitigation: add baseline tests proving `None` preserves current payloads and
publication behavior.

## Implementation Prompt For Later

This implementation prompt is dormant while the design remains deferred. A reactivation issue must revalidate it against current source before execution.

Implement Paper Response Phase 6A for `domain/drawing` and the minimal
`runenwerk_draw` fixture. Preserve CPU tile formation as the correctness oracle.

Scope:
- `domain/drawing` paper response resolver, procedural height sampling,
  formation diagnostics, determinism/cache identity, and tests;
- `apps/runenwerk_draw` default paper preset only after domain behavior is
  proven;
- no engine render changes;
- no GPU authority changes;
- no watercolor, package IO, layer UI, Workbench integration, or new tool
  behavior.

Required behavior:
- `PaperHeightSource::None` preserves existing CPU ink tile payload behavior;
- `PaperHeightSource::ProceduralNoise` deterministically affects CPU ink tile
  formation;
- unsupported paper height sources emit explicit diagnostics and no-op for
  pixel response;
- active paper response inputs affect determinism keys, product ids, and cache
  keys;
- product lineage includes the active paper revision;
- Draw product publication/query snapshot lifecycle remains unchanged.

Validation:

```text
cargo test -p drawing --test ink_tile
cargo test -p runenwerk_draw --test app_shell
cargo test -p runenwerk_draw
cargo check --workspace
task docs:validate
git diff --check
```
