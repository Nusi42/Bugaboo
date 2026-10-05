# Security: threat model and personal-data flows

Part of the security and data-flow documentation (COMP-07) for the CRA technical file and for customers' own risk assessments. It covers what is built so far and grows with every milestone: each milestone review checks that new entry points appear here.

The method is STRIDE per trust boundary (boundaries from [Context](02-context.md)). Every threat names the mitigation and what proves it: a check from the catalogue (`tools/checks/catalog.toml`), a test, or *review* when a person decides.

## Scope (status: M1)

| In scope now | Comes later (own section then) |
|--------------|-------------------------------|
| HTTPS and start-up config, security headers, sign-in (password, passkey, two-step codes), sessions, invitations, first-run setup code, rate limits, the security log, stored secrets, logs | Permissions engine (M2), search language (M3), attachments (M5), MCP and the assistant (M9), webhooks and mail (M4), the served model's control plane (M14) |

## Attack surface

```mermaid
flowchart LR
  browser(["Browser<br/>(untrusted)"])
  console(["Server console<br/>(operator)"])
  subgraph server["bugaboo process"]
    tls["HTTPS<br/>rustls"]
    hdr["Security headers<br/>rate limits"]
    api["/api/v1<br/>problem+json"]
    auth["Sign-in<br/>sessions, CSRF"]
    store["bb-store"]
  end
  subgraph data["Data folder (owner-only)"]
    db[("database")]
    keys["keys/master.key"]
    cert["tls/"]
    logs["logs"]
  end
  browser -- "1 · HTTPS" --> tls --> hdr --> api --> auth --> store --> db
  console -- "setup code<br/>bugaboo setup" --> server
  store --> keys
  tls --> cert
  server --> logs
```

Entry points in M1: the HTTPS listener (pages, assets, `/api/v1`, `/health/*`), the console and `bugaboo.toml`, and the data folder on disk.

## Threats and mitigations

### Boundary 1: browser ↔ server

| STRIDE | Threat | Mitigation | Proven by |
|--------|--------|-----------|-----------|
| Spoofing | Password guessing and credential stuffing | Argon2id, breached-password list, slow-down per account, person and network (failed tries only, reserved before they are checked so parallel requests are not parallel guesses, cleared by a success), a window that doubles up to 15 minutes after every window that ended with a failure (pacing does not help), never a permanent lock; the keys of a network keep a window of a minute and do not grow, because many people share a network | `sec_14_only_failed_tries_count_and_a_success_clears_them`, `sec_14_parallel_wrong_tries_at_one_account_get_no_more_than_the_allowance`, `sec_14_pacing_the_tries_just_under_the_allowance_does_not_dodge_the_growing_wait`, `sec_14_the_window_of_a_network_stays_a_minute_however_often_it_fails`, `sec_14_only_a_finished_second_step_clears_the_wrong_codes_of_a_person`, `sec_14_failures_of_one_network_are_limited_whatever_the_account`, `sec_14_failed_sign_ins_are_turned_away_with_a_wait_and_the_answers_look_alike`, `sec_14_one_network_is_limited_in_its_tries_whatever_the_account`; `auth_01_*` (password rules) |
| Spoofing | Finding out which e-mail addresses have accounts | Same answer and same work for unknown accounts and wrong passwords; no error says which of the two was wrong | `auth_01_an_unknown_address_and_a_wrong_password_get_the_same_answer`, the timing test `auth_01_an_unknown_account_takes_as_long_as_a_wrong_password`; ARCH-ERRORS |
| Spoofing | Stolen password alone is enough | Two-step codes (TOTP with backup codes) and passkeys; the setting for admins to require them per role is declared and takes effect with roles (M2) | `auth_03_*`, `auth_02_*` (`crates/bb-server/src/tests/sign_in/`) |
| Spoofing | Guessing or replaying a code of the second step after a password was stolen | A code works once (its step is written in the same change that opens the session), wrong codes count against the person and, after 10 in one half-finished sign-in, end it, backup codes are 80 bits and stored hashed | `auth_03_signing_in_with_a_code_opens_a_new_session_and_a_code_works_once`, `sec_14_wrong_second_step_codes_are_limited_per_person`, `sec_14_parallel_wrong_codes_at_one_half_finished_sign_in_get_no_more_than_the_allowance`, `sec_14_ten_wrong_codes_end_the_half_finished_sign_in_and_are_written_down`, `auth_03_a_code_step_is_accepted_once_even_when_requests_race` |
| Elevation of privilege | A thief with a stolen session adds a code app or a passkey of their own and keeps access | Adding a way to sign in asks for the password again (when it starts and when it is finished) | `sec_12_a_stolen_session_cannot_add_a_code_app_without_the_password`, `sec_12_a_stolen_session_cannot_add_a_passkey_without_the_password` |
| Spoofing | A passkey answer replayed, made by another passkey, or for another person | A challenge works once, for 5 minutes and for one person; the passkey is one the person registered; the answer is checked by `webauthn-rs`. A new passkey is asked to be discoverable (`residentKey: preferred`), so that it can also sign in on its own, without the password; a device that cannot store one still works as a second step | `auth_02_a_passkey_finishes_signing_in_after_the_password`, `auth_02_a_passkey_signs_in_on_its_own` |
| Spoofing | Someone on the network claims a fresh installation | One-time setup code printed on the server console only (a server that runs as a service has no console, so the code goes into `setup-code.txt` in the data folder, made in a folder of its own, under a name with random bytes, made new with a plain create (a name that is taken is never used, whoever made it) that only the account can enter (mode `0700`, or on Windows an access list for the account alone), made private itself (mode `0600`, or on Windows an access list for the service account with full control and the administrators of the computer with read, nothing inherited), filled, and only then moved into place, so nobody who can read the data folder can catch it half-made, and a failed write leaves nothing behind; when it cannot be saved the console says so and where else to look, never printing the code; the code is dead once the administrator exists, and the file is deleted at the next start, M1-F58); stored hashed, replaced at every start, used up by the change that makes the administrator, wrong codes limited | `auth_09_*` (`setup_and_password.rs`) and the TLS test `sec_12_over_tls_and_http2_a_person_sets_up_signs_out_and_signs_in_again` |
| Spoofing | Invitation links reused or guessed | 256-bit random, stored hashed, single use, 7-day expiry; the answer never says why a link does not work | `auth_06_an_administrator_invites_and_the_link_works_once`, `auth_06_an_invitation_runs_out_after_seven_days`, `auth_06_two_people_with_the_same_link_make_one_account` |
| Tampering | Cross-site request forgery | `__Host-` cookie with SameSite, `Origin` check, per-session token header checked for every change in constant time | `sec_12_a_change_needs_the_token_of_its_own_session`, `sec_12_every_route_that_changes_something_checks_the_token_except_signing_in_and_up`, `sec_12_every_session_has_its_own_secret_and_its_own_csrf_token` |
| Tampering | Session fixation | A new secret at sign-in, when the second step is done and when the password is changed | `auth_03_signing_in_with_a_code_opens_a_new_session_and_a_code_works_once`, `sec_12_changing_the_password_ends_other_sessions_and_gives_a_new_secret`, `sec_12_every_session_has_its_own_secret_and_its_own_csrf_token` |
| Tampering | Script injection into pages | CSP without inline code or `eval`, Trusted Types, one Markdown renderer | SEC-HEADERS; SEC-PATTERNS |
| Tampering | Clickjacking | `frame-ancestors 'none'` | SEC-HEADERS |
| Information disclosure | Traffic read on the network | HTTPS by default; plain HTTP only on loopback or behind a configured trusted proxy; HSTS | TLS start-up tests (WP-04); SEC-HEADERS |
| Information disclosure | Error messages leak internals or hidden things | problem+json with a code and message key only; details go to the log | ARCH-ERRORS |
| Information disclosure | Passwords, session secrets, codes or links end up in events, the security log or logs | Credentials are never part of the changes of an event; the log holds IDs, actions and shortened networks only | `sec_16_no_secret_is_written_to_an_event_or_to_the_security_log`, DATA-LOGS |
| Information disclosure | Path tricks reach files outside the app (`..`, Windows short names, odd percent-encoding) | Strict path decoding; 8.3 short names refused | static-file tests (WP-04, M0-F08) |
| Denial of service | Anonymous starts of passkey sign-in fill the waiting challenges so that the second step or adding a passkey cannot start | A pool for each kind of ceremony and a cap for each network | `sec_14_one_network_and_one_kind_of_ceremony_cannot_fill_the_waiting_challenges` (`bb-auth`) |
| Denial of service | Flooding sign-in or the API | Rate limits, request size limits, cheap rejection before Argon2id work (a password over 256 characters is refused at once, and hashing runs on as many threads as the computer has processors) | rate-limit test (WP-06), `sec_14_one_network_is_limited_in_its_tries_whatever_the_account` (every try of the sign-in family, 60 a minute) |
| Repudiation | A sign-in, a failed try, a limit or a change of credentials leaves no trace | Every sign-in event is written to the security log, whose chain verifies | `sec_15_the_security_log_of_signing_in_verifies_and_names_what_happened`, `adr_0007_*` (an event for every change) |
| Elevation of privilege | Stolen session cookie used elsewhere | HttpOnly, Secure, idle and absolute timeouts, *Where you're signed in* to end sessions | `sec_12_a_session_ends_after_it_was_idle_too_long_and_use_keeps_it_alive`, `sec_12_a_session_ends_after_the_absolute_time_however_busy_the_person_is`, `sec_12_people_list_their_sessions_and_end_any_of_them` |
| Elevation of privilege | A person makes invitations or changes who may sign up | Administrators only | `auth_06_only_administrators_manage_invitations`, `auth_06_signing_up_is_off_by_default_and_only_administrators_change_the_rule` |

### Boundary 4 (operator ↔ data) and the host

| STRIDE | Threat | Mitigation | Proven by |
|--------|--------|-----------|-----------|
| Information disclosure | Stored secrets read from a database copy or backup | Envelope encryption with XChaCha20-Poly1305; the master key lives outside the database | secrets tests (WP-05) |
| Information disclosure | Other local users read keys or certificates | `keys/` and `tls/` created owner-only (Windows ACL, Unix mode 0700/0600) | data-folder tests (WP-04) |
| Information disclosure | Personal data ends up in logs | Logs hold IDs only; IP addresses shortened | DATA-LOGS |
| Information disclosure | Secrets committed to the repository | Secret scanning on every commit | SEC-SECRETS |
| Repudiation | An admin denies an action, or someone edits the security log | Append-only security log, hash-chained with an HMAC under a log key that is not in the database (`keys/log.key`, never rotated), so somebody who can only change the database cannot make a changed chain verify again; every append is also written to an append-only anchor file outside the database (`logs/security-head.log`), so cutting off or replacing the end is found; a verify command, and the status page shows the result. The command reads the whole log every time; the status page reads only the entries added since its last good check (and the whole log again once a day), and keeps its answer for a few seconds unless the server wrote an entry in between, so a long log does not make every visit slow (M1-F60) | hash-chain property tests `sec_15_any_changed_row_breaks_the_chain` and `sec_15_a_rewritten_tail_without_the_key_does_not_verify`; security-log tests (WP-05); `ops_08_the_status_page_checks_only_new_entries_and_the_command_checks_all` (`bb-store`), `ops_08_the_status_page_follows_the_log_and_reuses_its_answer_until_something_is_written` |
| Repudiation | Somebody who holds the host's key files (`keys/log.key`) and the anchor file rewrites the log | Not stopped: this protects against attackers who reach the database only (SQL injection, a stolen database copy or backup), not against somebody with control of the host. The status page shows the head so an administrator can compare it with an earlier note | Out of scope for self-hosted installations (see the table of limits below); the served model keeps keys per customer (M14) |
| Tampering | A vulnerable or unlicensed dependency ships | Register, license allow list, advisory scan | DEP-REGISTER; DEP-LICENSES; SEC-ADVISORIES |
| Tampering | Database changes outside the defined schema | Schema and migrations defined once in Rust, refused on a newer schema | DATA-MIGRATIONS; ARCH-DB-PARITY |
| Elevation of privilege | Memory-safety bugs | No `unsafe` code unless an ADR allows it | ARCH-NO-UNSAFE |

### Accepted risks

| Risk | Why accepted | Revisit |
|------|-------------|---------|
| An administrator of the host machine can read the master key | Anyone with full control of the host can read process memory too; protection against them is out of scope for self-hosted installations | M14 (served model: keys per customer, operator without data access) |
| Somebody who controls the host (and so reads `keys/log.key` and can rewrite `logs/security-head.log`) can forge the security log | The log protects against attackers who reach only the database; the same control lets them read process memory and the master key | M14 (served model: keys per customer, operator without data access) |
| Signing up on one's own shows whether an address has an account (the refusal is the same as for a closed door, but it comes only for some addresses) | Nothing can confirm an address before mail exists; signing up is off by default and the setting says so (M1-F38) | M4 (mail) |
| Somebody who keeps failing at an account can make it wait, up to 15 minutes at a time once the windows have grown (the limit counts failures and a success clears them, so it is never a lock for good) | Any limit that stops guessing can be used to slow the person down; a permanent lock would be worse (SEC-14) | When the settings registry takes the numbers (OPS-10) |
| A security-log entry that cannot be written (the anchor file or the database fails) is reported in the server log but does not undo or refuse the sign-in. The server counts the entries it could not write since it started, and the status page warns when the count is above zero (`sec_15_the_status_page_warns_when_a_log_entry_could_not_be_written`); the status page and `bugaboo security-log verify` also name a database that lacks the newest entries that the anchor file knows of (a restore from an older backup, lost writes or a cut-off end) and say what to check (`sec_15_the_status_page_warns_when_the_anchor_is_ahead`) | A sign-in that fails because the log is full would let anybody who can fill the disk lock everybody out | The decision stays (M1-F40, done in M2 WP-04) |
| Self-signed certificates show a browser warning | Safer than plain HTTP; admins can provide a certificate, ACME comes in 1.0 | V1 (ACME) |

## Personal data in M1

The map below is written by hand for M1. From M1 WP-05 on, `cargo xtask gen privacy` generates the complete list from the personal-data marks (ADR-0015) into `docs/privacy/personal-data.md`; this diagram then only shows the flows.

```mermaid
flowchart LR
  person(["Person"])
  admin(["Admin"])
  subgraph bb["Bugaboo"]
    acc["Account<br/>name, e-mail"]
    cred["Sign-in data<br/>password hash, passkeys,<br/>two-step secret (encrypted),<br/>backup codes (hashed)"]
    sess["Sessions<br/>hashed ID, device label,<br/>shortened IP, times"]
    inv["Invitations<br/>e-mail, hashed link"]
    seclog["Security log<br/>person ID, action,<br/>shortened IP, time"]
    logs["Server logs<br/>IDs only"]
  end
  admin -- "invites" --> inv
  inv -- "accepted" --> acc
  person -- "signs up / signs in" --> acc & cred
  person -- "browser" --> sess
  acc & cred & sess --> seclog
  bb --> logs
```

| Data | Purpose | Legal basis (GDPR Art. 6) | Kept |
|------|---------|---------------------------|------|
| Name, e-mail | Account, sign-in, showing who did what | (b) contract / employment | While the account exists; then *Remove a person* (M2) |
| Password hash, passkeys, two-step secret, backup codes | Sign-in | (b) contract; (f) security | Until changed or the account is removed |
| Sessions with shortened IP and device label | Staying signed in, *Where you're signed in* | (b) contract; (f) security | Until signed out or expired |
| Invitation e-mail | Sending and checking the invitation | (b) contract | Until accepted or 7 days |
| Security log entries | Proving who did what, and finding attacks (SEC-15) | (f) legitimate interests, with the security of processing of Art. 32 | Keep period set by the admin (default 1 year); hash chain stays valid when old entries are deleted in whole blocks. "Remove a person" does not rewrite the log (a rewritten log would no longer verify): the person's ID becomes a pseudonym for a tombstone, and the entry goes when its block passes the keep period |
| Shortened IP in logs | Abuse defence | (f) security | 7 days by default |

No personal data leaves the server in M1: no mail, no AI, no outside fonts, scripts or analytics (PRIV-01).
