# WP-08 — Signed releases, updates, the support period, vulnerability handling and secure defaults

**Wave 2 · Folder / branch:** `bugaboo.win5` / `WIN5` · **Model:** Opus
**Features:** SEC-18, COMP-03, COMP-04, DIST-07. [ADR-0047](../../adr/0047-releases-signatures-parts-list-and-support.md) decisions 1, 2, 4 to 8; ADR-0040.

## Goal

`cargo xtask release` builds one folder with everything a release needs, the maintainer signs it with a key kept outside every work folder, and anybody can check each file with the installed `bugaboo update verify` or the common `minisign` tool. The program and the release page declare the support period. Bugaboo serves `security.txt`, and `SECURITY.md` and the runbook say how vulnerabilities are reported and handled within the CRA's deadlines. The settings that matter for security start safe, a test lists them, and a document maps the CRA's essential requirements to what meets them.

## Owns

`xtask/src/release/mod.rs` and `xtask/src/release/sign.rs` (new: the release folder, `SHA256SUMS`, the signatures, `release keygen`; the build on Windows and the static Linux build in WSL, `cargo xtask release --linux` run inside WSL by the same command); the public keys and the verification in `crates/bb-core/src/release.rs` (new: `RELEASE_PUBLIC_KEYS`, a list, read through `minisign-verify`); the command `bugaboo update verify <file>` in `crates/bb-server/src/cli/update.rs` (new); the support period in the build (`[workspace.metadata.release]` of `Cargo.toml`, read by the build and by `about`, one home) and in the answer of `GET /api/v1/about` (WP-05's route, one field more); `GET /.well-known/security.txt` in `crates/bb-api/src/web.rs` and the setting `security.contact` in `crates/bb-core/src/settings/security.rs`; `SECURITY.md`; `docs/security/vulnerability-runbook.md` (new); the test of secure defaults `comp_04_…` in `crates/bb-core/src/settings/tests.rs`; `docs/security/cra-requirements.md` (new); the release guide and notes template `docs/releases/TEMPLATE.md` (with the key rotation step) and the notes of 0.6.0 (`docs/releases/0.6.0.md`, written by WP-15 at the exit); `userDocs/install/verify-a-download.md`, `userDocs/install/upgrade.md` (with WP-07); the register rows of `minisign-verify` (shipped) and `minisign` (xtask only).

## What is decided

- **The release folder** (ADR-0047, decision 1): `target/release-folder/<version>/` with `bugaboo-<version>-windows-x64.exe`, `bugaboo-<version>-linux-x64` (static), `SHA256SUMS`, a `.minisig` for each of these and for the parts lists and notices of WP-05, the VEX file, the release notes. The version is `[workspace.package]`'s (ADR-0040); a folder for a version that has no tag on `HEAD` is marked as a test build in its name and its notes.
- **Signatures** (decision 2, M9-Q12): Ed25519 in the minisign format (an untrusted comment, a trusted comment with the file name and version, the signature and the global signature over both). The program verifies with the `minisign-verify` crate; xtask signs and makes keys with the `minisign` crate; no hand-written signature code. `cargo xtask release sign` reads the secret key only from a path the maintainer names on the command line or in `BUGABOO_RELEASE_KEY`, outside the repository and every work folder, and refuses a path inside one; it asks for the passphrase on the console. Tests make their own key pairs and never touch the real keys.
- **Two public keys** (plan B): `RELEASE_PUBLIC_KEYS` holds the release key and a backup key that the maintainer keeps apart. A file signed by either verifies. When a key is lost or leaked, the next release is signed with the other and drops the bad one from the list; the release guide writes the steps. The keys are written once in `bb-core` and copied into `SECURITY.md` and the release notes by the build (DOC-GENERATED compares).
- **`bugaboo update verify <file>`** is run by the **installed** program on the downloaded one, so the keys it trusts are the ones already on the computer. It checks the signature and the file's entry in a `SHA256SUMS` next to it; answers *Signed by Bugaboo, version X* or names what failed; warns when the version in the trusted comment is older than its own (a downgrade); never runs or replaces anything. The update guide: verify with the installed program, stop the service, replace the program, start (the backup and migrations of WP-07 follow).
- **Vulnerability handling** (decision 4, owner of COMP-03): `SECURITY.md` (supported versions as M9-Q13 answers, how to report through GitHub's private reporting, the answer within a week, disclosure within 90 days, the CRA reports at 24 h, 72 h and final through ENISA's single reporting platform); the runbook (who does what from report to advisory, the templates of the three reports; the maintainer's role, never a person's name or address); `security.txt` as M9-Q16 answers: `Contact:` the organization's (`security.contact`, empty at the start) first, then ours (one constant in `bb-core`); `Expires:` one year after the build's release date; `Policy:` the `SECURITY.md` address; `Preferred-Languages: en, de`; served with `text/plain`, the headers of SEC-HEADERS, no session needed. The VEX statements are WP-05's.
- **Support period** (decision 5, M9-Q13): written once in `Cargo.toml`'s release metadata; `about` answers it, the release notes say it, `SECURITY.md` names the supported versions. Patch releases come from a branch `release-X.Y` cut at the tag, as the release guide writes.
- **Secure defaults** (decision 6): one test lists the settings that matter for security with their starting values (HTTPS on, sign-up off, every kind of AI off, e-mail off, outside images blocked, the data folder owner-only, two-step sign-in offered, `security.contact` empty, no support access and no account but the first administrator's) and fails when one starts unsafe. The "nothing by default" line of DIST-03 (ADR-0048, decision 1) is in this test; the rest of support access comes in M14 (M9-Q20). `docs/security/cra-requirements.md` maps each essential requirement of Annex I of the CRA to the features, checks and tests that meet it, and names what the legal review must confirm.
- **Platforms** (decision 7, decided by the rules): Windows x64 and Linux x64 musl; ARM64 later.
- **Updates** (decision 8, M9-Q14): no notice inside Bugaboo in M9 (OPS-11, M14).
- **What is left for the maintainer:** making the release key and the backup key once (`cargo xtask release keygen --out <path outside>` prints each public key to paste into `bb-core`), keeping both offline and apart with their passphrases, signing each release, publishing it on GitHub. Until the real keys are handed over, the tests use their own pairs and the exit's release candidate is signed with a test key and marked so.

## Tasks

1. Write the acceptance tests below.
2. The dependencies and their register rows; the key list and the check in `bb-core`.
3. `cargo xtask release` (the folder, checksums), `release sign`, `release keygen`; the Linux build in WSL.
4. `bugaboo update verify` with the downgrade warning.
5. `security.txt` and its setting; `SECURITY.md`; the runbook.
6. The support period in the build and in `about`; the release guide with the rotation step.
7. The test of secure defaults; the CRA mapping; the guides.

## Acceptance tests (write first)

- `sec_18_a_signed_file_verifies_and_a_changed_byte_fails` (a key made for the test).
- `sec_18_a_file_signed_by_the_backup_key_verifies_and_one_by_another_key_is_refused`.
- `sec_18_a_file_missing_from_sha256sums_is_refused`.
- `sec_18_signing_refuses_a_key_inside_the_repository_or_a_work_folder`.
- `sec_18_the_release_folder_holds_every_file_and_a_signature_for_each` (DEP-SBOM at `release`).
- `sec_18_our_signatures_read_in_the_minisign_format` (a vector signed by the minisign tool, kept as test data with its public key).
- `dist_07_update_verify_warns_on_an_older_version`.
- `dist_07_update_verify_never_changes_a_file`.
- `comp_03_security_txt_names_the_organizations_contact_first_and_expires_a_year_after_the_release`.
- `comp_04_every_setting_that_matters_for_security_starts_safe`.
- `comp_04_about_answers_the_support_period_from_its_one_home`.
- SEC-MATRIX: `security.txt` signed out and signed in.

## Checks

DEP-SBOM (the `release` part), DEP-REGISTER, DEP-LICENSES, SEC-SECRETS, SEC-PRIVATE, SEC-HEADERS, ARCH-SETTINGS, LINT-PATHS, DOC-GENERATED, TXT-CATALOG.

## Review

Code review with the SSOT/DRY/ODR gate: the public keys, the version, the support period and the contact each written once. **Security review:** the secret keys never read from inside a folder of the repository, the passphrase never logged, the signature format checked against the vector, `update verify` never acts and trusts only the installed keys, `security.txt` and its headers, the runbook's deadlines.

## Depends on

WP-01 and WP-05 merged. Runs beside WP-06, WP-07 and WP-09 (xtask's release module, one file of `bb-core`, one command, `SECURITY.md`).
