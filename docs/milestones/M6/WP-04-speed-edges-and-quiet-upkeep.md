# WP-04 — Speed edges and quiet upkeep

**Wave 1 · Folder / branch:** `bugaboo.win4` / `WIN4`
**Features:** PERF-01 (server response targets), PERF-05. ADR-0017 (amendment M5, the statistics job), architecture §7 and §11 (*Speed on 1 M issues*). Follow-ups M5-F168, M5-F169, M5-F170 with M5-F89 (accepted by the maintainer as M6 follow-ups).

## Goal

No person's change fails because the database is doing its own upkeep, and the searches that sat at the edge of their budget on 1 M issues are back inside it, or their gap is measured and said.

## Owns

`crates/bb-app/src/statistics*` and its job kind, the start round of `bb-jobs` (`Ctx::start_daily_jobs`), the retry of a busy write in `bb-store` (one place, next to the busy timeout), the groups by *Assigned to* in `crates/bb-store/src/search/`, the preview of a change to many issues where it reads, `xtask/src/dataset*` (the `VACUUM ANALYZE` step), `[known_gaps]` of `tools/checks/perf-bench.toml`, the numbers in architecture §7 and §11, ADR-0017's lines about the statistics.

## What is decided (maintainer decision)

- **M5-F168:** no statistics refresh in the start round. It runs a few minutes after the start or at a quiet hour, and a write that meets SQLite's busy answer while upkeep holds the lock is tried again. Not a longer busy timeout for every writer.
- **M5-F169, F170 with M5-F89:** the groups by *Assigned to* and the preview of 5,000 issues are worked on in M6; on PostgreSQL the dataset gets `VACUUM ANALYZE` once it is made.

## Tasks

1. **The statistics after the start.** `statistics.refresh` is no longer queued in the start round; the first round runs at a set delay after the start (a named constant, a few minutes) and then daily at a quiet hour of the server's zone. A test proves the start round queues none.
2. **A busy write is tried again.** One place in `bb-store` tries a write that met `SQLITE_BUSY` again, a small number of times with a short wait, only while an upkeep job holds the lock (the job marks it), and reports a plain refusal after the last try. The busy timeout of every writer stays as it is. The write must be one that can be tried again whole (it is one transaction).
3. **Upkeep in smaller steps.** If the first refresh of a large table still holds the lock for seconds, run it in smaller steps (per table, and on SQLite with `PRAGMA analysis_limit`), and measure the longest hold on the cold 1 M dataset; the number goes into architecture §7.
4. **The groups by *Assigned to* (M5-F89, F169, F170).** Try the ideas of M5-F89 in order of how little they add: the groups from the assigned rows first and the issues by a lookup; the groups read in parallel slices; counts kept for the sight only if both fail. Measure each on both databases in the nightly run, five runs, with the computer otherwise quiet (no full run of another folder; ask the lead). Keep the one that meets 300 ms on PostgreSQL with room and comes under 300 ms on SQLite; then remove the entry from `[known_gaps]`. If none does on SQLite, keep the gap, write the numbers and why, and raise its limit only with the lead's word.
5. **The preview of 5,000 issues (M5-F169).** Measure where its 8 s go on PostgreSQL; cut the largest part.
6. **The dataset on PostgreSQL.** `cargo xtask dataset large` runs `VACUUM ANALYZE` when it has made the rows, so a measurement never starts on a database without statistics.

## Acceptance tests (write first)

- `adr_0017_the_start_round_queues_no_statistics_refresh`.
- `adr_0017_a_write_during_the_statistics_refresh_is_tried_again_and_lands` (SQLite: a job holds the lock longer than the busy timeout; the person's change lands after a retry; on PostgreSQL the same test passes without one).
- `adr_0017_a_write_that_stays_busy_is_refused_in_plain_words_after_the_last_try`.
- PERF-BENCH: `search-groups-assigned-to` and `many-changes-preview` within budget on both databases in five nightly runs, or the gap entry with its measured numbers.

## Checks

PERF-BENCH (the known gap removed or justified). PERF-N1 unchanged for the search routes.

## Review

The SSOT/DRY/ODR gate of coding-standards §1: one retry, in one place, with its limits as named constants; no second schedule of daily jobs. A security reviewer checks that a retried write is never applied twice (the event and the outbox entry once).

## Depends on

`main` at the M6 plan. The nightly runs need a quiet computer: the lead plans the wave so this package's measuring runs do not meet another folder's full run.
