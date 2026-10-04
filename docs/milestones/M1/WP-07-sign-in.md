# WP-07 — Signing in

**Wave 3 · Folder / branch:** `bugaboo.win4` / `WIN4`
**Features:** AUTH-01, AUTH-02, AUTH-03, AUTH-06, AUTH-09, SEC-12, SEC-14 (per-account part); ADR-0002, ADR-0004.

## Owns

`crates/bb-auth/**`, the auth routes in `crates/bb-api/src/api/auth*`, their OpenAPI entries.

## Tasks

1. **Passwords (AUTH-01):** Argon2id (argon2 0.6) with parameters from OWASP's current guidance, stored as PHC strings, re-hash on sign-in when parameters change; minimum length 12, no composition rules; check against a bundled breached-password list (k-anonymity range file, no network call); constant-time comparison and equal timing for unknown accounts.
2. **Sessions (SEC-12):** random 256-bit session IDs stored hashed; `__Host-` cookie via the WP-06 cookie module; idle and absolute timeouts; new ID after sign-in and privilege change; people can list their sessions and end any of them ("sign out everywhere" included); CSRF with synchronizer token for unsafe methods plus `Origin` check.
3. **Two-step codes (AUTH-03):** TOTP (totp-rs 6) with QR in the setup flow, ten single-use backup codes stored hashed; admins can require it per role (the setting is declared now; it takes effect once roles exist in M2).
4. **Passkeys (AUTH-02):** webauthn-rs 0.5.5 registration and sign-in; relying party ID from start-up config; usable both as the second step and for signing in without a password.
5. **Invitations (AUTH-06):** admin creates an invitation, the one-time link is shown to the admin (e-mail comes in M4), expiry 7 days, single use, stored hashed. Signing up without an invitation is off by default; when an admin turns it on it can be limited to approved e-mail domains.
6. **Setup code (AUTH-09):** first start prints a one-time setup code to the console/log file; the first admin account needs it; disabled afterwards.
7. **Brute-force protection (SEC-14, per account):** back-off per account and per IP (with WP-06 limits), never a permanent lockout that enables denial of service; every sign-in event goes to the security log (WP-05).
8. **Tests:** both databases; timing test for unknown vs known accounts (statistical, tolerant); WebAuthn with a soft authenticator.

## Acceptance

- Integration tests for every flow incl. failure paths; no error reveals whether an account exists.
- All checks green; threat model (WP-01) mitigations for sign-in each point to a passing test.
