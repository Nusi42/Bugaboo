# ADR-0045: Keep and delete rules

Status: **Accepted** · 2026-11-18 · refines PRIV-05, [ADR-0017](0017-background-jobs.md) and [ADR-0035](0035-time-on-issues-and-working-hours.md) decision 7; the answers to M9-Q1 and M9-Q2 of [the M9 plan](../milestones/M9/README.md)

## Context

PRIV-05 asks for "keep and delete rules per kind of data … with automatic deletion" and sensible defaults. UX design §5.7 draws one table: working hours, time on issues, the security log, IP addresses, notifications, the trash.

Some of it exists, scattered. The security log is pruned by a daily job after `security.log-keep-days`; notifications go after 90 days; dead sessions, drafts, OAuth grants and away times have daily jobs; the files of the trash go after 30 days. Work days are never deleted by Bugaboo: after their keep period an administrator deletes them, a decision of the maintainer (ADR-0035, decision 7). Issues in the trash stay for ever (M3-F25, M4-F131), and so do the events that carry old values (M4-F93, M6-F45), the payload of dead jobs (M1-F21) and the lines of the security log's anchor file (M1-F26).

The maintainer's rule is that a keep period is a minimum: as long as data is relevant to the organization, keeping it is legitimate, and only the organization can judge relevance. So business records are never deleted by Bugaboo by itself. Data that only serves a technical purpose, or that protects a person, has no such interest behind it and should go by itself.

## Decision

1. **Two kinds of rule** (M9-Q1). Every kind of data has one row in one list, `keeping::KEEP_KINDS` in `bb-core`, and each row is one of:
   - **Deleted by itself** after its period: data with no business purpose of its own. IP addresses in logs (7 days), sessions and sign-in history (90 days after they end), sign-up links, drafts, notifications (90 days), the trash (30 days, which finishes a deletion a person already made; an issue whose time entries are still inside their keep period stays in the trash, hidden, until the last of them leaves it, so the purge never takes a business record along, M9-Q1), data exports (7 days), the payload of dead jobs (30 days), away times (1 year after their last day, ADR-0030), the security log (1 year, in whole blocks, ADR-0007), backups (by their schedule, ADR-0046), and, from M14, the record of an ended support access (with the security log's period, ADR-0048).
   - **Deleted by an administrator** after its period: business records. Work days with their breaks and reasons (at least 2 years with the preset *Germany*, ADR-0035) and time entries (10 years at the start, M9-Q2: German law keeps receipts 8 years since 2025 and the books 10, and Bugaboo cannot tell which entries are which). Bugaboo shows how many are past the period and offers *Delete them*, as it does for work days today.

   Enforced by: ARCH-SETTINGS (each period is a setting of the registry), review (a new kind of business record joins the second group; a job that deletes one is a finding).
2. **One page shows them all.** *Organization settings › Privacy › Keep and delete rules* lists every row of `KEEP_KINDS`, with its period, what happens then, and why, in plain words. A period is changed there; a change that would delete something at once first says how much ("312 notifications will be deleted tonight"). The table of the user docs is written from the same list. Enforced by: DOC-GENERATED (the table of the user docs), ARCH-SETTINGS.
3. **A period cannot go below its floor.** Each row has a lowest period: the law's for records the law asks for (work days, ADR-0035), a technical floor for the rest (the trash at least 1 day, so a deletion can still be undone). Enforced by: ARCH-SETTINGS (the registry refuses a value below the floor), review (each floor names its reason).
4. **Records take their events along.** When a record is deleted, by a job or by an administrator, the events that carry its values go in the same change (ADR-0007): the domain event of an upload with its file name (M4-F93), the note and fields of a time entry (M6-F45), the reasons of a work day (as today). The history of a record that still exists is kept. Enforced by: ARCH-EVENTS, ARCH-DB-PARITY.
5. **One way to delete by itself.** Every row of the first group is a daily kind of `JobKind` (rhythm `FromTheStart`, or `QuietHour` for the large ones) that deletes in steps of `KEEP_STEP_ROWS` rows, one short write each, so a write of people never waits long behind it (M4-F104). The existing jobs move under this one shape; no second loop of deleting is written. The security log's anchor file is trimmed by the same job that prunes the log, keeping the newest line and writing the trimmed file next to the old one before it is renamed into place (M1-F26). Enforced by: ARCH-EVENTS, PERF-N1, review (one helper for deleting in steps).
6. **One way for an administrator to delete.** The count of what is past its period and *Delete them* work the same for every row of the second group, through one use case (the one that deletes old work days today becomes general). It needs *Change organization settings*, no key or agent can call it, and every run writes how many of what (and of whom) were deleted to the security log, each step in the same change as what it deleted, so a run that stops between steps leaves nothing deleted without its entry. A run that deletes the last kept work days of a removed person clears that person's legal-hold name in the same change (ADR-0044, decision 6). Enforced by: ARCH-PERMS, SEC-MATRIX, ARCH-EVENTS.

## Consequences

- Some rows of UX design §5.7 change: *Time on issues* reads "10 years, then an administrator deletes them" instead of "Deleted" (M9-Q2).
- The keep periods of a removed person's records come from the same list, so the window of *Remove a person* (ADR-0044) says "kept until 2028-10-01" from it.
- Every kind of data an upcoming feature adds (documents, M11) needs a row; a missing row is a finding.

## Alternatives considered

- **Automatic deletion for everything, as PRIV-05 first said:** the maintainer decided that business records are deleted only explicitly (M6, ADR-0035); doing it for some records and not others by accident is worse. Rejected.
- **Nothing deleted by itself:** keeps IP addresses, sessions and exports far longer than any purpose needs (Art. 5(1)(e) GDPR). Rejected.
- **A rule per project:** nobody asked, and it multiplies the questions of *Remove a person*. Not now.

## Enforced by

ARCH-SETTINGS, ARCH-EVENTS, ARCH-PERMS, SEC-MATRIX, ARCH-DB-PARITY, PERF-N1, DOC-GENERATED; review (which group a new kind of data joins).

## Amendment (M9, who reads the rules and what a change would delete)

- **Every person reads the rules.** `GET /api/v1/admin/keep-rules` answers every person who signs in with a session, not an access key or an agent: how long their data is kept is part of what GDPR Art. 15(1)(d) gives them, and *My settings › My data* shows it. The answer is the same for everybody, the setting and the floor of each row included, because a keep period is not secret. Changing a period and the counts stay with *Change organization settings*. Enforced by: SEC-MATRIX.
- **What a changed period would delete is counted with the cutoff of the deleting.** `GET /api/v1/admin/keep-rules/{kind}/would-delete?count=…` counts, before a period is saved, the records the job of a kind of the first group would delete with the changed period, or the records of a kind of the second group that would be past it. The settings as they would be come from `KeepRule::with_period`, the cutoff from the same function as the job's (`keep_cutoff_with`, `kept_from_today_with`), and each count from the condition the purge of its kind deletes by (`bb-store`, `keeping/past.rs`), so the count and the deleting cannot drift apart. Only a kind whose period is one setting has a count: a fixed period is never changed, and the rotation of backups is the work of two settings on the page of backups. Enforced by: SEC-MATRIX, ARCH-DB-PARITY (the counts on both databases), review (no second cutoff).

## Amendment (2026-11-24, releases, dashboards, pins and e-mailed searches)

M10 adds four kinds of data ([the M10 plan](../milestones/M10/README.md)). None of them is deleted after a time, so none has a period and none is a row of `KEEP_KINDS`, which holds the kinds deleted after a time (decision 1); this amendment is where each of them says why (the consequence "every kind of data needs a row" is met by a row or by a sentence here):

- **Releases** ([ADR-0049](0049-releases.md)) are business records: kept until a person with *Manage releases* deletes one that no issue names. Bugaboo never deletes one by itself (the maintainer's rule of explicit deletion, decision 1).
- **Dashboards** ([ADR-0050](0050-dashboards-and-totals.md)) are kept until their maker deletes them, or a person with *Change organization settings* deletes a shared one; the private ones go at *Remove a person*.
- **Pins** are kept until their person takes them off; a pin goes with the project, saved search or dashboard it points to, and at *Remove a person*.
- **E-mailed searches** ([ADR-0033](0033-notifications-and-e-mail.md), amendment 2026-11-24) are kept until their person deletes them; they go with their saved search and at *Remove a person*. The e-mail itself is a job like every e-mail, whose payload the row *Dead jobs* already covers.

A deleted release or dashboard goes to the trash first, which the row of the trash empties after its period, as for issues and saved searches. Enforced by: DATA-MARKS (each new entity carries its marks and person links), review (a job that deletes one of these four by time is a finding, and so is a new kind of data with neither a row nor a sentence here).
