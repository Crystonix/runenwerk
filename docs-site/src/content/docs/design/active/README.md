---
title: Active Designs
description: Architecture designs currently being discussed, implemented, or validated.
status: active
owner: workspace
layer: workspace
canonical: true
last_reviewed: 2026-09-12
---

# Active Designs

Use this folder for designs that are useful but not yet fully accepted or not yet
checked against implementation.

Active does not mean wrong or deprecated. It means the design is still part of
current planning, implementation, or validation work.

## Architecture spine

For Runenwerk-wide architecture, start with:

- [Runenwerk Platform Architecture](../../architecture/runenwerk-platform-architecture.md)

For the whole UI framework architecture, start with:

- [Runenwerk UI Framework Architecture](../../architecture/ui-framework-architecture.md)

Active design docs below are slice-level authorities, proposals, implementation
designs, or product references. They should not each restate the entire framework or
platform architecture. Accepted ADRs and canonical architecture spines win when an
active design contains older conflicting target language.

### Temporary compatibility pointer

- [Runenwerk Domain Workbench North Star Compatibility Pointer](runenwerk-domain-workbench-north-star.md) — noncanonical path-stable pointer used only by two legacy UI design consumers pending the separate RunenUI legacy/adoption cleanup under #205. Do not add new references.

## Current Designs

### Editor Domain

- [Editor Asset Pipeline and Content Workflow Design](editor-asset-pipeline-and-content-workflow-design.md)
- [Editor Procedural Content and Simulation Workflow Plan](editor-procedural-content-and-simulation-workflow-plan.md)
- [Editor UI Workspace Tool Surface Architecture](editor-ui-workspace-tool-surface-architecture.md)

### Engine Runtime

- [Game Runtime, Editor, ECS, Scripting, and Hot Reload Design](engine-game-runtime-editor-ecs-scripting-hot-reload-design.md)

### Gameplay

- [Gameplay Graph ATR IR and ECS Lowering Design](gameplay-graph-atr-ir-and-ecs-lowering-design.md)

### Net

- [ECS Net Replication Boundary](ecs-net-replication-boundary.md)
- [Net Authoritative Replication Protocol](net-authoritative-replication-protocol.md)
- [Net Declarative Replication Authoring](net-declarative-replication-authoring.md)
- [Net Diagnostics Inspection](net-diagnostics-inspection.md)
- [Net Interest Streaming Design](net-interest-streaming-design.md)
- [Net Plugin Runtime Bridge](net-plugin-runtime-bridge.md)
- [Net Prediction Reconciliation Boundary](net-prediction-reconciliation-boundary.md)
- [Net Reconnect History Recovery](net-reconnect-history-recovery.md)
- [Net Transport Lanes Delivery](net-transport-lanes-delivery.md)

### Drawing / Apps

- [Runenwerk Draw Pen-First Radial Tablet UX Design](runenwerk-draw-pen-first-radial-tablet-ux-design.md)

### UI Component Platform

- [UI Component Platform Accessibility / Focus / Inspection Design](ui-component-platform-accessibility-focus-inspection-design.md)
- [UI Component Platform Authoring Kit Design](ui-component-platform-authoring-kit-design.md)
- [UI Component Platform Base Control Packages Design](ui-component-platform-base-control-packages-design.md)
- [UI Component Platform Budget Evidence Design](ui-component-platform-budget-evidence-design.md)
- [UI Component Platform Catalog / Discovery / Inspection Design](ui-component-platform-catalog-discovery-inspection-design.md)
- [UI Component Platform Control Kernel Design](ui-component-platform-control-kernel-design.md)
- [UI Component Platform Executable Interaction Story Design](ui-component-platform-executable-interaction-story-design.md)
- [UI Component Platform Generic Interaction Design](ui-component-platform-generic-interaction-design.md)
- [UI Component Platform Generic Text Design](ui-component-platform-generic-text-design.md)
- [UI Component Platform Input / Gesture / Device Design](ui-component-platform-input-gesture-device-design.md)
- [UI Component Platform Layout / Container / Virtualization Design](ui-component-platform-layout-container-virtualization-design.md)
- [UI Component Platform Node Canvas Design](ui-component-platform-node-canvas-design.md)
- [UI Component Platform Overlay / Popup / Layering Design](ui-component-platform-overlay-popup-layering-design.md)
- [UI Component Platform Ownership Realignment Design](ui-component-platform-ownership-realignment-design.md)
- [UI Component Platform Port Graph Canvas Design](ui-component-platform-port-graph-canvas-design.md)
- [UI Component Platform Progression Tree View Design](ui-component-platform-progression-tree-view-design.md)
- [UI Component Platform Render Surface Output Design](ui-component-platform-render-surface-output-design.md)
- [UI Component Platform Spatial Canvas Design](ui-component-platform-spatial-canvas-design.md)
- [UI Component Platform State Binding / Host Intent Design](ui-component-platform-state-binding-host-intent-design.md)
- [UI Component Platform Story Proof Envelope Design](ui-component-platform-story-proof-envelope-design.md)
- [UI Component Platform Surface2D Design](ui-component-platform-surface2d-design.md)
- [UI Component Platform Text Editing / Editable Text Behavior Design](ui-component-platform-text-editing-design.md)
- [UI Component Platform Theme / State / Style Design](ui-component-platform-theme-state-style-design.md)
- [UI Component Platform Track Surface Design](ui-component-platform-track-surface-design.md)
- [UI Component Platform Transitions / Effects Design](ui-component-platform-transitions-effects-design.md)

### Repository Family Extraction

- [RunenSDF Extraction Design](runensdf-extraction-design.md)
- [RunenSDF Repository Identity Decision](runensdf-repository-identity-decision.md)
- [RunenGPU Architecture Design](runengpu-architecture-design.md)
- [RunenGPU G3 Access and Work Graph Design](runengpu-g3-access-work-graph-design.md)
- [RunenGPU G3R Definite Initialization Semantics Correction](runengpu-g3r-definite-initialization-correction.md)
- [RunenGPU G4 Context, Program, and WGPU Realization Design](runengpu-g4-context-program-realization-design.md)
- [RunenGPU Shader Authoring and Canonical WGSL Artifact Boundary](runengpu-shader-authoring-artifact-boundary.md)
- [RunenGPU G4B Contracts and G4C Delivery Design](runengpu-g4b-contracts-g4c-delivery-design.md)
- [RunenGPU G4C2 Presentation-Surface Binding Boundary](runengpu-g4c2-presentation-surface-binding-boundary.md)
- [RunenGPU G5 Execution Lifecycle Design](runengpu-g5-execution-lifecycle-design.md)
- [RunenGPU Post-G5C Hardening Design](runengpu-post-g5c-hardening-design.md)
- [RunenGPU Phase Requirements and Proof Matrix](runengpu-phase-requirements-proof-matrix.md)
- [RunenGPU and RunenRender Decomposition Execution Plan](runenrender-internal-decomposition-execution-plan.md)

### Domain Authoring And Standalone UI Framework

- [Typed App Program Counter Proof Design](typed-app-program-counter-proof-design.md)
- [UI Framework Runtime Requirements](ui-framework-runtime-requirements-design.md)
- [UI Source Projection And Program Lowering](ui-source-projection-and-program-lowering-design.md)
- [UI Component Composition, Slots, And Authoring](ui-component-composition-slots-and-authoring-design.md)
- [UI Data Binding, Forms, And Effects](ui-data-binding-forms-and-effects-design.md)
- [UI Reactive Runtime And Invalidation](ui-reactive-runtime-and-invalidation-design.md)
- [UI Live Editing And Preview](ui-live-editing-and-preview-design.md)
- [UI Game And World-Space Host Requirements](ui-game-and-worldspace-host-requirements-design.md)
- [UI Accessibility, Internationalization, And Text Conformance](ui-accessibility-internationalization-and-text-conformance-design.md)
- [UI Layout, Style, Theme, And Motion](ui-layout-style-theme-and-motion-design.md)
- [UI Performance, Virtualization, Assets, And Profiling](ui-performance-virtualization-assets-and-profiling-design.md)
- [UI Render Output And Host Renderer Boundary](ui-render-output-and-host-renderer-boundary-design.md)
- [UI Platform Input, Windowing, And OS Integration](ui-platform-input-windowing-and-os-integration-design.md)
- [UI Devtools, Inspection, And Workbench](ui-devtools-inspection-and-workbench-design.md)
- [UI Testing, Conformance, And Proof Matrix](ui-testing-conformance-and-proof-matrix-design.md)
- [UI Package Security, Versioning, And Migration](ui-package-security-versioning-and-migration-design.md)

### Workspace / Cross-Domain

- [Drawing Authoring and Comic Layout Platform Design](drawing-authoring-and-comic-layout-platform-design.md)
- [Drawing Domain Crate Design](drawing-domain-crate-design.md)
- [Material Lab And Material Preview Design](material-lab-and-material-preview-design.md)
- [Native Tablet Input and Latency Contract](native-tablet-input-and-latency-contract.md)
- [Typed App Program And UI Proof Design](typed-app-program-and-ui-proof-design.md)
- [Runenwerk Typed App Composition Plugin Framework Design](runenwerk-typed-app-composition-plugin-framework-design.md)
- [Runenwerk Typed App Composition Plugin Framework Roadmap](runenwerk-typed-app-composition-plugin-framework-implementation-roadmap.md)
- [UI Framework App Integration Direction Review](ui-framework-app-integration-direction-review.md)
- [Live UiPlugin Runtime And Surface Frame Rendering](live-uiplugin-runtime-and-surface-frame-rendering-design.md)
- [Live UiPlugin Runtime Full Cutover Plan](live-uiplugin-runtime-full-cutover-plan.md)
- [Runenwerk UI Platform Capability Roadmap](runenwerk-ui-platform-capability-roadmap.md)
- [Runenwerk UI Story Driven Golden Workflow Design](runenwerk-ui-story-driven-golden-workflow-design.md)
- [UI Program Architecture](ui-program-architecture.md)
- [UI Program Architecture Owner Map](ui-program-architecture-owner-map.md)
- [UI Program Contract Design](ui-program-contract-design.md)
- [UI Program Proof Slice Plan](ui-program-proof-slice-plan.md)
- [UI Runtime Rendering Pipeline Roadmap](ui-runtime-rendering-pipeline-roadmap.md)
- [SDF Prefab Composition System Design](sdf-prefab-composition-system-design.md)
- [SDF Procedural Animation and Animated Models Design](sdf-procedural-animation-and-animated-models-design.md)
- [Semantic Graph IR and Compilation Design](semantic-graph-ir-and-compilation-design.md)
- [Viewport Camera And Projection Contract Platform](viewport-camera-and-projection-contract-platform-design.md)
