# WP-13 — Dashboards and totals on screen

**Wave 3 · Folder / branch:** `bugaboo.win5` / `WIN5` · **Model:** Opus (a grid placed by drag and by keys, our own charts with tables, nothing moves under the cursor, people in boxes)
**Features:** CFG-03 (owner); the screens of QRY-08; PRJ-06 shown. UX design §4.10, §4.1, §2; answers M10-Q12 to Q15.

## Goal

*Dashboards* in the sidebar lists a person's dashboards and those shared with them. A dashboard shows its boxes on a grid; its maker places them by drag, resizes them by the corner, and adds boxes from *Add box*, which offers the ready-made boxes with previews. Charts are bars and single numbers, each with a table. A grouped list shows a totals row per group, and a board column its totals. On a phone the boxes stand in one column.

## Where

- **Owns:** `web/app/src/features/dashboards/**` (new, a lazy chunk: the list *Mine* / *Shared with me*, the dashboard, the grid, *Change* mode, *Add box* with previews, each kind of box, *Share*, *Save a copy*), the shared chart parts in `web/packages/ui/src/` (a bar chart and a number, each with its table), the section *Dashboards* in `web/app/src/features/shell/` and the address `/d/<id>` in `paths.ts`, *Home* as the person's dashboard in `web/app/src/features/home/`, the totals row of a grouped list in `web/app/src/features/issues/` (the list's group footer only) and the totals in a board column's header (`web/app/src/features/board/`, the header only, after WP-10 lands), the parts `dashboard-box`, `chart-bar` in `design/theme.kdl`, `userDocs/dashboards/` (new folder).
- **Reuses, does not copy:** the *Share* window of saved searches, the list of results, the timeline's drawing for *Small timeline* (read-only), the timer's block, the history's sentences for *Recent changes*, the drag helper and the shared list of drag kinds.
- **Shared files, one entry each:** `paths.ts`, `messages/en.json`, the shell's sections (WP-09 adds *Pinned*), `design/theme.kdl` (its own parts).

## What is decided

- **The grid** (UX design §4.10): 12 columns on a computer; *Change* mode shows each box's handle and corner; drag to place, drag the corner to resize, the other boxes make room only when the drop happens (nothing moves under the cursor while dragging: a shadow shows where the box will land). Each box's menu has *Move …* and *Resize …* (typed ways with arrow keys), *Change box*, *Remove*. A save has a version check.
- ***Add box*** offers the ready-made boxes of `READY_MADE_BOXES` (WP-06) with a live preview of each for the person, then the six kinds to build one's own (a search and the kind's options).
- **Charts** (M10-Q13): horizontal bars with the value written at each bar, or one large number; each has a *Show as table* that is also what screen readers read. A chart by a person field is in the order of names; there is no sort by size for it. Colors come from the theme's chart tokens and choice colors; a bar is never told apart by color alone.
- **What a viewer sees** is what the route answers: a box left out is not drawn and the grid closes up. The header says who made a shared dashboard (as *Who sees people* says) and has *Save a copy*.
- **Totals on screen** (M10-Q15): a list grouped by a field shows under each group a quiet totals row (count, and the lengths and number fields that are columns of the list), from WP-03's route; a board column's header shows its count and, when chosen in *Customize*, a sum. A total with fewer values than issues says "over 9 of 14 issues".
- ***Home*** (M10-Q14, taken; ADR-0050, decision 6): *Mentions* and *Recently opened* stay fixed blocks at the top (not boxes); below them the person's boxes, at first "My timer", "My open issues" and "Due soon"; *Customize* on *Home* is the dashboard's *Change*. The first cut if the screens run late (M10-Q21).
- **The phone:** boxes in one column in their order on the computer (row, then column); *Change* mode offers *Move up* / *Move down* instead of drag.

## Tasks

1. Write the acceptance tests below.
2. The list of dashboards, the dashboard, the grid, *Change* mode with drag, corner and the typed ways.
3. *Add box*, the previews, each kind of box.
4. Charts and tables in `packages/ui`.
5. *Share* and *Save a copy*; *Home*.
6. The totals row of grouped lists and the board column's totals.
7. User docs; pictures in the four themes on computer and phone, looked at against UX §1.1, §7 and §6.4 before the push; the report names them.

## Acceptance tests (write first)

- `cfg_03_a_dashboard_is_built_from_ready_made_boxes_with_previews` (browser).
- `cfg_03_a_box_is_placed_by_drag_and_resized_by_its_corner` (browser).
- `cfg_03_move_and_resize_do_what_a_drag_does` (browser, keyboard only).
- `cfg_03_nothing_moves_under_the_cursor_while_a_box_is_dragged` (browser, `expectStill`).
- `cfg_03_a_shared_dashboard_shows_the_viewer_their_own_answer_and_closes_gaps` (browser).
- `cfg_03_every_chart_has_a_table_and_a_person_chart_is_in_the_order_of_names` (browser).
- `cfg_03_the_phone_shows_boxes_in_one_column` (browser, phone).
- `cfg_03_customize_on_home_changes_the_persons_dashboard` (browser).
- `qry_08_a_grouped_list_shows_a_totals_row_per_group` (browser).
- `qry_08_a_board_column_shows_its_totals` (browser).
- `qry_08_open_bugs_by_priority_matches_the_grouped_list` (browser).
- TEST-A11Y: dashboards and grouped lists in the four themes; every drag of a dashboard in the list of drag kinds with its key.

## Checks

TEST-A11Y (grows), THEME-ONLY, STY-PARTS, STY-TOKENS, STY-BOLD, STY-CONTRAST, TXT-LITERAL, TXT-CATALOG, TXT-TERMS, TXT-DATES, ARCH-WEB-API, ARCH-WEB-FEATURES, PERF-BUNDLE (dashboards are a lazy chunk), LINT-TS.

## Docs to update

`userDocs/dashboards/README.md` and `building-a-dashboard.md` (new), `userDocs/issues/search.md` (totals), `userDocs/planning/board.md` (column totals), the user docs' README.

## Review

Code review with the SSOT/DRY/ODR gate: one chart part with its table, the *Share* window reused, each box drawn by the screen that exists for its data. Privacy review: no box ranks people; people as *Who sees people* says. A UX review of the pictures in the four themes on computer and phone against UX design §4.10 and WP-01's frames.

## Personal data

Boxes may show others' assignments and changes as the viewer may see them; *My time this week* shows the viewer's own time only. The screen stores nothing beyond the dashboard (WP-06).

## Depends on

WP-03 and WP-06 merged; the board column's totals after WP-10 lands.
