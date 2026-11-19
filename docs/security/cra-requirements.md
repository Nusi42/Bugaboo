# The essential requirements of the CRA, and what meets them

Each essential requirement of Annex I of the Cyber Resilience Act
(Regulation (EU) 2024/2847), with the features, checks and tests of Bugaboo
that meet it (COMP-04, ADR-0047 decision 6). Features are named by their ID
in the [feature list](../01-feature-list.md), checks by their ID in the
[catalogue of checks](../architecture/10-checks.md). Read it before each
release; the points the legal review must confirm are at the end.

## Part I: properties of the product

| Requirement | What meets it |
|-------------|---------------|
| (1) A level of cybersecurity that fits the risks | The security architecture ([architecture §12](../architecture/12-security.md)); every rule a machine can check is a check of the catalogue, run at every commit and in every full run (ADR-0020, ADR-0037). |
| (2)(a) No known exploitable vulnerabilities when placed on the market | SEC-ADVISORIES on every locked part; an advisory is accepted only with a reason and a VEX statement in the release (DEP-SBOM, COMP-03). |
| (2)(b) Secure by default, with a way back to the original state | The test `comp_04_every_setting_that_matters_for_security_starts_safe` lists every setting that matters with its safe start and fails on an unsafe one; *Reset to default* on every setting (CFG-06). Not settings: HTTPS from the first start with a certificate of its own (SEC-10); the data folder for the server's account only (`sec_16_folders_are_for_the_owner_only_and_repaired_on_linux`); no account but the first administrator's (AUTH-09); no support access by default (ADR-0048, decision 1). |
| (2)(c) Vulnerabilities fixed by security updates, where possible automatic, with a notice and a way to postpone | Signed releases and `bugaboo update verify` (SEC-18, DIST-07); the backup and migration at the first start of a new program (OPS-06); the support period on *About* and in `SECURITY.md` (COMP-04). The notice inside Bugaboo is OPS-11 (M14): see the legal review. |
| (2)(d) Protection from unauthorized access: authentication, identity and access management | Passwords with Argon2id, passkeys and two-step sign-in (AUTH-01 to AUTH-03); sessions with `__Host-` cookies (SEC-12, SEC-COOKIES); the one policy engine (SEC-01, ARCH-AUTHZ-ONLY); the permission matrix for every route and role on both databases (SEC-MATRIX). Attempts are logged (SEC-15). |
| (2)(e) Confidentiality of stored and sent data, by encryption | HTTPS with HSTS (SEC-10, SEC-HEADERS); secrets encrypted at rest with the master key (SEC-16, SEC-SECRETS); field permissions everywhere a value could leak (SEC-04, SEC-06). |
| (2)(f) Integrity of data, commands, programs and settings | Versions and edit locks on every change (ADR-0019); the hash-chained security log (SEC-15); signed release files (SEC-18); search compiled to parameterized SQL (SEC-17). |
| (2)(g) Only the data that is needed (data minimization) | Personal-data marks on every stored field (DATA-MARKS); no personal data in logs (DATA-LOGS); *Keep and delete rules* (PRIV-05). |
| (2)(h) Availability of essential functions, resilience against denial of service | Rate limits and brute-force protection (SEC-14); time limits on every request; backups made while people work (OPS-07); the status page (OPS-08). |
| (2)(i) No harm to the availability of other services | Nothing leaves the server by default: e-mail, the assistant, AI apps, agents and outside images are off until an administrator switches them on (an upgrade keeps AI apps and agents as they were, migration 59) (the secure-defaults test above); the limits of agents (ADR-0029). |
| (2)(j) A small attack surface, also of external interfaces | One program and one port; no outside images, scripts or fonts (SEC-HEADERS, the content security policy); only the routes the API declares; the MCP server only with a grant (ADR-0042). |
| (2)(k) Less impact of an incident, by mitigations of exploitation | No `unsafe` code (ARCH-NO-UNSAFE); one sanitizer for every text people write, with Trusted Types (SEC-13, SEC-RICHTEXT); previews in a child process with limits (ADR-0032); dangerous patterns refused (SEC-PATTERNS). |
| (2)(l) Security information by recording and monitoring of relevant internal activity | The security log (SEC-15) with its keep period; the status page and structured logs (OPS-08). |
| (2)(m) Users can remove all data and settings for good, and move them | *Remove a person* (PRIV-04); *Download my data* (PRIV-03); backups and restore (OPS-07); the whole data folder is one folder the administrator can delete. |

## Part II: vulnerability handling

| Requirement | What meets it |
|-------------|---------------|
| (1) Identify and document vulnerabilities and components, with a parts list | The parts list in CycloneDX and SPDX for every release, made from the lockfiles (COMP-01, DEP-SBOM, DEP-REGISTER, DEP-LICENSES). |
| (2) Fix vulnerabilities without delay, with security updates apart from new features | The support period and the patch releases from `release-X.Y` ([release guide](../releases/README.md)); the [vulnerability runbook](vulnerability-runbook.md). |
| (3) Test and review security regularly | `cargo xtask ci` before `main` moves (ADR-0037); a security review of every package that touches security; a security pass before each release. |
| (4) Disclose fixed vulnerabilities, with an advisory | GitHub security advisories, written as the runbook says. |
| (5) A policy of coordinated disclosure | [`SECURITY.md`](../../SECURITY.md): private reporting, an answer within a week, disclosure within 90 days. |
| (6) A contact address for reports | `SECURITY.md` and `/.well-known/security.txt` on every server (COMP-03, `comp_03_security_txt_names_the_organizations_contact_first_and_expires_a_year_after_the_release`). |
| (7) Secure distribution of updates | Every file signed with our Ed25519 key in the minisign format and listed in `SHA256SUMS`; checked by the installed program (SEC-18, DIST-07, `sec_18_a_signed_file_verifies_and_a_changed_byte_fails`). |
| (8) Security updates without delay and free of charge, with an advisory | The support period (COMP-04); every release is free (ADR-0036). |

Reporting actively exploited vulnerabilities and severe incidents
(Article 14) is the [vulnerability runbook](vulnerability-runbook.md), with
its deadlines of 24 hours, 72 hours and the final report.

## What the legal review must confirm

- Whether a free public preview, offered without any commercial activity
  (ADR-0036, decision 5), is placed on the market in the sense of the CRA,
  and from which release the duties apply in full.
- Which conformity assessment Bugaboo needs (a product with no class of
  Annex III, or one of its classes), and the CE marking (COMP-05, M14).
- Whether a support period of "until the next release" is enough for a
  preview, and that 5 years per minor release from 1.0 meets Article 13(8).
- Whether `bugaboo update verify` and the release notes are enough for
  requirement (2)(c) until the notice inside Bugaboo (OPS-11) exists.
- That the deadlines and contents of the runbook's reports match the
  platform of ENISA as it stands at the release.
