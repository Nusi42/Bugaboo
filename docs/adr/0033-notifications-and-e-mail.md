# ADR-0033: Notifications and e-mail: an outbox reader that checks again before it tells anyone

Status: **Accepted** · 2026-10-16 · refines NTF-01, NTF-02, SEC-06, SEC-16, PRIV-05, PRIV-08, PRIV-12 and [ADR-0007](0007-event-log.md); decisions M4-Q5…Q7, M4-Q9, M4-Q10

## Context

Since M3, Bugaboo stores who follows an issue, who is mentioned, and the notes of *Ask … to finish* and *Edit anyway*, but it tells nobody (M3-Q3, M3-F75). M4 adds notifications in the app (NTF-01) and by e-mail (NTF-02). A notification is a copy of information sent to one person, and an e-mail leaves the server, so both must show only what that person may see at the moment they see it (SEC-06, PRIV-12, ADR-0004). `bb-notify` is an empty crate, and the outbox has one reader, history (ADR-0007).

## Decision

### 1. An outbox reader builds notifications

- A second outbox reader, `notifications`, turns events into notification rows, one for each person who should hear about them. Like history, it is idempotent, keeps its place, and resumes after a restart (ADR-0007).
- **Who hears about what** (the defaults of M4-Q5; each person changes them under *Customize*):

  | Event | In the app | By e-mail |
  |---|---|---|
  | You are mentioned | on | on |
  | You are assigned | on | on |
  | A status change, a comment or a file on an issue you follow | on | off |
  | Other changes to issues you follow | off | off |
  | Your change to many issues has ended | on | off |
  | *Ask … to finish* and *Edit anyway* on your text | on | off |

- A person is never told about their own changes.

### 2. Checked when built and again when shown or sent

- When the reader builds a row, it skips a person who may not see the issue, and it leaves out every field that person may not see (the reader's mask, ADR-0004). Enforced by: SEC-MATRIX.
- When the row is shown in the app or put into an e-mail, Bugaboo checks again: a person who lost access in between gets nothing, and a follower who lost access is dropped from the followers when the reader builds the next event for the issue (M3-F132).
- People in a notification come as `PersonRef`, so *Who sees people* holds (ADR-0022).
- Enforced by: SEC-MATRIX (the e-mail and notifications column, M2-F65) and ARCH-PERSON-REF.

### 3. E-mail

- **Off at the start:** e-mail stays off until an administrator enters the organization's mail server and switches it on (PRIV-08). *Send me a test e-mail* tries the settings. The mail password is stored encrypted like other secrets (SEC-16).
- **Sending:** each e-mail is a `mail.send` job (ADR-0017), sent with `lettre`.
- **Batching:** changes to one issue within 5 minutes go into one e-mail. There is no daily summary in M4 (M4-Q6).
- **Contents:** the issue ID and title, who changed what, and the comment text, all filtered as in 2, with a link into Bugaboo. An e-mail is **plain text only**: nothing is rendered, no file is attached, no picture loads from outside. The real link comes first, before any text of a person; a comment is quoted line by line (`> `), so it cannot pass for the text of Bugaboo; control characters and the characters that turn text round or hide in it (marks of direction, zero-width characters) are removed.
- **Short e-mails:** an organization setting leaves out all issue text (title and comments). It is for places where mail leaves the company.
- **Addresses in logs:** e-mail addresses never appear in logs. Enforced by: DATA-LOGS.
- **Signing up proves the address (M1-F38, M4-Q10):** signing up on one's own is a **self-invitation**. The person gives only an address; nothing is made until they open the link sent to it (valid for 24 hours, used once). The link opens the invitation page, where they choose a name and a password and the account is made. The answer is the same for every address, and an address that has an account gets an e-mail that says so. Signing up needs e-mail.

### 4. In the app

- *Notifications* is a page and a menu entry with the number of unread ones (UX §4.11).
  - Entries are grouped by issue, newest first by when the event happened; the ID (the time the row was made) breaks a tie and is what *Mark all as read* counts up to (M5 exit).
  - Each entry has *Open*, which goes to the exact comment or change, and *Mark as read*.
- *Notifications* replaces the *Mentions* block on the start page. *Mentions* becomes a filter, and the start page shows the newest unread ones (M4-Q7).

### 5. Keeping and deleting

- Notifications are deleted after 90 days (PRIV-05, UX §5.7). A sent e-mail's job and its payload go when it is sent, or when it dies, with the dead jobs' keep period (M1-F21).
- Enforced by: DATA-MARKS and the keep-and-delete job's test.

### 6. Handles for mentions (M3-F70, M4-Q9)

- Every person gets a short **handle** (`@ana.brandt`), made from the name when the account is made, unique, and changeable in *My settings*.
- A mention stores the person, not the handle, so a changed handle never breaks an old comment.
- A handle is people information and follows *Who sees people*.
- A handle has only the ASCII letters `a`…`z`, digits, `.`, `-` and `_`. A name is spelled with them (`ä` is `ae`); a name with none of them gets the handle `person`.
- A new text resolves only the handle a person has now. The handle they gave up stays reserved, so nobody else can take it, and the texts written before keep naming them because a mention stores the person.
- A person cannot have the handle of a group or a role, and a group or a role cannot be given a name whose handle a person has or had, so an `@handle` never names two things. A person changes their handle once in seven days; the handle they were given with the account does not count.

## Consequences

- `bb-notify` holds the e-mail adapter behind a port (`Mailer` in `bb-app`). Tests use a fake mailer that keeps what it would have sent.
- Webhooks (NTF-03) and live updates later read the same outbox the same way.

## Alternatives considered

- **Sending from the use case that made the change:** the change would wait for the mail server, and a failed mail could undo nothing. Rejected; ADR-0007 already gives the outbox.
- **Filtering only when the row is built:** a person who loses access an hour later would still read yesterday's notifications. Rejected; checking again costs one permission lookup per row shown.

## Amendment (M4 WP-06)

The reader and the e-mail are built. What the build decided where this record left room:

- **The reader.** `notifications` is read by `run_due_jobs` before the jobs are taken, in batches of 200 events. A row is unique by the recipient and what it is about (`once`: the event, or the mention), so a restart that repeats a batch makes no second row. The rows, the jobs they queue and the new place of the reader are one transaction.
- **Followers.** A follower hears only of events from the time they began to follow. A follower who lost access is dropped when the reader builds the next event for the issue.
- **Shaped when shown.** A row keeps the changes and the person who made them. Every time it is listed or put in an e-mail it is shaped again for the recipient: the fields they may see, the people as *Who sees people* says (the same words as the history of an issue), the issue only if they still see it. The number of unread notifications is a route of its own, capped, and counts only rows that still show. One request for the list looks at a bounded number of rows and answers with a cursor when there are more.
- **E-mail.** Plain text only. One `mail.send` job for each person and issue is queued for 5 minutes after the first change; it takes every waiting row of that pair, and when more wait than the e-mail holds it says so and marks all of them, so a second e-mail does not repeat the first. A failed job is tried again with the pause of the job runner and the rows stay waiting; when it gives up, the rows are marked as never sent.
- **The mail server.** The password has its own routes (`PUT /api/v1/mail/password`, *Send me a test e-mail*). The test e-mail is sent at once, not as a job, and a person sends one every 10 seconds at most. Settings that would send a user name or a password without encryption to a mail server on another machine are refused (`mail.insecure`). Only the certificate authorities built into Bugaboo are trusted.
- **Signing up.** Signing up is a self-invitation (§3): `sign_up` takes an address, answers `202` with `check_your_mail` for every address, and queues a `mail.send` job; the job makes the `SignUpInvitation` (its own table, read as an invitation with nobody who invited) and sends the link, or, for an address that has an account, an e-mail that says so. A second sign-up for the same address within 10 minutes sends nothing. `sign_up` is refused while e-mail is off, and the status answer `sign_up_open` says so. `sessions.purge` deletes the links that were used or ran out.
- **Keeping.** Five more kinds of job: `notifications.purge` (rows after 90 days, and the payload of dead jobs after 30), `security-log.prune` (the setting *Days to keep the security log*, 365 at the start), `sessions.purge`, `server-log.rotate` (the size and the number of files are settings) and `mail.send`. Each can run twice with the same result.
- **Tags.** Renaming or combining tags counts each touched issue's version in the same change that rewrites its tags (M3-F126); a rename of a very large tag is therefore one long change, which a step job could split later.
- **Delivery in the matrix.** The record of SEC-MATRIX has a `delivery` line for each entry point (`notifications`, `e-mail`) and each starting role as the recipient; the check compares it with the tables of the starting roles.

## Amendment (M4 WP-08)

The screens are built. What the build decided where this record left room:

- **The count.** The number on the menu entry and on the bar of a phone is `GET /api/v1/notifications/unread`. Until live updates arrive the browser asks every 30 seconds (the time of the check of versions, ADR-0019) and when the person comes back to the tab. A count of `AT_ONCE_MAX` is shown as *100+*.
- **Open.** *Open* goes to the issue with `?comment=<ID>` for a comment and `?change=<moment>` for a change. The issue page marks the entry and reads earlier pages of the history until it finds it. A change is found by the moment it happened, to the millisecond, because the history and a notification are made from the same event.
- **The text of a comment** in a notification is HTML that the server made with `bb-markdown` for this reader (the mark `x-safe-html` on `NotificationEntry.html`), so the browser draws it as it draws a comment.
- **Customize** is *My settings › Notification settings*. Its rows come from `NotificationKind::setting`: `cargo xtask gen api` reads that function and writes the table into the browser client, so the settings of a kind are written once.
- **The mail server** is *Organization settings › Connections › E-mail & notifications*. The settings are the registry's (`mail.*`); the password goes through `PUT /api/v1/mail/password` and is never read back. `GET /api/v1/mail` says whether one is set (`credential_set`) and the server's own judgement of the mail server as entered (`readiness`: `ready`, `incomplete` when the host or the sender is missing, or `insecure` when a secret would go without encryption to another machine). It comes from the same code the test e-mail and the jobs use (`mail_server`), so the page shows the judgement and holds no copy of the rule. `credential_set` is a yes or no and not a secret: it tells an administrator who may change the mail server nothing they could not see by trying the test e-mail, and the page needs it to show *not set*. A change of the mail server, its port or the user name deletes the stored password in the same change (the event records it), so a new server is never handed the old password; the page then shows *not set*. The refusals of *Send me a test e-mail* are shown in words: not entered (`mail.off`), a user name or password in the clear to another machine (`mail.insecure`), turned away (`mail.rejected`, status 502), not reached (`mail.unavailable`), and too soon (`rate-limit.exceeded`, once in 10 seconds for a person; the button waits that long too).
- **The wait for the changes of one e-mail** is the start-up setting `mail_batch_seconds` (`BUGABOO_MAIL_BATCH_SECONDS`, 300 unless set), so a test or a demo can see the e-mail in seconds.
- **The first start** has an optional step for the mail server after the second check of the account.
- **Signing up** shows *Check your e-mail* after the form. Asking for a new link waits as long as the server does for one address (`RESEND_AFTER_MINUTES`), and the route has its own limit; a link that ran out leads to asking for a new one. While e-mail is off, the switch for signing up says so.
- **The *Mentions* block** of the start page is gone. The routes `/api/v1/me/mentions` and `/api/v1/me/mentions/read` are removed: there is one way to read mentions, the notifications.

## Amendment (M4 WP-13): one wording for the screen and the e-mail

The words of a change and of a notification are chosen once, in `bb-core` (`wording`). The server's e-mails call the code directly and the browser calls the same code through `bb-wasm` (ADR-0005), from the lazy parts only: the history of an issue, the notifications, the preview of a change to many issues and the sentence about a conflict on the issue page. They load the module with `import()` and show a skeleton until it is there (the first page does not grow; the module has its own budget, PERF-04). The workflow editor loads one more thing the same way: the rule of the same name (`bb_core::names`, `name_key`), so that *Add* and *Rename* of a status say a name is taken by the rule of the server and not by one written again in the browser (M4 WP-14); the editor keeps that check off until the module is there, and the refusal of the server stays the judge.

- **What is shared.** The choice of sentence (*changed*, *set*, *cleared*, the long text), the message keys and the sentences of every kind of notification, the words of a status and a priority, yes and no, a length of time, a list of names, and the rule that a line of an e-mail gets no full stop after a value that ends in `.`, `?` or `!`. The result is a tree of message keys and values (`Said`); it holds no English. The server renders it with its catalog, the browser looks the keys up in its own.
- **What is not shared, by decision.** Region-dependent formatting of dates, numbers and time zones is kept in two implementations, because the browser's `device` choice uses `Intl`, which the server cannot run. They are held together by vectors generated from Rust: `cargo xtask gen word-vectors` writes `tools/testdata/value-words.json` from `bb_core::wording::vectors` (the expected words are what the server writes), DOC-GENERATED fails when the file differs from that code, and the browser test `web/app/src/shared/value-words.test.ts` fails when the screen writes a case differently. The cases hold half-up rounding, the 12-hour clock, and the fixed date orders (day first, month first, year first).
- **The same vectors hold the small helpers of the screens that cannot wait for the module.** The field on the issue page (`ownValueText`, `durationText`) and the lists of names all over the app (`listText`) are written synchronously and are not part of a history or a notification. Their yes and no, length of time and list words are checked against the vectors generated from Rust, so a difference from the shared code fails the browser test. This is part of the same approval.
- **The types of the input and the output** of the module are generated from the Rust structs (`cargo xtask gen word-types` writes `web/packages/wasm/src/wording.generated.ts`, DOC-GENERATED holds it), and the module refuses an input it cannot read with an error that the app throws in development and under test, so a mismatch is found at once and not shown as empty words.
- **The input of the module** is built by each side from its own data (the API items of the browser, the shaped notifications of the server): the names of people, the name of a field and, for a number, a day or a moment, the words of the region. Mapping a data shape to that input is not wording.

## Amendment (M6, the time of day and lengths of time)

The maintainer decided M6-Q2 (M5-F137) by taking the recommendation as written. It changes what the vectors of the wording amendment hold; the approval of the two implementations stays as it is, and no new exception is added.

- ***Region* has a fourth choice, *Time of day*:** *24-hour (14:30)* or *12-hour (2:30 PM)*. Its default follows the date format: *Match my device* takes the device's clock, and every fixed date format starts with *24-hour*. A server does not know the device, so an e-mail takes the organization's choice for a person who left it on the device, like the other choices of *Region*.
- **One writer for each kind of value.** A date, a moment (a date and a time of day), a time of day and a length of time are written from the person's *Region* by one helper each: `bb_core::wording` for e-mails and the lazy parts, and the synchronous helpers of `web/app/src/shared/` for the screens that cannot wait for the module. So a date written day first never stands beside a 12-hour clock unless the person chose both, and the timer and the timesheet write their times the same way. The writer of lengths has three forms, written here only: the grid form `1:30` (a grid and a running timer), the short form `1h 30m` (a field, a list column, a short status line, a history line) and the words `1 hour 30 minutes` (a sentence: a warning, a window, a notification, an e-mail), for people who do not read abbreviations. UX §4.9 lists which screen uses which. The rule for the screens is written in coding-standards §4 (TXT-DATES); the two implementations are held together by DOC-GENERATED and the browser test of the vectors, as before.
- **What the vectors hold from M6:** both clocks with every fixed date order (day first, month first, year first), the moments around midnight and noon in each, and the lengths of time that the one reader of `bb_core::time` reads (`1:30`, `1.5`, `1,5` with a decimal comma, `90m`) with what the writer gives back for each. `cargo xtask gen word-vectors` writes them from Rust, as before.

## Amendment (M7, agents)

Agent work ([ADR-0029](0029-ai-agents.md)) adds six kinds of notification, built by the same reader and checked again in the same way (decision 2). Each person changes them under *Customize*; the starting values:

| Event | In the app | By e-mail |
|---|---|---|
| *An agent asks you* (a question of an agent of yours, or on an issue you handed over) | on | on |
| *A plan waits for you* (*Plan needs approval*) | on | on |
| *Ready for review* | on | on |
| *An agent gave up* | on | off |
| *A standing rule runs out soon* (3 days before its end) | on | off |
| *An agent was stopped* (by an administrator, by a limit, or because its person lost access) | on | off |

- The first three wait for the person, so they are on by e-mail at the start: a question can be answered from the phone (the M7 exit demo).
- The text of a question, a plan or a note is people's text and is written like a comment (decision 3: quoted, plain text, after the link). An agent is shown as *"Fixer, agent of Ada Brandt"*, its person by *Who sees people* (ADR-0022).
- The responsible person and the person who handed the issue over both hear of the first four; an issue they no longer see is skipped like any other (decision 2). Enforced by: SEC-MATRIX (the notifications column) and the tests of WP-08.

## Amendment (M8, work times and the time factor)

Work times and the time factor ([ADR-0030](0030-planning-with-people.md)) add two kinds of notification, built by the same reader and checked again in the same way (decision 2). They go to the person whose data changed, and only when somebody else changed it. Each person changes them under *Customize*; the starting values:

| Event | In the app | By e-mail |
|---|---|---|
| *Your work times were changed* (somebody with *Enter work times for others* entered or changed an away time, your usual work days or your work hours) | on | off |
| *Your time factor was changed* (somebody else set the time factor of an issue whose person on the bar you are) | on | off |

- An away time is named only by its days (*"Ada Brandt entered an away time for you: 26–30 Oct"*); it has no reason to name (ADR-0030, decision 2). The text of the note of an away time is never sent: only its person sees it.
- Neither kind names another person's work times, and no notification goes to anybody else about them. Enforced by: SEC-MATRIX (the notifications column) and the tests of WP-04 and WP-06.

## Amendment (M9, Download my data)

*Download my data* ([ADR-0044](0044-download-my-data-and-remove-a-person.md), decisions 2 and 4) adds two kinds of notification, built by the same reader from the events of the export and checked again in the same way (decision 2). Neither is about an issue: its e-mail names the kind in its subject and links to *Notifications*.

| Event | In the app | By e-mail |
|---|---|---|
| *Your data is ready* (the file you asked for can be downloaded; told to whoever asked for it) | on | on |
| *An administrator downloaded your data* (somebody with *Download someone's data* created a file of your data; told with the reason) | always | always |

- *Your data is ready* is changed under *Customize* like any other kind. The person waits for the file, so it is on by e-mail at the start.
- *An administrator downloaded your data* has no setting: nobody can switch it off, not the person and not the organization, because the person must learn that somebody took their data (decision 4). *Customize* shows it as always on. It is told when the administrator asks for the file, so it reaches the person even when the job that writes the file fails. The administrator is shown as *Who sees people* lets the person see authors, never by name when that hides them. The e-mail goes to the address on record, like every e-mail of a notification; e-mail itself must be switched on, because without a mail server nothing can be sent. Enforced by: the tests of the notices of exports in `bb-app`.

## Amendment (security fixes before the first release): a sign-in refused for the day

The limit on failed tries (SEC-14, ADR-0025) refuses sign-in to an account from networks it never signed in from once those networks have used the tries of a day at it. The person learns of it from one more kind of notification, built by the same reader from an event of their record (`sign_in_capped`, by the system) and checked again in the same way (decision 2). It is about no issue: its e-mail names the kind in its subject and links to *Notifications*.

| Event | In the app | By e-mail |
|---|---|---|
| *Sign-in to your account was refused for today* (somebody tried to sign in to your account so often that tries from networks you never signed in from are refused until tomorrow) | always | always |

- It has no setting, like *An administrator downloaded your data*: somebody is trying to get into the person's account, and only the person can tell whether it was them. It is told once a day at most, with the first refusal.

## Amendment (2026-11-24, e-mailed searches)

The maintainer took the recommendation of M10-Q16 of [the M10 plan](../milestones/M10/README.md). *E-mail me this search* (QRY-07) sends a saved search by e-mail on a schedule the person chooses. It is not a notification: no event starts it, and the outbox reader does not build it. But it leaves the server with issue data, so it follows the rules of decision 3. Decided:

1. **To the person themselves only.** An e-mailed search goes to the address on record of the person who set it, never to anybody else; there is no field for another address. Enforced by: SEC-MATRIX (the routes of e-mailed searches answer only their person).
2. **When:** on the weekdays the person chooses (at least one), at a whole hour of the person's time zone (*Region*). Each sending is a job of its own, spread over the hour, so many people at 8:00 do not run at once (ADR-0017).
3. **What:** the search runs as the person at sending time, with their rights then (QRY-05). The e-mail holds the name of the saved search and at most 50 issues in its order, each with its ID, title, status and link, then "and 120 more" when the search finds more (a count of what the person sees), a link to the search and a link to the e-mailed search in Bugaboo, where it is stopped. The links open Bugaboo and need a sign-in; nothing in the e-mail changes anything. Plain text by the one writer of decision 3: no picture, no attachment, the links first, control and direction characters removed. With *Short e-mails* on it leaves out the titles and the name of the search: IDs, statuses and links only. Enforced by: SEC-MATRIX (the e-mail as the person sees the issues), DATA-LOGS (the job logs IDs only).
4. ***Only when something is new or changed*** is on at the start: nothing is sent when no issue the search finds is new or changed since the last e-mail. The e-mailed search keeps only the time of its last e-mail, never the issues.
5. **It pauses and it ends.** Nothing is sent while it is paused, and the person sees why in their list. Two pauses end by themselves: while e-mail or the organization setting `search-mail.allowed` is off, nothing is sent, and sending goes on when it is on again. Two pauses wait for the person: when the person is switched off, and when they can no longer open the saved search ([ADR-0034](0034-saved-searches.md), amendment 2026-11-24); only the person starts it again. It is deleted with its saved search and at *Remove a person* ([ADR-0044](0044-download-my-data-and-remove-a-person.md), amendment 2026-11-24). Enforced by: SEC-MATRIX, ARCH-SETTINGS.
6. **Limits and switches.** At most 20 e-mailed searches per person, counted in the change that writes. E-mail itself must be switched on (decision 3). The organization setting `search-mail.allowed` switches the feature off for everyone; it is on at the start, because e-mail is off until an administrator sets it up. The list of a person's e-mailed searches is under *My settings › Notification settings*. Enforced by: ARCH-SETTINGS.
7. **No tool sets one.** AI apps and agents never set, change or stop an e-mailed search; the routes take a session only, like those of saved searches (M10-Q20). Enforced by: ARCH-AI-TOOLS, SEC-MATRIX.

How it is built (QRY-07): the e-mailed searches are the table `search_mail_schedule` (migration 68), one row of a saved search for a person, with the weekdays, the hour, why it waits for its person, when its next e-mail is due and when its last one went. The job `search-mail.find-due` comes every hour and queues one `search-mail.send` job for each e-mailed search whose hour starts before its next round, at the e-mailed search's own fixed place in the hour (`bb_core::search_mail::place_in_hour`), and moves it on to its first day after that next round in the person's time zone, both in one change; an hour missed while the server did not run is sent once, late, and stands for every hour up to the next round. The job names the hour it was due for; it sends nothing when its person changed the e-mailed search after it was queued, or when the e-mailed search is due again at that hour or before, so saving or changing it while its e-mail waits never brings two in one hour. `search-mail.send` is the one place that decides whether an e-mail goes (`bb_app::search_mail::job`): the search runs through the one search as the person (`Ctx::open_search`, `prepare_search`), *Only when something is new or changed* asks the same search for its newest *Changed*, and the text is written by the writer of the notifications (`notifications/mail.rs`, `search_mail_text`). The pause *the search no longer opens* covers a saved search in the trash too, which can come back; the e-mailed searches of a saved search go with its row (at *Remove a person* of its maker).

Enforced by (additional): SEC-MATRIX (`listSearchMails`, `setSearchMail`, `stopSearchMail` for every actor, *not-for-keys* for both keys; and the entry point `search-mail` of the delivery column: the e-mail of an e-mailed search of every starting role shows the status and the title as the tables of the starting roles say, and a switched-off person of each role gets none), DATA-LOGS (the job logs the ID of the e-mailed search only), ARCH-PERSON-REF (the address read in `run_search_mail` only), the tests `qry_07_*` of `bb-server` on both databases (to the person only, rights at sending time, 50 issues and how many more, *Short e-mails*, nothing new or changed, the hour across summer time, a switched-off person, a search that no longer opens, e-mail or the setting off, 20 per person, a field the person may not read; in `search_mail_rounds_http.rs`: saved again or changed while the e-mail waits, an hour missed, switched off and on again, what the person loses before the e-mail, a change of a field they may not read, somebody else's private search) and of `bb-core` (the next day and the place in the hour), and the browser test `search-mail.spec.ts`.

## Enforced by

SEC-MATRIX (the e-mail and notifications column), DOC-GENERATED and the browser test of the vectors (the wording amendment), ARCH-PERSON-REF, ARCH-EVENTS, DATA-MARKS, DATA-LOGS, DEP-REGISTER (`lettre`), the permission tests of M4 WP-06, and review (no other path sends mail).
