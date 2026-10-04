# ADR-0006: Own fields stored in a typed value table

Status: **Accepted** · 2026-10-04

## Context
Organizations add their own fields to issues, time entries, work days, documents, releases and projects. They must be filterable, sortable, permissioned per field, and fast on both databases.

## Decision
Store values in one **`field_value`** table: `(entity_kind, entity_id, field_id, value_text, value_number, value_date, value_ref, position)`, with indexes per typed column and per `(field_id, value_*)`. Multi-value fields use several rows. Field definitions (type, rules, options, look) live in `field` and `field_option`.

## Consequences
- The search compiler joins `field_value` per referenced field; indexes keep it fast at 1 M issues.
- Built-in fields stay real columns; own fields and built-in fields look the same to search, permissions and the API.

## Alternatives considered
- **A JSON column per entity:** indexing differs between SQLite and PostgreSQL and is weaker for sorting. Rejected.
- **Adding real columns at runtime:** schema changes from the UI are risky and slow. Rejected.

## Enforced by
ARCH-DB-PARITY, PERF-BENCH; table shape: review (see `tools/checks/catalog.toml`).
