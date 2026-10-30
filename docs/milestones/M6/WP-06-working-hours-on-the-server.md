# WP-06 — Working hours on the server

**Wave 2 · Folder / branch:** `bugaboo.win3` / `WIN3`
**Features:** TIME-08, TIME-09, TIME-10, TIME-12, PRIV-09. ADR-0035 §5 to §8, ADR-0025 amendment M6, ADR-0015, ADR-0017 (`working-hours.delete-old`), architecture §5 *Working hours* and §7 *Time*, *Legal holds*.

## Goal

Each person records when their work day started and ended and their breaks, as German law asks; the working-time rules of the organization are checked after every change and shown as plain warnings that never block; a past day changes only with a reason, the old value stays, and the person is told when someone else changed it; nobody else sees a person's working hours without the permission for it; old days are deleted when their keep period ends.

## Owns

`crates/bb-core/src/working_time/**` (the rules and the presets, pure), `crates/bb-app/src/working_hours/**`, `crates/bb-store/src/working_hours/**`, `crates/bb-api/src/api/working_hours*`, the settings of working hours in the registry, *See working hours of others* and *Change working hours of others* and the starting role *HR* in `bb-policy`, the notification kind of a changed work day, the daily job, its migrations (numbers from the lead), TEST-WORKING-TIME.

## What is decided (M6-Q8, Q9, Q10, Q11)

- **A work day:** person, date (the person's day of its start, WP-03), the time zone it was recorded in, start, end (may be the next morning; at most 24 hours after the start), breaks (start and end each), the warnings, a version. *Start work day* makes it; *Start break*, *End break*, *End work day* add to it; any of them may be typed afterwards.
- **A day left open** stays open; the next *Start work day* is refused with a structured answer that names the open day, and the screen asks for its end first.
- **The rules are data** (`bb_core::working_time`): a preset (*Germany*, *No rules*) with numbers an administrator may change (registry settings at the organization level). Germany: at most 10 hours a day; 30 minutes of break after 6 hours, 45 after 9; a break counts only in parts of at least 15 minutes; at most 6 hours in a row without a break; 11 hours of rest between two days. One pure function: a day (and the day before it) → warnings. It runs once per change, inside the change, and its warnings are stored with the day.
- **A past day** (any date before today in the person's zone) changes only with a reason (required, plain text, at most 500 characters), kept in the event with the old value. A change by another person needs *Change working hours of others* and sends the person a notification (in the app; by e-mail as their settings say, on at the start).
- **Who sees:** one's own always. Another person's days need *See working hours of others* (organization permission, topic *Time*); changing them needs *Change working hours of others*. The starting role *HR* holds both and is given to nobody; *Administrator* does not include them; a person who may decide roles for the whole organization may give *HR* (ADR-0025 amendment M6). With *Totals only* on, the routes for others answer week totals only. Every gift or removal of these permissions and every change of a working-hours setting is in the security log.
- **The keep period:** an organization setting, 2 years in the Germany preset and never below it while that preset is chosen; the daily job `working-hours.delete-old` deletes days older than it, idempotently, also of removed people (legal hold until then, architecture §7).

## Tasks

1. **The rules first** (pure, in `bb-core`): the presets, the function, the plain sentences of each warning as message keys with their numbers (*"10 h 35 min worked and only 15 min break. The rules allow 10 hours a day at most, with 45 min break after 9 hours."*). TEST-WORKING-TIME ("Adding a rule").
2. **Tables and marks:** `work_day`, `work_break`; person `identity`, the rest `employee_time`, basis legal obligation; the reasons `employee_time`.
3. **Use cases and routes:** one's own (`GET /api/v1/me/work-days?from=&to=`, `POST …/start`, `…/break/start`, `…/break/end`, `…/end`, `PATCH /api/v1/me/work-days/{date}` with an optional reason), another's (`GET /api/v1/people/{id}/work-days?from=&to=`, `PATCH …/{date}` with a required reason), the history of a day (its events, old and new values, who and why, for the person and for those who may see). The totals of a day and a week (worked, breaks) come from one function used by every answer.
4. **The difference to booked time:** a route answers, for one's own week, the working hours, the time booked on issues (through `TimeSpentStore`, the minutes of one person per date) and the difference per day; only for one's own week, never for another person.
5. **The permissions and the role:** the two permissions with their sentences and kinds, the starting role *HR*, the amended no-escalation rule, the security-log entries.
6. **The job:** `working-hours.delete-old`, daily (ADR-0017), with its idempotency test.
7. **Limits:** at most 20 breaks a day; at most one start a second; named constants.

## Acceptance tests (write first)

- TEST-WORKING-TIME: `time_09_ten_hours_and_a_minute_warns_and_is_still_recorded`, `time_09_breaks_under_fifteen_minutes_do_not_count`, `time_09_six_hours_in_a_row_without_a_break_warns`, `time_09_eleven_hours_of_rest_are_checked_against_the_day_before`, `time_09_no_rules_gives_no_warning`; property tests: a day across midnight, across the daylight-saving changes, in a zone east of UTC+12, a person who changed zone between two days.
- `time_08_start_break_end_make_one_work_day_with_its_totals`.
- `time_08_a_day_left_open_must_be_ended_before_the_next_starts`.
- `time_10_a_past_day_changes_only_with_a_reason_and_keeps_the_old_value`.
- `time_10_a_change_by_someone_else_tells_the_person`.
- `time_10_days_older_than_the_keep_period_are_deleted_and_a_second_run_does_nothing`.
- `time_12_without_the_permission_another_persons_days_answer_like_a_person_that_does_not_exist`.
- `time_12_totals_only_answers_week_totals_and_nothing_else`.
- `time_12_an_administrator_does_not_see_working_hours_until_given_hr`.
- `priv_09_every_change_of_who_sees_working_hours_is_in_the_security_log`.
- `priv_09_working_hours_are_in_no_search_list_or_order` (the search catalogue has no field of work days; no route of work days takes an order).
- SEC-MATRIX: the entry points `work-days` and `others-working-hours`, every role and the *HR* group, both databases.

## Checks

TEST-WORKING-TIME planned → active. SEC-MATRIX, ARCH-PERMS, ARCH-EVENTS, ARCH-VERSIONED, DATA-MARKS, PERF-N1, ARCH-DB-PARITY, ARCH-AI-TOOLS (unchanged: no tool reads working hours).

## Review

The SSOT/DRY/ODR gate of coding-standards §1: the rules once, in `bb-core`, as data; the totals of a day once; the person's day from WP-03. A security and privacy reviewer checks every way to another person's hours (routes, notifications, e-mails, history, counts, errors that differ) and the security-log entries (PRIV-09).

## Depends on

WP-01 (ADR-0035, the ADR-0025 amendment, the words), WP-03 merged. Runs beside WP-05; the difference to booked time (task 4) reads time entries through the port, so it is built against a fake of the port and does not wait for WP-05's code. Wave 2 merges WP-05 first; WP-06 then merges `main` and runs the test of task 4 on both databases before it is pushed.
