# Bugaboo — rules for agents

Read this before every task. The docs in `docs/` are the source of truth; this file only points to them and lists the rules that are easiest to break.

## Where you work

- **Never build, install or run anything in the master repository** `<master>` (OneDrive). Work only in your work folder under `<work root>\bugaboo.winN`, on your branch `WINn` (ADR-0024).
- Commit on your branch; push your branch to `origin` when your work package is done. Never push to `main`.
- Run `cargo xtask setup` once in a fresh or reset folder: it installs the pinned tools and the Git hooks (commit and push run the checks). Never skip the hooks (`--no-verify`).

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

## If you are the lead (the main session)

A new session picks up the work like this:

1. Find the current milestone: the highest `docs/milestones/Mx/` without an `mx-done` tag (`git tag`). Its `README.md` has the status of every work package.
2. Check the builder branches: `git log --oneline main..origin/WINn` in `bugaboo.win1` after `git fetch`. A package marked *building* with no running builder (builders belong to the session that started them) and commits on its branch is finished or half-finished: check `cargo xtask ci` in its folder, then review it or start a builder to finish it.
3. Follow "How we work" in `docs/04-milestones.md`: builders (Sonnet) in `bugaboo.win2` and up, reset to `main` first (`git fetch origin && git checkout -B WINn origin/main`, then `cargo xtask setup`, which also installs the Git hooks); reviewers with a fresh context; merge into `WIN1`, `cargo xtask ci`, push `WIN1`, `git merge --ff-only WIN1` in the master.
4. Every builder prompt repeats the rules in this file and names the work package file, the folder and the branch; the builder reports the commands it ran and their results.
5. The maintainer reviews on a phone: milestone review pages and changed design docs are published as commentable HTML pages; replies stay short and name feature IDs.
