# AGENTS.md

Runenwerk is a Dornglut `integration-product` repository. This file defines only the Runenwerk-specific executor contract.

Organization-wide work selection, project-neutral software-design defaults, repository and GitHub rules, validation-evidence semantics, contribution/licensing defaults, and cross-repository governance are owned by `dornglut/engineering`:

- [Authority and work](https://github.com/dornglut/engineering/blob/main/governance/authority-and-work.md)
- [Software design standard](https://github.com/dornglut/engineering/blob/main/standards/software-design.md)
- [Repository standard](https://github.com/dornglut/engineering/blob/main/standards/repositories.md)
- [GitHub standard](https://github.com/dornglut/engineering/blob/main/standards/github.md)
- [Validation standard](https://github.com/dornglut/engineering/blob/main/standards/validation.md)
- [Licensing standard](https://github.com/dornglut/engineering/blob/main/standards/licensing.md)
- [Runen-family architecture](https://github.com/dornglut/engineering/blob/main/architecture/runen-family.md)

When operating through GPT Web with the GitHub connector, also follow the [Engineering GPT Web GitHub procedure](https://github.com/dornglut/engineering/blob/main/tooling/gpt-web-github.md).

## Before editing

1. Identify the semantic owner of the behavior or invariant.
2. Inspect the current owning source and tests.
3. Read the relevant accepted Runenwerk ADR or design; use [`ARCHITECTURE.md`](ARCHITECTURE.md) for cross-domain work.
4. Check the owning active issue when the work is already accepted.
5. Keep the change to one coherent boundary.

For cross-repository boundaries or extraction, also read the Engineering Runen-family architecture and the owning framework authority. Do not infer reusable framework semantics from Runenwerk adapters or product integration.

## Runenwerk boundaries

- Runenwerk owns integration and product policy, not reusable framework semantics.
- Preserve one-way framework and local dependency direction.
- Do not add compatibility aliases, forwarding modules, source mirrors, or duplicate authority without a demonstrated current compatibility requirement and removal condition.
- Do not create generated prompts, work-state ledgers, truth certificates, execution locks, or temporary/self-authoring feature workflows as parallel authority.
- Do not use GitHub Actions to author feature-branch changes.

## Validation and evidence

Use focused checks while editing. The repository-owned merge baseline is:

```text
cargo validate
```

Run it from a checked-out executor before merge when that execution path is available. Acceptance requires repository-owned exact-head hosted CI to execute the canonical baseline for the unchanged reviewed feature head. See [`TESTING.md`](TESTING.md) for the current Runenwerk validation map.

Report only validation and behavior actually observed. Report what changed, the owning boundary, evidence actually obtained, and any remaining blocker.
