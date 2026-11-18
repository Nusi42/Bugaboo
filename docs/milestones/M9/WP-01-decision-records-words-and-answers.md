# WP-01 — Decision records, words, the UX gaps, the maintainer's answers

**Wave 0 · Folder / branch:** `bugaboo.win1` / `WIN1` · **Model:** Opus (the lead)
**Features:** PRIV-03, PRIV-04, PRIV-05, PRIV-06, OPS-04, OPS-06, OPS-07, SEC-18, COMP-01, COMP-03, COMP-04, COMP-15, DIST-03, DIST-07 (as docs). Docs only, like WP-01 of M8.

## Goal

Every answer of the maintainer to M9-Q1…Q21 is written where it belongs before any builder starts, so the package files, the decision records, the glossary, the architecture and the UX design say the same thing. The proposed decision records become accepted ones.

## Owns

`docs/adr/0044-…` to `docs/adr/0048-…` and the amendments below; `docs/glossary.md`; `docs/architecture/06-deployment.md`, `07-cross-cutting.md`, `10-checks.md`, `11-testing.md`, `12-security.md` (their M9 parts); `docs/02-ux-design.html` (§4.13, §5.1, §5.7 and the new frames); `docs/01-feature-list.md` (wording only); `docs/04-milestones.md` (the M9 exit demo, if an answer changes it); `docs/compliance.md` (the M9 state of CRA, GDPR and accessibility); the package files `docs/milestones/M9/WP-02-…` to `WP-15-…` where an answer changes them; the generated HTML pages these change (`cargo xtask gen docs`).

## What is decided

- Nothing new: this package writes down what the maintainer decides. A different answer from a recommendation changes the record and the package files first; the package's `## What is decided` sections are rewritten in the same commit.
- Pictures before questions: M9-Q1, M9-Q3, M9-Q4, M9-Q10, M9-Q12 and M9-Q15 are asked with pictures (the rule *questions need screenshots*). The screens do not exist yet, so they are drawn as small mocks in the style of the UX design page, with its tokens and words, light and dark, on one question page kept outside the repository; the warning Windows shows for an unsigned program is described in words with a plain mock, no real branding.

## Tasks

1. Ask M9-Q1…Q21 on the review page of the plan (commentable, link outside the repository), with the mocks of M9-Q1, M9-Q3, M9-Q4, M9-Q10, M9-Q12 and M9-Q15.
2. Write each answer under its question in the README of M9 (*Answer (date): …*), and where it belongs.
3. ADR-0044 to ADR-0048: status *Accepted*, changed by the answers. Amendments: ADR-0015 (the links of a person beside the marks), ADR-0017 (the kinds of M9: `privacy.export`, `privacy.finish-removal`, `backups.daily`, `support-access.end`, and the one shape of deleting in steps), ADR-0022 (the tombstone in full, *Switch off*, support accounts in `PersonRef`), ADR-0035 decision 7 (time entries under ADR-0045), ADR-0012 and ADR-0037 (releases built and signed on this computer), ADR-0025 (the new permissions *Download someone's data* and *Remove people*, and what no key or agent holds).
4. The glossary: the words of M9 that are not there yet, and the changed meaning of *Keep and delete rules* (two groups, M9-Q1). Each word with what not to say, checked against the message catalogue and the API first (TXT-TERMS).
5. Architecture: §6 (backups, restore, upgrades, the release folder, the service run on clean systems), §7 (*Privacy engineering*: the links and the walk; the keep list; *Compliance artifacts* with the parts list made by xtask), §10 (TEST-UPGRADE, the grown DEP-SBOM, TEST-CONCURRENT and DATA-MARKS, each as "Adding a rule" asks), §11 (*The upgrade test*, *The long run*, the M9 exit demo on the sample), §12 (the M9 part of the threat model; *Personal data in M9*: exports, the replay list, the legal-hold name, backups, e-mail changes, support accounts; boundary 4 with support access; the accepted risk of an unsigned program on Windows).
6. The UX gaps, as frames of the UX design page: *My settings › My data* and *Profile*; *Organization settings › Privacy › Keep and delete rules* (two groups) and *Data exports*; *Switch off* and the window *Remove a person* on a person's page; *System › Backups* with the window *Restore this backup* and the page *Bugaboo is being restored*; *About*; *Support access* with its one-time link. Each frame names its words; each follows the ten rules of §1.1 and §6.4.
7. The feature list: PRIV-05 (two groups), OPS-07 (as M9-Q10 and M9-Q20 answer), DIST-07 (as M9-Q14 answers), and DIST-03 (V1 if M9-Q20 moves it to M14).
8. `compliance.md`: what M9 meets of the CRA, the GDPR and accessibility, and the open points that stay for the legal review.

## Acceptance tests (write first)

Docs only: no code test. The checks of a docs change are the tests: PLAN-TRACE (every feature of M9 owned once), DOC-ADR-ENFORCED and META-RULE-TRACE (every rule of the records and chapters names its check), DOC-GENERATED (the HTML pages), TXT-TERMS (the new words), LINT-LINKS, LINT-SPELL, FMT-MD.

## Checks

`cargo xtask check staged`, PLAN-TRACE, DOC-ADR-ENFORCED, META-RULE-TRACE, DOC-GENERATED, TXT-TERMS, LINT-LINKS, LINT-SPELL, FMT-MD.

## Review

A reviewer with a fresh context reads the changed docs against the answers and this plan, applying the SSOT/DRY/ODR gate to the docs (one home for each rule, the others point to it). A docs-only round is read before the merge.

## Depends on

The maintainer's answers. Runs alone; every other package waits for it.
