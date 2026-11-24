# WP-05 — The relation map on the server

**Wave 1 · Folder / branch:** `bugaboo.win5` / `WIN5` · **Model:** Opus (a walk that must never pass through what the reader cannot see, property tests, speed on 1 M issues)
**Features:** ISS-16 on the server (the screen is WP-12's). UX design §4.15 (*Security*, *Speed and size*); answers M10-Q11, Q20; the roadmap's check "no chains through hidden issues".

## Goal

The server answers the map of an issue: every issue connected to it through the kinds of relation chosen, up to the steps chosen, at most 200 issues, each with its steps away, ID, title, status, type and priority, and the relations between them. It answers the map of a search's results the same way. The walk passes only through issues the reader sees, so a map can never reveal that a hidden issue exists, not even as a gap. Loops of *blocks* are found and named.

## Where

- **Owns:** `crates/bb-app/src/relations/map.rs` (new: the use case), the walk in `crates/bb-store/src/relations.rs` or a new `crates/bb-store/src/relation_map.rs` (built on the walk of `connected to` in `crates/bb-store/src/search/filter.rs`, which is shared, not copied), `ViewKind::Map` in `crates/bb-core/src/saved_search.rs`, the routes `GET /api/v1/issues/{id}/map` and `POST /api/v1/map` (`crates/bb-api/src/api/`), the tool `relation_map` (`crates/bb-app/src/ai/`), the matrix entry point `map` (`crates/bb-server/src/tests/matrix/`), the property tests (`crates/bb-app/src/tests/relation_map_sight.rs`, new).
- **Shared files, one entry each:** `saved_search.rs` (WP-04 adds *board* and *calendar*), `router.rs`, the tool registry, `messages/en.json`, the catalogue entry of SEC-MATRIX.
- **Does not touch:** the relations' use cases (`relate.rs`, `dependencies.rs`: read only), any screen.
- No migration. If the measurement needs an index on the relations, the lead gives out the next free number.

## What is decided

- **The walk** (M10-Q11): steps 1, 2, 3 or *Everything*, which is 5, the one limit of the search language (`limits.rs`; no second number). Kinds chosen from the kinds of relation the organization has, plus *part of* (sub-issues). Both directions of each kind. At most 200 issues: the walk stops inside the statement when it has them and the answer says `more: true`, so the screen suggests fewer steps or fewer kinds.
- **Only through what the reader sees** (ISS-16, PRIV-13): every step of the walk joins only issues of the reader's sight (`sight_condition`, the one the search uses). An issue reached only through a hidden one is not in the answer; nothing counts or names the hidden one. `more` is worked out on the visible walk only.
- **What it sends** (UX design §4.15, PERF-02): per issue its ID, title, status and status group, type and priority, and steps away; per relation its kind, its direction and the word of each end. Fields the reader may not read are left out as everywhere (SEC-06). People are not on the map.
- **Loops** of a kind with *Must finish first* and of *blocks* (ADR-0043: those stored before migration 49) are found in the answer and named as the issues on them, in order; the screen flags them.
- **The map of a search** (`ViewKind::Map`): the results of the search (at most 200, by its sort) and the relations between them, no walk beyond them.
- **AI apps and agents** (M10-Q20): `relation_map` reads the same route as the person, and its text of titles is marked as untrusted text like every tool answer (ADR-0029).

## Tasks

1. Write the acceptance tests and the property tests below.
2. The walk on both databases, sharing the walk of `connected to`; the cap inside the statement.
3. The use case, the routes, the answer's shape; loops.
4. `ViewKind::Map` and the map of a search.
5. The tool `relation_map`.
6. SEC-MATRIX grows: the entry point `map` and the named property tests; the catalogue entry, the check's code and its examples, by "Adding a rule".
7. A first measurement of `relation-map-three-steps` on the large dataset, early in the package; architecture §8 (*The map's walk*), §11, §12 (*Board, calendar, map and dashboards (M10)*).

## Acceptance tests (write first)

- `iss_16_the_map_of_an_issue_holds_every_issue_within_the_steps_chosen` (both databases).
- `iss_16_kinds_shown_narrow_the_walk_and_part_of_is_one_of_them`.
- `iss_16_everything_means_five_steps_the_limit_of_the_search_language`.
- `iss_16_the_walk_stops_at_200_and_says_there_are_more`.
- `iss_16_no_chain_passes_through_a_hidden_issue` (property test: random graphs, random hidden projects for the reader).
- `iss_16_the_map_is_the_map_of_the_issues_the_reader_sees` (property test: the answer equals the map of the graph with the hidden issues taken out; `more` included).
- `iss_16_a_field_the_reader_may_not_read_is_not_on_the_map`.
- `iss_16_a_loop_of_blocks_is_named_as_the_issues_on_it`.
- `iss_16_the_map_of_a_search_holds_its_results_and_the_relations_between_them`.
- `ai_21_relation_map_answers_what_the_person_would_see_and_marks_titles_untrusted` (ARCH-AI-TOOLS).
- SEC-MATRIX: the entry point `map`, for every starting role, both databases.
- PERF-N1: a map of 200 issues makes a fixed number of statements.

## Checks

SEC-MATRIX (grown), ARCH-DB-PARITY, ARCH-API-FREEZE, ARCH-AI-TOOLS, TEST-PROMPT-INJECTION (the tool's answer), PERF-N1, PERF-BENCH, ARCH-NO-COUNTERS.

## Docs to update

Architecture §8, §10 (SEC-MATRIX grown), §11, §12; ADR-0005 (the 5 steps as the one limit) if WP-01 left a line for the code. The user docs come with the screen (WP-12).

## Review

Code review with the SSOT/DRY/ODR gate: one walk for the search and the map, one limit of steps, one sight condition. **Security review before the merge** (fresh, Opus): the property tests, the cap and `more`, a timing that grows with hidden issues.

## Personal data

None: the map carries no people. It reads titles, which may hold personal data written by people; they reach only readers who may read the issue.

## Depends on

WP-01 merged (the words, *Kinds shown*).
