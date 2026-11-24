# WP-15 — Sample part 9, ten people, speed, the long run, the exit demo

**Wave 4 · Folder / branch:** `bugaboo.win3` / `WIN3` · **Model:** Opus (concurrency, speed diagnosis on 1 M issues, the release 0.6.1)
**Features:** PLAN-05, PLAN-06, PLAN-09, ISS-16, ISS-08, CFG-03, CFG-05, PRJ-06, QRY-07, QRY-08 (proved end to end). The exit demo of [04-milestones](../../04-milestones.md) (M10) and of the README; architecture §11.

## Goal

The sample organization has boards, calendars, releases, chains of relations, dashboards, pins and an e-mailed search. Ten people can drag cards, move dates and change dashboards at once without losing a change. The new reads answer within their budgets on 1 M issues. The release 0.6.1 holds the long run. The exit demo runs step by step as tests, with pictures in the four themes on computer and phone.

## Where

- **Owns:** sample part 9 in `crates/bb-sample/src/` (new files `releases.rs`, `boards.rs`, `dashboards.rs`, and additions to `relations.rs`, `searches.rs`) and its share of the large dataset (`large_*.rs`: releases and the two fields, relation chains dense enough for the map); the ten-people runs in `crates/bb-load-check/src/` and `web/app/e2e/concurrent/`; the new scenarios of `cargo xtask bench` (`web/tooling/bench/`: a new `views-scenarios.ts`) on the budgets that exist; `web/app/e2e/m10-exit-demo.spec.ts`; `crates/bb-server/src/tests/exit_demo_m10.rs` (the e-mail step); the M10 parts of architecture §11; the release notes of 0.6.1.

## What is decided

- **Sample part 9,** made through the use cases (TEST-SAMPLE-DATA): releases *2.4* (released) and *2.5* (planned, with a date) in *Website*, bugs found in 2.4 and planned for 2.5; a *Website* board (columns by status, rows by person, a card limit of 3 on *In progress*) and a calendar; the blocking chain of UX design §4.15 (WEB-142, WEB-147, WEB-148 and the others) and a loop of *blocks* stored as before migration 49 in another project; an issue of a project Ben has *No access* on between two of his; Ana's dashboard *Website this week* shared with the *Website* group; Ana's pins; Ben's weekly e-mailed search; a sub-project of *Website* with its own workflow.
- **Ten people at once** (TEST-CONCURRENT, 30 minutes on each database as in every milestone): two people drag the same card to different columns; a card moved while its workflow's allowed changes are edited; a calendar drag while someone changes the same due date on the issue page; two tabs of the maker change one dashboard; a release archived while issues are planned for it; e-mailed searches sent while their search is unshared. No change is lost; a stale change gets *Keep Ben's change / Use mine*; the invariants of architecture §11 hold.
- **The long run** of the release 0.6.1: two hours on each database, by hand at the exit on a quiet computer, with the backup in the middle (as since M9), the new scenarios included.
- **Speed** (PERF-BENCH, PERF-01) on 1 M issues, on the budgets that exist: `board-biggest-project` (search, 300 ms), `calendar-month-biggest-project` (search), `relation-map-three-steps` (search), `totals-by-priority-biggest-project` (search; a total of *Time spent* is measured against the known gap of M6-F186 and not given a new budget), `dashboard-ready-made-boxes` (each box's read on its own budget), `releases-of-a-project` (page, 100 ms); the board, calendar, map and dashboard chunks within PERF-04. A burst of 500 e-mailed searches at one hour is written down (`written_down`, its length in architecture §11). Measured by hand on a quiet computer when the lead says nothing else builds or tests.
- **The exit demo:** the ten steps of the README, each a test titled by its number and plain sentence, with pictures in the four themes on computer and phone; the e-mail of step 9 read from the test mailbox.

## Tasks

1. Write the acceptance tests below.
2. Sample part 9 and its share of the large dataset.
3. The ten-people runs of M10.
4. The scenarios of the benchmark; the measurement at the exit; the long run.
5. The exit demo as tests, with its pictures; the release notes of 0.6.1; the M10 part of architecture §11.

## Acceptance tests (write first)

- `iss_08_the_sample_has_releases_with_found_in_and_planned_for` and `cfg_03_the_sample_has_a_shared_dashboard_pins_and_an_e_mailed_search` (TEST-SAMPLE-DATA).
- The TEST-CONCURRENT cases above in `web/app/e2e/concurrent/views.spec.ts` and their `bb-load-check` counterparts: `plan_06_ten_people_drag_cards_and_no_status_is_lost`, `plan_06_a_move_while_the_workflow_changes_is_judged_by_the_newer_workflow`, `plan_09_a_calendar_drag_and_a_page_edit_of_one_due_date_ask_keep_or_use_mine`, `cfg_03_two_tabs_changing_one_dashboard_lose_nothing`, `iss_08_an_archived_release_stays_on_its_issues_under_load`, `qry_07_an_unshared_search_pauses_its_schedules_while_they_send`.
- The PERF-BENCH scenarios above.
- Each step of the exit demo as a test of `m10-exit-demo.spec.ts`: *"1. Ana walks a blocking chain on the relation map"*, *"2. Ben's map shows no chain through an issue he may not see"*, *"3. Ana drags a card across the board"*, *"4. Ana drags an issue across the calendar"*, *"5. A bug found in 2.4 is planned for 2.5, and the timeline warns"*, *"6. Deadline drivers show what sets the end of Mobile app 2.0"*, *"7. Ana builds a dashboard from ready-made boxes and shares it"*, *"8. Ana pins her board and her dashboard"*, *"9. Ben gets his weekly search by e-mail"*, *"10. One board covers Website and its sub-projects"*.

## Checks

TEST-SAMPLE-DATA, TEST-CONCURRENT, TEST-CONNECTIONS, PERF-BENCH, PERF-BUNDLE, META-SPEED, TEST-A11Y, TEST-VISUAL, THEME-ONLY.

## Docs to update

Architecture §11 (*Releases, boards and dashboards (M10, part 9)*, *The M10 exit demo on the sample*, the measurements on 1 M issues); the release notes of 0.6.1; `docs/compliance.md` if a measurement changes a promise.

## Review

Code review with the SSOT/DRY/ODR gate: the sample through the use cases only, one helper for each kind of wait in the browser tests (architecture §11), the budgets once in the bench budgets. A UX review of the pictures of the exit demo in the four themes, on computer and phone.

## Personal data

The sample's people are invented. The e-mail of step 9 goes only to the test mailbox of the run, which is deleted with `.work/`.

## Depends on

Waves 1 to 3 merged and WP-14 pushed. Runs alone at its measurements and the long run: no other folder builds or tests meanwhile.
