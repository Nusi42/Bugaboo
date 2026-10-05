# WP-10 — Permission checks everywhere: SEC-MATRIX, ARCH-PERMS, ARCH-API-OUT, ARCH-NO-COUNTERS

**Wave 3 · Folder / branch:** `bugaboo.win5` / `WIN5`
**Features:** SEC-01, SEC-06, PRIV-13 (proved for every route); ADR-0004, ADR-0021, ADR-0022; follow-up M1-F32 (checked).

## Goal

Machines prove that every route asks the policy engine and declares its permission, that only authorized views leave the API, that no answer counts what the viewer cannot see, and that every starting role, scope and *No access* gives the expected answer on every route — on every build.

## Owns

`crates/bb-server/src/tests/matrix/**` (new), the table of example requests per route (`crates/bb-server/src/tests/matrix/examples.rs`; packages of later waves add their rows), `xtask/src/checks/{sec_matrix,arch_perms,arch_api_out,arch_no_counters}.rs`, their fixtures in `tools/checks/fixtures/`, `tools/checks/{counter-exemptions,api-out-exemptions}.toml` (new), the *Enforced by* lines these checks complete in architecture §7 and ADR-0004.

## Tasks

1. **ARCH-PERMS:** every operation in the API description has `x-permission` (WP-05 writes it), the value is a catalogue permission or `public`, `signed-in`, `own-account`; every catalogue permission is used by a route or named as used by the engine only (with a reason, for example field levels); every permission has its explaining message key.
2. **SEC-MATRIX (generated):** a data-driven test reads the API description and the catalogue and runs every route for every actor of a fixed list — nobody signed in, a person without a role, each starting role on the whole organization, a role on the parent project, a role on a sibling project only, *No access* on the project, an access key with *See only*, an access key for another project — and checks *allowed* or *refused*; a refused hidden thing answers `resource.not_found` like a missing one. Field rows: every built-in field × role × level through `Authorized<T>`. People rows: every kind of people information × level through `PersonRef`. Entry-point column: *API* now; later milestones add theirs (M3 issues, M4 e-mail, M5 search, M8 assistant and AI apps, M9 exports). The test writes a record of what it covered; the check fails when a route of the API description is missing from it or has no example request.
3. **ARCH-API-OUT:** the answer wrapper accepts only views that went through `Authorized<T>` (a trait only `bb-policy` implements) plus the short list in `api-out-exemptions.toml` (problem bodies, health, sign-in steps that are about the caller), each with a reason; an ast-grep rule finds handlers that answer with `axum::Json` or a plain struct.
4. **ARCH-NO-COUNTERS:** an ast-grep rule finds public identifiers or labels made from a counter (autoincrement columns, sequences, `count + 1`) outside counting issue IDs (M3, ADR-0021); a rule on the API description finds `total`, `count` and similar properties in answers, except those in `counter-exemptions.toml` with a reason that says why the number includes only what the viewer may see.
5. **Fix what they find** in the existing routes (sign-in, invitations, status, and everything waves 1–3 added); no suppression without a follow-up.

## Acceptance tests (write first)

- Each check fails on its bad fixture and passes on its good one: a route without `x-permission`; a permission nobody uses; a handler answering a plain struct; a list answer with `total`; a matrix record that misses a route.
- The matrix runs on both databases in `cargo xtask ci` and covers every route of the API description.
- `sec_06_every_field_level_of_every_role_shows_or_hides_the_field_in_answers`.
- `priv_12_every_kind_and_level_shows_the_person_as_the_rule_says`.

## Checks

**SEC-MATRIX**, **ARCH-PERMS**, **ARCH-API-OUT** and **ARCH-NO-COUNTERS** switched on (catalogue entries exist; implementation, fixtures, the rules are ADR-0004, ADR-0021, ADR-0022, PRIV-13 and SEC-01). From this merge on, every new route needs its declared permission and a matrix example.

## Depends on

WP-05 (`x-permission`, `Authorized<T>`, `PersonRef`, catalogue), WP-02 (API description conventions). It runs beside WP-07, WP-08 and WP-09 and **merges last** in wave 3: before it is merged, it takes in their merged routes and adds their example requests, so no package of the wave builds on WP-10.
