# WP-05 — Store: schema once for both databases, event log, jobs, security log, secrets

**Wave 2 · Folder / branch:** `bugaboo.win2` / `WIN2`
**Features:** OPS-02, SEC-15, SEC-16, PRIV-02 (storage side); ADR-0003, ADR-0007, ADR-0017, ADR-0018.

## Goal

`bb-store` is the only crate that talks to a database. Schema and queries are written once (sea-query) and run on SQLite and PostgreSQL.

## Owns

`crates/bb-store/**`, its use of the WP-03 test harness, check ARCH-SQL-EDGE (only `bb-store` touches the database; no SQL built with `format!`), the storage part of ARCH-SETTINGS, DATA-MIGRATIONS (migration test part).

## Tasks

1. **Pools and migrations:** sqlx pools for both databases (SQLite in WAL mode, foreign keys on, busy timeout); migrations as Rust code with sea-query, numbered, forward-only, recorded in a migrations table; refuse to start on a newer schema than the binary knows.
2. **Event log and outbox (ADR-0007):** append-only `events` table storing the WP-02 envelope; the outbox is written in the same transaction as the change. One function for "change + event + outbox" — no other way to write domain data.
3. **Job queue (ADR-0017):** jobs table with lease, retries with back-off, dead-letter; workers poll; `SKIP LOCKED` on PostgreSQL, single-writer on SQLite.
4. **Security log (SEC-15):** append-only, hash-chained (each row stores the hash of the previous one), verify function and `bugaboo security-log verify` hook for WP-08; IP addresses truncated as in WP-04. Old entries are deleted after the keep period in whole blocks, leaving a signed checkpoint (hash of the last deleted entry) so the rest still verifies.
5. **Secrets (SEC-16):** master key in `keys/` (created on first start, owner-only), envelope encryption with XChaCha20-Poly1305 (architecture §7, Secrets and keys), key ID stored with each value, `zeroize` for keys in memory; rotation function.
6. **Settings storage:** values per level from the WP-02 registry; unknown keys rejected.
7. **Tests:** every repository test runs on both databases through the WP-03 harness; property test for the hash chain (any changed row breaks verification).

## Notes from Wave 1 (read before starting)

- **Test harness (WP-03):** `bb-testing::each_database` hands out typed pools (`Pool::Sqlite` / `Pool::Postgres`); use them with sea-query-sqlx, not an Any driver. `bb-testing` may only be a dev-dependency (ARCH-DEPS). Run tests through `cargo xtask db run -- cargo test …` or `cargo xtask ci`.
- **Migration test name:** DATA-MIGRATIONS requires a test named `migrations` that runs on both databases once `crates/bb-store/src/migrations*` exists.
- **IDs and time (WP-02):** `bb-core` is pure: IDs are built from a timestamp and random bytes passed in. Put the real clock and random source into the use-case context here (or in `bb-app`, per the module map), never into `bb-core`. People's IDs are fully random (no time in them).
- **Personal-data marks:** every stored struct derives `Entity` and is listed in `entities!`; a stored struct without marks is a finding (extend DATA-MARKS so a table definition without an `Entity` fails).
- **Settings:** `Setting::resolve` reports a wrongly typed stored value as an error; log it (ID only) and use the default only after that.

## Acceptance

- `cargo xtask ci` green with both databases recorded; ARCH-DB-PARITY and DATA-MIGRATIONS green.
- ARCH-SQL-EDGE and ARCH-EVENTS active and green.
