# WP-15 — The exit fixes of M6

**Exit · Folder / branch:** `bugaboo.win6` / `WIN6`
**Features:** TIME-01, TIME-04, TIME-08, TIME-10, TIME-12, PRIV-09 (fixes). The findings of the exit reviews of M6 (2026-11-07) and the follow-ups that the review asks to fix before M6 closes.

## Goal

M6 closes with no blocking finding: no history or total tells more about someone's working hours than ADR-0035 allows, a date, a time of day and a length are each written one way, and the timesheet shows what it promises on a computer.

## What is decided

The lead (2026-11-07), following ADR-0035 decision 8 and PRIV-09, so no question for the maintainer:

- A time-entry row in the history of an issue shows the entry's date, never the moment the timer stopped, to everyone but the person who booked it.
- Under *Totals only* a reader sees the totals of weeks that are over, never of the running week.
- *Change working hours of others* counts only together with *See working hours of others*.
- The keep period of working hours is a minimum: a day is deleted one day later rather than one day early.

## Tasks

1. **Privacy (server):** the four decisions above, each with a test on both databases.
2. **One "today" of the person:** one `personToday(region, now)` in `web/app/src/shared/clock.ts`, used by the timesheet, the sidebar, *Add time* and *Change start time*; the server's `zone_for` gets a case in the word vectors.
3. **One writer of a time of day, a date and a length** (TXT-DATES): the time of day built from `clockPartsOf` in the one writer (`8:43`, not `07:50`), with a vector for a one-digit hour; one weekday-and-date helper for the history title on computer and phone; the date boxes of *Add time* and *Change start time* in the person's Region (M6-F84, M6-F192); one form of a length on the timesheet and in *Add time* by UX §4.9 (M6-F148); `startOf`/`endOf` once, in `work-day-state.ts`.
4. **Limits as parameters:** the messages of the shortest and longest time entry and of the keep years take the constants, no literal "a day", "a minute", "2 years".
5. **Words:** the permission labels use the glossary names; one name for the notification of a changed work day; the wording of `time.timer.runs-long-on` for every long-timer text; examples of a time of day in the person's clock; "…· 1h worked so far"; the help of the break minutes; "Length" in the glossary or the glossary word.
6. **The timesheet on a computer:** the week *Total* visible at 1280 px (M6-F203, M6-F188); no cut words (M6-F185); level day headers and one mark of today (M6-F189, M6-F144, M6-F145); *Cancel* and *Throw away* placed as in other windows on the phone.
7. **The follow-ups the review asks to fix before M6 closes:** M6-F78, F134, F135 (the 16 px part), F136, F137, F147, F150, F164, F166, F167, F169, F174, F175, F177, F180 (decide on the timesheet), F201, F206, F210, F211, F49. Each row is marked done, or moved on with a reason.
8. **Small:** `bb-store/src/time/mod.rs:74-82` matches rows through a map, not a search inside a loop.

## Acceptance tests (write first)

- A Team member reading the history of an issue sees the date of Ana's time entry, not its hour; Ana sees her own hour.
- Under *Totals only*, the running week shows no total; last week does.
- A role with *Change* but not *See* working hours of others cannot change a day.
- A person whose Region is UTC on a device in another zone sees the same "today" in the sidebar and the timesheet.
- `8:05`, never `08:05`, in the grid and the history; the same date words on computer and phone.

## Checks

TEST-A11Y, TXT-DATES, TXT-LITERAL, the time and working-hours tests on both databases, the M6 exit demo, its pictures in four themes on computer and phone.

## Review

The SSOT/DRY/ODR gate of coding-standards §1, and a UX review of the retaken M6 demo pictures and the timesheet, the sidebar, *Add time* and the history in four themes on computer and phone.

## Depends on

M6 WP-01 to WP-14 merged (they are).
