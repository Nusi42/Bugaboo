# WP-14 — Visual tests and the accessibility of the new views

**Wave 4 · Folder / branch:** `bugaboo.win2` / `WIN2` · **Model:** Opus (a new check that must be stable across runs, the accessibility of four new screens)
**Features:** PLAN-06, PLAN-09, ISS-16 and CFG-03 checked (owned by WP-10 to WP-13): all views of *Show as* (List, Board, Timeline, Calendar, Map) and dashboards, in the four themes. The roadmap's "visual tests of all views in four themes"; answer M10-Q19.

## Goal

Every view of *Show as* and a dashboard has reference pictures in the four themes, on a computer and on a phone, and every full run compares the screen with them. A changed picture fails the run until a person looks at it and takes it. Every drag of the new screens has its key, and every new screen passes the accessibility measures of M9 in the four themes.

## Where

- **Owns:** `web/app/e2e/visual/**` (new: the suite, the fixed data and clock, the reference pictures), `xtask/src/checks/test_visual.rs` (new), `xtask/src/visual.rs` (new: `cargo xtask visual update`), the catalogue entry of TEST-VISUAL and its examples in `tools/checks/fixtures/TEST-VISUAL/`, the growth of TEST-A11Y (`xtask/src/checks/` and `web/app/e2e/a11y*.spec.ts`: the board, the calendar, the map, dashboards, *Releases*, *Pinned*, the window *E-mail me this search*), architecture §11 (*Visual tests*) and §10, the EN 301 549 report's rows that the new screens touch (`docs/accessibility/en-301-549.md`).
- **Fixes:** findings of its measures in the screens of WP-07 and WP-09 to WP-13, each as its own commit with its test; a finding that needs more than a small fix becomes a follow-up row and is told to the lead.
- **Shared files:** none in wave 4 except the screens it fixes.

## What is decided

- **What is pictured** (M10-Q19): List, Board, Timeline, Calendar (month), Map and a dashboard with four ready-made boxes, each on a fixed search of the sample organization, in the four built-in themes, on a computer (1280 × 800) and a phone (390 × 844): 48 pictures.
- **How they stay stable:** a fixed clock (the day of the sample, 10:00 in Berlin), the sample made fresh from its seed, motion switched off (`prefers-reduced-motion`), the caret hidden, fonts of the Windows run only; the comparison allows a small share of changed pixels per picture (set by measuring three runs in a row; written once in the suite's settings), and fails above it with a picture of the difference in `.work/`.
- **Where they run:** in `cargo xtask ci` on the Windows run only; WSL skips the suite and says so (fonts differ). The pictures live in Git under `web/app/e2e/visual/`, all together at most 10 MB, checked by TEST-VISUAL.
- **Taking a change:** `cargo xtask visual update` writes new references for the pictures that failed; the change is reviewed like code, with the before and after named in the report.
- **TEST-A11Y grows:** the list of drag kinds (`drag-kinds.ts`, shared since WP-10/WP-11) names every drag of the board, the calendar and a dashboard with its typed way, and the check fails a drag without one; axe and the M9 measures (reflow, zoom, text spacing, targets) run on every new address of `paths.ts`.
- The rule is written by "Adding a rule" (architecture §10): the rule in architecture §11 with `Enforced by: TEST-VISUAL`, the catalogue entry, the code, the examples.

## Tasks

1. Write the suite and its fixed data and clock; measure three runs in a row; set the allowed share.
2. TEST-VISUAL: the check, the catalogue entry, the examples, the size limit, `cargo xtask visual update`.
3. TEST-A11Y grows; the measures on every new address.
4. Fix what the measures find, each with its test; follow-up rows for the rest.
5. Architecture §10 and §11; the EN 301 549 report's rows.

## Acceptance tests (write first)

- The suite itself: one test per view, theme and size, named `ux_visual_<view>_<theme>_<computer|phone>`.
- `test_visual_fails_a_view_kind_without_its_pictures` and `test_visual_fails_when_the_pictures_exceed_10_mb` (the check's examples).
- `test_visual_three_runs_in_a_row_give_the_same_pictures` (run once by hand, its result in the report and in architecture §11).
- TEST-A11Y: `a11y_every_drag_of_the_board_calendar_and_dashboards_has_its_key`, and axe with the measures on every new address in the four themes.

## Checks

TEST-VISUAL (new), TEST-A11Y (grown), META-CATALOG, META-FIXTURES, META-RULE-TRACE, META-WIRING, GIT-FILES (the pictures' size), THEME-ONLY.

## Docs to update

Architecture §10 (TEST-VISUAL, TEST-A11Y grown) and §11 (*Visual tests*); UX design §1.1 points to the check; `docs/accessibility/en-301-549.md`.

## Review

Code review with the SSOT/DRY/ODR gate: the view kinds read from the API description, the allowed share written once, one list of drag kinds. A UX review of all 48 reference pictures against UX design §1.1 and §7 before they are taken.

## Personal data

None: the pictures show the sample organization's invented people only.

## Depends on

Waves 1 to 3 merged. WP-15's measurements wait for this package's last push.
