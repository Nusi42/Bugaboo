# Bugaboo — rules for agents

Read this before every task. The rules live in `docs/`; this file only says where. Read the sections it names, not just this list.

## Everyone

1. **Where you work:** "Working in a folder" in [`docs/04-milestones.md`](docs/04-milestones.md#working-in-a-folder): never build in the master (`<master>`); your own folder `bugaboo.winN` and branch `WINn`; `cargo xtask setup` first; CodeGraph per folder.
2. **Your work package** in `docs/milestones/Mx/`, and "Life of a work package" and "Definition of done" in the same file: tests first, `cargo xtask check staged` and your tests green, report the exact commands you ran and their results.
3. **[`docs/coding-standards.md`](docs/coding-standards.md), all of it.** These rules are the easiest to break:
   - §1: SSOT, DRY, ODR. This is a merge gate for existing and new code.
   - §2: commits, and no private information in Git.
   - §3: Rust. §4: no literal text, design tokens only, all theming in `design/theme.kdl`.
   - §5: dependencies. §7: zero warnings, never skip the hooks, never switch off a check.
4. **[`docs/glossary.md`](docs/glossary.md):** the only allowed names for things.
5. **The decision records** (`docs/adr/`) your package touches. Ports: ADR-0023. A new check: "Adding a rule" in [`docs/architecture/10-checks.md`](docs/architecture/10-checks.md).

## If you are the lead (the main session)

Read [`docs/STATUS.md`](docs/STATUS.md) first, then follow "The lead's routine" in [`docs/04-milestones.md`](docs/04-milestones.md#the-leads-routine).
