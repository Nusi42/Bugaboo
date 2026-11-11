# ADR-0043: Dates and dependencies

Status: **Accepted** · 2026-11-11 · refines PLAN-01, PLAN-02, PLAN-03, PLAN-04 and [ADR-0030](0030-planning-with-people.md); the maintainer's answers M8-Q1, M8-Q2 and M8-Q3

## Context

The timeline draws issues as bars over days, so an issue needs dates. Before M8 an issue has *Estimated time* and *Time left*, but no start date, and *Due date* is only a field key that search cannot read and forms block (M3-F101). Own fields can be dates. PLAN-01 asks that the dates come from start and due fields, "configurable which ones".

Relations are rows of kinds that the organization edits (*blocks*, *duplicate of*, *causes*, *related to*). None of them means anything for dates, although *blocks* says what the glossary calls *Must finish first*. PLAN-02 asks for that dependency in M8 and for *start together*, *finish together* and *finish when it starts* in V1, and it asks for one relation model for issues, the timeline and the map.

Many planning tools push the issues that wait when an earlier one moves. ADR-0030 rejected bars that move by themselves: it surprises people and breaks dates someone promised.

The exit demo of M8 asks for "a warning for the release", but releases (ISS-08) come in M10.

## Decision

1. **Two built-in date fields, *Start date* and *Due date*** (M8-Q1). Both are fields of the field catalogue in `bb-core`, like *Estimated time*: a calendar date with no time of day and no time zone, stored on the issue (migration 49). They have field permissions with the starting levels of *Estimated time*, a search word each (`start date`, `due date`) with the comparisons the search language has for dates, a place on forms and in layouts, and a column in lists. *Due date* is no longer blocked on forms (M3-F101). A *Start date* after the *Due date* of the same issue is refused with a sentence that names both. Enforced by: SEC-MATRIX (the field levels of both fields), TEST-SEARCH-ROUNDTRIP, ARCH-DB-PARITY, ARCH-API-FREEZE (two optional members, nothing renamed).
2. **A timeline may use other date fields.** The timeline is a view of a saved search (ADR-0034, the kind `ViewKind::Timeline`). It uses *Start date* and *Due date* unless the view names two own date fields instead, chosen in the view and stored with it. Dragging a bar changes the fields the view uses, each under its field permissions. A field the reader cannot read counts as empty for that reader, so the issue waits in *Not scheduled yet* and nothing tells that the field exists (UX §1, *Hidden stays hidden*). Enforced by: SEC-MATRIX (the timeline route under field levels), the tests of WP-06.
3. **How a bar gets its days.** A bar covers whole days, from its first day to its last day, both included.
   - With both dates, the bar runs from the start date to the due date.
   - With *Plan by effort* on (ADR-0030, decision 3), an issue with *Estimated time* and a person on the bar gets its other date worked out when it has only one: forward from the start date, or back from the due date. With both dates the bar keeps them, and *Not enough days* says when the effort does not fit; the due date never moves.
   - Otherwise an issue with one date is a bar of one day on that date.
   - An issue with no date waits in *Not scheduled yet*.
   Worked-out days are shown, never stored: a date is stored only when a person or a tool sets it. Enforced by: the tests of WP-06 (`plan_12_…`), review (a reviewer sees in the plan's code that only a change of an issue writes a date).
4. **The date effect of a kind of relation** (M8-Q2). A kind of relation gets a *date effect*: none, or *Must finish first*. In M8 *Must finish first* is the only effect. The starting kind *blocks* has it: the issue that blocks must finish first, and the one it blocks waits. An organization may give it to one other kind as well, so at most two kinds carry it; whoever may change kinds of relation (*Manage issue types, kinds of relation and layouts* for the whole organization) gives or takes it. The effects *start together*, *finish together* and *finish when it starts* come in V1 (PLAN-02). Enforced by: the tests of WP-03 (`plan_02_…`), SEC-MATRIX (the route that changes a kind of relation).
   - A relation of a kind with a date effect is a **dependency**. The timeline draws it as an arrow from the issue that must finish first to the one that waits. The issue page, the relation map and the timeline read the same rows of relations (PLAN-02; architecture §7, *Relations*): there is no second record of dependencies. Enforced by: review (a second table or type of dependencies is an ODR finding of coding-standards §1).
   - A dependency that would close a loop (*A* waits for *B*, and *B* for *A*, over any number of steps) is refused when the relation is made, and giving the effect to a kind is refused when its relations hold such a loop; the sentence names one loop. Loops of *blocks* that are stored already when migration 49 gives it the effect stay, and the timeline draws their arrows without checking an order along them. Enforced by: the tests of WP-03.
5. **A move that breaks a dependency is a warning, never a push.** When an issue that waits starts on or before the last day of an issue it waits for, on the days the plan shows, both bars and the list of warnings show the plan warning *Delay* (ADR-0030, decision 5): *"WEB-280 now ends 2 days after WEB-284 starts, and WEB-284 waits for it"*. Nothing else moves. Its ways out are those of ADR-0030: *Move it after* moves the issue that waits, and the issues that wait for it in turn, so that each starts on the first day after the one before it ends; a preview names every issue and its new dates before anything is saved. *Keep and accept* leaves the plan as it is. *Plan by effort* never moves a start along a dependency by itself. Enforced by: the tests of WP-06 (`plan_13_…`), review against ADR-0030 (*Moving bars by themselves* was rejected).
6. **Every change of a date is a change of the issue.** A change of *Start date* or *Due date* (on a form, by a drag, by *Set dates…*, by *Move it after*, by an AI app or an agent) is one event with the old and the new value (ADR-0007). The history shows it (*"Lea moved WEB-280 from 3–7 Nov to 10–14 Nov"*) and *Undo* takes it back. *Move it after* with the issues after it is one run, so *Undo* takes every date of it back in one step. A save carries the version it read: a change made by somebody else in between gets *Keep Ben's / Use mine*. Enforced by: ARCH-EVENTS, ARCH-VERSIONED, TEST-CONCURRENT (two people move the same bar at once).
7. **Goals on the timeline; no releases yet** (M8-Q3). A goal with a *Due date* is drawn as a diamond on that date: it is the milestone of the timeline that PLAN-04 names. A goal's span is a bracket over its sub-issues (UX §4.7). When a sub-issue ends after the due date of its goal, the goal shows *Delay* with the same ways out: *"Lea's away week moves Login screen past the due date of Mobile app 2.0"*. Releases (ISS-08) join the timeline in M10, and with them rows by release. Enforced by: the tests of WP-06 and WP-09.

## Consequences

- Migration 49 adds the two dates to the issue and the date effect to the kinds of relation, and gives *blocks* the effect *Must finish first*. Enforced by: DATA-MIGRATIONS, ARCH-DB-PARITY.
- Search, forms, layouts, lists and the API know the two fields; the API grows only by additions (ARCH-API-FREEZE).
- The plan of ADR-0030 reads its dates and dependencies from here, and the timeline draws them. The *Delay* sentence of ADR-0030, decision 5, changes: nothing "moves too" (ADR-0030, amendment M8).
- An organization that plans with its own date fields keeps them; it chooses them in the timeline view.

## Alternatives considered

- **Only own date fields, chosen in each view:** no search word and no starting field permissions, and every organization would first have to set them up. Not taken as the only way; a view may still choose them.
- **Dependencies as a record of their own beside relations:** two models for one connection, which PLAN-02 rules out. Rejected.
- **Push the issues that wait when an earlier one moves:** what other tools do. It moves dates that people promised, without a person choosing it. Rejected, as in ADR-0030.
- **All four date effects now:** the other three are V1 in PLAN-02 and need their own warnings. Not taken.
- **Releases on the timeline in M8:** releases are their own record with their own rules (ISS-08, M10). Not taken; a goal's due date stands in for them.

## Enforced by

SEC-MATRIX, ARCH-EVENTS, ARCH-VERSIONED, ARCH-DB-PARITY, ARCH-API-FREEZE, TEST-SEARCH-ROUNDTRIP, TEST-CONCURRENT, TEST-A11Y (every drag has a typed alternative: *Set dates…*, *Add dependency…*), the tests that WP-03, WP-06 and WP-09 of M8 name, and review against ADR-0030 (no bar moves by itself).

<!-- COMP-10 checked: dates and dependencies rank, evaluate or pick nobody; Move it after moves issues, never people. -->
