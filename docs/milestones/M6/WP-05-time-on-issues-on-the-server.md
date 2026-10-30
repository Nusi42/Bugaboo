# WP-05 — Time on issues on the server: the timer, time entries, *Activity* and *Cost center*

**Wave 2 · Folder / branch:** `bugaboo.win2` / `WIN2`
**Features:** TIME-01, TIME-02, TIME-13; PRIV-09 for time entries. ADR-0035 §1 to §4, ADR-0006 (own fields of time entries), ADR-0015, ADR-0017 (`timers.long-running`), ADR-0022 (the kind *Time entries*), ADR-0025 amendment M6 (*Book time*).

## Goal

A person's time on issues is kept on the server: one running timer that every device shows and that a closed browser does not stop, time entries made by the timer or by hand, each with *Activity* and *Cost center* filled in from the issue or its project, and nobody learns about another person's time more than *Who sees people* and the field levels allow.

## Owns

`crates/bb-app/src/time/**` (but `totals*`, which is WP-07's), `crates/bb-store/src/time/**`, `crates/bb-api/src/api/time*`, the starter fields of time entries (made by the system use case of the first start and on upgrade), the permission *Book time* in `bb-policy`, the notification kind of a long timer, its migrations (numbers from the lead), the implementation of `TimeSpentStore` (shape from WP-01).

## What is decided (M6-Q3, Q4, Q5, Q7)

- **The timer:** one per person (a unique key on the person), on an issue the person may see and book on. *Start*, *Stop* (saves an entry), *Switch issue* (saves the time so far with the defaults, no window, starts the new one), *Change start time* (not in the future, not before the end of the person's last entry), *Throw away* (saves nothing). Whole minutes; under one minute nothing is saved. Every change is a command with an event and a version.
- **A time entry:** person, issue, date (the person's day of its start, WP-03), length in minutes (1 to 1,440), an optional note (plain text, at most 2,000 characters, shown as text and never as HTML), *Billable* (only while the organization switch is on), own fields through `field_value` (ADR-0006), a version. Made by the timer or by hand (*Add time*, with a date and a length); changed and deleted only by its person in M6.
- ***Activity* and *Cost center*** are starter own fields of time entries, found by a stable mark on the field, never by name. *Cost center*'s choices have a number and a name. The default of an entry: the issue's value, then its project's, then the projects above (inherited). An administrator can make either required; an entry without a required value is refused with a message that names the field.
- ***Book time*** is a project permission, starting with *Contributor* and above. The routes declare it (ARCH-PERMS).
- **The long timer:** a job `timers.long-running`, queued when a timer starts, at its start plus the longest day of the organization's working-time rules (WP-06 reads it; until WP-06 is merged, the constant of the Germany preset), sends one notification and marks the timer; a timer stopped before then cancels nothing that matters (the job finds no timer and ends).

## Tasks

1. **Tables and marks:** `timer`, `time_entry`; every column marked (DATA-MARKS): person `identity`, the rest `employee_time`, basis contract.
2. **Use cases and routes:** the timer (`GET/POST /api/v1/me/timer`, `…/stop`, `…/switch`, `…/start-time`, `…/throw-away`), time entries (`GET /api/v1/me/time-entries?from=&to=` for one's own week, `POST/PATCH/DELETE` of one's own), the time entries of an issue (`GET /api/v1/issues/{id}/time-entries`: each person as `PersonRef` by the kind *Time entries*; the lengths only when the reader may read *Time spent*). Paged with cursors, lean fields (ARCH-LEAN), PERF-N1.
3. **The starter fields:** the system use case makes *Activity* and *Cost center* for time entries with their mark, once, with events; an upgrade of an existing server makes them too. The built-in `Field::Activity` and `Field::CostCenter` leave the catalogue (the generated field table of architecture §7 follows). The issue and the project get the two fields as defaults (their value is the default of an entry).
4. **The history:** a time entry made, changed or deleted is an event of its issue; the history shows it in time order (*"Ana booked 1:30 · Development"*), filtered by the reader's levels and *Who sees people*.
5. **The port of time spent:** `TimeSpentStore` as WP-01 wrote it, on both databases.
6. **Limits:** at most one start or switch a second per person; at most 200 entries a day per person; a note's length; all as named constants with their messages.

## Acceptance tests (write first)

- `time_01_one_person_has_one_running_timer_on_every_device` (two sessions start at once: one timer, one refusal with the conflict).
- `time_01_switch_issue_saves_the_time_so_far_with_the_defaults_of_its_issue`.
- `time_01_a_timer_under_one_minute_saves_nothing`.
- `time_01_a_long_timer_sends_one_notification`.
- `time_02_a_time_entry_by_hand_takes_a_date_a_length_and_the_defaults`.
- `time_13_activity_and_cost_center_are_found_by_their_mark_after_a_rename`.
- `time_13_the_default_comes_from_the_issue_then_the_project_then_the_projects_above`.
- `time_13_a_required_cost_center_refuses_an_entry_without_one`.
- `priv_09_a_reader_without_names_for_time_entries_sees_someone_and_never_a_name`.
- `priv_09_a_reader_who_may_not_read_time_spent_gets_no_lengths`.
- `priv_09_no_time_route_orders_or_ranks_people` (reads the generated API description: no route of time takes an order by person or by a total).
- SEC-MATRIX: the entry points `timer` and `time-entries`, every role, both databases.

## Checks

SEC-MATRIX, ARCH-PERMS, ARCH-PERSON-REF, ARCH-EVENTS, ARCH-VERSIONED, DATA-MARKS, PERF-N1, ARCH-DB-PARITY.

## Review

The SSOT/DRY/ODR gate of coding-standards §1: the length and the day from WP-03 (never a second calculation), the defaults read once, the fields found by their mark only. A security and privacy reviewer checks every answer for a person's time that the reader may not see, and that no answer orders people.

## Depends on

WP-01 (ADR-0035, the port shape, the words), WP-03 merged. Runs beside WP-06, WP-07, WP-08.
