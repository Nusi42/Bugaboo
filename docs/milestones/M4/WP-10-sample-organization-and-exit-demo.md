# WP-10 — Sample organization part 3, ten people with workflows, speed, the exit demo

**Wave 4 · Folder / branch:** `bugaboo.win2` / `WIN2`
**Features:** WF-01, ATT-01, NTF-01 (on realistic data); architecture §11 (sample organization, ten people, speed); follow-ups M3-F93, M3-F111, M3-F113, M3-F121.

## Goal

The sample organization uses workflows, files and notifications like a real team. Ten people at once still break no rule. The exit demo of M4 runs on it, on a computer and a phone.

## Owns

- `crates/bb-sample/**` and `tools/dev/sample.json`.
- The scenarios of `bb-load-check` and the ten-people run.
- The speed measurements of the sample.
- `web/app/e2e/m4-exit-demo.spec.ts` (new).
- The testing chapter's M4 demo section.

## Tasks

1. **The sample grows through the use cases** (never by writing rows directly):
   - the starter workflows for each issue type, and one project with its own workflow;
   - about 500 small generated files (images and PDFs) with previews;
   - notifications from the sample's own changes.
2. **Everybody works (M3-F111):** Contributors, Guests and Viewers do their part. Guests report and comment on their own issues; Contributors change their own; Viewers only read and follow. So workflows by role are real.
3. **Ten people at once:**
   - the scenarios change statuses under workflows with *Ask for*, add files, and read notifications;
   - the invariants include the title (M3-F93) and "no notification shows what its person may not see".
4. **Speed:**
   - the issue page with files, the notifications page, and a status change through the judge, against the budgets (PERF-01), in a release build;
   - one batch read of the authors on a page (M3-F113), if the notifications page shows it is needed.
5. **Jobs for an access key (M3-F121):** decided by the lead: not in M4. A key keeps getting `auth.access_key_not_allowed` for a change over 1,000 issues; the follow-up moved to M5 (automation) in `docs/milestones/M3/follow-ups.md`.
6. **The exit demo** (04-milestones, M4):
   - move a bug from Open to Done through its workflow, with *Ask for*;
   - paste a screenshot onto it;
   - see the notification and the e-mail (the fake mail server) arrive for a follower, without fields that person may not see;
   - on a computer and a phone, four themes;
   - the builder checks the screenshots against UX §1.1.

## Acceptance tests (write first)

- TEST-SAMPLE-DATA: the sample is the same on both databases and holds workflows, files and notifications.
- The ten-people run: every invariant holds on both databases.
- `m4-exit-demo.spec.ts` green on desktop and phone.

## Checks

TEST-SAMPLE-DATA and TEST-CONCURRENT grow with the new data and scenarios.

## Depends on

Everything else in M4.
