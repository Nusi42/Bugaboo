# WP-06 — Working hours on the server

**Wave 2 · Folder / branch:** `bugaboo.win3` / `WIN3`
**Features:** TIME-08, TIME-09, TIME-10, TIME-12, PRIV-09. ADR-0035 §5 to §8, ADR-0025 amendment M6, ADR-0015, architecture §5 *Working hours* and §7 *Time*, *Legal holds*.

## Goal

Each person records when their work day started and ended and their breaks, as German law asks; the working-time rules of the organization are checked after every change and shown as plain warnings that never block; a past day changes only with a reason, the old value stays, and the person is told when someone else changed it; nobody else sees a person's working hours without the permission for it; old days are deleted only when an administrator says so, after their keep period.

## Owns

`crates/bb-core/src/working_time/**` (the rules and the presets, pure), `crates/bb-app/src/working_hours/**`, `crates/bb-store/src/working_hours/**`, `crates/bb-api/src/api/working_hours*`, the settings of working hours in the registry, *See working hours of others* and *Change working hours of others* and the starting role *HR* in `bb-policy`, the notification kind of a changed work day, the deletion of old work days, its migrations (numbers from the lead), TEST-WORKING-TIME.

## What is decided (M6-Q8, Q9, Q10, Q11)

- **A work day, its periods and a day left open:** ADR-0035, decision 5 (with `WORK_DAY_HOURS_MAX`). The next *Start work day* after a day left open is refused with a structured answer that names the open day, and the screen asks for its end first.
- **The rules as data:** ADR-0035, decision 6 (`bb_core::working_time`, one pure function: a day and the days next to it → warnings, run once per change, inside the change, stored with the day).
- **Reasons, the old value, the notification:** ADR-0035, decision 7 (with `REASON_CHARS_MAX`).
- **Who sees and who changes, *Totals only*, the security log:** ADR-0035, decision 8, and ADR-0025, amendment M6 (the permissions, the starting role *HR* and how it is given).
- **The keep period, the explicit deletion and a removed person's name:** ADR-0035, decision 7 (also the legal hold, architecture §7). No job deletes working hours.

## Tasks

1. **The rules first** (pure, in `bb-core`): the presets, the function, the plain sentences of each warning as message keys with their numbers (*"10 hours 35 minutes worked and only 15 minutes of break. The rules allow 10 hours a day at most, with 45 minutes of break after 9 hours."*). TEST-WORKING-TIME ("Adding a rule").
2. **Tables and marks:** `work_day`, `work_break`; person `identity`, the rest `employee_time`, basis legal obligation; the reasons `employee_time`.
3. **Use cases and routes:** one's own (`GET /api/v1/me/work-days?from=&to=`, `POST …/start`, `…/break/start`, `…/break/end`, `…/end`, `PATCH /api/v1/me/work-days/{date}` with an optional reason), another's (`GET /api/v1/people/{id}/work-days?from=&to=`, `PATCH …/{date}` with a required reason), the history of a day (its events, old and new values, who and why, for the person and for those who may see). The totals of a day and a week (worked, breaks) come from one function used by every answer.
4. **The difference to booked time:** a route answers, for one's own week, the working hours, the time booked on issues (through `TimeSpentStore`, the minutes of one person per date) and the difference per day; only for one's own week, never for another person.
5. **The permissions and the role:** the two permissions with their sentences and kinds, the starting role *HR*, the amended no-escalation rule, the security-log entries.
6. **Old work days:** a route that counts the work days older than the keep period (for each person and in total) and one that deletes them (all, or one person's), declared with *Change organization settings*, with the security-log entry; a removed person's days keep the name (ADR-0035, decision 7).
7. **Limits:** at most 20 breaks a day; at most one start a second; named constants.

## Acceptance tests (write first)

- TEST-WORKING-TIME: `time_09_ten_hours_and_a_minute_warns_and_is_still_recorded`, `time_09_breaks_under_fifteen_minutes_do_not_count`, `time_09_six_hours_in_a_row_without_a_break_warns`, `time_09_eleven_hours_of_rest_are_checked_against_the_day_before`, `time_09_no_rules_gives_no_warning`; property tests: a day across midnight, across the daylight-saving changes, in a zone east of UTC+12, a person who changed zone between two days.
- `time_08_start_break_end_make_one_work_day_with_its_totals`.
- `time_08_a_day_left_open_must_be_ended_before_the_next_starts`.
- `time_10_a_past_day_changes_only_with_a_reason_and_keeps_the_old_value`.
- `time_10_a_change_by_someone_else_tells_the_person`.
- `priv_09_old_working_hours_are_deleted_only_when_an_administrator_says_so` (nothing is deleted without the route; the route deletes only days older than the keep period, with their events, and writes the security log).
- `time_10_a_removed_persons_work_days_keep_the_name_for_those_who_may_see_working_hours`.
- `time_12_without_the_permission_another_persons_days_answer_like_a_person_that_does_not_exist`.
- `time_12_totals_only_answers_week_totals_and_nothing_else`.
- `time_12_an_administrator_does_not_see_working_hours_until_given_hr`.
- `priv_09_every_change_of_who_sees_working_hours_is_in_the_security_log`.
- `priv_09_working_hours_are_in_no_search_list_or_order` (the search catalogue has no field of work days; no route of work days takes an order).
- SEC-MATRIX: the routes of work days and of *Working hours of others*, every role and the *HR* group, both databases.

## Checks

TEST-WORKING-TIME planned → active. SEC-MATRIX, ARCH-PERMS, ARCH-EVENTS, ARCH-VERSIONED, DATA-MARKS, PERF-N1, ARCH-DB-PARITY, ARCH-AI-TOOLS (unchanged: no tool reads working hours).

## Review

The SSOT/DRY/ODR gate of coding-standards §1: the rules once, in `bb-core`, as data; the totals of a day once; the person's day from WP-03. A security and privacy reviewer checks every way to another person's hours (routes, notifications, e-mails, history, counts, errors that differ) and the security-log entries (PRIV-09).

## Depends on

WP-01 (ADR-0035, the ADR-0025 amendment, the words), WP-03 merged. Runs beside WP-05; the difference to booked time (task 4) reads time entries through the port, so it is built against a fake of the port and does not wait for WP-05's code. Wave 2 merges WP-05 first; WP-06 then merges `main` and runs the test of task 4 on both databases before it is pushed.
