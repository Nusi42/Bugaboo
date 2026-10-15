# WP-17 — The M3 screens in the new look, and three visible bugs

**Wave 8 (the look) · Folder / branch:** `bugaboo.win3` / `WIN3`
**Features:** ISS-01, ISS-03, ISS-04, ISS-09, ISS-11, CF-08, UX-01, UX-04; UX design §3, §4.2, §4.4, §4.5 as written by WP-15.
**Source:** the maintainer's review of the M3 screenshots and the three bugs listed on the M3 review page.

## Goal

Every M3 screen follows the ten rules of UX §1 in four themes, on a computer, a tablet and a phone, using only the parts of WP-16.

## Owns

`web/app/src/features/{issues,history,sub-issues,related-issues,fields,labels,bulk,layouts}/**`, the app shell's layout files, `web/app/e2e/**` for these screens, the screenshot specs.

## Tasks

1. **Tests first:** e2e for the window (minimize and back, maximize fills the page, restore keeps the size, resize with the keyboard, the size is remembered after a reload, Esc keeps the draft); the list row shows the status group and priority by icon and accessible name, not color alone; the phone header of an issue no longer wraps; one test per bug below.
2. **Issue list** (UX §4.2): the row grid, left-aligned (the side list is no longer centered), groups by status group, row tint and priority edge, legend, bar for ticked issues.
3. **Issue page** (UX §4.4): text column at most 780, the big status button in its group color, Details as name-and-value rows saving in place with *Saved* and *Undo*; sub-issue and related-issue rows carry the row state.
4. **New-issue form** (UX §4.5): opens in the `Window`; the buttons stay put while the layout's fields load (they hold their space; follow-up from WP-13).
5. **Phone** (UX §3.3): top bar with *Back* and previous/next issue, so the header does not wrap; rows and Details as in the concept.
6. **The three bugs from the M3 review page:** after *Edit anyway* by someone else, the lost-lock note says who took over, not that the time ran out (`web/app/src/shared/editing/edit-lock-session.ts`, `ranOut`); the history says "Assigned to: nobody" instead of "set Assigned to to nobody"; the phone issue header (task 5).
7. **Screenshots:** every changed screen in four themes on a computer and a phone (`BB_SCREENSHOT_DIR`); the lead checks each for alignment, padding and state colors before the review page.

## Checks

THEME-ONLY, TXT-LITERAL, TXT-TERMS, TEST-A11Y, TEST-EDIT-LOCKS, TEST-CONCURRENT green; `cargo xtask ci` green on both databases.

## Depends on

WP-16 merged.
