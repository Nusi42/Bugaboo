# ADR-0030: Planning with people: who works on a bar, away times, time factors and plan warnings

Status: **Accepted** · 2026-10-16 · refines PLAN-01…05 and [ADR-0022](0022-who-sees-people.md); bounded by PRIV-09 and COMP-10

## Context

Requirement: the timeline shows three more things: the person who works on each issue; a factor per issue and person, because people need more or less time for some tasks; and away times (holidays, other absences) per person, with Bugaboo saying where they clash with the plan or delay it.

All three are about employees, so they meet the rules we already have: working time and performance are sensitive employee data (PRIV-09), there are no performance scores and no ranking of people (COMP-10), and people are shown only as *Who sees people* allows (PRIV-12). In Germany, a tool that records how fast employees work is also a matter for the works council (Works Constitution Act, §87 (1) no. 6), and the reason for an absence can be health data (GDPR Art. 9).

## Decision

### 1. The person on the bar

- Each bar shows the picture and first name of the person it is *Assigned to*, at its end (PLAN-10); *Group by person* gives one row per person, so overlapping work is visible. Both follow *Who sees people* for the viewer: *Role only* shows *"a Team member"* and offers no grouping by person, *Hidden* shows nobody. Enforced by: the test `plan_10_the_timeline_shows_people_only_as_who_sees_people_allows` (M7).

### 2. Away times

- **Away** is a stretch of whole or half days on which a person does not work (PLAN-11). A person enters their own; people with the new permission *Enter away times for others* (organization, kind *Manage*; at the start only Administrator) enter them for others, and the person gets a notification.
- **No reason is stored**: Bugaboo does not know whether it is a holiday, a course or an illness. Others only ever see *Away*. A note is possible and only the person sees it. Enforced by: review (the record has no reason field, which a reviewer sees in the migration), backed by the test `plan_11_others_see_only_away`.
- **Usual work days** per person (for example Monday to Thursday) and **public holidays** per region (presets per country and, for Germany, per federal state; the person picks their region) say when a person normally works.
- Away times are a new kind of people information in *Who sees people* (*Away times*: Names / Role only / Hidden; at the start Names for Team member and above, Hidden for Guests). They are deleted one year after they end (keep and delete rules, PRIV-05).

### 3. Plan by effort

- A timeline can **plan by effort** (PLAN-12): an issue with *Estimated time* (TIME-03) and a person gets as many of that person's work days as the estimate, times its time factor, needs, skipping away days, days off and public holidays. Away days inside a bar are shown as a hatched gap, so the bar gets longer instead of hiding the gap. An issue with a fixed due date keeps it and gets a warning when the days don't fit.
- Without an estimate, or with planning by effort off, bars keep the dates people set (PLAN-01).

### 4. The time factor: about this issue, not about the person

- **Time factor** (PLAN-14) is a value on the issue, for the person it is assigned to: *×1.5* means "this issue will take this person half again as long as the estimate". Whoever plans sets it by hand (it is a field with field permissions; at the start *Can change* for Project lead and Team member, *Hidden* for Guests); the assigned person always sees it on their own issues.
- **It belongs to the issue and goes back to ×1 when the issue is given to another person.** Bugaboo keeps no factor per person, no default per person or per kind of task, no totals, averages or reports of factors, and never sets or proposes a factor itself, with AI or otherwise. Without these limits it would be a stored judgement of how fast an employee works: a performance score (PRIV-09) that also needs the works council. Enforced by: review against COMP-10 and PRIV-09, backed by the tests `plan_14_the_time_factor_resets_when_the_person_changes` and `plan_14_no_report_or_search_sorts_by_time_factor`.

### 5. Plan warnings

- Bugaboo checks the plan and lists **plan warnings** (PLAN-13), each on the bar and in a list with what can be done:
  - *Away*: "Lea is away 26–30 Oct, while WEB-280 is planned for her";
  - *Not enough days*: "WEB-280 needs 6 work days, there are 3 before its due date";
  - *Delay*: "WEB-280 now ends 2 days later, and WEB-284 after it moves too" (along dependencies; the full chain to the end date with *Deadline drivers*, PLAN-05); Enforced by: the test `plan_13_a_delay_moves_along_dependencies` (M7);
  - *Too much at once*: "Lea has 3 issues on 27 Oct, more than her day has hours for".
- The way out is always a person's choice: *Move it after*, *Make it longer*, *Give it to someone else…* (opens the usual person picker, with no suggestion of whom, COMP-10), or *Keep and accept*. Accepted warnings stay visible in a faded form.
- Warnings appear only for issues and people the viewer may see; a hidden person's away time shows at most as *"the assigned person is away"*.

## Consequences

- New records: away times, usual work days, a region per person, public-holiday calendars; a field *Time factor* in the field catalogue; a kind *Away times* in *Who sees people*; the permission *Enter away times for others*.
- The timeline gains a picture per bar, rows per person, hatched away days, planning by effort and a warnings list. PLAN-07 (L) keeps baselines and export; the workload rows of PLAN-07 are now the rows per person here.
- The UX concept's timeline shows all of it.

## Alternatives considered

- **A factor per person and kind of task** ("Lea is slower at testing"): a performance profile of an employee. Rejected (PRIV-09, COMP-10, works council). A person may still say for themselves, on an issue, that they need longer; that is an estimate, not a profile.
- **Storing the kind of absence** (holiday, illness, training): health data and not needed for planning. Rejected.
- **Moving bars by themselves around away times:** surprises people and breaks dates someone promised. Rejected; dates move only when planning by effort is on, and fixed due dates never move by themselves.

## Enforced by

ARCH-PERSON-REF (people on bars come as `PersonRef`), SEC-MATRIX (rows for away times), the tests named above (M7), and review against PRIV-09 and COMP-10.

<!-- COMP-10 checked: nothing ranks, evaluates or picks people; no factor is kept per person; Bugaboo never suggests whom to give an issue. -->
