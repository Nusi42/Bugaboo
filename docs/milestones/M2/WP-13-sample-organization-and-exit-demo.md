# WP-13 — Sample organization, part 1, and the exit demo

**Wave 5 · Folder / branch:** `bugaboo.win4` / `WIN4`
**Features:** the M2 exit demo on realistic data — SEC-02, SEC-03, SEC-04, PRIV-12, THM-01, UX-01; architecture §11; ADR-0023.

## Goal

The sample organization holds the people, groups, roles and projects of architecture §11, made through the use cases of M2, and the M2 exit demo runs end to end on it in Playwright, on a phone and in all four themes. The screenshots for the M2 review page come out of the same run.

## Owns

`crates/bb-sample/**` (extends WP-06), `xtask/src/checks/test_sample_data.rs` and its fixtures, `web/app/e2e/m2-exit-demo.spec.ts` (new), the M2 screenshot set (`web/app/e2e/screenshots.spec.ts` lines for the M2 screens), the *Sample organization* section of `docs/architecture/11-testing.md`.

## Tasks

1. **Groups and roles:** the groups of §11 (Backend, Frontend, Mobile, QA, Support, Docs, Management, External) with their members; the role mix recorded by WP-06 given as assignments on the whole organization or on projects, a few through groups, one *No access* on a sub-project.
2. **Projects:** Mobile app (`APP`, with iOS `IOS` and Android `AND`), Website (`WEB`, with Shop `SHOP` and Blog `BLOG`), Operations (`OPS`), Handbook (`DOC`); a project lead for each; *Who sees people* for Guests and *Cost center* hidden from Contributors on `APP`, while `WEB` keeps the starting rules so the exit demo has something to change.
3. **People's settings and keys:** time zones and regions spread over the people; a few access keys (their secrets in the ignored file of WP-06); a few people whose role requires two-step sign-in, with a code app set up.
4. **Ana:** placed so that the demo's changes show in her *Check access* (a Contributor on `WEB` through a group), and named in the demo steps of the review page.
5. **TEST-SAMPLE-DATA, projects, groups and roles:** the canonical dump covers them; the check also fails when a role of the §11 mix is missing or a project has no lead.
6. **Exit demo (04-milestones, M2):** on a fresh sample organization, an administrator signs in, makes a role, hides *Cost center* from Contributors and names from Guests on `WEB`, checks Ana's access and reads the explanation; then the same screens on a phone and in all four themes, with axe on every step. The steps are written so that the maintainer can repeat them by hand with `cargo xtask dev --sample`.
7. **Screenshots:** every screen changed in M2, four themes × computer and phone, into the review-page folder (`<work root>/screenshots/M2`).

## Acceptance tests (write first)

- `the_sample_holds_the_groups_roles_and_projects_of_architecture_11` and the determinism test of WP-06 extended to them (both databases).
- `the_sample_shows_the_role_mix_through_check_access` (every role of the mix is held by the right number of people).
- `m2-exit-demo.spec.ts` passes on SQLite in `cargo xtask ci`.
- TEST-SAMPLE-DATA fails on its new bad fixture (a project without a lead) and passes on the good one.

## Checks

TEST-SAMPLE-DATA extended to projects, groups and roles (fixtures grow). TEST-A11Y runs on the demo steps.

## Depends on

WP-06 (the tool), WP-08 (projects), WP-09 (access keys, two-step by role), WP-11 and WP-12 (screens and use cases used by the demo).
