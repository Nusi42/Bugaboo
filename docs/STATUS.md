# Status: where things stand

The lead updates this file after every step (builder started, report in, review in, merge, decision) and fast-forwards `main`, so a new session started in the master can continue from here. It is the only place for this state; milestone READMEs hold the plan, not the progress. Finished milestones are recorded in their milestone folders and in the Git history.

## Now

- **Milestone:** M9, privacy, operations and the first release, is planned and decided ([`milestones/M9/`](milestones/M9/README.md)): the maintainer took every recommendation of M9-Q1 to Q21 on 2026-11-18, including three cuts (restore from the command line only; support access, `DIST-03`, moved to M14 as a V1 feature; M4-F61, M4-F63 and M4-F128 to a later operations package). ADR-0044 to ADR-0047 are accepted, ADR-0048 in part. Waves 1 and 2 are merged (WP-02 to WP-09). Next: wave 3.
- **Done:** M0 to M8 (`m0-done` to `m8-done`; M8 tagged on 2026-11-17). Version 0.5.1 (ADR-0040: 144 of 225 features, the minor number stays 5, so the patch number rises). The M8 review page is published (link outside the repository).
- **Wave:** M9 waves 0 and 1 are merged (2026-11-19); wave 2 (WP-06 to WP-09) is merged (2026-11-20). The tools for several agents and for agents that wait, hand over and undo are merged. Wave 3 is merged (WP-10, WP-11, WP-13). Wave 4: WP-14 is merged; the security pass before 0.6.0 (five reviewers, one per boundary) found no critical issue, and its fixes (sign-in, the API and AI; the host and files) are merged; WP-15's code is merged; the fixes for backups and privacy on a large organization land next, then the measurements run again, alone; the open points of the privacy screens (M9-F91, F92) are merged. The speed test of ten writers is within its budget again (M9-F87). M8 is complete (all of its packages merged).
- **`main`:** fast-forwarded to the last green `WIN1`.

## Work packages (M9)

| WP | Name | State | Folder / branch | Notes |
|----|------|-------|-----------------|-------|
| WP-01 | Decision records, words, the UX gaps, the maintainer's answers | done (2026-11-18) | `win1` / `WIN1` (written in `win2`) | Wave 0; docs only |
| WP-02 | Backup, and restore from the command line | merged (2026-11-19, two fix rounds; the second builder took over for the fixes) | `win2` / `WIN2` | Wave 1; Opus; migration 54; rebases onto WP-03's links |
| WP-03 | The walk of a person's data, and *Download my data* | merged (2026-11-19; person links three fix rounds, *Download my data* two more; four Opus builders in turn) | `win3` / `WIN3` | Wave 1; Opus; migration 55; its person links land first |
| WP-04 | Keep and delete rules on the server | merged (2026-11-19, three fix rounds; landed with WP-03 in one run) | `win4` / `WIN4` | Wave 1; Opus; migration 56; rebases onto WP-03's links |
| WP-05 | The parts list, the notices and the VEX statements | merged (2026-11-18, two fix rounds; Sonnet) | `win5` / `WIN5` | Wave 1; Sonnet |
| WP-06 | *Switch off* and *Remove a person* on the server | merged (*Switch off* and the removal plan 2026-11-19, three fix rounds; the removal itself 2026-11-20) | `win3` / `WIN3` | Wave 2; Opus; migration 57 |
| WP-07 | Upgrades: a backup before migrating, the way back, the upgrade test | merged (2026-11-19, two fix rounds; a second builder took over for the fixes) | `win2` / `WIN2` | Wave 2; Opus |
| WP-08 | Signed releases, updates, the support period, vulnerability handling, secure defaults | merged (2026-11-19, two fix rounds; new installations start with AI apps and agents off, migration 59 keeps old ones on) | `win5` / `WIN5` | Wave 2; Opus |
| WP-09 | Correct your own data, and corrections by administrators | merged (2026-11-19, two fix rounds; a second builder took over for the fixes; first-page budget raised to 260 KB by the maintainer) | `win4` / `WIN4` | Wave 2; Opus; migration 58 |
| WP-10 | The privacy screens | merged (2026-11-20; one fix round, then a rebase onto the operations screens; open points M9-F91 to F93) | `win3` / `WIN3` | Wave 3; Opus |
| WP-11 | The operations screens: *Backups* and *About* | merged (2026-11-20; two fix rounds, then three for the ci: the settings lists, one scroll box, the parts list in the debug build) | `win2` / `WIN2` | Wave 3; Sonnet |
| WP-12 | *Support access* | moved to M14 (2026-11-18) | — | Draft kept in the M9 folder |
| WP-13 | Run as a service, proven on clean systems | merged (2026-11-22; one fix round; also keys without a passphrase, the real public keys, the WSL distribution of the release build, and no Windows code signing for 0.6.0, M9-F126) | `win4` / `WIN4` | Wave 3; Opus |
| WP-14 | The accessibility pass, the EN 301 549 report, the statement | merged (2026-11-22; one fix round; open points M9-F130 to F136) | `win3` / `WIN3` | Wave 4; Opus |
| WP-15 | Sample part 8, the long run, speed, the release candidate, the exit demo | code merged (2026-11-23): the sample part 8, the long run with a backup in the middle, the exit demo, the release notes; its measurements on 1 M issues found that backups through the page, the removal of a person and the walk of a person's data do not scale (M9-F144, F145, F51); their fixes are built, and the measurements run again after them | `win2` / `WIN2` | Wave 4; Opus; measures alone on a quiet computer |
| WP-16 | Agents that wait for work, hand over and undo | merged (2026-11-20; two fix rounds, a second builder for the history of an agent and a third for the fixes) | `win4` / `WIN4` | Added 2026-11-19 from a tester's report; Opus; migration 62 |
| WP-17 | Several agents at once, and requests for agents by a program | merged (2026-11-20; one fix round) | `win5` / `WIN5` | Added 2026-11-19 from a tester's report; Opus; migration 61 |
| WP-17 | Several agents at once, and agents asked for by a program | building (Opus) | `win5` / `WIN5` | Wave 3; Opus; migration 61; added on 2026-11-19 by the maintainer's decision |

## Work packages (M8)

| WP | Name | State | Folder / branch | Notes |
|----|------|-------|-----------------|-------|
| WP-01 | Decision records, words, the UX gaps, the maintainer's answers | done (2026-11-11) | `win1` / `WIN1` | Wave 0; docs only |
| WP-02 | A full run that is green, and the two small screen gaps of M7 | merged (2026-11-11, one fix round; one known failure left, M8-F10) | `win2` / `WIN2` | Wave 0; runs alone on a quiet computer |
| WP-03 | Dates and dependencies on the server | merged (2026-11-12, two fix rounds) | `win3` / `WIN3` | Wave 1; migration 49; starts once WP-02 is merged |
| WP-04 | Work times on the server | merged (2026-11-12, three fix rounds) | `win4` / `WIN4` | Wave 1; migration 50; privacy review |
| WP-05 | Own themes on the server | merged (2026-11-12, two fix rounds) | `win5` / `WIN5` | Wave 1; migration 51; security review |
| WP-06 | The plan on the server | merged (two fix rounds) | `win3` / `WIN3` | Wave 2; migration 52; privacy review |
| WP-07 | Work times and the time factor on screen | merged (two fix rounds; the time factor after WP-06 with one more) | `win4` / `WIN4` | Wave 2; lands after WP-06 |
| WP-08 | The theme editor | merged (two fix rounds and a small last commit) | `win5` / `WIN5` | Wave 2; security review |
| WP-09 | The timeline on a computer | merged (four fix rounds) | `win3` / `WIN3` | Wave 3 |
| WP-10 | People and warnings on the timeline, and the phone timeline | merged (eight fix rounds) | `win2` / `WIN2` | Wave 3 |
| WP-13 | Nothing moves under the cursor | first part merged (2026-11-14, many fix rounds); the folding head of the issue page and the list merged (2026-11-15, three fix rounds); the folding head of settings pages and the timeline merged (2026-11-15, four fix rounds); dragging under a folded head, the rows left in the table and M8-F128, F129 merged (2026-11-15, three fix rounds); the pass of first reads merged (2026-11-15, three fix rounds), so WP-13 is merged | `win5` / `WIN5` | Before the exit; the maintainer's report of 2026-11-13 |
| WP-11 | Touch everywhere | merged (three fix rounds) | `win4` / `WIN4` | Wave 3 |
| WP-12 | Sample part 7, the timeline under load, speed, the exit demo | merged (2026-11-16; Sonnet built the first part and stopped halfway in its fix round, two Opus builders finished it in four fix rounds); the speed measurement on a quiet computer is left for the exit | `win2` / `WIN2` | Wave 4 |

## Work packages (M6)

| WP | Name | State | Folder / branch | Notes |
|----|------|-------|-----------------|-------|
| WP-01 | Decision records, words, the security chapter, the UX gaps, the maintainer's answers | merged (written in `win5`, reviewed three times) | `win1` / `WIN1` | Wave 1; must be merged before wave 2 starts |
| WP-02 | Theme files like the sister project | merged (one fix round) | `win2` / `WIN2` | Wave 1; first code package, merged first |
| WP-03 | Dates, the time of day and the person's day | merged (one fix round) | `win3` / `WIN3` | Wave 1 |
| WP-04 | Speed edges and quiet upkeep | merged (one fix round) | `win4` / `WIN4` | Wave 1 |
| WP-05 | Time on issues on the server | merged (four fix rounds) | `win2` / `WIN2` | Wave 2 |
| WP-06 | Working hours on the server | merged (three fix rounds) | `win3` / `WIN3` | Wave 2 |
| WP-07 | Estimates and totals | merged (six fix rounds) | `win4` / `WIN4` | Wave 2 |
| WP-08 | The maintainer's decisions on settings screens, label squares and IDs | merged (three fix rounds) | `win5` / `WIN5` | Wave 2 |
| WP-09 | The timer and the work day on screen | merged (five fix rounds) | `win2` / `WIN2` | Wave 3 |
| WP-10 | The timesheet | merged (five fix rounds) | `win3` / `WIN3` | Wave 3 |
| WP-11 | The settings of working hours and time | merged (four fix rounds) | `win4` / `WIN4` | Wave 3 |
| WP-12 | Sample part 5, timers under load, speed, the exit demo | merged (four fix rounds; the time totals of a project are a known gap, M6-F186) | `win2` / `WIN2` | Wave 4 |
| WP-13 | The time fields in layouts and an estimate when an issue is made | merged (four fix rounds; the board card waits for a board, M6-F160) | `win3` / `WIN3` | Wave 4 |
| WP-14 | Numbered choices: *Cost center* as number and name | merged (four fix rounds; the first page made 3 KB smaller to fit its budget) | `win4` / `WIN4` | Wave 4 |

## Resuming a package marked "building" in a new session

Builder agents belong to the session that started them; a new session finds none running. For each package marked *building*:

1. `git fetch origin` in `bugaboo.win1`, then `git log --oneline main..origin/WINn`: are there pushed commits?
2. In `bugaboo.winN`: `git status --short` and `git log --oneline origin/WINn..WINn`: uncommitted or unpushed work?
3. If the work looks complete: run `cargo xtask check all` and `cargo xtask ci` in that folder, then start a reviewer. If it is partial: start a new builder in the same folder with the work package, telling it to continue from the existing commits and files.

## Open findings

Recorded with their target package in the follow-up tables of the milestone folders, for example [`milestones/M1/follow-ups.md`](milestones/M1/follow-ups.md) and [`milestones/M0/follow-ups.md`](milestones/M0/follow-ups.md).

## Open questions

- **Done on 2026-11-21:** *Windows Sandbox* is switched on (M9-Q17), and the Linux release build has a WSL distribution to run in.
- **At the exit of M9:** signing and publishing 0.6.0. There is no pass with a screen reader by a person before the first release (the maintainer, 2026-11-21): the accessibility statement says the screens passed the automated checks and were not yet tried with a screen reader.
- **Linux:** `cargo xtask ci` runs on Windows and in the WSL distribution of this computer (Ubuntu, without systemd), against both databases; the static Linux program is built there. A Linux service manager is proven in M9 in a throwaway WSL distribution with systemd (WP-13). The old question of M1 (no Linux installed) is closed.

## Next action

M9 wave 1: reset `win2` to `win5` to `main` and run `cargo xtask setup` in each; start WP-03 first (Opus, `win3`) and land its person links as their own small landing; start WP-02 (Opus, `win2`), WP-04 (Opus, `win4`) and WP-05 (Sonnet, `win5`) beside it, WP-02 and WP-04 rebasing onto the links before they add person columns. Each builder prompt names its package file, its folder and branch, the SSOT/DRY/ODR gate, the glossary and the rule that no private information goes into Git.

The repository is public on GitHub, with GitHub Actions switched off and private reporting of security problems switched on (`SECURITY.md`). Before the push, every folder moved onto the one rewrite of the history; the old history is kept outside the repository. The push of a whole history reads every version of every file, so one old made-up example that a later rule reads as a profile path is listed as an old version in `tools/checks/private-exemptions.toml`.

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
- People on the timeline (ADR-0030, M8: PLAN-10 to PLAN-15): the person on each bar and rows per person; away times with no reason stored (others see only *Away*), usual work days, public holidays; plan by effort; plan warnings with a person's choice of what to do; a time factor per issue for its assigned person that goes back to ×1 when the person changes; work hours per day and a share in the project per person (PLAN-15); the assigned person may always set the time factor as their own estimate.
- AI agents (ADR-0029, M7): two ways in, an AI app acting for a person and agents that work on their own, each with one responsible person and never more than that person's rights. Work comes only by *Hand over* (issues by ID) or a *Standing rule* (a saved search, proposals or a few named fields, runs out). Agents are never assigned; agent work is its own record with attempts, questions and a review (*Accept, Send back, Throw away*); every attempt is one run that can be undone. One person may drive several agents (*My agents*). Bugaboo does not run agents.
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
- A speed number counts only on a dataset made by the code that is measured, and the benchmarks run by hand at each milestone exit, on a quiet computer, are the check of it (there is no scheduled nightly run; the maintainer, 2026-11-11). A font named in the theme must be on the computer or shipped. One fix round per gate finding, and a docs-only round is read before the merge. A browser test waits for what it depends on before it stops the page's clock or reads in a second window (architecture §11).
- Links to published pages count as private: they are kept outside the repository (coding-standards §2), and the docs name the page instead. No private information in Git (CLAUDE.md); the agent memory lives in `.claude/memory/`.
- CSRF is a session-bound token (server-derived from the session, sent in `__Host-bb_csrf`, echoed as `x-csrf-token`, compared in constant time) plus an `Origin` / `Sec-Fetch-Site` check that fails closed.
- Security log entries are signed with a separate, non-rotating log key and anchored in an append-only file outside the database; the person in an entry is personal data (basis: legitimate interest, Art. 6(1)(f) with Art. 32).
- All UI theming lives in `design/theme.kdl` (KDL), compiled at build time.
- `webauthn-rs` 0.6.2 (pure Rust, MPL-2.0, free; M1-F36 done). RUSTSEC-2023-0071 stays an accepted risk.
- The check runner runs the selected checks side by side; META-SPEED times the same function.
- Playwright run by hand serves `web/app/dist`: run `npm run build` first (`cargo xtask ci` builds it as step 1).
