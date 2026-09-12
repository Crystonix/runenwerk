# Runenwerk Docs Site

This is the canonical Runenwerk documentation source tree.

Docs live in:

```text
docs-site/src/content/docs
```

## Commands

Run these from `docs-site/`:

```sh
pnpm install
pnpm build
pnpm dev
```

Run repository-wide docs validation from the repository root:

```sh
python3 tools/docs/validate_docs.py
```

## Structure

- `workspace/`: repository orientation, agent rules, docs status, and workflow docs
- `guidelines/`: Runenwerk architecture, programming principles, module structure, and placement rules
- `domain/`: domain crate documentation
- `engine/`: engine/runtime/plugin docs
- `net/`: networking, simulation, replay, and runtime convergence docs
- `apps/`: runnable application docs
- `adr/`: accepted architecture decision records
- `design/`: architecture design proposals and templates
- `multiplayer/`: multiplayer design proposal material
- `templates/`: documentation templates
