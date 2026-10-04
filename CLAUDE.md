# Bugaboo — rules for agents

Read this before every task. The docs in `docs/` are the source of truth; this file only points to them and lists the rules that are easiest to break.

## Where you work
- **Never build, install or run anything in the master repository** `<master>` (OneDrive). Work only in your work folder under `<work root>\bugaboo.winN`, on your branch `WINn` (ADR-0024).
- Commit on your branch; push your branch to `origin` when your work package is done. Never push to `main`.

## Before you write code
1. Read your work package in `docs/milestones/Mx/`.
2. Read `docs/glossary.md` (the only allowed names for things), `docs/coding-standards.md`, and the decision records (`docs/adr/`) your package touches.
3. Write the acceptance tests first.

## Zero warnings, zero errors
- Every formatter and linter must report **0 warnings and 0 errors**. Warnings are errors.
- **Don't switch off checks or lints.** If a rule is truly wrong for one place, use the narrowest possible suppression with a reason that names an issue or a doc section, for example `#[allow(clippy::print_stdout, reason = "CLI output, coding-standards §3")]`. The META-SUPPRESS check counts suppressions; adding one needs the lead's approval in review.
- Fix the cause, not the symptom: no `#[allow]` to silence a real problem, no `// oxlint-disable` to make CI green, no skipped tests.
- Before you say you're done: `cargo xtask check staged` and your package's tests pass. Report the exact commands you ran and their results.

## Rules that are easy to forget
- **Single source of truth, DRY, one definition** (coding-standards §1): generate, don't copy. Every constant, type, port, path, message and setting is defined once.
- **No hard-coded user-facing text**: all text goes into `messages/en.json`. Plain words from the glossary only.
- **No hard-coded colors or sizes**: design tokens only (`design/tokens.json`).
- **Ports** come only from `tools/dev/ports.json` (47200–47299), always with `strictPort`. Never default ports (5173, 4173, 3000, 8080, 51204).
- **External tools** come only from `tools/dev/tools.toml` (pinned versions), installed by `cargo xtask setup` into `.tools/`. **New dependencies** need an entry in `docs/third-party/README.md` in the same change.
- **Rust:** no `unsafe`, no `unwrap`/`expect`/`panic!`/`dbg!`/`println!` outside tests and CLI output; errors are typed.
- **Commits:** Conventional Commits (`feat(xtask): …`, `fix(web): …`), one logical change each, ending with the attribution lines the session gives you.
- **A new check** needs four things: the rule in the docs, an entry in `tools/checks/catalog.toml`, the implementation in `xtask/src/checks/`, and a failing plus a passing example (a fixture folder or `bad_example`/`good_example` tests).
