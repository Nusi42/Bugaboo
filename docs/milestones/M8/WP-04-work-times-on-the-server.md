# WP-04 — Work times on the server: usual work days, work hours, shares, away times, public holidays

**Wave 1 · Folder / branch:** `bugaboo.win4` / `WIN4`
**Features:** PLAN-11, PLAN-15 (on the server). [ADR-0030](../../adr/0030-planning-with-people.md) decision 2 and amendment M8 (M8-Q6, M8-Q13); ADR-0017, ADR-0025, ADR-0033 (amendments M8).

## Goal

Bugaboo knows when and how much each person works: their usual work days and work hours, their share in each project, their away times and the public holidays that count for them. The data is guarded as employee data, others see an away time only as *Away*, and nothing is kept longer than ADR-0030 says. The plan (WP-06) and the screens (WP-07) read it through one module.

## Owns

The pure rules in `crates/bb-core/src/work_times/` (the records, the half days, the sum of shares) and the holidays in `crates/bb-core/src/holidays/` (Easter, the fixed days, the 16 states, the test vectors of 2026 to 2030), the use cases in `crates/bb-app/src/work_times/**`, the port and its store in `crates/bb-store/src/work_times*`, migration 50 (`crates/bb-store/src/migrations/m0050_work_times.rs`), the routes in `crates/bb-api/src/api/work_times*.rs`, the permission *Enter work times for others* in `crates/bb-policy/src/catalogue.rs`, the kind *Work times* in `crates/bb-core/src/people_kinds.rs`, the setting `work-times.holidays` in `crates/bb-core/src/settings/`, the two notification kinds in `crates/bb-app/src/notifications/`, the two daily jobs in `crates/bb-app/src/jobs/`.

## What is decided

- **Records** (ADR-0030, decision 2): usual work days per person (seven days, each ticked or not, with its work hours in minutes; start Monday to Friday, 8 hours); a share per person and project (0 to 100 %, start 100 %; a person with no row has 100 %); away times (a first and a last day, a half day at either end: *Afternoon only* for the first, *Morning only* for the last; a note only its person reads; who entered it). **No reason field**: the migration has none, and the review looks for one.
- **Public holidays** (amendment M8): computed in `bb-core`, pure, for *Germany* (the days every state has) and each of the 16 states by its ISO 3166-2 code (`de-by`, `de-be`, …); only holidays of a whole state. Tested against the official lists of 2026 to 2030, written into the test data from the published lists (data for the test, not a second source of the rules). No download and no outside service.
- **The setting *Holidays*** (`work-times.holidays`; *None*, *Germany*, or a state): at the level of the organization (start *Germany*) and of each person (start *As the organization*), through the settings registry (ADR-0018, ARCH-SETTINGS). Not part of *Region*.
- **Who may do what:** a person reads and enters their own work times. *Enter work times for others* (organization, *Manage*, *Administrator* at the start; no key and no agent holds it, ADR-0025 amendment M8) lets a person enter and change usual work days, work hours and away times of others, but never read or write the note of an away time. A share is set by a person with *Decide who has which role* in that project. Others read work times only as *Who sees people* allows for the kind *Work times* (start: *Names* for Team member and above, *Role only* for Contributor and Viewer, *Hidden* for Guest), and an away time only as *Away* with its days.
- **Shares over 100 %:** the module answers the sum of a person's shares for the person, and for a project lead the fact that it is over 100 % with only the projects the lead may see named.
- **Notifications** (ADR-0033, amendment M8): *Your work times were changed* when somebody else enters or changes an away time, usual work days or work hours; to the person only, never the note.
- **Deletion** (amendment M8, ADR-0017 amendment M8): `work-times.delete-old-away-times` daily, one year after the last day (`AWAY_TIME_KEPT_FOR`); `work-times.delete-left-shares` daily, for people with no role in the project any more; one function that deletes all of a person's work times when they leave, which *Remove a person* calls in M9, built and tested now.
- **The port for the plan:** `WorkTimesStore` in `crates/bb-app/src/ports.rs`, which WP-06 reads: for a set of people and a span of days, the minutes each person can work on each day in a project (usual work days × work hours × share, minus away times and holidays), and the away days and holidays by name, already narrowed to what the reader may see. Its shape is written first in this package, so WP-06 can test against a fake.
- **Personal data** as architecture §12, *Personal data in M8*: the marks on every new entity.

## Tasks

1. Write the acceptance tests below.
2. The holidays in `bb-core` with their vectors; the setting *Holidays*.
3. The records, migration 50 and the marks; the store on both databases.
4. The use cases and routes: read and change one's own work times, others' with the permission, shares on a project, the sum of shares; the permission and the kind *Work times* in the catalogues (the role page and *Who sees people* show them with their sentences).
5. The two notification kinds and their starting values.
6. The two daily jobs and the function for a person who leaves.
7. The port `WorkTimesStore` and its real implementation.

## Acceptance tests (write first)

- `plan_11_german_holidays_match_the_official_lists_2026_to_2030` (every state and *Germany*).
- `plan_11_others_see_only_away` (no note, no reason, the days only; both databases).
- `plan_11_work_times_follow_who_sees_people` (*Names*, *Role only*, *Hidden*).
- `plan_11_only_enter_work_times_for_others_changes_someone_elses_work_times`.
- `plan_11_nobody_but_its_person_reads_or_writes_the_note_of_an_away_time`.
- `plan_11_the_person_is_told_when_someone_else_enters_an_away_time`.
- `plan_11_an_away_time_is_deleted_one_year_after_its_last_day` (the clock of the app moved).
- `plan_11_a_person_who_leaves_loses_all_work_times`.
- `plan_15_a_share_is_set_only_with_decide_who_has_which_role_in_the_project`.
- `plan_15_a_share_goes_when_the_person_has_no_role_in_the_project_any_more`.
- `plan_15_a_project_lead_sees_over_100_percent_without_other_projects_names`.
- `plan_15_the_minutes_of_a_day_count_work_hours_share_away_times_and_holidays` (the port, as a table).
- SEC-MATRIX: rows for work times, away times, shares and *Enter work times for others*, for every starting role and each choice of *Work times*, both databases.

## Checks

DATA-MARKS, DATA-MIGRATIONS, DATA-LOGS, ARCH-PERMS, ARCH-SETTINGS, ARCH-PERSON-REF, ARCH-DB-PARITY, ARCH-EVENTS, ARCH-PURE, ARCH-AI-TOOLS (no tool reaches work times), SEC-MATRIX, PERF-N1, TXT-CATALOG, TXT-TERMS.

## Review

Code review with the SSOT/DRY/ODR gate: one module of work times that the plan reads through its port (no second sum of minutes in the plan), the holidays once in `bb-core`, the permission and the kind once in their catalogues, no reason anywhere. A **privacy review**: what others see of an away time, the note, the notifications, the deletion jobs, the marks and the legal basis against architecture §12; nothing of work times reaches a search, an export, a log or an AI tool.

## Depends on

WP-01 and WP-02 merged. Runs beside WP-03 and WP-05 (different crates and tables).
