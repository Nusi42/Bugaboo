# WP-03 — Issues, issue types, statuses and issue IDs

**Wave 1 · Folder / branch:** `bugaboo.win3` / `WIN3`
**Features:** ISS-01, ISS-01a, ISS-02, SEC-04 (issues), PRIV-13; UX design §2.1, §4.4, §5.1; ADR-0021, ADR-0025, ADR-0027; follow-ups M2-F21, M2-F63, M2-F65, M2-F96.

## Goal

Issues exist: people with the right to make issues create them in a project, see exactly the issues they may see, and change the built-in fields they may change; each issue has a readable issue ID, counting or random, that keeps working after the mode changes.

## Owns

`crates/bb-core/src/issue*.rs` (new: `Issue`, `IssueType`, `Status`, `StatusGroup`, the readable `IssueKey` and its parser), `crates/bb-app/src/issues/**` (new), `crates/bb-store/src/issues.rs` (new) and migration `m0007` (`issue`, `issue_type`, `issue_alias`, `issue_counter`), `crates/bb-api/src/api/issues*` (new), the issues column of SEC-MATRIX, `crates/bb-policy` (issue visibility and the nested-view mask fix), *Organization settings › Issue types* (`web/app/src/features/issue-types/**`, new), the issue-ID mode in *Project settings*, `userDocs/issues/*` (new).

## Tasks

1. **Issue types (ISS-02):** Bug, Task, Feature, Goal made at the first start from one definition; each has a name, an icon (Lucide, from a fixed list), and later its fields and layout (WP-05, WP-09). Add, rename, change the icon, *Retire* (no new issues of it; old ones keep it). Permission *Manage issue types, kinds of relation and layouts*.
2. **Statuses (M3-Q1):** the starting workflow, defined once in `bb-core`; every type uses it until M4; the closed date follows the *Done* group.
3. **Issues (ISS-01):** title, description (Markdown text, rendered by WP-02 when shown), type, status, priority, reported by, assigned to (one or more people), created, updated, closed; `version` on every change; soft delete into the trash with Undo; events for every command (ARCH-EVENTS table). *Make issues* in the project; *Change issues* or *Change own issues*; fields obey the mask; people through `PersonRef`.
4. **Issue IDs (ISS-01a, ADR-0021):** counting per project (the one allowed counter, M2-F63: teach ARCH-NO-COUNTERS this exception by name) or random (Crockford Base32, length 4–8, default 5, from the secure source; drawn again on collision); the organization default (a setting, starts as *Random*) and the project's choice (the first project-level setting: makes *Project settings* show settings, M2-F71); changing the mode gives every issue a new ID and keeps the old ones in `issue_alias`; `/i/WEB-142` and `/api/…/issues/WEB-K7Q2X` accept both and answer the current ID.
5. **Seeing issues:** *See issues* shows the project's issues; without it a person sees only the issues they reported (Guests). A hidden issue answers `resource.not_found` exactly like a missing one. Lists are lean and paged, no totals (PRIV-13).
6. **Masking nested views (M2-F21):** before the first view that holds another view (an issue holds people and, later, sub-issues), make `Masked` mask nested views by construction (a derive) or fail a check; a test proves a hidden field of a nested issue never leaves.
7. **SEC-MATRIX (M2-F65, M2-F96):** the issues column; for each starting role, every route of this package, and `fields=` asking for every field the role may not see.
8. **Screen:** *Organization settings › Issue types* (list → side panel). The issue page itself is WP-06.

## Acceptance tests (write first)

- `iss_01_a_person_makes_and_changes_an_issue_they_may` (both databases), `iss_01_a_hidden_issue_answers_not_found_like_a_missing_one`, `iss_01_a_guest_sees_only_the_issues_they_reported`, `iss_01_a_stale_version_is_refused_with_who_changed_it`.
- `iss_01a_random_ids_use_the_readable_alphabet_and_never_repeat` (property test), `iss_01a_counting_ids_count_per_project_without_gaps_under_ten_writers` (both databases), `iss_01a_an_old_id_redirects_after_the_mode_changes`.
- `iss_02_a_retired_type_keeps_its_issues_and_takes_no_new_ones`.
- `sec_04_a_nested_view_never_carries_a_hidden_field`, `sec_06_issue_answers_refuse_every_field_the_role_may_not_see`.
- `priv_13_no_issue_answer_holds_a_count`.
- Playwright: *Issue types* list and side panel, four themes, axe.

## Checks

**TEST-ISSUE-IDS** (planned → active): the tests above with that name prefix. ARCH-NO-COUNTERS knows counting issue IDs as the one allowed counter.

## Depends on

WP-01 (permissions, words, ADR-0027). WP-02 is not needed: this package stores Markdown text and does not render it.
