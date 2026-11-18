# WP-15 — Sample part 8, the long run, speed, the release candidate, the exit demo

**Wave 4 · Folder / branch:** `bugaboo.win2` / `WIN2` · **Model:** Opus (concurrency, speed, the security pass)
**Features:** PRIV-03, PRIV-04, OPS-04, OPS-06, OPS-07, SEC-18, COMP-01 (proved end to end). The exit demo of [04-milestones](../../04-milestones.md) (M9) as the README of M9 breaks it into steps; architecture §11.

## Goal

The sample organization shows the privacy and operations of M9; ten people work for two hours on each database while a backup is made and later restored, and nothing is lost; the benchmarks on 1 M issues hold their budgets; the release candidate 0.6.0 is built, signed, installed and upgraded; a security pass over the whole release finds nothing open; and the exit demo runs step by step as tests, with pictures in four themes on computer and phone.

## Owns

Sample part 8 in `crates/bb-sample/src/` (a switched-off person, a removed person with their kept work days, a finished export, two backups, an ended support access unless the cut moves it; made through the use cases); the long run in `xtask/src/load/` (`--long <length>`, the backup step in the middle, the restore into a second database and its invariants) and its rule in architecture §11, *The long run*; the grown TEST-CONCURRENT in `xtask/src/checks/test_concurrent.rs` and its catalogue entry and fixtures; `web/app/e2e/m9-exit-demo.spec.ts` and its support files; the Rust tests of the exit demo's command steps in `crates/bb-server/src/tests/exit_demo_m9.rs`; the release notes `docs/releases/0.6.0.md`; the M9 part of architecture §11; the report of the security pass (findings fixed in their packages' code, or follow-ups).

## What is decided

- **Sample part 8:** a former colleague already removed (comments kept as *Former user*; work days under the legal-hold name for Hana Weiss, *HR*) and one switched off; Ben Hale stays active for the exit demo, which switches him off and removes him; Lea Fischer's finished export; two backups; a support access that ended. TEST-SAMPLE-DATA checks it.
- **The long run** (decided by the rules: two hours per database, by hand at the exit; TEST-CONCURRENT grown, "Adding a rule"): `cargo xtask load --people 10 --long 2h` on SQLite and on PostgreSQL, by hand at the exit when nothing else builds or tests; at the middle a backup is made through the route while the ten people work; after the run that backup is restored into a second database and the invariants of architecture §11 (*Invariants after every run*) hold on both. The catalogue's tool text names `--long` instead of `--nightly`; the check makes sure the long scenario and its backup step exist and are not switched off.
- **Speed** (PERF-BENCH): every scenario of `cargo xtask bench` on both databases on the 1 M datasets, by hand on a quiet computer; new scenarios for the walk of a person with years of data (`person-data-walk`, the count of the removal plan within the budget of a page) and for a backup of the 1 M dataset (its length written down, no budget yet). The known gaps keep their limits (architecture §11).
- **The release candidate:** `cargo xtask release` for 0.6.0-rc.1 on Windows and in WSL; signed with the real key if the maintainer has handed it over (WP-08), otherwise with a test key and marked so; the try-out runs of WP-13 on it; TEST-UPGRADE from `m8-done` on it.
- **The security pass** (M9-Q19 as answered): one fresh Opus reviewer per boundary of architecture §12 (browser and server, the API and its keys, files and previews, AI and agents, the host and the operator), each reading the code of the release against the threat model; findings fixed before the exit or recorded as follow-ups with the maintainer's approval.
- **The exit demo:** the eight steps of the README of M9. Steps 4 to 8 are tests of `m9-exit-demo.spec.ts`, titled *"4. A backup made while people work is restored"*, *"5. Lea downloads her data, and an administrator downloads Ben's"*, *"6. Ben is removed"*, *"7. The About page shows the release"*, *"8. An installation of M8 is upgraded without losing anything"* (this one in Rust, `bb-server`); steps 1 to 3 are the release test of WP-08 and the try-out runs of WP-13 on the candidate, with their results on the review page. Pictures in four themes on computer and phone.

## Tasks

1. Write the acceptance tests below.
2. Sample part 8.
3. The long run with its backup step; the grown check and its rule.
4. The new bench scenarios; the measurement at the exit.
5. The release candidate and its runs; the release notes.
6. The security pass.
7. The exit demo as tests, with its pictures; the M9 part of architecture §11.

## Acceptance tests (write first)

- `priv_04_the_sample_has_a_switched_off_and_a_removed_person` (TEST-SAMPLE-DATA).
- The long run's invariants on both databases, and on the restored copy of its middle backup.
- TEST-CONCURRENT cases in `web/app/e2e/concurrent/privacy.spec.ts`: `priv_04_ten_people_work_while_a_person_is_removed_and_nothing_else_changes`, `ops_07_ten_people_save_while_a_backup_is_made_and_every_save_up_to_its_moment_is_in_it`, `priv_04_a_switched_off_persons_open_page_is_refused_at_its_next_save`, and their `bb-load-check` counterparts.
- The PERF-BENCH scenarios `person-data-walk` and `backup-large`.
- Each step of the exit demo as a test, titled as above.

## Checks

TEST-SAMPLE-DATA, TEST-CONCURRENT (grown), TEST-CONNECTIONS, TEST-UPGRADE, PERF-BENCH, META-SPEED, DEP-SBOM, TEST-A11Y, THEME-ONLY.

## Review

Code review with the SSOT/DRY/ODR gate: the sample through the use cases only, one helper for each kind of wait (architecture §11), the budgets once in the bench budgets. **UX review** of the exit demo's pictures in four themes, computer and phone. The security pass is its own review (above).

## Depends on

Waves 1 to 3 merged; WP-14 pushed before the browser runs. Runs alone at its measurements: no other folder builds or tests while the long run or the benchmarks run.
