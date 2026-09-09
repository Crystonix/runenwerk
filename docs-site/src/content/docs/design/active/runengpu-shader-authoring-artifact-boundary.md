---
title: RunenGPU Shader Authoring and Canonical WGSL Artifact Boundary
description: Ownership rules for shader meaning, canonical WGSL artifacts, compiler-known program facts, host layout refinements, and private backend realization.
status: active
owner: gpu
layer: framework/gpu
canonical: true
last_reviewed: 2026-08-26
related_docs:
  - ./runengpu-architecture-design.md
  - ./runengpu-g4-context-program-realization-design.md
  - ./runengpu-phase-requirements-proof-matrix.md
  - ../accepted/runenrender-decomposition-design.md
  - ./runenrender-internal-decomposition-execution-plan.md
  - ../../architecture/repository-family-architecture.md
  - ../../adr/accepted/0015-separate-gpu-execution-from-rendering.md
  - ../../workspace/specs/pt-runengpu-g4b-program-interface-layout.ron
  - ../../workspace/specs/pt-runengpu-g4c-wgpu-realization-cutover.ron
---

# RunenGPU Shader Authoring and Canonical WGSL Artifact Boundary

## Status and authority

This design originally clarified the accepted G4 program/source boundary through issue
`#203` and accepted merge:

```text
62c3949d31a7c03f1f554f8108120d9767139123
```

The G4 design remains the historical baseline for the phase it bound.

The G6-SH01 decision through issue `#333` narrowly supersedes the old G4 rule that a
caller-authored complete resource interface is authoritative and compiler reflection is
validation evidence only. For G6 and later, this document owns the current
shader-interface boundary wherever that older wording conflicts.

This revision does **not** reopen unrelated G4 context, capability, specialization,
pipeline, cache, realization, or backend-containment decisions.

The runtime source decision remains:

```text
GpuProgramSource admits canonical WGSL source only.
```

Human authoring language, admitted runtime artifact, program selection, and host layout
policy remain separate concerns.

## Decision summary

Runenwerk keeps one portable runtime path:

```text
consumer shader/kernel meaning
    -> Runenwerk authoring/toolchain policy
        -> canonical WGSL artifact

consumer program admission request
    -> selected entry-point names
    -> explicit host/layout refinements only where WGSL cannot decide

canonical WGSL + admission request
    -> private compiler analysis
        -> compiler-known entry-point + binding facts
        -> validated refinements
            -> one effective RunenGPU program interface
                -> derived capability requirements
                -> pipeline/layout/runtime validation
                    -> private backend realization
```

The language strategy remains:

1. Plain WGSL is valid authoring input and the only current RunenGPU runtime artifact.
2. WESL-to-WGSL is the preferred near-term modular authoring candidate, gated by a
   separate deterministic toolchain proof.
3. Slang-to-WGSL is the preferred future advanced candidate, gated by portable
   WGSL/WebGPU and Metal evidence.
4. WESL and Slang do not become direct `GpuProgramSource` variants.
5. SPIR-V, GLSL, HLSL, DXIL, MSL, Naga IR, and backend-native modules do not become
   public or stable RunenGPU authority through this decision.
6. No custom Runen shader language or custom source preprocessor is authorized.

# Ownership

## Consumer owns shader or kernel meaning

RunenRender owns rendering meaning: material, medium, lighting, transport,
image-formation choices, renderer variants, and lowering from prepared render state.

A non-render consumer owns its own kernel meaning. Simulation, SDF processing, image
processing, and other GPU users may use RunenGPU without depending on RunenRender.

Consumers select which admitted entry points they intend to use and supply explicit
host/layout policy only where the canonical shader cannot determine it. They do not
re-author compiler-known shader facts.

## Runenwerk owns authoring/toolchain policy

Runenwerk retains:

- shader source roots and filesystem paths;
- module/package resolution;
- compiler selection, version, and options;
- deterministic translation/compilation;
- source dependency graphs and invalidation;
- generated-artifact provenance and source maps;
- file watching and reload scheduling;
- atomic publication and last-known-good fallback;
- developer diagnostic presentation;
- build-time versus development-time compiler invocation;
- persisted artifact/capture/reproducibility policy.

These concerns do not enter the future-transferable RunenGPU API.

## RunenGPU owns admitted GPU contracts

RunenGPU owns:

- source keys, revisions, and canonical-source consistency;
- canonical WGSL runtime admission;
- validation of selected entry-point names;
- compiler-derived entry-point existence/stage facts;
- compiler-derived typed `(group, binding)` identities and normalized resource facts;
- bounded explicit host/layout refinements where WGSL cannot determine policy;
- one immutable effective `GpuProgramInterfaceDescriptor`;
- capability requirements deterministically derived from that effective interface;
- bind-group and pipeline-layout descriptors derived from that interface;
- specialization schemas/values and generic compute/render pipeline descriptors;
- compiler-observed shader-I/O conformance against render pipeline state;
- runtime-binding compatibility;
- context/device-generation-bound module/layout/bind-group/pipeline realization;
- correctness-complete realization/cache identity and structured rejection.

No canonical source is accepted merely because a frontend generated it. RunenGPU still
validates source identity, selected entries, compiler-known facts, explicit refinements,
derived requirements, pipeline compatibility, and runtime bindings at the owning
boundaries.

## Private compiler/backend implementation

RunenGPU may use a private parser/compiler implementation such as Naga to analyze
canonical WGSL and the selected backend to translate/realize it.

Public authority is normalized RunenGPU vocabulary. Naga IR, WGPU types, backend shader
modules, and generated SPIR-V/MSL/HLSL/GLSL are not public identity, persistence,
replay, wire, or interchange formats.

# Canonical artifact contract

## Canonical WGSL artifact

The runtime artifact is complete canonical WGSL source plus runtime source identity:

```text
canonical WGSL bytes
+ semantic source key
+ nonzero source revision
+ full source-content identity
+ bounded artifact provenance
```

Canonical WGSL may come from direct `.wgsl`, an accepted WESL/Slang translation, or a
future separately accepted frontend.

The artifact does **not** own:

- which entry points a consumer chooses to use;
- dynamic-offset policy;
- host minimum-binding-size policy;
- texture/sampler host filtering policy;
- visibility widening;
- render pipeline state;
- application resource selection.

Those are separate admission/pipeline inputs.

## Runtime identity

Filesystem paths, module names, package locations, and compiler-specific handles are not
runtime source identity.

RunenGPU source identity uses an opaque `GpuProgramSourceKey`, a nonzero owner-local
`GpuProgramSourceRevision`, full canonical WGSL equality, and a deterministic digest for
hashing/diagnostics only.

Reusing one key/revision with different WGSL bytes remains `SourceRevisionConflict`.

## Program admission request

A consumer supplies only the program choices not already encoded by the artifact:

```text
admitted canonical source
+ intended entry-point names
+ explicit host/layout refinements where required
+ bounded request provenance
```

Capability requirements implied by shader-visible resources are derived from the one
effective interface rather than supplied as a parallel caller-authored program contract.

The clean G6 result must not require callers to restate entry-point stages, binding
keys, resource classes, shader visibility, or other compiler-known facts merely for
agreement checking.

## Build artifact envelope

A future Runenwerk-owned persisted build envelope may record authoring-language and
compiler identity/options, dependency digest, canonical WGSL and digest, source maps,
and reproduction provenance.

It may also record product-specific admission inputs when needed for reproduction, but
that does not make those inputs frontend/compiler authority.

No persisted shader/build envelope is a RunenGPU stable format without separate
Runenwerk-owned versioning/migration authority.

# Program-interface authority

## Compiler-known facts

For canonical WGSL, program admission derives facts the artifact itself defines. These
include, where normalized safely into backend-neutral RunenGPU vocabulary:

- selected entry-point existence and stage;
- binding group/binding identity;
- resource class;
- uniform/storage-buffer class and storage access;
- sampled/storage texture class, dimension, multisampling, storage access, and format
  where WGSL encodes them;
- fixed binding-array cardinality;
- actual static stage use;
- compiler-known minimum buffer requirement.

Callers do not author a second copy of these facts.

If a required compiler fact cannot be normalized safely, admission rejects or the
design must be revisited. That does not justify exposing Naga/WGPU types.

## Explicit host/layout refinements

WGSL does not encode every host-side layout choice. Explicit typed refinements remain
only for those gaps, including where applicable:

- dynamic-buffer-offset enablement;
- optional host/layout minimum binding size;
- float texture filterable versus unfilterable policy;
- filtering versus non-filtering non-comparison sampler policy;
- optional visibility widening beyond observed static use, bounded to selected program
  stages;
- diagnostic labels/provenance, which remain non-semantic.

A refinement references compiler-owned identity. It cannot invent/remove/renumber a
binding or redefine compiler-known resource class/access/shape.

The implementation should use the smallest typed representation compatible with the
existing effective binding/layout model. This decision does not require a second durable
policy/interface hierarchy.

## Minimum-size distinction

Two facts remain distinct:

```text
compiler_required_minimum_size
    shader requirement derived from canonical WGSL when knowable

host_layout_minimum_size
    optional explicit host/layout policy
```

A host minimum cannot be weaker than a known compiler requirement. Absence of explicit
host policy must not be rewritten into a host policy merely because compiler analysis
knows a shader requirement.

Backend/runtime validation may consume both without conflating their authority.

## One effective resource interface

Program admission combines compiler-known binding facts with accepted refinements and
publishes one immutable effective `GpuProgramInterfaceDescriptor`.

That is the only ordinary resource-interface authority consumed by:

- capability derivation;
- bind-group/pipeline-layout derivation;
- runtime binding validation;
- pipeline/cache identity;
- private backend realization.

A public manually-authored full interface or public observed/reflection interface would
recreate parallel authority and is not part of the intended G6 result. Observation and
comparison machinery may remain private implementation/test/diagnostic detail.

## Entry points

Callers select intended source-level entry-point names. Program admission derives and
validates their stages from canonical WGSL.

Caller-authored stage values retained only to compare with WGSL are duplicate authority
and should disappear in the clean cutover.

Missing, malformed, duplicate, or incompatible selected entries reject before backend
pipeline realization.

## Shader I/O versus render pipeline state

Compiler analysis owns observed shader-I/O facts for selected vertex/fragment entries:
locations, shader value types, and builtins where normalized.

It does **not** own:

- vertex-buffer slot, stride, byte offset, or step mode;
- fragment target format selection;
- blend/write-mask policy;
- raster/depth/multisample policy.

Those remain render pipeline state. The accepted backend-neutral owner compares
compiler-observed signatures against pipeline expectations as early as practical.

Shader analysis must not turn render pipeline state into program-interface state.

## No semantic reflection expansion

Compiler analysis may establish shader/program facts. It may not infer:

- G3 hazards or operation ordering beyond authored operation accesses;
- initialization authority;
- retained-state/persistence meaning;
- application/domain meaning;
- runtime resource selection;
- reconstruction/recovery or product fallback policy.

A shader binding is not an application resource merely because its shader type is known.

## Rust byte preparation

Current `GpuParams`, `GpuUniform`, `GpuStorage`, `ToGpuValue`, `GpuUniformField`, and
associated derives remain byte-preparation helpers unless separately superseded.

They do not prove shader struct identity, binding identity, nested/array/runtime-array
ABI, matrix orientation, complete storage ABI, or complete program-interface
compatibility.

No universal shader ABI derive is authorized here.

# Authoring language disposition

## Plain WGSL

Plain WGSL remains the bootstrap/runtime baseline for direct authoring, generated
programs, conformance fixtures, minimal examples, fallback, and debugging.

Limited standardized module composition is an authoring concern, not a reason to add
runtime source variants.

## WESL

WESL remains the preferred near-term modular candidate because it lowers source
composition to canonical WGSL before RunenGPU admission.

A first WESL proof should cover deterministic imports/module linking, explicit module
roots, bounded conditional translation where proven necessary, deterministic WGSL,
source-map diagnostics, and complete dependency invalidation.

It must not create network package resolution during runtime, render-loop compiler
invocation, Runenwerk-specific syntax, or a second runtime source path.

Production applications should be able to consume prebuilt canonical WGSL without
shipping the WESL compiler unless separately proven necessary.

## Slang

Slang remains the preferred future advanced frontend candidate for modules, interfaces,
generics, specialization, reflection, and multiple outputs. It is not the baseline until
required portable WGSL/WebGPU and Metal paths are proven for this repository.

A future integration remains outside RunenGPU:

```text
Slang source
    -> pinned Runenwerk-owned Slang compiler
        -> canonical WGSL
            -> RunenGPU admission
```

Slang reflection may support frontend/tooling validation but does not become a second
runtime interface authority. A native Slang-to-SPIR-V path is not a parallel baseline.

## HLSL, GLSL, SPIR-V, and backend IR

HLSL, GLSL, and SPIR-V remain possible import/migration inputs only when actual source
assets justify them. They are not current RunenGPU source kinds.

Naga IR and backend outputs remain private implementation facts and are never stable
RunenGPU source/cache/replay/interchange authority.

## Custom language

No custom Runen shader language, parser, preprocessor, or compiler is authorized.
Existing standards/toolchains must be exhausted before accepting that maintenance
burden.

# Frontend promotion gate

Any authoring frontend other than plain WGSL requires separately accepted proof of:

1. pinned compiler/version/options;
2. deterministic canonical WGSL generation/normalization;
3. complete dependency discovery/invalidation;
4. diagnostics mapped to authoring files/spans;
5. exact group/binding preservation in generated WGSL;
6. entry-point name/stage preservation in generated WGSL;
7. successful RunenGPU compiler-known program/interface admission;
8. host-buffer/matrix/array/structure conformance where relevant;
9. specialization normalization and deterministic artifact identity;
10. validation by the pinned runtime compiler/backend stack;
11. representative compute/render execution for the declared target matrix;
12. structured unsupported-feature rejection before publication;
13. atomic reload preserving last-known-good artifacts after failure;
14. reproduction facts sufficient for a Runenwerk-owned build/capture envelope;
15. no second runtime admission/interface/pipeline authority.

A frontend remains experimental until every mandatory target in its declared profile
passes.

# Historical G4 baseline and G6 consequence

G4 correctly established typed program/binding/layout/pipeline contracts, deterministic
identity, private backend realization, and one runtime path. Its then-accepted complete
caller-authored resource interface was the bootstrap authority.

G6-SH01 changes only the source of compiler-known program facts. It does not invalidate
G4's normalized effective binding/layout vocabulary or downstream pipeline/runtime
ownership.

Where the G4 design says the explicit descriptor is authoritative and reflection cannot
replace it, the G6+ authority is:

```text
compiler-known WGSL facts
+ explicit host/layout refinements
    -> one effective RunenGPU resource interface
```

Private backend realization consumes the already-admitted source and effective
interface. It must not retain a second authoritative reflection-versus-manual-interface
path after the cutover.

Material/shader graphs and renderer composition remain RunenRender semantic authoring.
They lower to canonical program artifacts, admission choices/refinements where required,
pipeline state, resources, and work rather than becoming a universal RunenGPU IR.

# G6-SH01 implementation handoff

The later bounded implementation must prove:

- compiler-known binding facts derive from canonical WGSL without duplicate caller
  declarations;
- selected entry-point stages are compiler-derived;
- unknown/contradictory refinements reject before backend realization;
- compiler-required and host-layout minimum sizes remain distinguishable;
- ambiguous float-texture/sampler policy follows the accepted explicit choice;
- observed stage visibility is default and widening is bounded;
- render pipeline state remains authoritative for vertex-buffer/target policy while
  compiler-observed shader I/O validates compatibility;
- pipeline layout, runtime binding validation, and capability derivation use one
  effective resource interface;
- no public Naga/WGPU type or public manual/observed compatibility interface appears;
- no shader-derived G3 hazards, initialization, retained-state meaning, application
  semantics, or runtime resource selection appears;
- accepted reaction diffusion no longer repeats compiler-known binding/stage facts in
  ordinary setup;
- native RunenGPU conformance and Wasm compilation remain valid.

This closes `G6-SH01` only. `G6-E01` and `G6-P01` remain separate gates.

# Stop conditions

Stop and require another architecture decision if:

- a frontend cannot produce canonical WGSL for its declared portable profile;
- correct use requires a second runtime source/interface/pipeline path;
- a supposed compiler-known fact cannot be normalized without exposing private
  compiler/backend types;
- compiler analysis would have to own host/layout/application policy;
- shader analysis is used to manufacture G3 hazards, initialization, retained-state,
  application meaning, or runtime resource selection;
- authoring/compiler policy cannot remain outside RunenGPU;
- frontend adoption requires RunenRender meaning inside RunenGPU;
- a custom shader language or Runenwerk-specific syntax extension appears necessary;
- mandatory target conformance cannot be represented honestly;
- generated canonical WGSL is nondeterministic and cannot be normalized safely.

# References

External references reviewed for the original frontend decision on 2026-08-02 remain
informational inputs, not Dornglut authority:

- WebGPU/WGPU shader source model: `https://docs.rs/wgpu/latest/wgpu/enum.ShaderSource.html`
- WESL specification: `https://wesl-lang.dev/spec/README`
- WESL imports: `https://wesl-lang.dev/spec/Imports`
- Slang repository/support: `https://github.com/shader-slang/slang`
- Slang targets: `https://shader-slang.org/slang/user-guide/targets`
- Slang modules: `https://shader-slang.org/slang/user-guide/modules`
- Slang interfaces/generics: `https://docs.shader-slang.org/en/latest/external/slang/docs/user-guide/06-interfaces-generics.html`
- Slang reflection: `https://shader-slang.org/slang/user-guide/reflection.html`

Repository authority governs RunenGPU ownership and runtime contracts.
