# M8 — Timeline and theme editor: work packages

Goal, features and exit demo: see [04-milestones.md](../../04-milestones.md) (M8). Rules: "Working in a folder", "Life of a work package" and "Definition of done" in [04-milestones.md](../../04-milestones.md), the [coding standards](../../coding-standards.md) (the SSOT/DRY/ODR review gate first), the [glossary](../../glossary.md).

**Goal:** plan on a timeline with the people who do the work, and make Bugaboo look like your organization. A person sees their issues as bars over the days they will work on them. Away days, work hours and a share in the project shape the bars. Warnings say when a plan does not fit, and the person chooses what to do. An administrator or a person makes a theme of their own, and the editor says which colour pairs are hard to read.

M8 starts from M7 (`m7-done`; API v1 frozen at `m4-done`: new routes and optional members are additions, nothing is renamed, removed or tightened). What exists:

- **No dates to plan with.** An issue has *Estimated time* and *Time left* (`crates/bb-core/src/issue.rs`, migration 33), but no start date and no stored due date. *Due date* exists only as a field key that search cannot read and forms block (`crates/bb-core/src/fields.rs`, M3-F101). Own fields can be dates.
- **Relations have no meaning for dates.** Relation kinds are rows the organization edits (`crates/bb-core/src/relation.rs`); the starting kinds are *blocks*, *duplicate of*, *causes* and *related to*.
- **Goals exist, releases and milestones do not.** A goal is an issue type with sub-issues. Releases are ISS-08 in M10.
- **Working hours exist, work times do not.** M6 built the work day, its rules and the Germany preset (`bb-core::working_time`, `bb-app::working_hours`, ADR-0035). There are no usual work days, work hours per day, shares, away times or public holidays, and no people kind *Work times*.
- **Themes are files.** `design/theme.kdl` and one colour file per theme are compiled at build time (`cargo xtask gen theme`). The theme choices in the settings are static, the page allows no inline styles (`style-src 'self'`), and the contrast maths lives only in xtask (STY-CONTRAST, active for the built-in files).
- **Named already:** the part `timeline-bar` with status tints (M3-F163), `ViewKind::List` in saved searches (the record has room for other views, M5-Q12), the timeline of the mockup.
- **Open from M7:** a full browser run that is not green on this computer (M7-F244, F243) and two small screen gaps (M7-F245, F246). The maintainer asked for them first (review page of M7, 2026-11-11).

Decided already: ADR-0030 (people on the timeline: the person on the bar, away times, work hours and shares, plan by effort, the time factor, warnings), ADR-0009 (our own timeline component, no Gantt library), ADR-0010 and ADR-0026 (themes store token values, never CSS; theme files set colours and shadow), ADR-0019 (the theme editor takes an edit lock), ADR-0034 (the timeline is a view of a saved search), ADR-0035 (time and working hours kept apart; holidays and usual work days come in M8).

M8 runs in **five waves**: wave 0 alone, then four waves of at most four builders. **Wave 0 runs alone on a quiet computer:** it makes the full run green, and every later landing needs that. WP-01 (the lead's docs) is written in wave 0 beside it, so wave 1 starts as soon as both are merged.

| Wave | WP | Name | Who | Folder / branch |
|------|----|------|-----|-----------------|
| 0 | WP-01 | Decision records, words, the UX gaps, the maintainer's answers | Lead | `bugaboo.win1` / `WIN1` |
| 0 | [WP-02](WP-02-a-green-full-run.md) | A full run that is green, and the two small screen gaps of M7 | Builder | `bugaboo.win2` / `WIN2` |
| 1 | [WP-03](WP-03-dates-and-dependencies.md) | Dates and dependencies on the server | Builder | `bugaboo.win3` / `WIN3` |
| 1 | [WP-04](WP-04-work-times-on-the-server.md) | Work times on the server: usual work days, work hours, shares, away times, public holidays | Builder | `bugaboo.win4` / `WIN4` |
| 1 | [WP-05](WP-05-own-themes-on-the-server.md) | Themes of your own on the server: stored themes, their style sheet, contrast in one place | Builder | `bugaboo.win5` / `WIN5` |
| 2 | [WP-06](WP-06-the-plan-on-the-server.md) | The plan on the server: bars, plan by effort, the time factor, warnings | Builder | `bugaboo.win3` / `WIN3` |
| 2 | [WP-07](WP-07-work-times-and-the-time-factor-on-screen.md) | Work times and the time factor on screen | Builder | `bugaboo.win4` / `WIN4` |
| 2 | [WP-08](WP-08-the-theme-editor.md) | The theme editor | Builder | `bugaboo.win5` / `WIN5` |
| 3 | [WP-09](WP-09-the-timeline-on-a-computer.md) | The timeline on a computer: rows, zoom, bars, drag with Undo, dependencies, the tray | Builder | `bugaboo.win3` / `WIN3` |
| 3 | [WP-10](WP-10-people-and-warnings-on-the-timeline.md) | People and warnings on the timeline, and the timeline on a phone | Builder | `bugaboo.win2` / `WIN2` |
| 3 | [WP-11](WP-11-touch-everywhere.md) | Touch everywhere: targets and swipe actions | Builder | `bugaboo.win4` / `WIN4` |
| 4 | [WP-12](WP-12-sample-load-speed-and-exit-demo.md) | Sample part 7, the timeline under load, speed, the exit demo | Builder | `bugaboo.win2` / `WIN2` |

Progress is kept only in [`docs/STATUS.md`](../../STATUS.md). The files of the packages (`WP-nn-….md`) were written in WP-01, after the maintainer answered the questions below (2026-11-11). WP-02 depends on none of them and started at once.

## The packages in short

- **WP-01 (lead):** the maintainer's answers to M8-Q1…Q14, each written where it belongs. A new decision record *Dates and dependencies* (ADR-0043): the two date fields, the date effect of a relation kind, what moves when a date moves. Amendments: ADR-0030 (several assigned people, Q4; holidays, Q6; where the plan is worked out, Q7), ADR-0026 and ADR-0010 (themes of your own: what they may set, how they reach the page, Q8 to Q11). The glossary words of M8, the M8 part of the security chapter and *Personal data in M8* (away times, work hours, shares). The UX gaps: people on the timeline, entering away times, usual work days and work hours, the share on *Project members*, the time factor on the issue, the list of warnings, organization themes in *Look › Themes*, and the phone timeline (one wording, Q12). The files of WP-02 to WP-12.
  - **Done (2026-11-11).** The answers are written where each belongs (below, under each question). New: [ADR-0043](../../adr/0043-dates-and-dependencies.md) *Dates and dependencies*. Amended: ADR-0030 (M8), ADR-0026 (update M8), ADR-0010, ADR-0017, ADR-0025 and ADR-0033 (M8). The glossary has the words of M8 (*Start date*, *Due date*, *Date effect*, *Dependency*, *Person on the bar*, *Takes me …*, the ways out of a warning, *Set dates… / Add dependency…*, *Holidays*, *Own theme*, *Theme editor*, *Readability*), architecture §12 its part *Timeline, work times and own themes (M8)* and *Personal data in M8*, UX design the gaps of §4.4, §4.7, §4.13, §5.1, §5.2 and §5.6, the feature list the new wording of PLAN-01, PLAN-04, THM-03, THM-04 and UX-02, and 04-milestones the exit demo with M8-Q3. The package files WP-03 to WP-12 are linked in the table. Migration 52 (WP-06: the time factor and accepted warnings) follows 49 to 51.
- **WP-02 (M7-F243, F244, F245, F246):** a full run of `cargo xtask ci` that is green on this computer three times in a row. It finds why sockets run out under the browser tests (about 1,200 connections in TIME_WAIT, `net::ERR_NO_BUFFER_SPACE`) and fixes each wait at its cause, not with longer time limits: the clock of the lease test, `Instant::now()` in the limit of app calls, the pool of the session tests. It also fixes the chip under the bar on the phone and the empty area of the review window.
- **WP-03 (PLAN-01, PLAN-02 on the server; migration 49):** *Start date* and *Due date* as built-in fields of the field catalogue (readable in search, on forms, in layouts, with field permissions). A date effect on a relation kind: *Must finish first* means the later issue starts after the earlier one ends. A move that breaks a dependency is a warning, never a silent push. Every change of a date is in the history and can be undone.
- **WP-04 (PLAN-11, PLAN-15 on the server; migration 50):** usual work days and work hours per day for each person (start: Monday to Friday, 8 hours). A share in each project, set by a Project lead (start: 100 %). Away times in whole or half days, with no reason stored. Public holidays (Q6). The permission *Enter work times for others*, the people kind *Work times*, the notification when someone else enters an away time, and deletion: away times 1 year after they end, hours and shares when a person leaves (Q13).
- **WP-05 (THM-04 on the server, THM-03 contrast; migration 51):** own themes stored as token values (for everyone or personal), one style sheet per theme served by Bugaboo under `style-src 'self'`, theme choices in the settings that list built-in and custom themes, and import and export as JSON. The contrast maths moves from xtask into one place that the check, the server and the editor share (Q9).
- **WP-06 (PLAN-08, PLAN-10, PLAN-12, PLAN-13, PLAN-14 on the server):** the timeline as a view of a saved search (`ViewKind::Timeline`). Its rows, its bars and their dates, restricted issues left out entirely. Plan by effort: estimate × time factor over work hours × share, skipping away days, days off and holidays, never moving a fixed due date. The time factor on the issue: who set it, back to ×1 when the person changes, never set by AI, never summed or sorted. The four warnings and the ways out of each.
- **WP-07 (PLAN-11, PLAN-14, PLAN-15 on screen):** *My settings › Work times* (usual work days, work hours, region of holidays), away times for oneself and, with the permission, for others; the share on *Project members*; the time factor on the issue (*Takes me 10 days*) with who set it.
- **WP-08 (THM-03, THM-04 on screen):** the theme editor of UX design §5.6: colour groups, a live preview, readability ratings (Easy, OK, Hard), *Save a copy*, *Download*, *Upload*, organization themes in *Look › Themes* and personal themes in *My settings*. It takes an edit lock.
- **WP-09 (PLAN-01, PLAN-02, PLAN-03, PLAN-04 on screen):** the timeline of UX design §4.7 on a computer. Rows grouped and folded, zoom from day to quarter, the Today line, bars with status tints and a done part, dependency arrows, drag to move and resize with Undo and a typed alternative for every drag (*Set dates…*, *Add dependency…*), and the tray *Not scheduled yet*.
- **WP-10 (PLAN-10, PLAN-11, PLAN-13 on screen, the phone):** the person on the bar and *Group by person* (following *Who sees people*), hatched away days and gaps, the card of a bar ("Estimate 8 days · ×1.25 = 10 work days · Lea's own estimate"), the warnings with their ways out, and the timeline on a phone (Q12).
- **WP-11 (UX-02):** 44 px touch targets on every screen, including the *Compact* tick boxes (M5-F158) and the gaps of phone form footers (M6-F121); swipe actions on list rows, each also in *More*. Phone pictures of every screen.
- **WP-12 (exit):** sample part 7: Lea, the *Mobile app* goal with dated issues, dependencies, away times, work hours and shares. Ten people planning at once (`bb-load-check`, the browser scenarios). The timeline on 1 M issues against the budget of a page. The exit demo, step by step.

## Why the waves are in this order

Wave 0 first: a landing needs a green full run, and none has been green since 2026-11-05. Wave 1 builds the three foundations side by side; they touch different crates (issues and relations, work times, themes). Wave 2 needs them: the plan needs dates and work times, the screens of work times need their server, the editor needs stored themes. Wave 3 draws the timeline once the plan answers. Touch (WP-11) goes in wave 3 so it also covers the new screens. Wave 4 proves it on the sample and under load.

## Follow-ups handled in M8

| Follow-up | Package |
|-----------|---------|
| M7-F243, M7-F244 (a green full run), M7-F245, M7-F246 (screen gaps) | WP-02 |
| M3-F101 (*Due date* blocked on forms) | WP-03 |
| M3-F163 (place and drag of timeline bars), M2C-F105 (positions as custom properties of a part, no inline style) | WP-09 |
| M0-F03 (faint and muted text too close in Paper and Daylight), M6-F10 (one shadow shape), M6-F13 (`Theme.kind` as a type), M6-F74 (label squares written twice in the dark themes) | WP-05 |
| M5-F158 (*Compact* tick boxes on touch), M6-F121 (phone form footers) | WP-11 |

## Checks switched on or grown in M8

- STY-CONTRAST grows from the built-in files to every theme that is saved (WP-05), with the same code.
- TEST-A11Y: a keyboard alternative for every drag of the timeline (WP-09).
- Phone pictures of every screen (WP-11), as 04-milestones asks.
- PERF-BENCH: the timeline of the biggest project on 1 M issues (WP-12).
- SEC-MATRIX: rows for away times, work hours, shares and the time factor (WP-04, WP-06).

## Reviews

Every package gets a code review with the SSOT/DRY/ODR gate, with a fresh reviewer for each fix round. WP-04 and WP-06 also get a privacy review (away times are personal data; ADR-0030 keeps the reason out). WP-05 and WP-08 get a security review (a theme of your own reaches every page). Screens get a UX review from pictures in the four themes, on computer and phone.

## The exit demo

As written in 04-milestones (M8), with the answer to M8-Q3 for "a warning for the release". Each step is a test in `web/app/e2e/m8-exit-demo.spec.ts`, with pictures in the four themes on computer and phone.

## Questions for the maintainer

Each question has the lead's recommendation. Answering "take every recommendation" is fine; a different answer changes the files of the packages before they are written.

**Decided (2026-11-11): the maintainer took every recommendation below.**

**M8-Q1: Which dates does the timeline use?** PLAN-01 says the dates come from start and due fields, "configurable which ones". Today there is no start date and no stored due date.

- **Recommendation:** two built-in fields, *Start date* and *Due date*, like *Estimated time* (in the field catalogue, with field permissions, in search and layouts). A timeline view of a saved search may use own date fields instead, chosen in the view.
- **Answer (2026-11-11):** the recommendation. Written in ADR-0043, decisions 1 and 2; the glossary (*Start date*, *Due date*); PLAN-01.

**M8-Q2: Which relation means "must finish first"?** Relation kinds have no meaning for dates. The starting kind *blocks* says the same as the glossary's *Must finish first*. PLAN-02 also names *start together*, *finish together* and *finish when it starts*, "in V1".

- **Recommendation:** a relation kind gets a date effect. M8 has one effect, *Must finish first*, which the starting kind *blocks* gets; an organization may give it to one other kind. The other three effects stay in V1. A move that breaks a dependency shows a warning and never pushes other issues on its own.
- **Answer (2026-11-11):** the recommendation. Written in ADR-0043, decisions 4 and 5; the glossary (*Date effect*, *Dependency*, *Must finish first*).

**M8-Q3: Releases are not built until M10.** The exit demo asks for "a warning for the release", and UX design §4.7 draws a release as a diamond. ISS-08 (releases) is planned for M10.

- **Recommendation:** no releases in M8. A goal with a due date is drawn as a diamond at that date, and the demo's warning is for the goal: "Lea's away week moves *Login screen* past the due date of *Mobile app 2.0*." Releases join the timeline in M10.
- **Answer (2026-11-11):** the recommendation. Written in ADR-0043, decision 7; UX design §4.7; PLAN-04; the exit demo in 04-milestones.

**M8-Q4: An issue can have up to 20 assigned people. Whose plan shapes the bar?** ADR-0030 speaks of "the person on the bar".

- **Recommendation:** the first assigned person is the person on the bar. Their work times shape it, and the time factor is theirs: it goes back to ×1 when the first person changes. *Group by person* shows the bar in the row of each assigned person, shaped by the first one, with a mark that says so.
- **Answer (2026-11-11):** the recommendation. Written in ADR-0030, amendment M8; the glossary (*Person on the bar*); UX design §4.7.

**M8-Q5: May the time factor be searched?** ADR-0030 says no reports and no sorting by the time factor, and never set by AI.

- **Recommendation:** it is readable on the issue and over the API like any field the person may read, but search can neither filter, sort nor group by it, and no total uses it. The assistant and agents never change it (a rule of the tool registry, tested).
- **Answer (2026-11-11):** the recommendation. Written in ADR-0030, amendment M8; the glossary (*Time factor*); architecture §12.

**M8-Q6: Where do public holidays come from?** ADR-0030 asks for presets per country and German federal state.

- **Recommendation:** Bugaboo computes them itself: the fixed dates and the ones that follow Easter, for Germany and each of its 16 states, in Rust, tested against the official lists of 2026 to 2030. No download and no outside service. More countries come when someone asks. A person picks their *Holidays* (start: the organization's choice); it is a setting of its own, not part of *Region*, which is about formats.
- **Answer (2026-11-11):** the recommendation. Written in ADR-0030, amendment M8; the glossary (*Public holidays*, *Holidays*); UX design §4.13 and §5.1.

**M8-Q7: Where is the plan worked out?** Plan by effort and the warnings are rules.

- **Recommendation:** once, in Rust, on the server, which answers with the bars and their warnings. While a bar is dragged the browser only moves it; on release the server answers with the new plan within the budget of a page. No WebAssembly module for the plan, so the first page stays within its budget.
- **Answer (2026-11-11):** the recommendation. Written in ADR-0030, amendment M8; the glossary (*Plan*, internal).

**M8-Q8: How does a theme of your own reach the page?** The page allows no inline styles.

- **Recommendation:** Bugaboo serves each custom theme as a style sheet of its own, made from its stored token values (`/themes/{id}.css`, cached, changed with the theme). The theme choices in *My settings* list the built-in themes, the organization's themes and the person's own.
- **Answer (2026-11-11):** the recommendation. Written in ADR-0026, update M8; architecture §12.

**M8-Q9: Contrast in one place.** The contrast maths lives only in the check of the theme files.

- **Recommendation:** move it into `bb-core` and use it everywhere: in STY-CONTRAST, on the server when a theme is saved, and in the editor through the existing browser module (its size is checked against the budget).
- **Answer (2026-11-11):** the recommendation. Written in ADR-0026, update M8; ADR-0010, amendment M8.

**M8-Q10: What may a theme of your own change?** THM-03 says every token, typography and spacing included. ADR-0026 lets theme files set only colours and shadow. The UX picture of the editor has a group *Font & corners*.

- **Recommendation:** colours, the shadow, the corner radius, and the choice among the system font stacks Bugaboo already has. No spacing, density or font files. THM-03 is worded so.
- **Answer (2026-11-11):** the recommendation. Written in ADR-0026, update M8; THM-03; UX design §5.6.

**M8-Q11: What does a theme of your own start from?** Built-in themes change over time (M7-F241).

- **Recommendation:** a custom theme starts from one built-in theme and stores only the values it changes, so later fixes of the built-in theme reach it. A theme for everyone cannot be saved while a pair is *Hard* to read; a personal theme can, with a warning (as UX design §5.6 says).
- **Answer (2026-11-11):** the recommendation. Written in ADR-0026, update M8; UX design §5.6.

**M8-Q12: The timeline on a phone.** UX design says both "a day-by-day list" and "a list by goal with a seven-week span".

- **Recommendation:** a list grouped like the computer's rows, each issue with its dates, its person and a short bar over seven weeks; dragging is done with *Set dates…* on the phone.
- **Answer (2026-11-11):** the recommendation. Written in UX design §3.3 and §4.7; UX-02.

**M8-Q13: When are work times deleted?** ADR-0030: away times 1 year after they end, work hours and shares when the person leaves. The keep periods of PRIV-05 come in M9.

- **Recommendation:** the deletion jobs come in M8 with the data, so no away time is kept longer than decided. M9 shows them in the overview of keep periods.
- **Answer (2026-11-11):** the recommendation. Written in ADR-0030, amendment M8; ADR-0017, amendment M8; architecture §12, *Personal data in M8*.

**M8-Q14: What stays out of M8?**

- **Recommendation:** releases (M10), the other three dependency effects and *Deadline drivers* (V1), the critical path, baselines and levelling of work across people, fonts other than the system stacks, a plan made or changed by AI, and reports on time factors.
- **Answer (2026-11-11):** the recommendation. Written in the section below.

## What stays out of M8 (M8-Q14)

Each item keeps its place in the feature list and the plan:

- **Releases** on the timeline, and rows by release: ISS-08 and PLAN-04, M10. Until then a goal's due date is the milestone of the timeline (ADR-0043, decision 7).
- **The other three date effects** (*start together*, *finish together*, *finish when it starts*): PLAN-02, V1.
- ***Deadline drivers*** and the critical path: PLAN-05, V1 (UX design §4.7 marks the switch as V1).
- **Baselines** and export to pictures: PLAN-07, later.
- **Levelling work across people** (moving or giving issues to balance a plan): not planned; ADR-0030 keeps every way out a person's choice, and Bugaboo never suggests whom (COMP-10).
- **Fonts other than the system stacks**: not planned (ADR-0026, update on fonts and update M8).
- **A plan made or changed by AI**: not planned; no tool of the registry changes the time factor (ADR-0030, amendment M8), and dates change through tools only as any field does, with the person's rights.
- **Reports on time factors**: never (ADR-0030, decision 4; COMP-10, PRIV-09).
