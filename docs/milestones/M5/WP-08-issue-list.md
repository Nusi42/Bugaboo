# WP-08 — The issue list: columns, sorting, grouping, spacing, the count line, long lists

**Wave 3 · Folder / branch:** `bugaboo.win3` / `WIN3`
**Features:** CFG-02 (on screen), PERF-04 (long lists). UX design §4.2 (all of it, with the M5 additions); ADR-0009 amendment M5, ADR-0034 decision 6; follow-ups M3-F51, M3-F171, M3-F177, M4-F70.

## Goal

The issue list of UX §4.2 for every search: the columns a person chooses, sorted and grouped by any field they may read, with the count line, fast with tens of thousands of results, and on the *Search* page across projects.

## Owns

The list files of `web/app/src/features/issues/` (`IssueList.tsx`, `IssueGrid.tsx`, `issue-groups.ts`, `list-labels.ts` and what replaces them), `SelectionBars.tsx` and `use-selection.ts`, the list parts in `design/theme.kdl`, the *Customize → Columns* panel, the *Search* page, the screenshot specs of the list.

## Tasks

1. **Columns:** the project's columns (ADR-0034 decision 6) or the saved search's; *Customize → Columns* (UX §4.2), *Labels* among them (M3-F171); widths by dragging the edge, kept in the open search; sort by clicking a column name (it writes `sort by` into the search, so the search bar shows it).
2. **Grouping and spacing:** *Group by* any readable field, with the group counts of WP-04; *Spacing* *Comfortable* or *Compact*. The count line *"302 in Website · 13 shown"* (M3-F177).
3. **Long lists:** draw only the rows that can be seen with React Aria's `Virtualizer` (ADR-0009 amendment M5), loading the next page while scrolling, with the keyboard and screen-reader behaviour of the grid kept (M3-F51). One way for every long list; the people list keeps working.
4. **Tick all results** (UX §4.2): up to 5,000, then the sentence to narrow the search.
5. **The *Search* page** across projects, with the *Project* column.
6. **One place for the status-group colours** (M4-F70).
7. **Empty and error states, and the phone** (UX §4.2).
8. **Pictures:** the list in the four themes on computer and phone, grouped and not, compact and comfortable, ticked; looked at against UX §1.1 before you push; the report names the pictures you looked at.

## Acceptance tests (write first)

- `cfg_02_columns_chosen_in_customize_are_kept_in_the_saved_search`.
- `cfg_02_sorting_by_a_column_writes_sort_by_into_the_search`.
- `cfg_02_group_by_a_field_shows_each_group_with_its_count`.
- `perf_04_a_list_of_ten_thousand_draws_only_the_rows_on_screen` (the number of row elements stays small while scrolling).
- `cfg_02_tick_all_results_ticks_up_to_five_thousand` (the many-issues bar, M5-Q9).
- The speed project: the first page of the *Search* page on the sample within PERF-01.

## Depends on

WP-04 and WP-06 merged. Runs beside WP-07 and WP-09.
