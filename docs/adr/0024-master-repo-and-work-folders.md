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
- **Guard:** `cargo xtask` refuses to build, test or install anything when it runs inside a synced folder, with a message pointing to the work folders.
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
- **Git hooks in every work folder.** `cargo xtask setup` installs lefthook and the hooks; the master gets none, because nothing is committed there.
- **If the sync service (for example OneDrive) turns the master's folders into cloud placeholders,** `git merge --ff-only WIN1` fails with "unable to unlink", "cannot create directory" or "Directory not empty", and `fsutil reparsepoint query <folder>` shows a tag instead of error 4390. Recovery, with nothing open in the master: move every top-level entry except `.git` into a backup folder outside the synced folder, run `git checkout HEAD -- .` in the master, then repeat the fast-forward. No `reset --hard`, no copying files by hand. Checked by review (the lead runs the `fsutil` query when a fast-forward fails).

## Enforced by

BUILD-LOCATION and META-WIRING (see `tools/checks/catalog.toml`); the recovery steps by review.
