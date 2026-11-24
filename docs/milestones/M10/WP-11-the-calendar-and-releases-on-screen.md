# WP-11 — The calendar and the releases on screen

**Wave 3 · Folder / branch:** `bugaboo.win3` / `WIN3` · **Model:** Sonnet (two screens on routes that exist, frames and words fixed by UX design §4.16 and WP-01)
**Features:** PLAN-09 (owner); the screens of ISS-08; PRJ-06 shown. UX design §4.16, §3.3, §2; answers M10-Q5 to Q8.

## Goal

*Show as › Calendar* draws a saved search on a month or week calendar by the chosen date field, with releases and goals' due dates as marks. A person drags an issue to another day and its date changes, with *Undo*; every issue also has *Change due date …*. On a phone the calendar is a day-by-day list. A project has its page *Releases*, where people see its releases and those with *Manage releases* make and change them, and the issue page shows *Found in release* and *Planned for release*.

## Where

- **Owns:** `web/app/src/features/calendar/**` (new, a lazy chunk: month, week, the day cell with *+5 more*, the drag, *Change due date …*, the marks, the phone list, *Dates from* in the view's options), the entry *Calendar* of *Show as* in `web/app/src/features/search/`, `web/app/src/features/releases/**` (new: the project page *Releases*, the release window, mark *Released*, *Retire the release*, *Delete*), the project's sub-page in the project routes, the two fields' editors on the issue page and the new-issue form (`web/app/src/features/issues/`, the release pickers only), the parts `calendar-day`, `calendar-item`, `release-mark` in `design/theme.kdl`, `userDocs/planning/calendar.md` and `userDocs/issues/releases.md` (new).
- **Reuses, does not copy:** the drag helper and the shared list of drag kinds (see WP-10), the date writers of TXT-DATES, the first day of the week from *Region*, the *Undo* host, the issue finder, the choice picker of the issue page.
- **Shared files, one entry each:** the *Show as* switch, `paths.ts`, `messages/en.json`, `design/theme.kdl` (its own parts).

## What is decided

- **Month and week** (UX design §4.16): *‹ October 2026 ›*, *Today*, *Month · Week*; today highlighted; the first day of the week from the person's *Region*. A day with more issues than fit shows *+5 more*, which opens the day's list. Each issue shows its status icon, ID and title, tinted by status group (R4).
- **Dates from** (M10-Q5): *Due date* at the start, or *Start date*, or an own date field, stored with the view's options by the maker.
- **Drag:** to another day changes the field shown; the drop ends with *Undo* ("WEB-140 due date moved to Fri 16 Oct · Undo"); a stale move asks *Keep Ben's change / Use mine*. An issue the person may not change cannot be dragged, and pointing at it says why. ***Change due date …*** (named by the field shown) on every issue's menu: a date box. Keys: arrow keys move a picked issue by a day, with *Shift* by a week (TEST-A11Y).
- **Marks** (M10-Q8): releases on their *Release date* and goals on their *Due date*, as marks with a diamond; they are not dragged here.
- **The phone:** a day-by-day list from today, days without issues left out, *Change due date …* on each; nothing is dragged.
- **Releases page** (M10-Q6, Q7): the releases of the project and those offered from above, grouped by state (*Planned*, *Released*, *Retired* folded), each with its *Release date*, its count of issues planned for it that the reader sees, and a link to them as a search. *Create a release*, *Change*, *Mark as released*, *Retire the release*, *Delete* for people with *Manage releases*; a refused delete says "Issues name this release. Retire it instead." The issue page's *Found in release* and *Planned for release* use the choice picker with the releases offered there.

## Tasks

1. Write the acceptance tests below.
2. Month and week from the calendar route; *+N more*; marks.
3. Drag with *Undo*, *Change due date …*, the keys.
4. *Dates from*; the phone list.
5. The page *Releases*, the release window, the two fields on the issue page and the new-issue form.
6. User docs; pictures in the four themes on computer and phone, looked at against UX §1.1, §7 and §6.4 before the push; the report names them.

## Acceptance tests (write first)

- `plan_09_a_saved_search_shows_as_a_month_and_a_week_calendar_by_due_date` (browser).
- `plan_09_dragging_an_issue_to_another_day_changes_its_due_date_and_undo_takes_it_back` (browser).
- `plan_09_change_due_date_does_what_a_drag_does` (browser, keyboard only).
- `plan_09_dates_from_a_start_date_or_an_own_date_field_draws_and_drags_that_field` (browser).
- `plan_09_an_issue_the_person_may_not_change_cannot_be_dragged_and_says_why` (browser).
- `plan_09_releases_and_goals_stand_on_their_dates_as_marks` (browser).
- `plan_09_the_week_starts_on_the_first_day_of_the_persons_region` (browser).
- `plan_09_the_phone_shows_a_day_by_day_list` (browser, phone).
- `iss_08_manage_releases_makes_marks_retires_and_deletes_on_the_releases_page` (browser).
- `iss_08_a_refused_delete_offers_retire` (browser).
- `iss_08_found_in_and_planned_for_are_set_on_the_issue_page_and_the_new_issue_form` (browser).
- `iss_08_a_person_without_manage_releases_sees_the_page_without_changes` (browser).
- TEST-A11Y: the calendar and *Releases* in the four themes; every drag of the calendar in the list of drag kinds with its key.
- `expectStill`: a drop moves only the issue dropped.

## Checks

TEST-A11Y (grows), THEME-ONLY, STY-PARTS, STY-TOKENS, STY-BOLD, STY-CONTRAST, TXT-LITERAL, TXT-CATALOG, TXT-TERMS, TXT-DATES, ARCH-WEB-API, ARCH-WEB-FEATURES, PERF-BUNDLE (the calendar is a lazy chunk), LINT-TS.

## Docs to update

`userDocs/planning/calendar.md` and `userDocs/issues/releases.md` (new), the READMEs of both folders, `userDocs/issues/issue-page.md` (the two fields).

## Review

Code review with the SSOT/DRY/ODR gate: one drag helper and list of drag kinds, the date writers of TXT-DATES, the choice picker reused. A UX review of the pictures in the four themes on computer and phone against UX design §4.16 and WP-01's frame of *Releases*.

## Personal data

None new.

## Depends on

WP-02 (releases) and WP-04 (the calendar route) merged.
