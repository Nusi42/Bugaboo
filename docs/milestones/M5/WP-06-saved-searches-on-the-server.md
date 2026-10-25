# WP-06 — Saved searches on the server: stored, shared, a stable address

**Wave 2 · Folder / branch:** `bugaboo.win4` / `WIN4`
**Features:** QRY-06, CFG-02 (stored). ADR-0034 (all of it), ADR-0005 amendment M5 (the canonical text), ADR-0007, ADR-0008 (additions), ADR-0015 (personal-data marks).

## Goal

A person saves a search with how it is shown, shares it with groups or projects they may share with, and every receiver sees it only when it checks cleanly against their own rights, and then sees only what they may see.

## Owns

Saved searches in `bb-core` (the type), `bb-app` (use cases and port), `bb-store` (tables, migration), `bb-api` (routes); the permission *Share searches* in `bb-policy` and the generated tables of architecture §7 (`cargo xtask gen roles`); the project's default list as a project setting.

## Tasks

1. **The record** (ADR-0034, decision 1): name, canonical text, how it is shown (kind of view; columns with order and widths; *Spacing*; grouping), the maker, the shares. Columns name fields by their search word. A stable ID and the address `/s/<id>`.
2. **The permission** *Share searches* (counts per project; every starting role but Guest). Generated tables updated.
3. **Use cases:** save, change, share and stop sharing, delete into the trash with *Undo*, *Save a copy*, list mine and shared with me, open one. Every change is an event (ARCH-EVENTS).
4. **The receiver's check** (decision 4): listing and opening run the checker of WP-02 with the receiver's field catalogue (from WP-04, or its first version if WP-04 is not merged: agree with the lead) and *Who sees people*; a search that does not check cleanly is left out, and its address answers `not found`, the same as one that does not exist.
5. **Who may change what** (decisions 2 and 5): sharing needs *Share searches* at the place; a group needs being in it, or *Manage groups*; only the maker changes, shares or deletes; *Change organization settings* may delete any.
6. **The project's default list** (decision 6): a person with *Change project settings* chooses a saved search shared with the project; without one, the built-in default of UX §4.2.
7. **When a person is removed** (decision 8): private searches go with them; shared ones stay, made by *Former user …*. Personal-data marks on the name and the text.

## Acceptance tests (write first)

- `qry_06_a_shared_search_runs_with_the_rights_of_the_person_who_opens_it`.
- `qry_06_a_shared_search_with_a_field_the_receiver_cannot_read_is_not_listed_and_its_address_is_not_found`, and the same for a hidden person, group and project in its text.
- `qry_06_only_the_maker_changes_a_saved_search`, `qry_06_sharing_needs_share_searches_at_the_place`, `qry_06_a_group_needs_being_in_it_or_manage_groups`.
- `qry_06_save_a_copy_is_private_and_mine`, `qry_06_delete_goes_to_the_trash_with_undo`.
- SEC-MATRIX: every saved-search route; PERF-N1: the list of saved searches (forty as five).
- DATA-MARKS: the new columns are marked.

## Depends on

WP-02 merged. Runs beside WP-04 and WP-05.
