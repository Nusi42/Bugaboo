# WP-10 — Sample organization part 4, searching under load, speed on 1 M issues, the exit demo

**Wave 4 · Folder / branch:** `bugaboo.win2` / `WIN2`
**Features:** PERF-01 (met), PERF-05, QRY-06 (in the sample). Architecture §11 ("Sample organization", "Ten people at the same time", "Large dataset"); follow-ups M3-F133, M4-F137, M4-F82.

## Goal

M5 is shown working on the sample and proven fast on 1 M issues: the exit demo runs, ten people search while others change data and every rule holds, and the nightly benchmarks meet PERF-01 (the groups by *Assigned to* on SQLite are a known gap, M5-F89, accepted by the maintainer).

## Owns

`tools/dev/sample.json` and `bb-sample` (saved searches), the ten-people run (`bb-load-check`, its scenarios), the M5 exit demo spec and its section in architecture §11, the user docs of search, lists and *Quick search*.

## Tasks

1. **Sample part 4:** saved searches for every role, private and shared with groups and projects, one with a field some roles may not read (so a receiver who may not read it never sees it), and a project default list.
2. **Ten people searching:** the scenario "Searches while others change data" (architecture §11): every search answer holds the rules (no hidden issue, field or person; counts match the issues seen), on both databases.
3. **Speed on 1 M issues:** every PERF-01 scenario of WP-03 to WP-09 within its budget on both databases in the nightly run; the preview of 5,000 issues gets a budget and meets it (M3-F133); the fixed statements of a request are measured and the largest cut (M4-F137). Numbers into §11.
4. **The exit demo** (04-milestones M5): search the sample in both modes and switch; a Contributor who may not see *Customer* (an own field; *Cost center* is a built-in field no search can read yet, so it is not the one, lead) searches by it and gets the answer of an unknown field; the run on 1 M issues meets its budget. Each step a test, with pictures in the four themes on computer and phone.
5. **User docs:** searching, the search language with examples, saved searches and sharing, lists, *Quick search*; numbers taken from the code, not written by hand (M4-F82).

## Acceptance tests (write first)

- The sample's rules: a guest's searches never show what they may not see; every shared search a receiver sees checks cleanly for them.
- The ten-people run: "every rule held" with searches in the mix, on both databases.
- PERF-BENCH: the nightly run green on both databases.
- The exit demo spec, step by step.

## Depends on

WP-07, WP-08 and WP-09 merged.
