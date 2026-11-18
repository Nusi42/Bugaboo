# WP-09 — Correct your own data, and corrections by administrators

**Wave 2 · Folder / branch:** `bugaboo.win4` / `WIN4` · **Model:** Opus
**Features:** PRIV-06 (server and screen). [ADR-0044](../../adr/0044-download-my-data-and-remove-a-person.md) decision 9; ADR-0022; follow-up M4-F122.

## Goal

Every person can correct their own name and change their e-mail address, which takes effect only after they open a link sent to the new address, with a note to the old one. An administrator with *Manage people* corrects the name and e-mail address of others with a reason; the person is told, and the security log records it.

## Owns

The use cases in `crates/bb-app/src/people/corrections.rs` (new); migration 58 (`m0058_e_mail_changes.rs`: a pending change with its keyed digest of the token, the new address, the moment, its end); the keyed digest of addresses and tokens with a key of the server in `keys/` (M4-F122: the digest of the memory of sign-up e-mails moves to the same keyed digest, one function); the routes in `crates/bb-api/src/api/me_profile.rs` (`PUT /api/v1/me/name`, `POST /api/v1/me/e-mail-change`, `POST /api/v1/auth/e-mail-change/confirm`) and in `crates/bb-api/src/api/people.rs` (`PUT /api/v1/people/{person}/name`, `PUT /api/v1/people/{person}/e-mail`); the e-mails *Confirm your new e-mail address* and *Your e-mail address was changed* and the notification *An administrator corrected your details* in `crates/bb-notify/` and `crates/bb-app/src/notifications/`; the screen *My settings › Profile* in `web/app/src/features/my-settings/ProfilePage.tsx` (new) and the corrections on a person's page in `web/app/src/features/people/PersonPage.tsx`; their texts; `userDocs/my-settings/profile.md`.

## What is decided

- **Own name:** changed at once, like a short value with *Undo* (the UX concept); the history and the security log keep the change; *Who sees people* applies to who sees the new name as to the old.
- **Own e-mail address:** asks for the password (or a passkey) again; sends a link to the new address (one hour, one use, its token stored only as a keyed digest); the old address keeps working until the link is opened; then the old address gets *Your e-mail address was changed* with the moment and, if it was not them, what to do. An address that another account uses is answered exactly like a free one, and the e-mail to that address says an account exists there instead (no answer tells whether an address is taken). E-mail must be on; when it is off, the screen says an administrator changes the address.
- **By an administrator** (`manage-people`, which already "corrects their names"; the sentence grows by "and e-mail addresses"): name and e-mail address of others, each with a reason; an e-mail change by an administrator takes effect at once (the administrator vouches for it) and both addresses are told; the person gets *An administrator corrected your details* with the reason; the security log records who changed what of whom (not the old values in clear: their keyed digests). No key or agent may call these routes.
- **Other personal data** keeps its own way of correction (work days with a reason, ADR-0035; work times, ADR-0030; settings): the page *My data* (WP-10) says where each is corrected.
- **Sessions:** a change of the e-mail address does not end other sessions; the e-mail to the old address offers *Sign out everywhere*.

## Tasks

1. Write the acceptance tests below.
2. The keyed digest and its key; the sign-up memory moved onto it (M4-F122).
3. Migration 58; the use cases; the routes.
4. The e-mails and the notification.
5. *My settings › Profile* and the corrections on a person's page, with pictures in four themes on computer and phone.
6. The user docs.

## Acceptance tests (write first)

- `priv_06_a_person_corrects_their_own_name_and_it_is_in_the_history`.
- `priv_06_a_new_e_mail_address_counts_only_after_its_link_is_opened`.
- `priv_06_the_old_address_is_told_when_the_change_is_done`.
- `priv_06_a_taken_address_is_answered_like_a_free_one`.
- `priv_06_the_link_runs_out_after_an_hour_and_works_once` (the clock of the app moved).
- `priv_06_only_manage_people_corrects_others_and_needs_a_reason`.
- `priv_06_the_person_is_told_and_the_security_log_records_the_correction`.
- `sec_16_the_memory_of_sign_up_e_mails_keeps_a_keyed_digest` (M4-F122).
- Browser: `priv_06 the profile page changes the name with Undo`, `priv_06 a change of e-mail says to open the link` (four themes, computer and phone).
- SEC-MATRIX: rows for the five routes for every starting role and an access key, both databases.

## Checks

ARCH-PERMS, ARCH-EVENTS, ARCH-PERSON-REF, DATA-MARKS, DATA-MIGRATIONS, DATA-LOGS, SEC-MATRIX, SEC-SECRETS, TXT-LITERAL, TXT-CATALOG, TXT-TERMS, THEME-ONLY, TEST-A11Y, PERF-N1.

## Review

Code review with the SSOT/DRY/ODR gate: one keyed digest, one way to send a link that runs out (the invitation's, reused). **Security review:** account takeover through an e-mail change, what an answer tells about other accounts, the token's storage, the second check. **Privacy review:** what the security log keeps of old values. **UX review** of the pictures, and frame samples of the name saved in place (nothing moves under the cursor).

## Depends on

WP-01 merged. Runs beside WP-06, WP-07 and WP-08 (people's use cases and its own routes; `PersonPage.tsx` is shared with WP-10, which lands later and rebases).
