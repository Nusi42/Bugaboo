# Status: where things stand

The lead updates this file after every step (builder started, report in, review in, merge, decision) and fast-forwards `main`, so a new session started in the master can continue from here. It is the only place for this state; milestone READMEs hold the plan, not the progress. Finished milestones are recorded in their milestone folders and in the Git history.

## Now

- **Milestone:** M6, Time and working hours, [plan](milestones/M6/README.md): 12 work packages in 4 waves. Every recommendation of the plan's questions M6-Q1 to M6-Q13 is taken. Its first package is the theme structure of the sister project (M5-F167).
- **Done:** M0 to M5 (`m0-done` to `m5-done`).
- **Wave:** none is running. Wave 1 (WP-01 to WP-04) has not been started, and nothing of it is committed.
- **`main`:** fast-forwarded to the last green `WIN1`.

## Work packages (M6)

| WP | Name | State | Folder / branch | Notes |
|----|------|-------|-----------------|-------|
| WP-01 | Decision records, words, the security chapter, the UX gaps, the maintainer's answers | planned (lead) | `win1` / `WIN1` | Wave 1; must be merged before wave 2 starts |
| WP-02 | Theme files like the sister project | planned | `win2` / `WIN2` | Wave 1; first code package, merged first |
| WP-03 | Dates, the time of day and the person's day | planned | `win3` / `WIN3` | Wave 1 |
| WP-04 | Speed edges and quiet upkeep | planned | `win4` / `WIN4` | Wave 1 |
| WP-05 | Time on issues on the server | planned | `win2` / `WIN2` | Wave 2 |
| WP-06 | Working hours on the server | planned | `win3` / `WIN3` | Wave 2 |
| WP-07 | Estimates and totals | planned | `win4` / `WIN4` | Wave 2 |
| WP-08 | The maintainer's decisions on settings screens, label squares and IDs | planned | `win5` / `WIN5` | Wave 2 |
| WP-09 | The timer and the work day on screen | planned | `win2` / `WIN2` | Wave 3 |
| WP-10 | The timesheet | planned | `win3` / `WIN3` | Wave 3 |
| WP-11 | The settings of working hours and time | planned | `win4` / `WIN4` | Wave 3 |
| WP-12 | Sample part 5, timers under load, speed, the exit demo | planned | `win2` / `WIN2` | Wave 4 |

## Resuming a package marked "building" in a new session

Builder agents belong to the session that started them; a new session finds none running. For each package marked *building*:

1. `git fetch origin` in `bugaboo.win1`, then `git log --oneline main..origin/WINn`: are there pushed commits?
2. In `bugaboo.winN`: `git status --short` and `git log --oneline origin/WINn..WINn`: uncommitted or unpushed work?
3. If the work looks complete: run `cargo xtask check all` and `cargo xtask ci` in that folder, then start a reviewer. If it is partial: start a new builder in the same folder with the work package, telling it to continue from the existing commits and files.

## Open findings

Recorded with their target package in the follow-up tables of the milestone folders, for example [`milestones/M1/follow-ups.md`](milestones/M1/follow-ups.md) and [`milestones/M0/follow-ups.md`](milestones/M0/follow-ups.md).

## Open questions

- M1's exit demo includes installing in WSL. No Linux has run the code yet (M0-F02, M1-F02); the decision is to install no Linux distribution for now, so the M1 demo runs on Windows only unless that changes.

## Next action

Start M6 wave 1 (WP-01 to WP-04) from `main`. `win2` holds the M5 picture helpers on the local branch `shots`.

The public repository is on GitHub (private until the maintainer makes it public), pushed once from the master with its tags, with GitHub Actions switched off. Before the push, every folder moved onto the one rewrite of the history; the old history is kept outside the repository. The push of a whole history reads every version of every file, so one old made-up example that a later rule reads as a profile path is listed as an old version in `tools/checks/private-exemptions.toml`.

## Decisions that still bind

Product

- Time and working hours (M6): theme files whose tokens may name tokens (Q1); *Time of day* in *Region* (Q2); *Activity* and *Cost center* as starter own fields (Q3); *Book time* (Q4); the timer (Q5); *Time left* and totals over visible issues only (Q6); *Billable* off at the start (Q7); the Germany preset with the two details of ArbZG §4 (Q8); past days with a reason, and nights (Q9); the role *HR* that nobody holds at the start, and *Totals only* (Q10); 2 years for work days (Q11); the exit demo with four more steps (Q12); what stays out (Q13). WP-01 writes each answer where it belongs.
- Fonts and themes follow the sister project: no font files shipped, its system font stacks word for word, a base theme file plus one file per built-in theme that only sets colors (`extends="default"`).
- Screens: a phone opens an issue as its full page (the side panel is for computers); on a phone list, *Group by* and *Spacing* move into the *Filters* sheet and the color legend opens from a *Key* button; the app's bar is hidden on the phone issue page; a group opens as a page like a person and a role; one pattern for *Who sees people* and *Field permissions*; label squares may use looser colors; the sidebar is shorter in a small window.
- Dates and clock follow the person's *Region* (one form per person); an issue ID is always in the code font.
- The search language accepts and shows glossary words with spaces and display case (`status group is not Done`, `priority is High`); the old forms keep working. It is a WebAssembly module of its own with a budget of at most 100 KB (ADR-0005, amendment M5, decision 7).
- The verb for something new is *Create* everywhere; code names that differ from the glossary were renamed before the API freeze.
- API v1 is frozen at `m4-done`: ARCH-API-FREEZE compares every later copy with the one at the tag.
- The *Window size* button is the way for screen-reader users to resize a dialog until the resize keys are tested with a screen reader.
- The first-page budget (PERF-04, 250 KB Brotli) stays; the message catalog is split by area, and each lazy area brings its own texts.
- People on the timeline (ADR-0030, M7: PLAN-10 to PLAN-15): the person on each bar and rows per person; away times with no reason stored (others see only *Away*), usual work days, public holidays; plan by effort; plan warnings with a person's choice of what to do; a time factor per issue for its assigned person that goes back to ×1 when the person changes; work hours per day and a share in the project per person (PLAN-15); the assigned person may always set the time factor as their own estimate.
- AI agents (ADR-0029, M8): two ways in, an AI app acting for a person and agents that work on their own, each with one responsible person and never more than that person's rights. Work comes only by *Hand over* (issues by ID) or a *Standing rule* (a saved search, proposals or a few named fields, runs out). Agents are never assigned; agent work is its own record with attempts, questions and a review (*Accept, Send back, Throw away*); every attempt is one run that can be undone. One person may drive several agents (*My agents*). Bugaboo does not run agents.
- The UX concept: controls 36 px with a mouse and 44 px with touch; short values save in place with Undo, long texts keep Edit and Save; rows tinted by status group and edged by priority; forms open as windows.
- Organization and permissions: a project lead may pick from every person and group in the organization when giving roles; *Sign out everywhere* also ends a person's access keys and a password change does not; keys can never invite people or change roles; changing a short name needs Change project settings plus the right to make projects at that level; JSON above 1 KiB is compressed except secret answers.
- Edit locks cover only the long text; stopping someone else's run of many changes needs *Manage people*; renewing issue IDs counts the version up but does not change *Updated*.
- A text whose part is not loaded (ADR-0028) throws in development and tests; in production it logs once and shows nothing.

Engineering

- SSOT, ODR and DRY are a mandatory review gate for all existing and new code (coding-standards §1). Every fix round gets its own gate check by a reviewer with a fresh context.
- The wording of history, notifications and e-mails lives once, in Rust that builds for wasm32 (the server for e-mails, the browser through `bb-wasm`). Dates, numbers and time zones stay in two implementations, held together by test vectors generated from Rust; this exception is recorded in an ADR and extends to the synchronous TypeScript `listText`, `durationText` and `ownValueText`. Short e-mails do not name the project.
- Code never writes down a path of the system; it asks the operating system (coding-standards §2, "No fixed paths", LINT-PATHS).
- Nothing is written outside a work folder; logs, scripts, pictures and programs go into its ignored `.work/`. A builder writes its commit message into a file in `.work/`.
- A screen package's report names the pictures it looked at; a branch with a red test is not pushed.
- A speed number counts only on a dataset made by the code that is measured, and the nightly run is the check of it. A font named in the theme must be on the computer or shipped. One fix round per gate finding, and a docs-only round is read before the merge. A browser test waits for what it depends on before it stops the page's clock or reads in a second window (architecture §11).
- Links to published pages count as private: they are kept outside the repository (coding-standards §2), and the docs name the page instead. No private information in Git (CLAUDE.md); the agent memory lives in `.claude/memory/`.
- CSRF is a session-bound token (server-derived from the session, sent in `__Host-bb_csrf`, echoed as `x-csrf-token`, compared in constant time) plus an `Origin` / `Sec-Fetch-Site` check that fails closed.
- Security log entries are signed with a separate, non-rotating log key and anchored in an append-only file outside the database; the person in an entry is personal data (basis: legitimate interest, Art. 6(1)(f) with Art. 32).
- All UI theming lives in `design/theme.kdl` (KDL), compiled at build time.
- `webauthn-rs` 0.6.1-dev stays for M1 (pure Rust, MPL-2.0, free); switch to the first stable 0.6 before 1.0 (M1-F36). RUSTSEC-2023-0071 stays an accepted risk.
- The check runner runs the selected checks side by side; META-SPEED times the same function.
- Playwright run by hand serves `web/app/dist`: run `npm run build` first (`cargo xtask ci` builds it as step 1).
