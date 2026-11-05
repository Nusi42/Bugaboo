# WP-13 — The time fields in layouts and an estimate when an issue is made

**Wave 4 · Folder / branch:** `bugaboo.win3` / `WIN3`
**Features:** TIME-03 (on screen), CFG-01 (the time fields in the layout editor). Follow-ups M6-F60 and M6-F61.

## Goal

*Estimated time*, *Time left* and *Time spent* are fields like any other on the issue page: the layout editor places them, and a new issue can be given an estimate in the form that makes it.

## Owns

The time fields in the field list of the layout editor and in the default layouts, the estimate in the form of a new issue, and their tests.

## Tasks

1. **Layouts (M6-F60):** the layout editor lists *Estimated time*, *Time left* and *Time spent* among the fields it can place, on the issue page and on the board card; *Time spent* is shown, never typed, because it is the sum of the time entries. The default layouts place them where the time card of WP-07 shows them today, so an organization that changes nothing sees no change. Field permissions always win (CFG-01): a person who may not read a field never gets it, whatever the layout says.
2. **Estimate when an issue is made (M6-F61):** the form of a new issue takes *Estimated time* when the layout of the new-issue form places it, through the one reader of lengths of time (`1:30`, `1.5`, `90m`, WP-03). *Time left* starts equal to the estimate, as when the estimate is set in the issue details. The server checks the right to change the field, as it does for every field of a new issue.
3. **One way to show a length:** the issue page, the board card and the new-issue form write lengths through the shared helpers (TXT-DATES) in the code face (`weight-code`); nothing writes a length by hand.

## Acceptance tests (write first)

- The layout editor places each of the three fields on the issue page and the board card; the issue page shows them in that place; a person without the right to read *Estimated time* does not see it anywhere.
- *Time spent* in a layout cannot be typed into.
- A new issue made with `1:30` has *Estimated time* and *Time left* of 1 h 30 min, on both databases; a wrong length is told beside the field and nothing is made; a person without the right to change the estimate gets no estimate box.
- The default layouts show the same issue page as before (picture comparison).

## Checks

TEST-A11Y, TXT-DATES, the existing layout and field-permission tests.

## Review

The SSOT/DRY/ODR gate of coding-standards §1: the three fields go through the field list and the field permissions every other field uses, and the reader of lengths of WP-03; no second path for time fields. A UX reviewer looks at the pictures in four themes, on computer and phone: the layout editor with the time fields, the issue page, the board card, the new-issue form with an estimate.

## Depends on

WP-07 merged; after WP-11 in the order of merging (WP-11 changes the time card of the issue and the settings near it).
