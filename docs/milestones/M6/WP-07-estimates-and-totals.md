# WP-07 — Estimated time, time left and time spent: totals, search and columns

**Wave 2 · Folder / branch:** `bugaboo.win4` / `WIN4`
**Features:** TIME-03; QRY-01 and CFG-02 for the three fields; PRIV-13 for the totals. ADR-0035 §4, ADR-0005 decision 10 (lengths of time in the search language), ADR-0004 (visibility compiled into the statement).

## Goal

An issue has an *Estimated time* and a *Time left*, and its *Time spent* is the total of its time entries; an issue with sub-issues and a project show the totals below them. A total counts only what the reader may see, so it never tells about a hidden issue or a field the reader may not read. The three fields can be searched, sorted, grouped and shown as list columns like any other field.

## Owns

The columns of *Estimated time* and *Time left* on the issue in `bb-store` and their migration (number from the lead), `crates/bb-app/src/time/totals*`, the three fields in the field catalogue of `bb-core` (as stored fields) and in the readable fields of the search (`bb-app/src/search/readable.rs`), the issue page's Details rows of the three fields, the three list columns.

## What is decided (M6-Q6)

- *Estimated time* and *Time left* are lengths in minutes on the issue, changed like any field (a version, an event, the history, field levels). *Time spent* is read, never typed.
- While nobody has typed *Time left*, it is *Estimated time* minus *Time spent*, never below zero, shown in the quiet style of a value worked out; a typed value wins until it is cleared.
- **Totals:** an issue shows its own numbers and, when it has sub-issues, the totals with them (all levels below); a project shows the totals of its issues and its sub-projects. A total is computed over the issues the reader may see and only for the fields the reader may read, by the same compiled search that gives the list (one statement), and through `TimeSpentStore` for *Time spent*.

## Tasks

1. **Store and change** *Estimated time* and *Time left* (the change of one field, many issues at once through the existing run, the history).
2. **The worked-out *Time left*** in one place (`bb-core`), used by the issue answer, the list and the search.
3. **Totals** of an issue with its sub-issues and of a project with its sub-projects, through the search compiler, on both databases; the issue page and the project page show them with their words (*"With sub-issues: 12 h estimated · 9 h 30 min spent"*).
4. **Search:** `estimated time`, `time left` and `time spent` as fields of the search language (the words and the minutes of ADR-0005, decision 10), sortable and usable in *Group by* only as ranges, never as a list of people; a reader who may not read the field gets the answer of an unknown field.
5. **Columns:** the three fields in *Customize › Columns* of the list, written by the length helper of WP-03.
6. **Speed:** the totals of a project on 1 M issues within the issue-page budget of PERF-01 (WP-12 measures it nightly; this package adds the scenario).

## Acceptance tests (write first)

- `time_03_time_left_is_the_estimate_minus_the_time_spent_until_someone_types_it`.
- `time_03_the_totals_of_an_issue_count_its_sub_issues_at_every_level`.
- `priv_13_a_total_never_counts_an_issue_the_reader_cannot_see` (a sub-issue in a project the reader may not open changes nothing in the total they see).
- `priv_13_a_total_of_a_field_the_reader_may_not_read_is_not_shown`.
- `time_03_time_spent_is_searchable_in_hours_and_minutes` and `qry_05_a_hidden_time_field_answers_like_an_unknown_field`.
- ARCH-DB-PARITY: the same totals on both databases.
- PERF-BENCH: the scenario `project-time-totals`.

## Checks

SEC-MATRIX (totals under each role), ARCH-DB-PARITY, PERF-N1, PERF-BENCH (a new scenario), TEST-SEARCH-ROUNDTRIP (the three fields in the generated trees).

## Review

The SSOT/DRY/ODR gate of coding-standards §1: the worked-out *Time left* once; the totals by the one compiled search, not by a second walk of the tree in Rust; the lengths written by the helper of WP-03. A security reviewer checks every total against PRIV-13.

## Depends on

WP-01 (the port shape), WP-03 merged. Runs beside WP-05 (which implements the port); its tests use a fake of the port until WP-05 is merged, then the two-database tests run after a merge of `main`.
