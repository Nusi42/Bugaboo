# WP-06 — Dashboards on the server

**Wave 2 · Folder / branch:** `bugaboo.win3` / `WIN3` · **Model:** Opus (sharing by rules, a box left out without a trace, a new permission, people's data in boxes)
**Features:** CFG-03 on the server (the screens are WP-13's). ADR-0050 (WP-01), the pattern of ADR-0034; answers M10-Q12, Q13, Q14, Q20.

## Goal

A person makes dashboards of boxes, places and sizes them on a grid, and shares a dashboard with groups and projects. Each box reads a search (its own text or a saved search) and answers as the viewer: results, totals, a small timeline, the person's timer, or recent changes. A box whose search does not check for the viewer is left out and the grid closes up. The ready-made boxes are data the screen offers, not code for each box.

## Where

- **Owns:** `crates/bb-core/src/dashboard.rs` (new: the dashboard, the box, the kinds of box, the grid's rules), `crates/bb-app/src/dashboards/**` (new: make, change, place, share, *Save a copy*, delete to the trash, read for a viewer), `crates/bb-store/src/dashboards.rs` (new), migration **67** (`crates/bb-store/src/migrations/m0067_dashboards.rs`: `dashboard`, `dashboard_box`, `dashboard_share`), `crates/bb-api/src/api/dashboards.rs` (new), the permission `share-dashboards` in `crates/bb-policy/src/catalogue.rs`, the ready-made boxes as one list in `crates/bb-core/src/dashboard.rs` (`READY_MADE_BOXES`), the person links and keep entries (`KEEP_KINDS`), *Remove a person* and *Switch off* (the dashboard part only).
- **Shared files, one entry each:** `migrations/mod.rs` and `list.rs`, `router.rs`, `catalogue.rs`, the keep list, the walk of a person's data, `messages/en.json`.
- **Does not touch:** totals (WP-03: called), the timeline's plan (called), the timer (called), any screen.

## What is decided

- **A dashboard** (ADR-0050): a name, its boxes, its maker and its shares; an address `/d/<id>`. Private when made. Sharing as a saved search (ADR-0034, decisions 2 to 5): with a project, *Share dashboards* in that project; with a group, *Share dashboards* for the organization and being in that group, or *Manage groups*. The new permission starts with the roles of *Share searches* (M10-Q12). Only the maker changes, shares or deletes it; everyone who sees it may *Save a copy*.
- **A box:** a kind, a place on a grid of 12 columns (column, row, width, height; at least 3 × 2), a title, and its source: a search text (canonical, ADR-0005) or a saved search, plus the options of its kind. Kinds (M10-Q13): *Search results* (the first 10 issues by the search's sort), *Numbers and chart* (the totals of WP-03, by one field, as bars or one number), *Time totals* (estimated, left, spent of the search; or *My time this week*, the person's own entries), *Small timeline* (the timeline route of the search, one page, read-only), *My timer* (the viewer's own timer), *Recent changes* (the newest events of issues the search finds, as the history writes them, people by *Who sees people*). At most 20 boxes on a dashboard and 50 dashboards per person.
- **Each box runs as the viewer** (ADR-0034, decision 3). A box whose text does not check for the viewer, or whose saved search the viewer may not open, is left out of the answer, and the answer's places are packed upwards for that viewer, so no gap, no title and no count tells of it. The dashboard itself checks like a saved search: a viewer it does not reach gets `resource.not-found`.
- **No box ranks people** (M10-Q13, PRIV-09, COMP-10): a chart by a person field is in the order of names; there is no option to sort it by size and no "top" kind. *Recent changes* is about issues, never "the work of one person": it takes no person filter of its own (the search may name a person, as any search may).
- **Ready-made boxes** are entries of `READY_MADE_BOXES` (kind, title key, search text, options): "My open issues", "Due soon", "Open bugs by priority", "Issues by status group", "Progress of a release", "My time this week", "Time on this search", "Small timeline of a search", "My timer", "Recent changes in this project". *Add box* offers them with a preview made by the same reads.
- ***Home*** (M10-Q14, if taken): each person has one dashboard of their own, not shareable, made from the blocks of today's *Home* at first use; *Customize* on *Home* changes it. If Q14 is not taken or is cut (M10-Q21), this bullet goes and *Home* stays as it is.
- **Changes are events** (ADR-0007): made, changed, shared, no longer shared, deleted (to the trash, *Undo*). Changing a dashboard has a version check; a stale save is refused with the newer version.
- **AI apps and agents** (M10-Q20): no tool reaches dashboards; the routes take a session only, never an access key (like saved searches, ADR-0034 amendment M5 WP-06, item 3).

## Tasks

1. Write the acceptance tests below.
2. The record, the box kinds, the grid's rules; migration 67 on both databases; marks and person links.
3. Sharing and the permission; the read for a viewer with boxes left out and packed.
4. Each box's read through the routes that exist (search, WP-03's totals, the timeline, the timer, the history); the ready-made list.
5. *Home* as the person's dashboard (if M10-Q14 takes it).
6. Keep rules and removal: private dashboards deleted at *Remove a person*, shared ones kept as made by *Former user*; *Change organization settings* may delete any.
7. Architecture §7 (*Storage types*), §12; ADR-0050 *Enforced by* lines; ADR-0025 amendment (*Share dashboards*).

## Acceptance tests (write first)

- `cfg_03_a_dashboard_is_private_when_made_and_shared_like_a_saved_search` (both databases).
- `cfg_03_share_dashboards_is_needed_to_share_and_starts_with_the_roles_of_share_searches`.
- `cfg_03_each_box_runs_as_the_viewer`.
- `cfg_03_a_box_that_does_not_check_for_the_viewer_is_left_out_without_a_gap_or_a_title`.
- `cfg_03_a_viewer_the_dashboard_does_not_reach_gets_not_found`.
- `cfg_03_only_the_maker_changes_shares_or_deletes_and_others_save_a_copy`.
- `cfg_03_a_chart_by_a_person_is_in_the_order_of_names_and_cannot_be_sorted_by_size`.
- `cfg_03_recent_changes_show_people_as_who_sees_people_says`.
- `cfg_03_every_ready_made_box_answers_with_the_reads_that_exist` (one test per entry of the list, generated from it).
- `cfg_03_a_stale_change_of_a_dashboard_is_refused_with_the_newer_version`.
- `cfg_03_limits_of_boxes_and_dashboards_are_counted_inside_the_change`.
- `cfg_03_home_is_the_persons_own_dashboard_made_from_todays_blocks` (if M10-Q14 takes it).
- `priv_04_removal_deletes_private_dashboards_and_keeps_shared_ones_as_former_user`.
- `cfg_03_an_access_key_reaches_no_dashboard_route`.
- SEC-MATRIX: every dashboard route, for every starting role and an access key, both databases.
- PERF-N1: a dashboard of ten boxes makes a fixed number of statements per box.

## Checks

DATA-MIGRATIONS, DATA-MARKS (grows by entries), ARCH-DB-PARITY, ARCH-EVENTS, ARCH-VERSIONED, ARCH-PERMS, ARCH-PERSON-REF, ARCH-NO-COUNTERS, ARCH-API-FREEZE, SEC-MATRIX, PERF-N1, TXT-TERMS.

## Docs to update

Architecture §7, §12 and *Personal data in M10*; ADR-0050; ADR-0025, ADR-0044 and ADR-0045 amendments; the generated personal-data list. The user docs come with the screens (WP-13).

## Review

Code review with the SSOT/DRY/ODR gate: sharing reuses the code of saved searches (one `may_open` rule for both, not a copy), each box calls an existing read, the ready-made boxes are one list. Security review: a viewer with fewer rights reads every answer; a box left out leaves no trace. Privacy review: no box ranks people; removal.

## Personal data

A dashboard is content written by a person (its name and box titles, marked so) and names its maker. *My time this week* shows only the viewer's own time entries. Boxes may show others' work (assignments, changes) only as far as the viewer may see them (*Who sees people*). Kept until deleted; private ones go at *Remove a person*, shared ones stay as made by *Former user*.

## Depends on

WP-03 merged (totals). WP-09 lands after this package (pins of dashboards).
