# WP-03 — Totals on the server

**Wave 1 · Folder / branch:** `bugaboo.win3` / `WIN3` · **Model:** Opus (the search compiler, field permissions per issue, PRIV-13, speed on 1 M issues)
**Features:** QRY-08 (owner; the screens are WP-13's). ADR-0050 (WP-01); answer M10-Q15; follow-up M5-F44.

## Goal

Any search can be answered as totals grouped by one field: for each group the count of issues and the sums of *Estimated time*, *Time left*, *Time spent* and the own number fields the reader may read. A total never includes an issue or a value the reader may not see. Boards, dashboards and grouped lists read this one route.

## Where

- **Owns:** the totals in `crates/bb-app/src/search/` (`request.rs`, `run.rs`, a new `totals.rs`), the compiler of grouped sums in `crates/bb-store/src/search/` (a new `totals.rs` beside the groups), the paging of groups (M5-F44) in the same files, `crates/bb-api/src/api/search.rs` (one new route, `POST /api/v1/search/totals`), the tool `search_totals` (`crates/bb-app/src/ai/`).
- **Shared files, one entry each:** `router.rs`, the tool registry, `messages/en.json`.
- **Does not touch:** saved searches and view kinds (WP-04), dashboards (WP-06), any screen.
- No migration. If the measurement needs an index, the lead gives out the next free number.

## What is decided

- **One route, one shape** (M10-Q15): `POST /api/v1/search/totals` takes the search text, the field to group by (any field *Group by* takes) and the sums wanted (`count`, `estimated`, `time_left`, `time_spent`, `number:<field>`). It answers one row per group, with the group key shown as `Group by` shows it (`PersonRef` for people, ADR-0022), and a row for all groups together. The existing `LengthTotals` and `GroupCount` become rows of this shape, so there is one way to add up a search (ODR).
- **Over what the reader sees** (PRIV-13): only the issues the search finds for the reader. A sum of a field counts a value only where the reader may read that field on that issue (field permissions per project, SEC-04); the row says how many issues gave a value (`valued`), so "12 h over 9 of 14 issues" is honest without telling why. A field the reader may read nowhere is refused as an unknown field (QRY-05), with the same error.
- **Groups as pages** (M5-F44): over 200 groups the answer pages by the group key with a cursor, instead of cutting at 200; the board (WP-04) and the box of a dashboard (WP-06) read the first page.
- **People in groups:** a group by a person field follows *Who sees people*: *Role only* groups come as "Someone (Team member)" merged by role, *Hidden* as one group "Someone". The rows are in the order of the group key, never by size, when the field is a person (M10-Q13, PRIV-09); other fields may be sorted by count.
- **Limits:** the limits of a search hold (SEC-17: 2 seconds, then `search.too-slow`); a total never counts more than the search may read.
- **AI apps and agents** (M10-Q20): `search_totals` reads the same route as the person; it is never used on *Time factor* (M8-Q5: no total uses it) and refuses a group by it.

## Tasks

1. Write the acceptance tests below.
2. The shape and the route; `LengthTotals` and `GroupCount` folded into it.
3. The compiler of grouped sums on both databases, with field permissions per project inside the statement.
4. Paging of groups (M5-F44); the row of the follow-up closed.
5. The tool `search_totals`.
6. A first measurement of `totals-by-priority-biggest-project` and of a total of *Time spent* on the large dataset, written into architecture §11 (the exit's measurement is WP-15's).
7. Architecture §8 (*Totals*), ADR-0050's *Enforced by* lines.

## Acceptance tests (write first)

- `qry_08_totals_group_a_search_by_one_field_with_count_and_three_lengths` (both databases).
- `qry_08_a_total_never_includes_an_issue_the_reader_cannot_see` (PRIV-13).
- `qry_08_a_number_field_counts_only_where_the_reader_may_read_it_and_says_how_many_gave_a_value`.
- `qry_08_a_field_the_reader_reads_nowhere_is_refused_like_an_unknown_field`.
- `qry_08_people_groups_follow_who_sees_people_and_are_in_the_order_of_names`.
- `qry_08_over_200_groups_the_answer_pages_by_the_group_key` (M5-F44).
- `qry_08_the_total_of_all_groups_equals_the_sum_of_the_rows`.
- `qry_08_grouping_or_summing_by_time_factor_is_refused`.
- `ai_21_search_totals_answers_what_the_person_would_see` (ARCH-AI-TOOLS).
- SEC-MATRIX: the totals route for every starting role, both databases, in the entry point `search`.
- PERF-N1: totals of a grouped search make a fixed number of statements.

## Checks

ARCH-DB-PARITY, ARCH-NO-COUNTERS (grows: a total that adds hidden issues as a bad example), ARCH-PERSON-REF, ARCH-API-FREEZE, ARCH-AI-TOOLS, SEC-MATRIX, PERF-N1, PERF-BENCH, TXT-TERMS.

## Docs to update

Architecture §8 and §11; ADR-0050; ADR-0005 if the request names a new word. The user docs come with the screens (WP-13).

## Review

Code review with the SSOT/DRY/ODR gate: one shape of totals, the old two folded in, field permissions checked in one place. Security review: a total over a field hidden in some projects, a person group under *Role only*, a search that finds nothing.

## Personal data

Totals of *Time spent* add up time entries, which are employee data (PRIV-09, ADR-0035): they are only sums over issues, never per person unless the reader groups by a person field they may see, and never ranked. Nothing is stored.

## Depends on

WP-01 merged (ADR-0050, the words).
