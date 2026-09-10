---
title: UI Designer Workbench
description: Practical usage guide for the standalone and embedded Runenwerk UI Designer Workbench.
status: active
owner: editor
layer: app
canonical: true
last_reviewed: 2026-09-10
related_designs:
  - ../../design/accepted/ui-designer-workbench-product-design.md
  - ../../design/deferred/game-runtime-ui-projection-and-hud-platform-design.md
related_reports:
  - ../../reports/closeouts/pm-ui-designer-wb-008-runtime-proven-closeout-and-handoff/closeout.md
  - ../../reports/closeouts/pm-ui-designer-wb-v1-closure-006-runtime-proven-product-closeout-and-handoff/closeout.md
  - ../../reports/closeouts/pm-ui-designer-evidence-correction-001-runtime-evidence-source-revision-and-catalog-correction/closeout.md
---

# UI Designer Workbench

## Launch Paths

The standalone UI Designer is the focused authoring host:

```text
cargo run -p runenwerk_editor --bin runenwerk_ui_designer
```

The embedded host remains available through the editor shell and uses the same
underlying UI Designer product contracts.
