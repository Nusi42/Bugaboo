# WP-09 — Access keys and two-step sign-in by role

**Wave 3 · Folder / branch:** `bugaboo.win4` / `WIN4`
**Features:** AUTH-07, AUTH-03 (required per role); SEC-12, SEC-14, SEC-15, SEC-16; architecture §7 (Sign-in and sessions); ADR-0025; follow-up M1-F45 (sign-in asks for it).

## Goal

People make access keys for scripts and apps that can only do less than they can, see each key once, and delete it; a key runs out and follows its owner's permissions at every use. A role that requires two-step sign-in makes its people set up a second step before they can go on.

## Owns

`crates/bb-auth/src/access_key*` (new), `crates/bb-app/src/auth/access_keys.rs` (new), the two-step-required part of `crates/bb-app/src/auth/session.rs`, `crates/bb-store/src/auth.rs` (access keys) and migration `m0006`, `crates/bb-api/src/api/access_keys*` (new) and the access-key part of the actor extractor, `web/app/src/features/my-settings/access-keys/**` and the *Set up your second step* screen in `web/app/src/features/sign-in/`, the gitleaks rule for Bugaboo access keys in `.gitleaks.toml` with its fixture, `userDocs/access-keys/*` (new).

## Tasks

1. **Keys (AUTH-07):** `bb_` + a public prefix + a 256-bit secret; only a hash is stored (its own domain, like sessions); shown once; a name, *What it may do* (*See only*, *See and change*, *Everything I can do*, or single permissions under *More options*) and optional projects (M2-Q6); an expiry that is required: 90 days by default, at most the organization's longest time, both organization settings in the registry (`auth.access-key-*`, default 90 and 365 days, the longest never above 365); last use kept to the minute. Owner's list in *My settings › Access keys* (lean); delete at once (a deleted key cannot come back, so the dialog says so).
2. **Using a key:** `Authorization: Bearer bb_…`; the actor is the owner with the key's scopes passed to `bb-policy`, intersected with the owner's permissions at every request (a key never widens, AUTH-07); an expired, deleted or unknown key gets the same answer; keys need no CSRF token (no cookie) and are refused on the routes of the owner's own sign-in data (password, passkeys, two-step, sessions, access keys), so a stolen key cannot take over the account; a limit per key besides the network limit (SEC-14).
3. **Security log (SEC-15):** making and deleting keys, and refused keys (shortened network only, never the secret or the prefix of an unknown key).
4. **Two-step required by role (M1-F45):** when the person's roles have the switch (WP-05) or `security.two-step-required` is on and the person has no second step, the session after the password stays *pending* and can only set up a code app or a passkey; then it becomes active. A screen in plain words explains why and how; an administrator who switches it on for a role sees how many of its members still need to set one up — only people the administrator may see.
5. **Secret scan:** a gitleaks rule that finds Bugaboo access keys, with its bad and good examples (test files carry the allow marker of the secret scan with a reason, coding-standards §4).
6. **Threat model rows** (`12-security.md`): stolen key, key after the owner lost a permission, key used on own sign-in data, guessing keys; each with its test.

## Acceptance tests (write first)

- `auth_07_a_key_is_shown_once_and_only_its_hash_is_stored`, `auth_07_a_key_can_do_only_what_its_owner_can_and_its_scopes_allow` (property test over random scopes and roles), `auth_07_a_key_follows_its_owner_losing_a_role_at_once`.
- `auth_07_an_expired_a_deleted_and_an_unknown_key_get_the_same_answer`, `auth_07_a_key_cannot_change_the_owners_sign_in_data`, `sec_14_a_key_has_its_own_limit`.
- `auth_03_a_role_that_requires_two_step_sign_in_makes_its_people_set_it_up_first` and the same for the organization switch.
- `sec_15_making_and_deleting_keys_is_in_the_security_log`.
- Playwright: make a key, copy it, use it from the test with `fetch`, delete it, see it refused; sign in as a person whose role needs two-step sign-in and set up a code app; four themes, phone, axe.

## Checks

SEC-SECRETS gets the access-key rule and its fixture (no new check). The routes declare their permissions (`own-account`) for ARCH-PERMS; WP-10, which merges last in this wave, adds their SEC-MATRIX examples.

## Answers

- **What a key may do (M2-Q6):** the choices keep the permissions of their kinds (*See*; *See* and *Change*; all), and *See only* caps every field at *Can see*; projects limit the key to them and their sub-projects (by ID) and take away everything for the whole organization ([ADR-0025](../../adr/0025-roles-groups-and-assignments.md), decision 8; [architecture §7](../../architecture/07-cross-cutting.md#authorization-model), *Evaluation*, step 6).
- **Two-step sign-in by role (M1-F45):** a person needs a second step when `security.two-step-required` is on or any role they hold anywhere has the switch; it counts from the next request, also for sessions that are already open (decision 7).
- The personal-data marks of keys are in [12-security.md](../../architecture/12-security.md#personal-data-in-m2).

## Depends on

WP-05 (scopes in the engine, the role switch), WP-03 (*My settings* frame), WP-02 (lean answers).
