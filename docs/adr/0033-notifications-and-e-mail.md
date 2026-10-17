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
  | Other changes to fields of issues you follow | off | off |
  | Your run of many changes ended | on | off |
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
  - Entries are grouped by issue, newest first.
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
- **The mail server** is *Organization settings › Connections › E-mail & notifications*. The settings are the registry's (`mail.*`); the password goes through `PUT /api/v1/mail/password` and is never read back. `GET /api/v1/mail` says only whether one is set (`credential_set`). The refusals of *Send me a test e-mail* are shown in words: not entered (`mail.off`), a user name or password in the clear to another machine (`mail.insecure`), turned away (`mail.rejected`, status 502), not reached (`mail.unavailable`), and too soon (`rate_limit.exceeded`, once in 10 seconds for a person; the button waits that long too).
- **The wait for the changes of one e-mail** is the start-up setting `mail_batch_seconds` (`BUGABOO_MAIL_BATCH_SECONDS`, 300 unless set), so a test or a demo can see the e-mail in seconds.
- **The first start** has an optional step for the mail server after the second check of the account.
- **Signing up** shows *Check your e-mail* after the form. Asking for a new link waits as long as the server does for one address (`RESEND_AFTER_MINUTES`), and the route has its own limit; a link that ran out leads to asking for a new one. While e-mail is off, the switch for signing up says so.
- **The *Mentions* block** of the start page is gone. The routes `/api/v1/me/mentions` and `/api/v1/me/mentions/read` are removed: there is one way to read mentions, the notifications.

## Enforced by

SEC-MATRIX (the e-mail and notifications column), ARCH-PERSON-REF, ARCH-EVENTS, DATA-MARKS, DATA-LOGS, DEP-REGISTER (`lettre`), the permission tests of M4 WP-06, and review (no other path sends mail).
