# ADR-0037: No hosted CI; verification is local

Status: **Accepted** · 2026-11-01

## Context

The repository had a workflow file for a hosted CI service, prepared for the day the code moves to a public Git host. Hosted runners cost money on private repositories, repeat what the development computer already runs, and need a second place where the commands are listed and kept in step with the first.

## Decision

- **No hosted CI.** There is no `.github/workflows/` folder and no workflow linter (actionlint, check LINT-CI). META-WIRING no longer reads workflow files.
- **Verification is local.** The pre-push hook runs the push checks (`cargo xtask check push`); `cargo xtask ci` is the full local run (checks, tests of both databases, browser build, browser tests) and runs before `main` moves. Nothing reaches `main` without it.
- **Releases are built, signed and published locally;** the release page on the Git host is created by hand.
- **Hosted automation stays off** on the public repository (the repository setting is switched off before the first push).

## Consequences

- One list of commands (`catalog.toml`) and one runner; no second description of the same steps.
- A contributor's pull request is verified by the maintainer, who runs `cargo xtask ci` on it.

## Alternatives considered

- **Keep the workflow, run it by hand only:** a file that never runs drifts from the real commands. Rejected.
- **Run only the cheap checks in hosted CI:** a partial copy of the gate, and a second place to keep current. Rejected.

## Enforced by

META-WIRING (hooks call only xtask); the rest by review.
