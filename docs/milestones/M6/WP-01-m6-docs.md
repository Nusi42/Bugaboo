# WP-01 — Decision records, words, the security and privacy chapter, the UX gaps and the maintainer's answers

**Wave 1 · Folder / branch:** `bugaboo.win1` / `WIN1` (lead)
**Features:** all of M6, the docs they need (TIME-01, TIME-02, TIME-03, TIME-04, TIME-08, TIME-09, TIME-10, TIME-12, TIME-13, PRIV-09); ADR-0017, ADR-0025, ADR-0026, the new ADR-0035.

## Goal

Before code names anything new, the words, the decisions and the maintainer's answers are written down once, so the builders of waves 2 and 3 read one source. Wave 1's code packages (WP-02, WP-03, WP-04) need only what is decided already and start while the lead writes the rest.

## Tasks

1. **The maintainer's answers** to M6-Q1…Q13 ([README](README.md)), each written where it belongs:
   - **M6-Q1** (theme files): ADR-0026 amendment M6; coding-standards §4 and `CLAUDE.md` say "the theme files in `design/`" instead of "`design/theme.kdl`" (WP-02 writes the details of the syntax).
   - **M6-Q2** (*Time of day*): the glossary (*Region*, *Time of day*) and ADR-0033's wording amendment (what the vectors hold).
   - **M6-Q3, Q4, Q5, Q6, Q7** (time on issues): ADR-0035 §1 to §4 and the glossary.
   - **M6-Q8, Q9, Q11** (working hours, rules, reasons, keep period): ADR-0035 §5 to §7 and architecture §5 *Working hours*.
   - **M6-Q10** (who sees working hours): ADR-0035 §8 and the ADR-0025 amendment M6 (the permissions, the starting role *HR*, the giving of it).
   - **M6-Q12** (demo): 04-milestones M6 *Demo (exit)* only if the maintainer takes the added steps.
   - **M6-Q13** (what stays out): the follow-up tables and the README.
2. **A new decision record, ADR-0035 *Time on issues and working hours*:**
   - two records that are kept apart on purpose: *time entries* (what the time was spent on, for accounting) and *work days* (when a person worked, for the law);
   - the timer: one per person, on the server, *Switch issue*, whole minutes, the long-timer warning;
   - the day of a person: the date a moment falls on in the person's time zone, kept with each work day (architecture §7 *Time*);
   - the working-time rules as data (presets), checked after every change, stored as warnings with the day, never a block;
   - a reason for a change of a past day, the old value in the event, the person told;
   - who sees what: one's own always; others only with the permissions of M6-Q10; *Totals only*;
   - the keep period and its daily job;
   - **no scores, no rankings, no AI**: no route, export, search or list orders or compares people by a time number; no AI tool reads working hours (ADR-0013, COMP-10); enforced by SEC-MATRIX, ARCH-AI-TOOLS and the tests `priv_09_…` of WP-05 and WP-06.
3. **Amendments:**
   - ADR-0026 (M6-Q1): a base file and theme files; a token that names another (M5-F121).
   - ADR-0025 (M6-Q10): the topic *Time*, the three permissions *Book time*, *See working hours of others*, *Change working hours of others*, the starting role *HR*, and how an administrator gives it.
   - ADR-0017: the kinds of M6 (`working-hours.delete-old`, daily; `timers.long-running`, once per timer) and the statistics refresh after the start round (M5-F168, WP-04 writes the numbers).
4. **The port of time spent.** `TimeSpentStore` in `bb-app/src/ports`: the minutes spent per issue for a list of issues, the sum for the issues a compiled search finds, and the minutes one person booked per date in a range, so WP-07 builds the totals and WP-06 the difference to working hours while WP-05 builds the entries. The shape (names, inputs, outputs, the rule that it never reads an issue the caller did not pass) is written here before wave 2 starts.
5. **Glossary** (*Time* and *People and access*):
   - time on issues: *Switch issue*, *Change start time*, *Throw away* (of a timer), *Add time*, *Book time* (the permission), *Billable*, *Booked on issues*, *Working hours not booked on issues*;
   - working hours: *Working hours of others*, *See working hours of others*, *Change working hours of others*, *Totals only*, *Preset* (of the working-time rules: *Germany*, *No rules*), *Reason* (for a change of a past day), *HR* (the starting role), *Time of day* (in *Region*);
   - internal terms: *Person's day*, *Starter fields*, *Time spent port*.
   Each with its *Don't say* (for example *clock in*, *punch*, *attendance*, *overtime score*, *productivity*).
6. **Checks** (planned, in the catalogue, with their rule text): TEST-WORKING-TIME (WP-06) and TXT-DATES (WP-03), each written as "Adding a rule" in architecture §10 asks.
7. **Security chapter:** the section *Time and working hours (M6)* in architecture §12, STRIDE rows with what proves each:
   - a person reads another's working hours or time entries through a page, a total, a count, a history, a notification, an e-mail, an export or a search (information disclosure; PRIV-09, PRIV-13);
   - a person changes another's work day without the permission, or without a reason, or the person is not told (tampering, repudiation; TIME-10);
   - a person gives themselves *See working hours of others* (elevation of privilege; ADR-0025);
   - two devices start two timers, or a work day is saved twice (tampering; TEST-CONCURRENT);
   - a booked note carries script (SEC-13);
   - many timers or entries slow the server (denial of service; the limits of WP-05 and WP-06).
8. **Personal data in M6** (architecture §12 and `docs/privacy/personal-data.md` through DATA-MARKS): work days and breaks are `employee_time`, basis legal obligation (§16 ArbZG); time entries and their notes are `employee_time`, basis contract; the timer is `employee_time`; the reasons of changes are `employee_time`. The legal hold after *Remove a person* (architecture §7) names work days.
9. **UX design** (§4.9 and the settings chapter), drawn the way §4.9 draws the computer:
   - the timesheet on a phone (one day at a time, the week's totals on top);
   - *Working hours of others* (a person picker, the same grid read only or with *Change*, and *Totals only*);
   - the reason window for a past day and the history of a day;
   - *Organization settings › Working hours* (preset, rules, keep period, *Totals only*, *Billable*);
   - the long-timer warning, the question of a day left open, *Add time* on the issue page;
   - the phone issue page without the app's bar (M5-F172) and the shorter sidebar (M5-F150), so WP-09 has a picture to match.
10. **Follow-up tables:** the M6 targets of M2, M3, M4 and M5 point to their M6 package (README, "Follow-ups handled in M6"); M3-F49 moves to M11.

## Acceptance

- `cargo xtask check staged` green on the docs (LINT-LINKS, PLAN-TRACE, LINT-SPELL, FMT-MD, TXT-TERMS, META-RULE-TRACE for every new "must").
- Every answer of M6-Q1…Q13 is findable in the place this list names.
- The builders of wave 2 have the ADR, the words, the port shape and the pictures before they start.

## Review

A reviewer with a fresh context reads the docs against the maintainer's answers and applies the SSOT/DRY/ODR gate of coding-standards §1 to them: a rule written in two docs, or a word that the glossary names otherwise, blocks the merge like code does. A docs-only fix round is still read before the merge (M5 lesson).

## Depends on

The maintainer's answers to M6-Q1…Q13. Nothing else.
