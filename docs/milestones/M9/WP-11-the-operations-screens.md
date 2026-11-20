# WP-11 — The operations screens: *Backups* and *About*

**Wave 3 · Folder / branch:** `bugaboo.win2` / `WIN2` · **Model:** Sonnet (an exact brief: the routes exist, words and frames are fixed)
**Features:** OPS-07, COMP-01, COMP-04 (on screen). UX design §5.1 (*System*); ADR-0046, ADR-0047. M4-F63 and M4-F128 moved to a later operations package (M9-Q20).

## Goal

An administrator makes, schedules and lists backups on a screen, and the screen says how to restore one; everybody signed in sees on *About* which version runs, until when it gets security fixes, and the licenses and parts list of what is inside.

## Owns

`web/app/src/features/admin/BackupsPage.tsx` (new) and `web/app/src/features/admin/AboutPage.tsx` (new); the addresses in `web/app/src/shared/paths.ts` and the menu entries in `SettingsScreens.tsx`; their texts; `userDocs/install/backups.md` (screens part, with WP-02) and `userDocs/basics/about.md` (new).

## What is decided

- **Words and layout** follow UX design §5.1 and the mocks the maintainer answered on; nothing new is named here.
- ***Backups*:** where backups are and that they hold all the organization's data (one sentence); the schedule (*Daily backups* on or off, how many daily and weekly ones are kept); *Create a backup now* (a state in place while it runs, the new row appears at the top without moving the rest); the list (name, moment, size, created by whom or *the daily backup*, newest first; the list is not paged: the keep periods bound it, and it is drawn virtualized in a box of its own height); under it, how to restore: stop the service and run `bugaboo restore "<name>"`, with the name of the chosen backup filled in and a link to the user docs. There is no restore button (M9-Q10, M9-Q20).
- ***About*:** for everybody signed in: the version, the support period (*Only the newest release gets fixes* for a preview, *Security fixes until …* from 1.0), the license (AGPL-3.0-or-later), the parts by name, version and license (a virtualized list), *Download the parts list* (CycloneDX and SPDX), *Third-party notices*. Reached from *My settings* (System group), because everyone may see it.

## Tasks

1. Write the acceptance tests below.
2. *Backups*.
3. *About*.
4. The user docs; pictures in four themes on computer and phone; frame samples of *Create a backup now*.

## Acceptance tests (write first)

- `ops_07 an administrator makes a backup and it appears at the top without moving the list`.
- `ops_07 the backups page names the command that restores the chosen backup`.
- `comp_01 About shows the version the support period and the parts list to download`.
- TEST-A11Y for every new address; pictures in four themes, computer and phone.

## Checks

TXT-LITERAL, TXT-CATALOG, TXT-TERMS, TXT-DATES, THEME-ONLY, STY-PARTS, ARCH-WEB-API, LINT-DRY, TEST-A11Y, PERF-BUNDLE.

## Review

Code review with the SSOT/DRY/ODR gate: the command's wording comes from one message key, the support period is read from `about`. **UX review** of the pictures and the frame samples.

## Depends on

WP-02, WP-05 and WP-08 merged. Runs beside WP-10 and WP-13.
