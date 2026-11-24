# ADR-0049: Releases of a project: where they live, three states, two fields on issues

Status: **Accepted** · 2026-11-24 · refines ISS-08, PLAN-04, [ADR-0043](0043-dates-and-dependencies.md) decision 7 and [ADR-0005](0005-search-language.md) amendment M5, decision 11; the answers to M10-Q6, M10-Q7, M10-Q8 and M10-Q20 of [the M10 plan](../milestones/M10/README.md)

## Context

ISS-08 asks for *Found in release* and *Planned for release* on issues, "plus release status and dates". Until now a release is only a word: the glossary has *Release*, *Found in release* and *Planned for release*, the field catalogue reserves the two fields, and the search language reserves their words (`found in release`, `planned for release`) so that no own field can take them (M8-F57). The timeline draws a goal's due date as its milestone and promised releases "in M10" (ADR-0043, decision 7; M8-Q3); rows by release wait for them (PLAN-04).

A release here is a version of the product that an organization plans in Bugaboo (*2.4*). It is not a release of Bugaboo itself, which [ADR-0047](0047-releases-signatures-parts-list-and-support.md) is about.

Three things had to be decided: where a release lives, so that a product of several parts can plan one release for all of them; what a release holds and how it ends; and who manages releases.

## Decision

1. **A release lives in one project and is offered in that project and its sub-projects** (M10-Q6). A product with several parts plans one release for all of them by making it in their common project. Its name is unique within its project, by the one rule of names (`bb_core::names::same_name`). The offer follows the project tree as permissions do (PRJ-02). An issue moved out of the tree keeps its releases, and the issue shows them with a note that they are not offered there; a new choice of a release that is not offered where the issue is is refused. Enforced by: SEC-MATRIX (the routes of releases for every starting role), ARCH-DB-PARITY, and the tests `iss_08_…` of the package that builds releases.
2. **What a release is** (M10-Q7): a name, a *Release date* (the planned day: a calendar date with no time of day, optional), and a state, one of three, defined once in `bb-core`:
   - ***Planned***: the state when it is made;
   - ***Released***: the day it is marked is kept as *Released on*; it may go back to *Planned* with a reason, which the history shows;
   - ***Retired***: no longer offered for a new choice, still shown on the issues that name it. The glossary's word for "not offered any more, kept where it is used" is *Retire*, as for a choice, a label, an issue type, a field and a kind of relation; the recommendation called this state *Archived*, a word the glossary does not allow in the UI.

   A release has no start date: its span is the span of its issues. Enforced by: ARCH-EVENTS, ARCH-VERSIONED (every change of a release carries the version it read).
3. **Two built-in fields on issues** (M10-Q7). *Found in release* holds several releases (at most 20, a limit of its own, named once as a constant beside the field), because a bug can be seen in several releases; *Planned for release* holds one, because it is fixed in one. Both are fields of the field catalogue in `bb-core`, like *Due date*: field permissions with the starting levels of *Due date*, a place on forms and in layouts, a column in lists, a line in the history with the old and the new value, and their search words (decision 4). Enforced by: SEC-MATRIX (the field levels of both fields), ARCH-API-FREEZE (two optional members of an issue, nothing renamed), ARCH-EVENTS.
4. **The search words get their meaning.** `found in release` and `planned for release` take a release by its name as the person sees it, resolved like a status: the releases of that name in the projects the person sees, compared by `same_name`; a name the person cannot see is an unknown value with the same message as one that does not exist (ADR-0005, decision 9 of amendment M5). They take `is`, `is not`, `in (…)` and `is empty`; `sort by` orders by the *Release date*, and *Group by* groups by release. Enforced by: TEST-SEARCH-ROUNDTRIP, SEC-MATRIX (the `search` entry point).
5. **The permission *Manage releases*** (a project permission of kind *Manage*, topic *Work settings*, like *Manage labels and tags*; *Administrator* and *Project lead* hold it at the start) makes, renames, dates, marks and retires the releases of a project, and deletes them (decision 6). Seeing a release needs only seeing its project. Its sentence, kind and starting roles are in [ADR-0025](0025-roles-groups-and-assignments.md), amendment 2026-11-24. Enforced by: ARCH-PERMS, SEC-MATRIX.
6. **A release that an issue names is retired, never deleted.** Deleting it is refused with a sentence that says that issues name it and counts none of them (`release.in-use`), so the refusal tells nothing of issues the person cannot see. A person with *Manage releases* who is refused learns only that some issue names the release, never which or how many; this is accepted ([architecture §12](../architecture/12-security.md), *Accepted risks*). The counts that the releases page and the release window show ("14 issues are planned for it, 3 found in it") count only the issues the reader sees. A release that no issue names may be deleted by a person with *Manage releases*: it goes to the trash with *Undo*, like issues. Bugaboo never deletes a release by itself (explicit deletion, [ADR-0045](0045-keep-and-delete-rules.md), amendment 2026-11-24). Enforced by: SEC-MATRIX, ARCH-NO-COUNTERS (the refusal holds no count).
7. **Releases on the timeline, the calendar and *Home*** (M10-Q8). A release with a *Release date* is a diamond on the timeline and a mark on the calendar, like a goal's due date ([ADR-0043](0043-dates-and-dependencies.md), amendment 2026-11-24); it is not dragged there. *Group by release* (by *Planned for release*, with *None* last) groups the rows of the timeline and of a board. An issue planned for a release that ends after the release date shows *Delay* with the same ways out as for a goal. *Due soon* on *Home* lists the *Planned* releases due in the next 7 days. A release shows only to a reader who sees its project. Enforced by: SEC-MATRIX (the timeline and the calendar under visibility), and the tests `plan_05_…` and `iss_08_…` of the packages that draw them.
8. **AI apps and agents** (M10-Q20): a read tool `list_releases` (the releases offered in a project the person sees), and the two fields through `change_issue` with its preview, like any field. No tool makes, changes, marks, retires or deletes a release, so neither an AI app nor an agent manages releases. Enforced by: ARCH-AI-TOOLS.
9. **A release is business data, not personal data.** Its events name who changed it, shown as *Who sees people* allows (`PersonRef`); after *Remove a person* those events show *Former user*. Enforced by: DATA-MARKS, ARCH-PERSON-REF.

## Consequences

- A migration adds the releases of a project and the two fields of an issue; the API grows by the routes of releases under a project and two optional members of an issue (ARCH-API-FREEZE).
- The reserved words of the search language stand for something, and a saved search that used them before reads the same text with the new meaning (TEST-SEARCH-ROUNDTRIP).
- PLAN-04 is complete: rows by release join the timeline, and a release is a milestone of it beside a goal's due date.
- Standing rules of agents that change *Found in release* (AI-32) work on the new field through `change_issue` like any other field.

## Alternatives considered

- **One list of releases for the whole organization:** every project would see the releases of every other, and a name could not be reused. Rejected.
- **A release only in its own project, not below it:** a product of several parts would need the same release in every part. Rejected.
- **States the organization defines:** a workflow for releases that nobody asked for, and the timeline and the calendar would have to learn what each state means. Rejected; three states cover making, shipping and putting away.
- **One release per field, or several in both:** a bug can be seen in several releases but is fixed in one. Rejected.
- **A start date on a release:** its issues already have dates; a second span would disagree with them. Rejected.
- **The state *Archived*:** the recommendation's word, but the glossary keeps *archive* out of the UI and has *Retire* for exactly this meaning. Not taken; same meaning, the glossary's word.

## Enforced by

SEC-MATRIX, ARCH-PERMS, ARCH-EVENTS, ARCH-VERSIONED, ARCH-API-FREEZE, ARCH-DB-PARITY, ARCH-NO-COUNTERS, ARCH-AI-TOOLS, ARCH-PERSON-REF, DATA-MARKS, TEST-SEARCH-ROUNDTRIP, and the tests `iss_08_…` and `plan_05_…` that the packages of releases and of the timeline name.

<!-- COMP-10 checked: releases rank, evaluate or pick nobody; no tool lets AI manage them. -->
