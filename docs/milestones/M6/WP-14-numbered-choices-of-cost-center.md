# WP-14 — Numbered choices: *Cost center* as number and name

**Wave 4 · Folder / branch:** `bugaboo.win4` / `WIN4`
**Features:** TIME-13 (*Cost center* as a managed list of number + name), CF-08. Follow-up M6-F41.

## Goal

A choice of *Cost center* has a number and a name, for example *4100 Product development*; the list is sorted by the number everywhere it is shown, and the number can be searched for.

## What is decided

The maintainer (2026-11-05, M6-F41): number + name, sorted by the number.

## Owns

The number of a choice of an own choice field (store, API, the *Fields* page), the order of choices by number, its migration, and the choice boxes of time entries that show them.

## Tasks

1. **The number of a choice:** a choice of an own choice field may have a number besides its name. The *Fields* page lets an administrator set it; a number is unique within the field. A field whose choices have numbers lists them by number; one without keeps the order the administrator set. *Cost center* is made with numbers; *Activity* has none.
2. **Shown the same everywhere:** the choice box of a time entry, the *Stop timer* window, *Add time*, the timesheet, the project defaults of WP-11 and the search show "4100 Product development" through one helper (the `shown_as` text of the server); no screen joins number and name by hand. Where a screen can draw more than plain text (badges, the field page, the timesheet) the number is in the code face; in a plain browser drop-down, which cannot mix faces, the number stays in the text face.
3. **Search:** the search language finds a choice by its number or by its name (`cost center = 4100`), in the catalogue of the search, with the field permissions of QRY-05.
4. **Stored data:** a migration adds the number; the choices that exist keep their names and get no number until an administrator sets one; the starter field of a new organization gets its numbers. The number of the migration comes from the lead in the order of merging.

## Acceptance tests (write first)

- An administrator sets numbers on the *Fields* page; a number used twice is told and not saved; the choice box of a time entry lists the choices by number, written "4100 Product development".
- A field without numbers keeps the order of the administrator.
- `cost center = 4100` finds the entries booked on that choice; a person who may not read *Cost center* gets the usual answer for an unreadable field.
- The migration on both databases: existing choices keep their names and order.

## Checks

TEST-A11Y, TXT-LITERAL, the migration and search tests of both databases.

## Review

The SSOT/DRY/ODR gate of coding-standards §1: one place writes a numbered choice, one rule orders choices; the number is part of the choice of an own field, not a second list for *Cost center*. A UX reviewer looks at the pictures in four themes, on computer and phone: the *Fields* page with numbers, the choice box of a time entry, the timesheet, the project defaults.

## Depends on

WP-10 and WP-11 merged (both show *Cost center*; WP-10 sends the names of choices with each entry).
