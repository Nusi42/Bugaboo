# WP-03 — Database tooling: local PostgreSQL and a test harness for both databases

**Wave 1 · Folder / branch:** `bugaboo.win3` / `WIN3`
**Features:** groundwork for OPS-02; ADR-0003, ADR-0023, ADR-0024.

## Goal

Every developer machine can run the repository tests against SQLite **and** PostgreSQL with one command, without installing anything machine-wide.

## Owns

`tools/dev/tools.toml` (new entries), `xtask/src/db.rs`, a test-support crate or module for database tests (e.g. `crates/bb-store/tests/support/` or a `bb-testing` dev crate — declare it in the module map first), checks `arch_db_parity.rs`, `data_migrations.rs`.

## Tasks

1. **PostgreSQL 18.6** (latest supported, 2026-08-13) as a pinned download for Windows x64 (`postgresql-18.6-1-windows-x64-binaries.zip` from get.enterprisedb.com). EDB publishes no checksums: download once, compute the sha256, pin it in `tools.toml` with a comment saying how it was obtained. Linux: `kind = "system"` (must be on PATH, as on GitHub's Ubuntu runners); `setup` explains how to install it if missing.
2. **`cargo xtask db start | stop | reset | url`:** a throwaway cluster in `.tools/pg-data/` (initdb with a random password written to an ignored file, trust only on 127.0.0.1), port `postgres` from `tools/dev/ports.json` (47240), strict: fails clearly if the port is busy.
3. **Test harness:** a helper that runs the same test body against SQLite (temporary file) and PostgreSQL (fresh database per test, dropped afterwards), selected by `BB_TEST_DATABASES=sqlite,postgres`; `cargo xtask ci` starts the database, runs both, stops it. Records which databases ran to `target/db-parity.txt`.
4. **Checks** (active, with bad/good examples): ARCH-DB-PARITY (CI ran the repository tests on both databases — reads the record), DATA-MIGRATIONS (no `.sql` migration files; once WP-05 exists, the migration test passes on both).
5. **Register** sqlx (0.9.0, sqlite + postgres + runtime-tokio + tls-rustls features) if this package adds it, and every other new crate.

## Acceptance

- Fresh folder: `cargo xtask setup` installs PostgreSQL; `cargo xtask db start` → `psql`-free smoke test connects; `db stop` cleans up; second `setup` is a no-op.
- `cargo xtask ci` runs an example repository test on both databases.
- All checks green.
