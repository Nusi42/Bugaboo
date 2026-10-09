# WP-11 — Sample organization, part 2, speed, and the exit demo

**Wave 4 · Folder / branch:** `bugaboo.win2` / `WIN2`
**Features:** all of M3 (demo); PERF-01, PERF-05 for issue pages; architecture §11 (*Sample organization*); follow-ups M1-F23, M2-F26, M2-F110.

## Goal

The sample organization fills with about 10,000 realistic issues, so every M3 screen is shown, measured and demonstrated on real-looking data, and the exit demo runs from start to end on it.

## Owns

`crates/bb-sample/src/{issues,fields,relations,comments}*.rs` (new) through the `Later` extension point, `tools/dev/sample.json` (issue settings: types, own fields, ID modes), the TEST-SAMPLE-DATA additions and fixtures, `web/app/e2e/m3-exit-demo.spec.ts` (new), M3 screenshot specs and the showcase screens for issues, performance tests for issue pages.

## Tasks

1. **Part 2 (M3-Q10):** per architecture §11: APP random IDs (about 4,800 issues with IOS and AND), WEB counting (about 3,000 with SHOP and BLOG), OPS random (about 1,800), DOC counting (about 400); 18 months; Bug 45 %, Task 30 %, Feature 20 %, Goal 5 %; own fields *Severity*, *Platforms*, *Customer*; sub-issues under goals; about 3,000 relations of every kind, chains deeper than 4 and one deliberate *blocks* circle; about 30,000 comments with mentions, some edited; tags and labels; one edit lock held and one draft waiting. All through use cases, deterministic from the seed, no restricted issues and no releases.
2. **Speed:** issue page and issue list answer p95 under 100 ms on the big sample (PERF-01), no N+1 queries (PERF-05); measure the outbox counter (M1-F23) and the whole-tree reads (M2-F26) under ten writers and fix what misses the budget.
3. **Exit demo (04-milestones, M2-F110):** a Guest makes a bug with the short form; a Team member makes one with the full form (with *Cost center*, hidden from the Guest, who sees *Someone* for the person assigned); relate it to two issues; edit the description in two browsers (lock, *Edit anyway*, *Compare and save*). Screenshots of every step, four themes, computer and phone.
4. **Showcase:** add the issue screens to `screenshots-showcase.spec.ts`.

## Acceptance tests (write first)

- `test_sample_data_issues_match_the_plan` (counts per project and type within a tolerance, both databases, `--check`).
- `perf_01_issue_page_and_list_stay_under_100_ms_p95_on_the_sample`.
- `m3-exit-demo.spec.ts` as written above.

## Checks

TEST-SAMPLE-DATA: issues, own fields, relations and comments.

## Depends on

Every M3 package.
