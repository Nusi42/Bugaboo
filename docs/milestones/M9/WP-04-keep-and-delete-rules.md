# WP-04 — Keep and delete rules on the server

**Wave 1 · Folder / branch:** `bugaboo.win4` / `WIN4` · **Model:** Opus
**Features:** PRIV-05 (server; the screen is WP-10). [ADR-0045](../../adr/0045-keep-and-delete-rules.md); ADR-0017, ADR-0035 decision 7; follow-ups M1-F21, M1-F26, M3-F25, M4-F63 (the server part, unless M9-Q20 cuts it), M4-F93, M4-F104, M4-F131, M6-F45.

## Goal

One list says, for every kind of data, how long it is kept and whether Bugaboo deletes it by itself or an administrator does. Every deletion by itself runs through one job shape that deletes in short steps and takes the record's events along. Business records past their period are counted and deleted by an administrator through one use case.

## Owns

`crates/bb-core/src/keeping/` (new: `KEEP_KINDS`, each row with its kind, its setting, its floor and its group; pure), the settings of the periods in `crates/bb-core/src/settings/keeping.rs` (new; the existing `security.log-keep-days` and the period of work days stay where they are and are named by their rows, not copied) with their entries in `catalog.rs`; `crates/bb-app/src/keeping.rs` and `notifications/keep.rs` (the one shape of deleting in steps, `KEEP_STEP_ROWS`; every existing purge moved under it); the general use case of deleting old records in `crates/bb-app/src/keeping/old_records.rs` (today's deletion of old work days in `bb-app::working_hours` becomes its first user, time entries the second); migration 56 (`m0056_keeping.rs`: the index each purge needs to find old rows by moment, and the trash's deletion moment where a table lacks one); the routes in `crates/bb-api/src/api/keep_rules.rs` (`GET /api/v1/admin/keep-rules`, `GET /api/v1/admin/keep-rules/{kind}/past`, `POST /api/v1/admin/keep-rules/{kind}/delete-past`; the periods change through the settings routes that exist); the trimming of the anchor file in `crates/bb-store/src/anchor.rs` (shared with WP-02: WP-02 writes a new start after a restore, WP-04 trims old lines; one module, two functions); the generated table of `userDocs/privacy/keep-and-delete.md`; the route and use case of failed jobs (`GET /api/v1/admin/failed-jobs`, `POST /api/v1/admin/failed-jobs/{job}/try-again`, calling the store's `requeue_dead_job`, M3-F150, M4-F63) in `crates/bb-api/src/api/admin.rs` and `crates/bb-app/src/jobs.rs`, whose screen is WP-11's; the links of its new columns, after WP-03's first landing.

## What is decided

- **The two groups** (ADR-0045, decision 1, M9-Q1 and M9-Q2 as answered). By itself: IP addresses in logs, sessions and sign-in history, sign-up links, drafts, notifications, the trash, data exports, payloads of dead jobs, away times, the security log, backups. By an administrator: work days (with their breaks and reasons) and time entries. The starting periods and floors are the ones of the README's M9-Q1, each a setting of the registry with its floor (the trash at least 1 day; work days at least 2 years with *Germany*, as today).
- **One shape** (decision 5): a daily kind of `JobKind` per row of the first group, rhythm `FromTheStart`, or `QuietHour` for the security log, the trash and dead jobs; each deletes at most `KEEP_STEP_ROWS` rows in one short transaction and queues its next step while more is left (M4-F104). The existing kinds (`notifications.purge`, `security-log.prune`, `sessions.purge`, `editing.purge-drafts`, `files.clean-up`, `work-times.delete-old-away-times`, `oauth.clean-old-grants`) keep their names and move under the shape; no second loop is written.
- **The trash** (M3-F25, M4-F131, M9-Q1 as answered): issues deleted into the trash longer ago than the period are deleted for good with their comments, fields, relations, attachments (the files go when nothing else points to them) and events, **except** an issue with a time entry still inside its keep period: it stays in the trash, hidden, until its last time entry leaves the period, because time entries are business records (§147 AO, §257 HGB) that the purge must never take along. The trash says so on such an issue ("kept for its time entries until …").
- **Events go with their record** (decision 4): the deletion of a record deletes the events whose payload carries its values: the upload's file name (M4-F93), a time entry's note and fields (M6-F45), a work day's reasons (as today). The history of records that still exist is not touched.
- **Dead jobs** (M1-F21): a job on the dead-letter list longer than its period loses its payload, then its row.
- **The anchor file** (M1-F26): trimmed by `security-log.prune` after it deleted blocks; the newest line is kept; the trimmed file is written next to the old one and renamed into place; a crash between leaves the old file whole.
- **By an administrator** (decision 6): one use case counts what of a kind is past its period (per person and in total, counted when asked, no job counts) and deletes it on *Delete them*, for all or for one person, with *Change organization settings*; no key or agent; a security-log entry with how many of what and whose. Time entries past 10 years (or the organization's period) join work days there. A run that deletes the last kept work days of a removed person clears that person's legal-hold name in the same change (ADR-0044, decision 6).
- **Support access** (if it stays in M9): the record of an ended access is a row of the first group, kept for the security log's period (ADR-0048).
- **Failed jobs** (M4-F63, unless cut): a list for holders of *See how the server is doing* with each job's kind in plain words and its last failure's reason, never its payload; *Try again* with *Change organization settings*, no key or agent, a security-log entry.
- **Away times** stay as M8 decided (1 year, by itself; ADR-0030 amendment M8); the row only shows them.
- **A port for others:** `KeepPeriods` in `crates/bb-app/src/ports.rs` answers "kept until" for a kind and a record date. WP-06 reads it for the removal window, so its shape is written first in this package.

## Tasks

1. Write the acceptance tests below.
2. `KEEP_KINDS` and the settings with their floors.
3. The one shape of deleting in steps; every existing purge moved under it, each keeping its tests.
4. The new purges: trash, exports (the file goes with its row), dead jobs' payloads; events with their records; the anchor trim.
5. Migration 56; the general use case of old records and its routes; time entries as its second user.
6. The port `KeepPeriods`; the generated table of the user docs.

## Acceptance tests (write first)

- `priv_05_every_kind_of_data_has_one_row_and_one_group` (the list against the deleting jobs: a job without a row, or a business record with a job, fails).
- `priv_05_a_period_below_its_floor_is_refused`.
- `priv_05_each_purge_deletes_in_steps_and_a_write_does_not_wait_behind_it` (a step's transaction stays short with 100,000 old notifications; both databases).
- `priv_05_the_trash_is_deleted_after_its_period_with_its_files_and_events`.
- `priv_05_an_issue_in_the_trash_stays_while_its_time_entries_are_kept`.
- `priv_04_deleting_the_last_kept_work_days_clears_the_legal_hold_name` (with WP-06; until it lands, a test of the store).
- `ops_08_a_failed_job_is_tried_again_by_an_administrator` (both databases; M4-F63; unless cut).
- `ops_08_failed_jobs_show_the_reason_and_never_the_payload`.
- `priv_05_a_deleted_record_takes_the_events_that_carry_its_values` (an upload's file name, a time entry's note).
- `priv_05_a_dead_job_loses_its_payload_after_its_period`.
- `sec_15_the_anchor_file_is_trimmed_and_keeps_its_newest_line` (a crash between the write and the rename leaves the old file whole).
- `priv_05_time_entries_are_deleted_only_when_an_administrator_says_so` (and `priv_09_old_working_hours_are_deleted_only_when_an_administrator_says_so` still passes through the general use case).
- `priv_05_kept_until_follows_the_period_of_the_kind` (the port, as a table).
- `priv_05_every_purge_is_safe_to_run_again`.
- SEC-MATRIX: rows for the keep-rule routes and *Delete them* for every starting role and an access key, both databases.

## Checks

ARCH-SETTINGS, ARCH-EVENTS, ARCH-PERMS, ARCH-DB-PARITY, DATA-MIGRATIONS, DATA-MARKS, SEC-MATRIX, PERF-N1, DOC-GENERATED, TXT-CATALOG.

## Review

Code review with the SSOT/DRY/ODR gate: one list of kinds, one shape of deleting in steps, one use case of deleting old records; the work days' deletion is reused, not copied; no period written twice. **Privacy review:** each row's group and period against ADR-0045 and the maintainer's rule on explicit deletion; what goes with a deleted record; the security-log entries of *Delete them*.

## Depends on

WP-01 merged. Runs beside WP-02, WP-03 and WP-05. Touches the jobs of many modules, each only to move its purge under the shape; a conflict with another package of wave 1 is resolved on rebase, keeping both.
