---
title: Editor Native Multi-Window Presentation Design
description: Accepted design for Window > New Window, independent OS windows, multi-swapchain rendering, second-monitor workflows, and shared editor-session editing.
status: accepted
owner: editor
layer: app
canonical: true
last_reviewed: 2026-09-12
related_designs:
  - ./render-product-graph-platform-design.md
  - ./product-surface-platform-hardening-design.md
  - ../implemented/editor-workspace-document-mode-panel-architecture.md
  - ../active/editor-ui-workspace-tool-surface-architecture.md
  - ../implemented/render-product-surface-foundation-bundle-design.md
  - ../implemented/workspace-viewport-expression-upgrade-design.md
related_roadmaps:
  - ../../apps/runenwerk-editor/roadmap.md
  - ../../engine/plugins/render/docs/roadmap.md
  - ../../engine/roadmaps/render-final-architecture-migration.md
related:
  - ../../engine/reference/architecture.md
  - ../../engine/reference/plugins/render/architecture.md
---

# Editor Native Multi-Window Presentation Design

## Status

This is the accepted design contract for `PM-RENDER-PG-006`.

It accepts the native multi-window and multi-surface presentation direction
before implementation work starts. It does not mark `PM-RENDER-PG-006`
complete, does not assign `completion_quality`, and does not authorize product
code changes by itself. Implementation still requires `WR-009` to be legal for
the planned slice, `task production:plan -- --milestone PM-RENDER-PG-006 --roadmap WR-009`,
focused validation, closeout evidence, and a rerun of
`task ai:goal -- --track PT-RENDER-PG`.

If an implementation slice needs files outside the legal `WR-009` write scopes,
the roadmap row must be repaired through the normal roadmap workflow before
code changes start.

This design makes native OS windows a first-class editor capability: the user can choose `Window > New Window`, move the new window to another monitor, and keep editing the same project/session through a separate workspace root with its own swapchain, input focus, DPI scale, and render surface lifecycle.

## Locked Decisions

- Native OS windows are real platform windows, not floating hosts inside the
  main editor UI composition.
- `EditorWindowId` is editor-owned, `NativeWindowId` is engine-runtime-owned,
  and `RenderSurfaceId` is render-runtime-owned.
- The app owns the binding between logical editor windows, native runtime
  windows, and render surfaces.
- Editor-domain state must not store native handles, swapchains, backend
  surface objects, or render-runtime private handles.
- Generic engine/runtime APIs must not depend on editor concepts such as
  `EditorWindowId`, workspace roots, panels, tabs, documents, or tool surfaces.
- Render frames, prepare packets, submit packets, presentation, surface loss,
  resize, redraw, and diagnostics are scoped by native window or render surface
  identity.
- Submit and present APIs must make cross-surface presentation mistakes
  structurally difficult; singleton active-surface state is not an acceptable
  final implementation shape.
- Viewport and preview product targets remain viewport/product-owned. Native
  windows decide presentation surface, not product truth.
- The renderer must not infer product selection, source truth, freshness,
  fallback legality, authority, rebuild policy, residency policy, material
  semantics, or document semantics.
- Each editor window has independent focus, input capture, workspace root,
  active panel/tab, UI frame, DPI scale, cursor state, redraw state, and surface
  lifecycle while sharing the authoritative project/session where appropriate.
- Secondary-window close policy must not close the project unless it is the
  last project window and the app-level quit policy allows it.
- ADR review is required before changing global runtime/window ownership,
  platform event ownership, or render-surface ownership policy beyond the
  accepted boundary.

## Goal

Support this workflow:

```text
Window > New Window
  -> create native OS window
  -> create logical editor window record
  -> attach or create workspace root layout
  -> prepare UI and render products for that window
  -> render through that window's swapchain
  -> route input and commands through that window's focus context
```

The new window is not a fake floating panel inside the main window. It is an independent native window that can be placed on a second monitor and can edit the same project through the same authoritative editor session.

## Non-Negotiable Outcomes

- `Window > New Window` creates a real OS window.
- Each OS window has its own swapchain/surface, DPI scale, size, cursor, redraw state, and surface-loss lifecycle.
- All windows share the same editor session, project state, documents, command bus, undo/redo model, and runtime where appropriate.
- Each window has its own workspace root, focus state, input capture, active panel/tab, and UI frame.
- Closing a secondary window removes only that window's host/layout state; it does not close the project unless the last project window is closed and the app policy says to quit.
- Moving a tab, panel, viewport, or workspace area into a new window preserves explicit `PanelInstanceId`, `ToolSurfaceInstanceId`, document context, and provider/session state according to the move policy.
- Render submission is surface-scoped. Rendering one window cannot accidentally present into another window's swapchain.
- Viewport/product targets remain viewport/product-owned, not window-owned, but presentation into a window uses that window's UI frame and scale.

## Current Constraints

The accepted render product graph foundation, contract ergonomics,
feature-owned contribution path, execution compiler maturity, and
product-surface hardening provide the product-surface groundwork. PM-006 starts
after that foundation and must not reopen those completed ownership decisions.

Remaining runtime and presentation state is still singleton-shaped:

- `engine/src/runtime/window.rs::WindowState` represents one window.
- `engine/src/runtime/platform.rs::PlatformEvent` is not window-scoped.
- render surface handling assumes one active surface target.
- editor workspace state supports tab stacks, split resizing, and floating hosts, but floating hosts are still inside the main UI composition, not native OS windows.

Those are sufficient for a single-window editor and internal floating layouts, but not for `Window > New Window`.

## PM-006 Scope

PM-006 is the production-track milestone that turns accepted design into
multi-surface presentation capability. Its implementation must stay bounded to
native window and render-surface presentation mechanics.

In scope:

- logical editor window identity and window-local workspace focus/routing;
- runtime-native window registry and window-scoped platform events;
- render surface registry keyed by native window or render surface identity;
- surface-scoped frame prepare, submit, present, resize, surface-loss recovery,
  and diagnostics;
- app-owned binding from `EditorWindowId` to `NativeWindowId` and
  `RenderSurfaceId`;
- `Window > New Window` and close/focus policy needed to prove two native
  editor windows can render and edit the same session;
- focused viewport/product proof that presentation surface identity is correct.

Out of scope:

- render fragments, fragment hot reload, or data-driven fragment compiler
  maturity from `PM-RENDER-PG-007`;
- production-readiness inspection, capture/replay policy, and broad diagnostic
  hardening from `PM-RENDER-PG-008`;
- product truth, product selection, freshness, fallback legality, authority,
  rebuild policy, residency policy, or material lowering;
- broad product-surface hardening beyond the already completed PM-005 contract;
- renderer-owned editor/window policy shortcuts;
- renderer-private handles passed to editor viewport or preview producers;
- unrelated editor command, persistence, or workspace redesigns that are not
  required for native multi-window presentation.

## Ownership Boundaries

### Editor Domain

Owning modules:

```text
domain/editor/editor_shell/src/workspace/window.rs
domain/editor/editor_shell/src/workspace/state.rs
domain/editor/editor_shell/src/workspace/reducer.rs
domain/editor/editor_shell/src/commands/shell_command.rs
```

Responsibilities:

- define logical `EditorWindowId`;
- define which workspace root belongs to which editor window;
- define window-local active panel, focus, and routing context;
- define commands for new window, close window, move tab/panel to window, duplicate workspace into window, and focus window;
- keep document/session semantics out of window records.

The editor domain does not own native OS handles or swapchains.

### Editor App

Owning modules:

```text
apps/runenwerk_editor/src/shell/state.rs
apps/runenwerk_editor/src/shell/dispatch_shell_command.rs
apps/runenwerk_editor/src/shell/controller.rs
apps/runenwerk_editor/src/persistence/workspace_layout.rs
```

Responsibilities:

- allocate logical editor windows;
- request native windows from engine runtime;
- persist multi-window workspace placement/layout where appropriate;
- route each window's frame model through provider registry context;
- enforce app policy for closing last window, unsaved documents, and project quit.

### Engine Runtime

Owning modules:

```text
engine/src/runtime/window.rs
engine/src/runtime/platform.rs
engine/src/app/
engine/src/runtime/
```

Responsibilities:

- maintain `WindowStateRegistryResource` keyed by engine-level native window id;
- normalize platform events with a window id;
- manage native window creation/destruction requests;
- track per-window size, scale factor, cursor, close request, and redraw request;
- expose runtime events without editor-specific meaning.

### Render Runtime

Owning modules:

```text
engine/src/plugins/render/frame/packet.rs
engine/src/plugins/render/frame/view.rs
engine/src/plugins/render/runtime/frame_prepare.rs
engine/src/plugins/render/runtime/frame_submit.rs
engine/src/plugins/render/backend/surface.rs
engine/src/plugins/render/renderer/mod.rs
```

Responsibilities:

- manage one render surface/swapchain state per native window;
- prepare render packets per surface/window;
- execute and present each surface-scoped frame independently;
- recover from surface loss per window;
- expose surface diagnostics per window.

The render product surface foundation bundle provides the needed prepared render view and product-surface groundwork. Multi-window presentation adds native swapchain ownership and surface-scoped submit/present.

## Core Model

### Logical Editor Window

```rust
pub struct EditorWindowRecord {
    pub editor_window_id: EditorWindowId,
    pub workspace_root_id: WorkspaceRootId,
    pub title_policy: EditorWindowTitlePolicy,
    pub placement: EditorWindowPlacement,
    pub close_policy: EditorWindowClosePolicy,
}
```

This record belongs to editor shell/app state. It references workspace identity and window-local UI state, not native handles.

### Native Runtime Window

```rust
pub struct RuntimeWindowRecord {
    pub native_window_id: NativeWindowId,
    pub title: String,
    pub size_px: (u32, u32),
    pub scale_factor: f64,
    pub redraw_requested: bool,
    pub close_requested: bool,
}
```

This record belongs to engine runtime. It has no editor shell concepts.

### Window Binding

The app binds logical editor windows to native runtime windows:

```text
EditorWindowId -> NativeWindowId -> RenderSurfaceId
```

The binding is app-owned because it composes editor shell state, runtime window state, and render surface state.

## User-Facing Commands

Add shell/app commands:

- `OpenNewEditorWindow`
- `DuplicateCurrentWorkspaceToNewWindow`
- `MoveActiveTabToNewWindow`
- `MovePanelToNewWindow`
- `CloseEditorWindow`
- `FocusEditorWindow`
- `MoveEditorWindowToMonitor` when monitor enumeration is exposed

Initial `Window > New Window` should create a new logical window with either:

- the same workspace profile and a fresh default layout; or
- a duplicated current workspace layout, if the command variant requests duplication.

The default should be fresh layout for predictable identity. Duplication should be explicit because copied viewports/panels may carry camera/presentation state.

## Rendering Flow

Each native window produces one surface-scoped render packet:

```text
NativeWindowId
  -> RenderSurfaceId
  -> PreparedRenderSurfaceFrame
  -> prepared UI frame for EditorWindowId
  -> prepared product views referenced by that UI frame
  -> submit/present into that surface only
```

Window-local UI and app/product surfaces can share underlying product targets when explicitly selected, but normal viewport products remain per viewport instance.

The surface-scoped render packet carries presentation identity. Product-surface
manifests and prepared render requests carry product/view identity. The bridge
between them is explicit: a window-local UI frame chooses which prepared
products to present into that window's render surface without changing the
ownership or freshness rules for those products.

## Input And Focus

Platform events must include a native window id:

```rust
pub struct PlatformWindowEvent {
    pub native_window_id: NativeWindowId,
    pub event: PlatformEventKind,
}
```

The app maps `NativeWindowId` to `EditorWindowId`, then routes:

- keyboard focus;
- pointer hover/capture;
- drag/drop;
- tab/panel commands;
- viewport-local input;
- command palette context.

Focus is per window. A capture in one window must not consume pointer events in another.

## Lifecycle

### Create Window

1. User chooses `Window > New Window`.
2. App creates `EditorWindowRecord`.
3. App requests native runtime window creation.
4. Runtime creates native window and `RuntimeWindowRecord`.
5. Render creates surface/swapchain state for that native window.
6. Shell projects a UI frame for the new editor window.

### Close Window

1. Runtime receives native close event.
2. App maps native id to editor window id.
3. App checks unsaved/project close policy.
4. Secondary window closes by removing its logical workspace root binding and native window.
5. Last-window close follows application quit policy.

### Surface Loss Or Resize

Surface loss, resize, and scale-factor changes are scoped to one native window. Other windows continue rendering. Product target dimensions update only for viewports or surfaces affected by the window-local layout change.

## Implementation Phases

Each phase must be implemented through one legal bounded roadmap slice at a
time. The phases are dependency order, not permission to do broad code changes
without `task production:plan`, roadmap legality, validation, closeout evidence,
and `task ai:goal` reruns.

### MW1 - Logical Editor Window Domain

Change:

- `domain/editor/editor_shell/src/workspace/window.rs`
- `domain/editor/editor_shell/src/workspace/state.rs`
- `domain/editor/editor_shell/src/workspace/reducer.rs`
- `domain/editor/editor_shell/src/commands/shell_command.rs`

Exit criteria:

- shell state can represent multiple logical editor windows;
- commands can create, close, focus, and move/duplicate workspace content across logical windows;
- no native window handles enter domain state.

### MW2 - Runtime Window Registry

Change:

- `engine/src/runtime/window.rs`
- `engine/src/runtime/platform.rs`
- `engine/src/app/`
- engine runtime tests

Exit criteria:

- runtime has a registry of native windows;
- platform events are window-scoped;
- redraw, close, cursor, resize, and scale factor are per window.

### MW3 - Multi-Surface Render Runtime

Change:

- `engine/src/plugins/render/backend/surface.rs`
- `engine/src/plugins/render/frame/packet.rs`
- `engine/src/plugins/render/runtime/frame_prepare.rs`
- `engine/src/plugins/render/runtime/frame_submit.rs`
- `engine/src/plugins/render/renderer/mod.rs`

Exit criteria:

- render runtime maintains one surface/swapchain state per native window;
- prepared/submitted frames are surface-scoped;
- surface loss and resize are isolated by render surface id.

### MW4 - App Binding And Window Menu

Change:

- `apps/runenwerk_editor/src/shell/state.rs`
- `apps/runenwerk_editor/src/shell/dispatch_shell_command.rs`
- `apps/runenwerk_editor/src/shell/controller.rs`
- `domain/editor/editor_shell/src/composition/build_toolbar.rs`

Exit criteria:

- `Window > New Window` creates a real native window;
- new window renders editor shell UI;
- close/focus commands are routed through shell/app state.

### MW5 - Cross-Window Workspace Operations

Change:

- `domain/editor/editor_shell/src/workspace/reducer.rs`
- `apps/runenwerk_editor/src/shell/state.rs`
- `apps/runenwerk_editor/src/persistence/workspace_layout.rs`

Exit criteria:

- active tab/panel can move to a new window;
- workspace layout can duplicate into a new window;
- provider/session state follows the selected move/copy policy.

### MW6 - Multi-Window Viewport And Product Proof

Change:

- `apps/runenwerk_editor/src/runtime/viewport/*`
- `engine/src/plugins/render/*`
- viewport GPU smoke tests

Exit criteria:

- one viewport in each of two native windows renders independently;
- input/camera/picking are routed to the correct window and viewport;
- resizing one window does not corrupt the other window's render products.

## Validation

```text
task production:plan -- --milestone PM-RENDER-PG-006 --roadmap WR-009
cargo test -p editor_shell multi_window
cargo test -p engine window
cargo test -p engine --test render_multi_surface
cargo test -p runenwerk_editor window
cargo test -p runenwerk_editor --test viewport_architecture_guards
RUNENWERK_ENABLE_GPU_SMOKE=1 RUNENWERK_ENABLE_MACOS_MAIN_THREAD_GPU_SMOKE=1 cargo test -p runenwerk_editor --test viewport_gpu_truth_smoke -- --ignored
task docs:validate
task roadmap:render
task roadmap:validate
task roadmap:check
task production:render
task production:validate
task production:check
task planning:validate
task ai:goal -- --track PT-RENDER-PG
```

GPU smoke is required for final runtime proof when the host environment can run
it. If unavailable, closeout evidence must state that limitation and include
the strongest deterministic surface-scoping tests that ran.

## Final Acceptance Criteria

- `Window > New Window` opens a real OS window.
- Two editor windows can edit the same project/session.
- Each window has independent workspace focus, UI frame, input capture, render surface, swapchain, and DPI scale.
- Moving a tab/panel/viewport to another window preserves explicit identity and command routing.
- Closing a secondary window does not close the project.
- Render surface loss, resize, and presentation are isolated per window.
- Multi-window support does not introduce editor concepts into generic render/runtime APIs.

## Relationship To Render Product Surfaces

The render product surface foundation bundle is a prerequisite for robust viewport and preview products across windows. Native multi-window presentation adds OS window and swapchain multiplicity on top of that foundation.

Product surfaces answer: "what texture/product is this viewport or preview presenting?"

Native multi-window answers: "which OS window and swapchain presents this editor workspace UI?"

The two contracts are intentionally separate. PM-006 may route an already
prepared product surface into a window-local UI frame and render surface, but it
must not make native windows the owner of product truth or turn render surfaces
into product selection authorities.
