# WP-09 — Layout editor and role-based forms

**Wave 3 · Folder / branch:** `bugaboo.win3` / `WIN3`
**Features:** CFG-01 (issue page and new-issue form), ISS-02 (each type its layout); UX design §4.5, §5.5, §3.3; ADR-0027, ADR-0019 (the editor locks).

## Goal

A project lead decides, per project and issue type, which fields the issue page and the new-issue form show, in which sections and tabs, and for which roles, so a Guest gets a short form and a Team member the full one; field permissions always win.

## Owns

`crates/bb-app/src/layouts/**` (new), `crates/bb-store/src/layouts.rs` (new) and migration `m0012` (`layout`, `layout_section`, `layout_field`), `crates/bb-api/src/api/layouts*` (new), `web/app/src/features/layouts/**` (new: the editor, *Customize this form* from the new-issue dialog), the layout lookup the issue page and new-issue dialog call (WP-06's files call one hook that this package owns), `userDocs/layouts/*` (new).

## Tasks

1. **Layouts (CFG-01):** for the issue page and the new-issue form; per project and issue type, inherited down the tree and from the organization in ADR-0027's order; a starting layout per issue type made once.
2. **Editor:** sections and tabs; drag fields in, with buttons for everything a drag does (*Move up*, *Move to …*); per field a role checklist (*Team member ✓, Guest ✗*); phones get the list form of the editor. The editor takes an edit lock (WP-04) while open.
3. **Role-based forms:** the new-issue form shows a field only if the layout lists it for one of the person's roles *and* the field level lets them fill it; *Add a field* on an issue lists only fields the person may fill.
4. **Field permissions win:** a layout can never show a field the role may not see or let a role change a field it may only see; the editor marks such rows ("Guest can't see Cost center: Field permissions").

## Acceptance tests (write first)

- `cfg_01_a_guest_gets_the_short_form_and_a_team_member_the_full_one`, `cfg_01_a_layout_never_shows_a_field_the_role_cannot_see` (property test over layouts × levels), `cfg_01_a_sub_project_uses_its_parents_layout_until_it_has_its_own` (both databases).
- `iss_17_two_people_cannot_change_one_layout_at_once`.
- Playwright: open *Customize this form*, move *Due date* into a section, untick *Guest*, see the Guest's short form; four themes, phone and computer, axe; every drag also done by keyboard.

## Checks

None new.

## Depends on

WP-03 (issue types), WP-04 (locks), WP-05 (own fields), WP-06 (issue page and dialog).
