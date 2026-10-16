# WP-07 — The workflow editor and status changes on screen

**Wave 3 · Folder / branch:** `bugaboo.win2` / `WIN2`
**Features:** WF-02, WF-04 (screens), ISS-09 (the many-issues bar follows the workflow); UX design §4.2, §4.4 (status button), §5.4, §7.2; ADR-0031, ADR-0019; follow-ups M3-F175, M3-F181, M3-F172.

## Goal

A Project lead changes a workflow in plain words and tries it before it counts. Everybody changes a status by its words (*Start working*), and is asked only for what the change needs.

## Owns

- `web/app/src/features/workflows/**` (new; lazy-loaded): *Settings › Work › Workflows*, the editor.
- The status button and its menu (`features/issues/StatusChange.tsx`, `packages/ui/src/StatusMenu.tsx`).
- The *Ask for* window.
- The status part of the many-issues bar.
- The workflow parts in `design/theme.kdl`.
- `userDocs/workflows/*` (with WP-04).

## Tasks

1. **The editor (UX §5.4):**
   - **Plain list:** each status with its group, its allowed next statuses and their button texts.
   - **Rules:** clicking a status change opens its rules in plain words: *Who can do it*, *Only if …*, *Must be filled in*, *Ask for*.
   - **Draft and *Publish*:** changes are a draft under the edit lock; *Publish* asks where the issues of a dropped status go.
   - ***Try it***: pick an issue and a person and see which changes they would get, and why.
   - **Drawing view:** a computer also gets statuses as boxes and changes as arrows. A phone shows only the list.
   - **Keyboard:** everything works by keyboard.
2. **The status button (UX §4.4, M3-F175, F181):** it lists the status changes the person may make now, in their words, from the issue answer of WP-04. The current status keeps its group's color and icon.
3. ***Ask for***: a small window with only the fields the change needs. *Must be filled in* names the field at its box. The status changes after *Save*, with *Undo*.
4. **Many issues (UX §4.2):** the bar offers status changes by their words. The preview says which issues will be skipped and why.
5. **Resize keys (M3-F172):** decide with a screen-reader test (NVDA on Windows at least) whether Ctrl+Alt+arrows clash. If they do, move them to a resize mode from the title bar's *Window size* button.
6. **Screenshots** in four themes, computer and phone. The builder checks them against UX §1.1 before pushing.

## Acceptance tests (write first)

- Playwright:
  - add a status *Testing* to *Simple*, a change *Send to testing* with *Must be filled in: Found in version*, *Publish*;
  - on an issue, *Send to testing* opens *Ask for* with only that field;
  - *Try it* names why a Contributor gets no *Mark as done*;
  - the phone shows the plain list;
  - four themes, axe.
- Playwright: a person whose role may not make a change does not see it in the status button.
- Playwright: ticking three issues and *Send to testing* skips the one whose workflow does not allow it, in the preview's words.

## Checks

None new. TEST-A11Y, TXT-LITERAL and THEME-ONLY cover the new screens.

## Depends on

WP-04 (the workflow routes and the status changes in the issue answer).
