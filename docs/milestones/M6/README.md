# M6 — Time and working hours: work packages

Goal, features and exit demo: see [04-milestones.md](../../04-milestones.md) (M6). Rules: "Working in a folder", "Life of a work package" and "Definition of done" in [04-milestones.md](../../04-milestones.md), the [coding standards](../../coding-standards.md) (the SSOT/DRY/ODR review gate of §1 blocks a merge, in every review and every fix round) and the [glossary](../../glossary.md).

**Goal:** people track time on issues with a timer or by hand, with *Activity* and *Cost center* for accounting, and record their working hours of each day as German law asks, with the working-time rules as plain warnings; nobody sees anyone else's working hours without the permission for it, and nothing scores or ranks people (TIME-01, TIME-02, TIME-03, TIME-04, TIME-08, TIME-09, TIME-10, TIME-12, TIME-13, PRIV-09).

M6 starts from M5 (`m5-done`; API v1 frozen at `m4-done`: new routes and optional members are additions, nothing is renamed, removed or tightened). What exists:

- **Themes:** one file, `design/theme.kdl` (about 14,000 lines): each color token carries its four theme values on its own line (`daylight=`, `paper=`, `dusk=`, `midnight=`), and one token cannot name another (M5-F121). The fonts follow the sister project's system stacks.
- **Time fields:** the field catalogue of `bb-core` declares *Estimated time*, *Time left*, *Time spent*, *Activity* and *Cost center* (architecture §7), and *Who sees people* has the kind *Time entries*. Nothing stores them, no route reads them, and the search language already reads lengths of time in hours and minutes (ADR-0005, decision 10).
- **Dates:** *Region* has a time zone, a date format, a number format and the first day of the week, but no choice for the time of day, so a date written day first can stand beside a 12-hour clock (M5-F137). The server checks only the shape of a time zone (M2-F72). Lengths of time are worked out from days in four places (M4-F132).
- **Jobs:** `bb-jobs` runs the daily kinds of ADR-0017; `statistics.refresh` runs in the round at every start and can hold the write lock of a large table on SQLite for up to 9.7 s (M5-F168).
- **Speed:** the groups by *Assigned to* on 1 M issues are a known gap on SQLite (M5-F89, F170) and at the edge of their budget on PostgreSQL, like the preview of 5,000 issues (M5-F169).
- **Screens:** the timer is drawn in UX design §4.9 and §3.3 but not built (M3-F178); the sidebar is taller than a window 600 px high (M5-F150); on the phone issue page two *More* buttons show (M5-F172); a group opens as a panel, a person and a role as pages (M5-F173); *Who sees people* picks roles another way than *Field permissions* (M5-F135); the colors of the label squares are as pale as the fills that carry text (M5-F149).

Decided already: ADR-0006 (own fields of time entries and work days in `field_value`), ADR-0015 (every stored field marked; working hours are `employee_time`), ADR-0017 (jobs in the database), ADR-0018 (one settings registry), ADR-0022 and ADR-0025 (*Who sees people*, roles and groups, organization permissions given for the whole organization), ADR-0026 (the theme file), ADR-0030 (work times for planning are a separate thing, M7), architecture §5 *Working hours* and §7 (all moments in UTC, each work day keeps the person's time zone), UX design §4.9 (time and working hours) and §5.7 (two years for working hours).

M6 runs in **four waves**, then the exit visual pass and the exit waves. A wave starts when the previous one is merged to `main`. At most four builders run at once. **The first code package is the theme files (WP-02, a decision of the maintainer):** it is started first and merged first; no other package of wave 1 touches `design/`.

| Wave | WP | Name | Who | Folder / branch |
|------|----|------|-----|-----------------|
| 1 | [WP-01](WP-01-m6-docs.md) | Decision records, words, the security and privacy chapter, the UX gaps, the maintainer's answers | Lead | `bugaboo.win1` / `WIN1` |
| 1 | [WP-02](WP-02-theme-files.md) | Theme files like the sister project: a base file and one file of colors per built-in theme | Builder | `bugaboo.win2` / `WIN2` |
| 1 | [WP-03](WP-03-dates-clocks-and-the-persons-day.md) | Dates, the time of day and the person's day: one form per *Region*, one reader of lengths of time | Builder | `bugaboo.win3` / `WIN3` |
| 1 | [WP-04](WP-04-speed-edges-and-quiet-upkeep.md) | Speed edges and quiet upkeep: the statistics after the start, *Assigned to* groups, the preview | Builder | `bugaboo.win4` / `WIN4` |
| 2 | [WP-05](WP-05-time-on-issues-on-the-server.md) | Time on issues on the server: the timer, time entries, *Activity* and *Cost center* | Builder | `bugaboo.win2` / `WIN2` |
| 2 | [WP-06](WP-06-working-hours-on-the-server.md) | Working hours on the server: work days, breaks, the rules, reasons, who sees them, the keep period | Builder | `bugaboo.win3` / `WIN3` |
| 2 | [WP-07](WP-07-estimates-and-totals.md) | Estimated time, time left and time spent: totals up the issues and projects, search and columns | Builder | `bugaboo.win4` / `WIN4` |
| 2 | [WP-08](WP-08-maintainer-decisions-settings-labels-ids.md) | The maintainer's decisions on settings screens, label squares and issue IDs | Builder | `bugaboo.win5` / `WIN5` |
| 3 | [WP-09](WP-09-timer-and-work-day-on-screen.md) | The timer and the work day on screen: sidebar, *Home*, the issue page, the phone | Builder | `bugaboo.win2` / `WIN2` |
| 3 | [WP-10](WP-10-timesheet.md) | The timesheet: work-day rows, time on issues, warnings, reasons, the working hours of others | Builder | `bugaboo.win3` / `WIN3` |
| 3 | [WP-11](WP-11-working-hours-and-time-settings.md) | The settings of working hours and time: rules, keep period, *Totals only*, project defaults | Builder | `bugaboo.win4` / `WIN4` |
| 4 | [WP-12](WP-12-sample-load-speed-and-exit-demo.md) | Sample organization part 5, timers under load, speed of the time pages, the exit demo | Builder | `bugaboo.win2` / `WIN2` |

Progress is kept only in [`docs/STATUS.md`](../../STATUS.md). WP-01 writes the decision records and the words before any builder of wave 2 starts; wave 1's code packages need only what is decided already, so they start while the lead writes the rest.

## The packages in short

- **WP-01 (lead):**
  - The maintainer's answers to M6-Q1…Q13, each written where it belongs.
  - A new decision record *Time on issues and working hours* (ADR-0035): two separate records, the timer, the day of a person, the rules as data, reasons and the keep period, who sees what, no scores or rankings.
  - Amendments: ADR-0026 (theme files, M6-Q1), ADR-0025 (the time permissions and the starting role of M6-Q10), ADR-0017 (the jobs of M6 and the statistics after the start round, M5-F168).
  - The glossary words of M6, the M6 section of the security chapter and *Personal data in M6*.
  - The UX gaps: the phone timesheet, *Working hours of others*, the settings page of working hours, the long-timer note, *Add time*, the reason window.
  - The shape of the port of time spent (`TimeSpentStore`, ADR-0035 decision 4), so WP-05 and WP-07 run side by side. The trait takes the whole minutes of `bb_core::time`, which WP-03 makes, so the trait itself goes onto `main` as written there after WP-03 is merged and before wave 2 starts; WP-05 implements it and WP-07 tests against a fake of it.
- **WP-02 (THM-01, M5-F167, M5-F121):** a base theme file and `design/themes/<theme>.kdl` with only colors and `extends="default"`, as in the sister project; a token may name another token. The generated sheets stay the same byte for byte, apart from the two tokens of M5-F121.
- **WP-03 (CFG-04, UX-07, M5-F137 dates, M2-F72, M4-F132 lengths):** *Time of day* in *Region*; one way to write a date, a moment and a time of day on every screen, e-mail and notification; the server checks a time zone against the list of zones; the day, the week and the start of a day of a person in their zone, across daylight-saving changes; one reader of typed lengths of time (`1:30`, `1.5`, `90m`). Adds TXT-DATES.
- **WP-04 (PERF-01, M5-F168, F169, F170, F89):** no statistics refresh in the start round; a write that meets SQLite's busy answer during upkeep is tried again; the groups by *Assigned to* within budget, or the known gap kept with a measured reason; `VACUUM ANALYZE` after a PostgreSQL dataset is made.
- **WP-05 (TIME-01, TIME-02, TIME-13):** one running timer per person on the server, kept across devices and closed browsers; time entries by hand; *Activity* and *Cost center* as starter own fields with defaults from the issue, then the project; *Book time*; the long-timer notice; time entries in an issue's history.
- **WP-06 (TIME-08, TIME-09, TIME-10, TIME-12, PRIV-09):** work days with start, end and breaks; the working-time rules as data with the Germany preset; warnings stored with the day, never a block; a reason for a past day, the old value kept, the person told; *See working hours of others* and *Change working hours of others*; the keep period and its daily job. Adds TEST-WORKING-TIME.
- **WP-07 (TIME-03):** *Estimated time* and *Time left* stored on an issue; *Time spent* from the time entries; totals over sub-issues and projects that count only the issues the reader may see (PRIV-13); the three fields in the search language and as list columns.
- **WP-08 (PRIV-12, SEC-04, SEC-02, TAG-02):** a group opens as a page like a person and a role (M5-F173); one pattern for *Who sees people* and *Field permissions* and the rest of M5-F135; looser colors for label squares only (M5-F149); an issue ID always in the code font (M5-F137).
- **WP-09 (TIME-01, TIME-02, TIME-08 on screen):** the timer in the sidebar, at the top of *Home*, beside *Spent* on the issue page and in the phone's bottom bar (M3-F178); the *Stop timer* window; *Start work day*, *Start break*, *End break*, *End work day*; *Start your work day too?*; *Add time*; time entries in the history; *Start timer* in *Quick search*; the sidebar fits a window 600 px high (M5-F150); the app's bar hidden on the phone issue page (M5-F172).
- **WP-10 (TIME-04, TIME-08, TIME-09, TIME-10, TIME-12 on screen):** the weekly timesheet of UX §4.9 with fast keyboard entry, work-day rows, time on issues, the footer that compares them, warnings in plain sentences, the reason window, the history of a day, the phone timesheet, and *Working hours of others*.
- **WP-11 (TIME-09, TIME-10, TIME-12, TIME-13 settings):** *Organization settings › Working hours* (preset, rules, keep period, *Totals only*), the *Billable* switch, the defaults of *Activity* and *Cost center* in the project settings, and the security-log entries of every change (PRIV-09).
- **WP-12 (exit; TIME-01 under load, PERF-01):** sample organization part 5 (six months of working hours, about 20,000 time entries, a few running timers, an *HR* group); timers and work days in the ten-people run; time entries in the large dataset and the speed of the timesheet and the totals; the exit demo; the user docs of time and working hours.

## Why the waves are in this order

- **Wave 1** has what the rest stands on and touches none of its files. The theme files come first, as the maintainer decided, because every screen package of M6 adds rules or tokens to them. The dates and the person's day come before any time is stored: a time entry and a work day both need "the day of this moment for this person" and the one reader of lengths. The speed edges are in `bb-store` and `bb-jobs` and run beside both.
- **Wave 2** is the server of time and working hours, plus the maintainer's settings decisions. Time entries and work days are two records in two modules (ADR-0035), so WP-05 and WP-06 run side by side; the totals of WP-07 read time spent through the port WP-01 writes, which WP-05 implements. WP-08 changes only settings screens, labels and IDs, after the theme files are in.
- **Wave 3** is the screens of time. They need the routes of wave 2. WP-09 owns the time-entry form (*Stop timer*, *Add time*) and the work-day buttons; WP-10 uses the form for the details of a timesheet row and merges `main` itself after WP-09 is merged (the order of merging is WP-09, WP-10, WP-11).
- **Wave 4** puts it all on the sample, under load and on the large dataset, and runs the demo.
- **The exit visual pass** takes pictures of every changed screen in four themes on computer and phone, and a reviewer looks at each against UX §1.1 and §7 (the M5 lesson: a reviewer opens only so many, so the pass is split into sets with a list of what was looked at). The open M5 rows whose target is "the next round of the exit polish" or "the next round of pictures" (M5-F133, F134, F136, F138, F151, F153, F160, F163) are taken in this pass when their screens changed in M6, and moved on by name otherwise.
- **Exit waves** follow the exit reviews, as in M4 and M5. The lead plans them from the findings.

## File ownership

| Package | Owns |
|---------|------|
| WP-02 | `design/theme.kdl`, `design/themes/**`, `xtask/src/gen/theme*`, the generated sheets and names, ADR-0026's text of the details |
| WP-03 | `crates/bb-core/src/time/**`, the date and moment parts of `bb_core::wording` and its vectors, `web/app/src/shared/` date and length helpers, the Region settings, TXT-DATES |
| WP-04 | `crates/bb-app/src/statistics*`, the start round of `bb-jobs`, the retry of a busy write in `bb-store`, the `[known_gaps]` of `tools/checks/perf-bench.toml`, the groups by *Assigned to* in `bb-store/src/search/` |
| WP-05 | `crates/bb-app/src/time/**`, `crates/bb-store/src/time/**`, `crates/bb-api/src/api/time*`, the starter fields of time entries, its migrations |
| WP-06 | `crates/bb-core/src/working_time/**`, `crates/bb-app/src/working_hours/**`, `crates/bb-store/src/working_hours/**`, `crates/bb-api/src/api/working_hours*`, the time permissions in `bb-policy`, its migrations, TEST-WORKING-TIME |
| WP-07 | the estimate columns of the issue in `bb-store`, `crates/bb-app/src/time/totals*`, the time fields of the search catalogue, the time columns of the list |
| WP-08 | `web/app/src/features/access/**` (groups, *Who sees people*, *Field permissions*), the label square rule and its contrast line, the ID rule of the theme file |
| WP-09 | `web/app/src/features/time/timer*`, `web/app/src/features/time/entry*`, `web/app/src/features/time/work-day*`, the sidebar and the phone issue bar in `features/shell` and `features/issues` |
| WP-10 | `web/app/src/features/time/timesheet*`, `web/app/src/features/time/others*`, the route `/time` |
| WP-11 | `web/app/src/features/settings/working-hours*`, the project defaults of time entries |
| WP-12 | `tools/dev/sample.json` and `bb-sample` (time), `dataset large` (time entries), the ten-people scenarios of timers and work days, the M6 exit demo, the user docs of time |

Migrations get their numbers from the lead, in the order of merging. Ports come from each folder's block in `tools/dev/ports.json` (ADR-0023); M6 adds no port.

## Follow-ups handled in M6

| Follow-up | Package |
|-----------|---------|
| M5-F167 (theme files like the sister project), M5-F121 (a token that names another) | WP-02 |
| M5-F137, the dates and the time of day (one form per *Region*); M2-F72 (time zones checked by the server); M4-F132, the part about lengths of time (the part about keys of statuses stays with the next package that touches workflow keys) | WP-03 |
| M5-F168 (no statistics refresh in the start round, a busy write tried again during upkeep, no longer busy timeout for every writer), M5-F169, M5-F170 with M5-F89 | WP-04 |
| M5-F173 (a group as a page), M5-F135 (one pattern for *Who sees people* and *Field permissions*, and its small points), M5-F149 (looser colors for label squares only), M5-F137, the issue ID in the code font | WP-08 |
| M3-F178 (the timer on the phone's Details row and bottom bar), M5-F150 (the sidebar fits a window 600 px high), M5-F172 (the app's bar hidden while an issue is open on a phone) | WP-09 |
| M5-F10, F21, F22, F49 and the other "next package that changes …" rows | the M6 package that touches those files next; else the exit wave |
| M3-F49 (a long text for projects): its target names "documents, M6", but documents are M11 | moves on to M11 |
| M5-F137, the rest (the project icons in the rail have no words, the pen icon of the title bar): the maintainer decided only the dates and the ID font | not in M6: a question on the M6 review page |
| M5-F143 (a folder of its own for the nightly run): a new folder next to the work folders needs the maintainer's word | not in M6 (M6-Q13) |

## Checks switched on or grown in M6

- **TEST-WORKING-TIME** (new, planned → active, WP-06): the unit and property tests of the working-time rules exist by name and run (day boundaries, daylight-saving changes, zones, rest across days, a day that ends after midnight), and the two-database tests of work days run through `each_database`.
- **TXT-DATES** (new, planned → active, WP-03): a date, a moment, a time of day and a length of time on a screen are written only by the shared helpers; no `Intl.DateTimeFormat`, `toLocaleDateString`, `toLocaleTimeString` or `toLocaleString` of a date elsewhere in `web/`.
- **STY-CONTRAST** grows (WP-02, WP-08): it reads every theme file; a fill that carries no text (the label squares) has its own, looser line (M5-F149).
- **DOC-GENERATED** grows (WP-02): the sheets and names are the output of the base file and the theme files.
- **SEC-MATRIX** grows (WP-05, WP-06, WP-07): the routes of time entries, the timer, work days and *Working hours of others*; *Time spent* and the totals under field levels and visibility.
- **ARCH-PERMS** grows (WP-05, WP-06): *Book time*, *See working hours of others*, *Change working hours of others*.
- **DATA-MARKS** grows (WP-05, WP-06): every new table marked; working hours and time entries `employee_time`.
- **PERF-N1** grows (WP-05, WP-06, WP-07, WP-10): every new list route counts its statements.
- **PERF-BENCH** grows (WP-04, WP-12): the timesheet week and the totals of a project on 1 M issues; the entry of M5-F89 in `[known_gaps]` is removed when it is met.
- **TEST-CONCURRENT** grows (WP-12): "Timers and working hours start and stop at the same time" (architecture §11) runs on both databases.
- **TEST-SAMPLE-DATA** grows (WP-12): working hours and time entries of the sample hold the rules of the sample.

## Reviews

Every package gets a reviewer with a fresh context that applies the **SSOT/DRY/ODR merge gate of coding-standards §1** to the diff and the code it touches; a finding there blocks the merge, and every fix round gets its own gate check by a fresh reviewer, never by the lead or the builder who wrote it (04-milestones, "Life of a work package"). On top:

- **Security and privacy review:** WP-05, WP-06, WP-07, WP-10, WP-11 (personal data, permissions, *Who sees people*, GDPR, PRIV-09: no scores, no rankings, nothing about one person's time that a person without the permission can learn, also not through a total or a count).
- **UX review from pictures** (four themes × computer and phone): WP-02 (the pictures before and after are the same), WP-08, WP-09, WP-10, WP-11, WP-12.
- **Words review:** every package with new messages (glossary words only, plain language for people who are not technical).

## The exit demo

As in [04-milestones](../../04-milestones.md) (M6): *Ana starts her work day, runs a timer, takes a break and ends the day; the timesheet shows a rule warning on a long day; Ben can't see Ana's working hours.* Each step is a test (`web/app/e2e/m6-exit-demo.spec.ts`), with pictures in the four themes on computer and phone. M6-Q12 proposes four more steps. The outline names the sample "part 3"; M5 already made part 4, so M6 makes part 5 (a name only).

## Questions for the maintainer

Each question has the lead's recommendation. Accepting all of them takes every recommendation as written. WP-01 writes each answer where it belongs.

**Answered:** every recommendation was taken as written.

- **M6-Q1: The theme files (M5-F167, M5-F121).**
  - `design/theme.kdl` is the base theme `default`: every token with its base value, the choices (*Compact*, *Larger text*, *Less motion*), more contrast, forced colors, the contrast rules, mixins, keyframes, part rules and the design pages.
  - One file per built-in theme in `design/themes/` (`daylight.kdl`, `paper.kdl`, `dusk.kdl`, `midnight.kdl`): `theme "dusk" extends="default" kind="dark" { … }` with colors only (also the colors of shadows; the shape of a shadow stays in the base). The base holds the colors of *Daylight*, the default light theme, and each theme file sets only the colors it changes, so no color is written twice. The file name is the theme's ID; the list of themes is the files, written nowhere else.
  - **Recommendation for M5-F121:** a token may name another token (`color "hover-line" "$border-control"`), as the sister project's values do; `hover-line` and `st-ns` then name their source instead of repeating it. A loop or an unknown name is an error of `gen theme`.
  - The generated sheet is the same byte for byte before and after the split, apart from those two tokens; the pictures prove the look did not change.
- **M6-Q2: The time of day in *Region* (M5-F137).** *Region* gets a fourth choice, *Time of day*: *24-hour (14:30)* or *12-hour (2:30 PM)*. Its default follows the date format: *Match my device* takes the device's clock, every fixed date format starts with *24-hour*. Every date, moment and time of day on screens, in e-mails and in notifications is written by one helper from these choices, so a date written day first never stands beside a 12-hour clock unless the person chose both. The timesheet and the timer use the same.
- **M6-Q3: *Activity* and *Cost center* (TIME-13).**
  - Both are **starter own fields** of time entries, made at the first start (like the starter workflows) and on upgrade, which administrators may rename, change or remove like any own field. They leave the list of built-in fields.
  - Bugaboo finds them by a stable mark on the field, never by their name (a renamed *Activity* still fills the *Stop timer* window; M5-F43 showed what a name as identity costs).
  - *Activity* is a *Pick one* list: *Development, Testing, Documentation, Meetings, Support*. *Cost center* is a *Pick one* list whose choices have a number and a name (*4100 Product development*), sorted by number; it starts empty.
  - The default of an entry comes from the issue, then from its project and the projects above. Neither field is required at the start; an administrator can make either required.
- **M6-Q4: Who may book time.** A new project permission *Book time* (topic *Time*): booking one's own time on an issue one can see. The starting roles from *Contributor* up have it; *Viewer* and *Guest* do not. Changing and deleting one's own entries comes with it; other people's entries wait for TIME-06 (1.0).
- **M6-Q5: How the timer behaves (TIME-01).**
  - One running timer per person, on the server, the same on every device; closing the browser does not stop it.
  - Starting a timer on another issue is *Switch issue*: the time so far is saved as an entry with the defaults of its issue, without a window, and the new timer starts.
  - *Stop* opens the *Stop timer* window (note, *Activity*, *Cost center*, filled in from the issue). Time is kept in whole minutes; under one minute nothing is saved and the window says so.
  - A timer that runs longer than the longest day of the working-time rules (10 hours in the Germany preset; 10 hours with *No rules*) shows a warning on the timer and sends one notification. *End work day* while a timer runs asks *"Stop the timer on WEB-142 too?"*.
- **M6-Q6: *Time left* and the totals (TIME-03).**
  - *Time left* is what the person working on it typed. While nobody typed it, it shows *Estimated time* minus *Time spent*, never below zero, in the quiet style of a value worked out.
  - An issue shows its own numbers and, when it has sub-issues, the totals with them. A project shows the totals of its issues and sub-projects.
  - **A total counts only the issues the reader may see** and only the fields the reader may read, so a total never tells about a hidden issue (PRIV-13).
- **M6-Q7: *Billable* (TIME-02).** The optional mark is called *Billable* ("time that can be charged to a customer"). It is off for the organization at the start; an administrator switches it on in *Organization settings › Working hours*, and then the *Stop timer* window and the timesheet show the box.
- **M6-Q8: The Germany preset (TIME-09).**
  - The three rules of TIME-09: at most 10 hours a day; breaks of 30 minutes after 6 hours and 45 minutes after 9; 11 hours of rest between two work days.
  - **Recommendation:** also the two details of the same law (ArbZG §4): a break counts only in parts of at least 15 minutes, and nobody works more than 6 hours in a row without a break. Without them a day with three breaks of 10 minutes would pass.
  - Presets in M6: *Germany* and *No rules*. Administrators may change each number. Sundays, public holidays and the average over six months are not checked in M6 (public holidays come with M7).
- **M6-Q9: Past days, reasons and nights (TIME-08, TIME-10).**
  - A work day belongs to the date it starts on in the person's time zone and may end the next morning; it keeps its time zone.
  - A day is past from the next day on. Changing a past day asks for a reason in the person's own words (required). The old value stays in the history of the day.
  - When someone else changes a person's day, the person gets a notification (in the app, and by e-mail if they keep that on).
  - A work day that was never ended stays open; the next *Start work day* asks first *"When did your work day on Thursday end?"*. Nothing ends a day by itself, because the record must show what really happened.
- **M6-Q10: Who sees working hours (TIME-12, PRIV-09).**
  - Two organization permissions (topic *Time*): *See working hours of others* and *Change working hours of others*.
  - A new starting role *HR* holds both. **Nobody has it at the start, and the *Administrator* role does not include it.** An administrator gives *HR* to a group (for example *Payroll*). Since an administrator can give any role anyway, the no-escalation rule of ADR-0025 lets a person who decides roles for the whole organization give this one too.
  - Every gift and removal of these permissions, and every change of the working-time settings, goes into the security log (PRIV-09: "changes to these settings are recorded").
  - *Totals only* (organization setting, off at the start): with it on, *Working hours of others* shows each person's week totals only, never days, starts, ends or breaks; the team reports of 1.0 obey it too.
  - No list, page, export or search orders or ranks people by any time number, and no AI tool reads working hours (ADR-0013, COMP-10).
- **M6-Q11: How long working hours are kept (TIME-10).** The Germany preset keeps work days 2 years from the day (§16 ArbZG); with that preset the number cannot go below 2 years. A daily job deletes older days. Time entries keep no rule of their own in M6; their 10 years come with the table of *Keep and delete rules* (PRIV-05, M9).
- **M6-Q12: The exit demo.** Keep the three steps of the outline, and add four:
  - the *Stop timer* window shows *Activity* and *Cost center* taken from the issue;
  - Ana changes the end of yesterday and is asked for a reason; the history of the day keeps the old end;
  - Hana, in the group that has *HR*, sees Ana's working hours, and with *Totals only* on sees only her week total;
  - the four built-in themes look exactly as before the split into theme files.
- **M6-Q13: What stays out of M6.**
  - Reports, permissions on other people's time entries, closed months and the payroll export (TIME-05, TIME-06, TIME-07, TIME-11) stay in M13.
  - Public holidays, usual work days and away times stay in M7 (ADR-0030). M6 neither reads nor writes them.
  - The table of *Keep and delete rules* stays in M9; M6 deletes only old work days.
  - A team's view of working hours stays with the team reports (M13); M6 shows one person at a time.
  - A folder of its own for the nightly run (M5-F143) waits for the maintainer's word on a new folder; until then no full run crosses 03:00.
