# WP-14 — The accessibility pass, the EN 301 549 report and the statement

**Wave 4 · Folder / branch:** `bugaboo.win4` / `WIN4` · **Model:** Opus (judgement against the standard)
**Features:** COMP-15. UX-04; UX design §8; compliance.md §7.

## Goal

Every screen of the MVP meets WCAG 2.2 AA as far as a machine and a keyboard can tell, every finding is fixed or written down with a follow-up, and two documents say where Bugaboo stands: the EN 301 549 report, criterion by criterion, and an accessibility statement template an organization can publish.

## Owns

The grown TEST-A11Y in `web/app/e2e/a11y*.spec.ts` and `xtask/src/checks/test_a11y.rs` (the measures below for every address of `paths.ts`); fixes in the screens it finds wanting (each fix in the feature's own folder, one commit per finding, no new component where one exists); `docs/accessibility/en-301-549.md` (new: the report) and `userDocs/accessibility-statement.md` (new: the template); the checklist of the screen-reader pass `docs/accessibility/screen-reader-checklist.md` (new); the follow-ups of what stays open.

## What is decided

- **Measured on every address**, in the four themes, on computer and phone: axe with the WCAG 2.2 AA rules (as today); reflow at 320 CSS pixels wide with no sideways scroll except the timeline and the lists that are tables (WCAG 1.4.10 names them); zoom to 200 % with nothing cut off; text spacing (1.4.12: line height 1.5, letter spacing 0.12, word spacing 0.16, paragraph spacing 2 times) with nothing cut off or overlapping; forced colours (as today); focus visible and not hidden by a sticky part (2.4.11, 2.4.13 checked as AA's 2.4.11 only); targets of 24 pixels at least (2.5.8; 44 with touch, as M8); a keyboard walk of each screen's main task with no trap (2.1.2) and the order of the reading; motion off under *reduce motion*.
- **What a machine cannot tell** is checked by reading: names and roles of the timeline and the windows, the words of errors (3.3.1, 3.3.3), the status messages announced (4.1.3), the help in the same place (3.2.6), redundant entry (3.3.7), accessible sign-in without a test of memory (3.3.8: passkeys and paste allowed). Each criterion's result goes into the report with its evidence (a test name or a reason).
- **The report** follows EN 301 549 V3.2.1 clause by clause for software and web content (clauses 5, 9, 11 and 12): *Supports*, *Partly supports* with what is missing, *Not applicable* with why. Hardware and real-time text clauses are *Not applicable*.
- **The statement template**: the parts the EU model statement asks for (what conforms, what does not and why, how to give feedback, the date of the review), with gaps marked where the organization fills in its own contact and enforcement body.
- **The screen-reader pass** (M9-Q18): the checklist lists the tasks (sign in, the issue list, an issue and its comments, *New issue*, the timeline with its typed ways, a settings page, *My data*); the maintainer runs it with NVDA or Narrator, about an hour; until it ran, the report says *Partly supports* where only a screen reader can tell.

## Tasks

1. Write the acceptance tests below.
2. The measures in TEST-A11Y for every address.
3. Fix what fails, one commit per finding; write down what cannot be fixed in M9 as a follow-up with its reason.
4. The reading of the criteria a machine cannot tell.
5. The report, the statement template and the checklist.

## Acceptance tests (write first)

- `comp_15 every address reflows at 320 px without sideways scroll` (the named exceptions only).
- `comp_15 every address keeps its text at 200 % zoom and with wide text spacing`.
- `comp_15 every target is at least 24 px` (and 44 with touch).
- `comp_15 the main task of each screen works with the keyboard alone and has no trap`.
- `comp_15 status messages are announced` (the live regions of saves, refusals and the export's state).
- `comp_15_every_criterion_of_the_report_has_evidence_or_a_reason` (xtask: the report's rows against the list of clauses kept as test data).

## Checks

TEST-A11Y (grown), STY-CONTRAST, THEME-ONLY, TXT-LITERAL, TXT-CATALOG, LINT-LINKS, FMT-MD.

## Review

Code review with the SSOT/DRY/ODR gate on each fix. **UX review** of the pictures of every fixed screen in four themes, computer and phone, and frame samples where a fix changes something that moves. A reviewer reads the report against the tests it cites.

## Depends on

Waves 1 to 3 merged (every screen of M9 exists). Runs beside WP-15's building work; WP-15's browser runs and measurements wait until WP-14 has pushed its last change.
