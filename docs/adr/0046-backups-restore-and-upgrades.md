# ADR-0046: Backups, restore and upgrades

Status: **Accepted** · 2026-11-18 · refines OPS-06, OPS-07, [ADR-0003](0003-databases.md) and the upgrade line of [architecture §6](../architecture/06-deployment.md); the answers to M9-Q9 to M9-Q11 of [the M9 plan](../milestones/M9/README.md)

## Context

OPS-07 asks for online backup and restore, from the command line and from the screens, with scheduled backups and retention. OPS-06 asks that an upgrade migrates the database by itself, makes a backup first, and has a clear way back. Architecture §6 already promises it: "on start, Bugaboo makes a backup, then runs migrations; if a migration fails, it restores the backup and stops with a plain explanation". None of it is built: the data folder has an empty `backups/`, and migrations run at start with no copy made.

A backup of Bugaboo is more than the database: the files of attachments live in `files/`, the master key and the log key in `keys/`, and the security log is anchored in `logs/security-head.log`, outside the database. A restore of an older database leaves the anchor ahead of it, and the status page then stays at "problem" with no way out (M2-F09). A restore of a backup older than a *Remove a person*, a switch-off or a key taken back would bring them back (ADR-0044).

Bugaboo runs on SQLite (the start) and PostgreSQL. Both have to be backed up while people work.

## Decision

1. **One backup is one file** (M9-Q9): `bugaboo-<moment>-<version>.backup` in `backups/` (or a folder the administrator names), a tar archive with a manifest (`manifest.json`: the version of Bugaboo, the set of applied migrations with their checksums, the database kind, the moment, the head of the security log, and the SHA-256 of every member), the database, the files of attachments that the database points to, and a copy of the replay list (ADR-0044, decision 8). A backup holds **no keys**: whoever restores it on another computer brings the keys with `bugaboo keys export` and `keys import`, and a restore checks that the master key opens the stored secrets before it goes on. Enforced by: review (the manifest is the one description of a backup), DATA-LOGS.
2. **The database is copied while people work.** SQLite: `VACUUM INTO` a file of the backup's temporary folder, which reads one consistent state; writers do not wait for it, but the write-ahead log grows while the copy runs and is checkpointed after it. PostgreSQL: Bugaboo's own copy, table by table in one transaction at `REPEATABLE READ`, written in the order of the migrations, so no outside program (`pg_dump`) and no fixed path is needed (coding-standards §2). A restore goes into the same kind of database. Enforced by: ARCH-DB-PARITY (the backup and restore tests on both databases), LINT-PATHS.
3. **Making a backup.** `bugaboo backup` (to the folder of the settings, or `--to <folder>`), the button *Make a backup now* in *Organization settings › System › Backups* (with *Change organization settings*), and the schedule: daily at the quiet hour (`backups.daily`, on at the start), keeping the newest 7 daily and 4 weekly backups (`backups.keep-daily`, `backups.keep-weekly`). Every backup writes one line to the security log with its name and checksum. A backup that fails is an alert on the status page and a notification to administrators. Enforced by: ARCH-PERMS, ARCH-SETTINGS, SEC-MATRIX, ARCH-EVENTS.
4. **Restoring from the command line** (M9-Q10 and M9-Q20). `bugaboo restore <file>` runs with the server stopped, and the page *Backups* shows that command; a restore from the screen is not part of the MVP (OPS-07 as reworded). A restore:
   - checks every checksum of the manifest and refuses a backup of a newer version or a damaged one, changing nothing;
   - makes a backup of what is there now first, so a restore can itself be undone; that backup is kept out of the rotation for the keep period of the security log, because it is the evidence of what the restore replaced;
   - signs everybody out, the command line's restore included (every session of the restored database ends);
   - puts the database and the files back, runs the migrations of a backup from an older version (decision 5), merges the copies of the replay list (the data folder's, the backup's and one given with `--replay-list`) and runs every line newer than the backup (ADR-0044, decision 8: removals, switch-offs, keys, grants, passkeys and support accesses taken back);
   - verifies the restored security log, writes the head it reached to the anchor file as a new start with the lost range named, and writes a security-log entry that says what was restored and which entries after the backup's head are gone (M2-F09). The restore's report lists what the replay did again, and says that every other change after the backup's moment is gone, password and role changes included, so the administrator can tell the people concerned.

   Enforced by: ARCH-DB-PARITY (the restore tests on both databases), ARCH-EVENTS.
5. **Upgrades back up first.** When the server starts and the set of migrations applied to the database lacks some of the program's, the server makes a backup (decision 1) before the first migration, named `before-<version>`, and keeps it outside the rotation of decision 3; the newest three such backups are kept (`backups.keep-before-upgrade`), older ones go with the rotation. If a migration fails, the server restores that backup, stops, and says in plain words what failed, where the backup is, and that the previous program works with it. A database with a migration the program does not know is refused, as today. The migrations of a release can only be checked against a real older database, so every release is tested by upgrading a database made by the previous milestone's program (the upgrade test, WP-07). Enforced by: DATA-MIGRATIONS, ARCH-DB-PARITY, review (the upgrade test runs before a release).
6. **Where the files go.** Backups are as private as the data folder (owner only, at every start, M1-F65). Backups are not encrypted in the MVP; encryption with the master key is SEC-20 (V1). The page *Backups* says where they are and that they hold all the organization's data. Enforced by: review (the data-folder tests of `sec_16_…` cover `backups/`).

## Consequences

- Keys are backed up apart from data, on purpose: a stolen backup does not carry the key that opens its secrets.
- A restore is an event in the security log, and the chain explains the gap instead of breaking.
- Every release costs one more test: a database made by the program of the previous milestone, upgraded.

## Alternatives considered

- **`pg_dump` and `sqlite3` as outside programs:** extra installs, versions that must match the server, and a path to find. Rejected.
- **The keys inside the backup:** one file restores everything, but whoever holds the file holds every secret. Rejected until backups are encrypted (SEC-20, V1).
- **Restore from the screen in M9, as OPS-07 first said:** many administrators never open a console on the server, but the screen needs a restore mode, a second check and its own concurrency tests for an action that is rare and done with the service stopped. Not in the MVP (M9-Q20); a later milestone may add it on top of this record.
- **A folder per backup instead of one file:** easier to read, harder to move and to check whole. Rejected.

## Enforced by

ARCH-DB-PARITY, DATA-MIGRATIONS, ARCH-PERMS, ARCH-SETTINGS, ARCH-EVENTS, SEC-MATRIX, LINT-PATHS, DATA-LOGS; review (the manifest, and the upgrade test before each release).

## Amendment: a backup is sealed with the keys of its installation

The security review of the first build found that a restore took any backup file whose checksums matched its manifest, so a file that somebody else wrote, for example one whose database plants an administrator, would be restored, and that a master key of another installation passed when the backup held no stored secret. Decision 1 is refined:

- **The seal.** Every backup carries `manifest.seal` right after `manifest.json`: an HMAC-SHA256 of the exact bytes of the manifest under the log key of the installation, which never changes. The manifest names the SHA-256 of every other member, so the seal covers the whole backup. A restore checks it before it writes any member and refuses a backup without it, or with one its keys did not make, changing nothing. A backup of another installation is restored after its key files are imported (`bugaboo keys import`).
- **The check values of the master keys.** The manifest lists, for each master key of the installation, its ID and an HMAC of a fixed text under it. A restore refuses a master key of the same ID with another check value, and keys that share no ID with the backup, before anything is replaced, whether or not the backup holds stored secrets.
- Each HMAC starts with a text of its own, so neither can stand for the other or for a signature of the security log under the same key. The backup still holds no key (decision 1): an HMAC shows whether a key is the same key, not the key.

Enforced by: the restore tests `ops_07_a_backup_another_installation_sealed_is_refused_before_anything_is_replaced` and `ops_07_a_wrong_master_key_stops_the_restore_before_anything_is_replaced` (both databases), and the archive test `ops_07_a_backup_this_installation_did_not_seal_is_refused`.

## Amendment: the backup before an upgrade, as built

Building the upgrade showed where decision 5 had to be exact. Decision 5 is refined:

- **The name.** The backup is `bugaboo-before-upgrade-<moment>-<version>.backup`, with the version of the program that makes it, in the format of decision 1.
- **The folder.** It always goes into `backups/` of the data folder, never into a folder the administrator named for the other backups: that folder is a setting in the database, and the settings may not be readable before the migrations have run.
- **How many are kept.** The newest N are kept, N being `backups.keep-before-upgrade` (3 at the start); the rotation of decision 3 deletes older ones, in `backups/` of the data folder whatever folder the other backups go into.
- **`version.txt`** in the data folder exists only for the words of a stop: it holds the version of the program that last started on the data, so the words can name the previous version, and, from the backup to the end of the migrations, the name of the backup made before them. A start after one that was cut off part way then says that the database may be half changed, tells the administrator not to start Bugaboo on it, names that backup (while it is there) to be kept, and does not say that the previous program works with the database. Going back from it needs a restore that does not migrate (M9-F62).
- **The stop.** Every start that stops at an upgrade (a newer database, a migration this program does not have, or has in another form, a backup that failed, a migration that failed) exits with `UPGRADE_STOPPED` = 65, defined once in `crates/bb-server/src/upgrade.rs`; a Windows service reports it as its own service-specific code, so a service manager can tell that starting again does not help.

Enforced by: TEST-UPGRADE, DATA-MIGRATIONS, ARCH-DB-PARITY.

## Amendment (2026-11-22, a restore and other servers, the folder of backups)

- **A restore sees every server on its database.** The lock of the data folder (`server.lock`) only tells a restore about a server of the same computer. A PostgreSQL database can be used by servers on other computers too, so every server holds the advisory lock `ServerRunning` of the database shared, on a connection of its own, for as long as it runs, and `bugaboo restore` takes it alone first and refuses with `restore.database-in-use` while any server holds it; a server that starts during a restore waits for it to end. The connection of the hold is asked every 30 seconds whether it still answers (`CHECK_EVERY`, `db/server_hold.rs`), and the database server sends keep-alive packets on it, so a link that broke without a word is noticed on both ends; a broken one is replaced and the lock taken again, and when a restore holds it by then, the server stops with `server.database-hold-lost` and fails, so a service manager starts it again and the new start waits. A server that waits for a restore says so in the log. The database address must not lead through a pooler in transaction mode, which would hand the session of the lock to other clients (the settings page of the install guide says so) The lock lives with its connection, not with a transaction, so it stays outside the order of the locks of transactions (ARCH-LOCK-ORDER). On SQLite the lock of the data folder does the same, and nothing more is held (`ops_07_a_restore_finds_the_database_held_while_a_server_runs_on_it`, both databases; `ops_07_a_server_whose_hold_broke_takes_it_again_or_is_told_to_stop`, PostgreSQL).
