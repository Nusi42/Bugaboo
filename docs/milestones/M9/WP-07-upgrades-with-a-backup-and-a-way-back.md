# WP-07 — Upgrades: a backup before migrating, the way back, the upgrade test

**Wave 2 · Folder / branch:** `bugaboo.win2` / `WIN2` · **Model:** Opus
**Features:** OPS-06. [ADR-0046](../../adr/0046-backups-restore-and-upgrades.md) decision 5; DATA-MIGRATIONS; architecture §6, *Upgrades*.

## Goal

A new program started on an older database backs it up first, migrates it, and opens. When a migration fails, it restores that backup, stops, and says in plain words what happened and how to go back. Every release proves it by upgrading a database made by the program of the previous milestone, on both databases.

## Owns

The start-up order in `crates/bb-server/src/wiring.rs` and `crates/bb-store/src/migrations/mod.rs` (ask whether migrations are due, by comparing the set of migrations applied with the program's, never only the last one; back up through WP-02's code; run; on failure restore and stop); the messages of the stop (`server.upgrade-*` in `messages/en.json`, plain sentences with the backup's path and the previous version); the test migration that fails, only in tests (`crates/bb-store/src/migrations/tests.rs`); the upgrade test in `xtask/src/upgrade.rs` (new) and its step in `xtask/src/ci.rs`; the check TEST-UPGRADE in `xtask/src/checks/test_upgrade.rs` (new) with its catalogue entry and `tools/checks/fixtures/TEST-UPGRADE/`; architecture §11, *The upgrade test* (the rule, as "Adding a rule" asks); `userDocs/install/upgrade.md`.

## What is decided

- **Before the first migration** (ADR-0046, decision 5): the server compares the set of migrations applied to the database (names and checksums) with its own list; when migrations are due it makes a backup named `before-<version>` in `backups/` (WP-02's format; the newest three such backups are kept, `backups.keep-before-upgrade`, older ones go with the rotation) and logs where it is. A backup that fails stops the start before any migration, with the reason.
- **On failure:** the migration's transaction is rolled back where the database allows; the server restores `before-<version>` in every case (a migration of several steps may have committed some), checks that the restored database's set of migrations is the one it had before, writes the stop message (what failed, the backup's path, "the previous program, version X, works with this database"), and exits with a code of its own. The security log is restored with the database and needs no new anchor (nothing was written in between); the test proves it.
- **A database with a migration the program does not know** is refused, as today, with the version that made it.
- **Under a service:** this package owns only the exit code of an upgrade stop, one named constant in `bb-server`; WP-13 owns the service settings that read it so the service does not restart in a loop.
- **The upgrade test** (the check TEST-UPGRADE, "Adding a rule"): `cargo xtask upgrade-test` finds the newest `mN-done` tag before `HEAD` from Git (never written by hand), builds one release program of that tag (a `git worktree` in `.work/` with a `CARGO_TARGET_DIR` of its own; after the build only the program is kept, in `target/upgrade/<tag>/`, and the worktree and its build folder are deleted, so the disk keeps one file), makes the sample organization with it on SQLite and PostgreSQL, starts the program of `HEAD` on that data, and checks: the `before-…` backup exists, every migration ran, the counts of the sample's records and a list of known records (issues, comments, time entries, work days, work times, own themes, agents) read the same through the API, and the security log verifies. It runs in `cargo xtask ci` (the old build is kept between runs) and before each release.

## Tasks

1. Write the acceptance tests below.
2. The start-up order with the backup and the restore on failure.
3. The stop messages and the exit code.
4. The upgrade test in xtask and its ci step.
5. TEST-UPGRADE: catalogue entry, code, bad and good fixtures, the rule in architecture §11.
6. The user docs of upgrades.

## Acceptance tests (write first)

- `ops_06_an_upgrade_makes_a_backup_before_the_first_migration` (both databases).
- `ops_06_no_backup_is_made_when_no_migration_is_due`.
- `ops_06_a_failed_migration_restores_the_backup_and_stops_in_plain_words` (the failing test migration; both databases; the database reads as before).
- `ops_06_a_backup_that_fails_stops_the_start_before_any_migration`.
- `ops_06_a_newer_database_is_refused_with_its_version`.
- `ops_06_the_newest_three_backups_before_an_upgrade_are_kept`.
- `ops_06_a_database_with_a_gap_in_its_migrations_is_migrated_and_one_with_an_unknown_migration_refused`.
- `ops_06_the_upgrade_test_keeps_only_the_old_program_on_disk`.
- `ops_06_the_sample_of_the_previous_milestone_reads_the_same_after_the_upgrade` (the upgrade test, both databases).
- TEST-UPGRADE's fixtures: a ci without the step (bad); the wired step (good).

## Checks

TEST-UPGRADE (new), DATA-MIGRATIONS, ARCH-DB-PARITY, TXT-CATALOG, LINT-PATHS, META-CATALOG, META-FIXTURES, META-WIRING.

## Review

Code review with the SSOT/DRY/ODR gate: one backup routine (WP-02's), the tag found from Git, the list of migrations read from its one home. **Security review:** the rights of the `before-…` backup, the stop message names no secret.

## Depends on

WP-01 and WP-02 merged. Runs beside WP-06, WP-08 and WP-09 (start-up order, migrations module and xtask; the migrations module only in its runner, not in the list of migrations, which the other packages add to).
