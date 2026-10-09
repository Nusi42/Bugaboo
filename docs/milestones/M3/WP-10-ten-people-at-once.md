# WP-10 — Ten people at once: TEST-CONCURRENT, TEST-EDIT-LOCKS, WebKit

**Wave 3 (merges last) · Folder / branch:** `bugaboo.win4` / `WIN4`
**Features:** ISS-17, PERF-02 (many people at once); architecture §11 (*Many people at once*); ADR-0019; follow-ups M2-F12, M2C-F41.

## Goal

Ten people working on the same issues at once lose no text, never see a half-saved change, and always learn in plain words why a save did not go through; this holds on both databases and in a phone browser that is not Chromium.

## Owns

`web/app/e2e/concurrent/**` (new), `xtask/src/load/**` (new: `cargo xtask load --people 10`), the TEST-CONCURRENT entry and its nightly variant, the browser part of TEST-EDIT-LOCKS, the WebKit project in `web/tooling/playwright.config.ts` (M2-F12), `web/tooling/ports.ts` (M2C-F41: read the block variable and folder prefix from `ports.json`).

## Tasks

1. **Scenarios (architecture §11):** three people edit one description (one holds the lock, the others see "… is editing", *Edit anyway* keeps the first person's text); two people change one status within a second (one wins, the other sees the banner, nothing half-saved); a lock holder's network drops (the lock frees within a minute, the text comes back with *Compare and save*); ten people change different fields of one issue (all land, one event each); changing many issues while somebody edits one of them (that one is skipped and named).
2. **Two ways to run them:** ten Playwright contexts for the screens; `cargo xtask load --people 10` for the API at speed. A 2-minute run in `cargo xtask ci` on both databases, a 30-minute run nightly.
3. **Invariants after every run:** every change has an event; no lock outlives its lease; no draft or text is lost; versions only go up.
4. **WebKit (M2-F12):** a phone project on WebKit for the issue page, the new-issue dialog and the lock scenarios.
5. **Ports (M2C-F41):** `ports.ts` reads `blockVariable` and `workFolderPrefix` from `tools/dev/ports.json` like xtask.

## Acceptance tests (write first)

The scenarios above as `test_concurrent_*` and `iss_17_*` Playwright tests, and `test_concurrent_invariants_hold_after_a_run` against the database.

## Checks

- **TEST-CONCURRENT** (planned → active): short run in ci, long run nightly, both databases.
- **TEST-EDIT-LOCKS:** adds the browser scenarios to WP-04's server tests.

## Depends on

WP-04, WP-06, WP-08 (all screens and routes it drives). It merges last in wave 3.
