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
- When the row is shown in the app or put into an e-mail, Bugaboo checks again: a person who lost access in between gets nothing, and a follower who lost access is dropped from the followers (M3-F132).
- People in a notification come as `PersonRef`, so *Who sees people* holds (ADR-0022).
- Enforced by: SEC-MATRIX (the e-mail and notifications column, M2-F65) and ARCH-PERSON-REF.

### 3. E-mail

- **Off at the start:** e-mail stays off until an administrator enters the organization's mail server and switches it on (PRIV-08). *Send me a test e-mail* tries the settings. The mail password is stored encrypted like other secrets (SEC-16).
- **Sending:** each e-mail is a `mail.send` job (ADR-0017), sent with `lettre`.
- **Batching:** changes to one issue within 5 minutes go into one e-mail. There is no daily summary in M4 (M4-Q6).
- **Contents:** the issue ID and title, who changed what, and the comment text, all filtered as in 2, with a link into Bugaboo. Text is escaped for e-mail; no file is attached; no picture loads from outside.
- **Short e-mails:** an organization setting leaves out all issue text (title and comments). It is for places where mail leaves the company.
- **Addresses in logs:** e-mail addresses never appear in logs. Enforced by: DATA-LOGS.
- **Confirming an address (M1-F38, M4-Q10):** once mail is on, signing up on one's own sends a confirmation link, valid for 24 hours, and the account works only after it.

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

## Consequences

- `bb-notify` holds the e-mail adapter behind a port (`Mailer` in `bb-app`). Tests use a fake mailer that keeps what it would have sent.
- Webhooks (NTF-03) and live updates later read the same outbox the same way.

## Alternatives considered

- **Sending from the use case that made the change:** the change would wait for the mail server, and a failed mail could undo nothing. Rejected; ADR-0007 already gives the outbox.
- **Filtering only when the row is built:** a person who loses access an hour later would still read yesterday's notifications. Rejected; checking again costs one permission lookup per row shown.

## Enforced by

SEC-MATRIX (the e-mail and notifications column), ARCH-PERSON-REF, ARCH-EVENTS, DATA-MARKS, DATA-LOGS, DEP-REGISTER (`lettre`), the permission tests of M4 WP-06, and review (no other path sends mail).
