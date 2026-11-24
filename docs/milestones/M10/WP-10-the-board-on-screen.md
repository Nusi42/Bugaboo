# WP-10 — The board on screen

**Wave 3 · Folder / branch:** `bugaboo.win2` / `WIN2` · **Model:** Sonnet (a screen on routes that exist, its frames and words fixed by UX design §4.6 and WP-01)
**Features:** PLAN-06 (owner); PRJ-06 shown. UX design §4.6, §3.3; answers M10-Q1 to Q4.

## Goal

*Show as › Board* draws a saved search as UX design §4.6 shows it: columns with their counts and card limits, optional rows, cards with the fields of the layout *Card*. A person drags a card to another column and its status changes through the workflow; columns it cannot go to fade while dragging and say why; every card has *Move to …*. On a phone the board shows one column at a time with the column names as tabs.

## Where

- **Owns:** `web/app/src/features/board/**` (new, a lazy chunk: columns, rows, the card, the drag, *Move to …*, the board's *Customize* panel for columns, rows and card limits, the phone tabs), the entry *Board* of *Show as* in `web/app/src/features/search/`, the parts `board-column`, `board-card` and their states in `design/theme.kdl`, `userDocs/planning/board.md` (new).
- **Reuses, does not copy:** the drag helper of `web/app/src/shared/drag-and-drop.tsx` and the list of drag kinds of the timeline (made shared by this package or WP-11, whichever lands first; the other rebases), the status change window of the issue page (*Ask for*, *Must be filled in*), the *Undo* host, the virtualized rows of the list for long columns.
- **Shared files, one entry each:** the *Show as* switch, `paths.ts`, `messages/en.json`, `design/theme.kdl` (its own parts).

## What is decided

- **Columns and rows** come from the board route (WP-04); the screen holds no rule of the workflow. The title of a column says its name and count, and "4 of 3" when a limit is set; over the limit the title is drawn in the warning state (orange) and nothing is refused (M10-Q2). Rows fold; each row's header has its count.
- **Cards** show the issue ID, the title and the fields the route gives (layout *Card*, M10-Q4), tinted by status group and edged by priority as list rows are (R4). A long column shows its first page and *Show more*.
- **Drag** (PLAN-06): while a card is dragged, the columns `may_move` refuses fade, and pointing at one says the route's reason ("WEB-139's workflow has no status Testing"). A drop calls the status change; when the change asks for something (*Ask for*, *Must be filled in*), the window of the issue page opens; cancelling it leaves the card where it was. A drop ends with *Undo* ("WEB-139 moved to In progress · Undo"). A stale move asks as the issue page does (*Keep Ben's change / Use mine*).
- **No hand order** (M10-Q3): dropping inside the same column does nothing; the column header's menu names the sort.
- ***Move to …*** on every card (and in its menu, and on a phone): the columns as choices, the faded ones with their reason. Keys: arrow keys move the focus between cards and columns; *Space* picks a card up, arrows choose a column, *Enter* drops, *Esc* cancels (TEST-A11Y: every drag of the board has its key).
- ***Customize*** (the maker of the search only): columns (*Status*, *Status group*, a choice field), rows (none or a field), card limits per column; *Customize › Card* opens the layout *Card* for people who may change layouts.
- **The phone:** one column at a time, the column names as tabs with their counts; cards move with *Move to …*; nothing is dragged.

## Tasks

1. Write the acceptance tests below.
2. Columns, rows, cards from the route; the lazy chunk.
3. Drag with fading and reasons, the status change window, *Undo*; *Move to …* and the keys.
4. *Customize*; *Show more*.
5. The phone.
6. User docs; pictures in the four themes on computer and phone, looked at against UX §1.1, §7 and §6.4 before the push; the report names them.

## Acceptance tests (write first)

- `plan_06_a_saved_search_shows_as_a_board_with_columns_rows_and_counts` (browser).
- `plan_06_dragging_a_card_changes_its_status_and_undo_takes_it_back` (browser).
- `plan_06_a_column_the_workflow_does_not_allow_fades_and_says_why` (browser).
- `plan_06_a_drop_that_asks_for_a_value_opens_the_window_of_the_issue_page` (browser).
- `plan_06_over_the_card_limit_the_title_turns_orange_and_the_card_stays` (browser).
- `plan_06_move_to_does_what_a_drag_does` (browser, keyboard only).
- `plan_06_customize_sets_columns_rows_and_card_limits_for_the_maker_only` (browser).
- `plan_06_the_phone_shows_one_column_with_tabs_and_moves_by_move_to` (browser, phone).
- `prj_06_a_board_across_sub_projects_merges_status_columns_by_name` (browser).
- `plan_06_a_stale_move_asks_keep_bens_change_or_use_mine` (browser).
- TEST-A11Y: the board in the four themes; every drag of the board in the list of drag kinds with its key.
- `expectStill`: a drag ends with no card or column moving except the card moved.

## Checks

TEST-A11Y (grows), THEME-ONLY, STY-PARTS, STY-TOKENS, STY-BOLD, STY-CONTRAST, TXT-LITERAL, TXT-CATALOG, TXT-TERMS, ARCH-WEB-API, ARCH-WEB-FEATURES, PERF-BUNDLE (the board is a lazy chunk), LINT-TS.

## Docs to update

`userDocs/planning/board.md` (new) and the README of the planning docs.

## Review

Code review with the SSOT/DRY/ODR gate: one drag helper, one list of drag kinds, the issue page's status window reused, no workflow rule in TypeScript. A UX review of the pictures in the four themes on computer and phone against UX design §4.6.

## Personal data

None new. Rows by person and the person on a card follow *Who sees people* as the route answers.

## Depends on

WP-04 merged (the board route, the card layout).
