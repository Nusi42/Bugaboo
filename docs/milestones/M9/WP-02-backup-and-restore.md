# WP-02 — Backup and restore on the server and the command line

**Wave 1 · Folder / branch:** `bugaboo.win2` / `WIN2` · **Model:** Opus
**Features:** OPS-07 (server and command line; the screen is WP-11). [ADR-0046](../../adr/0046-backups-restore-and-upgrades.md) decisions 1 to 4 and 6; follow-up M2-F09.

## Goal

An administrator makes a backup while people work, on SQLite and PostgreSQL, from the command line, from a route and by a schedule, and restores it from the command line or from a route. A restore checks the backup first, keeps a copy of the present, runs what must run after it, and leaves a security log that verifies with its gap named.

## Owns

The backup format in `crates/bb-core/src/backup/` (the manifest, its parse and its checks, pure), the copy of the database in `crates/bb-store/src/backup/` (SQLite `VACUUM INTO`; PostgreSQL table by table in one `REPEATABLE READ` transaction in migration order; the restore into an empty database of the same kind), migration 54 (`crates/bb-store/src/migrations/m0054_backup_runs.rs`: one row per backup made, with its name, size, checksum, who or what made it), the use cases in `crates/bb-app/src/backups/`, the job kind `backups.daily` (rhythm `QuietHour`), the settings `backups.daily`, `backups.keep-daily`, `backups.keep-weekly`, `backups.folder` in `crates/bb-core/src/settings/backups.rs` (new) with their entries in `catalog.rs`, the routes in `crates/bb-api/src/api/backups.rs` (`GET` and `POST /api/v1/admin/backups`, `POST /api/v1/admin/backups/{backup}/restore`), the server's restore mode (`crates/bb-server/src/restore_mode.rs`: every other request answered with the page *Bugaboo is being restored* and `503`), the commands `bugaboo backup`, `bugaboo backup list`, `bugaboo restore <file>` and `bugaboo keys import <folder>` in `crates/bb-server/src/cli/backup.rs` and `cli/secrets.rs`, the new anchor after a restore in `crates/bb-store/src/anchor.rs`, user docs `userDocs/install/backups.md`.

## What is decided

- **The file** (ADR-0046, decision 1): a tar archive `bugaboo-<moment>-<version>.backup` with `manifest.json` (version, last migration, database kind, moment, head of the security log, SHA-256 of every member), the database copy and the files that the database points to. No keys. The tar crate (already in the lockfile for xtask) joins the shipped program; its register row says so. The member names are fixed and checked on reading: no path that leaves the restore's own folder is accepted (no `..`, no absolute path, no link).
- **While people work** (decision 2): the copy takes one consistent state; a write of a person waits at most one step of `VACUUM INTO` (SQLite) and never on PostgreSQL. The files are copied after the database, by the list the database copy names, so a file uploaded after the copy is not in the backup and nothing in the backup points to a missing file.
- **Who may:** making a backup and restoring need *Change organization settings*; no access key and no agent may call either route; the restore route also asks for the password and second step again in the request (the same check as the change of a password).
- **The schedule:** one backup a day at the quiet hour of the organization (`backups.daily`, on at the start); the newest 7 daily and 4 weekly ones are kept, the others deleted by the same job; the backup named `before-<version>` (WP-07) is never deleted by the rotation. The row *Backups* of the keep table (ADR-0045) reads these settings; whichever of WP-02 and WP-04 lands second adds it.
- **Restore** (decision 4): refuses a damaged backup or one of a newer version, changing nothing; makes a backup of the present first; puts back the database and the files; runs the migrations of an older backup; runs the removals of `privacy/removed.list` newer than the backup (the list is written by WP-06; until then the step reads an empty list, and its test uses a line written by the test); verifies the security log; writes the head it reached to the anchor file as a new start with the lost range, and a security-log entry that names the backup and the entries that are gone (M2-F09). On screen, the server enters restore mode, signs everybody out, restores in the running process (the pool is closed and opened again) and leaves restore mode; a failure leaves the copy of the present in place and says so.
- **Keys:** `bugaboo keys import <folder>` is the twin of `keys export` (same rules of rights, never replacing a key that opens the stored secrets); a restore checks that the master key opens the stored secrets of the backup before anything is replaced, and stops with the command to run when it does not.
- **Personal data:** the rows of migration 54 are marked (who made a backup is personal, basis legitimate interest); a backup holds personal data and is listed in *Personal data in M9* (WP-01).

## Tasks

1. Write the acceptance tests below.
2. The manifest in `bb-core` with its checks; the tar reading that refuses names that leave the folder.
3. The copy and the restore of the database on both databases; migration 54 and its marks.
4. The use cases, the job, the settings, the routes, the commands, restore mode.
5. The anchor after a restore and its security-log entry.
6. `keys import`; the user docs of backups and restore.

## Acceptance tests (write first)

- `ops_07_a_backup_made_while_people_save_restores_every_saved_change_up_to_its_moment` (both databases; writers run during the copy).
- `ops_07_a_backup_holds_no_keys`.
- `ops_07_a_damaged_or_newer_backup_is_refused_and_nothing_changes`.
- `ops_07_a_name_that_leaves_the_folder_is_refused` (`..`, absolute, link; a table of cases).
- `ops_07_a_restore_keeps_a_backup_of_the_present_first`.
- `ops_07_a_restore_of_an_older_backup_runs_its_migrations`.
- `ops_07_a_restore_runs_the_removals_made_after_the_backup`.
- `sec_15_after_a_restore_the_log_verifies_and_names_what_is_gone` (M2-F09; the status page is green after it).
- `ops_07_the_schedule_keeps_seven_daily_and_four_weekly_backups` (the clock of the app moved).
- `ops_07_a_restore_on_screen_signs_everybody_out_and_answers_others_with_the_restore_page`.
- `ops_07_a_wrong_master_key_stops_the_restore_before_anything_is_replaced`.
- SEC-MATRIX: rows for the backup and restore routes for every starting role and for an access key, both databases.

## Checks

ARCH-DB-PARITY, DATA-MARKS, DATA-MIGRATIONS, DATA-LOGS, ARCH-PERMS, ARCH-SETTINGS, ARCH-EVENTS, SEC-MATRIX, LINT-PATHS, DEP-REGISTER, PERF-N1, TXT-CATALOG.

## Review

Code review with the SSOT/DRY/ODR gate: the manifest is the one description of a backup, read by the command, the route and the restore; one copy routine per database; the anchor written by the one module that writes it today. **Security review:** the names inside an archive, the restore route's second check, restore mode, the rights of the backup files (owner only), no key in a backup. **Privacy review:** what a backup holds and how long (the keep table), the removal list after a restore.

## Depends on

WP-01 merged. Runs beside WP-03, WP-04 and WP-05 (its own crates' modules, its own table).
