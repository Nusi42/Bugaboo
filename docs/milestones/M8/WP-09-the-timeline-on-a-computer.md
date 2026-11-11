# WP-09 — The timeline on a computer: rows, zoom, bars, drag with Undo, dependencies, the tray

**Wave 3 · Folder / branch:** `bugaboo.win3` / `WIN3`
**Features:** PLAN-01, PLAN-02, PLAN-03, PLAN-04 (on screen). UX design §4.7; [ADR-0043](../../adr/0043-dates-and-dependencies.md); ADR-0009 (our own timeline component, no outside chart library); ADR-0034; follow-ups M3-F163, M2C-F105.

## Goal

*Show as › Timeline* draws a saved search as UX design §4.7 shows it on a computer: rows grouped and folded, zoom from day to quarter, the Today line, bars in the status colors with their done part, dependency arrows, the diamonds of goals' due dates, and the tray *Not scheduled yet*. A person moves and resizes bars by dragging, with *Undo*, and every drag has a typed alternative (*Set dates…*, *Add dependency…*).

## Owns

`web/app/src/features/timeline/**` (new, a lazy chunk: the scale, the rows, the bars, the arrows, the tray, the card of a bar, *Set dates…*, *Add dependency…*), the shared drawing parts in `web/packages/ui/src/` that the timeline needs (a scale of days and the drag helper of `web/app/src/shared/drag-and-drop.tsx`, reused), *Show as* in `web/app/src/features/search/` (the choice *Timeline* and the view's options: dates from, group by, plan by effort, zoom), the part `timeline-bar` and the timeline's parts in `design/theme.kdl`, the position tokens marked `run-time="true"` that the bars need, messages, the user docs `userDocs/planning/timeline.md` (new).

## What is decided

- **Our own component** (ADR-0009): built on the shared primitives (the virtualized rows of the list, the drag helper, a scale of days); no outside chart library.
- **Positions without inline styles** (M3-F163, M2C-F105): a bar's start and length are custom properties of its part, set only through `variableOf` on tokens that the base theme file marks `run-time="true"` (the one exception of coding-standards §4; THEME-ONLY). Colors are never set from code.
- **What the screen draws comes from the plan** (WP-06, `POST /api/v1/timeline`): rows, bars, worked-out days, arrows, diamonds and the count of the tray. The browser holds no rule of the plan (ADR-0030, amendment M8). While a bar is dragged it only moves; on release it saves the dates (WP-03) and draws the plan the server answers.
- **Rows:** grouped by the view's *Group by* (project, goal, person, a choice field), foldable, each with its status icon, ID and title; virtualized like the list. **Zoom:** *Days · Weeks · Months · Quarters*; *Today* jumps to today; weekends and days off shaded.
- **Bars:** status tints and priority edges as in the list (R4), the done part darker, a goal's span as a bracket and its due date as a diamond, the Today line in the accent color.
- **Drag** (PLAN-03): move a bar, or drag an end to change one date; the change is judged per issue as the person (an issue they may not change cannot be dragged, and pointing at it says why); every drag ends with *Undo* (*"WEB-280 moved 1 week later · Undo"*). Dragging the dot at a bar's end to another bar makes a dependency of the kind the organization gave *Must finish first* (*blocks* at the start).
- **Typed alternatives** (TEST-A11Y): the card of a selected bar and its menu have *Set dates…* (two date boxes), *Add dependency…* (the usual issue finder) and *Open*; arrow keys move a selected bar by a day, with *Shift* by a week; every action of a drag has a key.
- **The tray *Not scheduled yet*:** at the side, the issues of the search without dates, paged from the same route; dragging one onto the chart gives it dates, and *Set dates…* does the same.
- **Restricted issues are left out** (PLAN-08): the screen shows what the plan answers and nothing else; no empty rows.

## Tasks

1. Write the acceptance tests below.
2. The scale, zoom and Today; the rows with grouping and folding, virtualized.
3. Bars, the done part, brackets, diamonds and arrows, from the plan.
4. Drag to move and resize with *Undo*, the dependency dot, and the typed alternatives with their keys.
5. The tray *Not scheduled yet*.
6. *Show as › Timeline* and the view's options, saved with the search.
7. User docs; pictures in the four themes on a computer (and the narrow window), looked at against UX §1.1 and §7 before the push; the report names them.

## Acceptance tests (write first)

- `plan_01_a_saved_search_shows_as_a_timeline_with_its_dates` (browser).
- `plan_01_a_view_with_own_date_fields_draws_and_drags_those_fields` (browser).
- `plan_02_an_arrow_joins_an_issue_that_must_finish_first_to_the_one_that_waits` (browser).
- `plan_02_dragging_the_dot_makes_a_dependency_and_add_dependency_does_the_same` (browser).
- `plan_03_dragging_a_bar_moves_its_dates_and_undo_takes_it_back` (browser).
- `plan_03_an_issue_the_person_may_not_change_cannot_be_dragged_and_says_why` (browser).
- `plan_03_set_dates_does_what_a_drag_does` (browser, keyboard only).
- `plan_04_rows_group_fold_and_zoom_from_day_to_quarter` (browser).
- `plan_04_a_goal_with_a_due_date_is_a_diamond` (browser).
- `plan_01_an_issue_from_not_scheduled_yet_gets_dates_by_drag_or_by_set_dates` (browser).
- TEST-A11Y: a keyboard alternative for every drag of the timeline (the check grows here).

## Checks

TEST-A11Y (grows: every drag of the timeline has a key), THEME-ONLY, STY-PARTS, STY-TOKENS, STY-BOLD, STY-CONTRAST, TXT-LITERAL, TXT-CATALOG, TXT-TERMS, TXT-DATES, ARCH-WEB-API, ARCH-WEB-FEATURES, PERF-BUNDLE (the timeline is a lazy chunk), LINT-TS.

## Review

Code review with the SSOT/DRY/ODR gate: one scale of days, one drag helper, the rows of the list reused, positions only through `variableOf`, no rule of the plan in TypeScript. A **UX review** from the pictures in the four themes against UX design §4.7, §1.1 and §7: alignment of rows and bars, the tints, the Today line, the tray.

## Depends on

Wave 2 merged (WP-06: the plan and the timeline route; WP-03 before it).
