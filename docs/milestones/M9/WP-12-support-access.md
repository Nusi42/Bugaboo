# WP-12 — *Support access* with the customer's approval

**Wave 3 · Folder / branch:** `bugaboo.win5` / `WIN5` · **Model:** Opus
**Features:** DIST-03 (server and screen). [ADR-0048](../../adr/0048-support-access-with-the-customers-approval.md); ADR-0012; architecture §12, boundary 4.

## Goal

Nobody outside the organization can get in by default. An administrator can let one named supporter in for a few hours, with a role they choose and two-step sign-in, every request of the supporter in the security log, and the access ending by itself or at once on demand.

## Owns

The record and its rules in `crates/bb-core/src/support_access.rs` (new, pure: the end at most `support-access.longest`, the role no stronger than the maker's); migration 59 (`m0059_support_access.rs`: the supporter's name and e-mail, the reason, the role and its places, the start, the end, who made it, the one-time link's keyed digest); the use cases in `crates/bb-app/src/support_access/`; the job kind `support-access.end` (queued for each access's end, and a check every minute for ends that passed); the settings `support-access.longest` and `support-access.start-length` in `crates/bb-core/src/settings/support_access.rs` (new); the routes in `crates/bb-api/src/api/support_access.rs` (`GET` and `POST /api/v1/admin/support-access`, `POST /api/v1/admin/support-access/{access}/end`, `POST /api/v1/auth/support-access/accept`); the request log of a supporter in the security log (a layer of `bb-api` that writes route and record for each request of a support account, never the body); `PersonRef` for a support account (*Sam Lee (support access until 18:00)*); the screen `web/app/src/features/settings/SupportAccessPage.tsx` and the accept page `web/app/src/features/sign-in/SupportAccessAccept.tsx` (new); their texts; `userDocs/people/support-access.md`.

## What is decided

- **Nothing by default** (ADR-0048, decision 1): no account, key or route exists for us; the test of default settings (WP-08) includes that the list of support accesses is empty at the start.
- **Making it** (decision 2, M9-Q13 as answered): *Change organization settings*; a name, an e-mail address, a reason, a role among the roles that hold no permission the maker lacks (the start: *Viewer*), the projects it counts in, an end (8 hours at the start, at most 72); Bugaboo shows a one-time link good for one hour, which the administrator copies; e-mail is not needed.
- **Signing in** (decision 3): the link sets a password and a code app or passkey first; the account can make no access key, no AI app grant and no agent (the routes refuse a support account); it cannot make another support access, invite people or change roles, whatever its role says.
- **The record** (decision 4): making, accepting, each sign-in, every request (route and record ID), the end, each a security-log entry; a support account reads as *Name (support access until HH:MM)* everywhere a person is named.
- **The end** (decision 5): at its moment, by the job, or on *End now* by any administrator: the sessions end, the account is removed through WP-06's removal (its security-log entries stay), and the list shows it as ended with its log.

## Tasks

1. Write the acceptance tests below.
2. The record and its rules; migration 59 and its marks.
3. The use cases, the job and the settings; the routes; the request log layer.
4. `PersonRef` for support accounts.
5. The screen and the accept page; pictures in four themes on computer and phone.
6. The user docs.

## Acceptance tests (write first)

- `dist_03_nobody_outside_gets_in_by_default` (no support access at the start; no route answers without the organization's session or key).
- `dist_03_a_support_access_cannot_be_stronger_than_its_maker`.
- `dist_03_the_end_is_at_most_72_hours_away`.
- `dist_03_the_link_works_once_and_for_an_hour`.
- `dist_03_the_supporter_sets_a_second_step_before_anything_else`.
- `dist_03_a_support_account_makes_no_key_grant_agent_or_invitation`.
- `dist_03_every_request_of_the_supporter_is_in_the_security_log_without_its_body`.
- `dist_03_a_request_after_the_end_is_refused` (the clock of the app moved one minute past the end).
- `dist_03_end_now_ends_the_sessions_at_once`.
- `dist_03 an administrator makes support access copies the link and ends it` (browser, four themes).
- SEC-MATRIX: rows for the four routes for every starting role, an access key and a support account, both databases.

## Checks

ARCH-PERMS, ARCH-SETTINGS, ARCH-EVENTS, ARCH-PERSON-REF, ARCH-AI-TOOLS, DATA-MARKS, DATA-MIGRATIONS, DATA-LOGS, SEC-MATRIX, TXT-LITERAL, TXT-CATALOG, TXT-TERMS, THEME-ONLY, TEST-A11Y.

## Review

Code review with the SSOT/DRY/ODR gate: the removal at the end is WP-06's, the link is the invitation's way of a link that runs out, the second step is sign-in's. **Security review:** no second way in, the role check, the end, the request log, the link. **UX review** of the pictures.

## Depends on

WP-01 and WP-06 merged. Runs beside WP-10, WP-11 and WP-13.
