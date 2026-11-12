# Status: where things stand

The lead updates this file after every step (builder started, report in, review in, merge, decision) and fast-forwards `main`, so a new session started in the master can continue from here. It is the only place for this state; milestone READMEs hold the plan, not the progress. Finished milestones are recorded in their milestone folders and in the Git history.

## Now

- **Milestone:** M7 is done and tagged `m7-done` (2026-11-11, the maintainer let it land on a full run whose only failures were browser tests that pass alone, M7-F244). Next: M8, the timeline and theme editor, [roadmap](04-milestones.md#m8--timeline-and-theme-editor); its plan is written (`docs/milestones/M8/README.md`, 12 packages in 5 waves) and published as a page. The maintainer took every recommendation of M8-Q1…Q14 (2026-11-11). WP-01 (the lead's docs) is *done* on `WIN1`: ADR-0043 *Dates and dependencies*, the amendments of ADR-0030, 0026, 0010, 0017, 0025 and 0033, the words, the M8 part of the security chapter and *Personal data in M8*, the UX gaps, and the package files WP-03 to WP-12. WP-02 (a green full run) is *merged* (2026-11-11, one fix round): failing browser tests down from five to one a run, M7-F243, F245 and F246 done; the maintainer let it land with one known failure, `net::ERR_NO_BUFFER_SPACE` about one run in two, whose cause is looked for with a network trace of Windows (M8-F10 to F17, [follow-ups](milestones/M8/follow-ups.md)). Wave 1 is *merged* (2026-11-12, one full run): WP-03 (dates and dependencies, migration 49, two fix rounds), WP-04 (work times and public holidays, migration 50, three fix rounds, one privacy leak of shares found in review and fixed), WP-05 (own themes, migration 51, finished by a second builder, two fix rounds; the maintainer accepted a `bb-wasm` budget of 166,000 bytes, M8-F44). Who sees whose work times: only through the projects one shares with the person (the lead's choice, ADR-0030 amendment M8). Wave 2 is *building* since 2026-11-12, all three on Opus: WP-06 in `bugaboo.win3`, WP-07 in `bugaboo.win4`, WP-08 in `bugaboo.win5`.
- **Done:** M0 to M7 (`m0-done` to `m7-done`; M7 tagged on 2026-11-11). Version 0.5.0 (ADR-0040: 130 of 225 features). The M7 review page is published (link outside the repository).
- **Wave:** 1 and 2 are merged: WP-01 to WP-08. Wave 3 is merged: WP-09 to WP-11. Wave 4 is merged: WP-12, WP-13, WP-14. Every package of M6 is merged; the exit review of M6 comes next (M6-F212 asks it to read the next nightly speed run). M7 runs beside it: wave 1 (WP-01 to WP-04) is merged; of wave 2, WP-07 (runs and *Undo the run*), WP-06 (the assistant on the server) and WP-05 (the MCP server and the sign-in of AI apps) are merged, landed in one full run; WP-08 (hand-overs and standing rules) and M6 WP-15 (the exit fixes) are merged too, so wave 2 is complete and every package of M6 is merged; M6 is approved and tagged `m6-done`. Wave 3 builds: WP-09 (tools for agents, limits) is merged (2026-11-08, with a test group that runs the trybuild tests one at a time), WP-10 (the assistant panel) is merged (2026-11-08, three fix rounds, together with `cargo xtask folder clean <n>`, the one command that cleans an idle work folder); WP-11 (agent work on screen, four fix rounds, migration 46) is merged (2026-11-08); WP-12 (*My agents* and the agent pages, four fix rounds; the administrators' list sorts working agents first on the server) is merged (2026-11-09), so wave 3 is complete. Wave 4 is merged (2026-11-09, one full run): WP-13 (the sample, agents under load, speed, the AI guide in the app, the exit demo; migration 47; the speed of *changes since* for an agent is a known gap the maintainer accepted, at most 250 ms until the exit wave, M7-F127), WP-14 (standing rules that hand over what they find, rules that never run out when an administrator allows it; migration 48), M7-F52 (the assistant in the permission matrix) and the accepted SQLite advisory (M7-F240). WP-15 (each kind of AI its own switch for the organization and each project, the assistant's look-ups as a setting with *No limit* that is truly unlimited, the assistant last everywhere, pictures in Paper) is merged (2026-11-09), with the page `userDocs/ai/ai-in-bugaboo.md` and the README section on AI. The exit work is merged (2026-11-10): the light themes have more colour (Daylight one colour per area, Paper a stronger accent, M7-F241), apps and agents can preview a change over MCP (M7-F225, F227), flaky timing tests fixed at their cause (M7-F207, F221, F230; still open: M7-F243), a session is marked as used outside the request, and the reads of an agent's look at changes are batched with a single-use hand-off of its key. Measured on a quiet computer (2026-11-11): M7-F127 is done (61.2 ms on SQLite, 63.8 ms on PostgreSQL, best of three; its known gaps removed); M7-F226 was the bench driver giving up a scenario in its warm-up after a cold start, fixed (753 ms on SQLite, inside the known gap); M7-F222 was the missing `TCP_NODELAY` on accepted connections, fixed (the slowest request of the search page 59 ms instead of 335). There is no scheduled nightly run any more (the maintainer, 2026-11-11): the benchmarks run by hand at each milestone exit. work folders 4 and 5 are cleaned. The exit is done: the visual pass of the exit demo (22 steps, four themes, computer and phone; two small findings on the review page), the review page, version 0.5.0, the README and the tag `m7-done`, pushed to GitHub. The open Dependabot alerts of GitHub are fixed and closed. The faster local runs are merged (Rust tests with cargo-nextest, the sample organization made in the background, checks side by side); a folder runs `cargo xtask setup` once after it rebases onto them. Branches are now rebased onto `WIN1` and fast-forwarded, never merged (ADR-0041, GIT-LINEAR).
- **`main`:** fast-forwarded to the last green `WIN1`.

## Work packages (M8)

| WP | Name | State | Folder / branch | Notes |
|----|------|-------|-----------------|-------|
| WP-01 | Decision records, words, the UX gaps, the maintainer's answers | done (2026-11-11) | `win1` / `WIN1` | Wave 0; docs only |
| WP-02 | A full run that is green, and the two small screen gaps of M7 | merged (2026-11-11, one fix round; one known failure left, M8-F10) | `win2` / `WIN2` | Wave 0; runs alone on a quiet computer |
| WP-03 | Dates and dependencies on the server | merged (2026-11-12, two fix rounds) | `win3` / `WIN3` | Wave 1; migration 49; starts once WP-02 is merged |
| WP-04 | Work times on the server | merged (2026-11-12, three fix rounds) | `win4` / `WIN4` | Wave 1; migration 50; privacy review |
| WP-05 | Own themes on the server | merged (2026-11-12, two fix rounds) | `win5` / `WIN5` | Wave 1; migration 51; security review |
| WP-06 | The plan on the server | building (2026-11-12) | `win3` / `WIN3` | Wave 2; migration 52; privacy review |
| WP-07 | Work times and the time factor on screen | merged without the time factor (two fix rounds; the time factor follows WP-06, M8-F60) | `win4` / `WIN4` | Wave 2; lands after WP-06 |
| WP-08 | The theme editor | merged (two fix rounds and a small last commit) | `win5` / `WIN5` | Wave 2; security review |
| WP-09 | The timeline on a computer | planned | `win3` / `WIN3` | Wave 3 |
| WP-10 | People and warnings on the timeline, and the phone timeline | planned | `win2` / `WIN2` | Wave 3; after WP-09 |
| WP-11 | Touch everywhere | planned | `win4` / `WIN4` | Wave 3; lands last in the wave |
| WP-12 | Sample part 7, the timeline under load, speed, the exit demo | planned | `win2` / `WIN2` | Wave 4 |

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

- M1's exit demo includes installing in WSL. No Linux has run the code yet (M0-F02, M1-F02); the decision is to install no Linux distribution for now, so the M1 demo runs on Windows only unless that changes.

## Next action

M8: wave 2 builds (WP-06, WP-07, WP-08). The lead checks the builders, reviews each report with a fresh reviewer (SSOT/DRY/ODR gate; privacy review for WP-06, security review for WP-08, UX review from pictures for WP-07 and WP-08), joins them on `WIN1` and lands them in one full run. Builders do not push (their origin branches are stale and a force push is not allowed); the lead fetches from the folders. Beside it the lead reads a network trace of a full run when the maintainer starts one and fixes M8-F10 to F17 from it.

Wave 3 of M6 is merged (WP-09 to WP-11). Wave 4 runs: WP-13 is merged without the board-card half of its first task, because there is no board yet (M6-F160); WP-14 and WP-12 are merged, so M6 waits only for its exit review. The time totals of the biggest project (`project-time-totals`) stay over their budget of 100 ms on 1 M issues: the maintainer accepted a known gap (2026-11-07), held to its own limit in the nightly run, and kept totals or a project on each time entry come later. The passkeys run on the stable webauthn-rs 0.6.2 (M6-F127, M1-F36 done). Every work folder runs `cargo xtask setup` once after rebasing onto the faster runs, which need `cargo-nextest`. Each package gets a code review and a UX review of its pictures, then the lead's full run on a rebased copy. The lead does not wait for a nightly run before going on. After M6 comes M7, the assistant, AI apps and agents; the timeline and theme editor moved to M8 (the maintainer, 2026-11-06). M7 is planned (`docs/milestones/M7/`, 13 packages in 4 waves) and its wave 1 starts now, beside the end of M6 (the maintainer, 2026-11-07, who also accepted every recommendation of M7-Q1 to Q10): wave 1 (WP-01 docs, WP-02 the provider, WP-03 the tools and *Never sent to AI*, WP-04 agents) is merged, landed with the last M6 package in one full run. Wave 2: WP-07, WP-06 and WP-05 are merged (2026-11-08), with M6-F68 (the WebAssembly modules built for size, ADR-0005 amendment M7, follow-up M7-F206). WP-08 and M6 WP-15 landed in one full run after it (2026-11-08), with the first page made 4.8 KB smaller (the words of settings load with the settings) and a queue for the writers of SQLite (`db/gate.rs`; M6-F227 keeps its small open points). Migrations: 40 WP-07, 41 WP-06, 42 WP-08, 43 WP-05, 44 M6 WP-15, 45 WP-09, 46 WP-11. Follow-up rows beyond a package's block take M7-F200 and up. The search page stays over its 100 ms speed budget on this computer (p95 about 127 ms, also before this landing): M6-F212.

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
