# WP-02 — Releases on the server

**Wave 1 · Folder / branch:** `bugaboo.win2` / `WIN2` · **Model:** Opus (a new record with its own permission, two built-in fields read by search, forms and the plan)
**Features:** ISS-08 (owner; the screens are WP-11's, the timeline WP-07's). ADR-0049 (WP-01); answers M10-Q6, Q7, Q8, Q20; follow-ups M8-Q3 (in part), M8-F57.

## Goal

A project has releases. A person with *Manage releases* makes one, gives it a *Release date*, marks it *Released* and later *Retired*. Every issue of the project and its sub-projects can name the releases a bug was *Found in* and the one release its fix is *Planned for*; people search, sort and group by both, see them in the history, and AI apps read and set them as the person. Nothing is drawn yet beyond what the API answers.

## Where

- **Owns:** `crates/bb-core/src/release.rs` (new: the record, its states, its name rules), the two built-in fields in the field catalogue (`crates/bb-core/src/fields.rs`, `crates/bb-core/src/issue.rs`), `crates/bb-app/src/releases/**` (new: make, change, mark released, retire, delete, list offered in a project), the issue's read and change of the two fields in `crates/bb-app/src/issues/**` (these fields only), `crates/bb-store/src/releases.rs` (new) and the compiler's words in `crates/bb-store/src/search/` (the two fields only), `crates/bb-searchlang/src/fields.rs` (the reserved words get their field), migration **65** (`crates/bb-store/src/migrations/m0065_releases.rs`: the table `release`, `issue_found_in_release`, the column of *Planned for release* on the issue, their indexes), `crates/bb-api/src/api/releases.rs` (new) and the issue members, the permission `manage-releases` in `crates/bb-policy/src/catalogue.rs`, the tool `list_releases` and the two fields in `change_issue` (`crates/bb-app/src/ai/`), the history sentences in `bb-core::wording`.
- **Shared files, one entry each:** `catalog.rs`, `migrations/mod.rs` and `list.rs`, `router.rs`, `catalogue.rs`, the tool registry, `messages/en.json`.
- **Does not touch:** the plan (`crates/bb-app/src/plan/**`, WP-07), any screen (WP-11).

## What is decided

- **Where a release lives** (M10-Q6): in one project; offered in that project and its sub-projects. Its name is unique within its project (`bb_core::names::same_name`, `release.name-taken`). An issue moved out of the tree keeps its releases; the answer marks them `offered: false`.
- **Fields and states** (M10-Q7): name, *Release date* (a calendar date, optional, ADR-0043's kind of date), state *Planned* → *Released* (keeps *Released on*, the day it was marked) → *Retired* (no longer offered for new choices, still shown on issues). *Released* may go back to *Planned* with a reason in the history. One enum of states in `bb-core`.
- **On issues:** *Found in release*, several (at most 20, a limit of its own: a named constant beside the field); *Planned for release*, one. Both are built-in fields with field levels and field permissions like *Due date*, places on forms and in layouts, list columns, search words `found in release` and `planned for release` (equality, `in (…)`, `is empty`, sorting by release date, grouping). A choice of a release that is not offered where the issue is is refused (`release.not-offered`).
- **The permission *Manage releases*** (project, *Manage*, topic *Work settings* like `manage-labels`, so no agent holds it; Administrator and Project lead at the start): make, change, mark, retire and delete. Reading a release needs only seeing its project. A release that an issue names cannot be deleted (`release.in-use`, which counts nothing the person cannot see: it says only that issues name it); it is retired instead. Deleting one that no issue names goes to the trash with *Undo* like issues (explicit deletion).
- **Every change is an event** (ADR-0007, ARCH-VERSIONED): made, renamed, dated, marked, retired, deleted; and on the issue the change of either field with old and new values. History sentences in the one wording.
- **AI apps and agents** (M10-Q20): `list_releases` (read: the releases offered in a project the person sees); the two fields through `change_issue` with its preview. No tool makes, marks or deletes a release.
- **API v1 grows only by additions:** the release routes under `/api/v1/projects/{key}/releases`, two optional members of an issue (ARCH-API-FREEZE).

## Tasks

1. Write the acceptance tests below.
2. The record, the states and the permission; migration 65 on both databases; marks and person links of `made_by` and `changed_by` columns (DATA-MARKS).
3. The two fields in the catalogue, the issue's read and change, forms and layouts, the field levels.
4. Search: the words, the compiler on both databases, the round trip.
5. The use cases and routes; the trash and *Undo*; the history sentences.
6. The tool registry: `list_releases` and the two fields in `change_issue`.
7. Architecture §7 (*Storage types*: the release tables), §8 (the release words), §12 (*Releases (M10)*); ADR-0049's *Enforced by* lines filled with the test names.

## Acceptance tests (write first)

- `iss_08_a_release_is_offered_in_its_project_and_sub_projects_only` (both databases).
- `iss_08_a_release_name_is_unique_within_its_project`.
- `iss_08_only_manage_releases_makes_changes_marks_and_deletes_a_release`.
- `iss_08_a_release_moves_planned_released_retired_and_keeps_the_day_released`.
- `iss_08_a_retired_release_is_not_offered_but_stays_on_its_issues`.
- `iss_08_a_release_an_issue_names_cannot_be_deleted_and_the_refusal_counts_nothing`.
- `iss_08_found_in_takes_up_to_20_releases_and_planned_for_one`.
- `iss_08_a_release_not_offered_where_the_issue_is_is_refused_on_every_way_in` (form, API, change of many issues, `change_issue`).
- `iss_08_the_two_fields_follow_field_permissions_everywhere` (list, search, history, e-mail, MCP).
- `iss_08_search_filters_sorts_and_groups_by_both_release_fields` (and TEST-SEARCH-ROUNDTRIP).
- `iss_08_every_release_change_is_an_event_with_old_and_new_values`.
- `iss_08_an_issue_moved_out_of_the_tree_keeps_its_release_marked_not_offered`.
- `ai_21_list_releases_reads_only_releases_of_projects_the_person_sees` and `ai_21_change_issue_sets_planned_for_release_with_its_preview` (ARCH-AI-TOOLS).
- SEC-MATRIX: every release route and the two issue members, for every starting role, both databases.
- PERF-N1: the list of releases and the list of issues with both columns.

## Checks

DATA-MIGRATIONS, DATA-MARKS, ARCH-DB-PARITY, ARCH-EVENTS, ARCH-VERSIONED, ARCH-API-FREEZE, ARCH-PERMS, ARCH-AI-TOOLS, SEC-MATRIX, TEST-SEARCH-ROUNDTRIP, PERF-N1, TXT-TERMS, TXT-CATALOG, ARCH-NO-COUNTERS.

## Docs to update

Architecture §7, §8, §12 as in task 7; ADR-0049 and the ADR-0025 amendment (*Manage releases*); the user docs come with the screens (WP-11).

## Review

Code review with the SSOT/DRY/ODR gate: one enum of states, the two fields as entries of the one field catalogue (no second list of release fields), the offered-in rule written once and used by forms, API, many changes and tools. Security review: the refusal of a delete tells nothing about issues the person cannot see.

## Personal data

A release is business data, not personal; its events name who changed it (`PersonRef`, *Who sees people*). Releases are kept until someone with *Manage releases* deletes them (ADR-0045: not deleted by themselves). At *Remove a person* their changes stay as *Former user*.

## Depends on

WP-01 merged (ADR-0049, the words).
