# WP-04 — Searching on the server: the compiler, permissions, limits, the search routes

**Wave 2 · Folder / branch:** `bugaboo.win2` / `WIN2`
**Features:** QRY-04 (the helpers on the server), QRY-05, SEC-17, PERF-05 (search routes), PERF-01 (search scenarios). ADR-0004, ADR-0005 (amendment M5), ADR-0006, ADR-0022, ADR-0008 (additions after the freeze); architecture §5 runtime "Searching", §7 "Search limits"; follow-ups M2-F65, M3-F24, M3-F45, M3-F68, M3-F120, M4-F11, M4-F12.

## Goal

A search typed by a person becomes one parameterized statement that finds exactly the issues that person may see, filtered, sorted and counted only by fields they may read, on SQLite and PostgreSQL alike, within the limits of SEC-17. A project's list is the same search with `within KEY`.

## Owns

`crates/bb-store/src/search/**` (the compiler), `crates/bb-app/src/search/**` (the use case and its port), `crates/bb-api/src/api/search*` (the routes), the issue list route and its use case (it becomes a search), the migrations for the indexes the compiled joins need, the search scenarios of the benchmark (WP-03's registry), the `search` entry point of SEC-MATRIX, the PERF-N1 tests of the search routes.

## Tasks

1. **The field catalogue for a person** (`bb-app`): built from the field definitions, the person's field levels (`Authorized<T>`, `field_level`) and *Who sees people*. It is the catalogue the checker of WP-02 gets, built once per request; the browser asks for the same catalogue through a route, so both sides check against the same fields.
2. **The compiler** (`bb-store`): the checked tree → one statement through the query builder, with the visibility predicate of ADR-0004 in it (sees the project, and *See issues* or reported it), a join per own field used (ADR-0006), relative times turned into moments in the person's region zone, the helpers (`member of`, `within`, `under`, `blocks`, `blocked by`, `connected to … within N steps` as a recursive query with the step limit), and `contains` through the port of WP-05 (task 6).
3. **Sorting and paging:** `sort by` any readable field, with the ID as the last key, so issues with the same value keep one order and a cursor never skips or repeats one (M3-F24). The cursor is opaque and carries the sort values. Without `sort by`: newest first, or with text the order of ADR-0016 (amendment M5).
4. **Counts and groups:** the count line and the count per group (UX §4.2) come from the same compiled search; a group by a field the person may not read is refused like the field.
5. **The limits** (architecture §7 "Search limits"): 2 seconds per statement on both databases (PostgreSQL `statement_timeout` for the transaction, SQLite a progress handler), answered as `search.too-slow`; the rate of 60 searches a minute per person through the existing limiter; page sizes as today.
6. **The port of full-text search** (shared with WP-05, which implements it): `contains` compiles to a condition on the text index with the sources the reader may see. Agree the shape with WP-05 through the lead before you write it; until WP-05 is merged, a test double stands in.
7. **Routes** (additions to API v1, ADR-0008): a search across the projects a person sees (`q` = the canonical text, `cursor`, `limit`, `fields`), its count and groups, the field catalogue of the person, and the project's list as the same use case. `listIssues` keeps its shape and becomes a search with `within`. Every route answers a mistake in the search as problem+json with the checker's message key and range.
8. **Indexes:** the indexes the compiled joins and sorts need on both databases (M3-F45), measured with WP-03's benchmark on 1 M issues before and after.
9. **The follow-ups:** a `Version` type in the store ports (M4-F11); setting keys typed in the ports (M4-F12); the history cap that could hint at hidden rows (M3-F68); the capped lists that become searches (M3-F120).

## Acceptance tests (write first)

- `qry_05_a_hidden_field_in_a_filter_sort_or_group_gives_the_unknown_field_answer`, on both databases, also through an access key.
- `qry_05_a_hidden_field_takes_as_long_as_an_unknown_field` (no measurable difference over many tries).
- SEC-MATRIX: the `search` entry point for every permission that hides an issue or a field.
- `sec_17_a_search_text_never_becomes_sql` (quotes, comment marks, statement ends in values).
- `sec_17_a_search_over_its_time_answers_too_slow`, `sec_17_the_sixty_first_search_in_a_minute_waits`.
- PERF-N1: a page of forty results runs as many statements as a page of five, for every search route.
- ARCH-DB-PARITY: a table of searches gives the same issues in the same order on both databases.
- The benchmark scenarios of PERF-01 for a saved search and for the complex search of architecture §8, within budget on 1 M issues.

## Depends on

WP-02 (the tree and the checker) and WP-03 (the benchmarks), merged. Runs beside WP-05 and WP-06.
