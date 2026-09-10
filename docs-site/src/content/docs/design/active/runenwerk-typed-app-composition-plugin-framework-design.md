---
title: Runenwerk Typed App Composition Plugin Framework Design
description: Architecture direction for typed app recipes, plugin suites, extension points, host profiles, and domain-owned contribution registries.
status: active
owner: workspace
layer: architecture
canonical: false
last_reviewed: 2026-08-04
related_docs:
  - ./runenwerk-domain-workbench-north-star.md
  - ./ui-program-architecture.md
  - ./ui-program-architecture-owner-map.md
  - ./ui-component-platform-base-control-packages-design.md
  - ./ui-component-platform-ownership-realignment-design.md
  - ./runenwerk-typed-app-composition-plugin-framework-implementation-roadmap.md
  - ../../guidelines/runenwerk-architecture.md
  - ../../guidelines/domain-program-architecture-pattern.md
  - ../../guidelines/programming-principles.md
  - ../../domain/ui/architecture.md
---

# Runenwerk Typed App Composition Plugin Framework Design

## Status

Documentation status: `active`.

Decision status: proposed architecture direction.

Target decision: accept, revise, or reject the direction through its owning issue and review.

This document is a proposed architecture direction. It is not an active implementation plan, not a production track, and not approval to create a broad shared framework.

Architecture acceptance is not implementation authorization.

## Decision summary

Runenwerk should compose products from typed app recipes and plugin suites. Plugins contribute capabilities through typed extension points. Domains own contribution meaning, validation, descriptors, catalogs, compilers, evaluators, migrations, fixtures, and diagnostics. The shared composition layer owns structure: plugin manifests, plugin graph ordering, extension routing, capability matching, host compatibility, freeze points, and assembly reports.

The design should be accepted only as architecture direction. Implementation remains separately authorized through existing owner tracks, starting with the UI Component Platform Phase 11 proof and then one non-UI proof such as MaterialNodes or ProcgenNodes before shared extraction.

## Not authorized by this design

This design does not authorize:

```text
active implementation
new production track
foundation/meta
generic plugin framework extraction
universal Registry<T> public API
universal graph runtime
universal evaluator/compiler
ECS ownership of static domain semantics
renderer ownership of product truth
runtime mutation of static contribution registries
SurfaceSlot as the primary structural composition vocabulary
```

## Problem

Runenwerk is growing beyond one executable, one editor shell, and one UI component track. The platform needs a way to assemble products such as the full editor, material lab, procgen lab, UI designer, runtime preview, headless proof runner, game runtime, and world-space authoring host from reusable capabilities.

Current architecture already points in this direction:

- `UiProgram` is the first domain-program proving track.
- UI controls are becoming descriptor-backed, story-proven, catalog-visible packages.
- Material, procgen, asset import, animation, behavior, gameplay, ability, tool, inventory, and quest systems are future domain-program candidates.
- `ui_composition` owns app-neutral structural composition checkpoints.
- `ui_surface` owns semantic surface and mount compatibility vocabulary.
- ECS already gives good runtime-system registration ergonomics.

The missing piece is a typed composition layer that assembles products from capabilities without collapsing domain ownership into one generic registry, one ECS universe, one renderer truth, or one universal node graph.

## Design laws

```text
Recipes compose products.
Plugins compose capabilities.
Sub-plugins compose suites.
Extension points keep domains bounded.
Contributions declare domain intent.
Domain registries validate and lower contributions.
Descriptors and catalogs expose frozen read models.
Host profiles decide where outputs can run.
Runtime plans wire executable systems.
ECS runs live behavior.
Hosts perform effects.
Reports make the assembly inspectable.
```

## Vocabulary

### AppRecipe

An `AppRecipe` names product assembly intent. A recipe selects a product profile, host target, root plugin suite, optional proof sets, and validation expectations.

Examples:

```text
runenwerk.editor.full
runenwerk.editor.material_lab
runenwerk.editor.procgen_lab
runenwerk.headless.ui_proofs
runenwerk.runtime.game
runenwerk.world_space.authoring
```

### ProductProfile

A `ProductProfile` is a curated product shape, such as `FullEditor`, `MaterialLab`, `ProcgenLab`, `UiDesigner`, `RuntimePreview`, `HeadlessProofRunner`, `GameRuntime`, or `WorldSpaceAuthoring`.

Prefer product profiles over scattered feature booleans.

### Plugin and PluginSuite

A `Plugin` is a composition unit. It contributes capabilities through typed extension points and may contain sub-plugins. A plugin is not itself the durable descriptor format for a domain feature.

A `PluginSuite` is a plugin whose main job is to compose child plugins.

Example:

```text
EditorSuitePlugin
  BaseUiPlugin
  EditorHostPlugin
  EditorSurfacePlugin
  EditorWorkspacePlugin
  MaterialGraphEditorPlugin
  ProcgenEditorPlugin
  AssetBrowserPlugin
```

### ExtensionPoint

An `ExtensionPoint` is a typed contribution channel owned by a domain or platform boundary.

Examples:

```text
UiControls
UiThemes
UiHosts
UiCompositionDefinitions
UiSurfaces
WorkspaceProfiles
MaterialNodes
ProcgenNodes
AbilityNodes
BehaviorNodes
AnimationNodes
AssetKinds
AssetImporters
ArtifactProducers
EditorTools
EditorCommands
EcsSchedules
RenderBackends
InputDevices
ProofSets
```

Extension points map to ownership boundaries, not to tiny feature details.

### Contribution

A `Contribution` is a domain-specific declaration supplied through an extension point.

Examples:

```text
Button
TextureSample2d
HeightNoise
DamageEffect
ConeTarget
ForeignMeshImporter
EditorHostAdapter
MaterialGraphSurface
UiUpdateSchedule
```

### Registry, descriptor, catalog

A domain `Registry` is the write-side collection, validation, and lowering owner for one extension point.

A `Descriptor` is normalized inert metadata. It is inspectable and may be serialized or snapshotted where appropriate. It does not perform runtime behavior.

A `Catalog` is the read-only query/inspection view over validated descriptors.

### HostProfile and HostAdapter

A `HostProfile` declares what a host can support. Examples include `EditorHostProfile`, `GameHostProfile`, `WorldSpaceHostProfile`, `HeadlessHostProfile`, `CliHostProfile`, and `RemotePreviewHostProfile`.

A `HostAdapter` bridges facts and effects between Runenwerk and a concrete environment.

### Capability

A `Capability` is a declared ability or requirement. It is not permission to mutate arbitrary global state.

Capabilities exist to support validation, compatibility, trust policy, and reports.

## Corrected UI composition vocabulary

Use current `ui_composition` vocabulary for structural composition:

```text
PresentationTarget
CompositionRoot
Region
MountedUnit
MountedContentRef
```

Use current `ui_surface` vocabulary for semantic surface and mount compatibility:

```text
SurfaceDefinition
SurfaceDefinitionRegistry
SurfaceInstanceId
SurfaceHostInstanceId
MountedSurfaceInstance
MountedSurfaceRegistry
WorldSpacePromptMount
SurfaceCapability
SurfaceCapabilitySet
```

Avoid `SurfaceSlot` as the primary north-star term unless a later accepted design reintroduces it with an owner and migration path.

## Ownership map

### Foundation

Foundation may own only low-level reusable vocabulary after extraction gates are satisfied.

Candidate future primitives after proof:

```text
PluginId
PluginVersion
ExtensionPointId
PluginDiagnosticId
```

Foundation must not own domain contribution semantics, command execution, editor policy, ECS mutation, renderer product truth, global mutable registries, generic graph interpretation, or domain validation rules.

### Engine / runtime

The engine/runtime layer may own runtime composition and plugin integration:

```text
plugin graph execution
app recipe expansion
runtime plan freeze
ECS schedule registration
host adapter activation
render backend activation
runtime resource ownership
app lifecycle
```

It must not own UI, material, procgen, ability, behavior, animation, asset, editor, or gameplay semantics.

### Domain crates

Domains own their contribution types, descriptors, registries, catalogs, validators, compilers, evaluators, migration rules, fixtures, and diagnostics.

Examples:

```text
ui_controls owns ControlContribution and ControlCatalog.
material_graph owns MaterialNodeContribution and MaterialNodeCatalog.
procgen owns ProcgenNodeContribution and ProcgenNodeCatalog.
asset owns asset kind/importer/artifact producer contracts.
ability owns ability graph semantics once accepted.
```

### Apps and adapters

Apps and adapters compose products and connect to external systems. They may select recipes, configure hosts, load project content, and start processes. They must not define durable domain invariants.

## Core flow

```text
AppRecipe
  -> PluginGraph
  -> typed extension registration
  -> domain validation
  -> descriptor/catalog lowering
  -> host compatibility resolution
  -> proof/report generation
  -> runtime plan freeze
  -> ECS/runtime startup
  -> host effects
```

No static registry mutation is allowed after freeze.

## Public API target

The public authoring API should feel broad but typed:

```rust
app.extend::<UiControls>().add::<Button>();
app.extend::<MaterialNodes>().add::<TextureSample2d>();
app.extend::<ProcgenNodes>().add::<HeightNoise>();
app.extend::<AbilityNodes>().add::<DamageEffect>();
app.extend::<UiHosts>().add::<EditorHostAdapter>();
app.extend::<EcsSchedules>().systems::<UiUpdate>((
    resolve_ui_bindings,
    process_ui_interactions,
    emit_ui_frame,
));
```

Do not expose app composition as a generic service locator:

```rust
app.registry().add::<Button>();
app.registry().add::<TextureSample2d>();
app.registry().add::<DamageEffect>();
```

The internals may route through a shared extension map, but public APIs should name the extension point.

## UI control proving slice

Phase 11 should use this design as a reference for a UI-local proving slice, not as a reason to create a shared framework immediately.

Target shape for base controls:

```text
BaseControlsPlugin
UiControls extension point or local equivalent
ControlContribution
ControlDef builder
control presets
field groups
theme groups
ControlCompiler / lowering layer
ControlCatalog projection
ControlInspection projection
```

Base controls covered:

```text
Label
Button
InspectorField
ColorPicker
ActionPrompt
ListView
TreeView
TableView
```

Phase 11 must not take over full runtime interaction. That remains Phase 12.

## Non-UI second-domain proof

Shared extraction is blocked until one non-UI domain proves the same structure.

Preferred candidates:

```text
MaterialNodes
ProcgenNodes
AssetImporters
```

Material nodes are the preferred first non-UI proof because material graphs already have descriptor/catalog pressure and clear node contribution semantics.

## Reports

The framework direction should eventually produce stable reports:

```text
PluginGraphReport
PluginBuildReport
ExtensionPointReport
ContributionValidationReport
DescriptorLoweringReport
HostCompatibilityMatrix
AppAssemblyReport
ProofManifestReport
MigrationReport
RuntimePlanReport
```

Reports should support humans, CI, and AI agents. Generated reports are evidence or mirrors unless an accepted design explicitly promotes one to a narrow machine contract.

## Risks and safeguards

### Over-generalization

Risk:

```text
foundation/meta
universal registry
universal graph runtime
universal evaluator
universal compiler
```

Safeguard: keep shared structure limited to plugin graph, lifecycle, manifests, extension routing, dependency ordering, capability negotiation, and reports until UI plus one non-UI proof justify extraction.

### Extension point explosion

Risk: extension points become tiny feature details.

Safeguard: extension points map to ownership boundaries.

### Host compatibility explosion

Risk: every contribution manually lists every host.

Safeguard: derive compatibility from requirements and capabilities.

### Hidden preset inheritance

Risk: presets become opaque inheritance chains.

Safeguard: every preset expansion appears in reports and can be overridden or rejected explicitly.

## Implementation authorization

No implementation is authorized by this design alone.

Implementation requires an owning GitHub issue and pull request under the repository executor contract in [`AGENTS.md`](../../../../../../AGENTS.md) and must record:

```text
accepted architecture direction
exact owner crates/files
scope and non-goals
validation envelope
stop conditions
evidence expectation
```

## Acceptance criteria

This design can be accepted as direction if:

```text
- dependency direction remains foundation -> domain -> engine/runtime -> apps/adapters/tools;
- domain meaning remains in owning domains;
- the shared framework owns structure, not semantics;
- extension points are typed and owner-aligned;
- host compatibility is requirements/capabilities based;
- current ui_composition vocabulary is used for structural composition;
- ui_surface remains semantic surface and mount compatibility vocabulary;
- ECS stays runtime fabric;
- renderer stays expression/backend execution;
- reports are scriptless-workflow friendly;
- shared extraction waits for at least two proving domains and an accepted extraction design.
```

## Continuation rule

If accepted, this document records accepted architecture direction, not a production track or implementation authorization.

Immediate continuation:

```text
1. Use this design to review/refactor Phase 11 base control package work.
2. Keep implementation inside UI owner crates.
3. Prove contribution/preset/lowering ergonomics locally in UI controls.
4. Do not extract shared plugin primitives yet.
5. After UI proof, prove MaterialNodes or ProcgenNodes.
6. Only then consider controlled shared extraction.
```
