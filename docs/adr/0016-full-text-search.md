# ADR-0016: Full-text search inside the database

Status: **Accepted** · 2026-10-04

## Context

Text search must respect permissions and work on both databases at the scale target.

## Decision

Use **SQLite FTS5** and **PostgreSQL text search** (tsvector with GIN index) behind one port. The search compiler adds the text condition to the same SQL query as the permission predicates. Index updates come from the outbox.

## Consequences

- No second index to secure, back up or rebuild separately.
- Ranking quality is good, not perfect; acceptable for 1.0.

## Alternatives considered

- **tantivy or an external search server:** better ranking, but permission filtering outside SQL and another store to protect. Rejected for 1.0.

## Enforced by

SEC-MATRIX (search entry point), ARCH-DB-PARITY (see `tools/checks/catalog.toml`).
