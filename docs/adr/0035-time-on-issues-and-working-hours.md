# ADR-0035: Time on issues and working hours

Status: **Accepted** · 2026-10-31 · refines TIME-01, TIME-02, TIME-03, TIME-04, TIME-08, TIME-09, TIME-10, TIME-12, TIME-13, PRIV-09, [ADR-0006](0006-own-field-storage.md), [ADR-0015](0015-personal-data-marks.md), [ADR-0022](0022-who-sees-people.md) and [ADR-0025](0025-roles-groups-and-assignments.md); decisions M6-Q3…Q11

## Context

M6 lets people record two things about their time. The first is what they spent it on: a timer or a typed length on an issue, with *Activity* and *Cost center* for accounting (TIME-01, TIME-02, TIME-13). The second is when they worked: the start, the end and the breaks of each day, which German law asks every employer to record (TIME-08; the Federal Labour Court's ruling of 2022), checked against the working-time rules (TIME-09) and kept as a record that can be trusted (TIME-10).

Both are about employees, so they meet the rules that exist already: working time is sensitive employee data (PRIV-09), nothing scores or ranks people (COMP-10, ADR-0013), people are shown only as *Who sees people* allows (PRIV-12, ADR-0022), and a total never tells about something the reader cannot see (PRIV-13). Every stored field is marked `employee_time` (ADR-0015). Usual work days, away times and public holidays for planning are something else and come in M7 (ADR-0030); M6 neither reads nor writes them.

The maintainer answered M6-Q3 to M6-Q11 by taking every recommendation of the [M6 plan](../milestones/M6/README.md). This record writes those answers down, so the packages that build the server and the screens read one source.

## Decision

**Two records, kept apart on purpose.** A *time entry* says what time was spent on, for accounting: a person, an issue, a date, a length, a note and the fields of accounting. A *work day* says when a person worked, for the law: a start, an end, breaks and the person's time zone. They live in two modules (`bb-app::time` and `bb-app::working_hours`, each with its own tables) and never derive one from the other: a timer does not start a work day by itself, and a work day books no time on an issue. The timesheet shows both and compares them (*Booked on issues*, *Working hours not booked on issues*), and that comparison is the only place where they meet. Enforced by: review: a reviewer of the two server packages checks that neither module writes the other's tables or makes a record from the other's, and that the comparison reads time spent only through `TimeSpentStore` (decision 4).

### 1. Time entries, *Activity*, *Cost center* and *Billable* (M6-Q3, M6-Q7)

- A **time entry** holds its person, its issue, its date (the person's day of its start, decision 5, or the date typed with *Add time*), its length in whole minutes (1 to 1,440), an optional note (plain text, at most 2,000 characters, shown as text and never as HTML), *Billable* and the values of its own fields in `field_value` (ADR-0006). It has a version, and every change is a command with its event (ARCH-EVENTS, ARCH-VERSIONED).
- **The length is read and written once.** A typed length (`1:30`, `1.5`, `90m`) is read by the one reader of `bb_core::time`, and a length is written by the one writer of the wording (ADR-0033, amendment M6). Enforced by: TXT-DATES on screens, and review: no second reader of typed lengths exists.
- ***Activity* and *Cost center* are starter fields:** own fields of time entries that Bugaboo makes at the first start and on the upgrade of an existing server, like the starting workflow. Administrators rename, change or remove them like any own field (CF-08). They are not built-in fields: they leave the field catalogue of `bb-core`.
  - Bugaboo finds each by a **stable mark** on the field (its starter key, `activity` or `cost-center`), never by its name, so a renamed *Activity* still fills the *Stop timer* window (the lesson of M5-F43). Enforced by: review (no code compares the name of a field with a word), backed by the test `time_13_activity_and_cost_center_are_found_by_their_mark_after_a_rename` of WP-05.
  - *Activity* is a *Pick one* list with *Development, Testing, Documentation, Meetings, Support*. *Cost center* is a *Pick one* list whose choices have a number and a name (*4100 Product development*), sorted by the number; it starts empty.
  - **The default of an entry** is the value on its issue, then on its project, then on the projects above (the nearest wins). Neither field is required at the start; an administrator can make either required, and an entry without a required value is refused with a sentence that names the field.
- ***Billable*** ("time that can be charged to a customer") is a yes-or-no on a time entry. Whether it is offered is an organization setting, off at the start; an administrator switches it on in *Organization settings › Working hours*, and only then do the *Stop timer* window, *Add time* and the timesheet show the box. Switching it off hides the box and keeps the values stored.

### 2. Who books time: *Book time* (M6-Q4)

- A new project permission, ***Book time***: book one's own time on an issue one can see. The starting roles from *Contributor* up have it; *Viewer* and *Guest* do not. Its row in the catalogue (topic *Time*, kind, starting values) is in ADR-0025, amendment M6.
- It also covers changing and deleting one's own entries. Nobody changes or deletes another person's entries in M6; that is TIME-06 (1.0).
- Enforced by: ARCH-PERMS (every route of time declares it or `own-account`), SEC-MATRIX (the entry points `timer` and `time-entries` for every starting role).

### 3. The timer (M6-Q5)

- **One running timer per person, kept on the server.** Every device shows the same timer, and closing the browser does not stop it. The store holds at most one timer for a person (a unique key on the person), so two devices that start at once get one timer and one refusal with the conflict. Enforced by: TEST-CONCURRENT ("Timers and working hours start and stop at the same time", architecture §11), backed by the test `time_01_one_person_has_one_running_timer_on_every_device` of WP-05.
- **What a person does with it:** *Start* (on an issue the person can see and book on), *Stop*, ***Switch issue***, ***Change start time*** (never in the future, never before the end of the person's last entry) and ***Throw away*** (saves nothing).
- ***Switch issue*** is starting a timer on another issue while one runs: the time so far is saved as a time entry with the defaults of its issue, without a window, and the new timer starts.
- ***Stop*** opens the *Stop timer* window: the note, *Activity* and *Cost center* filled in from the issue, and *Billable* while it is on.
- **Whole minutes.** A timer is saved as whole minutes, rounded down. Under one minute nothing is saved, and the window says so.
- **The long-timer warning.** A timer that runs longer than the longest day of the organization's working-time rules (10 hours in the preset *Germany*, and 10 hours with *No rules*) shows a warning on the timer and sends the person one notification. The job `timers.long-running` (ADR-0017, amendment M6) is queued when the timer starts, for its start plus that length; a timer that was stopped before then leaves the job nothing to do.
- **The work day and the timer** stay two records: starting a timer while the work day has not started asks *"Start your work day too?"*, and *End work day* while a timer runs asks *"Stop the timer on WEB-142 too?"*. Neither starts or ends anything without the person's answer.

### 4. *Estimated time*, *Time left*, totals and the port of time spent (M6-Q6)

- ***Estimated time*** and ***Time left*** are lengths in minutes stored on the issue and changed like any field (a version, an event, the history, field levels). ***Time spent*** is the total of the issue's time entries; it is read, never typed.
- **The worked-out *Time left*.** While nobody has typed *Time left*, it is *Estimated time* minus *Time spent*, never below zero, and the screens show it in the quiet style of a value worked out. A typed value counts until it is cleared. It is worked out in one place, in `bb-core`.
- **Totals.** An issue shows its own numbers and, when it has sub-issues, the totals with them (every level below). A project shows the totals of its issues and its sub-projects.
- **A total counts only what the reader may see:** only the issues the reader sees and only the fields the reader may read, by the same compiled search that gives the list (ADR-0004), so a total never tells about a hidden issue or a hidden field (PRIV-13). Enforced by: SEC-MATRIX (the totals under each starting role), ARCH-DB-PARITY (the same totals on both databases).
- **The port of time spent, `TimeSpentStore`,** in `crates/bb-app/src/time/spent.rs`, is how everything outside the module of time entries reads time spent. WP-05 implements it in `bb-store`; the totals (WP-07) and the timesheet's comparison with working hours (WP-06, WP-10) read it, and their tests use a fake of it until the store has it. Its shape:

  ```rust
  /// The minutes booked on issues, for the readers outside the module of time entries.
  pub trait TimeSpentStore: ChangeStore + 'static {
      /// The minutes booked on each issue of `issues`, one row for each, `Minutes(0)` for an
      /// issue without entries. Reads only the issues in the list.
      fn spent_on_issues(
          &self,
          issues: &[IssueId],
      ) -> impl Future<Output = Result<Vec<(IssueId, Minutes)>, Self::Error>> + Send;

      /// The sum of the minutes booked on the issues that `request` finds: the compiled search
      /// of the list, with the reader's sight and fields in it, in one statement.
      fn spent_on_search(
          &self,
          request: &SearchRequest,
      ) -> impl Future<Output = Result<Minutes, SearchFailure>> + Send;

      /// The minutes `person` booked on each date from `first` to `last` (both included), by
      /// the date of each entry; a date without entries is left out.
      fn booked_per_date(
          &self,
          person: UserId,
          first: Date,
          last: Date,
      ) -> impl Future<Output = Result<Vec<(Date, Minutes)>, Self::Error>> + Send;
  }
  ```

  `Minutes` is the whole minutes of `bb_core::time` (the type of lengths, coding-standards §3), `Date` is `jiff::civil::Date`, and `SearchRequest` and `SearchFailure` are those of `bb-app::search`.
- **The port decides nothing.** It never reads an issue the caller did not pass: `spent_on_issues` reads the rows of the listed issues and no other, `spent_on_search` reads the issues the compiled search finds, whose visibility predicates the use case put into the request, and `booked_per_date` reads one person's own entries. Which issues a reader may see, whether they may read *Time spent*, and whose dates they may read are decided by the use case with the policy engine before it calls the port. `booked_per_date` is called for the person themselves only: *Working hours of others* shows no time on issues in M6 (decision 8). Enforced by: ARCH-AUTHZ-ONLY (only the use case turns what the port answers into an answer), SEC-MATRIX (the totals), and review: the port's methods take no person who reads.

### 5. Work days and the person's day (M6-Q9)

- A **work day** holds its person, its date, its start, its end (none while it runs), its breaks (each with a start and an end), its time zone, the warnings of the working-time rules (decision 6) and a version. All moments are stored in UTC (architecture §7, *Time*).
- **The person's day.** The date of a moment for a person is the date it falls on in the person's time zone (`bb_core::time`, the one answer; across the changes of daylight-saving time a day has 23 or 25 hours). A work day belongs to the date it starts on in the person's zone and may end the next morning; it keeps the zone it started in, so moving to another zone later never moves a past day. A time entry's date is the person's day of its start. Enforced by: TEST-WORKING-TIME (day boundaries, daylight-saving changes, zones, a day that ends after midnight).
- *Start work day*, *Start break*, *End break* and *End work day* record the moment of the tap; the timesheet's *Work day* rows take the same values typed afterwards.
- **A day left open stays open.** A work day that was never ended is not ended by Bugaboo: the record has to show what really happened (TIME-10). The next *Start work day* first asks *"When did your work day on Thursday end?"*, and the answer is a change of a past day (decision 7). Enforced by: TEST-WORKING-TIME (no job and no use case ends a day without the person's answer).
- **Two days of one person never overlap**, and a break lies inside its day. Two devices that start a day at once get one work day and one refusal. Enforced by: TEST-CONCURRENT, ARCH-VERSIONED.

### 6. The working-time rules as data (M6-Q8)

- **Rules are data, never code.** The organization has one set of rules, made from a **preset** and stored as settings in the registry (ADR-0018): a number for each rule, which administrators may change. M6 has two presets:
  - ***Germany*** (ArbZG): at most 10 hours of work a day (§3); a break of 30 minutes after 6 hours of work and of 45 minutes after 9 hours (§4); a break counts only in parts of at least 15 minutes, and nobody works more than 6 hours in a row without such a part (§4, the two details of M6-Q8); at least 11 hours of rest between the end of one work day and the start of the next (§5).
  - ***No rules***: nothing is checked, and the long-timer warning uses 10 hours.
- Sundays, public holidays and the average over six months are not checked in M6 (public holidays come with M7, ADR-0030).
- **Checked after every change, never a block.** After every change of a work day the rules are checked for that day and for the days next to it (the rest between two days crosses midnight), with the rules in force at that change. A broken rule becomes a **warning stored with the day** (which rule, and the numbers that broke it), shown in plain sentences on the timesheet. Recording is never refused because of a rule, because the record has to show what really happened (TIME-09). A later change of the rules does not rewrite the warnings of days nobody changed. Enforced by: TEST-WORKING-TIME (the unit and property tests of every rule of the preset *Germany*, rest across days included).
- The rules are worked out in one pure place, `bb_core::working_time`, from the moments and the zone of the days; the store and the screens hold no copy of them (`bb-core` does no I/O, ARCH-PURE).

### 7. Past days, reasons and the keep period (M6-Q9, M6-Q11)

- **A day is past from the next day on** in the zone of the day. Changing a past day (its start, its end, a break, or ending a day left open) asks for a **reason** in the person's own words; the change is refused without one. The event of the change carries the old values and the reason, and the history of the day shows both. Enforced by: ARCH-EVENTS (every change of a work day writes its event), TEST-WORKING-TIME (a change of a past day without a reason is refused).
- **The person is told.** When somebody else changes a person's day, the person gets a notification in the app, and by e-mail if they keep that on (ADR-0033). The notification names who changed what, with the reason, and goes to that person only.
- **The keep period.** Work days, their breaks and the reasons of their changes are kept for a number of years from their date, an organization setting. The preset *Germany* keeps them 2 years (ArbZG §16 (2)); with that preset the number cannot be set below 2 years. The daily job `working-hours.delete-old` (ADR-0017, amendment M6) deletes the days that are older. It deletes the events of those days too, so the old values and the reasons do not outlive the day (events follow the keep-and-delete rules, ADR-0007). Enforced by: ARCH-SETTINGS (the lower bound is the registry's), and review: a reviewer checks that the job's delete covers the events of the day.
- **Time entries keep no rule of their own in M6.** Their 10 years come with the table of *Keep and delete rules* (PRIV-05, M9).
- ***Remove a person*** (PRIV-04, M9) keeps the person's work days until their keep period ends, with the person shown as *Former user*; it does not delete them early, because the law asks for them (architecture §7, *Legal holds*).

### 8. Who sees working hours (M6-Q10)

- **One's own, always.** Every person sees and records their own working hours and time entries.
- **Others only with a permission.** Two new organization permissions: ***See working hours of others*** and ***Change working hours of others***. A new starting role, ***HR***, holds both; nobody has it at the start, the *Administrator* role does not include either, and an administrator gives *HR* to a group (for example *Payroll*). The rows in the catalogue, the role, and how a person who decides roles for the whole organization gives it, are in ADR-0025, amendment M6.
- ***Working hours of others*** is the one place that shows another person's working hours: a person picker, then that person's week in the timesheet's work-day rows, read only, or with *Change* for a holder of *Change working hours of others*. It lists people by name to holders of *See working hours of others*. It shows no time on issues; reading other people's time entries waits for TIME-06 (1.0).
- ***Totals only*** (an organization setting, off at the start): while it is on, *Working hours of others* shows each person's total of the week and nothing else: no days, no starts, ends or breaks, no warnings. The team reports of 1.0 obey it too.
- **Nowhere else.** No other page, total, count, history, notification, e-mail, export or search tells a person about another's working hours. A time entry of another person shows only as *Who sees people* and the field levels allow (the kind *Time entries*; *Time spent* as a field). Enforced by: SEC-MATRIX (the entry points `work-days` and `others-working-hours` for every starting role, *HR* included, and with *Totals only* on), ARCH-PERSON-REF.
- **Every gift and every removal** of these two permissions, and every change of the working-time settings (the preset, a rule, the keep period, *Totals only*, *Billable*), is written to the security log (PRIV-09: "changes to these settings are recorded"). Enforced by: ARCH-EVENTS, and the generated test `sec_15_every_change_of_permissions_is_in_the_security_log` of ADR-0025, decision 9, which covers the new permissions.

### 9. No scores, no rankings, no AI

- **No route, export, search or list orders, ranks or compares people by a number of time**: not by working hours, not by time booked, not by warnings, not by *Time spent*. A list of people is in the order of their names, and a total of a person is shown for that person alone. Grouping by person in the search language never gives a length per person.
- **No AI tool reads working hours.** The tool registry has no tool that reads work days, breaks, warnings or the totals of a person, and none that reads another person's time entries (ADR-0013, COMP-10).
- Enforced by: SEC-MATRIX (no route of time takes an order by person or by a total), ARCH-AI-TOOLS (no tool reaches the use cases of working hours), and the tests `priv_09_…` of WP-05 and WP-06.

## Consequences

- New tables for timers, time entries, work days and breaks, each column with its personal-data mark (DATA-MARKS); the values of own fields of time entries in `field_value`. Every column that says something about a person's time is `employee_time` (architecture §12, *Personal data in M6*).
- New permissions in the catalogue (*Book time*, *See working hours of others*, *Change working hours of others*), a new topic *Time* and a new starting role *HR* (ADR-0025, amendment M6).
- New settings in the registry: the preset and each number of the working-time rules, the keep period, *Totals only* and *Billable*, all on *Organization settings › Working hours* (UX §5.1).
- Two new kinds of job (ADR-0017, amendment M6), two new notification kinds (a long timer; a day changed by somebody else), and two new checks, TEST-WORKING-TIME and TXT-DATES.
- *Activity* and *Cost center* leave the field catalogue of `bb-core`, so the generated table of fields in architecture §7 loses their rows when WP-05 builds them as starter fields.

## Alternatives considered

- **One record for both (time entries carry the start and end of the day):** a person who books two hours on an issue has not said when they worked, and a day's working hours are not the sum of what was booked on issues. The law asks for the second, accounting for the first. Rejected.
- **Ending a day left open by itself at midnight or after 10 hours:** the record would show something that did not happen. Rejected (TIME-10).
- **Refusing a day that breaks a rule:** a person who worked 11 hours could not record it, and the record would lie. Rejected (TIME-09).
- **Giving *See working hours of others* to the *Administrator* role:** the people who set up the server would see everybody's working hours without anyone deciding it. Rejected; an administrator gives *HR* on purpose, and the security log records it.
- **A timer that lives in the browser:** two devices would run two timers, and a closed browser would lose the time. Rejected (TIME-01).
- ***Activity* and *Cost center* as built-in fields:** an organization could not rename or remove them (TIME-13). Rejected.

## Enforced by

TEST-WORKING-TIME, TXT-DATES, TEST-CONCURRENT, SEC-MATRIX, ARCH-PERMS, ARCH-PERSON-REF, ARCH-AUTHZ-ONLY, ARCH-AI-TOOLS, ARCH-EVENTS, ARCH-VERSIONED, ARCH-SETTINGS, ARCH-PURE, ARCH-DB-PARITY and DATA-MARKS (see `tools/checks/catalog.toml`), backed by the named tests of WP-05, WP-06 and WP-07, and review where the decisions say so.

<!-- COMP-10 checked: nothing ranks, evaluates or compares people by time; no AI tool reads working hours or other people's time entries. -->
