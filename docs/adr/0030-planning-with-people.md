# ADR-0030: Planning with people: who works on a bar, work times, time factors and plan warnings

Status: **Accepted** · amended the same day (work hours and shares; the person's own estimate) and in M8 (the person on the bar, holidays, where the plan is worked out, deletion) · refines PLAN-01…05 and [ADR-0022](0022-who-sees-people.md); bounded by PRIV-09 and COMP-10

## Context

Requirement: the timeline shows three more things: the person who works on each issue; a factor per issue and person, because people need more or less time for some tasks; and away times (holidays, other absences) per person, with Bugaboo saying where they clash with the plan or delay it.

Asked how other planning tools handle different paces, the lead found five ways: time per person (hours per day, a share per project, a calendar; nearly every tool), the person doing the work estimates it (LiquidPlanner, Scrum), the pace of the whole team (Jira Plans and other sprint tools), skill levels to find people but not to set dates (Primavera P6, Planview, Kantata), and paces learned from past work. The first two were taken.

All three are about employees, so they meet the rules we already have: working time and performance are sensitive employee data (PRIV-09), there are no performance scores and no ranking of people (COMP-10), and people are shown only as *Who sees people* allows (PRIV-12). In Germany, a tool that records how fast employees work is also a matter for the works council (Works Constitution Act, §87 (1) no. 6), and the reason for an absence can be health data (GDPR Art. 9).

## Decision

### 1. The person on the bar

- Each bar shows the picture and first name of the person it is *Assigned to*, at its end (PLAN-10); *Group by person* gives one row per person, so overlapping work is visible. Both follow *Who sees people* for the viewer: *Role only* shows *"Someone (Team member)"* and offers no grouping by person, *Hidden* shows nobody. Enforced by: the test `plan_10_the_timeline_shows_people_only_as_who_sees_people_allows` (M8).

### 2. Work times: when and how much a person works

- **Away** is a stretch of whole or half days on which a person does not work (PLAN-11). A person enters their own; people with the new permission *Enter work times for others* (organization, kind *Manage*; at the start only Administrator) enter them for others, and the person gets a notification.
- **No reason is stored**: Bugaboo does not know whether it is a holiday, a course or an illness. Others only ever see *Away*. A note is possible and only the person sees it. Enforced by: review (the record has no reason field, which a reviewer sees in the migration), backed by the test `plan_11_others_see_only_away`.
- **Usual work days** per person (for example Monday to Thursday) and **public holidays** per region (presets per country and, for Germany, per federal state; the person picks their region) say when a person normally works.
- **Work hours** per usual work day (PLAN-15; for example 6 hours on Monday to Thursday; at the start 8) say how much. They come from the work contract, so the person or someone with *Enter work times for others* enters them, never Bugaboo.
- **Share in the project** (PLAN-15): how much of a person's work hours belong to one project, for example 60 % for *Mobile app*. A Project lead sets it on *Project members*; at the start 100 %. When the shares of one person add up to more than 100 %, the person and the leads of those projects see that, without seeing the other projects' names unless they may.
- Usual work days, work hours, shares and away times are a new kind of people information in *Who sees people* (*Work times*: Names / Role only / Hidden; at the start Names for Team member and above, Hidden for Guests). Away times are deleted one year after they end; work hours and shares when the person leaves the organization (keep and delete rules, PRIV-05).

### 3. Plan by effort

- A timeline can **plan by effort** (PLAN-12): an issue with *Estimated time* (TIME-03) and a person gets as many of that person's work days as the estimate, times its time factor, needs: each day counts with the person's work hours times their share in the project, skipping away days, days off and public holidays. A person with 6 hours and a 50 % share gives an issue of 9 hours three days. Away days inside a bar are shown as a hatched gap, so the bar gets longer instead of hiding the gap. An issue with a fixed due date keeps it and gets a warning when the days don't fit.
- Without an estimate, or with planning by effort off, bars keep the dates people set (PLAN-01).

### 4. The time factor: the person's own estimate for this issue

- **Time factor** (PLAN-14) is a value on the issue, for the person it is assigned to: *×1.5* means "this issue will take this person half again as long as the estimate". It is a field with field permissions (at the start *Can change* for Project lead and Team member, *Hidden* for Guests), but **the assigned person may always change it on their own issues**, also where the field permissions would not allow it: they know best how long it will take them. They can enter it as a time (*"Takes me 10 days"*), and Bugaboo works out the factor. The issue shows who set it: *"Lea's own estimate"* or *"set by Ada Brandt"*; the assigned person always sees it and is notified when someone else changes it.
- **It belongs to the issue and goes back to ×1 when the issue is given to another person.** Bugaboo keeps no factor per person, no default per person or per kind of task, no totals, averages or reports of factors, and never sets or proposes a factor itself, with AI or otherwise. Without these limits it would be a stored judgement of how fast an employee works: a performance score (PRIV-09) that also needs the works council. Enforced by: review against COMP-10 and PRIV-09, backed by the tests `plan_14_the_time_factor_resets_when_the_person_changes` and `plan_14_no_report_or_search_sorts_by_time_factor`.

### 5. Plan warnings

- Bugaboo checks the plan and lists **plan warnings** (PLAN-13), each on the bar and in a list with what can be done:
  - *Away*: "Lea is away 26–30 Oct, while WEB-280 is planned for her";
  - *Not enough days*: "WEB-280 needs 6 work days, there are 3 before its due date";
  - *Delay*: "WEB-280 now ends 2 days later, and WEB-284 after it moves too" (along dependencies; the full chain to the end date with *Deadline drivers*, PLAN-05); Enforced by: the test `plan_13_a_delay_moves_along_dependencies` (M8);
  - *Too much at once*: "Lea has 9 hours planned on 27 Oct in Mobile app; her share there is 3.6 hours" (work hours times share, PLAN-15).
- The way out is always a person's choice: *Move it after*, *Make it longer*, *Give it to someone else…* (opens the usual person picker, with no suggestion of whom, COMP-10), or *Keep and accept*. Accepted warnings stay visible in a faded form.
- Warnings appear only for issues and people the viewer may see; a hidden person's away time shows at most as *"the assigned person is away"*.

## Amendment (M8, the maintainer's answers M8-Q4 to Q7 and Q13)

The maintainer took every recommendation of the M8 plan (2026-11-11). Where these points and a decision above differ, these points count. Dates and dependencies are [ADR-0043](0043-dates-and-dependencies.md).

- **The person on the bar is the first assigned person** (M8-Q4). An issue may have up to 20 people in *Assigned to*; the first of them is the *person on the bar*. Their work times shape the bar, *Too much at once* counts the issue for them only, and the time factor is theirs. The time factor goes back to ×1 when the first person changes (another person is put first, or the first is taken off); adding or taking off the people after the first leaves it alone. *Group by person* shows the bar in the row of each assigned person, every copy shaped by the first person's work times; the copies in the other rows carry a mark that says so (*"Planned with Lea's work times"*). The tests: `plan_10_the_first_assigned_person_shapes_the_bar` and `plan_14_the_time_factor_resets_when_the_person_changes` (the first person, as here).
- **The time factor is readable, never searched or summed** (M8-Q5). It is read on the issue and through the API like any field the reader may read, under its field permissions; the person on the bar always sees their own, on the issue and on the bar, also where the field is hidden from their role (decision 4). The search language has no word for it, so no search filters, sorts or groups by it; no list offers it as a column (every column of a list sorts); no total, average or count uses it. No tool of the tool registry changes it, so the assistant, AI apps and agents never set it (a rule of the registry, tested: `plan_14_no_tool_of_the_registry_changes_the_time_factor`, beside `plan_14_no_report_or_search_sorts_by_time_factor`).
- **Bugaboo works out public holidays itself** (M8-Q6). `bb-core` computes the fixed holidays and the ones that follow Easter, for Germany and for each of its 16 federal states, with no download and no outside service. Only holidays of a whole state count; a holiday of one town or of part of a state (the *Augsburger Friedensfest*, *Mariä Himmelfahrt* in the Bavarian towns that keep it) is entered as an away time. The results are tested against the official lists of the years 2026 to 2030 (`plan_11_german_holidays_match_the_official_lists_2026_to_2030`). More countries come when someone asks. Which holidays count for a person is the setting *Holidays* (`work-times.holidays`: *None*, *Germany* for the holidays every state has, or one of the 16 states): each person chooses theirs in *My settings › Work times*, and its default is the organization's choice, which starts as *Germany*. It is a setting of its own and not part of *Region*, which is about how dates and numbers are written. This replaces "the person picks their region" in decision 2.
- **The plan is worked out once, on the server** (M8-Q7). The bars of the rows on screen, their worked-out days and their warnings are computed in Rust: the rules in `bb-core` (pure, ARCH-PURE), the use case in `bb-app`, which reads dates, dependencies and work times as the reader. The browser draws what the server answers. While a bar is dragged the browser only moves it; on release it saves the dates, and the server answers with the new plan of the rows on screen within the budget of timeline data (PERF-01: p95 under 300 ms on 1 M issues). There is no WebAssembly module of the plan and no rule of the plan in TypeScript, so the first page stays within its budget (PERF-04) and the rules live once.
- **Work times are deleted on time from M8 on** (M8-Q13). The deletion comes with the data, so nothing is kept longer than decision 2 says: a daily job deletes each away time one year after its last day; all of a person's work times (usual work days, work hours, shares, away times, the choice of *Holidays*) are deleted when the person leaves the organization (the step that *Remove a person* calls, M9). A share is not deleted when the person loses their role in a project: it counts as nothing while they have no role there and counts as before when a role comes back, because business data is deleted only explicitly (the maintainer's rule). M9 shows these rules in the overview of keep periods (PRIV-05). The job is in [ADR-0017](0017-background-jobs.md), amendment M8.
- **Who sees *Work times* at the start.** The kind *Work times* starts like *Project members*: *Names* for Administrator, Project lead and Team member, *Role only* for Contributor and Viewer, *Hidden* for Guest and for *HR* (which gives no work in projects). This settles the roles that "Names for Team member and above, Hidden for Guests" of decision 2 left open. **Whose work times a reader sees:** the reader's level is the most open level they have in the projects where the person has a role; with no project in common the person's work times are *Hidden* to them (holders of *Enter work times for others* keep their reach and see names).
- **What a bar tells of work times** (M8 review). A reader who may not know a person's work times (*Work times* is *Hidden* for them) still gets that person's bars planned by effort and *Not enough days*, because the plan needs them. A bar's length can then tell how many days someone works; it never tells which days are away, holidays or days off, or why: hatched days, *Away* and *Too much at once* go only to a reader who may know the person's work times. An access key reads nobody's work times: its bars keep the dates people set.
- **Days without work of a person the reader does not see** (the maintainer's decision of 2026-11-13). A reader who sees the person on the bar by role only or not at all (*Hidden*) gets every day without work of that person as one neutral kind: away days, public holidays and days off look the same, with no word, and the answer does not tell them apart either, so nobody learns that a day was an absence. A warning *Away* about such a person says only *"the assigned person is away"*, with no day, and offers only *Keep and accept*. Readers who see the person by name keep the distinct patterns. Enforced by: the test `plan_11_a_reader_who_does_not_see_the_person_sees_every_day_without_work_alike` (M8).
- ***Keep and accept* is stored.** An accepted warning is kept with its issue, its kind, the dates it was accepted for and who accepted it, so everybody who sees the issue sees it faded; it comes back when the dates of its issue change.
- **The words of *Delay*** follow ADR-0043, decision 5: nothing moves by itself, so *Delay* says *"WEB-280 now ends 2 days after WEB-284 starts, and WEB-284 waits for it"*, and the test of decision 5 is `plan_13_a_delay_is_shown_along_dependencies_and_moves_nothing`.

## Consequences

- New records: away times, usual work days, work hours per day, a share per person and project, the setting *Holidays* per person, public holidays worked out in `bb-core`; a field *Time factor* in the field catalogue, with who set it; a kind *Work times* in *Who sees people*; the permission *Enter work times for others*.
- The timeline gains a picture per bar, rows per person, hatched away days, planning by effort and a warnings list. PLAN-07 (L) keeps baselines and export; the workload rows of PLAN-07 are now the rows per person here.
- The UX concept's timeline shows all of it.

## Alternatives considered

- **A factor per person and kind of task** ("Lea is slower at testing"), also as an option an organization switches on: a performance profile of an employee. Not taken (PRIV-09, COMP-10, and the works council where there is one). The person's own estimate on an issue gives the same plans without a profile.
- **Paces learned from past work** (estimate compared with time spent, per person): a hidden speed rating. Rejected (PRIV-09, COMP-10).
- **Only the pace of the whole team** (sprint velocity): fits sprint teams, but says nothing about away times or who is busy when. Not taken.
- **Storing the kind of absence** (holiday, illness, training): health data and not needed for planning. Rejected.
- **Moving bars by themselves around away times:** surprises people and breaks dates someone promised. Rejected; dates move only when planning by effort is on, and fixed due dates never move by themselves.

## Enforced by

ARCH-PERSON-REF (people on bars come as `PersonRef`), SEC-MATRIX (rows for away times), the tests named above (M8), and review against PRIV-09 and COMP-10.

<!-- COMP-10 checked: nothing ranks, evaluates or picks people; no factor is kept per person; Bugaboo never suggests whom to give an issue. -->
