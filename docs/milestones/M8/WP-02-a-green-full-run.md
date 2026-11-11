# WP-02 — A full run that is green, and the two small screen gaps of M7

**Wave 0 · Folder / branch:** `bugaboo.win2` / `WIN2`
**Features:** AI-33, AI-35 (fixes on screen). Follow-ups M7-F243, M7-F244, M7-F245, M7-F246; architecture §11 (the full run, the browser tests, the ten-people runs).

## Goal

`cargo xtask ci` is green on this computer, run after run. Since 2026-11-05 almost every full run had one to three browser tests fail under load that pass alone, and some Rust tests fail now and then for the same reason. Each failure is fixed at its cause, so a red run again means a real fault. The two small screen gaps of the M7 exit are fixed too.

## Owns

The browser test support in `web/app/e2e/support/**` and the specs it fixes, `web/tooling/playwright.config.ts`, the connection guard of the tests (`web/tooling/connection*.ts`), the Rust tests named in M7-F243 and the code under them that the fix needs (`crates/bb-app/src/auth/oauth_rate.rs`, the lease clock of the concurrent test, the pool of the session tests), and for the screen gaps the issue page's agent work on the phone and the review window (`web/app/src/features/agent-work/**`, only their layout and the parts of `design/theme.kdl` they use).

## What is decided

- **No longer time limits and no retries** to make a test pass. A test waits for what it depends on (architecture §11). A retry inside a test is allowed only where pressing again cannot change the result (as `ai_10` now presses until the panel opens).
- **Sockets first.** The run of 2026-11-11 hit `net::ERR_NO_BUFFER_SPACE` with about 1,200 connections in TIME_WAIT (the guard's limit is 1,800). Find where the connections come from (new connections per request, servers started per test, `Connection: close`) and make the tests reuse them. The guard's numbers before and after go into the report.
- **A clock that moves with the work.** The lease test and the limit of app calls count with the wall clock (`Instant::now()`), so a slow computer runs out of time that the test did not mean to spend. They take the clock of the app, which tests can move.
- **Three green full runs in a row** on this computer is the acceptance, with nothing else building or testing at the time.
- **The screen gaps:** on the phone the chip *Waiting for you* stays fully visible below the bar when the issue page scrolls to agent work (M7-F245). The review window is as tall as its content for the choice made, with no empty area under *Move the issue to* (M7-F246). Both are checked in the four themes, on computer and phone.

## Tasks

1. Read the logs of the failing full runs in `bugaboo.win1/.work/` (`ci-m7exit*.log`, `ci-wp*.log`) and list every test that failed, with how often and its first error line.
2. Measure the connections of a full browser run (the guard's report, `netstat` during the run) and find the specs and helpers that open the most. Fix the cause.
3. Fix each listed test at its cause (M7-F243, F244). For each one, write in the report what the cause was.
4. Fix M7-F245 and M7-F246, with pictures.
5. Run `cargo xtask ci` three times in a row on a quiet computer; all three green.

## Acceptance tests (write first)

- `ai_33_the_state_of_agent_work_stays_below_the_bar_of_the_phone_page` (phone).
- `ai_35_the_review_window_is_as_tall_as_the_choice_made` (computer and phone).
- For each Rust test of M7-F243: a test that moves the app's clock past the limit and shows the old wall-clock failure cannot happen.
- The guard of the connections reports a peak below 600 in TIME_WAIT for a full browser run (or the lower number the fix reaches; the report names it).

## Checks

TEST-CONNECTIONS (its limit may be lowered to the new peak plus a margin), TEST-A11Y, TEST-CONCURRENT.

## Review

Code review with the SSOT/DRY/ODR gate: one helper for each kind of wait, not one per spec. The reviewer checks that no time limit grew and no retry hides a fault.

## Depends on

Nothing. Runs alone: while it runs, no other folder builds or tests.
