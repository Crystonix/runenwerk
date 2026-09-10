---
title: Editor Tool Suite Registry And Workbench Host Design
description: Implemented bounded design for editor-local typed tool-suite registration, provider-owned routing, capability policy, and Workbench host composition.
status: implemented
owner: editor
layer: domain/app
canonical: true
last_reviewed: 2026-09-10
related_adrs:
  - ../../adr/accepted/0001-use-domain-owned-commands.md
  - ../../adr/accepted/0004-separate-description-from-execution.md
  - ../../adr/accepted/0005-projections-are-derived-state.md
  - ../../adr/accepted/0010-graph-substrate-canvas-boundary.md
  - ../../adr/accepted/0013-app-neutral-ui-composition-clean-cutover.md
  - ../../adr/superseded/0006-editor-surface-provider-plugin-seam.md
  - ../../adr/superseded/0012-capability-workbench-clean-break.md
related_designs:
  - ../active/editor-ui-workspace-tool-surface-architecture.md
  - ../active/material-lab-and-material-preview-design.md
  - ../superseded/runenwerk-capability-workbench-target-architecture.md
related_reports:
  - ../../reports/closeouts/tool-suite-registry-inspector/closeout.md
  - ../../reports/closeouts/wr-037-host-capability-policy/closeout.md
  - ../../reports/closeouts/wr-038-product-and-service-capability-declarations/closeout.md
---

# Editor Tool Suite Registry And Workbench Host Design

## Status

Implemented as a bounded editor/tool-host platform. The typed Tool Suite Registry, provider-family routing, Workbench composition compiler, host capability policy, product/service capability declarations, stable-key authority migration, and the Tool Suite Registry Inspector proof all exist in current code and completed evidence.

This document is not the structural composition authority. [ADR 0013](../../adr/accepted/0013-app-neutral-ui-composition-clean-cutover.md) supersedes the older workspace-centric structural target and makes app-neutral `ui_composition` the long-term structural authority. The contracts recorded here remain implemented editor-local tool-suite, provider, capability, and host integration machinery while the separate workspace/tool-surface migration boundary still exists.

The former broad Capability Workbench target is also superseded. Nothing in this implemented design creates a Runenwerk-wide capability ontology, global registry, or universal execution runtime.

## Implemented Boundary

The durable editor-local shape is:

```text
app/tool suite declarations
  -> typed Tool Suite Registry
  -> provider-family / stable-key resolution
  -> Workbench composition compiler
  -> app-owned RunenwerkWorkbenchHost
  -> provider-owned interaction proposals
  -> existing app/domain command and product paths
```

`domain/editor/editor_shell` owns generic editor tool-host contract vocabulary and validation. Concrete apps and owning domains retain product semantics, source truth, IO, runtime integration, and mutation authority.

## Tool Suite Contracts

Current code provides typed editor-local contracts including:

- `ToolSuiteId`, `SuiteRef`, `ToolSurfaceStableKey`, and `SurfaceRef`;
- `ProviderFamilyId` and provider-family assignments;
- `EditorToolSuite`, `ToolSurfaceDefinition`, and `ToolSuiteRegistry`;
- structural `ToolSurfaceRole`, `ToolSurfaceRoute`, persistence and creation policy;
- target/profile compatibility metadata;
- `ToolSuiteCapabilityDeclaration`, `ProductCapabilityNeed`, and `ToolServiceNeed`;
- typed host capability requirements and fail-closed policy evaluation.

These declarations describe what a tool or host needs. They do not grant semantic validity, product authority, or permission by themselves.

## Workbench Composition

`domain/editor/editor_shell/src/workbench/compiler.rs::compile_workbench_composition` compiles the current editor-local Workbench inputs into validated registries and host composition. It checks suite/profile identity, stable surface references, provider-family assignments, authored layout references, and workspace compatibility before exposing a compiled composition.

Concrete built-in composition and provider installation remains app-owned under `apps/runenwerk_editor`. The editor, Material Lab, UI Designer, headless/constrained, and test/custom compositions consume the same bounded compiler path where currently supported.

The retained `WorkspaceProfile`, `WorkspaceProfileId`, workspace layout, and tool-surface structures are current-code migration inputs. They are not permission to reassert them as the long-term structural model after ADR 0013.

## Stable-Key And Provider Authority

The completed C1-C6D migration established the bounded authority stack that current code still uses:

- `ToolSurfaceStableKey` is the normal tool-surface identity for retained editor Workbench state, mutations, profile defaults, projection, provider requests, provider-family filtering, and provider matching;
- `ToolSurfaceKind` is a labeled legacy/compatibility boundary for old persistence, authored legacy adapters, named wrappers, command compatibility, and tests rather than normal identity authority;
- `PanelKind` remains structural shell/layout grouping only and must not replace stable tool-surface identity;
- provider-family filtering narrows ownership before deterministic provider support/priority resolution;
- stable-key ambiguity and missing ownership fail closed rather than falling back silently.

Generic structural composition is separately governed by ADR 0013 and the accepted app-neutral composition designs.

## Provider-Owned Routing

`GraphCanvasAction` and generic UI graph interaction remain semantic-free. Tool-specific providers map provider-local routes and structural interactions into typed proposals owned by the appropriate app/domain path. `editor_shell` must not grow semantic branches for each material, procgen, gameplay, animation, physics, particle, or future graph tool.

This is the implemented reason to retain provider-owned routing as a bounded editor host contract even though older workspace structural assumptions are being superseded elsewhere.

## Capability And Semantic Authority

Host permission and semantic validity are separate gates:

```text
tool declaration
  -> host capability policy allow/deny
  -> owning domain semantic validation
  -> app/domain formation or command path
```

`HostCapabilityPolicy` cannot grant domain semantic validity. Domain validation cannot grant host permission. Product/service capability declarations are requested needs, not execution authority. Current WR-037 and WR-038 closeouts provide the bounded implementation evidence for these rules.

Service declarations do not select a process protocol, ABI, sandbox, dynamic plugin system, or external component model.

## Tool Suite Registry Inspector Proof

The Tool Suite Registry Inspector is the post-migration proof that a new editor tool surface can be registered and reached through stable-key/provider-family machinery without adding a new `ToolSurfaceKind` variant. Its provider remains read-only; it inspects registry/provider/workspace/persistence state and does not mutate those authorities.

This proof closes the original registry extensibility question. It does not make the registry a universal platform ontology.

## Ownership

`domain/editor/editor_shell` owns:

- typed tool-suite and stable-surface declaration vocabulary;
- provider-family contracts and deterministic resolution vocabulary;
- editor-local Workbench compilation and compatibility validation;
- typed host capability policy/declaration contracts;
- structural route-to-provider handoff.

`apps/runenwerk_editor` and other concrete apps own:

- installed concrete composition and provider implementations;
- project/file IO and persistence paths;
- runtime/render adapters and preview orchestration;
- concrete command execution and product integration.

Owning domains own their source truth, commands, ratification, semantic IR, product meaning, and diagnostics. Render/GPU layers consume formed/prepared products and do not become tool or domain authority.

## Diagrams

Implemented-era PlantUML references move with this design:

- [Tool Suite registration flow](diagrams/editor-tool-suite-registration-flow.puml)
- [Tool Suite ownership](diagrams/editor-tool-suite-ownership.puml)
- [Provider-owned graph routing](diagrams/provider-owned-graph-routing.puml)
- [Stable tool-surface key persistence](diagrams/stable-tool-surface-key-persistence.puml)
- [Workbench host compositions](diagrams/workbench-host-compositions.puml)

They document the implemented editor-local Tool Suite/Workbench boundary. Where older workspace structural terminology conflicts with ADR 0013, ADR 0013 is authoritative.

## Non-Goals

- No dynamic external plugin loading or marketplace architecture.
- No WASM/process/plugin ABI decision.
- No universal Runenwerk Workbench ontology or global mutable registry.
- No movement of material, texture, procgen, animation, gameplay, physics, scene, or other domain semantics into `editor_shell`.
- No restoration of `ToolSurfaceKind` as normal tool identity.
- No claim that retained editor workspace structures supersede app-neutral composition.
- No change to runtime/editor behavior from this lifecycle reconciliation.

## Completion Evidence

Code and tests own current behavior. Key implementation anchors are `domain/editor/editor_shell/src/tool_suite/`, `domain/editor/editor_shell/src/workbench/compiler.rs`, and `apps/runenwerk_editor/src/shell/workbench_host.rs`. Delivery evidence is retained in the completed Workbench/Tool Suite closeouts, especially [Tool Suite Registry Inspector Closeout](../../reports/closeouts/tool-suite-registry-inspector/closeout.md), [WR-037 Host Capability Policy Closeout](../../reports/closeouts/wr-037-host-capability-policy/closeout.md), and [WR-038 Product And Service Capability Declarations Closeout](../../reports/closeouts/wr-038-product-and-service-capability-declarations/closeout.md).
