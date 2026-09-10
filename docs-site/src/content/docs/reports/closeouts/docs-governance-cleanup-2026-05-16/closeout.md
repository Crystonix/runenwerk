---
title: Docs Governance Cleanup Closeout 2026-05-16
description: Closeout evidence for the strict documentation lifecycle, ownership-map, batch-history, and validation hardening milestone.
status: completed
owner: workspace
layer: workspace
canonical: true
last_reviewed: 2026-05-16
related:
  - ../../../workspace/roadmap-items.yaml
  - ../../../workspace/roadmap-decision-register.md
  - ../../../guidelines/domain-map.md
  - ../../../workspace/crate-docs-status.md
  - ../../../design/implemented/README.md
  - ../../../reports/batches/README.md
---

# Docs Governance Cleanup Closeout 2026-05-16

## Scope

This closeout records the no-shortcuts documentation governance milestone for
`WR-005`. Existing dirty work was preserved. The milestone only touched docs
and docs-validation tooling, except for generated roadmap outputs produced by
`task roadmap:render`.

Baseline inspection used:

- `git status --short`
- `git diff --name-status`
- focused diffs for active design lifecycle files, canonical maps, batch
  manifests, and docs validation tooling

## Lifecycle Results

Implemented baseline designs were moved from `design/active/` to
`design/implemented/` and indexed in
[`design/implemented/README.md`](../../../design/implemented/README.md):

- Foundation Commands
- Foundation Schema
- Foundation Vocabulary Crates
- Editor Self-Authoring and UI Workspace Design
- Runenwerk Editor Workspace-Document-Mode-Panel Architecture
- Workspace Identity Contract and Migration Map
- UI Definition Formation Framework Design
- Render Product Surface Foundation Bundle Design
- Viewport Dynamic Product Target Allocation Design

The remaining active designs that contain completed-slice evidence are marked
with `lifecycle_exception: active_phase_evidence` and include a lifecycle note.
They remain active because the broader design scope is not fully implemented:

- Drawing Authoring and Comic Layout Platform Design
- Editor Procedural Content and Simulation Workflow Plan

## Source-Of-Truth Repairs

- Canonical domain ownership was aligned in
  [`guidelines/domain-map.md`](https://github.com/dornglut/runenwerk/blob/deb649bff7acb49133784b6365de6cbedfe9c09f/docs-site/src/content/docs/guidelines/domain-map.md).
- Crate documentation coverage now includes `domain/asset`, `domain/product`,
  grouped `domain/ui/*`, grouped `domain/editor/*`, and
  `apps/runenwerk_runtime_preview` in
  [`workspace/crate-docs-status.md`](../../../workspace/crate-docs-status.md).
- The domain overview now treats `domain/procgen` as implemented domain scope
  and points asset/product contract readers to the accepted product docs until
  deeper crate guides exist.

## Batch Artifact Preservation

Batch history was preserved and indexed in
[`reports/batches/README.md`](../../batches/README.md). Historical absolute
`prompt_path` values were normalized to repository-relative paths. The current
WR-018 continuation proposal remains explicitly indexed as a proposal rather
than closeout evidence.

## Validation Hardening

`task docs:validate` now fails on:

- design lifecycle status mismatches between folder and frontmatter;
- active designs that claim implementation or completion without an allowed
  lifecycle exception;
- missing crate-doc coverage for workspace members, with intentional grouped
  rows for `domain/ui/*` and `domain/editor/*`;
- canonical domain-map drift for current workspace ownership markers;
- absolute batch prompt paths;
- completed batches missing `batch.md`;
- unindexed stale active-looking batch manifest states.

## Validation

Required validation for this milestone:

- `task docs:validate`
- `task roadmap:validate`
- `task roadmap:check`

Negative validation probes were performed by temporarily introducing:

- an active-design completion claim without `lifecycle_exception`;
- a missing design lifecycle index link;
- an absolute batch `prompt_path`;
- a missing crate-doc coverage row.

Each probe was rejected by `task docs:validate` before the temporary mutation
was removed.

## Remaining Deliberately Deferred Work

- Deeper current-state crate guides for thin domains such as `domain/asset`,
  `domain/product`, `domain/procgen`, `domain/world_ops`, and
  `domain/world_sdf`.
- Further splitting of very large long-horizon design documents when their next
  implementation slice is selected.
- No deletion of historical batch evidence; preservation remains the policy.
