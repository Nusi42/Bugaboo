# WP-10 — The timesheet

**Wave 3 · Folder / branch:** `bugaboo.win3` / `WIN3`
**Features:** TIME-04; TIME-08, TIME-09, TIME-10, TIME-12 on screen; PRIV-09. UX design §4.9 (the timesheet as drawn) and the phone timesheet and *Working hours of others* that WP-01 draws.

## Goal

A person sees and fills in their week in one grid: the work-day rows (*Start*, *End*, *Breaks*, *Working hours*) above the time on issues (one row per issue with *Activity* and *Cost center*, one column per day), and a footer that compares the two. Typing is fast (`1:30`, `1.5`, `90m`), a broken rule is a plain sentence under its day, and a past day asks for a reason. A person with *See working hours of others* opens the same view for one other person.

## Owns

`web/app/src/features/time/timesheet*`, `web/app/src/features/time/others*`, the route `/time` (and `/time/people/<id>` for others), the keyboard shortcut `g` `t`, the theme rules of these parts, the user docs of the timesheet and of working hours.

## What is decided (M6-Q8, Q9, Q10)

- The grid of UX §4.9 on a computer; on a phone one day at a time with the week's totals on top (WP-01 draws it).
- A cell of a work-day row takes a time of day in the person's *Time of day*; a cell of an issue row takes a length (WP-03's reader, with the person's decimal mark). *Enter* and *Tab* move as in a spreadsheet; a wrong value is marked in place with a plain message and nothing is saved until it reads.
- A change to a past day opens the reason window before it saves; the day shows *Changed* with its history (who, when, old and new, why).
- Warnings are the sentences of WP-06 under their day, in the warning color, never as a block.
- The footer: *Booked on issues* and *Working hours not booked on issues*, per day and for the week (the route of WP-06).
- *Working hours of others*: a person picker (only for those with the permission), the work-day rows read only, or with *Change* when the person has *Change working hours of others* (with the reason window always, and the note that the person will be told); with *Totals only*, only the week totals. No list of people with numbers, no order by any number.

## Tasks

1. **The grid** with the week picker (*First day of the week* of the person), the work-day rows and the issue rows, the new row (*Add an issue*, finding an issue by ID or title as the issue field does), the details of a row through the time-entry form of WP-09.
2. **Keyboard entry:** every cell reachable and editable by keyboard; a value saves on leaving the cell, *Saving…* then *Saved*, a conflict explained (ARCH-VERSIONED answers).
3. **Warnings, reasons and the history of a day.**
4. **The footer** and the totals of the week.
5. **The phone timesheet.**
6. ***Working hours of others*** with the picker, read only or with *Change*, and *Totals only*.
7. **Pictures** of the timesheet (a normal week, a week with a warning, a past change with its reason, *Totals only*, empty) in four themes on computer and phone, looked at against UX §1.1 and §7 before the push; the report names them.

## Acceptance tests (write first)

- `time_04_typing_1_30_or_1_5_or_90m_books_ninety_minutes`.
- `time_04_the_whole_week_can_be_filled_in_with_the_keyboard_alone`.
- `time_08_working_hours_typed_afterwards_into_the_work_day_rows`.
- `time_09_a_long_day_shows_its_warning_in_plain_words_and_is_saved`.
- `time_10_changing_yesterday_asks_for_a_reason_and_the_day_keeps_its_history`.
- `time_12_ben_cannot_open_anas_working_hours` (the address answers like a person that does not exist; no picker).
- `time_12_hr_sees_anas_week_and_with_totals_only_only_the_week_total`.
- `time_04_the_footer_shows_working_hours_not_booked_on_issues`.
- TEST-A11Y on the timesheet in every state.

## Checks

TEST-A11Y, THEME-ONLY, STY-PARTS, STY-BOLD, TXT-DATES, TXT-TERMS, PERF-BUNDLE (the timesheet is a lazy part), PERF-N1 (the routes it calls).

## Review

The SSOT/DRY/ODR gate of coding-standards §1: the time-entry form of WP-09, the reader of WP-03, the totals and warnings from the server (never worked out again in the browser). A security and privacy reviewer tries every way to another person's hours from the browser. A UX reviewer compares every picture with UX §4.9, §1.1 and §7.

## Depends on

Wave 2 merged. Builds the grid and the work-day rows first; merges `main` after WP-09 is merged and then uses its time-entry form for the details of a row.
