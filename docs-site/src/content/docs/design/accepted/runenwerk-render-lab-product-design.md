---
title: Runenwerk Render Lab Product Design
description: Accepted product architecture for Render Lab as a Runenwerk-owned renderer conformance, inspection, visual-proof, comparison, and stress consumer over RunenRender and RunenGPU.
status: accepted
owner: workspace
layer: product / app / renderer-integration
canonical: true
last_reviewed: 2026-09-11
related_adrs:
  - ../../adr/accepted/0017-cross-authority-consistency-and-graph-semantics.md
  - ../../adr/accepted/0018-semantic-federation-and-physical-realization.md
  - ../../adr/accepted/0019-batteries-included-application-composition.md
  - ../../adr/accepted/0021-ratify-runenrender-semantic-rendering-architecture.md
related_designs:
  - ./runenrender-decomposition-design.md
  - ../active/runenrender-internal-decomposition-execution-plan.md
  - ./ui-designer-workbench-product-design.md
related_roadmaps:
  - ../../workspace/planning/roadmap.md
---

# Runenwerk Render Lab Product Design

## Status

This is the accepted product architecture for **Runenwerk Render Lab**.

It defines durable product ownership, boundaries, proof stages, and acceptance laws. It does not by itself authorize Rust/Cargo implementation work. Live activation, implementation slices, and exact-head evidence remain owned by GitHub issues and pull requests under the canonical roadmap and Runenwerk governance.

RL0 acceptance was censused on accepted `main` after #541 / PR #562. The recorded source state is provenance only; future implementation must re-census exact current `main` rather than reuse an old SHA as authority.

## Purpose

RunenRender now has a normalized semantic spine and a permanent public-RunenGPU execution proof, but its strongest evidence is still proof-local. Runenwerk needs a maintained product consumer that turns those contracts into visible, inspectable, repeatable workloads without moving renderer semantics into product code.

Render Lab exists to provide that consumer.

It is the product-level place for:

- controlled rendering scenarios;
- meaningful human-viewable renderer proof;
- renderer result and diagnostic inspection through legitimate public contracts;
- method/representation comparison;
- deterministic and stochastic conformance evidence appropriate to each method;
- bounded scale/stress characterization;
- headless regression evidence;
- interactive renderer experiments;
- concrete pressure on public RunenRender/RunenGPU ergonomics.

It is not a second renderer architecture.

## Product decision

Render Lab is a **distinct Runenwerk product identity**.

The ownership stack is:

```text
Runenwerk Render Lab
    scenarios / experiment choice / execution policy /
    comparison policy / evidence presentation / artifacts
        |
        v
RunenRender
    renderer scene / request / method / planning /
    admission / result semantics
        |
        v
RunenGPU
    generic physical GPU execution
```

Source domains and peer frameworks keep their own semantic authority. Runenwerk adapters may translate source meaning into RunenRender meaning, but Render Lab does not become a source-domain ontology.

Render Lab is not:

```text
RunenRender framework semantics
RunenGPU framework semantics
an Editor document model
an Editor viewport mode
Material Lab
UI Designer
a generic scene editor
a universal renderer-test DSL
a generic Workbench reflection protocol
a path-tracing framework
a global conformance truth authority
```

## Post-#541 census and current implementation truth

The RL0 activation census establishes the following current facts.

### Runenwerk ownership is correct

Runenwerk is the integration/product repository. `ARCHITECTURE.md` and the platform architecture assign application lifecycle, windows/event-loop policy, product policy, diagnostics presentation, applications, and cross-framework adapters to Runenwerk while RunenRender owns semantic rendering and RunenGPU owns generic GPU execution.

That is sufficient architectural authority for a Runenwerk-owned Render Lab. No new ADR is required merely to define this product while those ownership rules remain unchanged.

### Existing Workbench hosts are implementation evidence, not Render Lab authority

Current focused products demonstrate several useful host shapes:

- `RunenwerkWorkbenchHost` has editor-owned `FullEditor`, `MaterialLab`, `UiDesigner`, headless, constrained, and custom compositions;
- runtime app composition has `FullEditor`, `MaterialLab`, `UiDesigner`, and `UiGallery` modes;
- Material Lab and UI Designer prove focused standalone/headless product composition can exist;
- UI Gallery proves a focused app path can exist without installing the editor host resource.

These are useful implementation precedents only. Their editor-specific packages and types do not make `apps/runenwerk_editor` or `domain/editor` the semantic owner of Render Lab.

### The founding renderer is not yet a product surface

The accepted R6/R7 proof establishes the semantic spine and public RunenGPU execution, but the founding realization/evaluator remains deliberately proof-local/test-only. The founding perspective proof also uses a `2 x 2` sample lattice: sufficient for semantic/numeric proof, not for a meaningful human-viewable renderer demonstration.

RL1 therefore must not import or expose test-only R6 proof machinery merely to make a product quickly. It must exercise a legitimate product-accessible RunenRender path.

### `RenderResult` is semantic authority but is not yet a public product API

After #541, RunenRender has one private bounded `RenderResult` semantic authority. It retains immutable semantic provenance without physical bindings, GPU environment facts, readback IDs, or output bytes.

The module and result types are currently crate-private. Render Lab must not reach through that boundary or create a mirrored product copy of private renderer state.

If an RL1 consumer genuinely requires result facts that current public RunenRender contracts do not expose, that is concrete framework-surface pressure. Open the smallest owner-correct RunenRender issue and expose only the required stable semantic/inspection contract. Do not solve it inside product code and do not pre-author a generic reflection API.

### #552 remains deferred

The deterministic founding renderer does not require a generalized stochastic finite-evaluation model. The accepted sequence therefore keeps #552 deferred until after deterministic visual and real-time product proof.

## Normalized product model

Render Lab preserves six distinct roles:

```text
SCENARIO
    controlled product-owned situation to exercise

RENDER INTENT
    RunenRender scene/request semantics formed for that scenario

METHOD / EXPERIMENT CHOICE
    compatible method, representation, comparison, or trial choice

EXECUTION POLICY
    host/product constraints such as repetition, mode, capture, and budget

OBSERVED EVIDENCE
    semantic outcomes, diagnostics, values, oracle/comparison facts, and cost observations

PRESENTATION / ARTIFACT
    UI views, images, reports, recordings, and persisted evidence
```

These roles must not collapse into one generic `RenderLabConfig` authority.

The product flow is conceptually:

```text
Scenario
    -> form renderer intent
    -> choose legal experiment
    -> execute through RunenRender -> RunenGPU
    -> observe legitimate evidence
    -> classify according to the scenario contract
    -> present and/or retain product evidence
```

This is a product workflow, not a universal engine runtime pipeline.

## Scenario ownership

A Render Lab scenario is product-owned controlled intent.

It may define only what is needed to establish a repeatable consumer, for example:

```text
scenario identity / revision
fixture or source setup
expected observation/output intent
legal method families
comparison/oracle policy
controlled mutation sequence
bounded stress parameters
expected supported/unsupported behavior
```

A scenario must not become:

- a second `RenderSceneSnapshot`;
- a generic world/scene authoring format;
- persisted RunenRender scene authority;
- a source-asset ontology;
- a universal renderer DSL;
- a store of mirrored private RunenRender state.

Scenario-to-renderer formation happens through explicit product/adaptor code and normal RunenRender contracts.

The initial catalogue should reuse accepted founding semantics where practical: analytic surfaces, a field/SDF-backed surface, minimum diffuse appearance, controlled directional illumination, perspective observation, spectral radiance, forward depth/distance, object identity, and a scalar radiance probe.

## Method and experiment law

Render Lab may deliberately select or compare methods because method choice can be part of a **product experiment** even when it is not ordinary requested renderer meaning.

Preserve:

```text
same requested semantic target
!= same RenderMethod
```

Useful experiment classes include:

```text
method A vs method B on the same semantic target
CPU/reference oracle vs RunenRender result
clean/full vs incremental execution
representation A vs representation B when both are legal
cold vs retained/warm execution when retained state exists
repeated stochastic evaluations under one declared evaluation contract
```

Every comparison declares the equality, tolerance, or statistical criterion that makes sense for that scenario. Render Lab must not standardize one universal pixel-equality or quality metric.

## Evidence model

Render Lab owns **product evidence policy**, not renderer truth.

A scenario run may correlate only legitimate observable facts, such as:

```text
public RunenRender request/result/diagnostic facts
public plan/admission/inspection evidence when exposed
public RunenGPU execution/status/capability evidence
output/readback/presentation values through legitimate paths
CPU/reference-oracle values
scenario-specific expected facts
product-owned timing/cost observations
```

A product run may be classified `pass`, `fail`, or `inconclusive` according to that scenario's declared evidence contract. That classification is product evidence. It is not a framework-wide truth certificate and does not mutate RunenRender or RunenGPU authority.

Output bytes, encoded images, screenshots, videos, manifests, and reports remain product/presentation artifacts rather than `RenderResult` semantics.

## Interactive and headless parity

Render Lab targets two host modes over the same underlying scenario/run/evidence model:

```text
headless
    execute named scenarios
    emit deterministic-structure machine-readable evidence
    retain review artifacts when requested

interactive
    choose scenario/experiment
    inspect output/evidence/diagnostics
    interact with the renderer workload
```

These modes must not construct semantically different renderer paths for the same scenario merely because one has a window.

The exact package, binary, and host placement is deliberately **not frozen by RL0**. At each implementation activation, inspect current app composition and choose the smallest owner-correct form.

Allowed directions include a dedicated app package, a dedicated executable composition, or reuse of genuinely app-neutral host/UI machinery. Convenience does not justify making the editor shell the semantic owner or extracting a generic `RunenLab` host framework from one consumer.

## Public-surface rule

Render Lab is an adversarial **public-contract consumer**.

It must not:

- import test-only R6 modules;
- depend on crate-private RunenRender state;
- reach through RunenRender into private RunenGPU/WGPU state;
- copy private framework state into a long-lived product mirror;
- add privileged friend APIs only for the Lab.

When a scenario cannot be implemented honestly through current public contracts:

```text
scenario pressure
    -> exact owner census
    -> separate bounded framework issue
    -> smallest stable owner-correct contract correction
    -> scenario proves the corrected public path
```

This is a feature of Render Lab: it reveals whether the candidate standalone RunenRender boundary is actually usable.

## RL1 — first meaningful visual proof

RL1 is the first implementation slice after this design and a fresh exact-main census.

Its goal is not a window. Its goal is the **first meaningful human-viewable render produced through the new semantic spine**.

RL1 must provide one named deterministic founding-direct scenario that:

- uses a legitimate product-accessible RunenRender -> public RunenGPU path;
- renders at a genuinely human-inspectable useful resolution rather than the proof-only `2 x 2` lattice;
- retains one product-owned visual artifact;
- retains structured scenario/semantic/oracle/diagnostic evidence;
- proves the field-backed representation still participates where the founding scenario declares it;
- does not use private/test-only framework reach-through.

A review bundle is conceptually:

```text
render-lab/founding-direct/
    radiance.<product-owned image format>
    evidence.<structured product format>
    [depth visualization]
    [identity visualization]
```

The exact file formats and resolution are activation-time product decisions, not RunenRender semantics.

The founding radiance contract is spectral, not generic RGB. Product visualization must not silently relabel one wavelength as RGB color. A grayscale or false-color inspection mapping is acceptable only when explicitly product-owned and labeled as visualization.

RL1 may be driven headlessly. Interactive shell breadth is not an RL1 acceptance requirement.

If RL1 demonstrates that a product-accessible deterministic evaluator, result inspection surface, or output transport contract is missing, stop and open the smallest framework-owned correction rather than importing proof code.

## RL2 — deterministic real-time product proof

RL2 is the first real-time native Render Lab.

For the founding deterministic scenario, normal behavior is:

```text
continuous mouse input
    -> camera intent changes
    -> appropriate renderer request/state change
    -> RunenRender evaluation
    -> RunenGPU execution
    -> newly presented frame
```

Founding interaction scope is:

```text
orbit
pan
zoom
```

The reference product target is:

```text
60 presented frames per second
<= 16.67 ms nominal frame interval
continuous mouse-driven interaction
image updates while dragging, not only after release
no intentional synchronous CPU readback in the interactive presentation path
```

Acceptance records exact hardware/device, backend, output extent, scenario, RenderMethod, build profile, and measurement procedure. This is a declared product/reference-workload target, not a universal RunenRender performance guarantee.

RL2 review evidence includes one screenshot of the actual product surface, one short interaction capture, and one recorded reference frame-rate/frame-time measurement. Those are review artifacts, not authorization for a generic media-capture subsystem.

## Product surface

The first interactive surface should expose only what the initial scenarios require:

```text
Scenario
    scenario / variant selection

Render
    observation/output summary
    legal method/experiment choice
    execute/re-execute

Output
    presented product visualization

Evidence
    available public renderer provenance/outcome
    oracle/comparison result
    structured diagnostics

Execution
    bounded timing/work/capability observations when legitimately available
```

Do not build a scene editor, node graph, timeline, asset browser, material authoring suite, or general profiler merely to make Render Lab look complete.

## Artifact ownership

Runenwerk may persist Render Lab evidence because capture/encoding/persistence policy belongs above RunenRender.

Potential product artifacts include:

```text
scenario/run manifest
structured diagnostics/evidence
reference/comparison summary
captured visualization or numeric output
performance observation
review screenshot / short interaction capture
```

Artifacts are evidence about a run. They are not a framework persistence schema and are not required for ordinary RunenRender use.

## Stress and scale role

Render Lab is the preferred product for controlled renderer scale characterization once real scenarios exist.

Possible product-owned dimensions include:

```text
scene object count
representation count
incremental change size
observation/output count
emitter/population count
representation refinement/detail
retained-state pressure
output size
repeated invocation count
```

Stress scenarios report measured behavior. They do not declare aspirational universal limits or encode product presets such as `Low/High/Ultra` into renderer semantics.

## Sequencing and proof ladder

The accepted current proof ladder is:

```text
#541 complete semantic RenderResult/readback proof
    -> exact-main RunenRender/product census
    -> RL0 accepted Render Lab product design          [this document]
    -> RL1 meaningful deterministic visual artifact
    -> smallest R7/public-surface correction only if RL1 proves one necessary
    -> RL2 deterministic real-time interactive Render Lab
    -> fresh exact-main census
    -> #552 requested target vs finite stochastic evaluation normalization
    -> P1 stateless multi-bounce path-tracing RenderMethod
    -> direct-vs-path-tracer proof through the same semantic spine
    -> explicit RunenRender extraction-readiness review
    -> RL3 interactive path-tracing/material-view scenario
    -> adaptive-refinement consumer
    -> only then any retained evaluation/session/history semantics demanded by evidence
    -> broader Render Lab scale/conformance qualification
    -> R8
    -> RX according to the then-accepted extraction decision
```

The durable canonical phase order remains:

```text
R7 -> R8 -> RX
```

The extraction-readiness review is a decision checkpoint, not automatic permission to reorder phases.

## #552 and P1 boundary

#552 remains deliberately after RL2.

Deterministic product proof should first establish:

- meaningful visible output;
- legitimate product presentation;
- input/camera mutation;
- request/state invalidation behavior;
- continuous frame delivery;
- public-contract usability.

Only then should #552 normalize requested target versus finite stochastic evaluation for the first materially different stochastic method.

P1 is a **RunenRender method**, not Render Lab algorithm code. Its first proof is stateless, headless, and correctness-oriented. Raw noisy finite stochastic output may be useful diagnostic evidence there; it does not define the normal interactive product experience.

## RL3 and adaptive refinement

RL3 later adds an interactive path-tracing/material-view scenario with controlled geometry/material/lighting, direct-vs-path-trace comparison, camera interaction, and inspectable evaluation evidence.

The later adaptive-refinement consumer supplies concrete pressure for retained evaluation state, reconstruction, continuation, convergence, history, or `RenderSession` semantics.

The required UX law is:

```text
first presented frame = coherent / usable
additional useful work = improves detail and/or evaluation fidelity
scene/camera/material change = invalidates only incompatible evidence
```

Do not pre-author session/progressive machinery merely because that consumer is planned.

## Extraction-readiness checkpoint

After P1 and the direct-vs-path-tracer proof, explicitly review whether RunenRender is ready for a clean standalone cutover.

Check at minimum:

```text
real maintained Runenwerk consumer uses candidate public RunenRender surface
two materially different RenderMethods share one semantic spine
public RunenGPU only; no WGPU/private reach-through
ordinary API is usable without proof-private constructors
standalone conformance can be formed
consumer migration + predecessor deletion can be one clean cutover
remaining R7/R8 work is maturity/scale rather than unresolved ownership/API repair
```

If evidence supports earlier extraction, propose the smallest canonical phase-order amendment separately. If not, continue proving the boundary internally.

## Adjacent lab boundary

Render Lab remains renderer-focused.

```text
Render Lab
    RunenRender methods / representations / results / rendering conformance and stress

Reaction-Diffusion Lab
    simulation/field semantics + realtime RunenGPU compute + visualization

Cellular-Automata Lab
    discrete simulation semantics + realtime RunenGPU compute + visualization

World / Terrain Lab
    RunenSDF + RunenSpatial + Runenwerk world policy + RunenRender integration
```

Repeated product-neutral host/input/inspection machinery may be extracted only after structurally different real consumers prove the repeated neutral primitive. RL0 does not authorize a generic `RunenLab` framework.

## Explicit non-goals

This design does not authorize:

- RunenRender semantic changes merely to populate UI;
- RunenGPU contract changes without separate owner pressure;
- publicizing all private RunenRender proof state;
- importing R6 test/proof modules into product code;
- a universal Workbench inspection ABI;
- a generic scenario/scene/fixture DSL;
- an Editor compatibility layer for Render Lab;
- general scene/material/texture authoring;
- gameplay/runtime-preview integration;
- world/chunk streaming;
- broad profiler/benchmark infrastructure;
- shader-editor or hot-reload product features;
- render-farm/distributed orchestration;
- generic image/video/EXR pipeline design;
- hardware-ray-tracing work merely for Lab completeness;
- sessions/history/reconstruction/denoising without a concrete consumer;
- path tracing in the product layer;
- claiming R8 complete from one product;
- extracting generic Lab infrastructure from one proof.

## Validation and acceptance laws

Each implementation slice must re-resolve exact current authority and prove only its own boundary.

RL0 acceptance requires:

1. post-#541 exact-main RunenRender/product census;
2. canonical Runenwerk product ownership;
3. RunenRender and RunenGPU authority preserved;
4. current editor-host machinery treated as implementation evidence, not authority;
5. current private/test-only RunenRender seams identified rather than silently bypassed;
6. normalized scenario/intent/experiment/execution/evidence/presentation separation;
7. headless/interactive parity law;
8. RL1/RL2 proof ladder and #552 deferral made canonical;
9. no Rust/Cargo implementation in RL0;
10. documentation build, repository validation, and exact-head CI green for the unchanged reviewed documentation head.

Later slices must add focused product evidence in addition to the normal repository baseline.

## Cold-start path and live work

Cold-start order for Render Lab work:

1. this document — product mission, ownership, normalized model, and proof ladder;
2. [Runenwerk Platform Architecture](../../architecture/runenwerk-platform-architecture.md) — product/Workbench ownership;
3. [RunenRender Architecture and Decomposition Design](./runenrender-decomposition-design.md) — renderer semantic authority;
4. [RunenRender internal execution plan](../active/runenrender-internal-decomposition-execution-plan.md) and the [canonical roadmap](../../workspace/planning/roadmap.md) — durable framework sequence;
5. the current owning GitHub issue/PR — live activation, implementation scope, blockers, and validation evidence.

Historical issue comments and prior editor renderer designs are evidence only. They do not override this accepted design or current code truth.

## Final position

Render Lab is the maintained Runenwerk consumer that makes the RunenRender boundary prove itself in product reality:

```text
product owns the experiment
renderer owns rendering meaning
GPU owns physical execution
evidence crosses boundaries through explicit contracts
visual proof precedes stochastic generalization
public-surface pressure is fixed at the owning boundary
shared infrastructure is extracted only after repeated proof
```

That is the accepted RL0 architecture.