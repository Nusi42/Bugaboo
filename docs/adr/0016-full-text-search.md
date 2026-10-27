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

1. **No stemming, the same on both databases.** Texts are made comparable once, in Rust: lower case, Unicode NFC, split into words by one rule (`bb-core`). No letter is changed into another (requirement: the text handling is UTF-8 clean): an accent stays and `ß` stays `ß`, so *müller* finds *Müller*, written as one character or as two, and *Muller* and *Mueller* do not. The databases only store and match the words Rust prepared, and both find the same issues: the FTS5 table has the tokenizer `unicode61 remove_diacritics 0 categories 'L* N* Co M*'`, which cuts a word only at what is no letter, number or mark, and the `tsvector` of PostgreSQL is made from the stored words split at spaces and matched with quoted lexemes, so no parser of the locale decides what a word is. German and English text behave alike. A word ending in `*` matches every word that starts with it; at least two letters stand before the `*` (a search limit, architecture §7).

   **A word has at most 255 bytes** (`MAX_WORD_BYTES` in `bb-core`). A real word is never that long, and a longer run (a line of Chinese text without a stop, a hash) is not what anybody searches for; PostgreSQL refuses a word of 2047 bytes or more, and one such word in a comment would stop the whole text from being stored. So a longer run is left out when a text is stored (the other words of the text stay, and the text is stored), and a longer word typed in a search is refused by the checker with a message (the same checker in the browser and on the server; it counts the lower-cased word as typed, in bytes). One edge: NFC can make a word that passed the checker longer (a letter that takes apart into more bytes; the most is a few times as long, far below the limit of PostgreSQL, a test proves it). Such a word is looked up as it is, and because the index never keeps a word that long it is found nowhere; an issue must have every word of a search, so that search finds nothing. The port does the same for a caller that did not run the checker, and no word of that length reaches a database.

   Two steps, each with one rule. The checker of the search language decides on the text as typed (lower case, split, no NFC): how many words a `contains` has and how many letters stand before a `*`, so the browser and the server answer alike. The words that are stored and looked up come from the whole text put into NFC first and then split. The NFC step is behind the feature `canonical-text` of `bb-core`, which the store switches on, because its tables are too large for the search module of the browser, which never prepares a word for the index. A start that NFC shortens below two letters (a syllable written as two jamo and typed with `*`) passes the checker and is looked up as the whole word.
2. **What is indexed:** the title, the description, each comment, the names of attached files, and the own long-text fields. Each text is a row of its own with its source (title, description, a comment, a file, a field), so the search can leave out every source the reader may not see: a field they may not read, a comment or a file on an issue they do not see. A hit never comes from a hidden text.
3. **`contains` on its own** searches the title, the description, the comments and the names of attached files (M5-Q6); `<field> contains "…"` searches one long-text field, only if the reader may read it. What is inside a file is not searched.
4. **All the words of a `contains`** must be found in the issue, each in any of the texts searched (the lead, from the review of M5 WP-05): `contains "login error"` finds an issue with *login* in its title and *error* in a comment. One `contains` holds a limited number of words (a search limit, architecture §7), so the statement stays bounded.
5. **The order of results** when a search has text and no `sort by`: issues whose title has the words first, then the newest, then the Issue ID. The words that decide it are those of the first `contains` on its own that every found issue must meet (not one inside `or` or `not`); a search without such a `contains` is ordered newest first (the lead, from the review of M5 WP-04).
6. **The index is a reader of the outbox** (ADR-0007): it keeps its place, catches up after a restart, and `bugaboo search-index rebuild` can make it again from the stored texts. When the rule that prepares a text changes, a migration empties the index and puts a mark on the place of the reader (migration 25, `REBUILD_WANTED`). The server then, beside the jobs, takes the head of the outbox, makes the rows of every issue again a page and a transaction at a time, and puts the reader at that head. Until it has finished, the index is empty or partial and a search finds less; the jobs do not wait, and the reader reads no event until the rebuild is done, then reads what came after the head. A benchmark dataset builds its index again when its marker is older than that migration.

Enforced by: ARCH-DB-PARITY (the same issues in the same order on both databases), SEC-MATRIX (the `search` entry point with a hidden field, comment and file), and the tests of M5 WP-05.

## Enforced by

SEC-MATRIX (search entry point), ARCH-DB-PARITY (see `tools/checks/catalog.toml`).
