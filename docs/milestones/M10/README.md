# M10 — Board, calendar, relation map and dashboards: work packages

Goal, features and exit demo: see [04-milestones.md](../../04-milestones.md) (M10). Rules: "Working in a folder", "Life of a work package", "Definition of done" and "The lead's routine" in [04-milestones.md](../../04-milestones.md), the [coding standards](../../coding-standards.md) (the SSOT/DRY/ODR review gate first), the [glossary](../../glossary.md).

**Goal:** more ways to see and plan work. A saved search shows as a board, a calendar or a map, besides the list and the timeline. A person drags a card to another column and its status changes through the workflow; drags an issue to another day and its due date changes. The relation map shows everything connected to an issue, step by step, and never a chain through an issue the person may not see. Releases say where a bug was found and where its fix is planned, and they stand on the timeline and the calendar. The timeline outlines the chain that sets the end date. Dashboards are built from ready-made boxes and shared like saved searches. A person pins what they use to the sidebar, gets a saved search by e-mail on a schedule, and sees totals for any search, grouped by a field. Every view works across a project and its sub-projects.

M10 starts from M9 (`m9-done`, version 0.6.0, the first release; API v1 frozen at `m4-done`: new routes and optional members are additions, nothing is renamed, removed or tightened). What exists:

- **Two views of a saved search.** `ViewKind` has `List` and `Timeline` (`crates/bb-core/src/saved_search.rs`); the record has room for the other kinds (ADR-0034, decision 1; M5-Q12). *Show as* draws *List* and *Timeline*; *Board*, *Calendar* and *Map* are drawn in UX design §3.1, §4.6, §4.15 and §4.16 but not built.
- **Search does more than the screens show.** `project within WEB` is a project and everything below it, and a project's list is the search with it filled in (M5-Q8). `connected to WEB-7 within 3 steps` walks the relations through issues the person reads, at most 5 steps (`crates/bb-store/src/search/filter.rs`, architecture §8). `blocks WEB-1` and `blocked by WEB-1` read the two ends of a relation.
- **Groups and totals, in part.** A search answers its groups with a count each (at most 200, M5-F44) and the totals of *Estimated time*, *Time left* and *Time spent* (`LengthTotals`, `crates/bb-app/src/search/request.rs`), never over what the reader cannot see (PRIV-13). The totals of the biggest project are a known gap on 1 M issues (`project-time-totals`, M6-F186, `tools/checks/perf-bench.toml`).
- **Relations with dates, no map.** Kinds of relation are rows the organization edits; a kind may have the date effect *Must finish first* (ADR-0043). A new loop of dependencies is refused; loops of *blocks* stored before migration 49 stay (`crates/bb-app/src/relations/dependencies.rs`).
- **No releases.** `found in release` and `planned for release` are reserved words of the search language that stand for nothing yet (ADR-0005, amendment M5, decision 11; M8-F57). The milestone of the timeline is a goal's due date (ADR-0043, decision 7, M8-Q3); rows by release wait for M10 (PLAN-04, M8 WP-06).
- **No card.** `LayoutKind` has `NewIssue` and `IssuePage` (`crates/bb-core/src/layout.rs`); the board card waits for a layout of its own (M6-F160).
- **No pins, no dashboards.** The sidebar has its sections and the project tree (`web/app/src/features/shell/Sidebar.tsx`). *Home* shows fixed blocks; its *Customize* of UX design §4.1 is not built. The start page is a setting of the person (`start.page`).
- **E-mail per issue only.** Notification e-mails are plain text, batched per issue, with no summaries (ADR-0033, decision 3; M4-Q6). The organization setting *Short e-mails* leaves out all issue text.
- **Migrations 1 to 64** (`m0064_walk_index`); decision records up to ADR-0048.

Decided already: ADR-0034 (saved searches: owned by one person, shared without giving anyone more), ADR-0043 (dates and dependencies), ADR-0030 (planning with people), ADR-0009 (our own timeline, calendar and charts; the map's own layout, amendment 2026-11-24, M10-Q10), ADR-0027 (layouts per issue type), ADR-0022 (*Who sees people*), ADR-0025 (roles and permissions), ADR-0033 (notifications and e-mail), ADR-0005 (the search language), ADR-0029 and ADR-0042 (agents and AI apps), ADR-0044 and ADR-0045 (removal, keep and delete rules), ADR-0040 (version numbers: M10 done makes 167 of 225 features, `floor(9 × 167 / 225) = 6`, so the minor number stays and the release is **0.6.1**).

**Decided by the rules, not asked** (the lead):

- Board, calendar and map are views of a saved search (ADR-0034, decision 1): a view kind and its options, stored with the search, shared by its rules, run with the rights of the person who opens it.
- Every drag has a typed way and *Undo* (UX design §4.6, §4.7, §4.16; TEST-A11Y): *Move to …* on a card, *Change due date …* on the calendar, a menu on a dashboard box.
- Nothing moves under the cursor (UX design §6.4, M8 WP-13): every new screen is measured with `expectStill`.
- The new reads use the budgets that exist: `search` (300 ms) for a board, a calendar month, a map and grouped totals; `page` (100 ms) for a list of releases, a dashboard's frame and the pins. No new budget name (SSOT, `tools/checks/perf-bench.toml`).
- "Hidden" in M10 means an issue in a project the person may not see, or a field they may not read. Restricted issues (SEC-05) come in M12, and the same tests then cover them.
- The map's chips are not called *Follow*: in the glossary *Follow* means hearing of changes to an issue. They are called *Kinds shown* (the glossary).
- The release 0.6.1 at the exit gets the long run of ten people (two hours on each database, by hand on a quiet computer), as every release does since M9 (TEST-CONCURRENT, `when` includes `release`).

## Waves

M10 runs in **five waves**: wave 0 alone, then four waves of at most four builders in `bugaboo.win2` to `bugaboo.win5`.

- **Wave 0 (the lead, docs only):** WP-01 writes the answers where they belong, the new decision records, the words and the UX frames that are missing.
- **Wave 1 (four side by side):** the server foundations, in different crates and tables: releases (WP-02), totals (WP-03), board and calendar with the card layout and views across projects (WP-04), the relation map (WP-05).
- **Wave 2 (four side by side):** what stands on them: dashboards on totals (WP-06), deadline drivers and releases on the timeline (WP-07), *E-mail me this search* (WP-08) and pinned navigation (WP-09). **WP-09 lands after WP-06**, because a dashboard can be pinned.
- **Wave 3 (four side by side):** the screens: the board (WP-10), the calendar and the releases (WP-11), the relation map (WP-12), dashboards and totals (WP-13).
- **Wave 4 (one, then one):** visual tests and the accessibility of every new view (WP-14), then the exit (WP-15): sample part 9, ten people on boards and calendars, speed on 1 M issues, the long run, the exit demo. WP-15's measurements run alone on a quiet computer.

| Wave | WP | Name | Model | Folder / branch | Features (owner in **bold**) | Depends on |
|------|----|------|-------|-----------------|------------------------------|------------|
| 0 | [WP-01](WP-01-decision-records-words-and-answers.md) | Decision records, words, the UX gaps, the maintainer's answers | Opus (lead) | `bugaboo.win1` / `WIN1` | all of M10, as docs | the answers |
| 1 | [WP-02](WP-02-releases-on-the-server.md) | Releases on the server | Opus | `bugaboo.win2` / `WIN2` | **ISS-08** | WP-01 |
| 1 | [WP-03](WP-03-totals-on-the-server.md) | Totals on the server | Opus | `bugaboo.win3` / `WIN3` | **QRY-08** | WP-01 |
| 1 | [WP-04](WP-04-board-and-calendar-on-the-server.md) | Board and calendar on the server, the card, views across projects | Opus | `bugaboo.win4` / `WIN4` | **PRJ-06**, PLAN-06, PLAN-09 (server) | WP-01 |
| 1 | [WP-05](WP-05-the-relation-map-on-the-server.md) | The relation map on the server | Opus | `bugaboo.win5` / `WIN5` | ISS-16 (server) | WP-01 |
| 2 | [WP-06](WP-06-dashboards-on-the-server.md) | Dashboards on the server | Opus | `bugaboo.win3` / `WIN3` | CFG-03 (server) | WP-03 |
| 2 | [WP-07](WP-07-deadline-drivers-and-releases-on-the-timeline.md) | Deadline drivers, and releases on the timeline | Opus | `bugaboo.win2` / `WIN2` | **PLAN-05**; ISS-08, PLAN-04 (timeline) | WP-02 |
| 2 | [WP-08](WP-08-e-mail-me-this-search.md) | *E-mail me this search* | Opus | `bugaboo.win4` / `WIN4` | **QRY-07** | WP-01 |
| 2 | [WP-09](WP-09-pinned-navigation.md) | Pinned navigation | Sonnet | `bugaboo.win5` / `WIN5` | **CFG-05** | WP-06 landed |
| 3 | [WP-10](WP-10-the-board-on-screen.md) | The board on screen | Sonnet | `bugaboo.win2` / `WIN2` | **PLAN-06** | WP-04 |
| 3 | [WP-11](WP-11-the-calendar-and-releases-on-screen.md) | The calendar and the releases on screen | Sonnet | `bugaboo.win3` / `WIN3` | **PLAN-09**; ISS-08 (screens) | WP-02, WP-04 |
| 3 | [WP-12](WP-12-the-relation-map-on-screen.md) | The relation map on screen | Opus | `bugaboo.win4` / `WIN4` | **ISS-16** | WP-05 |
| 3 | [WP-13](WP-13-dashboards-and-totals-on-screen.md) | Dashboards and totals on screen | Opus | `bugaboo.win5` / `WIN5` | **CFG-03**; QRY-08 (screens) | WP-03, WP-06 |
| 4 | [WP-14](WP-14-visual-tests-and-accessibility-of-the-views.md) | Visual tests and the accessibility of the new views | Opus | `bugaboo.win2` / `WIN2` | all views, in four themes | waves 1 to 3 |
| 4 | [WP-15](WP-15-sample-load-speed-and-exit-demo.md) | Sample part 9, ten people, speed, the long run, the exit demo | Opus | `bugaboo.win3` / `WIN3` | all of M10, proved end to end | WP-14 pushed |

Progress is kept only in [`docs/STATUS.md`](../../STATUS.md). The model column follows "The lead's routine", point 12, and the trial of M8 and M9: Opus for the server, permissions, the plan, concurrency and open UX work (the map's layout, the dashboard grid, the visual tests); Sonnet where the package is an exact brief on routes that exist (WP-09: one small table and a sidebar section; WP-10 and WP-11: screens whose frames, words and routes are fixed by WP-01 and wave 1); Haiku for the chores around them (re-runs, follow-up rows, notes commits, folder cleaning). The lead counts the fix rounds of each package and model.

## Every feature in exactly one package

PLAN-TRACE checks that every feature of the milestone is named; this table says which package **owns** each one (delivers it and proves it). Screens of a feature owned elsewhere name it too.

| Feature | Owner | Also on screen or proved in |
|---------|-------|-----------------------------|
| PLAN-05 Deadline drivers | WP-07 | WP-15 |
| PLAN-06 Board | WP-10 | WP-04 (server), WP-14, WP-15 |
| PLAN-09 Calendar | WP-11 | WP-04 (server), WP-14, WP-15 |
| ISS-16 Relation map | WP-12 | WP-05 (server), WP-14, WP-15 |
| ISS-08 Releases | WP-02 | WP-07 (timeline), WP-11 (screens), WP-15 |
| CFG-03 Dashboards | WP-13 | WP-06 (server), WP-14, WP-15 |
| CFG-05 Customizable navigation | WP-09 | WP-15 |
| PRJ-06 Across projects | WP-04 | WP-10, WP-11, WP-12, WP-13, WP-15 |
| QRY-07 "E-mail me this search" | WP-08 | WP-15 |
| QRY-08 Totals for dashboards and reports | WP-03 | WP-13 (screens), WP-15 |

PLAN-04 (rows by release, an M feature done in M8 without releases) is completed by WP-07.

## The packages in short

- **WP-01 (lead, docs only):** the answers to M10-Q1…Q22, each written where it belongs. New decision records: **ADR-0049** *Releases* (where a release lives, its states and dates, *Found in release* and *Planned for release*, who manages them, on the timeline and the calendar) and **ADR-0050** *Dashboards and totals* (boxes, sharing, what a total adds up, no box that ranks people). Amendments: ADR-0034 (the views *Board*, *Calendar* and *Map* and their options; a dashboard box as a reader of a search), ADR-0027 (the layout *Card*), ADR-0043 (deadline drivers, the new plan warnings, releases as milestones), ADR-0009 (the map's layout, M10-Q10), ADR-0033 (e-mailed searches), ADR-0025 (*Manage releases*, *Share dashboards*), ADR-0005 (the release words of the search language), ADR-0044 and ADR-0045 (dashboards, pins and e-mailed searches at *Switch off*, *Remove a person* and in the keep rules). The glossary words of M10; the UX frames that are missing; the feature list wording of CFG-03, CFG-05, QRY-08 and PRJ-06 as the answers say.
- **WP-02 (ISS-08 on the server; migration 65):** the release as a record of a project, chosen in that project and its sub-projects; its states (*Planned*, *Released*, *Retired*) and dates; the permission *Manage releases*; *Found in release* and *Planned for release* as built-in fields of the field catalogue, readable in search, forms, layouts and history, with field permissions; the reserved words of the search language get their meaning; MCP reads releases and sets the two fields through `change_issue`.
- **WP-03 (QRY-08 on the server):** one route for the totals of a search, grouped by one field: the count, *Estimated time*, *Time left*, *Time spent* and own number fields, each over the issues and values the reader may see; the groups of a board as pages when a field has more than 200 values (M5-F44); the MCP read tool `search_totals`.
- **WP-04 (PRJ-06; PLAN-06 and PLAN-09 on the server; migration 66):** `ViewKind::Board` and `ViewKind::Calendar` with their options (columns, rows, card limits; the date field); the layout *Card* with at most four fields (M6-F160); the board read (each column's count and its first page of cards, by the search's sort); the calendar read (a window of days); a card's move as the status change of the workflow or the change of a choice, through the use cases that exist; columns across several workflows; the choice *Only this project* written into the search text.
- **WP-05 (ISS-16 on the server):** the map of an issue (steps away, kinds shown, at most 200 issues) and the map of a search's results, walked only through issues the reader sees; loops of *blocks* found and named; the property tests that no chain passes through a hidden issue; `ViewKind::Map`; the MCP read tool `relation_map`.
- **WP-06 (CFG-03 on the server; migration 67):** dashboards and their boxes, owned by one person and shared with groups and projects as saved searches are, with the permission *Share dashboards*; each box a reader of a search, run as the viewer; a box that does not check for a viewer left out; the ready-made boxes as data, never as code per box.
- **WP-07 (PLAN-05; ISS-08 and PLAN-04 on the timeline):** deadline drivers worked out by the plan over the issues the reader sees; the new plan warnings of M10-Q9; releases as diamonds, *Group by release*, *Delay* for a release; the switch and the outline on the timeline, on a computer and on a phone.
- **WP-08 (QRY-07; migration 68):** a schedule on a saved search for the person themselves; the job that runs the search as the person and sends a plain-text e-mail, following *Short e-mails*; the window *E-mail me this search*, the list under *My settings › Notification settings*; stop at *Switch off*, delete at *Remove a person*; the keep rule.
- **WP-09 (CFG-05; migration 69):** pins of projects, saved searches and dashboards, the person's own, in order; a section *Pinned* in the sidebar and in the phone's *More*; *Pin to sidebar* on each pinnable page; a pin to something the person no longer sees is left out without a word.
- **WP-10 (PLAN-06 on screen):** the board of UX design §4.6: columns, rows, card limits, drag through the workflow (*Ask for* and *Must be filled in* as on the issue page), columns that fade with the reason, *Move to …*, the phone's one column with tabs, and the keys.
- **WP-11 (PLAN-09 on screen; the screens of ISS-08):** the calendar of UX design §4.16 (month and week, drag with *Undo*, *Change due date …*, releases and goals' due dates, the phone's day list); the project page *Releases*, the release window, the two fields on the issue page.
- **WP-12 (ISS-16 on screen):** the relation map of UX design §4.15: the picture, the list form, steps away, kinds shown, *Put in the middle*, the chain as sentences, loops flagged, *Show as › Map*, the phone's list first.
- **WP-13 (CFG-03 on screen; the screens of QRY-08):** *Dashboards*, the grid, *Add box* with the ready-made boxes and their previews, place and resize by drag and by keys, the charts (bars and numbers, each with a table), the totals row of a grouped list and the totals of a board column.
- **WP-14 (all views):** the new check TEST-VISUAL: reference pictures of every view of *Show as* and of a dashboard, in the four themes, on a computer and a phone; TEST-A11Y grows: every drag of the board, the calendar and the dashboard has its key; the screen-reader texts of the map's list form.
- **WP-15 (exit):** sample part 9; ten people on boards, calendars and dashboards; the new scenarios of the benchmark on 1 M issues; the long run of the release 0.6.1; the exit demo.

## Why the waves are in this order

Wave 0 first, because the answers decide what a release is, what a board column is, how a dashboard is shared and what may be e-mailed. Wave 1 lays four foundations in different places: releases a new record and two built-in fields (`bb-core`, a new `bb-app::releases`), totals the search compiler (`bb-app::search`, `bb-store::search`), board and calendar the saved search, the layouts and two new reads, the map a walk beside the relations. Wave 2 needs them: dashboards add up with totals; deadline drivers and release diamonds read releases; e-mailed searches only need saved searches and could start in wave 1, but four builders fill it; pins wait for dashboards to land. Wave 3 draws the screens once their routes answer: the four screens are in different feature folders, and each adds one entry to *Show as*. Wave 4 compares every view in the four themes once every view exists, and the exit proves everything together. WP-15's measurements start only when WP-14 has pushed its last change, because both run browser tests (one full run at a time, "The lead's routine", point 6).

**Shared files, one entry each; rebase conflicts expected:** the packages of a wave each add their entries to `crates/bb-core/src/saved_search.rs` (`ViewKind`: WP-04 *board* and *calendar*, WP-05 *map*), `crates/bb-core/src/settings/catalog.rs`, `crates/bb-store/src/migrations/mod.rs` and its list `list.rs`, `crates/bb-api/src/router.rs`, `crates/bb-policy/src/catalogue.rs`, `bb_app::jobs::JobKind`, the tool registry of `crates/bb-app/src/ai/`, `messages/en.json`, `web/app/src/shared/paths.ts`, the *Show as* switch of `web/app/src/features/search/`, and `web/app/src/features/shell/` (WP-09's section; WP-13 adds *Dashboards*), and nothing else in those files. The one who lands later rebases and keeps both. **Migrations given out:** 65 (WP-02, releases), 66 (WP-04, view options and the card layout), 67 (WP-06, dashboards), 68 (WP-08, e-mailed searches), 69 (WP-09, pins). WP-03, WP-05 and WP-07 add none; if a measurement asks one of them for an index, the lead gives out the next free number. None depends on another (architecture §7, *Storage types*).

**New dependencies:** none planned. Charts, the calendar, the board and the map's layout are our own components on the shared primitives (ADR-0009; M10-Q10 takes d3-force out). A package that finds it needs one stops and asks (coding-standards §5).

## Follow-ups handled in M10

| Follow-up | Package |
|-----------|---------|
| M8-Q3 (releases on the timeline: diamonds, rows by release, the warning for a release), the reserved release words of the search language (M8-F57) | WP-02, WP-07 |
| M5-F44 (the groups of a search as pages when a board needs more than 200) | WP-03 |
| M6-F160 (the layout of the board card, which places the three lengths of time through the same field list) | WP-04 |

**Not taken into M10:** M2-F31 (the narrow sidebar shows only the first page of the top projects): pins make it less pressing; it waits for a search in the narrow sidebar. M6-F186 (the totals of the biggest project on 1 M issues) keeps its known gap; WP-03's totals of *Time spent* read through the same statement and inherit it, and WP-15 measures them against the same limit. M5-F150 (the sidebar in a window 600 px high) is not reopened, but WP-09's section must keep it true (see Risks).

## Checks added or grown in M10

Each follows "Adding a rule" of [architecture §10](../../architecture/10-checks.md): the rule written where it belongs with `Enforced by: <ID>`, the entry in `tools/checks/catalog.toml`, the code in `xtask/src/checks/`, and its bad and good examples in `tools/checks/fixtures/<ID>/`. The META checks fail until all four are there.

- **SEC-MATRIX, the relation map** (WP-05; grown). Rule: ISS-16, PRIV-12, PLAN-08, UX design §4.15 (*Security*), written into architecture §12 (*Board, calendar, map and dashboards (M10)*). Code: the matrix gains the entry point `map` (the map of an issue and of a search) for every starting role on both databases; beside it, the named property tests `iss_16_no_chain_passes_through_a_hidden_issue` and `iss_16_the_map_is_the_map_of_the_issues_the_reader_sees` (random graphs of relations with random projects hidden from the reader: the answer equals the map of the graph with the hidden issues taken out, so no gap, no count and no line tells of them). The check fails when the property tests are missing or switched off. Examples: a map read that walks the relations without the reader's sight (bad), the walk through `sight_condition` (good).
- **TEST-VISUAL** (WP-14; new). Rule: the roadmap's "visual tests of all views in four themes", written into architecture §11 (*Visual tests*) and UX design §1.1. Catalogue: `when = ["ci"]`, files `web/app/e2e/visual/**`, `web/app/src/features/{search,board,calendar,timeline,relation-map,dashboards}/**`, `design/**`. Code: `xtask/src/checks/test_visual.rs` checks that every view kind of `ViewKind` (read from the API description, never written by hand) and the dashboard have a reference picture for each of the four themes on a computer and a phone, and that the suite runs in `cargo xtask ci`; the suite compares each picture with its reference, made from fixed data and a fixed clock, on the Windows run only (M10-Q19). Examples: a view kind without its pictures (bad), the full set (good).
- **TEST-A11Y** (WP-14, with WP-10, WP-11 and WP-13; grown). Every drag of the board, the calendar and a dashboard is named in one list of drag kinds with its typed way, as the timeline's are (`drag-kinds.ts` becomes shared); the map's list form is the screen-reader form and is measured by axe in the four themes.
- **ARCH-NO-COUNTERS and PRIV-13** (WP-03; grown). A card limit's count, a column's count, a total and a dashboard number count only what the reader sees; the check's examples grow by a total that adds hidden issues (bad).
- **ARCH-AI-TOOLS** (WP-02, WP-03, WP-05; grown by entries). The new tools `search_totals`, `relation_map` and `list_releases` declare *read* and have their tests; no tool changes a dashboard, a pin or an e-mailed search (M10-Q20).
- **DATA-MARKS** (WP-06, WP-08, WP-09; grown by entries). Dashboards, boxes, pins and schedules carry their marks and the links of the person (ADR-0044, decision 1).
- **PERF-BENCH** (WP-15; grown by scenarios): `board-biggest-project`, `calendar-month-biggest-project`, `relation-map-three-steps`, `totals-by-priority-biggest-project`, `dashboard-ready-made-boxes`, `releases-of-a-project`, each on a budget that exists.
- **TEST-CONCURRENT** (WP-15; grown by scenarios): board, calendar and dashboard cases of ten people.

## Reviews

Every package gets a code review with the SSOT/DRY/ODR gate, with a fresh reviewer for each fix round. **Security review** (Opus, fresh): WP-03 (a total over hidden values), WP-04 (columns across workflows and the move of a card), WP-05 (the walk, before its merge), WP-06 (a shared dashboard and its boxes), WP-08 (what leaves by e-mail, the link to stop it), WP-12 (the map on screen tells nothing more than its answer). **Privacy review** (Opus, fresh): WP-06 and WP-13 (no box ranks people; *Who sees people* in every box), WP-08 (e-mail content, the schedule's keep and removal), WP-09 (pins at removal). Screens (WP-07, WP-09 to WP-14) get a UX review from pictures in the four themes on computer and phone against UX design §1.1, §7 and the frames of WP-01, and a look at frame samples for anything that changes in place (§6.4).

## The exit demo

As written in 04-milestones (M10), broken into steps, with the other features proved too. Each step is a test in `web/app/e2e/m10-exit-demo.spec.ts` titled by its number and a plain sentence, with pictures in the four themes on computer and phone (WP-15).

1. **"Ana walks a blocking chain on the relation map"**: on WEB-142, *Show relation map*; she points at WEB-148 and sees its chain back to WEB-142, clicks it and reads "WEB-142 blocks WEB-147, which is blocked by WEB-148"; *Put in the middle* makes WEB-148 the start; *Show as list* gives the same chains as sentences.
2. **"Ben's map shows no chain through an issue he may not see"**: Ben has *No access* on the project of the issue between two of his; his map of the same start has no line, no gap and no count of it.
3. **"Ana drags a card across the board"**: the *Website* board, columns by status, rows by person; she drags WEB-139 from *Open* to *In progress*, the column title turns orange at "4 of 3", and the card stays; she drags it to *Testing*, whose status change asks for a value, and the window asks as the issue page does; a column its workflow does not allow fades and says why; on the phone she uses *Move to …*.
4. **"Ana drags an issue across the calendar"**: in the month view she drags WEB-140 to the next Friday, its due date changes, and *Undo* takes it back; *Change due date …* does the same with the keyboard; release *2.4* stands on its day.
5. **"A bug found in 2.4 is planned for 2.5, and the timeline warns"**: Ana sets *Found in release* and *Planned for release*; the timeline shows release *2.5* as a diamond, groups rows by release, and warns with *Delay* when an issue planned for 2.5 ends after its release date.
6. **"Deadline drivers show what sets the end of Mobile app 2.0"**: Lea switches *Deadline drivers* on; the chain that sets the end is outlined, and a loop of *blocks* left from before shows its warning.
7. **"Ana builds a dashboard from ready-made boxes and shares it"**: *Add box* offers "Open bugs by priority", "My time this week" and the others with previews; she places and resizes them, shares the dashboard with the *Website* group, and Ben sees the same boxes with his own rights; the totals of "Open bugs by priority" match the grouped list.
8. **"Ana pins her board and her dashboard"**: both stand under *Pinned* in her sidebar, in her order, and in *More* on her phone.
9. **"Ben gets his weekly search by e-mail"**: *E-mail me this search* on a saved search, every Monday at 8:00; the e-mail arrives in the test mailbox with the issues Ben may see; with *Short e-mails* on it has no titles.
10. **"One board covers Website and its sub-projects"**: the board of the project shows the issues of its sub-projects, with columns of two workflows merged by name; *Only this project* narrows it.

## Questions for the maintainer

Each question has its options and the lead's recommendation with the reason. The answers are written by WP-01 where they belong.

**Decided (2026-11-24): the maintainer took every recommendation below.** Nothing is cut up front; M10-Q21 is the order of cuts if the screens run late.

**M10-Q1: What are the columns of a board?** PLAN-06 says "columns from status or any choice field". A board across projects can meet several workflows whose statuses differ (PRJ-06).

- Options: (a) statuses only, and a board only for a search within one workflow; (b) status groups only (*Not started*, *In progress*, *Done*); (c) statuses by default, merged by name across the workflows of the search (the one rule `same_name`), with *Status group* and any choice field of one value as other choices.
- **Recommendation: (c).** It is what UX design §4.6 draws and what people expect. A card whose workflow has no status of a column cannot be dropped there: the column fades while dragging, and pointing at it says "WEB-139's workflow has no status Testing". *Status group* is the answer for a board across very different workflows. Columns come from the workflows of the issues found, in the order of the workflow of the first project; an empty column of a status stays.
- **Answer (2026-11-24):** as recommended. Board columns are statuses by default, merged by name across the workflows of the search, with *Status group* and any choice field of one value as the other choices; a column a card cannot go to fades with its reason. Written in ADR-0034, amendment 2026-11-24; the glossary (*Board*); UX design §4.6.

**M10-Q2: What does a card limit do?** UX design §4.6 says going over turns the title orange "but is allowed".

- Options: (a) a soft limit: the title shows "4 of 3" in orange, nothing is refused; (b) a hard limit: a card cannot be dropped into a full column.
- **Recommendation: (a), as drawn.** A hard limit blocks real work and makes people change statuses elsewhere. The count is of the cards the reader sees (PRIV-13), the limit is an option of the board stored with its saved search, and only the person who made the search changes it (ADR-0034, decision 5).
- **Answer (2026-11-24):** as recommended. A card limit is soft: "4 of 3" in orange, nothing refused, counting only the cards the reader sees, stored with the saved search and changed by its maker. Written in ADR-0034, amendment 2026-11-24; the glossary (*Card limit*); UX design §4.6.

**M10-Q3: May cards be put in an order by hand inside a column?**

- Options: (a) no: cards follow the sort of the search (at the start: priority, then last changed), and the column's header says so; (b) yes: a hand order per board, stored for each issue.
- **Recommendation: (a).** A hand order needs a stored rank per board and issue, which two shared boards would fight over, which grows with 1 M issues, and which a person who sees fewer issues cannot keep. Sorting by a field (priority, due date) gives the same help without it.
- **Answer (2026-11-24):** as recommended. No order by hand: cards follow the sort of the search (priority, then last changed), and the column says so. Written in ADR-0034, amendment 2026-11-24; UX design §4.6.

**M10-Q4: What does a card show, and who decides?** UX design §4.6 says "up to 4 fields (Customize → Card)"; M6-F160 asks for "a kind of layout for the card".

- Options: (a) a layout *Card* per issue type, edited in *Layouts* like the issue page (ADR-0027); (b) fields chosen per board and kept with its saved search, like a list's columns.
- **Recommendation: (a).** A card then shows the same fields wherever it appears, follows field permissions and the roles of the layout as the issue page does, and places the lengths of time through the same field list (M6-F160). *Customize › Card* on a board opens the layout for people who may change layouts; others do not see it.
- **Answer (2026-11-24):** as recommended. A card shows the fields of a layout *Card* per issue type (at most four), edited in *Layouts*; *Customize › Card* opens it for people who may change layouts. Written in ADR-0027, amendment 2026-11-24; the glossary (*Card*, *Layout*); UX design §4.6 and §5.5.

**M10-Q5: Which date does the calendar use?** PLAN-09 says "by due date"; UX design §4.16 says "Releases and milestones appear too".

- Options: (a) *Due date* only; (b) *Due date* at the start, with the view option *Dates from*: *Due date*, *Start date* or an own date field, one field and one day per issue; (c) a span from start to due date.
- **Recommendation: (b).** It mirrors *Dates from* of the timeline (ADR-0043), and the drag changes the field shown. Spans are the timeline's work. Releases stand on their release date and goals on their due date, as marks that are not dragged on the calendar.
- **Answer (2026-11-24):** as recommended. The calendar uses *Due date* at the start, and *Dates from* may choose *Start date* or an own date field; one day per issue; releases and goals stand as marks that are not dragged. Written in ADR-0034, amendment 2026-11-24; the glossary (*Calendar*, *Dates from*); UX design §4.16.

**M10-Q6: Where does a release live?** ISS-08 does not say.

- Options: (a) in one project, and offered in that project and its sub-projects; (b) one list for the whole organization; (c) in one project only.
- **Recommendation: (a).** A product with several parts plans one release for all of them by making it in their common project. The name is unique within its project. It follows the tree as permissions do (PRJ-02), and an issue moved out of the tree keeps its release, shown with a note that it is not offered there.
- **Answer (2026-11-24):** as recommended. A release lives in one project and is offered in it and its sub-projects; its name is unique in its project; an issue moved out keeps it, marked as not offered there. Written in ADR-0049, decision 1; UX design §4.4 and §4.16.

**M10-Q7: What is a release, and who manages it?**

- Options for the fields: (a) *Found in release* several, *Planned for release* one; (b) one each; (c) several each.
- Options for states: (a) *Planned*, *Released* (the day is kept) and *Archived* (no longer offered, still shown on issues); (b) any states the organization defines.
- **Recommendation: fields (a), states (a), one date *Release date*** (the planned day; *Released on* is kept when it is marked *Released*), no start date (the span is its issues). A bug can be seen in several releases but is fixed in one. A new project permission ***Manage releases***, held at the start by *Administrator* and *Project lead*. A release that an issue names cannot be deleted, only archived; one that no issue names is deleted by someone with *Manage releases* (explicit deletion).
- **Answer (2026-11-24):** as recommended. *Found in release* holds several, *Planned for release* one; the states are *Planned*, *Released* (keeps *Released on*) and *Retired* (the recommendation's *Archived*: the same meaning, but the glossary keeps *archive* out of the UI and has *Retire* for it); one *Release date*, no start date; the new permission *Manage releases* (Administrator and Project lead); a release that issues name is retired, never deleted. Written in ADR-0049, decisions 2, 3, 5 and 6; ADR-0025, amendment 2026-11-24; the glossary.

**M10-Q8: Releases on the timeline and the calendar.** ADR-0043, decision 7 and M8-Q3 promised releases "in M10".

- **Recommendation:** a release with a *Release date* is a diamond on the timeline and a mark on the calendar, like a goal's due date. *Group by release* (by *Planned for release*) joins the timeline and the board's rows. An issue planned for a release that ends after its release date shows *Delay* with the same ways out as for a goal. *Due soon* on *Home* shows releases due in the next 7 days, as UX design §4.1 says.
- **Answer (2026-11-24):** as recommended. A release with a *Release date* is a diamond on the timeline and a mark on the calendar; *Group by release*; *Delay* for a release; *Due soon* on *Home* shows releases of the next 7 days. Written in ADR-0049, decision 7; ADR-0043, amendment 2026-11-24; UX design §4.1, §4.7 and §4.16.

**M10-Q9: What do deadline drivers show, and what is an "impossible dependency"?** PLAN-05: "highlight the chain of issues that sets the end date, and warn about impossible dependencies".

- Options for the end: (a) one end, the last day of the view; (b) the end of each goal and release with a date in the view, and the last day of the view.
- **Recommendation: (b),** worked out by the plan on the server over **only the issues the reader sees** (PLAN-08): a chain through a hidden issue is not outlined, and nothing tells that one exists. Two new plan warnings for impossible dependencies: ***Loop*** for a loop of dependencies stored before M8 (nothing in it can start; its way out is *Open the relation*) and ***Cannot fit*** when an issue's fixed due date lies before the earliest day the issues it waits for can end (*Make it longer*, *Keep and accept*). The words are proposals for the glossary.
- **Answer (2026-11-24):** as recommended. Deadline drivers for each goal and release with a date in the view and the last day of the view, worked out on the server over only the issues the reader sees; the new plan warnings *Loop* and *Cannot fit*. Written in ADR-0043, amendment 2026-11-24; the glossary (*Deadline drivers*, *Loop*, *Cannot fit*, *Plan warnings*); UX design §4.7.

**M10-Q10: How is the relation map laid out?** ADR-0009 names d3-force. A force layout keeps moving while it settles, which breaks *Nothing moves under the cursor* (UX design §6.4), and gives a different picture each time, which a visual test cannot compare. UX design §4.15 draws columns by steps away.

- Options: (a) d3-force, run to rest before it is drawn; (b) our own layout in columns by steps away, the same picture for the same answer, no new dependency.
- **Recommendation: (b).** It is what the UX design draws, it never moves under the cursor, it is the same in every test, and it saves a dependency. ADR-0009 is amended.
- **Answer (2026-11-24):** as recommended. Our own layout in columns by steps away, the same picture for the same answer; d3-force is not added. Written in ADR-0009, amendment 2026-11-24; the glossary (*Relation map*); UX design §4.15.

**M10-Q11: How far does the map walk?** UX design §4.15 offers 1, 2, 3 or *Everything*, at most 200 issues. The search language allows `within 5 steps` at most.

- **Recommendation:** *Everything* means 5 steps, the one limit of the search language (SSOT, `limits.rs`), and at most 200 issues; above that the map says so and suggests fewer steps or fewer kinds. *Part of* (sub-issues) is one of the kinds shown. *Show as › Map* draws the results of a search (at most 200) and the relations between them.
- **Answer (2026-11-24):** as recommended. *Everything* is 5 steps, the one limit of the search language, and a map holds at most 200 issues; *part of* is one of the *Kinds shown*; *Show as › Map* draws a search's results and their relations. Written in ADR-0005, amendment 2026-11-24; architecture §7 (*The walk of the relation map*); the glossary (*Steps away*, *Kinds shown*); UX design §4.15.

**M10-Q12: Who may share a dashboard, and what does a viewer see?**

- Options: (a) as saved searches: private at the start, shared with groups and projects with a permission; (b) everyone may share; (c) only administrators share.
- **Recommendation: (a), with a new permission *Share dashboards*** that starts with the same roles as *Share searches*, because a dashboard can show time and work of others and an organization may want to allow one and not the other. Each box runs as the viewer; a box whose search does not check for the viewer is left out and the grid closes up, so no gap tells of it (ADR-0034, decision 4). Only the maker changes it; others *Save a copy*.
- **Answer (2026-11-24):** as recommended. Dashboards are shared like saved searches, with the new permission *Share dashboards* (the roles of *Share searches* at the start); each box runs as the viewer, and a box that does not check is left out without a gap; only the maker changes it, others *Save a copy*. Written in ADR-0050, decisions 1 to 3; ADR-0025, amendment 2026-11-24; the glossary.

**M10-Q13: Which ready-made boxes?** CFG-03 names "search results, charts, time summaries, Gantt snippet, my timer, and recent activity".

- **Recommendation:** six kinds of box (*Search results*, *Numbers and chart*, *Time totals*, *Small timeline*, *My timer*, *Recent changes*) and these ready-made ones: "My open issues", "Due soon", "Open bugs by priority", "Issues by status group", "Progress of a release", "My time this week", "Time on this search", "Small timeline of a search", "My timer", "Recent changes in this project". *Recent changes* lists changes to issues the viewer sees, never the work of one person. **No box ranks people:** a chart grouped by a person is in the order of names, never of size, and there is no "top" of people (PRIV-09, COMP-10). Charts are bars and single numbers, each with a table for screen readers; no pie charts.
- **Answer (2026-11-24):** as recommended. Six kinds of box and the ten ready-made boxes as data; no box ranks people; bars and single numbers, each with a table; no pie charts. Written in ADR-0050, decisions 4 and 5; the glossary (*Box*, *Ready-made box*, *Small timeline*, *Recent changes*); UX design §4.10.

**M10-Q14: Is *Home* a dashboard?** UX design §4.1 says *Home*'s blocks are saved searches and *Customize* reorders, removes and adds blocks; it is not built.

- Options: (a) *Home* becomes the person's own dashboard, made of the same boxes, starting with today's blocks; (b) *Home* stays fixed, dashboards are separate pages.
- **Recommendation: (a).** One kind of box instead of two, and *Customize* on *Home* is the dashboard's *Change*. It is the first cut if the milestone runs late (M10-Q21).
- **Answer (2026-11-24):** as recommended. *Home* becomes the person's own dashboard, starting with today's blocks; *Customize* on *Home* is the dashboard's *Change*. Written in ADR-0050, decision 6; UX design §4.1.

**M10-Q15: What do totals add up?** QRY-08 says "count, sum of estimated time, grouped by a field, for dashboards and reports".

- **Recommendation:** the count, *Estimated time*, *Time left*, *Time spent* and own number fields, grouped by one field. A value counts only where the reader may read the field on that issue; a total never includes a hidden issue (PRIV-13). "Reports" in M10 means the totals row of a grouped list, the totals of a board column and the *Numbers and chart* box; time reports with export stay TIME-05 (M13) and CSV export NTF-05 (M12). The feature list is reworded so.
- **Answer (2026-11-24):** as recommended. The count, *Estimated time*, *Time left*, *Time spent* and own number fields, grouped by one field, only over what the reader may see; reports are the totals row of a grouped list, the totals of a board column and the box *Numbers and chart*. Written in ADR-0050, decisions 7 and 8; the glossary (*Totals*); the feature list (QRY-08); UX design §4.2 and §4.6.

**M10-Q16: How does *E-mail me this search* work?** QRY-07: "scheduled e-mail summaries of a saved search". The e-mail leaves the organization's systems with data.

- **Recommendation:** to the person themselves only, never to others; on chosen weekdays at a chosen hour of the person's time zone; plain text with at most 50 issues (ID, title, status, link) and "and 120 more", run as the person at sending time; *Only when something is new or changed* on at the start; it follows *Short e-mails* (no titles); at most 20 schedules per person; it needs the organization's e-mail switched on; a link to the schedule in Bugaboo (sign-in needed) to stop it. It stops at *Switch off* and when the person can no longer open the search, and is deleted at *Remove a person*. An organization setting may switch the feature off (on at the start, since e-mail itself is off until set up).
- **Answer (2026-11-24):** as recommended. To the person only, on chosen weekdays at a chosen hour, plain text with at most 50 issues, *Only when something is new or changed* on at the start, following *Short e-mails*, at most 20 per person, stopped at *Switch off* and deleted at *Remove a person*, with an organization setting to switch it off. Written in ADR-0033, amendment 2026-11-24; ADR-0044 and ADR-0045, amendments 2026-11-24; the glossary (*E-mail me this search*, *E-mailed search*); UX design §4.2 and §4.13.

**M10-Q17: What can be pinned, and by whom?** CFG-05: "pin projects, saved searches and documents to the sidebar". Documents come in M11.

- **Recommendation:** each person pins for themselves only: projects, saved searches and dashboards now, documents when M11 adds them. At most 30 pins, in the person's order (drag, or *Move up* and *Move down*). No pins set by the organization for everybody in M10. A pin to something the person no longer sees is left out without a word.
- **Answer (2026-11-24):** as recommended. Each person pins projects, saved searches and dashboards for themselves, at most 30, in their own order; no pins set by the organization; a pin to something no longer seen is left out without a word. Written in ADR-0044 and ADR-0045, amendments 2026-11-24; the glossary (*Pinned*, *Pin to sidebar*); the feature list (CFG-05); UX design §3.1 and §3.3.

**M10-Q18: What does "across projects" add?** PRJ-06: "any search, board or timeline can cover a project and everything below it". Search does that already (`project within WEB`, M5-Q8).

- **Recommendation:** every view of *Show as* and every dashboard box takes any search, `project within` included, and works with several workflows, fields and ID kinds at once (M10-Q1). A project's views cover the project and its sub-projects at the start; the choice *Only this project* writes `project is WEB` instead of `project within WEB` into the search text, so the search stays the one source of what is shown.
- **Answer (2026-11-24):** as recommended. Every view and every box takes any search and works across workflows; a project's views cover its sub-projects, and *Only this project* writes `project is WEB` into the search text. Written in ADR-0034 and ADR-0005, amendments 2026-11-24; the glossary (*Only this project*); the feature list (PRJ-06).

**M10-Q19: Where are the reference pictures of the visual tests kept?** The roadmap asks for "visual tests of all views in four themes". The repository is public.

- Options: (a) in Git, compared on the Windows run only (fonts differ in WSL), with a size limit; (b) no stored pictures: pictures are only looked at in reviews, as today.
- **Recommendation: (a).** Six screens × four themes × computer and phone are 48 pictures, about 5 MB, kept under `web/app/e2e/visual/` with a limit of 10 MB checked by TEST-VISUAL. They are made from fixed data and a fixed clock; `cargo xtask visual update` makes new ones, and a changed picture is reviewed like code.
- **Answer (2026-11-24):** as recommended. The reference pictures are kept in Git under `web/app/e2e/visual/`, compared on the Windows run only, with a limit of 10 MB checked by TEST-VISUAL. Written in WP-14 and in *Checks added or grown* above.

**M10-Q20: What may AI apps and agents do with the new things?**

- **Recommendation:** new read tools `relation_map`, `search_totals` and `list_releases`; *Found in release* and *Planned for release* through `change_issue`, with its preview; a move on a board is a status change through `change_issue` as today. No tools for dashboards, pins or e-mailed searches: like saved searches they are the person's own and reached only from a session (ADR-0034, amendment M5 WP-06, item 3). Agents never manage releases in M10.
- **Answer (2026-11-24):** as recommended. The read tools `relation_map`, `search_totals` and `list_releases`; the two release fields through `change_issue`; no tools for dashboards, pins or e-mailed searches; agents never manage releases. Written in ADR-0049, decision 8; ADR-0050, decision 10; ADR-0033, amendment 2026-11-24; ADR-0025, amendment 2026-11-24.

**M10-Q21: What is cut if M10 is too big?** Ten features, three new views and dashboards.

- **Recommendation:** no cut up front. If wave 3 runs late, cut in this order: (1) *Home* as a dashboard (M10-Q14), back to a later milestone; (2) *Show as › Map* for a whole search, keeping the map of one issue; (3) *Only when something is new or changed* of e-mailed searches. The features stay whole without them.
- **Answer (2026-11-24):** as recommended. Nothing is cut up front; if the screens run late, the order of cuts is *Home* as a dashboard, then *Show as › Map* for a whole search, then *Only when something is new or changed*. Written in ADR-0050, decision 6.

**M10-Q22: What stays out of M10?**

- **Recommendation:** the list below.
- **Answer (2026-11-24):** as recommended. The list below stays out. Written in the section that follows.

## What stays out of M10 (M10-Q22)

Each item keeps its place in the feature list and the plan:

- **Live updates** of boards, calendars and dashboards (ISS-14, M15): a screen shows what it read and reads again on a change of its own, on focus and on *Refresh*.
- **Restricted issues** (SEC-05, M12); the tests of hidden issues cover them then.
- **Pinned documents** (with documents, M11); **pins set by the organization** for everybody.
- **Time reports** by person and period with export (TIME-05, M13); CSV export (NTF-05, M12).
- **A hand order of cards** (M10-Q3) and hard card limits (M10-Q2).
- **The other date effects** (*start together* and the others, PLAN-02, V1 later), baselines and export of the timeline (PLAN-07, later).
- **E-mails to other people or groups** and e-mails with attached files or pictures.
- **Pie charts** and charts of other kinds than bars and numbers.
- **Releases across organizations** and release notes written by Bugaboo.

## What the maintainer still does

- **Before wave 1:** answer M10-Q1…Q22 on the question page (link outside the repository); look at the mocks of M10-Q1, Q4, Q9, Q10, Q13 and Q14 on it. Done 2026-11-24: every recommendation taken.
- **During wave 3:** look at the first pictures of each new view when the lead posts them (UX quality gate).
- **At the exit:** try the demo, approve on the review page; the lead then signs and publishes the release 0.6.1.

## Risks

- **A drag through the workflow is three things at once.** A drop on the board can need *Ask for*, *Must be filled in* or a refusal with a reason. WP-04 moves a card only through the status change that exists, and WP-10 opens the window of the issue page; no rule of the workflow is written again in TypeScript.
- **Totals on 1 M issues.** *Time spent* totals read through the statement of M6-F186, a known gap. WP-03 measures each total separately; a grouped total over the biggest project may need its own known gap, which only the maintainer accepts.
- **The map's walk on 1 M issues.** A recursive walk with 5 steps can touch many rows before it stops at 200. WP-05 stops the walk at the cap inside the statement and measures `relation-map-three-steps` early, not only at the exit.
- **A shared dashboard as a leak.** A box that is left out must leave no trace: no gap, no count, no title. The security review of WP-06 reads every answer of a dashboard for a viewer with fewer rights.
- **E-mail is data leaving.** Plain text, *Short e-mails*, to the person only, run as them at sending time; the privacy review of WP-08 reads the e-mail against the issue page of the same person.
- **Pictures that differ by a pixel.** Visual tests break on fonts, clocks and animations. WP-14 fixes the clock and data, switches motion off, compares on the Windows run only, and measures how stable a run is (three runs in a row) before the check is switched on.
- **The sidebar's height.** Pins add lines; the sidebar must still fit a window 600 px high (M5-F150). WP-09 folds *Pinned* after a number of lines and tests the 600 px window.
- **Four screens in one switch.** WP-10 to WP-13 each add one entry to *Show as* and to `paths.ts`; the later one rebases.
- **Size.** Ten features in fifteen packages is the largest milestone after M9. The cuts of M10-Q21 are ready.
