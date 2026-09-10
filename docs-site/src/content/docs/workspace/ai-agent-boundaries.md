---
title: AI Agent Boundaries
description: Runenwerk-specific placement boundary for runtime AI integrations.
status: active
owner: workspace
layer: workspace
canonical: true
last_reviewed: 2026-09-10
related_docs:
  - ../../../AGENTS.md
---

# AI Agent Boundaries

The root [`AGENTS.md`](../../../../../AGENTS.md) owns the Runenwerk executor contract. Organization-wide GitHub and GPT Web connector procedure belongs to [`dornglut/engineering`](https://github.com/dornglut/engineering).

This page owns only the Runenwerk-specific placement rule for runtime AI integrations.

## Runtime integration placement

Runtime AI integrations belong in:

```text
apps/
tools/
adapters/
```

Do not add LLM clients, prompts, autonomous agents, or workflow-specific AI policy to `foundation/` or pure `domain/` crates.

Tool-assisted development and repository automation do not make runtime AI a foundation or domain semantic owner.

## Concept ownership

Use [`DOMAIN_MAP.md`](../../../../../DOMAIN_MAP.md) for concept placement. Do not duplicate the concept map here.
