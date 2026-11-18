# WP-08 — Signed releases, updates, the support period and secure defaults

**Wave 2 · Folder / branch:** `bugaboo.win5` / `WIN5` · **Model:** Opus
**Features:** SEC-18, COMP-04, DIST-07. [ADR-0047](../../adr/0047-releases-signatures-parts-list-and-support.md) decisions 1, 2, 5 to 8; ADR-0040.

## Goal

`cargo xtask release` builds one folder with everything a release needs, the maintainer signs it with a key kept outside every work folder, and anybody can check each file with `bugaboo update verify` or the common `minisign` tool. The program and the release page declare the support period. The settings that matter for security start safe, a test lists them, and a document maps the CRA's essential requirements to what meets them.

## Owns

`xtask/src/release/mod.rs` and `xtask/src/release/sign.rs` (new: the release folder, `SHA256SUMS`, the signatures; the build on Windows and the static Linux build in WSL, `cargo xtask release --linux` run inside WSL by the same command), the public key and the verification in `crates/bb-core/src/release.rs` (new: `RELEASE_PUBLIC_KEY`, the minisign format read and checked, pure), the command `bugaboo update verify <file>` in `crates/bb-server/src/cli/update.rs` (new), the support period in the build (`[workspace.metadata.release]` of `Cargo.toml`, read by the build and by `about`, one home) and in the answer of `GET /api/v1/about` (WP-05's route, one field more); the test of secure defaults `comp_04_…` in `crates/bb-core/src/settings/tests.rs`; `docs/security/cra-requirements.md` (new); the release notes template `docs/releases/TEMPLATE.md` and the notes of 0.6.0 (`docs/releases/0.6.0.md`, written by WP-15 at the exit); `userDocs/install/verify-a-download.md`, `userDocs/install/upgrade.md` (with WP-07); the register row of the signature crate.

## What is decided

- **The release folder** (ADR-0047, decision 1): `target/release-folder/<version>/` with `bugaboo-<version>-windows-x64.exe`, `bugaboo-<version>-linux-x64` (static), `SHA256SUMS`, a `.minisig` for each of these and for the parts lists and notices of WP-05, the VEX file, the release notes. The version is `[workspace.package]`'s (ADR-0040); a folder for a version that has no tag on `HEAD` is marked as a test build in its name and its notes.
- **Signatures** (decision 2, M9-Q9 as answered): Ed25519 in the minisign format (an untrusted comment, a trusted comment with the file name and version, the signature and the global signature over both). One new dependency for the signature (`ed25519-dalek`, or the smallest maintained crate that verifies minisign files; the choice and its register row are this package's), shared by xtask and `bb-core`. `cargo xtask release sign` reads the secret key only from a path the maintainer names on the command line or in `BUGABOO_RELEASE_KEY`, outside the repository and every work folder, and refuses a path inside one; it asks for the passphrase on the console. Tests make their own key pair and never touch the real key. The public key is written once in `bb-core` and copied into `SECURITY.md` and the release notes by the build (DOC-GENERATED compares).
- **`bugaboo update verify <file>`**: checks the file's signature with the built-in key and its entry in a `SHA256SUMS` next to it; answers *Signed by Bugaboo, version X* or names what failed; never runs or replaces anything. The update guide: verify, stop the service, replace the program, start (the backup and migrations of WP-07 follow).
- **Support period** (decision 5, M9-Q10 as answered): written once in `Cargo.toml`'s release metadata; `about` answers it, the release notes say it, `SECURITY.md` names the supported versions. Patch releases come from a branch `release-X.Y` cut at the tag; how is written in `docs/releases/TEMPLATE.md`.
- **Secure defaults** (decision 6): one test lists the settings that matter for security with their starting values (HTTPS on, sign-up off, every kind of AI off, e-mail off, outside images blocked, the data folder owner-only, two-step sign-in offered, `security.contact` empty) and fails when one starts unsafe. `docs/security/cra-requirements.md` maps each essential requirement of Annex I of the CRA to the features, checks and tests that meet it, and names what the legal review must confirm.
- **Platforms** (decision 7, M9-Q11 as answered): Windows x64 and Linux x64 musl.
- **Updates** (decision 8, M9-Q12 as answered): no notice inside Bugaboo in M9 (OPS-11, M14).
- **What is left for the maintainer:** making the release key once (`cargo xtask release keygen --out <path outside>` prints the public key to paste into `bb-core`), keeping it offline with its passphrase, signing each release, publishing it on GitHub. WP-08 replaces the placeholder public key only when the maintainer hands over the real one; until then the tests use their own pair and the exit's release candidate is signed with a test key and marked so.

## Tasks

1. Write the acceptance tests below.
2. The minisign reading and checking in `bb-core`; the dependency and its register row.
3. `cargo xtask release` (the folder, checksums), `release sign`, `release keygen`; the Linux build in WSL.
4. `bugaboo update verify`.
5. The support period in the build and in `about`; the release notes template.
6. The test of secure defaults; the CRA mapping; the guides.

## Acceptance tests (write first)

- `sec_18_a_signed_file_verifies_and_a_changed_byte_fails` (a key made for the test).
- `sec_18_a_signature_by_another_key_is_refused`.
- `sec_18_a_file_missing_from_sha256sums_is_refused`.
- `sec_18_signing_refuses_a_key_inside_the_repository_or_a_work_folder`.
- `sec_18_the_release_folder_holds_every_file_and_a_signature_for_each` (DEP-SBOM at `release`).
- `sec_18_our_signatures_read_in_the_minisign_format` (a vector signed by the minisign tool, kept as test data with its public key).
- `comp_04_every_setting_that_matters_for_security_starts_safe`.
- `comp_04_about_answers_the_support_period_from_its_one_home`.
- `dist_07_update_verify_never_changes_a_file`.

## Checks

DEP-SBOM (the `release` part), DEP-REGISTER, DEP-LICENSES, SEC-SECRETS, SEC-PRIVATE, ARCH-SETTINGS, LINT-PATHS, DOC-GENERATED, TXT-CATALOG.

## Review

Code review with the SSOT/DRY/ODR gate: the public key, the version and the support period each written once. **Security review:** the secret key never read from inside a folder of the repository, the passphrase never logged, the signature format checked against the vector, `update verify` never acts.

## Depends on

WP-01 and WP-05 merged. Runs beside WP-06, WP-07 and WP-09 (xtask's release module, one file of `bb-core`, one command).
