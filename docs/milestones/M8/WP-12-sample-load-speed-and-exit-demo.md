# WP-12 — Sample part 7, the timeline under load, speed, the exit demo

**Wave 4 · Folder / branch:** `bugaboo.win2` / `WIN2`
**Features:** PLAN-01, PLAN-11, PLAN-12, PLAN-13, PLAN-14, PLAN-15, THM-03, THM-04 (proved end to end). The exit demo of [04-milestones](../../04-milestones.md) (M8) with the answer to M8-Q3; architecture §11.

## Goal

The sample organization plans with people, ten people can plan at once without losing a date, the timeline of the biggest project on 1 M issues answers within its budget, and the exit demo runs step by step as tests, with pictures in the four themes on computer and phone.

## Owns

Sample part 7 in `crates/bb-sample/src/` (the plan of the *Mobile app* goal: dated issues, dependencies, Lea's work times, away times, shares, a time factor) and its share of the large dataset (`large_*.rs`); the ten-people runs of planning in `crates/bb-load-check/src/` and `web/app/e2e/concurrent/`; the timeline scenario of `cargo xtask bench` and its budget (`xtask/src/bench*.rs`); `web/app/e2e/m8-exit-demo.spec.ts`; the M8 part of architecture §11.

## What is decided

- **Sample part 7:** Lea Fischer and two colleagues in *Mobile app*; the goal *Mobile app 2.0* with a due date; its issues with start and due dates, estimates and dependencies of *blocks*; Lea's usual work days and work hours, a share of 100 % at first, holidays of one German state, no away time yet; one issue with Lea's own estimate. Made through the use cases, like every part of the sample (TEST-SAMPLE-DATA).
- **Ten people at once** (TEST-CONCURRENT): two people drag the same bar; one moves a bar while another accepts its warning; a person enters an away time while the plan is read; *Move it after* while someone changes an issue it moves. No date is lost, a stale save gets *Keep Ben's / Use mine*, and every run is undone whole.
- **Speed** (PERF-BENCH): the timeline route for the biggest project of the large dataset (1 M issues), a page of rows, with and without *Plan by effort*, p95 under 300 ms (PERF-01); the timeline chunk and the editor chunk within PERF-04. Measured by hand on a quiet computer at the exit, when the lead says nothing else builds or tests.
- **The exit demo** (04-milestones M8, with M8-Q3): plan *Mobile app 2.0* by dragging; enter a week away for Lea and see her bars grow around it and the warning *"Lea's away week moves Login screen past the due date of Mobile app 2.0"*; set Lea's share to 50 % and see her bars double; as Lea, say an issue *Takes me 10 days* and see *Lea's own estimate*; give it to someone else and see its time factor go back to ×1; create an own theme for everyone that fails readability and fix it. Each step is a test in `web/app/e2e/m8-exit-demo.spec.ts`, with pictures in the four themes on computer and phone.

## Tasks

1. Write the acceptance tests below.
2. Sample part 7 and its share of the large dataset.
3. The ten-people runs of planning.
4. The timeline scenario of the benchmark and its budget; the measurement at the exit.
5. The exit demo as tests, with its pictures; the M8 part of architecture §11.

## Acceptance tests (write first)

- `plan_12_the_sample_plans_mobile_app_2_0_with_lea` (the sample's check, TEST-SAMPLE-DATA).
- The TEST-CONCURRENT cases above in `web/app/e2e/concurrent/planning.spec.ts`, named like M7's (`ai_30_ten_people_…`): `plan_03_ten_people_move_bars_at_the_same_moments_and_no_date_is_lost`, `plan_13_a_warning_accepted_while_its_bar_moves_comes_back`, `plan_11_an_away_time_entered_while_the_plan_is_read_shows_on_the_next_answer`, `plan_13_move_it_after_while_someone_changes_an_issue_it_moves_is_refused_whole`, and their `bb-load-check` counterparts.
- The PERF-BENCH scenario `timeline-biggest-project` (with and without *Plan by effort*).
- Each step of the exit demo as a test of `m8-exit-demo.spec.ts`, titled like M7's by its number and a plain sentence: *"1. Mobile app 2.0 is planned by dragging"*, *"2. Lea's away week grows her bars and warns about the goal's due date"*, *"3. A share of 50 % doubles Lea's bars"*, *"4. Lea says an issue takes her 10 days and it shows as her own estimate"*, *"5. Given to someone else, the time factor goes back to ×1"*, *"6. An own theme that is hard to read is fixed and saved for everyone"*.

## Checks

TEST-SAMPLE-DATA, TEST-CONCURRENT, TEST-CONNECTIONS, PERF-BENCH, PERF-BUNDLE, META-SPEED, TEST-A11Y, THEME-ONLY.

## Review

Code review with the SSOT/DRY/ODR gate: the sample through the use cases only, one helper for each kind of wait in the browser tests (architecture §11), the budget once in the bench budgets. A **UX review** of the pictures of the exit demo in the four themes, on computer and phone.

## Depends on

Waves 1 to 3 merged. Runs alone at its measurement: no other folder builds or tests while the benchmark runs.
