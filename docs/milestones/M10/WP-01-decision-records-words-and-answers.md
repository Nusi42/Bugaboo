# WP-01 — Decision records, words, the UX gaps, the maintainer's answers

**Wave 0 · Folder / branch:** `bugaboo.win1` / `WIN1` · **Model:** Opus (the lead)
**Features:** PLAN-05, PLAN-06, PLAN-09, ISS-16, ISS-08, CFG-03, CFG-05, PRJ-06, QRY-07, QRY-08 (as docs). Docs only, like WP-01 of M8 and M9.

## Goal

Every answer of the maintainer to M10-Q1…Q22 is written where it belongs before any builder starts, so the package files, the decision records, the glossary, the architecture and the UX design say the same thing. The two new decision records are accepted, and the frames that the UX design does not draw yet are drawn.

## Owns

`docs/adr/0049-releases.md` and `docs/adr/0050-dashboards-and-totals.md` (new) and the amendments below; `docs/glossary.md`; `docs/architecture/07-cross-cutting.md` (§8, the release words of the search language and the map's walk, as decisions; the code's details are written by the packages), `docs/architecture/12-security.md` (the plan of *Board, calendar, map and dashboards (M10)* and *Personal data in M10*); `docs/02-ux-design.html` (§2, §3.1, §3.3, §4.1, §4.6, §4.7, §4.10, §4.15, §4.16, the new frames); `docs/01-feature-list.md` (wording only: CFG-03, CFG-05, QRY-08, PRJ-06, and ISS-08 if the answers change it); `docs/04-milestones.md` (the M10 outline and exit demo, if an answer changes them; a cut of M10-Q21); the package files `WP-02-…` to `WP-15-…` where an answer changes them; the generated HTML pages these change (`cargo xtask gen docs`).

## What is decided

- Nothing new: this package writes down what the maintainer decides. An answer that differs from a recommendation changes the record and the package files first, in the same commit.
- Pictures before questions: M10-Q1 (columns across workflows, a faded column with its reason), Q4 (a card with four fields), Q9 (the outline of deadline drivers, the warnings *Loop* and *Cannot fit*), Q10 (the map in columns by steps away), Q13 (the ready-made boxes in *Add box*) and Q14 (*Home* as a dashboard) are asked with small mocks in the style of the UX design page, with its tokens and words, light and dark, on one question page kept outside the repository.
- Words are checked against the message catalogue and the API before they are added (TXT-TERMS).

## Tasks

1. Ask M10-Q1…Q22 on the question page (commentable, link outside the repository), with the mocks above.
2. Write each answer under its question in the README of M10 (*Answer (date): …*) and where it belongs.
3. **ADR-0049 *Releases*:** where a release lives (Q6), its fields, states and dates, the permission *Manage releases*, deletion and archiving (Q7), releases on the timeline, the calendar and *Home* (Q8), the meaning of the search words `found in release` and `planned for release`, what AI apps and agents may do (Q20).
4. **ADR-0050 *Dashboards and totals*:** what a dashboard and a box are, sharing and *Share dashboards*, the box left out for a viewer (Q12), the kinds of box and the ready-made boxes, no ranking of people (Q13), *Home* (Q14), what a total adds up and over what (Q15), *Save a copy*, removal of the maker.
5. **Amendments:** ADR-0034 (the view kinds *board*, *calendar*, *map* and their options; the choice *Only this project* as search text, Q18; a box as a reader of a search), ADR-0027 (the layout *Card*, Q4), ADR-0043 (deadline drivers over the issues the reader sees, the warnings *Loop* and *Cannot fit*, releases as milestones and *Delay* for a release; Q8, Q9), ADR-0009 (the map's own layout; d3-force out, Q10), ADR-0033 (e-mailed searches, Q16), ADR-0025 (the permissions *Manage releases* and *Share dashboards*), ADR-0005 (the release words, the map's 5 steps as the one limit, Q11), ADR-0045 (the keep kinds of dashboards, pins and schedules), ADR-0044 (what *Switch off* and *Remove a person* do with them).
6. **The glossary:** the words of M10 that are missing, each with what not to say. Proposed: *Release date*, *Released on*, the states *Planned*, *Released*, *Archived* (of a release), *Manage releases*, *Card* (on a board), *Move to …*, *Change due date …*, *Kinds shown* (the map's chips; not *Follow*), *Put in the middle*, *Loop* and *Cannot fit* (plan warnings), *Box* and *Add box* (on a dashboard; not *widget*), *Ready-made box*, *Share dashboards*, *Pin to sidebar*, *Pinned*, *E-mail me this search*, *E-mailed search*, *Totals*, *Only this project*. *Small timeline* replaces "Gantt snippet". *Milestone* loses "Releases join it in M10" and names releases.
7. **The UX gaps,** as frames of the UX design page, each with its words and following the ten rules of §1.1 and §6.4: the board's *Customize* (columns, rows, card limits) and a faded column with its reason; the board on a phone with column tabs; the calendar's week view and its *Dates from*; the project page *Releases* and the release window; the two release fields on the issue page; the switch *Deadline drivers* with its outline and the two new warnings on the timeline; the map in columns by steps away; *Dashboards* (the list), a dashboard while it is changed, *Add box* with previews, a box's menu (*Move*, *Resize*, *Remove*); *Home* as a dashboard (if Q14 takes it); the section *Pinned* on a computer and in *More* on a phone; the window *E-mail me this search* and its list in *Notification settings*; the totals row of a grouped list and the totals in a board column's header.
8. **The architecture:** §12 *Board, calendar, map and dashboards (M10)* (the threats: a chain through a hidden issue, a total over hidden values, a shared box, e-mail as data leaving, a column of another workflow) and *Personal data in M10* (dashboards, pins, schedules, releases); §8 the decisions of the walk and the release words. The code's details are added by the packages.
9. **The feature list:** CFG-03 (boxes, not widgets; *Small timeline*; *Recent changes*), CFG-05 (documents with M11), QRY-08 ("reports" as Q15 answers), PRJ-06 (as Q18 answers).

## Acceptance tests (write first)

Docs only: no code test. The checks of a docs change are the tests: PLAN-TRACE (every feature of M10 owned once), DOC-ADR-ENFORCED and META-RULE-TRACE (every rule of the records and chapters names its check), DOC-GENERATED (the HTML pages), TXT-TERMS (the new words), LINT-LINKS, LINT-SPELL, FMT-MD.

## Checks

`cargo xtask check staged`, PLAN-TRACE, DOC-ADR-ENFORCED, META-RULE-TRACE, DOC-GENERATED, TXT-TERMS, LINT-LINKS, LINT-SPELL, FMT-MD.

## Review

A reviewer with a fresh context reads the changed docs against the answers and this plan, applying the SSOT/DRY/ODR gate to the docs (one home for each rule, the others point to it). A docs-only round is read before the merge.

## Personal data

The records name what M10 stores about people: who made a dashboard and shared it, a person's pins and e-mail schedules, who changed a release. *Personal data in M10* says how each is kept and removed.

## Depends on

The maintainer's answers. Runs alone; every other package waits for it.
