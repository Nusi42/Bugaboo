# WP-09 — The issue page after the new look (M3 follow-ups)

**Wave 3 · Folder / branch:** `bugaboo.win3` / `WIN3`
**Features:** ISS-01, TAG-04 (labels and tags on the issue page), CFG-01 (the layout decides), ISS-17 (editing); UX design §1.1, §4.4, §4.5; ADR-0019, ADR-0027; follow-ups M3-F102, M3-F105, M3-F107, M3-F173, M3-F176, M3-F179, M3-F180, M3-F184…M3-F189.

## Goal

The issue page follows the approved UX concept everywhere, not only in the parts M3 redrew. Editing never makes a person wait to learn why a save was refused.

## Owns

- *Details* and the side list of the issue page (`features/issues/**`, except the status button and *Ask for*, which belong to WP-07).
- `features/labels` on the issue page.
- The window parts in `packages/ui` (footer note).
- `shared/editing/**` (the lock release and the refusal).
- The history lines of empty values (`bb-app/src/history`).
- The `edit.taken_over` answer in `bb-api`.
- Migration `m0019` (`layout_off_fields`): the table of the fields that were taken off a layout (M3-F107). The roles of such a field stay in `layout_field_role`, like those of a field on the form. `m0018` belongs to WP-11; its checksum is pinned by the lead after review.

## Tasks

1. ***Labels and tags* as name-and-value rows** in *Details* (M3-F179), and one `<dl>` for all rows, own fields included; `SaveForm.tsx`'s section gets its name (M3-F188).
2. **The priority edge on sub-issue and related rows** (M3-F173): `IssueRef` carries the priority.
3. **The facts bar follows the layout** (M3-F102).
4. **A field put back on a layout keeps its roles** (M3-F107).
5. **A closed window lets its edit lock go at once** (M3-F105): the release request uses `keepalive`.
6. **A refused save names who took over at once** (M3-F176, F180): the server answers `edit.taken_over` with the person (a `PersonRef`), with no extra heartbeat.
7. **The history leaves out empty values set at creation** (M3-F185): no *"set Assigned to: nobody"*.
8. **The window's note under the buttons** (M3-F186), as UX §4.5 says.
9. **The smaller points of M3:**
   - the phone's *Details* button words and *"Add Assigned to"* (M3-F184);
   - one way to hide a label (M3-F187);
   - the sub-issues form and the small visual points (M3-F189), including *Stop following* hung back and the gap under *Project*.
10. **Screenshots** in four themes, computer and phone, before and after. The builder checks them against UX §1.1 before pushing.

## Acceptance tests (write first)

- `iss_17_a_refused_save_names_who_took_over` (both databases).
- `iss_06_history_leaves_out_empty_values_at_creation`.
- `cfg_01_a_field_put_back_keeps_its_roles`.
- Playwright:
  - closing a window frees the lock at once, so a second person can edit within a second;
  - *Labels and tags* rows change in place with *Saved* and *Undo*;
  - four themes, computer and phone, axe.

## Checks

None new.

## Depends on

WP-05 (its *Files* block is on the same page; this package starts after it is merged). It works beside WP-07 through `features/issues/slots.ts`.
