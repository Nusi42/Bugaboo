# WP-06 — Sample organization tool and its people

**Wave 2 · Folder / branch:** `bugaboo.win3` / `WIN3`
**Features:** groundwork for the M2 exit demo and every later demo, using AUTH-06 (invitations) and AUTH-03 (code apps) to make the sample people; architecture §11 (Sample organization); ADR-0023.

## Goal

`cargo xtask dataset sample` makes the same fictional organization every time from a seed, on SQLite and PostgreSQL, through the real use cases. In this package it holds the 50 people; WP-13 adds groups, roles and projects. End-to-end tests and `cargo xtask dev --sample` start from a fresh copy of it.

## Owns

`crates/bb-sample/**` (new; module map entry from WP-01), `xtask/src/dataset.rs` (new) and its wiring in `xtask/src/main.rs`, the sample option of `cargo xtask dev`, the e2e server start in `web/app/e2e/support/server.ts`, `xtask/src/checks/test_sample_data.rs` and `tools/checks/fixtures/TEST-SAMPLE-DATA/`, the *Sample organization* section of `docs/architecture/11-testing.md` (how to make and use it).

## Tasks

1. **The generator (`bb-sample`):** deterministic from a seed (default 42) with its own random source; makes the first administrator through the setup use case and everybody else as that administrator through the same use cases the API calls (invitation, then accepting it), so every change has its event and its security-log entry. It never reads the clock: dates come from one start time passed in.
2. **People (architecture §11):** 50 people with the role mix of §11 recorded for WP-13 (2 Administrators, 4 Project leads, 30 Team members, 8 Contributors, 4 Viewers, 2 Guests); names from a list written for Bugaboo (no third-party name data), e-mail addresses only under `bugaboo.example`; the people the demos name are fixed (Ana and the administrator who runs the demo). The bot account, removed people and passkeys come later (AUTH-08, PRIV-04).
3. **Signing in as a sample person:** each run gives the sample people a random password written to an ignored file in the work folder (`target/sample/sign-in.txt`), never a fixed password in the repository (SEC-SECRETS); a few get a code app (their secret in the same file) for second-step tests.
4. **`cargo xtask dataset sample [--seed N] [--database sqlite|postgres] [--out <folder>] [--check]`:** xtask runs the `bb-sample` program as a process, so xtask itself depends on no product crate. Under a minute on SQLite.
5. **Using it:** the e2e server copies a freshly made sample database for each run (one per Playwright worker, ADR-0023 ports); `cargo xtask dev --sample` starts on it and prints where the passwords are. The M1 setup tests keep starting from an empty data folder.
6. **TEST-SAMPLE-DATA (people):** `--check` builds the organization twice from the same seed and on both databases and compares a canonical dump (IDs left out where they are random by design), checks that every e-mail address is under `bugaboo.example`, that every name comes from the bundled list, and that no secret is written outside the ignored file.

## Acceptance tests (write first)

- `the_sample_is_the_same_from_the_same_seed_on_both_databases` (canonical dump compared).
- `every_sample_address_is_under_bugaboo_example_and_every_name_is_from_the_list`.
- `every_sample_change_has_its_event_and_the_security_log_verifies`.
- A Playwright test signs in as a sample Team member with the password from the file.
- TEST-SAMPLE-DATA fails on its bad fixtures (an address outside `bugaboo.example`; two dumps that differ) and passes on the good one.

## Checks

TEST-SAMPLE-DATA switched on for people (catalogue entry exists; implementation, fixtures); WP-13 extends it to groups, roles and projects. ARCH-DEPS gets the rule for `bb-sample` from the module map. New crate registered.

## Depends on

WP-01 (module map entry). It uses only the sign-in use cases of M1, so it runs beside WP-05; when WP-05 merges first, the generator's first administrator comes from the new Administrator role without other changes.
