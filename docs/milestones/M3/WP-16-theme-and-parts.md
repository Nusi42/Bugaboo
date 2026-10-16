# WP-16 — The new look: theme tokens and shared parts

**Wave 7 (the look) · Folder / branch:** `bugaboo.win2` / `WIN2`
**Features:** THM-01, THM-06, UX-01, UX-02 (target sizes), UX-04; UX design §1, §6, §7 as written by WP-15; ADR-0010, ADR-0026.

## Goal

Every value of the approved UX concept exists once, in `design/theme.kdl`, and every shared part of `web/packages/ui` draws from it. After this package, a screen gets the new look by using the parts; no screen sets a size or a color of its own.

## Owns

`design/theme.kdl`, the generated `web/packages/ui/src/theme.css` and `docs/design/*.css` (`cargo xtask gen theme`), `web/packages/ui/**`, `docs/design/` (the design pages' styles), the pictures in `docs/02-ux-design.html` (task 9). Screens in `web/app/src/features/**` belong to WP-17; this package changes a screen only where a part's props changed, and only that line.

## Tasks

1. **Tests first:** part tests (`parts*.test.ts(x)`) for every new part and state below; STY-CONTRAST rows for every new color pair in all four themes and their *more contrast* variants; a test that no control is below 24 px and no touch control below 44 px.
2. **Tokens:** the spacing scale and the padding per container (UX §7); control height 36 with a mouse and 44 with `pointer: coarse` (the old single `size-target` of 44 becomes the touch value; WCAG 2.2 AA asks for 24); list row 44 / 64; question dialog 440 (`size-dialog` changes from 560); window start width 760, smallest 480 × 360; the timeline day width (16 at the week zoom). Paper's In progress teal (UX §7) with its soft and *more contrast* values. Row tints for In progress and Done; the priority colors come from *danger* and *warning* (no new hues).
3. **Status and priority parts:** status icon (dashed ring, half-filled ring, check), status pill and the big status button in the group color; priority icon with and without its word. Color never alone: the icon and the word (or its accessible name) are always there.
4. **Row state:** one part state for "status group" and one for "priority" that any row can carry (list row, sub-issue row, phone row, timeline bar): tint by group, 4 px left edge for Critical and High, hover and selected still distinct (`distinct` rule).
5. **List row grid:** the fixed columns of UX §4.2 as one part, with the column header and the status-group header.
6. **Name-and-value row** for Details (label 112 on a computer, 120 on a phone), with the *Saved* and *Undo* state.
7. **Window:** a new `Window` part in `web/packages/ui` built on the existing `Overlay` (one overlay, M2C-F100): title bar with *Minimize*, *Maximize* / *Restore down*, *Close* (Windows order, 46 × the title bar's height, Close turns red on hover); resize at the edges and the corner with the pointer and with the keyboard (a menu item or keys, UX-04); maximized fills the page; minimized becomes a bar at the bottom; layout by the window's own width (container queries: one column below 880, side fields from 880, centered text from 1200); size remembered per person and form through the settings registry (ADR-0018), not browser storage alone. Focus stays inside while open; Esc closes (draft kept); on a phone it is a full screen without resize. `ConfirmDialog` stays the small fixed dialog.
8. **Switch** and **legend** parts as in the concept (the switch is a real `role="switch"`).
9. **Design pages:** `docs/design/*.css` regenerated, so the UX design and the mockup show the new parts. Redraw the older pictures of UX §3.1, §3.3, §4.2, §4.4, §4.5, §4.7 and §7.2 with the new parts (status icons instead of status pills, row tints and edges, name-and-value Details, the window, the timeline as written), then remove the note *"The look, approved"* at the top of the UX design. `docs/02-ux-design.html` pictures belong to this package; its text stays as WP-15 wrote it.

## Checks

THEME-ONLY, LINT-CSS, STY-CONTRAST, TEST-A11Y, DOC-GENERATED green; no `style=` in web code. SSOT/DRY/ODR: one place for each size and color; the old 44-only rule removed, not kept beside the new one.

## Depends on

WP-15 merged (the numbers come from UX §1, §6, §7).
