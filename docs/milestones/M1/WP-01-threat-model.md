# WP-01 — Threat model and data-flow docs, plan (lead)

**Features:** COMP-07.

## Tasks

1. `docs/architecture/12-security.md`: threat model for M1's attack surface (sign-in, sessions, setup, HTTPS, secrets, security log), with STRIDE per trust boundary from architecture §2, the mitigations and the check or test that proves each one.
2. Data-flow map of personal data (people, sessions, security log, IP addresses), to be generated from the personal-data marks once WP-02/WP-05 exist; until then written by hand with a pointer to the generator.
3. Plan: this folder, waves, ports (`postgres` in `tools/dev/ports.json`).

## Acceptance

- META-RULE-TRACE and DOC-ADR-ENFORCED pass on the new chapter; the docs build includes it.
