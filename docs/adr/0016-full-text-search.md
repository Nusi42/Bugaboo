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

## Amendment M5: words, what is indexed, the order of results

The M5 proposals (M5-Q5, Q6) were accepted. Decided:

1. **No stemming, the same on both databases.** Texts are made comparable once, in Rust: lower case, accents removed, split into words by one rule (`bb-core`). The index and the words of a search go through the same function; SQLite FTS5 and PostgreSQL (`simple` configuration) only store and match the prepared words. German and English text behave alike, and both databases find the same issues. A word ending in `*` matches every word that starts with it; at least two letters stand before the `*` (a search limit, architecture §7).
2. **What is indexed:** the title, the description, each comment, the names of attached files, and the own long-text fields. Each text is a row of its own with its source (title, description, a comment, a file, a field), so the search can leave out every source the reader may not see: a field they may not read, a comment or a file on an issue they do not see. A hit never comes from a hidden text.
3. **`contains` on its own** searches the title, the description, the comments and the names of attached files (M5-Q6); `<field> contains "…"` searches one long-text field, only if the reader may read it. What is inside a file is not searched.
4. **All the words of a `contains`** must be found in the issue, each in any of the texts searched (the lead, from the review of M5 WP-05): `contains "login error"` finds an issue with *login* in its title and *error* in a comment. One `contains` holds a limited number of words (a search limit, architecture §7), so the statement stays bounded.
5. **The order of results** when a search has text and no `sort by`: issues whose title has the words first, then the newest.
6. **The index is a reader of the outbox** (ADR-0007): it keeps its place, catches up after a restart, and `cargo xtask` can rebuild it from the stored texts.

Enforced by: ARCH-DB-PARITY (the same issues in the same order on both databases), SEC-MATRIX (the `search` entry point with a hidden field, comment and file), and the tests of M5 WP-05.

## Enforced by

SEC-MATRIX (search entry point), ARCH-DB-PARITY (see `tools/checks/catalog.toml`).
