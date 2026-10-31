# ADR-0039: Explicit master configuration; only the master publishes

Status: **Accepted** · 2026-11-01

## Context

ADR-0024 keeps a master checkout that never builds, and recognised it by a path segment (a folder whose name starts with `OneDrive`). That is one person's setup, not the project's: anyone who clones into a synced folder would be locked out of building, and the master would be undetectable anywhere else. Publishing was also open: any checkout could push anywhere, and the public remote is reached from exactly one place.

## Decision

- **The master is marked, never guessed.** A checkout is the master only when `BUGABOO_CHECKOUT_ROLE=master` is set in the environment or in the ignored file `.env.local` at its root. `cargo xtask guard-master` writes it, from a work folder (the master builds nothing): the `.env.local` mark, a `pre-push` hook in `.git/bugaboo-hooks/` together with `core.hooksPath` pointing there (so the hook runs whatever other hook folder is configured), and a program for Cargo's `rustc-wrapper` in `.cargo/config.local.toml`, so even a plain `cargo build` stops. Without the mark a plain clone builds, tests and commits as it is. The edit hook of Claude Code reads the same mark.
- **Only the master publishes.** A work folder pushes only to a remote that is a path (the master); only the master pushes to a remote on the network (the public one): `git -C <master> push github main`. One tested function, `publish::plan`, decides from the role and the URL of the remote (a path or a network address): a work folder to a path runs the checks of a push; the master to the network runs only the checks that build nothing (SEC-PRIVATE on the commits to push and on every tracked file, DEP-LICENSES); every other combination is refused with a message that names the master command. `main` only moves to a commit that passed the full run in a work folder, so the master does not repeat it.
- **The master's hook uses a program built elsewhere.** The hook calls the xtask program of the work folder that ran `guard-master`; if that program is gone, the hook says to run the command again.
- **Placeholders.** Rule texts name the master as `<master>` and the folder of the work folders as `<work root>`; the concrete paths are the maintainer's own setup, kept outside the repository.

## Consequences

- No path decides anything; the old guard for `OneDrive` folders and the tracked Windows batch file are gone, the build-refusing program is written per master.
- When the xtask program that the hook calls moves or is deleted, `guard-master` is run again.
- ADR-0024 is amended: its guard and its hook rule now follow this record.

## Alternatives considered

- **Keep detecting by path:** locks out other people's setups. Rejected.
- **Publish from a work folder (the earlier flow):** the public remote would be reachable from every checkout and needs the full checks repeated by the hook. Rejected.
- **A tracked hook script for the master:** a second place to keep current, and a file type without a linter. The hook is written by the command instead.

## Enforced by

BUILD-LOCATION (the mark is read before every build, test and install command); META-WIRING (the pre-push command of lefthook and the edit hook); the pre-push plan by its tests in `xtask/src/publish.rs` and `xtask/src/main.rs`.
