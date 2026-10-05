# Bugaboo — rules for agents

Read this before every task. The docs in `docs/` are the source of truth; this file only points to them and lists the rules that are easiest to break.

## Where you work

- **Never build, install or run anything in the master repository** `<master>` (OneDrive). Work only in your work folder under `<work root>\bugaboo.winN`, on your branch `WINn` (ADR-0024).
- Commit on your branch; push your branch to `origin` when your work package is done. Never push to `main`.
- **CodeGraph** works per work folder: if yours has no index (`.codegraph/codegraph.db` missing), run `codegraph init` in it, then pass your folder as `projectPath` to `codegraph_explore` (or run `codegraph explore` there). Never index the master.
- Run `cargo xtask setup` once in a fresh or reset folder: it installs the pinned tools and the Git hooks (commit and push run the checks). Never skip the hooks (`--no-verify`). `git push --force-with-lease` runs the push checks too. When a hook fails, fix the cause and commit again; don't amend or rewrite commits you already pushed.

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
- **Ports** come only from `tools/dev/ports.json`, always with `strictPort`; your work folder `bugaboo.winN` has its own block of 100 ports (47200–48099 in all; block N starts at 47200 + (N−1)·100; any other folder uses block 1). Never default ports (5173, 4173, 3000, 8080, 51204).
- **External tools** come only from `tools/dev/tools.toml` (pinned versions), installed by `cargo xtask setup` into `.tools/`. **New dependencies** need an entry in `docs/third-party/README.md` in the same change.
- **Rust:** no `unsafe`, no `unwrap`/`expect`/`panic!`/`dbg!`/`println!` outside tests and CLI output; errors are typed.
- **Commits:** Conventional Commits (`feat(xtask): …`, `fix(web): …`), one logical change each, ending with the attribution lines the session gives you.
- **A new check** needs four things: the rule in the docs, an entry in `tools/checks/catalog.toml`, the implementation in `xtask/src/checks/`, and a failing plus a passing example (a fixture folder or `bad_example`/`good_example` tests).

## If you are the lead (the main session)

The maintainer may close a session at any time and start a new one in the master with "continue where we left off". Everything needed to continue is in the repository:

1. **Read [`docs/STATUS.md`](docs/STATUS.md) first.** It says the current milestone and wave, each work package's state and folder, open questions for the owner, and the very next action.
2. **Agents belong to the session that started them.** A new session finds no running builders or reviewers. Resume a package marked *building* by checking its branch (pushed commits?) and its folder (uncommitted work?) as `docs/STATUS.md` describes — never by waiting for it.
3. **Update `docs/STATUS.md` after every step** (builder started, report in, review in, merge, decision), commit it on `WIN1` and fast-forward `main`, so the master's copy is always current. A commit that changes only docs needs the Git hooks to pass; code needs `cargo xtask ci` before `main` moves.
4. Follow "How we work" in `docs/04-milestones.md`: builders (Sonnet) in `bugaboo.win2` and up, reset to `main` first (`git fetch origin && git checkout -B WINn origin/main`, then `cargo xtask setup`, which also installs the Git hooks); reviewers with a fresh context; merge into `WIN1` with Git's own subject (`git merge --no-ff origin/WINn -m "Merge branch 'WINn' (Mx WP-nn name)"`; the commit-message check accepts only that form for merges), `cargo xtask ci`, push `WIN1`, `git merge --ff-only WIN1` in the master.
5. Every builder prompt repeats the rules in this file and names the work package file, the folder and the branch; the builder reports the commands it ran and their results.
6. **"Can I close this session?"** Answer with: running background agents and commands; uncommitted or unpushed work in each `bugaboo.winN` (`git status --short`, `git log origin/WINn..WINn`); `main` vs `WIN1`; whether `docs/STATUS.md` is current. Say "safe" only when all are clean, or when `docs/STATUS.md` names exactly what a running builder will leave behind.
7. **Memory** lives in `bugaboo.win1/.claude/memory/` (the master's ignored `.claude/settings.local.json` points there). Commit memory changes with the next `docs/STATUS.md` update.
8. **CodeGraph:** the lead's index lives in `bugaboo.win1/.codegraph` (the master has none). Pass `projectPath: <work root>\bugaboo.win1` to `codegraph_explore`, or run `codegraph explore` in that folder. If `win1` has no index yet (`.codegraph/codegraph.db` missing), run `codegraph init` there.
9. The maintainer reviews on a phone: milestone review pages and changed design docs are published as commentable HTML pages; replies stay short and name feature IDs.
