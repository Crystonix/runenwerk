---
title: Editor Shell Menu And Tab Chrome Polish Design
description: Active design for popup contrast, scrollable menus, viewport statistics, submenu anchoring, and left-side close/active indicators.
status: active
owner: domain/editor/editor_shell
layer: editor-ui
canonical: true
last_reviewed: 2026-05-15
related_designs:
  - ../implemented/editor-ui-runtime-v2-and-interaction-formation-design.md
  - ./editor-ui-popup-adornment-drop-preview-contract.md
  - ../implemented/editor-self-authoring-and-final-ui-design.md
related_adrs:
  - ../../adr/accepted/0009-ui-interaction-formation-v2.md
related_roadmaps:
  - ../../apps/runenwerk-editor/roadmap.md
  - ../../domain/ui/roadmap.md
---

# Editor Shell Menu And Tab Chrome Polish Design

## Status

Active supporting evidence. This is retained UI polish, not a new UI execution
strategy and not the long-term interaction contract owner.

This document is subordinate to ADR 0009 and
`editor-ui-runtime-v2-and-interaction-formation-design.md`. Keep it active while
the immediate polish work is useful, but do not treat it as the long-term owner
for popup stack, scroll ownership, focus return, chrome slots, docking/drop-zone
precedence, menu sizing, or status overflow contracts.

Do not start durable popup, scroll, focus, docking-zone, menu-sizing,
status-overflow, or viewport-input policy from this document before the
corresponding WR-025 Interaction V2 contract slice is defined. Work from this
document is valid only as a retained-UI implementation slice that consumes
Interaction V2 contracts, or as explicitly bounded compatibility evidence.

## WR-024 Entry Boundary

WR-024 may use this document only after the relevant Interaction V2 slice names
the definition vocabulary, `FormedInteractionModel` output, retained formation
adapter, runtime enforcement point, shell adapter, app boundary, and regression
guard.

Valid WR-024 work is limited to retained UI implementation and compatibility
proofs for:

- popup/menu contrast and scrollable popup content after popup stack, menu
  sizing, scroll ownership, outside-dismiss, and focus-return contracts exist;
- Switch Type submenu anchoring after parent/child menu scope and anchor
  stability are formed contracts;
- tab/workspace close and active indicators after structural chrome slots are
  formed contracts;
- viewport FPS/frame-time display after status overflow and viewport input
  arbitration are formed contracts.

Invalid WR-024 work includes defining a new popup policy in shell code, making
viewport arbitration app-local UI truth, accepting compiled-reactive or
ECS-driven UI, or changing retained UI execution strategy.

2026-05-15 boundary update: WR-025 now provides the first code-bearing
`IV2-menu-stack`, `IV2-scroll-ownership`, `IV2-menu-sizing`,
`IV2-chrome-slots`, `IV2-dock-drop-zones`, and
`IV2-status-and-viewport-arbitration` slices through
`domain/ui/ui_definition/src/interaction.rs`,
`domain/ui/ui_definition/src/validate.rs::validate_menu`,
`domain/ui/ui_runtime/src/layout/engine.rs::layout_popup`,
`domain/ui/ui_runtime/src/input/hit_test.rs`,
`domain/ui/ui_runtime/src/input/pointer.rs::dispatch_pointer_event`,
`domain/ui/ui_runtime/src/runtime/ui_runtime.rs::dispatch_keyboard_event`, and
toolbar/tab-stack adapters in
`domain/editor/editor_shell/src/composition/build_editor_shell.rs::build_editor_shell_frame_with_docking_visual_state`.
WR-024 may consume those landed contracts for popup layer behavior,
outside-dismiss, focus return, Escape dismissal, Switch Type submenu anchoring,
wheel boundary ownership, menu item fill, popup clamp, scroll fallback, and
tab/workspace close, label/drag, active-indicator slot placement, and tab
reorder versus split-preview drop-zone precedence. It may also consume the
formed viewport status overflow and scene-input fallback arbitration records
before changing FPS/frame-time status display behavior.

## Interaction V2 Slice Dependencies

WR-024 must cite one of these WR-025 slice names for every retained UI change:

| Polish symptom | Required Interaction V2 slice |
|---|---|
| Popup contrast and menu layer behavior | `IV2-menu-stack` |
| Scrollable popup content and wheel boundary behavior | `IV2-scroll-ownership` plus `IV2-menu-sizing` |
| Switch Type submenu placement | `IV2-menu-stack` |
| Tab/workspace close affordance and active indicator placement | `IV2-chrome-slots` |
| Tab reorder versus split preview precedence | `IV2-dock-drop-zones` |
| Viewport FPS/frame-time status display and overflow | `IV2-status-and-viewport-arbitration` |

Compatibility-only WR-024 work must name the old retained path, the target slice
above, and the guard that prevents the compatibility path from becoming durable
policy.

## Decisions

Owning code paths:

- `domain/editor/editor_shell/src/composition/surface_definition_context.rs::contrast_popup_theme`
- `domain/ui/ui_runtime/src/layout/engine.rs::layout_popup`
- `domain/editor/editor_shell/src/composition/build_editor_shell.rs::build_tab_strip_from_frame`
- `domain/editor/editor_shell/src/composition/build_editor_shell.rs::register_tab_stack_chrome_routes`
- `domain/editor/editor_shell/src/composition/build_viewport_panel.rs::viewport_statistics_text`
- `domain/editor/editor_shell/src/composition/toolbar_definition.rs::project_workspace_close_buttons`

## Requirements

- Popup/menu contrast is centralized in shell theme helpers.
- Menus with more content than available screen space use scrollable popup content.
- Switch Type opens as a level-2 menu next to the Switch Type row.
- Tab and workspace close buttons sit on the left side of their anchor.
- The opposite side carries a fixed-size active indicator: open circle for inactive, filled dot for active.
- Viewport statistics include frame rate and frame time when metrics are available.

## Non Goals

- No change to the retained UI execution strategy.
- No icon-font dependency change.
- No hard-coded one-off popup colors outside shared helpers.

## Tests

Required coverage:

- `domain/ui/ui_runtime/src/layout/engine.rs::layout_popup` stretches a single scroll child when popup height clamps.
- toolbar, viewport, and tab-stack popups use the contrast helper.
- Switch Type route anchors to `tab_stack_surface_submenu_anchor_widget_id`.
- tab/workspace close overlays use left placement.
- active indicators reserve the same 18x18 space as close affordances.
