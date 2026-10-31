# ADR-0024: Master repository in a synced folder, work folders outside it

Status: **Accepted** · 2026-10-04

## Context

The repository lives in a folder that a sync service (OneDrive or similar) keeps in step. Build output (Rust `target`, `node_modules`, caches, test reports) would make that service sync gigabytes and lock files during builds. The public remote is added later; until then the repository is local. The project uses a pattern for this: a master repository (`<master>`) plus clones in `<work root>/<project>.winN`, each on its own branch `WINn`.

## Decision

- **Master:** `<master>` holds `main` and the shared history. **Nothing is built or installed there**; it only receives pushes and fast-forward merges.
- **Work folders** (outside the synced folder), clones of the master with `origin` pointing to it:
  - `<work root>/bugaboo.win1`, branch `WIN1`: the lead's folder. Work packages are merged here, `cargo xtask ci` runs here, and only then is `main` fast-forwarded.
  - `bugaboo.win2`, `bugaboo.win3`, …, branches `WIN2`, `WIN3`, …: one per builder agent working in parallel. Each starts from `main`, pushes its branch when its work package is done, and is reset to `main` for the next one.
- **Integration:** in `win1`: fetch `WINn`, merge, run `cargo xtask ci`; push `WIN1`; in the master: `git merge --ff-only WIN1`. `main` therefore only ever moves to states that passed CI.
- **Build folders** stay inside each work folder (they are outside the synced folder already). Machine-specific overrides go into `.cargo/config.local.toml`, which the committed `.cargo/config.toml` includes optionally (works on stable Cargo) and Git ignores.
- **Guard:** `cargo xtask` refuses to build, test or install anything in a checkout marked as the master, with a message pointing to the work folders (marked explicitly, not by its path: Amendment, ADR-0039).
- **Later:** when the repository moves to the public remote (`<public remote>`), it becomes `origin` for the work folders and the master becomes one more clone; nothing else changes.

## Consequences

- The sync service only syncs source files and Git objects, not build output.
- Parallel builders never share a build folder, so they don't block each other.
- Each work folder needs its own build (several GB each); plan the disk space for it.

## Alternatives considered

- **One shared `CARGO_TARGET_DIR` for the user:** would also change the developer's other Rust projects on the same machine. Rejected.
- **Git worktrees of the master:** Git state would live inside the synced folder and be written by every agent at once. Clones keep each folder independent. Rejected.

## Amendment (M1)

- **Edit hook decides by the edited file.** The Claude Code session often runs in the master, so `$CLAUDE_PROJECT_DIR` points into the synced folder for every agent. The `PostToolUse` command therefore looks at the edited file: it finds the nearest folder above it with `.git` and `xtask/Cargo.toml`, skips files in the synced folder or outside any work folder, and runs that work folder's own `cargo xtask hook post-edit`. Enforced by META-WIRING (the command is compared byte for byte).
- **Agent memory in the repository.** Claude Code's memory files live in `.claude/memory/` and are committed like any doc; the master's ignored `.claude/settings.local.json` sets `autoMemoryDirectory` to `bugaboo.win1/.claude/memory`, so a session started in the master reads and writes the work folder's copy (never files in the synced folder).
- **Builds refused in the master by Cargo itself.** Besides the xtask guard, the master's ignored `.cargo/config.local.toml` sets `build.rustc-wrapper` to `tools/dev/refuse-build.cmd`, which exits with a message, so even a plain `cargo build` or an editor's Rust tooling started in the synced folder stops before anything is built. Tool data that does not belong in the synced folder (for example CodeGraph's index database) lives in a work folder, not in the master. Checked by review (the lead's set-up check).
- **Git hooks in every work folder.** `cargo xtask setup` installs lefthook and the hooks; the master gets none, because nothing is committed there.
- **If the sync service (for example OneDrive) turns the master's folders into cloud placeholders,** `git merge --ff-only WIN1` fails with "unable to unlink", "cannot create directory" or "Directory not empty", and `fsutil reparsepoint query <folder>` shows a tag instead of error 4390. Recovery, with nothing open in the master: move every top-level entry except `.git` into a backup folder outside the synced folder, run `git checkout HEAD -- .` in the master, then repeat the fast-forward. No `reset --hard`, no copying files by hand. Checked by review (the lead runs the `fsutil` query when a fast-forward fails).

## Amendment (M3 WP-10)

- **The browsers of the end-to-end tests.** `cargo xtask setup` also runs the pinned Playwright's `install` for Chromium and WebKit (the phone project in WebKit, M2-F12). Which revision of each browser is installed is decided by the Playwright version in `package-lock.json`, so the versions are pinned like the other tools. Playwright keeps the browsers in one cache of the user (`ms-playwright`), shared by all work folders, and skips what is there; they are the one thing `setup` does not put into `.tools/`, because each copy is several hundred megabytes.

## Amendment (ADR-0039)

- **The master is marked, not detected.** `BUGABOO_CHECKOUT_ROLE=master` in `.env.local` (written by `cargo xtask guard-master`) replaces every test of the path for a folder name. The edit hook, the build guard and the refusing `rustc-wrapper` read that mark; the tracked batch file is gone. A synced folder, such as OneDrive, may still be a good place for the master, and is nothing special to the tooling.
- **Only the master pushes to the public remote** (`git -C <master> push github main`); work folders push only to the master. See ADR-0039.

## Enforced by

BUILD-LOCATION and META-WIRING (see `tools/checks/catalog.toml`); the recovery steps by review.
