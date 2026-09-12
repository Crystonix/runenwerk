---
title: Editor UI Popup, Adornment, And Drop Preview Contract
description: Historical retained-node migration plan for separating menu popups, anchored adornments, dock previews, and radial menus in the retained editor UI.
status: superseded
owner: editor
layer: app
canonical: false
last_reviewed: 2026-09-12
superseded_by:
  - ../../adr/accepted/0009-ui-interaction-formation-v2.md
  - ../implemented/editor-ui-runtime-v2-and-interaction-formation-design.md
related:
  - ../../apps/runenwerk-editor/current-architecture.md
  - ../../apps/runenwerk-editor/execution-priority-checklist.md
  - ../active/editor-ui-workspace-tool-surface-architecture.md
  - ../implemented/editor-ui-runtime-v2-and-interaction-formation-design.md
  - ../../adr/accepted/0009-ui-interaction-formation-v2.md
  - ../implemented/surface-workflow-contract-redesign.md
---

# Editor UI Popup, Adornment, And Drop Preview Contract

## Status

Superseded historical migration plan. Current durable popup, scroll, focus, docking/drop-zone, radial/menu hit-testing, chrome-slot, and viewport-input authority is [ADR 0009](../../adr/accepted/0009-ui-interaction-formation-v2.md) together with the [implemented Interaction V2 design](../implemented/editor-ui-runtime-v2-and-interaction-formation-design.md); current code and tests own exact behavior.

The remaining sections preserve the point-in-time refactor sequence and implementation targets. Their phase/future language does not activate work or create current architecture authority.

## Purpose

The retained UI historically used one `PopupNode` for too many concepts:

- menu panels that need chrome, focus, outside-dismiss, and keyboard behavior;
- small anchored visuals such as tab close buttons;
- viewport-local overlays;
- docking previews that should describe layout intent without reserving real layout unless a target is active.

This causes regressions where a close button behaves like menu chrome, a drop preview changes spacing when it should only preview insertion, or an overlay competes with scrollbars and sibling panel headings.

The long-term fix is to split those concepts in the UI contract instead of continuing to encode them as style variants of one popup node.

This document is supporting contract evidence under ADR 0009. It may name
retained-node migration slices, but Interaction V2 owns the durable popup,
scroll, focus, docking-zone, radial/menu hit-testing, and viewport-input
arbitration architecture.

## Role In Interaction V2

This document names the retained-node concepts that Interaction V2 must form
and enforce. It does not independently authorize runtime behavior changes.

Before any phase below is implemented, the owning Interaction V2 slice must
define:

- the `FormedInteractionModel` concept that distinguishes `MenuPopup`,
  `OverlayAdornment`, `DockDropPreview`, or `RadialMenu`;
- the validation rule that prevents the concept from being encoded as a generic
  popup style variant;
- the retained formation adapter that maps the formed concept into `ui_tree`
  nodes without persisting runtime `WidgetId` or editor command semantics;
- the `ui_runtime` enforcement rule for layer order, clipping, hit testing,
  outside-dismiss, focus return, or pointer capture;
- the editor/app guard that proves viewport input, split borders, scrollbars,
  and sibling panels do not receive events after UI ownership is claimed.

The retained-node concepts in this document map to the WR-025 slice catalog:

| Retained concept | Required Interaction V2 slice |
|---|---|
| `MenuPopup` | `IV2-menu-stack`, `IV2-scroll-ownership`, and `IV2-menu-sizing` |
| `OverlayAdornment` | `IV2-chrome-slots` when used for tab/workspace chrome; otherwise a future explicit adornment slice before durable behavior changes |
| `DockDropPreview` | `IV2-dock-drop-zones` |
| `RadialMenu` | `IV2-menu-stack` plus a later radial hit-testing/focus sub-slice before implementation |

The mappings above are guardrails, not implementation permission. A phase below
can start only when the corresponding slice has validation, formed model output,
retained formation, runtime enforcement, and editor/app guard coverage.

## Target Concepts

### MenuPopup

Owning module:

```text
domain/ui/ui_tree/src/tree/node.rs
```

Responsibilities:

- paints panel chrome;
- owns outside-dismiss and focus-return policy;
- participates in keyboard navigation and command routing;
- uses the menu layer by default;
- may escape parent layout clips when explicitly configured as a menu surface.

Current candidates:

- toolbar menus;
- tab-stack action menus;
- tab-stack surface-type menus;
- viewport options/details menu.

### OverlayAdornment

Owning module:

```text
domain/ui/ui_tree/src/tree/node.rs
```

Responsibilities:

- anchored visual child only;
- transparent by default;
- inherits parent clipping and scroll clipping;
- does not own outside-dismiss, focus trapping, or menu keyboard behavior;
- uses normal content/adornment layer order, below scrollbars and menus.

Current candidates:

- tab close button overlays;
- workspace-profile close button overlays;
- viewport metadata overlays that are visually attached to the viewport surface.

### DockDropPreview

Owning module:

```text
domain/editor/editor_shell/src/workspace/projection.rs
```

Responsibilities:

- represents docking intent only;
- does not mutate workspace state until drop commit;
- does not reserve global or side-column layout space before drop commit;
- distinguishes tab insertion, scoped split insertion, floating-host creation, and invalid/no-target states.

Current candidates:

- tab-strip insertion spacing;
- area split preview;
- group split preview;
- workspace split preview;
- floating-host creation preview;
- candidate cycling for overlapping split scopes on the same side.

### RadialMenu

Owning modules:

```text
domain/ui/ui_tree/src/tree/node.rs
domain/ui/ui_runtime/src/input/
domain/editor/editor_shell/src/commands/
```

Responsibilities:

- polar layout around an anchor or pointer position;
- pointer capture while open;
- keyboard navigation around wedges;
- command routing through the same `RoutedShellAction` and surface-local action path as menus;
- cancellation and focus return consistent with `MenuPopup`.

Radial menus do not require special rendering first. The missing contract is input, focus, command routing, and polar hit testing.

## Refactor Sequence

### Phase 1 - Type Split

Add explicit retained node kinds:

```text
MenuPopup
OverlayAdornment
DockDropPreview
```

Keep compatibility builders only where a staged migration still needs them.
The shell-owned tab close path no longer uses the deleted shell-chrome close
overlay promotion bridge.

Implementation targets:

- `domain/ui/ui_tree/src/tree/node.rs::UiNodeKind`
- `domain/ui/ui_runtime/src/layout/engine.rs::layout_popup`
- `domain/ui/ui_runtime/src/output/build_ui_frame.rs::emit_node`
- `domain/editor/editor_shell/src/composition/build_editor_shell.rs::build_tab_strip`

### Phase 2 - Dismiss And Focus Policy

Move outside-dismiss from ad hoc shell controller functions into a menu contract table.

Implementation targets:

- `apps/runenwerk_editor/src/shell/controller.rs::handle_toolbar_menu_dismiss_event`
- `apps/runenwerk_editor/src/shell/controller.rs::handle_tab_popup_dismiss_event`
- `apps/runenwerk_editor/src/shell/controller.rs::handle_viewport_options_menu_dismiss_event`
- `domain/ui/ui_runtime/src/input/pointer.rs::dispatch_pointer_event`

Exit criteria:

- menus close consistently on outside click;
- adornments never register as dismissable menus;
- clicking a scrollbar, split border, or sibling panel cannot accidentally route through a stale popup target.

### Phase 3 - Drop Preview Contract

Replace reserved-space previews with `DockDropPreview` records projected from workspace structure and active drag state.

Implementation targets:

- `apps/runenwerk_editor/src/shell/state.rs::DockingInteractionVisualState`
- `apps/runenwerk_editor/src/shell/controller.rs::resolve_tab_drop_preview_target`
- `domain/editor/editor_shell/src/composition/build_editor_shell.rs::build_dock_split_preview_overlays`
- `domain/editor/editor_shell/src/workspace/projection.rs::ProjectedTabDropTarget`

Exit criteria:

- dragging a tab does not create persistent side-column spacing;
- all valid scoped split candidates render as visual-only previews;
- the active scoped split candidate receives stronger accent treatment and a scope label;
- the preview is not confused with committed workspace layout.

### Phase 4 - Radial Menu

Implement radial menus as a new menu presentation, not as a one-off viewport gesture.

Implementation targets:

- `domain/ui/ui_tree/src/tree/node.rs::RadialMenuNode`
- `domain/ui/ui_runtime/src/layout/engine.rs::layout_radial_menu`
- `domain/ui/ui_runtime/src/input/pointer.rs::radial_menu_hit_test`
- `domain/editor/editor_shell/src/commands/map_interactions.rs`

Exit criteria:

- pointer hold, keyboard navigation, cancel, and command activation are covered by tests;
- radial entries use normal command ids and surface-local routes;
- viewport radial menus can expose select/move/rotate/scale without returning those controls to the global toolbar.

## Required Test Coverage

Add focused tests before or with each phase:

- frame-order tests for scroll clipping, scrollbar priority, and adornment layer order;
- interaction tests for outside-dismiss and focus return;
- controller tests for split border precedence over tab hit testing;
- dock-drag tests proving previews do not reserve inactive layout space;
- radial-menu hit-testing tests for wedge selection and cancellation.

For visual regressions, add a small screenshot/primitive-order harness for the editor shell after the node split. Existing unit tests catch routing and state, but the recent bugs show that frame-order and clipping need explicit coverage.

<!-- BEGIN RUNENWERK:UI_COMPONENT_PLATFORM:overlay-popup-layering -->
## Component Platform overlay/popup/layering alignment

Overlay, popup, tooltip, context menu, submenu, picker popup, modal readiness, sheet readiness, z-order, anchor placement, collision/clamping, outside dismiss, Escape dismiss, focus return, scroll containment, pointer capture, and diagnostic overlays are reusable Component Platform contracts. Editor-specific popup/adornment/drop-preview behavior consumes these contracts instead of redefining them privately.
<!-- END RUNENWERK:UI_COMPONENT_PLATFORM:overlay-popup-layering -->
