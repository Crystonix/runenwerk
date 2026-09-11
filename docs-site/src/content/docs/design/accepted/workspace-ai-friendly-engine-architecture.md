---
title: AI-Friendly Engine Architecture
description: Architecture doctrine for explicit, inspectable, testable, AI-safe Runenwerk systems.
status: accepted
owner: workspace
layer: workspace
canonical: true
last_reviewed: 2026-09-11
related_adrs:
  - ../../adr/accepted/0001-use-domain-owned-commands.md
  - ../../adr/accepted/0002-keep-ai-out-of-foundation.md
  - ../../adr/accepted/0003-ratification-is-domain-specific.md
  - ../../adr/accepted/0004-separate-description-from-execution.md
  - ../../adr/accepted/0005-projections-are-derived-state.md
---

# AI-Friendly Engine Architecture

## Purpose

This document defines how the engine should be structured so that AI-assisted development, debugging, tooling, and automation can operate safely without relying on guesswork.

The goal is not to embed an AI agent into the engine core. The goal is to make the engine sufficiently explicit, inspectable, testable, and contract-driven that AI tools can understand and interact with it through the same boundaries used by humans, editor tools, tests, and scripts.

An AI-friendly engine is one where:

- architecture is explicit;
- ownership boundaries are documented;
- state can be inspected through stable views;
- mutation happens through validated commands;
- diagnostics are structured and queryable;
- generated or transformed state is ratified before acceptance;
- schemas describe engine concepts in machine-readable form;
- tests encode invariants and expected behavior.

The engine should become AI-friendly because it is well-architected, not because it is coupled to a specific AI product.

---

## Core Doctrine

AI must interact with the engine through the same contracts as humans, tools, tests, scripts, and editor systems.

There must be no privileged AI mutation path.

There must be no hidden AI-only state access.

There must be no bypass around validation, ratification, diagnostics, or tests.

The correct model is:

```text
AI proposes.
Domains validate.
Ratifiers check.
Diagnostics explain.
Tests protect.
Schemas describe.
Inspection views expose.
Commands mutate.
```

This keeps the engine maintainable even if AI tooling changes, improves, or disappears.

---

## Architectural Position

AI-friendliness is a cross-cutting architectural quality. It should not be implemented as a single `ai` crate at the center of the engine.

The correct design is to strengthen the engine’s existing architectural layers:

```text
foundation
  -> domain crates
    -> engine/runtime
      -> apps/adapters/tools
```

AI-specific integrations, if they exist later, belong near the application/tooling layer.

Reusable primitives that make AI-safe tooling possible belong in foundation only when they are general engine concepts, not AI concepts.

Foundation should own vocabulary such as:

- typed identity;
- diagnostics;
- ratification;
- schema descriptors;
- command descriptors;
- inspection contracts.

Foundation should not own:

- LLM clients;
- prompt logic;
- AI agents;
- editor automation policy;
- product-specific integrations;
- runtime assistant behavior.

---

## Scope

This design covers the architectural affordances needed to make the engine AI-friendly.

In scope:

- crate boundary documentation;
- stable identity;
- diagnostics;
- ratification;
- command-based mutation;
- inspection DTOs;
- schema metadata;
- architecture index files;
- executor and runtime-AI placement guidance;
- generated-code isolation;
- golden tests;
- validation workflows;
- editor and runtime introspection.

---

## Non-Scope

This design does not define:

- an in-engine LLM runtime;
- a chatbot;
- AI gameplay behavior;
- NPC decision-making;
- procedural content generation;
- model hosting;
- embedding/vector search infrastructure;
- prompt engineering strategy;
- AI vendor selection;
- cloud inference architecture.

Those may be added later as application-level or tooling-level integrations, but they are not part of the core engine architecture.

---

## Architectural Invariants

The following invariants must hold across the engine.

### Dependency Direction

Foundation crates must not depend on domain, engine, editor, app, or adapter crates.

Domain crates may depend on foundation crates and carefully selected lower-level domain contract crates.

Domain crates must not depend on engine runtime, app code, UI backends, windowing backends, or AI integrations.

Engine/runtime crates may compose domain crates but must not introduce editor-specific or AI-specific concepts into generic runtime APIs.

Apps, adapters, and tools may depend on higher-level systems but must not define core domain invariants.

### AI Isolation

AI-specific logic must not become part of foundational or domain invariants.

AI tooling may propose changes, commands, patches, or content, but the owning domain must validate those changes.

No domain crate may trust data because it originated from an AI tool.

### Controlled Mutation

Important domain state must not be mutated arbitrarily from outside its owning domain.

Mutation should flow through explicit command APIs, import pipelines, builders, or controlled transaction objects.

### Inspectability Without Internal Leakage

AI-facing inspection must use stable DTOs, snapshots, summaries, or views.

Internal data structures must remain refactorable.

Inspection APIs must expose enough information to reason about state without exposing unrestricted mutable internals.

### Ratification at Boundaries

Projected, imported, generated, migrated, or externally modified state must be ratified by the owning domain before being accepted.

Ratification belongs in multiple domains, not in one global validator.

---

## Repository Documentation Requirements

Repository documentation should expose a small normalized root surface and route detailed ownership directly to canonical long-form owners.

The current root entrypoints are:

```text
README.md
AGENTS.md
ARCHITECTURE.md
TESTING.md
```

### `README.md`

Describes repository purpose, boundary, maturity, decisive capabilities/limitations, and routes readers to the canonical architecture, testing, contribution, security, and licensing owners.

### `AGENTS.md`

Defines the Runenwerk-specific executor contract: where to start, ownership constraints, prohibited operations, canonical validation, and required delivery evidence. Organization-wide governance remains owned by `dornglut/engineering`.

### `ARCHITECTURE.md`

Describes the repository boundary, major layers, dependency direction, repository-family position, and links to canonical long-form architecture.

### `TESTING.md`

Defines focused checks, the canonical repository validation baseline, hosted-CI relationship, and evidence rules.

### Dependency guidance

Canonical Runenwerk-local dependency and integration rules live in [`../../guidelines/dependency-rules.md`](../../guidelines/dependency-rules.md). Cross-repository family membership, repository relationships, and source-transfer policy remain owned by `dornglut/engineering`; do not recreate a root forwarding summary.

### Canonical long-form owners

Use the docs-site owners directly for detailed repository facts:

- [`../../workspace/crate-inventory.md`](../../workspace/crate-inventory.md) — canonical human-readable active workspace inventory; `Cargo.toml` remains executable membership truth.
- [`../../guidelines/architecture.md`](../../guidelines/architecture.md) — current Runenwerk placement and boundary guide.
- [`../../guidelines/dependency-rules.md`](../../guidelines/dependency-rules.md) — canonical Runenwerk-local dependency direction, adapter boundaries, and framework-consumer rules.
- [`../../workspace/glossary.md`](../../workspace/glossary.md) — shared architecture and authoring vocabulary.
- [`../../workspace/ai-agent-boundaries.md`](../../workspace/ai-agent-boundaries.md) — Runenwerk-specific placement rule for runtime AI integrations.

Do not recreate root crate inventories, domain maps, glossaries, or a second AI/executor guide when these owners already exist.

---

## Crate Design Documentation

Each important crate should include a `design.md` or `architecture.md`.

The document should contain:

```text
Purpose
Scope
Non-scope
Ownership rules
Dependency rules
Public API policy
Invariants
Failure modes
Extension points
Testing strategy
```

The most important rule is that every crate must say what it owns and what it must never know about.

A crate-level design document should answer:

- What domain concept does this crate own?
- What concepts are explicitly outside its authority?
- Which crates may depend on it?
- Which crates may it depend on?
- Which types are stable public contracts?
- Which types are internal implementation details?
- What invariants must always hold?
- How are invalid states reported?
- How is the crate tested?
- How should future contributors extend it?

---

## Foundational Crates

AI-friendliness benefits from several reusable foundational crates, but they should be introduced carefully.

Recommended priority:

```text
1. foundation/id
2. foundation/diagnostics
3. foundation/ratification
4. foundation/schema
5. foundation/commands
6. foundation/reflection
```

These should not all be created prematurely. Each should be introduced when there are at least two real consumers and a stable boundary can be described.

---

## `foundation/id`

### Purpose

Provide reusable identity primitives and conventions for strongly typed IDs.

### Responsibilities

- typed IDs;
- generation-aware IDs where appropriate;
- stable ID documentation conventions;
- domain-specific ID wrappers;
- ID formatting/parsing helpers where safe.

### Non-Responsibilities

- global entity registry;
- asset database;
- ECS storage;
- UUID policy for every domain;
- stringly typed identifiers;
- distributed identity system.

### Required ID Documentation

Every important ID type must document:

```text
Who creates it?
Where is it valid?
Is it stable across frames?
Is it stable across sessions?
Is it serializable?
Can users author it?
Can it be reused?
What invalidates it?
```

Example ID categories:

```text
SurfaceDefinitionId
SurfaceInstanceId
SurfaceHostInstanceId
WidgetId
CommandRouteId
SceneEntityId
AssetId
AssetPath
RenderGraphNodeId
RenderPassId
ResourceId
```

Typed IDs reduce accidental misuse and make AI-assisted refactors safer.

---

## `foundation/diagnostics`

### Purpose

Provide shared diagnostic vocabulary for structured, machine-readable, human-readable error reporting.

### Responsibilities

The crate may define:

```text
Diagnostic
DiagnosticCode
Severity
DiagnosticSource
DiagnosticDomain
DiagnosticSubject
DiagnosticLocation
DiagnosticContext
DiagnosticSink
```

Diagnostics should be:

- structured;
- stable;
- queryable;
- domain-tagged;
- human-readable;
- machine-readable.

### Non-Responsibilities

This crate must not become a full observability platform.

It should not own:

- OpenTelemetry setup;
- tracing subscriber configuration;
- log file rotation;
- profiler UI;
- runtime dashboards;
- network telemetry;
- metrics backends;
- editor diagnostic panels.

Those belong at engine/runtime/app level.

### Example Diagnostic Codes

```text
scene.validation.missing_component
asset.import.unsupported_format
ui.surface.invalid_mount
render.graph.unbound_resource
editor.command.rejected
workspace.projection.invalid_route
```

### Diagnostic Invariants

A diagnostic should answer:

```text
What failed?
Where did it fail?
Which domain owns the failure?
Is it recoverable?
Which invariant was violated?
What subject does it refer to?
```

---

## `foundation/ratification`

### Purpose

Provide shared vocabulary for validating generated, projected, imported, migrated, or externally modified state.

### Responsibilities

The crate may define:

```text
Ratifier
RatificationReport
RatificationStatus
RatificationIssue
RatificationContext
RatificationSubject
```

### Non-Responsibilities

It must not contain all domain-specific validation logic.

Foundation may define the interface and report model. Domains own their own rules.

### Domain Ratifiers

The engine should have multiple ratifiers:

```text
SceneRatifier
AssetGraphRatifier
UiSurfaceRatifier
EditorShellRatifier
WorkspaceRatifier
RenderGraphRatifier
AnimationGraphRatifier
```

Ratification should happen at architectural boundaries.

Examples:

```text
imported asset -> asset ratifier
projected workspace -> workspace/editor shell ratifier
generated render graph -> render graph ratifier
AI-generated scene edit -> scene ratifier
migration output -> owning-domain ratifier
```

### Core Rule

AI may propose a state change, but the owning domain decides whether the result is valid.

---

## `foundation/schema`

### Purpose

Provide schema vocabulary for describing commands, components, assets, settings, diagnostics, editor tools, and other structured engine concepts.

### Responsibilities

The crate may define:

```text
SchemaId
SchemaVersion
FieldDescriptor
EnumDescriptor
StructDescriptor
CommandDescriptor
DiagnosticDescriptor
ValueKind
Optionality
DeprecationMetadata
```

### Non-Responsibilities

Foundation schema must not own every engine schema.

It should provide the vocabulary. Each domain crate should publish its own descriptors.

### Schema Consumers

Schema metadata can support:

- AI tool use;
- editor inspectors;
- scripting;
- serialization;
- validation;
- documentation generation;
- test generation;
- command palettes;
- migration tooling.

### Schema Invariants

Schemas must be versioned when they cross persistence, tooling, or app boundaries.

Schemas must distinguish internal implementation details from stable contracts.

Schemas must not imply that arbitrary mutation is allowed.

---

## `foundation/commands`

### Purpose

Provide shared vocabulary for command descriptors, command results, command failure reporting, and command metadata.

### Responsibilities

The crate may define:

```text
CommandId
CommandDescriptor
CommandInputSchema
CommandResult
CommandFailure
CommandEffect
CommandPrecondition
CommandUndoPolicy
```

### Non-Responsibilities

This crate must not define one universal command enum for the entire engine.

Each domain owns its own command model.

Recommended domain command families:

```text
SceneCommand
WorkspaceCommand
SurfaceCommand
AssetCommand
RenderGraphCommand
AnimationCommand
EditorCommand
```

### Command Metadata

Each command should be describable by:

```text
Command name
Domain owner
Input schema
Output schema
Validation rules
Authorization or capability requirements
Undo/redo behavior
Diagnostics emitted
Expected result
Failure cases
Side effects
```

### Mutation Rule

Important state changes should flow through domain commands, not through arbitrary external field mutation.

---

## `foundation/reflection`

### Purpose

Provide reusable reflection/introspection vocabulary after schema and commands are stable.

### Responsibilities

Reflection may support:

- type metadata;
- field metadata;
- read-only value inspection;
- editor inspectors;
- serialization helpers;
- debug views;
- scripting bridges.

### Non-Responsibilities

Reflection must not become a global mutable object graph API.

It must not allow uncontrolled mutation of domain internals.

It must not replace explicit domain commands.

### Design Constraint

Reflection should be introduced later than IDs, diagnostics, ratification, schema, and commands.

Reflection is useful only after the underlying contracts are clear.

---

## Command-Based Mutation

AI-friendly systems should avoid arbitrary mutation.

Instead of exposing direct mutation like:

```text
workspace.panels.push(panel)
workspace.selected = entity
scene.entities.remove(id)
```

The engine should prefer explicit domain commands:

```text
WorkspaceCommand::MountSurface
WorkspaceCommand::SelectSurface
SceneCommand::DeleteEntity
SceneCommand::AttachComponent
AssetCommand::ImportAsset
RenderGraphCommand::AddPass
```

Each command should be validated by the owning domain.

The command boundary enables:

- undo/redo;
- scripting;
- editor automation;
- replay;
- testing;
- diagnostics;
- permissions/capabilities;
- multiplayer/editor collaboration;
- AI-assisted changes.

There should not be one global command enum. Commands should remain domain-owned.

---

## Inspection Views

AI tools need to inspect current state without depending on internal structures.

Each important domain should provide read-only inspection DTOs.

Recommended inspection surfaces:

```text
SceneInspection
EntityInspection
ComponentInspection
WorkspaceInspection
SurfaceInspection
AssetGraphInspection
RenderGraphInspection
DiagnosticInspection
CommandInspection
```

Inspection views should be:

- stable enough for tools;
- read-only;
- serializable where useful;
- detached from internal storage layout;
- documented as contracts;
- versioned when persisted or exposed externally.

Example inspection concepts:

```text
List worlds
List entities
Inspect entity components
List assets
Inspect asset dependencies
List editor surfaces
Inspect layout tree
Inspect render graph
Inspect diagnostics
Inspect command history
Inspect active routes
```

The same introspection model should serve:

- human developers;
- tests;
- debug overlays;
- editor tools;
- scripts;
- AI assistants.

There should not be a separate AI-only inspection channel unless absolutely necessary.

---

## Editor and Runtime Introspection

The editor should eventually expose structured developer commands for inspecting system state.

Examples:

```text
/editor/surfaces/list
/editor/surfaces/inspect <id>
/editor/workspace/project
/editor/commands/list
/editor/commands/describe <command>
/scene/entities/list
/scene/entity/inspect <id>
/assets/graph/inspect
/render/graph/inspect
/diagnostics/list
```

These commands should use the same DTOs and command descriptors used by tests and tools.

Introspection should be safe by default.

Read-only inspection should not mutate state.

Mutating debug commands should still go through domain commands or controlled transactions.

---

## Projection and Golden Tests

Projection-heavy systems need golden tests.

For editor architecture, examples include:

```text
WorkspaceState
  -> EditorShellProjection
  -> UiSurfaceMountPlan
  -> InteractionRoutes
```

Golden tests should verify:

- the same input state produces the same projection;
- invalid input produces known diagnostics;
- command route metadata remains stable;
- layout projection does not leak backend details;
- structural context survives projection;
- generated mount plans are ratified;
- UI-facing DTOs remain compatible.

Golden tests are especially valuable for AI-assisted refactoring because they protect subtle structural behavior.

---

## Generated Code Policy

Generated code should be isolated from hand-authored domain logic.

Acceptable structures:

```text
crates/foo/src/
  lib.rs
  model.rs
  commands.rs
  ratify.rs
  generated/
    schema.rs
    reflection.rs
    command_descriptors.rs
```

Or:

```text
crates/foo_codegen/
crates/foo_macros/
```

Rules:

- generated code is never edited manually;
- generated code must be reproducible;
- generated code must be reviewable through diffs;
- domain invariants remain hand-authored;
- generated code must not bypass validation;
- generated code must not define core architecture policy.

AI-generated code should follow the same discipline as other generated or assisted code.

---

## Patch-Oriented AI Editing

AI tools should not be encouraged to rewrite large areas blindly.

The preferred model is explicit, bounded patches:

```text
Create file
Delete file
Replace method
Add enum variant
Move type
Rename module
Add test
Update Cargo dependency
Update design document
```

Patch proposals should include:

```text
Changed files
Reason
Affected domains
Expected invariant impact
Tests to run
Rollback consideration
Documentation impact
```

This makes AI-assisted refactors safer and easier to review.

---

## Design-Before-Implementation Workflow

Large architectural changes should begin with a design note.

Examples:

```text
docs-site/src/content/docs/design/editor-surface-routing.md
docs-site/src/content/docs/design/accepted/foundation-diagnostics-design.md
docs-site/src/content/docs/design/scene-identity.md
docs-site/src/content/docs/design/asset-graph.md
docs-site/src/content/docs/design/render-graph-ratification.md
```

Each design note should include:

```text
Problem
Current state
Constraints
Alternatives considered
Decision
Consequences
Migration plan
Validation plan
```

This allows implementation to be checked against an explicit decision instead of relying on implicit intent.

---

## Testing Strategy

Tests should describe behavior, not implementation accidents.

Good test names:

```text
mounting_unknown_surface_definition_is_rejected
workspace_projection_preserves_selected_surface
scene_ratifier_reports_missing_required_transform
render_graph_rejects_unbound_storage_resource
command_rejection_emits_diagnostic
```

Poor test names:

```text
test_1
works
surface_test
foo_case
```

Each important invariant should have at least one test.

Testing tiers should include:

```text
unit tests
domain invariant tests
ratification tests
projection golden tests
command behavior tests
serialization/schema compatibility tests
editor smoke tests
runtime smoke tests
```

Tests are not just correctness checks. They are executable documentation for humans and AI tools.

---

## Executor and Runtime-AI Guidance Requirements

Repository executor guidance lives in root `AGENTS.md`. Runenwerk-specific runtime-AI placement guidance lives in [`../../workspace/ai-agent-boundaries.md`](../../workspace/ai-agent-boundaries.md). Do not create a second `AI_GUIDE.md` authority that duplicates either surface.

The executor contract should route assistants to:

```text
semantic owner first
accepted architecture/design authority
owning issue when work is accepted
repository dependency boundaries
focused checks and canonical validation
exact-head delivery evidence
```

Runtime-AI placement guidance should preserve these rules:

```text
AI integrations belong in apps, tools, or adapters.
Foundation and pure domain crates do not own LLM clients, prompts, or autonomous agents.
AI tools use normal command, ratification, diagnostic, schema, and inspection boundaries.
No AI-only mutation or privileged inspection path is introduced.
```

Example repository rules remain:

```text
Do not add engine dependencies to foundation crates.

Do not place editor-only concepts in runtime crates.

Do not bypass command validation by mutating domain state directly.

Do not introduce stringly typed IDs where typed IDs exist.

Do not add AI-specific concepts to domain invariants.

When changing projection logic, update golden tests.

When adding a new command, document its schema, diagnostics, and failure cases.

When adding a new diagnostic, assign a stable diagnostic code.
```

---

## Recommended Roadmap

### Phase 1: Make the Repository Understandable

Establish or improve the normalized entrypoints and their canonical long-form owners:

```text
README.md
AGENTS.md
ARCHITECTURE.md
TESTING.md
docs-site/src/content/docs/workspace/crate-inventory.md
docs-site/src/content/docs/guidelines/architecture.md
docs-site/src/content/docs/guidelines/dependency-rules.md
docs-site/src/content/docs/workspace/glossary.md
docs-site/src/content/docs/workspace/ai-agent-boundaries.md
```

Document:

```text
crate ownership
allowed dependencies
public API rules
where new concepts belong
how to validate changes
```

Keep root summaries concise and route detailed facts directly to their canonical owner rather than adding forwarding inventories or duplicate AI guidance.

This phase should happen before adding AI-specific features.

---

### Phase 2: Stabilize Identity and Diagnostics

Introduce or formalize:

```text
foundation/id
foundation/diagnostics
```

Use typed IDs consistently in:

```text
scene
assets
ui surface
editor shell
render graph
workspace
commands
```

Use structured diagnostics in:

```text
validation
ratification
command rejection
asset import
render graph assembly
surface mounting
workspace projection
```

---

### Phase 3: Add Ratification

Introduce:

```text
foundation/ratification
```

Then add domain ratifiers:

```text
SceneRatifier
UiSurfaceRatifier
EditorShellRatifier
WorkspaceRatifier
AssetGraphRatifier
RenderGraphRatifier
```

Ratification should become the standard way to accept generated, imported, projected, or migrated state.

---

### Phase 4: Normalize Command Boundaries

Introduce domain-owned command APIs:

```text
SceneCommand
WorkspaceCommand
SurfaceCommand
AssetCommand
RenderGraphCommand
EditorCommand
```

Each command should document:

```text
input schema
validation rules
diagnostics
side effects
undo/redo policy
failure cases
```

Mutation should increasingly move behind command boundaries.

---

### Phase 5: Add Inspection DTOs

Add read-only inspection views for important domains:

```text
SceneInspection
EntityInspection
WorkspaceInspection
SurfaceInspection
AssetGraphInspection
RenderGraphInspection
DiagnosticInspection
CommandInspection
```

These should power:

```text
debug overlays
developer console
tests
editor tools
AI tools
```

---

### Phase 6: Add Schema Metadata

Introduce:

```text
foundation/schema
```

Then publish domain schema descriptors for:

```text
commands
components
assets
editor tools
settings
diagnostics
surface definitions
render graph nodes
```

Schema metadata should support editor tooling, scripting, validation, documentation, and AI-assisted workflows.

---

### Phase 7: Add Reflection Carefully

Introduce reflection only after command, schema, diagnostic, and ratification boundaries are stable.

Reflection should support inspection and tooling.

It should not become an unrestricted mutation system.

---

### Phase 8: Build AI Tooling on Top

Only after the previous phases should higher-level AI tooling be added.

Possible later features:

```text
natural-language command palette
AI-assisted editor commands
automated refactor proposals
debug explanation tools
asset generation pipelines
test generation helpers
documentation generation helpers
```

These should remain app/tooling integrations.

They should not change core domain ownership.

---

## Things to Avoid

Avoid:

```text
AI agent runtime inside core engine crates
LLM calls from foundation or domain crates
AI-specific abstractions in foundation
giant global reflection system
one universal command enum
one universal diagnostic enum
untyped string APIs
direct editor mutation of domain internals
macro-heavy magic in core crates
tool-specific hacks for ChatGPT, Copilot, Codex, or any one vendor
```

The engine should be compatible with AI-assisted development without being architecturally dependent on any AI system.

---

## Final Design Direction

The engine should become AI-friendly by strengthening its architecture around these pillars:

```text
1. Explicit crate doctrine
2. Typed IDs
3. Structured diagnostics
4. Domain ratification
5. Command-based mutation
6. Read-only inspection DTOs
7. Schema metadata
8. Golden tests
9. Architecture documentation
10. AI contributor guidance
```

The most important architectural rule is:

```text
Do not make the engine depend on AI.
Make the engine understandable and controllable enough that AI can safely work with it.
```
