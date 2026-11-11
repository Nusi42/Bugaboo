# WP-07 — Work times and the time factor on screen

**Wave 2 · Folder / branch:** `bugaboo.win4` / `WIN4`
**Features:** PLAN-11, PLAN-14, PLAN-15 (on screen). [ADR-0030](../../adr/0030-planning-with-people.md) and amendment M8; UX design §4.13 (*Work times*, *Project members and their shares*) and §4.4 (*Dates and the time factor*).

## Goal

A person sees and changes their own work times in *My settings › Work times*, a person with *Enter work times for others* does the same for somebody else, a project lead sets shares on *Project members*, and the issue page shows the dates and the time factor with who set it. The words are plain and say once that Bugaboo stores no reason for an away time.

## Owns

`web/app/src/features/work-times/**` (new: the page of work times, the window *Add away time*, the line of shares, the same page for someone else), the entry of the page in `web/app/src/features/my-settings/` and on the person's page in `web/app/src/features/people/PersonPage.tsx`, the page *Project settings › Project members* in `web/app/src/features/projects/` (with its entry in the project settings menu), which reuses the project's existing role list (`ProjectRolesPage.tsx` and the data it reads) for its people: no second list of members, the rows of dates and of the time factor and the window *How long will it take you?* in `web/app/src/features/issues/` (`TimeFacts.tsx`, `TimeRows.tsx` and a new component beside them), the organization's choice of *Holidays* on `web/app/src/features/settings/WorkingHoursPage.tsx`, their messages in `messages/en.json`, the parts they need in `design/theme.kdl`, the user docs `userDocs/time/work-times.md` (new) and the page of the time factor.

## What is decided

- **The screens are UX design §4.13 and §4.4**, as written there: the seven days in the order of the person's *Region*, the summary line, the picker of *Holidays* with the next three holidays, away times with *Add away time* (whole days, *Afternoon only*, *Morning only*, a note only its person sees), the read-only line of shares with the sum over 100 %, the same page for someone else with *"You are changing Lea's work times. Lea is told."* and no note.
- **Short values save in place with *Undo*** (the UX concept): the tick and the hours of a day, the choice of *Holidays*, a share. An away time is a window, because it has several values.
- **Dates and the time factor on the issue:** *Start date* and *Due date* as details with the refusal at the box; the row *Time factor* with *"×1.25 · Lea's own estimate"* or *"set by Ada Brandt"*; the window asks *"How long will it take you?"* for the person on the bar and *"How long will it take Lea?"* for others, takes a length or a factor, shows the sum as it is typed, and adds *"Lea will be told."* for others. No list column, filter or sort offers the time factor.
- **What the screen may not see is not shown** (UX §1, *Hidden stays hidden*): a person without *Enter work times for others* has no entry on the person's page; *Work times* as *Role only* or *Hidden* shows *Away* or nothing; the row of the time factor is missing for a reader whose field level is *Hidden*, except for the person on the bar.
- **The server decides:** no rule of work times or of the time factor is written in TypeScript; the screens show what the API answers (coding-standards §4).
- **Lengths and dates** through the shared helpers, from the person's *Region* (TXT-DATES).

## Tasks

1. Write the acceptance tests below (browser and component tests).
2. *My settings › Work times* and the window *Add away time*; the same page for someone else.
3. *Project members* with the shares.
4. The dates and the time factor on the issue page, with the window.
5. The organization's choice of *Holidays* on *Work › Working hours*.
6. User docs: how work times and the time factor are used, and that no reason is stored.
7. Pictures of every new screen and window in the four themes, on computer and phone, looked at against UX §1.1 and §7 before the push; the report names them.

## Acceptance tests (write first)

- `plan_11_a_person_enters_an_away_time_and_others_see_only_away` (browser, two people).
- `plan_11_the_window_says_once_that_no_reason_is_stored` (browser).
- `plan_11_someone_with_enter_work_times_for_others_changes_lea_and_lea_is_told` (browser).
- `plan_11_the_holidays_picker_shows_the_next_three_holidays` (browser).
- `plan_15_a_project_lead_sets_a_share_on_project_members_with_undo` (browser).
- `plan_15_the_sum_over_100_percent_names_only_projects_the_reader_sees` (browser).
- `plan_14_the_person_on_the_bar_says_takes_me_10_days_and_sees_their_own_estimate` (browser).
- `plan_14_someone_else_sets_the_time_factor_and_the_issue_says_who` (browser).
- `plan_14_no_list_offers_the_time_factor_as_a_column_or_filter` (component).
- TEST-A11Y: every new screen and window, keyboard only.

## Checks

TXT-LITERAL, TXT-CATALOG, TXT-TERMS, TXT-DATES, THEME-ONLY, STY-PARTS, STY-TOKENS, STY-BOLD, ARCH-WEB-API, ARCH-WEB-FEATURES, ARCH-WEB-KEYS, TEST-A11Y, PERF-BUNDLE, LINT-TS.

## Review

Code review with the SSOT/DRY/ODR gate: one form of a day row and one picker of a person, reused from the existing screens of working hours where they fit; the messages once; no rule of the server copied into the browser. A **UX review** from the pictures in the four themes, on computer and phone, against UX design §4.13, §4.4, §1.1 and §7.

## Depends on

Wave 1 merged (WP-04: the routes of work times; WP-03: the date fields). The time factor's route comes with WP-06 of the same wave: this package builds its row against the API description of WP-06 and lands after WP-06.
