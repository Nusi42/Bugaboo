# WP-08 — Projects, sub-projects, short names, moving with a preview

**Wave 3 · Folder / branch:** `bugaboo.win3` / `WIN3`
**Features:** PRJ-01, PRJ-03, PRJ-02 (the tree that permissions and settings follow), PRIV-13 (no counts in the tree); UX design §2, §2.1, §3.1, §5.1; ADR-0021, ADR-0025.

## Goal

People see the projects they may see, as a tree in the sidebar and in *Organization settings › Projects*; project leads make sub-projects, rename them and move them, and before a move takes effect they see in plain words who gains or loses access.

## Owns

`crates/bb-app/src/projects/**` (new; WP-05's *make a project* use case moves here), `crates/bb-api/src/api/projects*` (new), `crates/bb-store/src/projects.rs` (new) and migration `m0005`, `web/app/src/features/projects/**` (new), the project tree slot of the sidebar and the projects source of quick search (WP-03's extension points), `userDocs/projects/*` (new).

## Tasks

1. **Projects (PRJ-01):** make a project at the top level (permission *make projects* on the whole organization) or a sub-project (*make sub-projects* in the parent), nested to any depth; name and short name (rules of M2-Q4, one definition in `bb-core`'s `ShortName`); rename; change the short name if M2-Q4 allows it, with old addresses redirecting (`project_alias`). Version checks on every change; events and security-log entries for moves and short-name changes.
2. **Seeing projects:** a list and a tree of exactly the projects the person may see, lean and paged; a project you may not see answers `resource.not_found`, exactly like one that does not exist (UX §2.1); a visible project under a hidden parent appears at the top of the tree, and neither its address nor its *where you are* line names the hidden parent. No counts of sub-projects or of anything else that could include hidden things (PRIV-13).
3. **Moving (PRJ-03):** a preview that evaluates the policy for the tree as it would be after the move and lists, in plain sentences, which people and groups gain or lose access or change role (*Show, then do*, principle 6), and fields or people information whose visibility changes; confirming with the preview's version applies it, and a tree that changed in between asks to look again. A project cannot move below itself. Moving needs the right in the old and the new parent.
4. **Screens:** the project tree in the sidebar (expand and collapse, keyboard, remembered per browser); *Organization settings › Projects* as a list → editor; the project overview `/p/WEB` (name, short name, sub-projects, *where you are*; an empty state with one next step); *Project settings* `/p/WEB/settings` for project leads with name, short name and *Move*; the move dialog with the preview. *Who has which role*, field permissions and *Who sees people* on the project come in WP-11 and WP-12.
5. **Quick search and start page:** add projects as a source to WP-03's page sources, so `Ctrl+K` finds them and *My settings › Start page* (WP-07) offers them.

## Acceptance tests (write first)

- `prj_01_projects_nest_to_any_depth_and_short_names_are_unique` (both databases), `prj_01_a_short_name_breaks_no_rule_of_m2_q4`, `prj_01_an_old_short_name_redirects` (if M2-Q4 allows changes).
- `prj_01_a_hidden_project_answers_not_found_like_a_missing_one` and `prj_01_a_visible_sub_project_of_a_hidden_project_does_not_name_it`.
- `prj_03_the_move_preview_names_who_gains_and_loses_access` (property test: the preview equals the difference of evaluations before and after), `prj_03_a_project_cannot_move_below_itself`, `prj_03_a_move_after_the_tree_changed_asks_again`.
- `priv_13_no_project_answer_holds_a_count`.
- Playwright: make a project and a sub-project, see them in the sidebar on computer and in *More* on the phone, move a sub-project and read the preview, open a hidden project's address and see *Not found*; four themes, axe.

## Checks

None new. ARCH-LEAN and ARCH-EVENTS cover the new routes; each route declares its permission (WP-05's mechanism), and WP-10, which merges last in this wave, adds their SEC-MATRIX examples.

## Depends on

WP-03 (sidebar slot, page sources), WP-05 (project table, policy, permissions), WP-02 (lean answers).
