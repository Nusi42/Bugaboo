# WP-06 — The plan on the server: bars, plan by effort, the time factor, warnings

**Wave 2 · Folder / branch:** `bugaboo.win3` / `WIN3`
**Features:** PLAN-08, PLAN-10, PLAN-12, PLAN-13, PLAN-14 (on the server), PLAN-04 (rows). [ADR-0030](../../adr/0030-planning-with-people.md) decisions 1, 3, 4, 5 and amendment M8 (M8-Q4, Q5, Q7); [ADR-0043](../../adr/0043-dates-and-dependencies.md) decisions 2, 3, 5, 7; ADR-0034.

## Goal

The server answers the timeline: for a saved search (or an unsaved one) and a span of days, the rows, the bars with their days, the dependencies, the plan warnings and the count of *Not scheduled yet*, worked out once in Rust for the reader. *Plan by effort* gives bars the days their estimate and time factor need, and the time factor is the person's own estimate on the issue. Nothing moves by itself; every way out is a person's choice.

## Owns

The pure rules of the plan in `crates/bb-core/src/plan/` (the days of a bar, plan by effort, the four warnings, the order along dependencies), the use case in `crates/bb-app/src/plan/**` (it reads issues through the compiled search, dates and dependencies from WP-03, work times through `WorkTimesStore` from WP-04), the field *Time factor* with who set it (`crates/bb-core/src/fields.rs`, `crates/bb-app/src/issues/`), the time factor and the accepted warnings in the store with migration 52 (`crates/bb-store/src/migrations/m0052_time_factor_and_accepted_warnings.rs`), the view kind `ViewKind::Timeline` and its options in `crates/bb-core/src/saved_search.rs` and `crates/bb-app/src/saved_searches/view.rs`, the routes in `crates/bb-api/src/api/timeline*.rs`, the notification *Your time factor was changed*, the rule of the tool registry for the time factor (`crates/bb-app/src/ai/tools/`).

## What is decided

- **The timeline is a view of a saved search** (ADR-0034): `ViewKind::Timeline` with its options stored on the saved search: the date fields (*Start date* and *Due date*, or two own date fields), *Group by* (project, goal, person, or a choice field; release waits for M10), *Plan by effort* on or off, and the zoom. A project's timeline is the view of its default search.
- **One route answers the plan** (`POST /api/v1/timeline`): a search (the ID of a saved search or its text), the view's options and a span of days; it answers a page of rows (at most `TIMELINE_ROWS_MOST`, with a cursor), each row's bars with their days, the done part, the person on the bar (as `PersonRef`), the hatched days (away, holiday, day off) as *Who sees people* allows, the dependencies between issues on the page, the warnings of the issues on the page, and the count of *Not scheduled yet* (whose issues come from the same route, paged). Within PERF-01's budget for timeline data (p95 under 300 ms on 1 M issues).
- **Restricted issues are left out entirely** (PLAN-08): no bar, no arrow, no warning, no count; a dependency on a hidden issue is neither drawn nor checked; a date field the reader cannot read counts as empty (ADR-0043, decision 2).
- **The days of a bar** as ADR-0043, decision 3; worked-out days are answered, never stored. The done part is *Estimated time* minus *Time left* over *Estimated time* (none without an estimate; a bar of the status group *Done* is done).
- **Plan by effort** (ADR-0030, decision 3): estimate × time factor, spread over the days of the person on the bar, each day with the minutes `WorkTimesStore` gives for the project (work hours × share, nothing on away days, days off and holidays). A due date never moves; an issue without a person or an estimate keeps its set dates.
- **The person on the bar** (amendment M8): the first assigned person shapes the bar; *Group by person* puts the bar in the row of each assigned person, with a mark on the copies that are not the first person's; *Too much at once* counts the issue for the first person only. With *Role only* for *Assigned to* there is no grouping by person (ADR-0030, decision 1).
- **The time factor** (decision 4, amendment M8): a field of the catalogue with field levels (start: *Can change* for Project lead and Team member, *Hidden* for Guest) and who set it; the person on the bar may always change it on their issue. It is given as a factor or as a length (*Takes me 10 days*, worked out against *Estimated time*). It goes back to ×1 in the same change that changes the first assigned person, and the history says why. It has no search word, no list column and no total; no tool of the registry changes it; the person on the bar is told when somebody else changes it (ADR-0033, amendment M8).
- **The four warnings** (decision 5, ADR-0043 decisions 5 and 7), each with the issue, the days and the ways out that fit it (UX §4.7, *The list of plan warnings*): *Away*, *Not enough days*, *Delay* (a broken dependency, or a sub-issue that ends after its goal's due date), *Too much at once*. *Move it after* answers a preview of every issue and its new dates first, then applies them as one run through WP-03's route; *Make it longer* answers the due date it needs and changes it only when the person confirms; *Give it to someone else…* is the usual change of *Assigned to*, with nobody suggested; *Keep and accept* stores the accepted warning (issue, kind, the dates it was accepted for, who), so it shows faded until the dates of its issue change.
- **Personal data** as architecture §12, *Personal data in M8*: the time factor with who set it; who accepted a warning (`identity`, `contract`).

## Tasks

1. Write the acceptance tests below.
2. The rules of the plan in `bb-core`, as tables of cases (the days of a bar, plan by effort, each warning, the order along dependencies).
3. The time factor: the field, who set it, the reset with the first person, the length form, the notification, the rule of the registry.
4. `ViewKind::Timeline` and its options; the route with its pages and cursor; the accepted warnings.
5. The ways out: the preview and run of *Move it after*, *Make it longer*, *Keep and accept*.
6. A first measurement of the route on the large dataset (`cargo xtask bench`, the timeline scenario that WP-12 finishes), on a quiet computer when the lead says so.

## Acceptance tests (write first)

- `plan_08_a_restricted_issue_has_no_bar_no_arrow_no_warning_and_no_count`.
- `plan_10_the_timeline_shows_people_only_as_who_sees_people_allows`.
- `plan_10_the_first_assigned_person_shapes_the_bar`.
- `plan_10_group_by_person_marks_the_copies_of_other_assigned_people`.
- `plan_12_plan_by_effort_counts_work_hours_share_away_days_and_holidays` (6 hours, 50 %, 9 hours: three days).
- `plan_12_a_fixed_due_date_never_moves`.
- `plan_13_a_delay_is_shown_along_dependencies_and_moves_nothing`.
- `plan_13_a_sub_issue_after_its_goals_due_date_gives_the_goal_a_delay`.
- `plan_13_move_it_after_previews_then_moves_as_one_run`.
- `plan_13_an_accepted_warning_comes_back_when_its_dates_change`.
- `plan_13_a_hidden_person_is_named_only_as_the_assigned_person`.
- `plan_14_the_time_factor_resets_when_the_person_changes` (the first person; a change of the others leaves it).
- `plan_14_the_person_on_the_bar_may_always_set_the_time_factor`.
- `plan_14_no_report_or_search_sorts_by_time_factor`.
- `plan_14_no_tool_of_the_registry_changes_the_time_factor`.
- SEC-MATRIX: the timeline route and the time factor, for every starting role and each choice of *Who sees people*, both databases.

## Checks

ARCH-PURE, ARCH-PERSON-REF, ARCH-AUTHZ-ONLY, ARCH-AI-TOOLS, ARCH-EVENTS, ARCH-DB-PARITY, DATA-MARKS, SEC-MATRIX, PERF-N1, PERF-BENCH, ARCH-API-FREEZE, TXT-TERMS.

## Review

Code review with the SSOT/DRY/ODR gate: the rules of the plan live once in `bb-core` (none in the browser, ADR-0030 amendment M8), the minutes of a day come only from `WorkTimesStore`, the loop and order along dependencies reuse WP-03's walk, the warnings are one enum with their sentences in the one wording. A **privacy review**: what a warning, a card and a row by person show of another person's work times; that the time factor is never searched, sorted, totalled or set by AI; the marks (COMP-10, PRIV-09).

## Depends on

Wave 1 merged: WP-03 (dates, dependencies, the run of several moves), WP-04 (`WorkTimesStore`).
