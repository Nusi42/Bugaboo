# WP-11 — The operations screens: *Backups*, *About*, failed jobs, notes for administrators

**Wave 3 · Folder / branch:** `bugaboo.win2` / `WIN2` · **Model:** Sonnet (an exact brief: routes exist, words and frames come from WP-01)
**Features:** OPS-07, COMP-01, COMP-04 (on screen). UX design §5.1 (*System*) as WP-01 completes it; ADR-0046, ADR-0047; follow-ups M4-F63, M4-F128.

## Goal

An administrator makes, schedules, lists and restores backups on a screen; everybody signed in sees on *About* which version runs, until when it gets security fixes, and the licenses and parts list of what is inside; an administrator sees failed jobs and can try one again; a note says when sign-up e-mails were held back.

## Owns

`web/app/src/features/admin/BackupsPage.tsx` and `RestoreWindow.tsx` (new), the page *Bugaboo is being restored* (`web/app/src/features/admin/RestoringPage.tsx`, shown when the server answers with restore mode), `web/app/src/features/admin/AboutPage.tsx` (new), `web/app/src/features/admin/FailedJobsPage.tsx` (new); the route of failed jobs in `crates/bb-api/src/api/admin.rs` (`GET /api/v1/admin/failed-jobs`, `POST /api/v1/admin/failed-jobs/{job}/try-again`, calling the store's `requeue_dead_job`, M3-F150, M4-F63) and its use case in `crates/bb-app/src/jobs.rs`; the note of capped sign-up e-mails on the status page (`StatusRows.tsx`, the count from `crates/bb-app/src/status.rs`, M4-F128); the addresses in `paths.ts` and the menu entries; their texts; `userDocs/install/backups.md` (screens part, with WP-02) and `userDocs/basics/about.md` (new).

## What is decided

- **Words and layout** come from WP-01's frames; nothing new is named here.
- ***Backups*:** where backups are and that they hold all the organization's data (one sentence); the schedule (*Daily backups* on or off, how many daily and weekly ones are kept); *Make a backup now* (a state in place while it runs, the new row appears at the top without moving the rest); the list (name, moment, size, made by whom or *the schedule*, newest first, paged); *Restore this backup…* opens the window: what happens (everybody is signed out, Bugaboo is closed while it restores, a backup of the present is made first), the password and second step, then the button. While the server restores, every page shows *Bugaboo is being restored* and asks again every few seconds; when it is open, the sign-in page comes back.
- ***About*:** for everybody signed in: the version, the support period (*Security fixes until …* or, for a preview, *Only the newest release gets fixes*), the license (AGPL-3.0-or-later), the parts by name, version and license (a virtualized list), *Download the parts list* (CycloneDX and SPDX), *Third-party notices*. Reached from the menu of the person.
- **Failed jobs:** a list for holders of *See how the server is doing* (`see-status`); *Try again* needs *Change organization settings*; a job is named by its kind in plain words and its last failure's reason, never its payload (it can hold personal data).
- **Capped sign-up e-mails** (M4-F128): the status page shows "Sign-up e-mails were held back N times in the last day" while it happened, with the advice to set approved domains.

## Tasks

1. Write the acceptance tests below.
2. The route and use case of failed jobs.
3. *Backups*, the restore window and the restore page.
4. *About*.
5. *Failed jobs* and the note on the status page.
6. The user docs; pictures in four themes on computer and phone; frame samples of *Make a backup now*.

## Acceptance tests (write first)

- `ops_07 an administrator makes a backup and it appears at the top without moving the list`.
- `ops_07 the restore window asks for the second step and every page shows the restore page until it is open again`.
- `comp_01 About shows the version the support period and the parts list to download`.
- `ops_08_a_failed_job_is_tried_again_by_an_administrator` (Rust, both databases; M4-F63).
- `ops_08 failed jobs show the reason and never the payload`.
- `ops_08 the status page notes held back sign-up e-mails` (M4-F128).
- SEC-MATRIX: rows for the two routes of failed jobs for every starting role and an access key.
- TEST-A11Y for every new address; pictures in four themes, computer and phone.

## Checks

TXT-LITERAL, TXT-CATALOG, TXT-TERMS, TXT-DATES, THEME-ONLY, STY-PARTS, ARCH-WEB-API, ARCH-PERMS, SEC-MATRIX, LINT-DRY, TEST-A11Y, PERF-BUNDLE.

## Review

Code review with the SSOT/DRY/ODR gate: the restore page is one component reached from the server's answer, the support period read from `about`. **UX review** of the pictures and the frame samples.

## Depends on

WP-02, WP-05 and WP-08 merged. Runs beside WP-10, WP-12 and WP-13.
