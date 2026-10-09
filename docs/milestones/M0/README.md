# M0 — Foundation: work packages

Status: **approved by the product owner**, tagged `m0-done`. Review page published (link outside Git). Accepted review points: [follow-ups](follow-ups.md).

Goal and exit demo: see [04-milestones.md](../../04-milestones.md) (M0). Every package follows the Definition of done there and the rules in [`CLAUDE.md`](../../../CLAUDE.md): **0 warnings, 0 errors, no disabled useful rules.**

| WP | Name | Who | Folder / branch | Depends on |
|----|------|-----|-----------------|------------|
| [WP-01](WP-01-workspace-and-runner.md) | Workspace and check runner | Lead | `bugaboo.win1` / `WIN1` | — (done first) |
| [WP-02](WP-02-browser-toolchain.md) | Browser toolchain, tokens and texts | Builder | `bugaboo.win2` / `WIN2` | WP-01 |
| [WP-03](WP-03-rust-and-repo-checks.md) | Rust and repository checks, tool installer | Builder | `bugaboo.win3` / `WIN3` | WP-01 |
| [WP-04](WP-04-docs-and-meta-checks.md) | Docs builder and meta checks | Builder | `bugaboo.win4` / `WIN4` | WP-01 |
| [WP-05](WP-05-server-dev-and-hooks.md) | Server hello, dev runner, hooks and local CI | Builder | `bugaboo.win5` / `WIN5` | WP-01 |

**File ownership** keeps parallel work apart: a package changes only the files listed under *Owns*. Shared files (`xtask/src/checks/mod.rs`, `xtask/src/main.rs`, `xtask/Cargo.toml`, `tools/checks/catalog.toml`, `docs/third-party/README.md`, `.gitignore`) may get **added lines only**; the lead resolves merge conflicts.

**Merge order:** WP-03 → WP-02 → WP-05 → WP-04 (meta checks last, because they look at everything the others built).
