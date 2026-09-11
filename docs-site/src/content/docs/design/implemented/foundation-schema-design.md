---
title: Foundation Schema
description: Design direction for the Runenwerk foundation schema vocabulary crate.
status: implemented
owner: foundation
layer: foundation
canonical: true
last_reviewed: 2026-05-16
related_adrs: []
---

# Foundation Schema

## Review note

This design was revised through bounded review passes that focused on repository evidence, foundation-layer boundaries, dependency direction, diagnostics and ratification separation, command descriptor preparation, and migration realism.

The main review pressure was to keep `foundation/schema` as portable description vocabulary only. Earlier drafts drifted too close to editor inspector behavior, ECS reflection, generic validation, and future command execution. The accepted design keeps those concerns out of the crate. Phase 1 through Phase 4 now implement the foundation-only vocabulary, optional diagnostics projection, one narrow editor-inspector interoperability consumer, and one domain-owned descriptor publication while leaving command vocabulary design for Phase 5.

## Purpose

`foundation/schema` defines Runenwerk's shared vocabulary for describing typed data shapes.

It exists so domains, editor tools, tests, scripts, future command descriptors, persistence workflows, and AI-assisted proposal tooling can talk about the shape of values without taking ownership of domain validity, runtime behavior, editor policy, reflection, mutation, or serialization backends.

The crate answers questions such as:

```text
What is this value's shape?
What fields does this shape expose?
What path identifies this nested field?
What primitive or composite value kind is expected here?
What descriptive constraints should tools display before a domain ratifier decides acceptance?
What stable schema identity/version describes this contract?
```

It does not answer:

```text
Is this game object valid?
Should this command execute?
Can this candidate be accepted?
How should this editor widget render?
How should a reflected Rust value be mutated?
How should an LLM produce a proposal?
```

The doctrine remains:

```text
diagnostics = explain issues
ratification = accept/reject candidates
schema = describe typed shapes/fields/paths/values/constraints
commands = describe requestable mutations/proposals/outcomes
```

## Critical premise check: should this crate exist now?

Yes, `foundation/schema` is justified and now exists as a foundation vocabulary crate.

The current repository has enough converging pressure to justify a shared schema vocabulary:

1. The architecture doctrine already names schemas as part of the core flow: AI proposes, domains validate, ratifiers check, diagnostics explain, tests protect, schemas describe, inspection views expose, and commands mutate. It also requires important mutations to go through explicit command, builder, import, or transaction boundaries and requires generated, imported, migrated, or AI-proposed state to be ratified before acceptance.
2. The foundation vocabulary plan explicitly lists `foundation/schema` and `foundation/commands` as the next vocabulary crates, and says schema should describe reusable shapes and value contracts without becoming a serialization framework, ECS reflection system, editor inspector engine, or runtime object model.
3. Current workspace crates already include repeated schema-like needs: `editor_inspector` has inspector paths, values, field nodes, sections, validation messages, and reflective field editing; `editor_scene` command intents already carry path/value pairs for field edits; ECS reflection already exposes type and field metadata.
4. AI tooling must not receive privileged mutation paths. A shared schema vocabulary helps expose shapes to tools while preserving the normal command, ratification, and diagnostic contracts.

The crate should not wait until every domain has mature schema needs. Waiting would allow editor inspector paths, command parameter descriptions, persistence validation metadata, and future AI proposal contracts to drift into incompatible local vocabularies. The first implementation should still be narrow: define vocabulary only, then adopt it incrementally in one or two existing call sites.

The crate was implemented only after the design was accepted and the diagnostics/ratification gate remained green. Current implementation status:

```text
Phase 0: complete - accepted design
Phase 1: complete - crate skeleton and core vocabulary
Phase 2: complete - optional diagnostics bridge for schema-definition issues
Phase 3: complete - editor_inspector path/value interoperability helpers
Phase 4: complete - scene LocalTransform descriptor publication
Phase 5: complete - foundation/commands design accepted and Phase 1 vocabulary implemented separately
```

The separate `foundation/commands` crate now owns command descriptor/proposal vocabulary. Schema still must not add command execution, global registries, broad schema catalogs, editor inspector rewrites, reflection-driven mutation, schema macros, or generic `SchemaValue`-against-`SchemaShape` validation.

## Current repo evidence

### Repository doctrine

Runenwerk is organized by domain. `AGENTS.md` says code is organized by domain, each domain owns its models/services/logic, and changes must respect boundaries. It also places foundation as cross-domain/runtime-neutral shared contracts, domain as engine-agnostic reusable logic, and engine as runtime glue.

`ARCHITECTURE.md` defines the dependency direction:

```text
foundation -> domain crates -> engine/runtime -> apps/adapters/tools
```

It also requires separation between description/model and execution/runtime layers. Descriptions should be serializable, inspectable, ratifiable, diffable, testable, and suitable for AI-assisted editing; execution objects may be optimized, backend-aware, resource-owning, and non-serializable.

[`../../guidelines/dependency-rules.md`](../../guidelines/dependency-rules.md) says foundation may depend only on justified foundation crates and low-level external libraries, and must not depend on domain, runtime, editor, app, adapter, AI integration, UI framework, or backend crates. It also says boundary pressure should first ask whether a DTO, command, ratifier, contract crate, or test-support crate is missing.

### Current foundation crates

The active workspace includes:

```text
foundation/id
foundation/id_macros
foundation/diagnostics
foundation/ratification
foundation/schema
```

`foundation/diagnostics` is already a `no_std` capable vocabulary crate with `alloc` and optional `serde`. Its public surface includes diagnostic codes, domains, locations, messages, metadata, reports, severity, sink, and subjects.

`foundation/ratification` is also `no_std` capable, uses `alloc` for report-bearing types, has optional `diagnostics` and optional `serde`, and explicitly does not own concrete domain validation rules, command execution, editor history, undo/redo, reconciliation, runtime policy, or AI behavior.

`foundation/schema` is a `no_std` capable vocabulary crate with `alloc`, optional `serde`, and optional `diagnostics`. It currently owns schema IDs, versions, paths, values, shapes, fields, constraints, descriptors, compatibility metadata, schema issues, deterministic metadata/value ordering, and an optional diagnostics bridge for schema-definition issues.

`editor_inspector` now depends on `foundation/schema` for one narrow consumer: explicit path/value interoperability helpers. The helpers project `InspectorPath` and supported inspector values into schema vocabulary, project supported schema paths back into inspector paths, and reject unsupported schema path segments clearly. They do not replace inspector editing, publish descriptors, validate values against shapes, mutate ECS state, or use reflection.

`scene` now depends on `foundation/schema` for one domain-owned descriptor publication: `scene.local_transform` version `1`. The descriptor is explicit and deterministic, describes `LocalTransform` as translation, rotation, and scale object fields, and does not register itself globally, validate runtime values, mutate state, or call ECS reflection.

### Existing schema-like code

`editor_inspector` currently owns inspector-specific models:

```text
InspectorPath
InspectorPathSegment
InspectorValue
InspectorField
InspectorSection
ValidationMessage
InspectorEditValue
```

These are editor-inspector concepts, not foundation concepts. They prove that shape/path/value vocabulary exists today, but they should not be lifted wholesale into foundation because they include inspector presentation, validation messages, and reflective mutation concerns.

`editor_inspector` also contains path-based reflective mutation for ECS-backed inspector targets. That is an editor/domain integration concern and must not move into foundation. Foundation schema may provide a path vocabulary that editor inspector can map to, but not the reflective mutation engine.

ECS reflection exposes field metadata and accessors. That is reflection behavior, not schema vocabulary. `foundation/schema` must not become the ECS reflection system.

`editor_scene` command intents already include component/resource field edits with `InspectorPath` and `InspectorEditValue`. Future command descriptors need a domain-neutral schema vocabulary for parameter and value shapes, but executable commands and concrete command enums remain owned by their domains.

### Existing command-like code

`editor_core` owns current command contracts for deterministic mutation pipelines: command metadata, command outcomes, command context, and the `Command` trait with `apply` and `undo`. This is not a foundation command vocabulary yet; it is editor governance and execution-facing.

`editor_scene` owns concrete executable scene commands and scene command intents. `foundation/schema` must not absorb these. It should only supply parameter/value/descriptor vocabulary that future command descriptors may reference.

### Docs/code disagreement or drift

Current code implements `foundation/schema` Phase 1, Phase 2, Phase 3, and Phase 4. The crate is present at `foundation/schema`, the first consumer is present in `domain/editor/editor_inspector`, and the first domain-owned descriptor set is present in `domain/scene`.

The docs and code agree on the broad direction:

```text
foundation/schema exists
foundation/schema describes shapes
domains own concrete schemas and invariants
commands mutate
AI has no privileged mutation path
```

The remaining practical caveat is that some existing schema-like behavior still lives in `editor_inspector` and ECS reflection today. That is current behavior and must be respected during later migration. The schema crate must continue to interoperate incrementally instead of absorbing inspector presentation, reflective mutation, or ECS reflection behavior.

## Scope

`foundation/schema` owns shared vocabulary for:

- schema identity;
- schema version and compatibility metadata;
- schema descriptors;
- type/shape descriptors;
- field descriptors;
- path descriptors;
- portable value descriptors;
- descriptive schema facts and tooling hints;
- schema definition well-formedness issues;
- optional conversion of schema-definition issues into diagnostics;
- optional serialization of schema descriptors, paths, values, and constraints.

The crate may define vocabulary for these categories:

```text
SchemaId
SchemaVersion
SchemaDescriptor
SchemaKind / SchemaShape
SchemaField
SchemaFieldSet
SchemaPath
SchemaPathSegment
SchemaValue
SchemaConstraint
SchemaCompatibility
SchemaMetadata
SchemaIssue
SchemaIssueCode
SchemaIssueSubject
```

The crate may define lightweight helpers for schema self-invariants, such as:

```text
schema id is non-empty
schema id uses stable identifier syntax
field name is non-empty
object field names are unique inside one object shape
path segments are structurally valid
constraint descriptor is internally well-formed
schema version starts at 1
```

These helpers validate the schema description itself, not domain data.

Phase 1 must not implement generic validation of `SchemaValue` against `SchemaShape` beyond schema-definition self-invariants. That includes rejecting an arbitrary candidate value because it does not match a descriptor. Generic structural value checking can be reconsidered later only if its API and naming cannot be mistaken for domain ratification or command acceptance.

## Non-scope

`foundation/schema` must not own:

```text
runtime execution
command execution
command routing
editor policy
inspector widget selection
ECS reflection
global type registries
global schema registries
LLM clients
prompts
AI agents
domain-specific validation
domain-specific schema catalogs
domain-specific diagnostic codes
domain-specific ratification issue codes
persistence backends
database adapters
network protocols
asset importers
scene mutation
undo/redo
history retention
reconciliation
reflection-based mutation
serialization framework behavior
```

It does not promise compatibility with external schema or reflection systems:

```text
JSON Schema
OpenAPI
TypeScript schemas
serde data models
Rust reflection metadata
```

Adapters may bridge to or from those systems later when a domain or tool has a concrete need. Compatibility with them is not a `foundation/schema` invariant.

It must not introduce:

```text
EngineObject
AnyDomainState
GlobalRegistry
GlobalSchemaRegistry
UniversalCommand
GlobalCommandExecutor
AiCommandRunner
EditorCommandBus
ReflectionMutator
SchemaDrivenExecutor
```

It must not provide a backdoor where a tool can mutate arbitrary domain state by saying:

```text
path = "whatever.field"
value = "new value"
apply()
```

Paths and values are descriptive and transport vocabulary. Owning domains decide whether a proposed edit is meaningful, legal, executable, ratified, persisted, or rejected.

## Architectural position

`foundation/schema` belongs in the foundation layer.

```text
foundation/id
foundation/diagnostics
foundation/ratification
foundation/schema
    ↓
domain crates
    ↓
engine/runtime
    ↓
apps/adapters/tools
```

It should be usable by:

```text
domain/editor/editor_inspector
domain/editor/editor_scene
domain/editor/editor_persistence
domain/ui/ui_surface
domain/scheduler
domain/ecs
domain/world_ops
future domain/gameplay/*
future foundation/commands
apps/runenwerk_editor
tools and AI integrations
```

It must be independent of all of them.

The architectural role is:

```text
SchemaDescriptor describes shape.
Domain ratifier checks candidate against domain rules.
DiagnosticReport explains observations.
CommandDescriptor, later, describes requestable mutations.
CommandExecutor, currently domain/editor/runtime-owned, executes through the owning boundary.
```

## Dependency rules

Initial dependencies:

```text
core
alloc behind feature = "alloc"
serde behind feature = "serde"
diagnostics behind feature = "diagnostics"
```

Allowed foundation dependencies:

```text
diagnostics, optional only
```

No dependency on `ratification` in Phase 1.

Rationale:

- Schema should be usable without ratification.
- Ratification should be usable without schema.
- Owning domains can combine schema and ratification in their own ratifiers.
- Avoid foundation-to-foundation cycles.
- Avoid premature coupling where every schema issue becomes a ratification issue.

Feature policy:

```text
default = ["std"]
std = ["alloc"]
alloc = []
serde = ["dep:serde", "alloc"]
diagnostics = ["dep:diagnostics", "alloc"]
```

The diagnostics feature should use `diagnostics` with `default-features = false` and `features = ["alloc"]`, matching the ratification pattern.

`serde` must remain optional and must not be required by core schema usage.

`foundation/schema` must forbid unsafe code.

## Core concepts

### Schema identity

A schema identity names a reusable shape contract.

Examples:

```text
scene.local_transform
editor_scene.rename_entity.parameters
ui_surface.definition
scheduler.graph_definition
world_ops.chunk_delta
```

A schema ID is not a Rust `TypeId`, ECS reflection ID, runtime resource ID, editor component ID, or persistence file path.

A schema ID should be stable across builds and tools.

### Schema version

A schema version identifies the shape contract version.

Versions start at `1`.

Schema versioning is about compatibility of the schema contract, not runtime entity version, document version, save-file version, or editor history version.

### Schema descriptor

A schema descriptor is a named, versioned description of a value shape.

It may include:

```text
id
version
display name
description
root shape
metadata
compatibility notes
```

It does not include:

```text
executor
validator
ratifier
mutation function
reflection accessors
global registration hook
```

### Shape

A shape describes the expected structure of a value.

Initial shape categories should be small:

```text
Bool
Integer
Float
String
Enum
Object
List
Map
Optional
Opaque
```

`Opaque` is necessary because not every domain value should be structurally inspectable by foundation. It lets domains publish stable descriptors for values that are known but not tool-editable.

### Field

A field describes one named member of an object shape.

It may include:

```text
name
display name
description
shape reference or inline shape
constraints
read-only hint
presence hint
metadata
```

`ReadOnlyHint` is a tooling hint, not an authorization guarantee. `RequiredPresence` describes expected shape presence, not domain acceptance.

### Path

A schema path identifies a location inside a described shape.

It must be stable, structural, and independent of editor widgets.

### Value

A schema value is a portable representation of a proposed or observed value.

It is not a replacement for domain structs.

### Constraint

A constraint describes generic facts that tools can display or that domain-owned ratifiers can choose to interpret.

It does not own domain validity.

## Public vocabulary shape

The public vocabulary should be organized by subdomain responsibility, not by technical layer:

```text
foundation/schema/src/lib.rs
foundation/schema/src/id.rs
foundation/schema/src/version.rs
foundation/schema/src/path.rs
foundation/schema/src/value.rs
foundation/schema/src/shape.rs
foundation/schema/src/field.rs
foundation/schema/src/constraint.rs
foundation/schema/src/descriptor.rs
foundation/schema/src/compatibility.rs
foundation/schema/src/issue.rs
foundation/schema/src/metadata.rs
foundation/schema/src/diagnostic.rs
foundation/schema/src/prelude.rs
```

This is a target module structure, not an implementation requirement for this document.

Recommended public modules:

```text
schema::id
schema::version
schema::path
schema::value
schema::shape
schema::field
schema::constraint
schema::descriptor
schema::compatibility
schema::issue
schema::metadata
schema::diagnostic
schema::prelude
```

Recommended prelude exports should include only common authoring/consumption vocabulary:

```text
SchemaId
SchemaVersion
SchemaDescriptor
SchemaShape
SchemaField
SchemaPath
SchemaPathSegment
SchemaValue
SchemaConstraint
SchemaCompatibility
```

Do not export advanced diagnostics bridge types from the prelude unless they prove common.

## Relationship to diagnostics

Diagnostics explain observed issues.

Schema may produce diagnostics for schema-definition problems, such as:

```text
schema.id.invalid_format
schema.field.duplicate_name
schema.version.zero
schema.constraint.invalid_range
schema.path.empty_segment
```

These are about malformed schema descriptions, not invalid domain data.

Constructor and well-formedness errors remain typed control-flow errors in `foundation/schema`. Diagnostics are optional reporting projections of those errors or issues. The optional `diagnostics` feature may map schema-definition issues into diagnostic reports only when the caller explicitly requests that projection.

Policy:

- `foundation/schema` may define `SchemaIssue` and `SchemaIssueCode` for schema self-invariants.
- With optional `diagnostics`, schema issues may be converted into `DiagnosticReport` by explicit caller action.
- Domains still own domain diagnostic codes.
- Schema must not require every consumer to depend on diagnostics.
- Diagnostics must not become acceptance policy.
- Diagnostics must not become normal constructor return types.

Correct relationship:

```text
SchemaIssue -> optional Diagnostic
DomainValidationIssue -> owning domain Diagnostic
RatificationIssue -> optional Diagnostic via domain mapper
```

Forbidden relationship:

```text
Diagnostic severity decides whether a schema-described command executes.
```

## Relationship to ratification

Ratification answers whether a candidate is accepted.

Schema describes what shape a candidate claims to have.

The two should remain separate in Phase 1.

Correct usage:

```text
domain owns Candidate
domain owns CandidateRatifier
domain ratifier reads SchemaDescriptor if useful
domain ratifier emits RatificationReport
domain maps issues to diagnostics if useful
```

`foundation/schema` should not depend on `foundation/ratification` initially.

`foundation/ratification` should not depend on `foundation/schema` initially.

A later optional integration is allowed only if real duplication appears in multiple domains. Even then, it should be an adapter or bridge, not a new authority model.

## Relationship to commands

`foundation/commands` uses schema vocabulary for command parameter and result description.

Expected relationship:

```text
CommandDescriptor
  id
  display_name
  parameter_schema: SchemaId + SchemaVersion
  result_schema: Option<SchemaId + SchemaVersion>
```

Command descriptors should primarily reference `SchemaId` plus `SchemaVersion`. Inline `SchemaDescriptor` usage should be limited to tests, generated documentation, explicitly local/private descriptors, or cases justified by the `foundation/commands` design.

Command proposals may carry:

```text
command contract id/version
parameter values as SchemaValue
origin/provenance handled elsewhere if designed
```

But `foundation/schema` must not define command descriptors, command proposal types, command outcomes, or command execution APIs. Shared descriptor/proposal vocabulary belongs to `foundation/commands`; concrete command meaning remains owned by domains.

Concrete command enums remain domain-owned:

```text
editor_scene::SceneCommandIntent
editor_shell::ShellCommand
future gameplay commands
future asset commands
```

Command execution remains owning-domain/runtime/editor behavior.

## Relationship to reflection and inspector systems

Schema is not reflection.

Reflection can discover or expose runtime/Rust type structure. Schema describes a stable contract shape.

Current ECS reflection has accessors and runtime type metadata. Schema must not copy that execution model into foundation.

Correct relationship:

```text
ECS reflection -> domain/editor adapter -> SchemaDescriptor
ECS reflection -> editor inspector adapter -> InspectorSection
SchemaDescriptor -> editor tool display hints
```

Forbidden relationship:

```text
foundation/schema -> ECS World
foundation/schema -> Rust TypeId
foundation/schema -> global reflected type registry
foundation/schema -> mutable field accessor
```

Schema is not the inspector.

The editor inspector may eventually map:

```text
InspectorPath       <-> SchemaPath
InspectorValue      <-> SchemaValue
InspectorField      <- SchemaField plus domain/editor presentation state
InspectorSection    <- editor-owned grouping of schema-described fields
InspectorEditValue  <- subset/projection of SchemaValue
```

But `editor_inspector` should retain inspector-specific concepts:

```text
focus
draft state
expanded sections
widget kind
validation message display
target resolution
ECS bridge
reflective mutation
```

The schema crate should not know about any of those.

## Relationship to ECS/editor/runtime/apps/adapters

### ECS

ECS may publish schema descriptors for reflected components/resources later.

ECS owns:

```text
World
Entity
Component
Resource
Reflect
TypeRegistry
field accessors
query/runtime behavior
```

Schema owns only the portable description vocabulary.

### Editor core

`editor_core` owns editor governance, command metadata, history-facing concepts, transactions, selection/session state, sharing, reconciliation, and `RatifiedChange`.

Schema must not move those into foundation.

### Editor inspector

`editor_inspector` can consume schema descriptors to improve field display and future editing workflows.

It still owns target resolution, inspector sessions, adapter contracts, UI-facing field grouping, and mutation dispatch.

### Editor scene

`editor_scene` can use schema vocabulary for field edit parameter descriptions and future command descriptors.

It still owns concrete scene commands and scene-authoring invariants.

### Editor persistence

`editor_persistence` can use schema descriptors for migration/import/export validation context.

It still owns concrete persistence formats, codecs, migrations, normalization, and persistence diagnostics.

### UI surface

`ui_surface` can use schema descriptors for surface definitions, tool surface metadata, and editor-facing capability descriptions.

It still owns surface semantics, mount constraints, observation contracts, and surface ratifiers.

### Runtime/engine

Runtime may consume schema descriptors to expose debug/tooling metadata.

Runtime must not become schema-authoritative and must not use schema to bypass domain command boundaries.

### Apps/adapters/tools

Apps and adapters may serialize, inspect, display, or route schema-described proposals.

They must not define core domain invariants.

## Relationship to AI/editor tooling

AI is a proposal source, not a privileged authority.

Schema improves AI/editor tooling by making request shapes explicit:

```text
tool asks: what parameters does this command accept?
schema answers: entity_id and new_display_name
AI proposes: values for those parameters
domain command boundary receives proposal
domain ratifier accepts/rejects
diagnostics explain the result
```

Rules:

- AI integrations live in apps/tools/adapters.
- AI consumes schema descriptors but does not define authoritative domain schemas.
- AI proposals use the same command/schema/ratification/diagnostic contracts as humans, tests, scripts, and editor tools.
- AI must not mutate `World`, editor state, scene state, or persistence files directly through schema paths.
- Prompt design, model selection, LLM clients, and agent planning are outside foundation.

Schema can make AI safer by reducing ambiguity, but it cannot make AI authoritative.

## Schema path model

The path model should be structural, stable, and portable.

Initial path segment kinds:

```text
field(name)
index(number)
key(string)
variant(name)
```

Meanings:

- `field(name)`: named object/struct field.
- `index(number)`: list/tuple index.
- `key(string)`: map/dictionary key.
- `variant(name)`: enum/union variant payload.

The existing `InspectorPath` supports field and index segments. `SchemaPath` should be a foundation-general version that can later replace or interoperate with inspector paths.

Path rules:

- A root path is valid.
- A path is an ordered list of segments.
- Segment names must not be empty.
- String rendering must be deterministic.
- Display formatting must not be the only stored representation.
- Paths do not resolve themselves against runtime state.
- Paths do not mutate state.
- Paths do not guarantee that the target exists in a domain object.

The crate may provide stable formatting/parsing only if it can be done without ambiguity.

Do not use raw dot strings as the authoritative path representation. Dot strings are acceptable display/export forms only.

## Value model

The value model should be portable and bounded.

Initial value kinds:

```text
Null
Bool
Integer
UnsignedInteger
Float
String
EnumSymbol
List
Map
Object
Opaque
```

Policy:

- `SchemaValue` is for proposals, transport, tests, inspection, and descriptors.
- It is not a replacement for domain structs.
- It is not arbitrary untyped JSON under a new name.
- It must preserve enough type distinction for common editor/tool flows.
- It must not carry runtime pointers, Rust `TypeId`, closures, ECS entity handles, or backend resources.
- Integer and unsigned integer should remain distinct.
- Float values should define finite-value policy. Non-finite floats should be rejected by constructor or represented only through an explicit non-finite policy if a domain proves the need.
- Object/map ordering must be deterministic when `alloc` is enabled.
- Public serialized output must not depend on hash-map iteration order.

`SchemaValue` should support both current inspector edit values and future command parameter proposals, but not every possible runtime object.

Mapping from current inspector values:

```text
InspectorEditValue::Bool    -> SchemaValue::Bool
InspectorEditValue::Integer -> SchemaValue::Integer
InspectorEditValue::Float   -> SchemaValue::Float
InspectorEditValue::Text    -> SchemaValue::String
```

Unsupported or domain-specific values should remain opaque or domain-owned until a stable shared need appears.

## Deterministic storage and ordering

Schema descriptors must be deterministic as data, not only when formatted for display.

Deterministic ordering is required for:

- object fields;
- map/object values;
- schema metadata;
- constraint lists;
- issue lists and reports;
- serialized descriptors, paths, values, metadata, and constraints.

Phase 1 should prefer explicitly ordered representations, such as vectors of key/value pairs with duplicate-key checks, for owned object/map values and metadata. Another representation is acceptable only if its public iteration and serialization order are explicitly deterministic.

Public serialized output must not depend on `HashMap` iteration order. If hash-based storage is ever used internally, the public API must sort or otherwise canonicalize output before iteration, reporting, or serialization.

## Constraint model

Constraints describe generic, portable schema facts and tooling hints. Their names should make that descriptive role visible. They are not domain acceptance rules and must not be named like policy enforcement hooks.

Initial constraint categories:

```text
RequiredPresence
ReadOnlyHint
Deprecated
NumericMin
NumericMax
NumericRange
StringMinLength
StringMaxLength
NamedStringPatternHint
EnumOptions
ListMinLength
ListMaxLength
MapKeyShape
SuggestedDefaultValue
RecommendedValue
DisplayUnitLabel
Documentation
```

Important distinction:

```text
constraint descriptor = shape metadata
domain invariant = owning domain rule
```

Examples of descriptive constraints:

```text
field display_name is required
integer count has minimum 0
string name has max length 64
enum primitive_kind has options box/sphere/capsule
field transform.translation uses unit "meters"
```

Examples of domain invariants that do not belong in foundation:

```text
entity parent must not create hierarchy cycle
surface instance host must exist in this workspace
scene entity id must resolve in current document
network command must match current tick
ability cost must be affordable by current actor
```

Foundation may validate constraint self-consistency:

```text
min <= max
enum options are non-empty
required presence cannot be applied to impossible location if schema shape knows it
pattern name is non-empty
```

Foundation must not validate candidate data against domain meaning.

Phase 1 must not include generic `SchemaValue`-against-`SchemaShape` structural validation beyond schema-definition self-invariants. A later generic structural checker may be considered only if there is a demonstrated need and its API cannot be mistaken for domain ratification, command acceptance, or editor governance.

## Versioning and compatibility

Schema descriptors must be versioned from version `1`.

Compatibility metadata should describe intended compatibility, not enforce migration.

Initial compatibility vocabulary:

```text
Compatible
BackwardCompatible
ForwardCompatible
Breaking
Deprecated
Unknown
```

Compatibility applies to schema contracts, not automatically to persisted file formats or runtime state.

Rules:

- New schemas start at version `1`.
- Changing only display names or documentation does not require a version bump unless tools depend on them.
- Adding an optional field is usually backward-compatible.
- Adding a required field is usually breaking.
- Removing a field is usually breaking.
- Changing a primitive kind is breaking.
- Renaming a field is breaking unless an alias/migration path is explicitly supplied by the owning domain.
- Foundation does not run migrations.
- Owning domains own schema version publication and compatibility promises.

Schema aliases may be useful later, but Phase 1 should avoid migration machinery.

## Serialization policy

`serde` should be optional.

Serialization exists for:

```text
tooling
tests
snapshot/golden tests
command descriptors
AI/tool proposal payloads
documentation generation
persistence-adjacent metadata
```

Serialization does not make schema the persistence format owner.

Rules:

- `serde` requires `alloc`.
- Serialized schema descriptors must preserve stable IDs, versions, paths, values, and constraints.
- Serialized object/map values, schema fields, metadata, constraints, and issue lists must use deterministic ordering.
- Public serialization must not expose `HashMap` iteration order.
- Serialization format choice is caller-owned.
- Do not hard-code JSON, RON, YAML, or binary encoding policy into foundation.
- Do not add persistence migrations to foundation/schema.
- Do not make serde mandatory for normal Rust usage.

## Validation and ratification policy

Use the word "validation" carefully.

Allowed in foundation/schema:

```text
schema definition well-formedness checks
constructor-level invariants
schema id format checks
path segment format checks
field uniqueness inside one descriptor
constraint descriptor consistency checks
```

Forbidden in foundation/schema:

```text
generic SchemaValue-against-SchemaShape validation in Phase 1
domain object validity
candidate acceptance
command preconditions
entity existence checks
workspace mount validity
scene hierarchy cycle checks
editor session validity
AI proposal trust checks
runtime resource availability checks
```

Phase 1 validation is limited to schema-definition self-invariants and constructor invariants. It must not offer a general API that accepts a `SchemaValue` and `SchemaShape` and returns a pass/fail result for candidate data. That API shape would be too easy to mistake for domain ratification.

Ratification policy:

- Owning domains ratify candidates.
- Domain ratifiers may use schema descriptors as context.
- Schema constraints can inform ratification, but do not replace ratification.
- `RatificationReport` remains the acceptance report vocabulary.
- `RatifiedChange` remains editor governance, not foundation schema.
- `GoverningChangeError` remains typed control flow for current editor governance.

Correct flow:

```text
Schema describes shape.
Tool builds proposal.
Owning domain resolves target.
Owning domain ratifies candidate.
Owning domain executes through command/import/builder boundary if accepted.
Diagnostics explain issues.
```

## Error/diagnostic policy

Schema should have local typed errors for constructor and well-formedness failures.

Examples:

```text
SchemaIdError
SchemaVersionError
SchemaPathError
SchemaFieldError
SchemaConstraintError
SchemaDescriptorError
```

These are not diagnostics by default.

With optional diagnostics support, schema can map schema-definition issues into diagnostics when the caller explicitly asks for reporting/projection.

Rules:

- Constructor errors are typed control flow.
- Well-formedness errors are typed control flow.
- Diagnostics are reporting artifacts.
- Diagnostic reports are optional projections, not authoritative schema state.
- Ratification reports are acceptance artifacts.
- Do not collapse all three into one type.
- Domain diagnostic code families remain domain-owned.
- Foundation schema may own only `foundation.schema.*` infrastructure issue codes.
- Diagnostics must not become normal constructor return types.
- Diagnostics must not define schema acceptance policy.

## Invariants

`foundation/schema` must preserve these invariants:

1. It does not depend on domain, engine, runtime, app, adapter, backend, editor, UI, ECS, or AI integration crates.
2. It does not execute commands.
3. It does not mutate state.
4. It does not reflect Rust values.
5. It does not resolve ECS entities, resources, components, or TypeIds.
6. It does not own domain validation rules.
7. It does not own concrete domain schemas.
8. It does not own a global schema registry.
9. It does not define AI behavior.
10. It does not define editor widget policy.
11. It does not replace diagnostics.
12. It does not replace ratification.
13. It does not replace commands.
14. It does not replace persistence formats.
15. All public IDs and paths have validating constructors or equivalent invariant-preserving construction.
16. Descriptor ordering is deterministic.
17. Object/map values, schema fields, metadata, constraint lists, issue lists, and serialized output have deterministic order.
18. Public serialized output does not depend on `HashMap` iteration order.
19. Phase 1 does not provide generic `SchemaValue`-against-`SchemaShape` validation beyond schema-definition self-invariants.
20. Schema versions start at `1`.
21. Optional `serde` support preserves structured data, not display-only strings.
22. `no_std` without `alloc` remains possible for scalar/static identity pieces.
23. `alloc` enables owned descriptors, values, fields, metadata, and reports.

## Anti-goals

The crate is explicitly not:

```text
a reflection system
a dynamic object model
a universal scene graph
a JSON schema clone
a serde framework
a command bus
a command executor
a domain validator
an editor inspector implementation
a UI widget factory
a global registry
an ECS world accessor
an AI command runner
a persistence migration engine
a runtime resource graph
```

It is also not a compatibility layer for:

```text
JSON Schema
OpenAPI
TypeScript schemas
serde data models
Rust reflection metadata
```

Specific rejected designs:

### Rejected: `GlobalSchemaRegistry`

A global registry would centralize ownership and create order-of-initialization, plugin, testing, and authority problems. Domains may publish registries or descriptor sets behind their own APIs. Apps/tools may aggregate them explicitly.

### Rejected: `SchemaValue::Object(Box<dyn Any>)`

This would destroy portability, serialization, no_std compatibility, and tooling usefulness.

### Rejected: `SchemaPath::apply_mut`

A path vocabulary must not mutate state. Mutation belongs to command/import/builder/domain-specific transaction boundaries.

### Rejected: `SchemaConstraint::DomainRule(String)`

This would become a stringly-typed validation backdoor. Domain rules belong to owning ratifiers and typed errors.

### Rejected: direct ECS reflection dependency

ECS reflection is useful but domain-owned. Schema can describe output produced from reflection; it must not depend on reflection.

## Examples of correct usage

### Domain publishes a descriptor

A scene-authoring domain publishes a descriptor for a local transform component shape.

The descriptor says:

```text
schema id: scene.local_transform
version: 1
shape: object
fields:
  translation: vec3-like object
  rotation: quaternion-like object or opaque
  scale: vec3-like object
```

The scene domain still owns what values are valid.

### Inspector consumes a descriptor

The editor inspector receives a descriptor for a selected component.

It uses field names, display names, `ReadOnlyHint`, enum options, and `DisplayUnitLabel` to build a better inspector view.

When the user commits a change, the inspector sends an owning-domain command or intent. The schema descriptor does not mutate the component.

### Command descriptor references parameter schema

Future `foundation/commands` describes `scene.rename_entity`.

Its parameter schema says:

```text
entity: entity identifier value
new_display_name: string with minimum length 1
```

A tool can build a form or AI proposal from that shape. The scene domain still checks that the entity exists and the rename is legal.

### AI proposal uses schema without privilege

An AI tool asks for available command descriptors and schemas.

It proposes:

```text
scene.rename_entity(entity = 12, new_display_name = "Player")
```

The editor/app router submits that proposal through the same command boundary as a human UI action. The owning domain ratifies it. Diagnostics explain rejection or warnings.

### Persistence validation uses schema context

An import pipeline sees a field whose serialized value does not match a known descriptor.

It can report a diagnostic with a schema path and expected shape.

The persistence domain owns the import decision and migration behavior.

## Examples of forbidden usage

### Forbidden: foundation-level mutation

```text
schema.apply(world, path, value)
```

Reason: mutation belongs to owning domains/runtime/editor command boundaries.

### Forbidden: global schema authority

```text
GlobalSchemaRegistry::register_everything()
GlobalSchemaRegistry::get("scene.local_transform")
```

Reason: global registries create hidden coupling and authority ambiguity.

### Forbidden: domain invariants in foundation constraints

```text
SchemaConstraint::NoSceneHierarchyCycle
SchemaConstraint::SurfaceHostMustExist
SchemaConstraint::ActorMustHaveEnoughMana
```

Reason: these are domain invariants.

### Forbidden: AI-only schema execution path

```text
AiCommandRunner::execute_schema_value_patch(...)
```

Reason: AI must use the same contracts as humans, tests, scripts, and editor tools.

### Forbidden: reflection backdoor

```text
SchemaPath -> Rust TypeId -> field accessor -> mutation
```

Reason: this bypasses commands and ratification.

### Forbidden: replacing editor governance

```text
SchemaValidationResult replaces GoverningChangeError
SchemaCommit replaces RatifiedChange
```

Reason: current editor governance remains editor-owned.

## Migration/adoption plan

### Phase 0: Accept this design

Status: complete.

Review this document against:

```text
ARCHITECTURE.md
docs-site/src/content/docs/guidelines/dependency-rules.md
AI_GUIDE.md
foundation-vocabulary-crates-design.md
foundation-ratification-phase5-evaluation.md
current editor_inspector code
current editor_scene command code
current ECS reflection code
```

### Phase 1: Add crate skeleton and core vocabulary

Status: complete.

Implemented `foundation/schema` with:

```text
Cargo.toml
src/lib.rs
src/id.rs
src/version.rs
src/path.rs
src/value.rs
src/shape.rs
src/field.rs
src/constraint.rs
src/descriptor.rs
src/compatibility.rs
src/issue.rs
src/metadata.rs
src/prelude.rs
```

Implemented:

```text
no_std/std/alloc feature layout
optional serde
forbid unsafe
basic constructor invariants
deterministic ordered storage for fields, metadata, constraints, object values, and map values
unit tests
```

Do not add integrations.

Do not add global registry.

Do not add reflection.

Do not add command descriptors.

Do not add generic `SchemaValue`-against-`SchemaShape` structural validation.

### Phase 2: Add optional diagnostics bridge

Status: complete.

Implemented optional `diagnostics` feature for schema-definition issues only. The bridge projects existing schema issues and constructor/well-formedness errors into `foundation/diagnostics` reporting artifacts. Constructor errors remain typed control flow, and diagnostics do not decide acceptance or execution.

Validated:

```text
cargo test -p schema
cargo test -p schema --no-default-features
cargo test -p schema --features alloc
cargo test -p schema --features serde
cargo test -p schema --features diagnostics
cargo test -p schema --features serde,diagnostics
cargo clippy -p schema --all-targets --all-features -- -D warnings
python3 tools/docs/validate_docs.py
./quiet_full_gate.sh
```

After each phase, run the phase completion drift-check routine before starting the next phase.

### Phase 3: One low-risk consumer

Status: complete.

Implemented one narrow consumer:

```text
editor_inspector path/value interoperability helpers
```

The helpers:

```text
convert InspectorPath to SchemaPath
convert supported SchemaPath values back to InspectorPath
reject unsupported SchemaPath key and variant segments
convert supported InspectorValue values to SchemaValue
convert InspectorEditValue values to SchemaValue
```

The implementation intentionally did not rewrite the inspector, replace inspector edit flow, change command execution, publish descriptors, add registries, add reflection, or validate `SchemaValue` against `SchemaShape`.

### Phase 4: Descriptor publication for one domain shape

Status: complete.

Published one domain-owned descriptor set:

```text
scene local transform
```

Implemented:

```text
domain/scene/src/schema.rs
local_transform_schema_descriptor
LOCAL_TRANSFORM_SCHEMA_ID = "scene.local_transform"
```

The descriptor root shape is an object with deterministic field order:

```text
translation
rotation
scale
```

Nested value shapes preserve deterministic order:

```text
Vec3Value: x, y, z
QuatValue: x, y, z, w
```

The owning domain publishes the descriptor. Foundation does not register it globally. The descriptor does not validate domain values, mutate state, call ECS reflection, or participate in command execution.

### Phase 5: Prepare `foundation/commands` design

Status: complete.

The accepted commands design references `SchemaId`, `SchemaVersion`, and `SchemaValue`. Phase 1 of the separate commands crate intentionally does not add a top-level proposal `SchemaPath`; target identity belongs in schema-described parameters unless a future consumer proves otherwise.

## Testing strategy

### Foundation schema tests

Expected behavior tests:

```text
schema_id_rejects_empty_identifier
schema_id_rejects_whitespace
schema_version_rejects_zero
schema_path_root_is_valid
schema_path_preserves_segment_order
schema_path_rejects_empty_field_name
schema_value_preserves_integer_signedness
schema_value_rejects_non_finite_float
schema_value_object_rejects_duplicate_keys
schema_value_map_rejects_duplicate_keys
schema_value_object_preserves_key_order
object_shape_rejects_duplicate_field_names
constraint_range_rejects_min_greater_than_max
descriptor_preserves_field_order
descriptor_preserves_constraint_order
metadata_preserves_key_order
descriptor_reports_highest_schema_issue_deterministically
phase1_has_no_generic_schema_value_shape_validator
```

### Feature tests

Expected feature-gate tests:

```text
cargo test -p schema
cargo test -p schema --no-default-features
cargo test -p schema --features alloc
cargo test -p schema --features serde
cargo test -p schema --features diagnostics
cargo test -p schema --features serde,diagnostics
```

### Serialization tests

When `serde` is enabled:

```text
schema_descriptor_round_trips_with_version
schema_path_round_trips_structurally
schema_value_round_trips_without_losing_numeric_kind
constraint_round_trips_without_display_string_parsing
```

### Diagnostics bridge tests

When `diagnostics` is enabled:

```text
schema_issue_maps_to_expected_diagnostic_code
schema_issue_maps_to_useful_diagnostic_subject
multiple_schema_issues_preserve_order
diagnostics_bridge_does_not_define_acceptance_or_ratification_semantics
```

### Integration tests after adoption

For editor inspector interop:

```text
inspector_path_converts_to_schema_path
schema_path_converts_to_inspector_path_when_supported
schema_path_with_unsupported_segment_is_rejected_by_inspector_interop
inspector_value_converts_to_schema_value
inspector_edit_value_converts_to_schema_value
schema_interop_does_not_validate_value_against_shape
```

For command descriptor preparation:

```text
scene_edit_component_field_descriptor_references_parameter_schema
command_parameter_schema_does_not_execute_command
```

### Phase 3 validation

Validated:

```text
cargo fmt --all
cargo test -p editor_inspector
cargo clippy -p editor_inspector --all-targets --all-features -- -D warnings
```

### Phase 4 validation

Validated:

```text
cargo fmt --all
cargo test -p scene
cargo clippy -p scene --all-targets --all-features -- -D warnings
```

### Workspace gates

After implementation:

```text
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
python3 tools/docs/validate_docs.py
./quiet_full_gate.sh
```

## Open questions

No blocking open questions remain for Phase 5 design preparation.

Deferred questions:

1. Should a later `foundation/schema_macros` exist?
   - Defer. Derive support should wait until manual descriptors prove stable.
2. Should ECS reflection generate schema descriptors automatically?
   - Defer. This belongs in an ECS/editor adapter, not foundation.
3. Should schema include aliases/migrations?
   - Defer. Owning domains should first publish versioned descriptors manually.
4. Should schema include generic structural validation of `SchemaValue` against `SchemaShape`?
   - Defer. Phase 1 through Phase 4 did not include it. It can be reconsidered later only if its API, naming, and documentation cannot be mistaken for domain ratification, command acceptance, or editor governance.
5. Should `foundation/commands` depend directly on `foundation/schema`?
   - Yes for Phase 1 of the separate commands crate. It depends on schema for schema references and proposal parameter values.
6. Should schema metadata include provenance?
   - Defer to a future provenance design if repeated cross-domain pressure appears.

## Final recommendation

Keep `docs-site/src/content/docs/design/implemented/foundation-schema-design.md` as the active phase roadmap for `foundation/schema`.

Phase 0, Phase 1, Phase 2, Phase 3, Phase 4, and Phase 5 are complete.

The next schema-specific step is maintenance only. Command descriptor/proposal vocabulary now lives in `foundation/commands`; do not add command execution, registries, broad schema catalogs, editor inspector rewrites, reflection-driven mutation, schema macros, or generic `SchemaValue`-against-`SchemaShape` validation to `foundation/schema`.

Do not include runtime behavior, command execution, editor inspector policy, ECS reflection, global registries, domain validation, AI behavior, or persistence backends.

The current implementation milestone is:

```text
foundation/schema = portable shape vocabulary
optional diagnostics projection for schema-definition issues
one narrow editor_inspector path/value interop consumer
one domain-owned scene.local_transform descriptor publication
foundation/commands design accepted; Phase 1 vocabulary implemented separately
```

The long-term shape remains:

```text
foundation/schema describes typed shape
foundation/commands describes requestable mutation contracts and inert proposals
owning domains publish concrete descriptors and execute commands
owning ratifiers decide acceptance
diagnostics explain observations
AI/editor/tools use the same contracts as everyone else
```
