# ADR-0003: Databases: SQLite by default, PostgreSQL optional

Status: **Accepted** · 2026-10-04

## Context
Installation must work with zero setup, but larger installations and the served model need a server database.

## Decision
Support **SQLite** (bundled, WAL mode) and **PostgreSQL 16+** through **sqlx**. Write queries with **sea-query** (bound through `sea-query-sqlx`) so one query definition produces both dialects. Define the **schema and migrations once in Rust** with the same query builder; both databases run the same migration list. Dialect-specific code is allowed only behind the `bb-store` port implementations and needs a comment explaining why.

## Consequences
- Every repository test runs against both databases in CI.
- Some PostgreSQL features (for example row-level security) are not used in core paths; authorization stays in `bb-policy` anyway.
- The served model uses PostgreSQL; self-hosted installations start with SQLite and can migrate with `bugaboo migrate-to-postgres`.

## Alternatives considered
- **PostgreSQL only:** simpler code, harder installation. Rejected for the default.
- **SeaORM:** less control over generated SQL for permission predicates. Rejected.
- **Two hand-written migration folders:** violates SSOT. Rejected.

## Enforced by
ARCH-SQL-EDGE, DATA-MIGRATIONS, ARCH-DB-PARITY (see `tools/checks/catalog.toml`).
