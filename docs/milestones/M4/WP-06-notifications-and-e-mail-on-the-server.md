# WP-06 — Notifications and e-mail on the server

**Wave 2 · Folder / branch:** `bugaboo.win4` / `WIN4`
**Features:** NTF-01, NTF-02 (server), SEC-06 (notifications and e-mails), SEC-16 (the mail password), PRIV-05 (keep periods), PRIV-08 (e-mail off at the start), PRIV-12, ISS-07 (following delivers); ADR-0033, ADR-0007, ADR-0017, ADR-0022; follow-ups M3-F70, M3-F75, M3-F125, M3-F126, M3-F132, M3-F150, M3-F155, M2-F65, M1-F21, M1-F43, M1-F61.

## Goal

People hear about what matters to them, in the app and by e-mail. Nothing they hear shows a field, a name or an issue they may not see, not even an hour after they lost access.

## Owns

- `crates/bb-notify/**` (the mail adapter with `lettre`) and the port `Mailer` in `bb-app`, with a fake mailer for tests.
- `crates/bb-app/src/notifications/**` (new): the notifications reader and the settings per person.
- `crates/bb-store/src/notifications.rs` (new) and migration `m0017`.
- `crates/bb-api/src/api/notifications*` (new).
- The mail settings of the organization and the person in the settings catalogue.
- Job kinds `mail.send`, `notifications.purge`, `security-log.prune`, `sessions.purge`, `server-log.rotate`.
- Handles for people (M3-F70).
- The e-mail and notifications column of SEC-MATRIX (M2-F65).
- Third-party row for `lettre`.

The screens are WP-08.

## Tasks

1. **The notifications reader (ADR-0033 §1):** an outbox reader named `notifications`, idempotent, resuming. It builds rows for the events and defaults of M4-Q5, and never for the person's own changes.
2. **Checked twice (§2):**
   - When the row is built: the recipient's mask and *Who sees people*.
   - When it is read or sent: the issue is still visible. A follower who lost access is dropped (M3-F132).
   - Mentions and *Ask … to finish* are delivered through the same path (M3-F75).
3. **Runs that end (M3-F125, F150):**
   - The end of a run of many changes is an event its person hears about.
   - A dead job's person hears about it too.
   - Renaming or combining a tag counts each issue's version (M3-F126), in the step job.
4. **E-mail (§3):**
   - **The mail server:** settings with the password encrypted (SEC-16), e-mail off until switched on (PRIV-08), and *Send me a test e-mail* as a route.
   - **Sending:** each e-mail is a `mail.send` job. Changes to one issue within 5 minutes go into one e-mail. *Short e-mails* is an organization setting. Text is escaped and nothing loads from outside.
   - **Addresses** never appear in logs.
5. **Confirming an address (M1-F38) on the server:** a link for 24 hours, and the account works only after it, when mail is on.
6. **Handles (§6, M3-F70):**
   - Unique, made from the name, changeable.
   - Mentions store the person, not the handle.
   - The handle follows *Who sees people*.
7. **Keeping and deleting (§5, M1-F21, F43, F61):**
   - Notifications are deleted after 90 days.
   - The payloads of dead jobs have a keep period.
   - The security log has a keep setting and a prune job.
   - Expired sessions are removed by a job.
   - The Windows service's `logs/server.log` is rotated.
   - Each job has its idempotency test (ADR-0017).
8. **The job runner's own test (M3-F155):** the loop of `bb-jobs` gets a test, and its constants are named.

## Acceptance tests (write first)

- `ntf_01_*`:
  - `ntf_01_a_follower_hears_of_a_status_change_but_not_of_their_own`;
  - `ntf_01_a_mention_reaches_the_person_and_the_members_of_a_group`;
  - `ntf_01_the_reader_resumes_after_a_restart_without_doubles` (both databases).
- `sec_06_*`:
  - `sec_06_a_notification_leaves_out_a_field_the_recipient_cannot_see`;
  - `sec_06_a_person_who_lost_access_gets_nothing_more`;
  - `sec_06_a_follower_without_access_is_dropped`.
- `ntf_02_*`:
  - `ntf_02_an_e_mail_holds_only_what_the_recipient_may_see` (the fake mailer);
  - `ntf_02_changes_within_five_minutes_make_one_e_mail`;
  - `ntf_02_short_e_mails_leave_out_all_issue_text`;
  - `ntf_02_no_e_mail_goes_while_mail_is_off`.
- `sec_16_the_mail_password_is_stored_encrypted`; `data_logs_no_e_mail_address_in_the_log` (DATA-LOGS).
- `priv_05_notifications_go_after_90_days`.
- `auth_*`:
  - `auth_a_signed_up_account_works_only_after_the_link_when_mail_is_on`;
  - `auth_the_link_runs_out_after_24_hours`.
- SEC-MATRIX: the e-mail and notifications column for every starting role.

## Checks

SEC-MATRIX grows the e-mail and notifications column. DATA-LOGS, DATA-MARKS, ARCH-PERSON-REF and ARCH-EVENTS cover the new code.

## Depends on

WP-01 (ADR-0033), WP-02 (the new names, the free migration number).
