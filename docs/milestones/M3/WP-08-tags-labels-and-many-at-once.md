# WP-08 — Tags, labels, and changing many issues at once

**Wave 3 · Folder / branch:** `bugaboo.win2` / `WIN2`
**Features:** TAG-01, TAG-02, TAG-03, TAG-04 (API, permissions, history), ISS-09, ISS-04 (relations for many issues); UX design §4.2, §5.1, §5.4, principle 6; ADR-0017, ADR-0021 (moves); follow-up M2C-F80.

## Goal

People sort issues with free tags and the organization's labels, and change many issues at once after a preview that says, per issue, what will happen and what will be skipped and why.

## Owns

`crates/bb-app/src/{tags,bulk}/**` (new), `crates/bb-store/src/tags.rs` (new) and migration `m0011` (`tag`, `label`, `label_group`, `issue_tag`, `issue_label`), `crates/bb-api/src/api/{tags,labels,bulk}*` (new), *Organization settings › Labels & tags* (`web/app/src/features/labels/**`, new), the labels and tags fields of the issue page (WP-06's slot), the bar of the issue list (WP-06's checkboxes), the long-preview component (M2C-F80) and its use in the project move preview, `userDocs/labels/*` (new).

## Tasks

1. **Tags (TAG-01, M3-Q8):** anybody who may change an issue adds a tag; autocomplete from tags used in projects the person can see; each tag has a color token and can be limited to some projects.
2. **Labels (TAG-02):** a list kept by people with *Manage labels and tags*, optionally in groups; a *Pick one* group allows one label of the group per issue.
3. **Rename and combine (TAG-03):** for tags and labels, across all issues, as one change with a preview; recorded in the history of every issue it touches.
4. **Changing many issues (ISS-09):** select in the list, then *Change fields · Change status · Move to project · Add label · Add relation · Delete*; a preview from one evaluation per issue (permission, field level, workflow, M3-Q4 for moves) that lists what will change and what is skipped and why; confirm with the preview's versions; issues that changed in between are named and left alone. More than 1,000 issues run as a background job (ADR-0017) with progress and a note when done. One event per issue.
5. **Long previews (M2C-F80):** one component and one answer shape for long previews: a summary first ("118 will change, 2 will be skipped"), grouped by issue or by person, "Show all"; the project move preview of M2 uses it too.

## Acceptance tests (write first)

- `tag_01_autocomplete_never_offers_a_tag_only_used_in_a_hidden_project`, `tag_02_a_pick_one_group_keeps_one_label_per_issue`, `tag_03_combining_two_labels_changes_every_issue_once_and_records_it` (both databases).
- `iss_09_the_preview_equals_what_happens` (property test: preview, then apply, compare per issue), `iss_09_an_issue_you_cannot_change_is_skipped_with_its_reason`, `iss_09_an_issue_changed_after_the_preview_is_left_alone_and_named`, `iss_09_a_move_skips_issues_whose_type_the_target_lacks`, `iss_09_more_than_a_thousand_run_as_a_job`.
- Playwright: select issues, add a label, read the preview, apply, undo; four themes, phone and computer, axe.

## Checks

None new.

## Depends on

WP-03 (issues, IDs, moves), WP-05 (fields for *Change fields*), WP-06 (list and slots), WP-07 (relations).
