# ADR-0047: Releases: signatures, the parts list, the support period and vulnerability handling

Status: **Proposed** · 2026-11-18 · refines SEC-18, COMP-01, COMP-03, COMP-04, DIST-07 and [ADR-0040](0040-version-numbers.md); open questions M9-Q9 to M9-Q12 and M9-Q14, M9-Q15 of [the M9 plan](../milestones/M9/README.md)

## Context

M9 ends with the first release of Bugaboo that people can install and run for real. The Cyber Resilience Act asks a manufacturer for a software bill of materials, a way to report vulnerabilities, advisories, reports of actively exploited vulnerabilities within 24 and 72 hours (in force since 11 September 2026), free security updates for a declared support period, and secure defaults (compliance.md §1). The new Product Liability Directive covers software placed on the market from 9 December 2026. Bugaboo is free, AGPL-licensed, and nothing commercial is offered until a lawyer has reviewed it (ADR-0036, decision 5).

Today the repository has lockfiles, the register of direct dependencies, the license allow list and the advisory scan (DEP-REGISTER, DEP-LICENSES, SEC-ADVISORIES), `SECURITY.md` with private reporting on GitHub, and `cargo xtask build`, which builds the Windows program. DEP-SBOM is planned and not built. Nothing is signed. There is no certificate for Windows code signing on this computer, no hosted build (ADR-0037), and the static Linux program is built only on Linux.

## Decision

1. **What a release is.** A release is a tag `vX.Y.Z` on `main`, the version of ADR-0040 (M9 brings 0.6.0). `cargo xtask release` builds it on the development PC and in WSL and writes one folder: the programs (decision 7), `SHA256SUMS`, a signature for each file, the parts list (decision 3), the third-party notices, the release notes and known issues. The maintainer publishes that folder as a GitHub release and as the files a served instance is updated from. Enforced by: DEP-SBOM, review (the maintainer publishes).
2. **Signatures with our own key** (M9-Q9, recommendation). Every file of a release, the checksum list and the parts list are signed with one Ed25519 release key in the `minisign` format, so anybody can check them with that common tool or with `bugaboo update verify <file>`, which knows the public key. The public key is written once (`bb-core::release::RELEASE_PUBLIC_KEY`), in `SECURITY.md` and on the release page. The private key never enters the repository or a work folder: the maintainer keeps it offline with a passphrase and signs with `cargo xtask release sign`. Tests sign with a key made for the test. Windows code signing (Authenticode) waits for a certificate the maintainer decides on; until then Windows warns about an unknown publisher, and the install guide says how to check the signature first. Enforced by: SEC-SECRETS (no key in the repository), review (the signing step reads the key from outside the work folder).
3. **The parts list** (SBOM; M9-Q15, recommendation). `cargo xtask release` writes it in CycloneDX 1.6 JSON and SPDX 2.3 JSON from the lockfiles (single source, coding-standards §1): every crate that is compiled into each program (the dependency tree of the target, without build-only and test-only crates), every npm package bundled into the browser app, and the texts of `docs/third-party/notices/`, each with version, license and checksum. xtask writes both formats itself from `cargo metadata` and `package-lock.json`; no new tool is installed. The program carries the parts list and the notices of its own build, so *About* shows them. Enforced by: DEP-SBOM (made, complete against the lockfiles, valid against both schemas), DEP-REGISTER, DEP-LICENSES.
4. **Vulnerability handling.**
   - `SECURITY.md` says which versions get fixes (decision 5), how to report (GitHub's private reporting), the answer within one week, coordinated disclosure within 90 days, and that we report actively exploited vulnerabilities and severe incidents through ENISA's single reporting platform within 24 hours (early warning), 72 hours (notification) and the final report.
   - The runbook `docs/security/vulnerability-runbook.md` lists the steps from report to advisory, with the CRA deadlines; the full incident runbook for us as manufacturer and operator is DIST-08 (V1).
   - Every advisory that is accepted in `deny.toml` or `tools/checks/npm-advisories.toml` gets a VEX statement in the release folder (CycloneDX VEX: *not affected* and why), written from the same entries and the register's *Accepted risks*, so the reason is written once (M6-F126).
   - Bugaboo serves `/.well-known/security.txt` (RFC 9116, M9-Q14): our contact first, the organization's own contact when an administrator sets one, and an expiry one year after the release, so an old installation shows its age.

   Enforced by: SEC-ADVISORIES, DEP-SBOM, SEC-HEADERS, review (the runbook is read before each release).
5. **The support period** (M9-Q10, recommendation). Until 1.0, every release is a free public preview: only the newest release gets fixes, and a fix comes as the next patch release (0.6.1). From 1.0 on, each minor release gets security fixes for 5 years from its date, declared on the release page and in *About*. Security fixes ship as patch releases made from the release's tag on a branch `release-X.Y`, without new features. Enforced by: review (the release notes name the support period).
6. **Secure by default** (COMP-04). The settings that matter for security start safe and a test lists them: HTTPS on, sign-up off, every kind of AI off, e-mail off, outside images blocked, two-step sign-in offered, the data folder owner-only. `docs/security/cra-requirements.md` maps each essential requirement of the CRA (Annex I) to the features, checks and tests that meet it. Enforced by: ARCH-SETTINGS, SEC-HEADERS, review (the mapping is read before each release).
7. **Platforms of the first release** (M9-Q11, recommendation): Windows x64 and Linux x64 (static, for any distribution and WSL). Windows ARM64 waits for the ARM64 build tools (M1-F01), Linux ARM64 for a cross linker; both join a later release without a change to the code. Enforced by: review (the release folder lists its platforms).
8. **Updates** (DIST-07; M9-Q12, recommendation). A self-hosted installation is updated by replacing the program after `bugaboo update verify` has checked its signature; the next start backs up and migrates (ADR-0046). The update notice inside Bugaboo and its signed check of the feed are OPS-11 (V1, M14). Served instances are updated by us with the same signed files, in announced maintenance windows, from M14 (DIST-02). Enforced by: review (the install guide and the release notes describe the update).

## Consequences

- A release needs the maintainer twice: to sign with the offline key, and to publish.
- A person on Windows sees a warning about an unknown publisher until a code-signing certificate is bought (M9-Q9).
- The parts list is checked in every full run, not only at release time, so a release never finds a gap late.

## Alternatives considered

- **Sigstore (cosign keyless):** needs a hosted build identity and the network at signing time; ADR-0037 has no hosted build. Not now.
- **GPG signatures:** heavier for people to check, with key servers and trust models. Rejected in favour of minisign's single public key.
- **`cargo cyclonedx` and `@cyclonedx/cyclonedx-npm`:** two more pinned tools that each cover half and still need merging; SPDX needs a third. Kept as the fallback answer of M9-Q15.
- **5 years of support for 0.x:** a promise a single maintainer cannot keep for previews, and a preview is not placed on the market commercially. Rejected until 1.0.

## Enforced by

DEP-SBOM, DEP-REGISTER, DEP-LICENSES, SEC-ADVISORIES, SEC-SECRETS, SEC-HEADERS, ARCH-SETTINGS; review (signing, publishing and the runbook are steps of the maintainer).
