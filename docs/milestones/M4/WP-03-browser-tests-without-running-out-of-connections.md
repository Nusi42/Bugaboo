# WP-03 — Browser tests that never run out of connections

**Wave 1 · Folder / branch:** `bugaboo.win3` / `WIN3`
**Features:** none of its own; it keeps the browser tests of UX-01 and UX-04 reliable. Architecture §11 (testing); follow-up M2C-F104.

## Goal

`cargo xtask ci` passes on the first run. In each of the last three merges of M3, the first run lost one browser test to `net::ERR_NO_BUFFER_SPACE`. On Windows that error means the computer has no free network ports or socket buffers left. A test run must never be able to cause it.

## Owns

`web/tooling/**` (Playwright config, ports), `web/app/e2e/support/**` (fixtures, server start, API helpers), the e2e part of `xtask/src/ci.rs`, `tools/dev/ports.json`, the testing chapter (`docs/architecture/11-testing.md`, the e2e section). It does not change product code. If a cause turns out to be in the server (for example connections never closed), the change goes into `crates/bb-api/src/listener.rs` in its own commit, and the report names it.

## What is known (from the lead's reading)

- `web/tooling/playwright.config.ts`:
  - `workers = min(20 ports, floor(cpus/2))`;
  - files run side by side;
  - no retries except the `speed` project.
- `web/app/e2e/support/fixtures.ts:88-111`: `forFile` stops the worker's server and starts a fresh one for every spec file (75 files per project, four projects).
- `web/app/e2e/support/server.ts:60-119`: readiness is a raw TCP connect every 100 ms for up to 30 s. Each probe is a short connection that is destroyed at once and leaves a port in TIME_WAIT for about two minutes.
- `web/app/e2e/support/api.ts`: `request.newContext` in 9 places; each context opens its own TLS connections.
- `crates/bb-api/src/listener.rs`: no keep-alive or TCP options are set.
- Polling in the app (edit-lock heartbeat, version watch, run progress) multiplies connections in the `concurrent` specs with ten browser contexts.

## Tasks

1. **Measure first.**
   - During one full `cargo xtask ci` e2e run, sample the connections of the test processes every few seconds: count by state (ESTABLISHED, TIME_WAIT, CLOSE_WAIT) with `Get-NetTCPConnection` or `netstat`.
   - Write down the peak and which project or spec reaches it, in the testing chapter.
   - Each later task says what it changed in the peak.
2. **Readiness without a storm.**
   - Wait for the server by its own signal: a line it prints when it listens, or HTTP requests to `/health/live` with back-off.
   - Never a TCP probe every 100 ms.
3. **Fewer server starts.**
   - Keep one server per worker, and give each spec file a clean organization through a reset route that exists only in the test build, or through a fresh data folder on the same process.
   - Restart the server only when a spec needs a different start (sign-up switched off, another setting at start).
   - Whichever way: a spec still cannot see what an earlier spec left behind. A test proves it.
4. **Reuse connections.**
   - One API request context per worker (or per file) with keep-alive, not one per helper call. Each context is disposed when its file ends.
   - If the server closes idle connections too early, or never closes them, fix it in the listener (keep-alive time as a named constant).
5. **A guard.** At the end of the e2e run, the connections in TIME_WAIT and CLOSE_WAIT owned by the run must be under a limit (a named constant, set from task 1 with room to spare). Over the limit, the run fails with a sentence that names the peak and the spec. Follow "Adding a rule" in architecture §10: the rule, the catalogue entry, the implementation, a bad and a good example.
6. **Showcase pictures the same on every run (M2C-F104).** Fixed times and fixed e-mail addresses in the showcase specs, so a second run makes the same pictures.
7. **No hiding.** No retries are added to make the flake go away. `fixtures.ts:23` keeps counting every `net::ERR_*` as a failure.

## Acceptance tests (write first)

- A unit test of the readiness wait: it makes at most a handful of connections before the server answers (a fake server that answers late).
- A spec-isolation test: a spec that creates an issue, and the next spec on the same worker does not see it.
- The guard's own tests: a run over the limit fails with the sentence; a run under it passes.
- Three full `cargo xtask ci` runs in a row, all green on the first try, with the connection peak of each in the report.

## Checks

The guard of task 5 (new, through "Adding a rule"). No other check changes.

## Depends on

Nothing in M4. It touches only the test tooling, so it runs beside WP-02.
