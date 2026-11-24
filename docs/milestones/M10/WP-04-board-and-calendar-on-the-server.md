# WP-04 — Board and calendar on the server, the card, views across projects

**Wave 1 · Folder / branch:** `bugaboo.win4` / `WIN4` · **Model:** Opus (saved searches shared by rules, several workflows at once, the move of a card through the workflow)
**Features:** PRJ-06 (owner); PLAN-06 and PLAN-09 on the server (the screens are WP-10's and WP-11's). ADR-0034 and ADR-0027 amendments (WP-01); answers M10-Q1, Q2, Q3, Q4, Q5, Q18; follow-up M6-F160.

## Goal

A saved search can be shown as a board or a calendar, with its options stored and shared by the rules of saved searches. The server answers a board as columns (and rows) with their counts and first cards, and a calendar as the issues of a window of days by the chosen date field. A card's move is the status change of its workflow or the change of a choice, through the use cases that exist. The card shows the fields of the layout *Card*. Every view takes a search across a project and its sub-projects, with several workflows at once.

## Where

- **Owns:** `ViewKind::Board` and `ViewKind::Calendar` and their options in `crates/bb-core/src/saved_search.rs` (columns: *Status*, *Status group* or a choice field of one value; rows: none or a field; card limits per column; for the calendar *Dates from* and the first view, month or week), `LayoutKind::Card` in `crates/bb-core/src/layout.rs` and its starting layout (ID, title and at most four fields), `crates/bb-app/src/boards/**` (new: the board read, the columns across workflows, the move), `crates/bb-app/src/calendar/**` (new: the read of a window of days), the store reads beside the search (`crates/bb-store/src/search/`: reads of a column's page and of a date window only), migration **66** (`crates/bb-store/src/migrations/m0066_board_calendar_card.rs`: the view options and the card layout's rows), the routes `POST /api/v1/board` and `POST /api/v1/calendar` (`crates/bb-api/src/api/`), the choice *Only this project* in the search text (`crates/bb-app/src/search/`, the helper that writes `project is` or `project within`).
- **Shared files, one entry each:** `saved_search.rs` (WP-05 adds *map*), `migrations/mod.rs` and `list.rs`, `router.rs`, `messages/en.json`.
- **Does not touch:** the workflow's rules (`crates/bb-app/src/workflows/**`: called, not changed), any screen.

## What is decided

- **Views of a saved search** (ADR-0034, decision 1): the options are part of how the search is shown; only its maker changes them; a receiver opens it with their own rights. A column, row or limit that names a value the receiver cannot know (a status of a workflow they do not see, a person they see by role only) makes the search not check for them, as any other name does (ADR-0034, amendment M5 WP-06, item 8).
- **Columns** (M10-Q1): *Status* merges statuses of the same name across the workflows of the issues found, in the order of the workflow of the first project; *Status group* has its three columns; a choice field of one value has one column per choice and *None*. Empty columns stay.
- **Card limits** (M10-Q2): soft; the answer gives each column's count of the issues the reader sees and its limit, and `over: true` when the count is above it. Nothing is refused.
- **Order** (M10-Q3): cards follow the sort of the search (at the start: priority, then last changed). There is no stored rank.
- **The board read:** per column (and per row), the count and the first page of cards (at most 50, with a cursor for *Show more*), each card with the fields of its type's layout *Card* that the reader may read. Groups over 200 page as WP-03 pages them (M5-F44); WP-04 reads through that code and does not write a second grouping.
- **The move** of a card is one call of the status change that exists (with its *Ask for*, *Must be filled in* and refusals with reasons, ADR-0031) or of the field change; the board route answers, for a card being dragged, which columns it may go to and why not for the others (`may_move`, from the workflow's allowed changes), so the screen fades them with the reason. No rule of the workflow is written again.
- **The calendar read** (M10-Q5): the issues whose chosen date field lies in a window of at most 6 weeks, each on its day, at most 200 per window (the answer says when there are more), plus the releases with a *Release date* and the goals with a *Due date* in the window as marks (releases come from WP-02; until it lands, the marks are goals only). A move is the change of that date field, with its version check (*Keep Ben's / Use mine*).
- **The card layout** (M10-Q4, M6-F160): a layout kind of its own, per issue type and place like the others (ADR-0027), placing at most four fields from the one field list, the lengths of time among them. The starting layout shows *Priority*, *Assigned to* and *Due date*.
- **Across projects** (PRJ-06, M10-Q18): every read takes any search; a project's views start with `project within KEY`; *Only this project* writes `project is KEY`. The answer of the board names, per card, the workflow it follows only by what the reader may see.

## Tasks

1. Write the acceptance tests below.
2. The two view kinds and their options; migration 66 on both databases; the round trip of a saved board and calendar.
3. The layout *Card*: the kind, the starting layout, the field list; M6-F160 closed.
4. The board read: columns across workflows, rows, counts, first pages, `may_move`; the move through the existing use cases.
5. The calendar read and the move of a date.
6. *Only this project* as search text.
7. Architecture §8 (*Views of a search*), ADR-0034 and ADR-0027 *Enforced by* lines; a first measurement of `board-biggest-project` and `calendar-month-biggest-project` in architecture §11.

## Acceptance tests (write first)

- `plan_06_a_saved_search_keeps_its_board_columns_rows_and_card_limits` (both databases).
- `plan_06_status_columns_merge_statuses_of_the_same_name_across_workflows`.
- `plan_06_a_card_may_not_move_to_a_status_its_workflow_lacks_and_the_answer_says_why`.
- `plan_06_a_move_is_the_status_change_of_the_workflow_with_ask_for_and_must_be_filled_in`.
- `plan_06_a_move_by_a_choice_field_is_the_field_change_with_its_permissions`.
- `plan_06_a_card_limit_counts_only_what_the_reader_sees_and_refuses_nothing`.
- `plan_06_cards_follow_the_sort_of_the_search`.
- `plan_06_a_card_shows_only_the_layout_fields_the_reader_may_read` (M6-F160).
- `plan_06_a_shared_board_that_names_an_unknown_status_does_not_check_for_the_receiver`.
- `plan_09_the_calendar_answers_a_window_of_days_by_the_chosen_date_field`.
- `plan_09_moving_an_issue_changes_that_date_and_an_old_version_is_refused_with_the_newer_value`.
- `plan_09_goals_and_releases_stand_on_their_dates_as_marks`.
- `prj_06_a_project_view_covers_its_sub_projects_and_only_this_project_narrows_it`.
- `prj_06_a_board_across_projects_shows_each_card_with_its_own_workflow`.
- SEC-MATRIX: the board and calendar routes, for every starting role, both databases.
- PERF-N1: a board of 6 columns and 4 rows makes a fixed number of statements.

## Checks

DATA-MIGRATIONS, ARCH-DB-PARITY, ARCH-VERSIONED, ARCH-API-FREEZE, ARCH-NO-COUNTERS, ARCH-PERSON-REF, SEC-MATRIX, TEST-SEARCH-ROUNDTRIP, TEST-WORKFLOWS, PERF-N1, PERF-BENCH, TXT-TERMS.

## Docs to update

Architecture §8 and §11; ADR-0034 and ADR-0027 amendments. The user docs come with the screens (WP-10, WP-11).

## Review

Code review with the SSOT/DRY/ODR gate: the move calls the one status change, the groups read through WP-03's paging, the card fields come from the one field list. Security review: a column of a workflow the reader does not see, a shared board under fewer rights, a move refused with a reason that names nothing hidden.

## Personal data

None new: the view options belong to the saved search (ADR-0034, decision 8). A row by person follows *Who sees people*.

## Depends on

WP-01 merged. Reads WP-03's paging of groups once it lands; until then it uses the existing groups with their cap.
