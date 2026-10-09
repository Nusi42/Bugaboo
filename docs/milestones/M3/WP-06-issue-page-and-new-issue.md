# WP-06 — Issue page, issue list and the new-issue form

**Wave 2 · Folder / branch:** `bugaboo.win3` / `WIN3`
**Features:** ISS-01, ISS-11, ISS-17 (browser), UX-04 (focus view, side panel) for issues; UX design §3.1–3.4, §4.2, §4.4, §4.5, §6; ADR-0019; follow-ups M2-F11, M2-F70, M2-F99, M2C-F81, M2C-F100.

## Goal

People open an issue in the focus view or the side panel, change any value in place with *Saving… / Saved*, write the description under an edit lock without ever losing text, and make new issues in a dialog that keeps their draft.

## Owns

`web/app/src/features/issues/**` (new: the issue list of a project, the issue page, the new-issue dialog), the issue page's slots (main column blocks and fields panel) that WP-05, WP-07, WP-08 and WP-09 fill, the issue parts in `design/theme.kdl`, removing `/frames` and `frames.*` (M2-F11), `web/app/src/shared/editing/**` (new: the lock, heartbeat, draft and watch hooks on top of WP-04's API), `userDocs/issues/*` (with WP-03).

## Tasks

1. **Issue list of a project:** a lean, paged list (title, issue ID, type, status, assigned to, priority), sorted by last change; checkboxes for WP-08's bar; `j/k`; the slim list of the focus view comes from it.
2. **Issue page (UX §4.4):** header (type, issue ID that copies a link, title changed in place, *More*, *Open full page*, *Close*); facts bar; description; the slots for fields, sub-issues, related issues and history; *Details* column on wide screens, *All details* on smaller ones; tablet sheet; phone column with the bottom bar *Status · Comment · More*. Focus view by default, side panel by choice, read from *Open issues in* (M2-F70); the focus view fills the width on a computer (M2C-F81).
3. **Fields panel:** empty fields you may change as a quiet "Add due date"; empty fields you can't change left out; read-only fields with a lock and the reason ("Only team members can change Priority."); people as `PersonRef` (a Guest sees *Someone* or the role, M2-F99).
4. **Changing in place:** short values save at once against the version (*Saving… → Saved · 10:42*), with Undo for 10 s; a conflict shows the banner of M3-Q5 (d) built from WP-04's structured answer.
5. **Long text (ISS-17):** the lock on the first keystroke, heartbeat every 15 s, "Ana is editing the description · since 2 min" with *Ask Ana to finish* and *Edit anyway*; *Not saved* states with *Compare and save* (side by side, pick per paragraph or keep yours); the watch every 30 s and on return ("Ben changed the description after you · Show changes").
6. **New issue (ISS-11, UX §4.5):** `+ New` and `c`; project → type → the fields of the new-issue form (WP-09 makes it role-based; until then all fields the person may fill); required fields say "required"; *Create*, *Create and start another*, *Cancel*; the draft in the browser and on the server ("Your draft is kept if you close this window"; "Continue your draft from yesterday"). The dialog uses the shell's one overlay part (M2C-F100: `ConfirmDialog` moves onto it as a variant).
7. **Shortcuts:** `c`, `j/k`, `s` (status), `a` (assign), `e` (edit description), listed in `?`.

## Acceptance tests (write first)

- Playwright (four themes, computer, tablet and phone, axe): open an issue in the focus view and in the side panel; change status and priority in place and see *Saved*; a Guest sees *Someone*; a read-only field shows its reason.
- Playwright, two browsers: Ana types in the description, Ben sees "Ana is editing", Ben uses *Edit anyway*, Ana's text is kept and offered with *Compare and save*.
- Playwright: close the new-issue dialog half-filled, reopen it, *Continue your draft*; the draft is gone after *Create*.
- `ux_04_frames_demo_is_gone` (the route answers *Not found*; no `frames.*` key left).

## Checks

None new. TEST-A11Y, TXT-LITERAL and THEME-ONLY cover the new screens.

## Depends on

WP-02 (Markdown view and editor), WP-03 (issues, routes), WP-04 (versions, locks, drafts, watch).
