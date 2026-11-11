# WP-03 — Dates and dependencies on the server

**Wave 1 · Folder / branch:** `bugaboo.win3` / `WIN3`
**Features:** PLAN-01, PLAN-02, PLAN-03 (on the server). [ADR-0043](../../adr/0043-dates-and-dependencies.md) decisions 1, 4, 6; follow-up M3-F101.

## Goal

An issue has a *Start date* and a *Due date* that people set, search and see in its history, and a kind of relation can mean *Must finish first*. Everything the plan (WP-06) and the timeline (WP-09) need from issues and relations is on the server and in the API, with nothing drawn yet.

## Owns

The two fields in the field catalogue (`crates/bb-core/src/fields.rs`, `crates/bb-core/src/issue.rs`) and in the search language (`crates/bb-searchlang/src/fields.rs`, the compiler in `crates/bb-store`), the issue's change and read in `crates/bb-app/src/issues/**` (dates only), the date effect of a kind of relation (`crates/bb-core/src/relation.rs`, `crates/bb-app/src/relations/**`, `crates/bb-store/src/relations*`), migration 49 (`crates/bb-store/src/migrations/m0049_dates_and_date_effect.rs`), the API members and routes (`crates/bb-api/src/api/issues.rs`, `relations.rs`, `fields*`), the history sentence of a date change, the field on the issue page and the new-issue form only as far as M3-F101 needs (`web/app/src/features/issues/` date row), the date effect on *Kinds of relation* (`web/app/src/features/relation-kinds/**`).

**Shared files, one entry each; rebase conflicts expected:** the three packages of wave 1 each add one entry to `crates/bb-core/src/settings/catalog.rs`, `crates/bb-store/src/migrations/mod.rs`, the API router (`crates/bb-api/src/router.rs`) and `messages/en.json`, and nothing else in those files. The one who lands later rebases and keeps both entries.

## What is decided

- **Two built-in fields** (ADR-0043, decision 1): *Start date* and *Due date*, calendar dates with no time of day, on the issue row, with the starting field levels of *Estimated time*, search words `start date` and `due date`, places on forms, in layouts and as list columns. A start after the due date of the same issue is refused (`issue.start-after-due`, one error code), on every way in (form, API, change of many issues, tool of the registry).
- **The date effect** (decision 4): a column of the kind of relation with *none* or *Must finish first* (a Rust enum with one effect besides none, so V1 adds a variant). Migration 49 gives *blocks* the effect, read by the stable mark of the starting kind, never by its name. At most two kinds carry it (`DATE_EFFECT_KINDS_MOST`); the third is refused with a sentence that names the two.
- **No loops:** making a relation of a kind with the effect, or giving the effect to a kind, is refused when it would close a loop of dependencies; the sentence names one loop as issue IDs the person may see (a hidden issue in the loop is named *an issue you can't see*). Loops stored before migration 49 stay (ADR-0043, decision 4).
- **Every change of a date is an event** with the old and the new value (decision 6), on the versioned issue (ARCH-VERSIONED): the history says *"Lea moved WEB-280 from 3–7 Nov to 10–14 Nov"* when both change, *"Lea set the due date of WEB-280 to 14 Nov"* when one does. *Undo* of a date change is the usual change back. A move of several issues in one step (used by *Move it after* in WP-06) is one run of the existing run model (`change_run`), undone in one step.
- **Search** reads the dates like any date field: `due date < 2026-12-01`, `start date is empty`, sorting and grouping by them. The search round trip covers them.
- **API v1 grows only by additions:** two optional members of an issue, the date effect of a kind of relation, and a route that moves the dates of several issues in one run (WP-06 calls it). Nothing renamed or tightened (ARCH-API-FREEZE).

## Tasks

1. Write the acceptance tests below.
2. The fields: catalogue entries, columns (migration 49), personal-data marks (no: ADR-0043 dates are not personal data, architecture §12 *Personal data in M8*), field levels, the issue's read and change, forms and layouts, search words and the compiler on both databases.
3. M3-F101: *Due date* is no longer blocked on forms; the row of the follow-up is closed with this package.
4. The date effect on kinds of relation: the column, the starting value for *blocks*, the rule of two kinds, the loop check, the route, and the choice on *Kinds of relation* (*Date effect: None / Must finish first*), with pictures in four themes on computer and phone.
5. The history sentences of date changes (in the one wording of `bb-core::wording`, ADR-0033 amendment M4 WP-13), and the move of several dates as one run.
6. The tool registry: the tools that change fields accept the two dates under the same rules (ARCH-AI-TOOLS).

## Acceptance tests (write first)

- `plan_01_start_and_due_date_are_fields_with_field_permissions` (both databases).
- `plan_01_a_start_after_the_due_date_is_refused_on_every_way_in`.
- `plan_01_search_filters_sorts_and_groups_by_the_dates` (and the round trip of TEST-SEARCH-ROUNDTRIP).
- `plan_01_the_due_date_can_be_set_on_the_new_issue_form` (browser; M3-F101).
- `plan_02_blocks_has_the_date_effect_after_migration_49`.
- `plan_02_at_most_two_kinds_carry_must_finish_first`.
- `plan_02_a_relation_that_closes_a_loop_of_dependencies_is_refused`.
- `plan_02_a_loop_names_a_hidden_issue_only_as_an_issue_you_cannot_see`.
- `plan_03_a_date_change_is_one_event_in_the_history_and_undone_by_undo`.
- `plan_03_a_move_of_several_issues_is_one_run_undone_in_one_step`.
- `plan_03_a_date_change_with_an_old_version_is_refused_with_the_newer_value`.
- SEC-MATRIX: the routes of dates and of the date effect, for every starting role, both databases.

## Checks

DATA-MIGRATIONS, DATA-MARKS, ARCH-DB-PARITY, ARCH-EVENTS, ARCH-VERSIONED, ARCH-API-FREEZE, ARCH-AI-TOOLS, SEC-MATRIX, TEST-SEARCH-ROUNDTRIP, PERF-N1, TXT-TERMS, TEST-A11Y, THEME-ONLY.

## Review

Code review with the SSOT/DRY/ODR gate of coding-standards §1: the dates are fields of the one field catalogue (no second list of date fields), the date effect is one enum, the loop check is one walk over the relations that the plan of WP-06 can reuse, the history sentence is written once in `bb-core`. A UX review of the pictures of *Kinds of relation* in four themes, on computer and phone.

## Depends on

WP-01 (ADR-0043, the words) and WP-02 merged (a green full run).
