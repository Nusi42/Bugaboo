# WP-12 — Sample organization part 5, timers under load, speed of the time pages, the exit demo

**Wave 4 · Folder / branch:** `bugaboo.win2` / `WIN2`
**Features:** TIME-01, TIME-08 (under load), TIME-04 and TIME-03 (speed), PERF-01. Architecture §11 (*Sample organization*, *Ten people at the same time*, *Large dataset*).

## Goal

M6 is shown working on the sample and proven under load and on 1 M issues: the exit demo runs, ten people start and stop timers and work days at the same moments and every rule holds, and the timesheet and the totals of a project meet PERF-01.

## Owns

`tools/dev/sample.json` and `bb-sample` (time and working hours), `dataset large` (time entries and work days), the ten-people scenarios of timers and work days (`bb-load-check`, the browser scenarios), the M6 exit demo spec and its section in architecture §11, the user docs of time and working hours (with numbers taken from the code).

## Tasks

1. **Sample part 5** (the outline's "part 3"; M5 made part 4): six months of working hours for every employee (some days that break a rule, a day changed with a reason by the person and one by *HR*), about 20,000 time entries with *Activity* and *Cost center*, estimates on a third of the issues, a few running timers, a group *Payroll* that holds *HR*, and *Hana* in it. The sample's own rules (TEST-SAMPLE-DATA) check that every day and entry passes the server's rules of shape and that no time entry is on an issue its person cannot see.
2. **Ten people with timers** (architecture §11, "Timers and working hours start and stop at the same time"): in the load run on both databases and in the browser scenarios on SQLite; `bb-load-check` judges: one timer per person, no entry twice, every warning evaluated once per change, every change with its event.
3. **Speed on 1 M issues:** `dataset large` adds time entries (two per issue on average) and work days for its 500 people (the data is made by the code that is measured, M5 lesson); the scenarios `timesheet-week`, `project-time-totals` and `issue-time-entries` within PERF-01 on both databases in the nightly run; numbers into §11.
4. **The exit demo** (04-milestones M6, with the steps the maintainer takes in M6-Q12): Ana starts her work day, runs a timer, stops it (the window shows *Activity* and *Cost center* from the issue), takes a break and ends the day; the timesheet shows a rule warning on a long day; Ana changes yesterday's end and gives a reason; Ben cannot see Ana's working hours; Hana sees them, and with *Totals only* on sees only the week total; the four themes look as before the theme files. Each step a test, with pictures in the four themes on computer and phone.
5. **User docs:** the timer, time entries, *Activity* and *Cost center*, estimates and totals, the timesheet, working hours and the rules (with the Germany numbers taken from the preset in the code), who sees working hours and why (PRIV-09), how long they are kept.

## Acceptance tests (write first)

- TEST-SAMPLE-DATA: the rules of part 5.
- TEST-CONCURRENT: "every rule held" with timers and work days in the mix, on both databases.
- PERF-BENCH: the nightly run green on both databases with the three new scenarios.
- The exit demo spec, step by step (`web/app/e2e/m6-exit-demo.spec.ts`).

## Checks

TEST-SAMPLE-DATA, TEST-CONCURRENT, PERF-BENCH, TEST-A11Y.

## Review

The SSOT/DRY/ODR gate of coding-standards §1: the sample through the use cases, the large dataset through the one bulk writer of M5 WP-03, the numbers in the user docs read from the code. A UX reviewer looks at the demo pictures. Then the exit visual pass of the milestone (README).

## Depends on

WP-09, WP-10 and WP-11 merged.
