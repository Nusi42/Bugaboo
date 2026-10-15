# WP-15 — The UX concept into the UX design

**Wave 6 (the look) · Folder / branch:** `bugaboo.win1` / `WIN1` (lead)
**Features:** UX-01, UX-04, THM-01, PLAN-01…05, PLAN-08 (words and pictures only); UX design §1, §3, §4.2, §4.4, §4.5, §4.7, §6, §7, §8.
**Source:** the approved UX concept (proposal 3, a published design canvas, not linked here). It answers the review of the M3 screenshots, which found edges that don't line up, text against borders, dialogs that can't grow, no color per issue state.

## Goal

The UX design (`docs/02-ux-design.html`) says exactly what the concept shows, with every number, so the builders of WP-16 and WP-17 need nothing else. The canvas stays a picture; the UX design is the single source of truth.

## Owns

`docs/02-ux-design.html`, `docs/glossary.md` (words only), `docs/04-milestones.md` (M7 outline and the definition of done), this milestone's README.

## Tasks

1. **§1 UX principles:** the ten rules R1…R10 (one left edge; one spacing scale; no text touches a line; color means state; never color alone; one main action per area; change in place; two text colors, few links; same thing, same place; forms open as windows).
2. **§7 Visual design:** the spacing scale (4, 8, 12, 16, 24, 32, 48) with the padding per container (control 12–14, box 16, dialog and page 24, between fields 20, between sections 28); control height 36 with a mouse and 44 with touch (`pointer: coarse`), list rows 44 / 64; text sizes 24 / 20 / 16 bold / 16 reading / 14 / 13 / 12 and the mono face for IDs and times; the status-group colors and icons (Not started: grey dashed ring; In progress: blue half-filled ring; Done: green check) and the priority icons and colors (Critical red double arrow, bold; High orange arrow, bold; Normal grey; Low light grey) in all four themes; In progress in Paper moves to teal (`#1d6b80`, soft `#dfeef1`, plus a *more contrast* value).
3. **Rows carry their state:** a list row, a sub-issue row, a phone row and a timeline bar are tinted by their status group (In progress blue, Done green, Not started plain) and edged on the left by their priority (4 px, Critical red, High orange). The tint is a mix of the group's soft color and the raised surface; hover and selected stay distinguishable (STY-CONTRAST `distinct`).
4. **§4.2 Issue list:** one grid with fixed columns (tick, status icon, ID, type icon, title, labels, priority, assigned person, changed), left-aligned, groups by status group with colored headers, the legend, the bar for ticked issues.
5. **§4.4 Issue page:** menu narrowed to icons, slim list, text column at most 780, the big status button in its group color, Details as name-and-value rows that save in place with *Saved* and *Undo*; descriptions and comments keep *Edit* and *Save* (edit lock).
6. **§4.5 New issue and §6 (windows):** a form opens as a window: *Minimize* (the draft waits as a bar at the bottom), *Maximize* / *Restore down* (maximized fills the whole page, menu included), *Close* (the draft is kept), in the Windows order at the top right; resize at the edges and the corner, smallest 480 × 360, start width 760; the form rearranges itself to the window's own width (one column below 880, fields at the side from 880, text centered from 1200); title and buttons stay while the form scrolls; the size is remembered per person and form. Esc and the keyboard behave as in a dialog (focus stays inside while it is not minimized). A question stays a small fixed dialog (440). On a phone a window is a full screen.
7. **§4.7 Timeline:** replace the sketch with the concept: rows by goal (foldable), goal bracket, release diamond with its date, bars in the status colors with priority edges and the done part darker, dependency arrows, *Deadline drivers* outlining the chain in the accent color, today as an accent line with a label (red means Critical now), week scale with shaded weekends, a selected bar with handles, a dot to drag a dependency and a small card with *Set dates…* and *Add dependency…*, the *Not scheduled yet* tray, the legend, Undo after every drag; on the phone a list by goal with a small span on a seven-week scale.
8. **§3 Layout and §8:** the computer layout map (menu 64 or 240, list 320, text at most 780, Details 300–340) and that color never stands alone.
9. **Glossary:** *Minimize*, *Maximize*, *Restore down* (window buttons, common words), *Deadline drivers*, *Not scheduled yet* if missing.
10. **04-milestones.md:** M7's timeline outline points to UX §4.7; the definition of done gains "screens follow the ten rules of UX §1; the reviewer checks alignment, padding and state colors on every screenshot".

## Checks

The Git hooks for docs (FMT-MD, LINT-SPELL, SEC-PRIVATE, META-RULE-TRACE for any new *must*). No link to the canvas in the repository.

## Depends on

The maintainer's approval of the concept.
