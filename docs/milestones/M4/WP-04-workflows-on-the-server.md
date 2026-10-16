# WP-04 — Workflows on the server: statuses as data, status changes and their rules, starter workflows

**Wave 2 · Folder / branch:** `bugaboo.win2` / `WIN2`
**Features:** WF-01, WF-03, WF-04 (server), WF-06, ISS-02 (a workflow per issue type), PRJ-02 (inherited down the project tree), ISS-09 (changes follow the workflow); ADR-0031, ADR-0027, ADR-0019, ADR-0017; follow-ups M3-F22, M3-F44, M3-F53, M3-F122, M3-F124, M3-F154 (if it touches those files).

## Goal

An organization and its projects have their own workflows. Every status change, wherever it comes from, is decided by one judge that knows *Who can do it*, *Only if …*, *Must be filled in* and *Ask for*. Stored issues keep their statuses.

## Owns

- `crates/bb-core/src/workflow*.rs` (new) and the move of `bb-core/src/issue.rs`'s `Status` to data.
- `crates/bb-app/src/workflows/**` (new) and the status part of `issues/change.rs`, `issues/create.rs`, `many_changes/**`.
- `crates/bb-store/src/workflows.rs` (new) and migration `m0015`.
- `crates/bb-api/src/api/workflows*` (new) and the status in the issue answers.
- The permission *Manage workflows* in `crates/bb-policy/src/catalogue.rs`.
- The rows of SEC-MATRIX for these routes.
- TEST-WORKFLOWS (planned → active).
- `userDocs/workflows/*` (new).

The screens are WP-07. The browser only stops depending on a fixed list of statuses, because the generated client changes (task 3).

## Tasks

1. **Statuses as data (ADR-0031 §1).**
   - Tables for workflows, statuses (key, name, group, position) and status changes (from, to, button text, rules).
   - The starting workflow becomes the organization's *Simple*, keeping the keys `open`, `in-progress` and `done`. A database written by M3 keeps every issue as it is.
   - `closed_after` follows the *Done* group of the data.
2. **Which workflow counts (§2):** a row per place and issue type, found with `nearest_row`. A project may change the workflow it inherited; its sub-projects follow.
3. **The API:**
   - **Statuses:** a status is sent as key, name and group, no longer an enum in the OpenAPI document. Fix every browser use of `statusWords` so the build stays green; the screens keep working with the same three statuses.
   - **Status changes:** each issue answer carries the status changes the reader may make now, with the button text and what *Ask for* needs. One request, no extra call per issue.
4. **The judge (§3):**
   - One function in `bb-core` that answers *allowed*, *needs these fields* or *not allowed, because …*.
   - Every caller uses it: changing an issue, creating one, *Change many issues* with its preview and apply, and the routes for *Try it*.
   - A hidden field is never asked for; the sentence names no hidden field.
5. **The rules (WF-03, WF-04):**
   - **Who can do it:** roles, groups, *the person assigned*, *the person who reported it*.
   - **Only if …:** conditions on fields.
   - **Must be filled in:** reuses `must_be_asked_for` (ADR-0027 amendment WP-09).
   - **Ask for:** a list of fields.
6. **Who may change a field (M3-F53):** one policy question, "which roles may change this field here", with its route, matrix row and privacy rules. The editor's *Who can do it* uses the same answer.
7. **Changing a workflow (§4):**
   - Drafts under the edit lock (`WORKFLOW_PART` next to `LAYOUT_PART`) and *Publish* with a version.
   - When a status is dropped, *Publish* names where its issues go and moves them as a run of many changes, with history.
   - The permission *Manage workflows*: Admin everywhere, Project lead in their projects, never in an access key.
8. **Starter workflows (§5, WF-06):** *Simple*, *Bug* and *Development* from one definition. On a new organization they go to Task/Goal, Bug and Feature. On an upgraded one, *Simple* stays everywhere.
9. **Per project (M3-F22, F122, F124):** issue types and labels may be limited to projects. A retired type no longer counts as missing when an issue moves.
10. **Check access per issue type (M3-F44):** *Check access* names the level per issue type where they differ.

## Acceptance tests (write first)

- `wf_01_an_m3_database_keeps_every_status_after_the_upgrade` (both databases, from a fixture written by `m3-done`).
- `wf_01_a_project_finds_its_workflow_like_a_layout`.
- `wf_03_*`:
  - `wf_03_who_can_do_it_allows_only_the_named_roles_and_people`;
  - `wf_03_only_if_refuses_with_a_sentence`;
  - `wf_03_a_rule_never_gives_a_right_change_issues_does_not`.
- `wf_04_*`:
  - `wf_04_must_be_filled_in_names_the_missing_field`;
  - `wf_04_ask_for_never_asks_for_a_hidden_field`.
- `wf_06_a_new_organization_has_the_three_starter_workflows`.
- `iss_09_many_changes_skip_an_issue_whose_workflow_does_not_allow_it`, and the preview says so (UX §4.2).
- `wf_02_publishing_without_a_status_moves_its_issues_with_history`.
- SEC-MATRIX rows for every workflow route, and `sec_06` for the field names in refusals.

## Checks

TEST-WORKFLOWS (planned → active): the tests above with the prefixes `wf_01`…`wf_06` exist and the two-database ones run through `each_database`.

## Depends on

WP-01 (ADR-0031), WP-02 (the new names, the free migration number).
