# WP-09 — The timer and the work day on screen

**Wave 3 · Folder / branch:** `bugaboo.win2` / `WIN2`
**Features:** TIME-01, TIME-02, TIME-08 (on screen); CFG-08 (*Start timer* in *Quick search*); UX-01, UX-05. UX design §3.3, §4.9, §6 (the phone); follow-ups M3-F178, M5-F150, M5-F172.

## Goal

The timer and the work day are one tap away wherever a person is: at the bottom of the sidebar on a computer, at the top of *Home* and in the bottom bar on a phone, beside *Spent* on the issue page. Stopping the timer asks only what is needed, with *Activity* and *Cost center* already filled in. Starting a timer before the work day asks *"Start your work day too?"*.

## Owns

`web/app/src/features/time/timer*`, `web/app/src/features/time/entry*` (the time-entry form, used by *Stop timer*, *Add time* and, in WP-10, the details of a timesheet row), `web/app/src/features/time/work-day*` (the four buttons and the line *"Working since 8:02 · 6h 51m so far"*), the sidebar and the phone bottom bar in `features/shell`, the phone issue bar and the time rows of the issue page in `features/issues`, *Quick search* actions of time, the keyboard shortcut `t`, the theme rules of these parts, the user docs of the timer.

## What is decided (M6-Q5, the maintainer on M5-F150 and F172)

- The timer shows the issue, the running time and *Stop*; tapping it opens *Stop · Switch issue · Change start time · Throw away*.
- *Stop timer* is a window with the sentence *"You worked 42 min on …"*, *Activity*, *Cost center*, *What did you do? (optional)*, *Billable* when the organization has it on, and *Throw away* / *Save time* (UX §4.9). Under one minute it says nothing will be saved.
- A timer past the longest day of the rules shows a warning on itself (and the notification of WP-05 arrives).
- *End work day* while a timer runs asks *"Stop the timer on WEB-142 too?"*.
- **M5-F150:** the sidebar fits a window 600 px high with the timer and the work-day buttons: first shorter notes for empty sections and no heading for an empty *Saved searches*; if that is not enough, the group at the bottom folds.
- **M5-F172:** while an issue is open on a phone, the app's bar is hidden; *Back* in the top bar leads out. The issue's own bar (*Status · Comment · Timer · More*) is the only bar, so one *More* shows (M3-F178: the *Timer* in that bar and on the Details row).

## Tasks

1. **The time-entry form** once (`entry*`): fields from the starter fields by their mark, the length helper and the day of WP-03, required fields from the field definitions; used by *Stop timer* and *Add time* (the issue page: date, length, note, *Activity*, *Cost center*).
2. **The timer part** in the sidebar, on *Home* and in the phone bottom bar; the running time counts from the server's start moment (never from the browser's clock alone); it updates from the server when the page gets focus, so a timer started on the phone shows on the computer.
3. **The work-day part:** *Start work day*, *Start break*, *End break*, *End work day*; the line of today's hours; the question for a day left open (*"When did your work day on Thursday end?"*); *Start your work day too?*.
4. **The issue page:** the play button beside *Spent*; *Add time*; the time entries in the history with the words of WP-05; the phone's issue bar with *Timer* and the hidden app bar (F172, F178).
5. **The sidebar** within 600 px (F150), with a browser test at 600 px height on a computer.
6. ***Quick search*:** *Start timer* (on the open issue or a chosen one), *Stop timer*; the key `t` (start or stop on the open issue) in *Keyboard shortcuts*.
7. **Pictures** of every changed screen in four themes on computer and phone, looked at against UX §1.1 and §7 before the push; the report names them.

## Acceptance tests (write first)

- `time_01_the_timer_started_on_the_phone_shows_on_the_computer` (two browser contexts).
- `time_01_stop_timer_shows_activity_and_cost_center_from_the_issue`.
- `time_01_switch_issue_from_the_timer_keeps_one_running_timer`.
- `time_08_starting_a_timer_before_the_work_day_asks_to_start_it_too`.
- `time_08_start_break_end_break_and_end_work_day_from_the_sidebar`.
- `time_02_add_time_on_the_issue_page_books_a_length_on_a_date`.
- `ux_01_the_sidebar_fits_a_window_600_px_high` (M5-F150).
- `ux_01_on_a_phone_an_open_issue_shows_one_bar_and_one_more_button` (M5-F172).
- `cfg_08_quick_search_starts_and_stops_the_timer`.
- TEST-A11Y on every changed screen; the keyboard alone starts, stops and saves.

## Checks

TEST-A11Y, THEME-ONLY, STY-PARTS, STY-BOLD, TXT-DATES, TXT-TERMS, PERF-BUNDLE (the timer is in the first page; the form and the window are lazy).

## Review

The SSOT/DRY/ODR gate of coding-standards §1: one time-entry form, one timer part used in three places, the lengths and days from WP-03, no second copy of the defaults rule (the server gives the defaults). A UX reviewer compares every picture with UX §4.9, §1.1 and §7. A words reviewer reads every new message.

## Depends on

Wave 2 merged (WP-05, WP-06, WP-07, WP-08). Merges first in wave 3; WP-10 takes the time-entry form from `main` after this merge.
