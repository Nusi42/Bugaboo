# WP-09 — Pinned navigation

**Wave 2 · Folder / branch:** `bugaboo.win5` / `WIN5` · **Model:** Sonnet (one small table, three routes and a sidebar section, all fixed by this brief and WP-01's frames)
**Features:** CFG-05 (owner, server and screens). Answer M10-Q17; UX design §3.1, §3.3 and WP-01's frames of *Pinned*.

## Goal

A person pins projects, saved searches and dashboards to their sidebar and puts the pins in their own order. The pins stand in a section *Pinned* near the top of the sidebar and at the top of *More* on a phone. A pin to something the person no longer sees is left out without a word.

## Where

- **Owns:** `crates/bb-core/src/pin.rs` (new: the kinds `project`, `saved-search`, `dashboard`; documents come in M11), `crates/bb-app/src/pins/**` (new: pin, unpin, move, list for the person), `crates/bb-store/src/pins.rs` (new), migration **69** (`crates/bb-store/src/migrations/m0069_pins.rs`: `pin` with the person, the kind, the target and the place), `crates/bb-api/src/api/pins.rs` (new: `GET`, `POST`, `DELETE /api/v1/me/pins`, `PUT /api/v1/me/pins/order`), the section in `web/app/src/features/shell/` (`Sidebar.tsx`, `sidebar-sections.ts`, `MoreSheet.tsx`, a new `PinnedSection.tsx`), the entry *Pin to sidebar* / *Unpin* on a project's page, a saved search's menu and a dashboard's menu, the keep entry and the removal part, `userDocs/basics/moving-around.md`.
- **Shared files, one entry each:** `migrations/mod.rs` and `list.rs`, `router.rs`, the keep list, the walk of a person's data, `messages/en.json`, `web/app/src/features/shell/` (WP-13 adds *Dashboards* to the sections).
- **Does not touch:** projects, saved searches, dashboards (their `may_open` rules are called, not changed).

## What is decided

- **The person's own** (M10-Q17): pins are kept with the person; nobody sets pins for others. At most 30 pins, counted inside the change (`limit.reached`).
- **Order:** the person's order, kept as places; *Move up* and *Move down* in each pin's menu, and drag in the section (the drag's typed way is the menu; TEST-A11Y).
- **What shows:** each pin with its icon of kind and its name as the person sees it (a project's name, a search's name, a dashboard's name). The list is read with each target's own rule of who may open it (`may_open` of saved searches and dashboards, the project's sight); a pin whose target the person no longer sees is left out of the answer, without a word and without a count. A pin of a deleted target is deleted with the target.
- **The sidebar** (M5-F150): *Pinned* stands after the main sections and before the project tree; with more than 8 pins it shows 8 and *Show all pinned*, so the sidebar fits a window 600 px high. In the sidebar narrowed to icons, pins show as their icons with the name for screen readers and on pointing. On a phone, *Pinned* is the first group of *More*.
- **Routes** take a session only, never an access key (the person's own data, like saved searches).

## Tasks

1. Write the acceptance tests below.
2. The record and migration 69 on both databases; marks and the person link.
3. The routes; the list with each target's own rule.
4. The section, *Pin to sidebar* on the three pages, the order with drag and menu, the narrow sidebar and the phone; pictures in the four themes on computer (wide and narrow sidebar) and phone, looked at against UX §1.1 and §6.4 before the push.
5. Keep and removal: deleted at *Remove a person*; in *Download my data*.
6. User docs; architecture §7 (*Storage types*) and *Personal data in M10*.

## Acceptance tests (write first)

- `cfg_05_a_person_pins_a_project_a_saved_search_and_a_dashboard` (both databases).
- `cfg_05_pins_keep_the_persons_order_and_move_up_and_down`.
- `cfg_05_a_pin_to_something_the_person_no_longer_sees_is_left_out_without_a_word`.
- `cfg_05_a_pin_of_a_deleted_target_is_deleted_with_it`.
- `cfg_05_at_most_30_pins`.
- `cfg_05_an_access_key_reaches_no_pin_route`.
- `cfg_05_pinned_stands_in_the_sidebar_and_first_in_more_on_a_phone` (browser).
- `cfg_05_more_than_8_pins_fold_and_the_sidebar_fits_a_window_600_px_high` (browser, M5-F150).
- `cfg_05_a_pin_is_moved_by_the_keyboard_as_by_drag` (browser, keyboard only).
- `priv_04_removal_deletes_the_persons_pins`.
- SEC-MATRIX: every pin route, for every starting role and an access key, both databases.
- `expectStill`: pinning from a page moves nothing under the cursor.

## Checks

DATA-MIGRATIONS, DATA-MARKS, ARCH-DB-PARITY, ARCH-API-FREEZE, SEC-MATRIX, TEST-A11Y, THEME-ONLY, STY-PARTS, STY-TOKENS, TXT-LITERAL, TXT-CATALOG, TXT-TERMS, ARCH-WEB-API, ARCH-WEB-FEATURES.

## Docs to update

`userDocs/basics/moving-around.md` (pins); architecture §7 and *Personal data in M10*; ADR-0044 and ADR-0045 amendments; the generated personal-data list.

## Review

Code review with the SSOT/DRY/ODR gate: each target's own `may_open`, never a second rule of who sees what. A UX review of the pictures in the four themes, wide and narrow sidebar and phone.

## Personal data

A pin names its person and what they use (marked, linked to the person). Deleted at *Remove a person*; in *Download my data*.

## Depends on

WP-01 merged; **lands after WP-06** (the kind `dashboard` calls its `may_open`). It may start with projects and saved searches and rebase onto WP-06 for dashboards.
