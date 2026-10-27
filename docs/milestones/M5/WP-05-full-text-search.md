# WP-05 — Full-text search on both databases

**Wave 2 · Folder / branch:** `bugaboo.win3` / `WIN3`
**Features:** ISS-13, QRY-04 (`contains`). ADR-0016 with its amendment M5, ADR-0007 (outbox readers); architecture §7.

## Goal

`contains "crash on save"` finds the issues whose title, description, comments or file names have those words, on SQLite and PostgreSQL with the same results, and a hit never comes from a text the reader may not see.

## Owns

`crates/bb-store/src/text_index/**`, the outbox reader that keeps the index (in `bb-app`, beside the history and notifications readers), the text-preparing function in `bb-core`, the migration of the index tables, the port the compiler of WP-04 calls (its shape agreed with WP-04 through the lead), a rebuild command in `cargo xtask`, the text-search scenarios of the benchmark.

## Tasks

1. **One way to prepare text** (`bb-core`): lower case, Unicode NFC (no letter changed into another, M5-F53), split into words by one rule, used for what is indexed and for the words of a search. A word ending in `*` matches by its start. Tests with German and English text (umlauts, ß, hyphens, issue IDs, e-mail-like words).
2. **The index:** one row per text, with the issue, its source (title, description, a comment, a file name, an own long-text field) and the prepared words; SQLite FTS5 (the `ascii` tokenizer, the prepared words only, from migration 25) and PostgreSQL `tsvector` made from the stored words split at spaces (migration 25) with a GIN index. Deleted and trashed issues and comments leave the index with them; a restored one comes back.
3. **The reader of the outbox:** keeps its place, is idempotent, catches up after a restart, and never holds up a change. A rebuild from the stored texts for a new database or after a change of the rule in task 1.
4. **The port:** for a set of words and the sources the reader may see (fields they may read; comments and files only on issues they see), the condition the compiler puts into its one statement, and the order of ADR-0016 amendment M5 (title hits first, then the newest).
5. **The sample and the large dataset** get their index built; the benchmark gets text-search scenarios (one common word, one rare word, a prefix) within the 300 ms of a complex search.

## Acceptance tests (write first)

- `iss_13_a_word_in_a_comment_finds_the_issue`, and the same for the title, the description, a file name and a long-text field.
- `iss_13_a_hidden_long_text_field_never_finds_an_issue`, `iss_13_a_comment_on_an_issue_the_reader_cannot_see_never_finds_it`.
- `iss_13_crash_does_not_find_crashes_but_crash_star_does`; `iss_13_case_and_the_way_a_letter_is_written_do_not_matter_but_the_letter_does`.
- ARCH-DB-PARITY: the same words find the same issues in the same order on both databases.
- The reader: a change while the server is stopped is in the index after a start; reading an event twice changes nothing.

## Depends on

WP-02 merged (for `contains` in the tree). Runs beside WP-04 and WP-06; the compiler uses this port, so the lead merges WP-05 before WP-04 if both are ready.
