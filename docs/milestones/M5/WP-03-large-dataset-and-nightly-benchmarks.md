# WP-03 — The large dataset, the benchmarks and the nightly run

**Wave 1 · Folder / branch:** `bugaboo.win3` / `WIN3`
**Features:** PERF-01 (the measuring part; the searches it measures come with WP-04 to WP-06). Architecture §11 "Large dataset" and "Speed on the sample", §7 (the dataset generator); follow-ups M3-F92, M3-F114.

## Goal

PERF-01 is measured where it is promised: on about 1 million issues, 500 people and 500 projects, on both databases, in a release build, every night. M5's searches are then held to their budget from the day they are merged, and the numbers of today's reads are known before anything changes.

## Owns

`xtask/src/dataset*` and the new `large` part of it; the benchmark runner and its scenarios; `tools/checks/perf-bench.toml` (new: the budgets, the only place they are written); PERF-BENCH in `tools/checks/catalog.toml` and its check; the nightly command; a bulk-write port used only by the dataset (in `bb-store`, behind a feature or a module that the server does not build in); the sections "Large dataset" and "Speed on the sample" of architecture §11, and the dataset line of §7.

It does not change product behaviour. A slow read found by the measurement is reported to the lead, not fixed here.

## What is decided (M5-Q4)

- `cargo xtask dataset large [--issues N]` (default 1,000,000) writes the rows in bulk, not through the use cases. The history it writes is valid and is checked by the same rules as the sample.
- The nightly run is on this computer, at night, on SQLite and PostgreSQL, release build. Results stay in the folder's `.work/`; a short summary goes into the repository by the package that changes the numbers.
- The budgets of PERF-01 count on this computer until a reference server exists: p95 under 100 ms for an issue page and a saved search, under 300 ms for a complex search (for example `statusGroup is not Done and priority is High` across 500 projects, architecture §8).

## Tasks

1. **The shape.** The same kinds of things as the sample (`tools/dev/sample.json`, `bb-sample`), at the PERF scale: 500 people in groups, every starting role in use, 500 projects (some with sub-projects), the starter workflows and project workflows, issue types, own fields including *Cost center* that Contributors may not see, labels, tags, relations and sub-issues (chains for `connected to`), on average two comments per issue with real-looking text in English and German (full-text search needs it), followers and notifications. File records without file bytes. The seed is fixed, so two runs make the same data.
2. **Bulk writes.** A port that writes prepared rows in large transactions (PostgreSQL `COPY` or multi-row inserts, SQLite batches in one transaction), with the events and history rows the use cases would have written. The server never builds it in.
3. **Proof that the data is valid.** A test makes a small large-dataset (a few thousand issues) and runs the sample's own checks on it (the rules `bb-sample` checks, e.g. guests are told only about what they may see). Where a rule cannot hold for bulk data, ask the lead; do not leave it out silently.
4. **Time and disk.** Measure how long 1 M issues take and how much disk they use on each database; write both into §11. The data lives in the folder's `.work/dataset/`, never outside the folder; one dataset per database is kept, older ones deleted.
5. **The benchmark runner** (`cargo xtask bench`): starts a release server on the large dataset, warms up, then runs each scenario a fixed number of times and writes p50, p95 and p99 per scenario to `.work/bench/<date>.json`. Scenarios are registered in one place, so WP-04 to WP-06 and WP-08 add theirs. Start with today's reads: an issue page (the reads the browser makes at once), the first page of a project's list for a big and a small project, the notifications page, the project list (M3-F114).
6. **PERF-BENCH.** Through "Adding a rule" (architecture §10): the budgets in `perf-bench.toml`, every scenario names its budget, and a run over a budget fails with the scenario, the number and the budget. The check that runs at commit time checks the file and the scenario list; the nightly run checks the numbers.
7. **The nightly command** (`cargo xtask nightly`): the full `cargo xtask ci`, then `dataset large` if the dataset is older than the last migration, then `bench` on both databases. It waits while another full run is on the computer (the rule "One full run at a time"), with a lock that writes nothing outside the folder (a named system lock), and writes one log into `.work/nightly/`. Also write `cargo xtask nightly --install`, which registers the scheduled task (Windows Task Scheduler, 03:00, in the lead's folder `bugaboo.win1`). **Do not install it yourself**: the lead does after the merge.
8. **The first numbers.** Run the benchmarks once on both databases, and write the numbers of today's reads into §11 next to their budget. A read over its budget is reported to the lead with its scenario.

## Acceptance tests (write first)

- The dataset is the same twice from the same seed (a hash of a small run).
- The sample's checks pass on a small large-dataset, on both databases.
- PERF-BENCH: a result over a budget fails with the sentence; a scenario without a budget fails; a good file passes.
- The nightly command waits while the lock is taken (a test with the lock held).

## Checks

PERF-BENCH planned → active. META-FIXTURES gets its bad and good example.

## Depends on

`main` at the M5 plan. Nothing else in M5; it runs beside WP-02. Disk is tight: keep one dataset per database, and say in the report how much disk they take.
