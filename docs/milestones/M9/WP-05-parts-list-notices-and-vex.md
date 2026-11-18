# WP-05 — The parts list, the notices and the VEX statements

**Wave 1 · Folder / branch:** `bugaboo.win5` / `WIN5` · **Model:** Sonnet (an exact brief: generators with fixed formats from fixed sources)
**Features:** COMP-01 (owner); the VEX part of COMP-03 (owned by WP-08). [ADR-0047](../../adr/0047-releases-signatures-parts-list-and-support.md) decision 3 and the VEX line of decision 4; COMP-02; follow-up M6-F126.

## Goal

Every full run makes Bugaboo's parts list (SBOM) in CycloneDX and SPDX from the lockfiles and fails when it is incomplete or invalid. The program carries its parts list and third-party notices, and answers them. Accepted advisories become VEX statements, written from the entries that accept them.

## Owns

`xtask/src/release/parts_list.rs` (new: the parts list from `cargo metadata` and `package-lock.json`), `xtask/src/release/notices.rs` (new: the third-party notices file from the license texts and `docs/third-party/notices/`), `xtask/src/release/vex.rs` (new: VEX from `deny.toml`, `tools/checks/npm-advisories.toml` and the register's *Accepted risks*), `xtask/src/checks/dep_sbom.rs` (new) and `tools/checks/fixtures/DEP-SBOM/`, the schemas of CycloneDX 1.6 and SPDX 2.3 in `tools/testdata/sbom/` (copied from their published versions, with their license), the entry DEP-SBOM in `tools/checks/catalog.toml`; the step in `xtask/src/build.rs` that writes both files into the build before the program is compiled; the embedded files and the route `GET /api/v1/about` in `crates/bb-api/src/api/about.rs` (version, licenses, parts list download; WP-08 adds the support period to its answer); coding-standards §5 and architecture §7 *Compliance artifacts* (the rule of DEP-SBOM, as "Adding a rule" asks); the register row of the schema validator.

## What is decided

- **Sources** (ADR-0047, decision 3, decided by the rules): the crates compiled into each program are what `cargo tree -e normal` names for the release targets (Windows x64, Linux x64 musl) and the WebAssembly modules, with licenses and edges from `cargo metadata`, not build or test crates; the npm packages are the ones `package-lock.json` marks as bundled into `web/app` (not `dev`); each with name, version, license expression, checksum from the lockfile, and the register's purpose. The notices of `docs/third-party/notices/` are listed as files with their license.
- **Fonts and icons** (COMP-01 names them): Bugaboo bundles no font files (the system font stacks, ADR-0026); the Lucide icons (ISC) are listed as the npm package they come from, or, where they are copied into `web/packages/ui/src/icons.ts`, as a component of their own with their license and source. The test of the parts list fails if a font file or an icon set appears in the build without an entry.
- **Formats:** CycloneDX 1.6 JSON (`bugaboo-<version>.cdx.json`) and SPDX 2.3 JSON (`bugaboo-<version>.spdx.json`), the program itself as the top component with its version and AGPL-3.0-or-later. Written with `serde_json`. Fixed order (sorted by name and version), so two runs of one commit write the same bytes.
- **The schema validator** is a dependency of xtask that never ships (the smallest maintained JSON Schema crate that supports the drafts both schemas use: `jsonschema`, without its network code); its register row is this package's. It is a normal dependency, not a development-only one, because DEP-SBOM validates inside the xtask program (amendment in ADR-0047).
- **DEP-SBOM** (the check, "Adding a rule"): runs in `ci` and `release`; fails when a shipped crate or package is missing, has no license or checksum, when an entry has no package, or when a file fails its schema. Status becomes active; its `tool` names xtask. The `release` part (every file signed) is WP-08's.
- **In the program:** `cargo xtask build` writes the parts list and the notices into the build first, and `rust-embed` takes them in; `GET /api/v1/about` answers the version, the license, the list of parts by name, version and license, and links to the two files and the notices. Signed-in people may read it (the version is not shown to strangers).
- **VEX** (M6-F126): one CycloneDX VEX file per release, one statement per accepted advisory (*not affected*, with the reason of its entry); written from the entries, never by hand. Architecture §7 drops the folder `security/vex/` and names the release folder.

## Tasks

1. Write the acceptance tests below.
2. The parts list in both formats; the schemas as test data with the validator; the notices file.
3. DEP-SBOM: catalogue entry, code, bad and good fixtures, the rule in coding-standards §5 and architecture §7.
4. The build step and the embedded files; `GET /api/v1/about`.
5. VEX.

## Acceptance tests (write first)

- `comp_01_the_parts_list_names_every_crate_the_program_compiles_in` (against `cargo metadata` of each release target).
- `comp_01_the_parts_list_names_every_npm_package_the_browser_app_bundles`.
- `comp_01_no_font_or_icon_set_ships_without_an_entry`.
- `comp_01_both_files_are_valid_against_their_schemas`.
- `comp_01_two_runs_of_one_commit_write_the_same_bytes`.
- `comp_02_the_notices_hold_every_license_text_the_parts_need`.
- `comp_01_about_answers_the_version_and_the_parts_to_signed_in_people_only`.
- `comp_03_every_accepted_advisory_has_a_vex_statement_with_its_reason`.
- DEP-SBOM's fixtures: a lockfile with a crate the list leaves out (bad); a list that matches (good).
- SEC-MATRIX: `GET /api/v1/about` for every starting role, signed out and with an access key.

## Checks

DEP-SBOM (new, active), DEP-REGISTER, DEP-LICENSES, SEC-ADVISORIES, ARCH-API-LINT, META-CATALOG, META-FIXTURES, META-WIRING, DOC-GENERATED.

## Review

Code review with the SSOT/DRY/ODR gate: the lockfiles as the one source, the reasons of accepted advisories written once.

## Depends on

WP-01 merged. Runs beside WP-02, WP-03 and WP-04 (xtask and one file of `bb-api`). WP-08 signs what it writes.
