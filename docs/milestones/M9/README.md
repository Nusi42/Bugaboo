# M9 — Privacy, operations and the first release: work packages

Goal, features and exit demo: see [04-milestones.md](../../04-milestones.md) (M9). Rules: "Working in a folder", "Life of a work package", "Definition of done" and "The lead's routine" in [04-milestones.md](../../04-milestones.md), the [coding standards](../../coding-standards.md) (the SSOT/DRY/ODR review gate first), the [glossary](../../glossary.md).

**Goal:** everything needed to run the MVP for real, and its first release. Every person can download what Bugaboo stores about them and correct it; an administrator can switch a person off and, when the time comes, remove them, with the records the law asks for kept and explained. One page lists how long each kind of data is kept and who deletes it. An administrator makes and restores backups while people work, an upgrade backs up before it migrates and finds its way back, and Bugaboo runs as a service on Windows and Linux. The release is signed, comes with its parts list (SBOM), licenses, support period and a way to report vulnerabilities, and an administrator can let a supporter in for a few hours, seen in the security log. Every screen meets WCAG 2.2 AA, and an accessibility statement says so.

M9 starts from M8 (`m8-done`, version 0.5.1; API v1 frozen at `m4-done`: new routes and optional members are additions, nothing is renamed, removed or tightened). What exists:

- **The marks, not their use.** Every stored field carries a personal-data mark (ADR-0015, `bb-core::personal`, the generated [personal-data list](../../privacy/personal-data.md)), but nothing reads them to export or remove a person's data. The marks say whether a field is personal, not whose data a row is.
- **No way to remove or switch off a person.** A person has an ID, a display name and an e-mail address (`crates/bb-core/src/person.rs`). *Former user* exists only as a word of `PersonRef` (ADR-0022). Pieces wait: `delete_work_times_in` (M8-F30), `delete_private_searches_of` (M5-F62), the stop of agents (M7-F36). *Manage people* says "correct their names", but no route changes a name or an e-mail address.
- **Keeping and deleting in pieces.** Daily jobs delete dead sessions, drafts, notifications after 90 days, OAuth grants, away times after a year, files of the trash after 30 days, and prune the security log after `security.log-keep-days` (`crates/bb-app/src/keeping.rs`, `notifications/keep.rs`, ADR-0017). Work days are deleted only by an administrator after their keep period (ADR-0035, decision 7, the page *Working hours*). Issues in the trash, the events that carry old values and the anchor file of the security log are kept for ever.
- **No backups.** The data folder has an empty `backups/`; migrations (53 so far) run at start with no copy made (`crates/bb-store/src/migrations/`). The status page warns when the anchor of the security log is ahead of the database, and nothing can start a new anchor (M2-F09).
- **A service that never ran for real.** `bugaboo service install` and `uninstall` exist for Windows and systemd (`crates/bb-server/src/cli/service/`), tested as plans and texts only (M1-F57); the systemd unit is only partly hardened (M1-F63).
- **Release pieces, no release.** Lockfiles, the register, the license allow list and the advisory scan run in every full run (DEP-REGISTER, DEP-LICENSES, SEC-ADVISORIES); `cargo xtask build` builds the Windows program with the browser app inside; the static Linux program is built by `cargo xtask ci` in WSL. DEP-SBOM is planned, not built. Nothing is signed. `SECURITY.md` points to GitHub's private reporting; Bugaboo serves no `security.txt` and has no *About* page.
- **Accessibility, checked by machine.** TEST-A11Y runs axe on every address and the keyboard alternatives of the timeline; forced colours and touch targets have their tests. No statement and no EN 301 549 report exist.

Decided already: ADR-0015 (personal-data marks), ADR-0022 (the tombstone *Former user*), ADR-0035 decision 7 (work days kept by law, deleted only by an administrator, with the person's name after a removal), ADR-0030 amendment M8 (away times deleted after a year; work times go when a person leaves), ADR-0017 (jobs), ADR-0007 (events and the security log), ADR-0012 (sold and served), ADR-0036 (license; nothing commercial before the legal review), ADR-0037 (no hosted CI), ADR-0040 (version numbers: M9 done makes 0.6.0).

Proposed in this plan, for the maintainer's answers: [ADR-0044](../../adr/0044-download-my-data-and-remove-a-person.md) (*Download my data* and *Remove a person*), [ADR-0045](../../adr/0045-keep-and-delete-rules.md) (keep and delete rules), [ADR-0046](../../adr/0046-backups-restore-and-upgrades.md) (backups, restore and upgrades), [ADR-0047](../../adr/0047-releases-signatures-parts-list-and-support.md) (releases: signatures, the parts list, the support period, vulnerability handling), [ADR-0048](../../adr/0048-support-access-with-the-customers-approval.md) (support access with the customer's approval).

## Waves

M9 runs in **five waves**: wave 0 alone, then four waves of at most four builders in `bugaboo.win2` to `bugaboo.win5`.

- **Wave 0 (the lead, docs only):** WP-01 writes the maintainer's answers where they belong and turns the proposed decision records into accepted ones. Nothing builds before it is merged, because almost every answer changes a package.
- **Wave 1 (four side by side):** the foundations, in different crates and tables: backups (WP-02), the walk of a person's data with *Download my data* (WP-03), the keep and delete rules (WP-04), and the parts list with vulnerability handling, which is xtask and docs (WP-05).
- **Wave 2 (four side by side):** what stands on them: *Remove a person* on the walk and the keep rules (WP-06), upgrades on backups (WP-07), signed releases on the parts list (WP-08), and corrections of one's own data (WP-09), which needs nothing of wave 1 but fills the fourth folder.
- **Wave 3 (four side by side):** the screens of privacy (WP-10) and operations (WP-11), support access (WP-12), and the service proven on real systems (WP-13), which needs the signed program and the upgrade path.
- **Wave 4 (two side by side, then alone):** the accessibility pass over every screen (WP-14), then the exit (WP-15): the sample, the long run of ten people, speed, the release candidate and the exit demo. WP-15's measurements run alone on a quiet computer.

| Wave | WP | Name | Model | Folder / branch | Features (owner in **bold**) |
|------|----|------|-------|-----------------|------------------------------|
| 0 | [WP-01](WP-01-decision-records-words-and-answers.md) | Decision records, words, the UX gaps, the maintainer's answers | Opus (lead) | `bugaboo.win1` / `WIN1` | all of M9, as docs |
| 1 | [WP-02](WP-02-backup-and-restore.md) | Backup and restore on the server and the command line | Opus | `bugaboo.win2` / `WIN2` | **OPS-07** |
| 1 | [WP-03](WP-03-the-persons-data-and-download-my-data.md) | The walk of a person's data, and *Download my data* on the server | Opus | `bugaboo.win3` / `WIN3` | **PRIV-03** |
| 1 | [WP-04](WP-04-keep-and-delete-rules.md) | Keep and delete rules on the server | Opus | `bugaboo.win4` / `WIN4` | **PRIV-05** |
| 1 | [WP-05](WP-05-parts-list-and-vulnerability-handling.md) | The parts list, the notices and vulnerability handling | Sonnet | `bugaboo.win5` / `WIN5` | **COMP-01**, **COMP-03** |
| 2 | [WP-06](WP-06-switch-off-and-remove-a-person.md) | *Switch off* and *Remove a person* on the server | Opus | `bugaboo.win3` / `WIN3` | **PRIV-04** |
| 2 | [WP-07](WP-07-upgrades-with-a-backup-and-a-way-back.md) | Upgrades: a backup before migrating, the way back, the upgrade test | Opus | `bugaboo.win2` / `WIN2` | **OPS-06** |
| 2 | [WP-08](WP-08-signed-releases-and-the-support-period.md) | Signed releases, updates, the support period and secure defaults | Opus | `bugaboo.win5` / `WIN5` | **SEC-18**, **COMP-04**, **DIST-07** |
| 2 | [WP-09](WP-09-correct-your-own-data.md) | Correct your own data, and corrections by administrators | Opus | `bugaboo.win4` / `WIN4` | **PRIV-06** |
| 3 | [WP-10](WP-10-the-privacy-screens.md) | The privacy screens: *My data*, *Keep and delete rules*, *Data exports*, *Remove a person* | Opus | `bugaboo.win3` / `WIN3` | PRIV-03, PRIV-04, PRIV-05 on screen |
| 3 | [WP-11](WP-11-the-operations-screens.md) | The operations screens: *Backups*, *About*, failed jobs, notes for administrators | Sonnet | `bugaboo.win2` / `WIN2` | OPS-07, COMP-01, COMP-04 on screen |
| 3 | [WP-12](WP-12-support-access.md) | *Support access* with the customer's approval, server and screen | Opus | `bugaboo.win5` / `WIN5` | **DIST-03** |
| 3 | [WP-13](WP-13-run-as-a-service.md) | Run as a service, proven on a clean Windows and a Linux with systemd | Sonnet | `bugaboo.win4` / `WIN4` | **OPS-04** |
| 4 | [WP-14](WP-14-accessibility-pass-and-statement.md) | The accessibility pass, the EN 301 549 report and the statement | Opus | `bugaboo.win4` / `WIN4` | **COMP-15** |
| 4 | [WP-15](WP-15-sample-long-run-release-candidate-and-exit-demo.md) | Sample part 8, the long run, speed, the release candidate, the exit demo | Opus | `bugaboo.win2` / `WIN2` | all of M9, proved end to end |

Progress is kept only in [`docs/STATUS.md`](../../STATUS.md). The model column follows "The lead's routine", point 12, and the maintainer's trial result of M8: Opus for privacy, security, the plan and open UX work; Sonnet only where the package is an exact brief (WP-05: generators and docs with fixed formats; WP-11: screens whose words and layout WP-01 fixes; WP-13: commands that exist, proven step by step); Haiku for the chores around them (re-runs, follow-up rows, notes commits, folder cleaning). The lead counts the fix rounds of each package and model, as in M8.

## Every feature in exactly one package

PLAN-TRACE checks that every feature of the milestone is named; this table says which package **owns** each one (delivers it and proves it). Screens of a feature owned elsewhere name it too.

| Feature | Owner | Also on screen or proved in |
|---------|-------|-----------------------------|
| PRIV-03 Download my data | WP-03 | WP-10, WP-15 |
| PRIV-04 Remove a person | WP-06 | WP-10, WP-15 |
| PRIV-05 Keep and delete rules | WP-04 | WP-10 |
| PRIV-06 Correct one's own profile data | WP-09 | — |
| OPS-04 Run as a service | WP-13 | WP-15 |
| OPS-06 Migrations on upgrade with a backup | WP-07 | WP-13, WP-15 |
| OPS-07 Online backup and restore | WP-02 | WP-11, WP-15 |
| SEC-18 Supply chain (signed artifacts, checksums) | WP-08 | WP-15 |
| COMP-01 Parts list (SBOM) for every release | WP-05 | WP-08, WP-11 |
| COMP-03 Vulnerability handling | WP-05 | — |
| COMP-04 Secure by default and security updates | WP-08 | WP-11 |
| COMP-15 Accessibility | WP-14 | — |
| DIST-03 Operator access | WP-12 | — |
| DIST-07 Updates | WP-08 | WP-13 |

## The packages in short

- **WP-01 (lead, docs only):** the answers to M9-Q1…Q20, each written where it belongs. ADR-0044 to ADR-0048 from *Proposed* to *Accepted*, changed by the answers. Amendments: ADR-0015 (links of a person, beside the marks), ADR-0017 (the kinds of M9 and the one shape of deleting in steps), ADR-0022 (the tombstone, *Switch off*, support accounts), ADR-0035 decision 7 (time entries under ADR-0045), ADR-0012 and ADR-0037 (releases from this computer). The glossary words of M9. Architecture §6 (backups, restore, upgrades, releases, the service), §7 (privacy engineering, the keep list), §10 (the four new or grown checks), §11 (the upgrade test, the long run), §12 (the M9 part of the threat model, *Personal data in M9*, boundary 4 with support access). The UX gaps: *My settings › My data* and *Profile*, *Organization settings › Privacy* (*Keep and delete rules*, *Data exports*, the window *Remove a person*, *Switch off*), *System › Backups*, *About*, *Support access*, the page *Bugaboo is being restored*. The feature list's new wording of PRIV-05, OPS-07 and DIST-07. The package files stay as written here unless an answer changes them.
- **WP-02 (OPS-07; migration 54):** one backup file with a manifest (ADR-0046), made while people work on both databases, by `bugaboo backup`, by the route behind *Make a backup now* and by the schedule; `bugaboo restore` and the route for the restore on screen, with the server's page *Bugaboo is being restored*; every restore verifies the security log and starts a new anchor with the lost range named (M2-F09); `bugaboo keys import`, the twin of `keys export`.
- **WP-03 (PRIV-03; migration 55):** the links of each entity to a person, generated beside the marks; the one walk `PersonData` in `bb-store`; *Download my data* as a job that writes one ZIP of readable pages and JSON; the administrator's export with *Download someone's data*; DATA-MARKS grows so an entity with a person column and no link fails.
- **WP-04 (PRIV-05; migration 56):** `KEEP_KINDS`, the one list of what is kept how long and who deletes it; every existing purge moved under one shape that deletes in steps; the trash purged after its period with its files (M3-F25, M4-F131); records take their events along (M4-F93, M6-F45); dead jobs' payloads (M1-F21); the anchor file trimmed (M1-F26); time entries counted past their period and deleted only by an administrator, through the use case that deletes old work days, made general.
- **WP-05 (COMP-01, COMP-03):** `cargo xtask release parts-list` writes the CycloneDX and SPDX files from the lockfiles; DEP-SBOM goes from planned to active in every full run; the third-party notices file; the program carries both, and `GET /api/v1/about` answers them; VEX statements from the accepted advisories (M6-F126); `/.well-known/security.txt`; `SECURITY.md` and the vulnerability runbook.
- **WP-06 (PRIV-04; migration 57):** *Switch off* and *Switch on*; the removal plan (what goes, what stays as *Former user*, what is kept until when); *Remove a person* in one change plus the job `privacy.finish-removal`; every waiting follow-up (M2-F75, M3-F34, M3-F67, M4-F52, M4-F64, M5-F52, M5-F62, M5-F63, M7-F36, M8-F30, M8-F74); the removal list that survives a restore.
- **WP-07 (OPS-06):** a backup named `before-<version>` before the first migration of an upgrade; on a failed migration, the restore of that backup and a plain explanation; the upgrade test from the program of `m8-done`, which becomes the check TEST-UPGRADE.
- **WP-08 (SEC-18, COMP-04, DIST-07):** `cargo xtask release` builds the release folder; `cargo xtask release sign` signs it with the offline key; `bugaboo update verify`; the support period in the program and on the release page; the test of secure defaults and the CRA mapping; the release notes and the install and update guides.
- **WP-09 (PRIV-06; migration 58):** *My settings › Profile*: the name, and the e-mail address changed through a link to the new address with a note to the old one; an administrator with *Manage people* corrects name and e-mail of others with a reason, the person told and the security log written.
- **WP-10 (screens of PRIV-03, PRIV-04, PRIV-05):** *My settings › My data* (download, what is kept and how long, who may see what), *Organization settings › Privacy › Keep and delete rules*, *Data exports* (who made which export of whom), *Switch off* and the window *Remove a person* on a person's page, with the words of UX design §5.7.
- **WP-11 (screens of OPS-07, COMP-01, COMP-04):** *Organization settings › System › Backups* (list, schedule, *Make a backup now*, *Restore this backup*), *About* (version, support period, licenses, parts list), failed jobs with *Try again* (M4-F63), the note when sign-up e-mails were capped (M4-F128).
- **WP-12 (DIST-03; migration 59):** *Support access* as ADR-0048 says: made by an administrator, a one-time link, two steps, a role, an end at most 72 hours away, every request in the security log, *End now*.
- **WP-13 (OPS-04):** the service installed, started, upgraded and removed for real: in Windows Sandbox as the clean Windows computer, and in a fresh WSL distribution with systemd (M1-F57); the systemd unit hardened (M1-F63); `bugaboo service status`; the mail server's own certificate authority (M4-F61).
- **WP-14 (COMP-15):** every screen measured against WCAG 2.2 AA and EN 301 549 (axe, keyboard walks, reflow at 320 px, zoom to 200 %, text spacing, forced colours, touch targets); every finding fixed or written down; the EN 301 549 report and the accessibility statement template; the checklist for the screen-reader pass of the maintainer.
- **WP-15 (exit):** sample part 8 (a switched-off and a removed person, exports, backups, a support access); ten people for two hours on each database with a backup taken in the middle (the long TEST-CONCURRENT); the benchmarks on 1 M issues; the release candidate 0.6.0 built, signed and installed; an Opus security pass over the whole release; the exit demo step by step.

## Why the waves are in this order

Wave 0 first, because the answers decide what is kept, what is deleted and how a release is signed. Wave 1 lays the four foundations, which touch different places: backups `bb-server` and `bb-store`'s database code, the walk `bb-core::personal` and a new store module, the keep rules the jobs, and the parts list xtask. Wave 2 needs them: *Remove a person* reads the walk and the keep periods, upgrades back up with WP-02's code, signing signs WP-05's parts list. WP-09 fits in wave 2 because it shares no file with the others but the catalogues. Wave 3 draws the screens once their routes answer and proves the service once there is a signed program that upgrades itself; support access joins it because its removal at the end reuses WP-06. Wave 4 measures accessibility once every screen of M9 exists, and the exit proves everything together.

Inside wave 2, WP-06 starts as soon as WP-03 is merged; it does not wait for the rest of wave 1 except WP-04's list of keep periods, which it reads through a port whose shape WP-04 writes first. Inside wave 4, WP-15's measurements start only when WP-14 has pushed its last change, because both run browser tests (one full run at a time, "The lead's routine", point 6).

**Shared files, one entry each; rebase conflicts expected:** the packages of a wave each add their entries to `crates/bb-core/src/settings/catalog.rs`, `crates/bb-store/src/migrations/mod.rs`, `crates/bb-api/src/router.rs`, `crates/bb-policy/src/catalogue.rs`, `bb_app::jobs::JobKind`, `messages/en.json` and `crates/bb-server/src/cli/mod.rs`, and nothing else in those files. The one who lands later rebases and keeps both. **Migrations given out:** 54 (WP-02), 55 (WP-03), 56 (WP-04), 57 (WP-06), 58 (WP-09), 59 (WP-12); none depends on another (architecture §7, *Storage types*).

## Follow-ups handled in M9

| Follow-up | Package |
|-----------|---------|
| M2-F09 (a new anchor after a restore) | WP-02 |
| M3-F45 (the mark of the text of a field), M4-F52 (file bytes in the export), M6-F58 (marks of the notifications of work days), M7-F65 (the mark of text written with the assistant in exports) | WP-03 |
| M1-F21 (payloads of dead jobs), M1-F26 (the anchor file grows), M3-F25 (the trash's keep period), M4-F93 (file names in events), M4-F104 (purges in steps), M4-F131 (files of the trash), M6-F45 (time entries in events) | WP-04 |
| M6-F126 (VEX statements and their folder) | WP-05 |
| M2-F75, M3-F34, M3-F67, M4-F64, M5-F52, M5-F62, M5-F63, M7-F36, M8-F30, M8-F74 (what *Remove a person* deletes or scrubs) | WP-06 |
| M4-F122 (a keyed digest for the memory of sign-up e-mails) | WP-09 |
| M4-F63 (try a failed job again), M4-F128 (the note when sign-up e-mails were capped) | WP-11 |
| M1-F57 (a real service on Windows and systemd), M1-F63 (the systemd unit hardened), M4-F61 (the mail server's own certificate authority) | WP-13 |

**Not taken into M9:** M4-F121 (a sandbox for the child of a preview: still no safe crate; the accepted risk stays, and WP-15's security pass looks at it again), M4-F166 (binding many projects in one statement: needs thousands of projects), M0-F02 and M1-F02 (a Linux CI runner: WSL proves Linux locally, ADR-0037), and the speed follow-ups of the search (M5-F89 and its family), which keep their known gaps until the exit benchmarks say otherwise.

## Checks added or grown in M9

Each follows "Adding a rule" of [architecture §10](../../architecture/10-checks.md): the rule written where it belongs with `Enforced by: <ID>`, the entry in `tools/checks/catalog.toml`, the code in `xtask/src/checks/`, and its bad and good examples in `tools/checks/fixtures/<ID>/`. The META checks fail until all four are there.

- **DEP-SBOM** (WP-05; planned until now). Rule: COMP-01 and ADR-0047, decision 3, written into coding-standards §5 and architecture §7, *Compliance artifacts*. Catalogue: `when` grows from `release` to `ci` and `release`, `status` becomes active, `tool` names the generator of xtask instead of the outside tools. Code: `xtask/src/checks/dep_sbom.rs` makes the parts list into `target/` and fails when a crate that a program compiles in, or an npm package the browser app bundles, is missing or has no license or checksum, when an entry has no package, or when either file fails its schema (the schemas of CycloneDX 1.6 and SPDX 2.3 kept as test data); at `release` it also fails when the release folder lacks a file or its signature. Examples: a lockfile with a crate the parts list leaves out (bad), a parts list that matches (good).
- **TEST-UPGRADE** (WP-07; new). Rule: ADR-0046, decision 5, in architecture §11, *The upgrade test*. Catalogue: `when = ["ci", "release"]`, files `crates/bb-store/src/migrations/**`, `xtask/src/upgrade*`. Code: `xtask/src/checks/test_upgrade.rs` checks that the upgrade test exists, that it names the tag of the previous milestone (`m8-done` now, the newest `mN-done` tag later, read from Git, never written by hand), and that `cargo xtask ci` runs it; the test itself (`xtask/src/upgrade.rs`) builds the old program once into `target/upgrade/<tag>/`, makes the sample with it on both databases, starts the new program on that data, and checks that the backup `before-<version>` was made, that every migration ran, and that the sample's counts and a list of known records read the same. Examples: a ci without the step (bad), the wired step (good).
- **TEST-CONCURRENT, the long run** (WP-15; grown). Rule: architecture §11, *Ten people at the same time*, grows a paragraph *The long run*: two hours on each database, by hand at the exit on a quiet computer, with an online backup made in the middle and restored into a second database after the run, whose invariants must hold too. Catalogue: the tool text names `cargo xtask load --people 10 --long 2h` instead of `--nightly` (there is no nightly run), and `when` gains `release`. Code: `xtask/src/load/` takes `--long <length>` and the backup step; `xtask/src/checks/test_concurrent.rs` checks that the long scenario and its backup step exist and are not switched off. Examples: a load plan without the backup step (bad), the full plan (good).
- **DATA-MARKS, the links of a person** (WP-03; grown). Rule: ADR-0044, decision 1, in ADR-0015 (amendment) and architecture §7, *Privacy engineering*. Code: the derive of the marks and `xtask/src/checks/data_marks.rs` fail an entity that has a column of a person's ID and no link that says what the person is to the row. Examples: an entity with `author: UserId` and no link (bad), with its link (good).
- **SEC-MATRIX** grows rows for every new route (exports, removal, switching off, corrections, backups, restore, support access, keep rules) for every starting role, on both databases (each package for its routes).
- **TEST-A11Y** grows the measures of WP-14 (reflow, zoom, text spacing) for every address of `paths.ts`.

## Reviews

Every package gets a code review with the SSOT/DRY/ODR gate, with a fresh reviewer for each fix round. **Privacy review** (Opus, fresh): WP-03, WP-04, WP-06, WP-09, WP-10, and WP-02 for what a backup and a restore keep. **Security review** (Opus, fresh): WP-02 (restore on screen), WP-05 (`security.txt`), WP-07, WP-08 (signing, the key never in a folder), WP-09 (change of e-mail), WP-12 (support access), WP-13 (the service account and the unit). Screens (WP-09, WP-10, WP-11, WP-12, WP-14) get a UX review from pictures in the four themes on computer and phone, and a look at frame samples for anything that changes in place (UX §6.4, nothing moves under the cursor).

## The exit demo

As written in 04-milestones (M9), broken into steps. Steps 1 to 3 are tests of xtask and runs on the clean systems, recorded on the review page; steps 4 to 8 are tests in `web/app/e2e/m9-exit-demo.spec.ts` (with the commands of steps 4 and 8 in Rust tests of `bb-server`), titled by number and a plain sentence, with pictures in the four themes on computer and phone.

1. **"The release 0.6.0 is built, signed and checked"**: `cargo xtask release` writes the folder; `bugaboo update verify` accepts every file and refuses one with a changed byte (`sec_18_a_changed_byte_fails_the_signature`, a key made for the test).
2. **"Installed on a clean Windows computer as a service"**: in Windows Sandbox, the signed program is checked, `bugaboo setup --non-interactive` and `bugaboo service install` run, the service starts with the computer's service manager, and the browser setup opens with the setup code (WP-13's run script, its log kept in `.work/`).
3. **"Installed in a fresh WSL as a service"**: the same in a WSL distribution imported for the test, with systemd on, then removed again.
4. **"A backup made while people work is restored"**: Ada makes a backup from *Backups* while two people save issues; an issue is changed; *Restore this backup* brings the backup back, the change is gone, and the security log says what was restored.
5. **"Lea downloads her data, and an administrator downloads Ben's"**: Lea gets her file from *My data*; Ada makes Ben's in *Data exports*; Ben is told, and the security log names both.
6. **"Ben is switched off, then removed"**: Ben's session and his agent stop at *Switch off*; the window *Remove a person* says what goes, what stays and that his work days are kept until their date; after it Ben is *Former user K7Q2* on his issues and comments, and Hana (*HR*) still sees his work days under his name.
7. **"The About page shows the release"**: version 0.6.0, the support period, the licenses and the parts list to download; the release folder holds the same parts list, its signature and the notices.
8. **"An installation of M8 is upgraded without losing anything"**: TEST-UPGRADE on both databases, and once by hand on the service of step 2.

## Questions for the maintainer

Each question has the planner's recommendation and the options. Answering "take every recommendation" is fine; a different answer changes the decision records and the package files in WP-01 before any builder starts. **Questions marked "needs pictures"** get screenshots from the lead before they are asked (the bad case, and the choices side by side, light and dark, computer and phone where it matters).

**M9-Q1: Which data does Bugaboo delete by itself, and which only an administrator?** PRIV-05 says "automatic deletion"; your rule is that business data is deleted only explicitly (work days today). **Needs pictures:** the keep table of UX design §5.7, drawn with the changed column.

- **Recommendation:** two groups in one table (ADR-0045, decision 1). By itself: IP addresses (7 days), sessions and sign-in history (90 days), notifications (90 days), the trash (30 days), data exports (7 days), payloads of dead jobs (30 days), away times (1 year), the security log (1 year), backups (by their schedule). By an administrator after the period, with a count and *Delete them*: work days (2 years with *Germany*) and time entries (Q2).
- **Options:** (a) the recommendation; (b) everything by itself after its period, as PRIV-05 first said; (c) nothing by itself, every kind waits for an administrator.

**M9-Q2: How long are time entries kept, and who deletes them?** UX design §5.7 says "10 years, then deleted"; ADR-0035 left it to M9.

- **Recommendation:** at least 10 years at the start (an organization setting, no floor of law because Bugaboo cannot know which entries are accounting records), then an administrator deletes them like work days.
- **Options:** (a) the recommendation; (b) 10 years, then deleted by themselves; (c) another start, for example 8 years.

**M9-Q3: What happens to the texts of a removed person?** PRIV-04 says the organization chooses whether they stay. **Needs pictures:** the window *Remove a person* of UX design §5.7, and an issue page with *Former user K7Q2* on a comment kept and on a comment deleted.

- **Recommendation:** the window offers *Keep their comments, shown as Former user* (the start, an organization setting) or *Delete their comments*. Issue descriptions, fields, time entries and files stay as records of the organization, shown as *Former user*.
- **Options:** (a) the recommendation; (b) the choice also deletes the descriptions of issues they reported; (c) no choice, comments always stay.

**M9-Q4: May an administrator switch a person off before removing them?** No feature names it; people leave long before anybody erases their data, and their agents must stop (M7-F36).

- **Recommendation:** yes: *Switch off* and *Switch on* with *Manage people*, ending sign-in, sessions, keys, AI app grants and agents at once and keeping every record (ADR-0044, decision 5). A new glossary word.
- **Options:** (a) the recommendation; (b) no switch, only *Remove a person*; (c) a switch with an end date that removes the person by itself (against your rule on explicit deletion).

**M9-Q5: What is in *Download my data*?**

- **Recommendation:** everything the walk finds as the person's own, files they uploaded with their bytes, the security-log entries that name them; other people as the person may see them now; others' texts not copied; the full-text index left out (ADR-0044, decision 3).
- **Options:** (a) the recommendation; (b) leave file bytes out and list the files only (smaller files); (c) also include others' comments on the person's issues.

**M9-Q6: How does an administrator make someone else's export?**

- **Recommendation:** with a new permission *Download someone's data* (*Administrator* at the start, no key or agent); the file holds the same as the person's own, whatever the administrator may read otherwise; the person is told and the security log records it.
- **Options:** (a) the recommendation; (b) under *Manage people*, no new permission; (c) the export follows the administrator's own rights (it could then miss the person's work days).

**M9-Q7: What is in a backup?**

- **Recommendation:** one file with a manifest, the database and the files of attachments, no keys (ADR-0046, decision 1); the keys are exported apart with `bugaboo keys export`, and a restore checks they fit.
- **Options:** (a) the recommendation; (b) the keys inside the backup (one file restores all, but a stolen backup opens every secret); (c) a folder per backup instead of one file.

**M9-Q8: Restore from the screen, or only from the command line?** OPS-07 says both.

- **Recommendation:** both. On screen, after the password and second step again, everybody is signed out and sees *Bugaboo is being restored* until it opens again (ADR-0046, decision 4).
- **Options:** (a) the recommendation; (b) the command line only in M9, with the screen showing the command (OPS-07 reworded); (c) on screen without signing everybody out (unsafe: saves during the restore are lost).

**M9-Q9: How are releases signed?** There is no code-signing certificate. **Needs pictures:** the warning Windows shows for the unsigned program (taken in Windows Sandbox), next to the steps to check the signature.

- **Recommendation:** every file signed with our own Ed25519 key in the `minisign` format, checked by `bugaboo update verify` or the common tool (ADR-0047, decision 2). You make the key once and keep it offline with a passphrase; you sign each release. Windows code signing later, when you decide on a certificate.
- **Options:** (a) the recommendation; (b) also buy a code-signing certificate now (a yearly cost, and a company or person check by the seller); (c) checksums only, no signatures (SEC-18 not met).

**M9-Q10: What is the support period of the first release?** COMP-04 says "normally at least 5 years". The Product Liability Directive covers software placed on the market from 9 December 2026; nothing commercial is offered before the legal review (ADR-0036).

- **Recommendation:** 0.6.0 is a free public preview: until 1.0 only the newest release gets fixes, as `SECURITY.md` says today; from 1.0, 5 years of security fixes per minor release, as patch releases from a branch `release-X.Y` (ADR-0047, decision 5). The legal review (compliance.md, open point 6) confirms it before 1.0.
- **Options:** (a) the recommendation; (b) 5 years from 0.6.0 already; (c) 1 year per release until 1.0.

**M9-Q11: Which systems get the first release?**

- **Recommendation:** Windows x64 and Linux x64 (static, for any distribution and WSL); ARM64 for both in a later release, when the build tools are there (M1-F01).
- **Options:** (a) the recommendation; (b) all four now (needs the Windows ARM64 tools and a Linux cross linker on this computer).

**M9-Q12: What does DIST-07 (updates) mean before OPS-11?** DIST-07 (MVP) points to the update notice of OPS-11, which is V1 (M14).

- **Recommendation:** M9 gives signed releases, `bugaboo update verify`, the automatic backup and migration at start, release notes and the update guide. The notice inside Bugaboo stays OPS-11 (M14); updates of served instances come with M14. DIST-07 is reworded so in the feature list.
- **Options:** (a) the recommendation; (b) move OPS-11 into M9 (a signed feed and a notice on the status page: about one package more).

**M9-Q13: How does support access work before the served model?**

- **Recommendation:** an administrator makes a time-limited account for one named supporter (at most 72 hours, 8 at the start), with a role no stronger than their own (*Viewer* at the start), signed in with two steps through a one-time link; every request in the security log; *End now* at any moment (ADR-0048). The screens call it *Support access*.
- **Options:** (a) the recommendation; (b) read only, always (simpler, but a supporter often needs to change a setting); (c) wait for M14 (DIST-03 is an MVP feature).

**M9-Q14: Whose contact is in `security.txt`?**

- **Recommendation:** ours (GitHub's private reporting) for problems in Bugaboo, and the organization's own contact first when an administrator sets one; it expires one year after the release.
- **Options:** (a) the recommendation; (b) ours only; (c) no `security.txt` in the program, only in the repository.

**M9-Q15: Who writes the parts list?**

- **Recommendation:** xtask, from `cargo metadata` and `package-lock.json`, in both formats, checked against both schemas; no new tool (ADR-0047, decision 3).
- **Options:** (a) the recommendation; (b) the outside tools `cargo-cyclonedx` and `@cyclonedx/cyclonedx-npm`, merged by xtask, plus a converter for SPDX.

**M9-Q16: How long is the long run of ten people?** The run of 30 minutes was a nightly one, and there is no nightly run.

- **Recommendation:** two hours on each database, by hand at the exit on a quiet computer, with an online backup made in the middle and restored afterwards.
- **Options:** (a) the recommendation; (b) 30 minutes each, as before; (c) eight hours each, overnight while nothing else runs.

**M9-Q17: May the builders use Windows Sandbox and a throwaway WSL distribution as the clean computers?** The exit demo asks for a clean Windows PC and WSL; the WSL of this computer runs without systemd.

- **Recommendation:** yes. You switch on the Windows feature *Windows Sandbox* once (it needs administrator rights and a restart); WP-13 starts it with a configuration file in its `.work/` that runs the install script as the sandbox's administrator. A fresh Ubuntu distribution is imported into `.work/` (`wsl --import`), systemd is switched on inside it only, and it is unregistered after the run. You repeat the demo on a real clean PC if you wish.
- **Options:** (a) the recommendation; (b) you run the service installs yourself, from WP-13's checklist; (c) systemd on in the existing WSL distribution.

**M9-Q18: Who does the screen-reader pass?** Agents cannot listen to a screen reader.

- **Recommendation:** WP-14 does everything a machine can measure and fixes it; you spend about an hour with NVDA (free) or Narrator on the checklist of WP-14 (sign in, the issue list, an issue, the timeline's typed ways, a settings page); the statement says "partly conforms" for what was not checked by ear. An outside test before 1.0 (M15).
- **Options:** (a) the recommendation; (b) an outside tester now; (c) machine checks only, and the statement says so.

**M9-Q19: Which security review before the release candidate?** The roadmap recommends an outside one.

- **Recommendation:** an Opus security pass over the whole release in WP-15, one fresh reviewer per boundary of architecture §12, with the findings fixed before 0.6.0; a paid outside test before 1.0 (M15 plans one).
- **Options:** (a) the recommendation; (b) a paid outside test before 0.6.0 (cost, and it waits for a tester).

**M9-Q20: What stays out of M9?**

- **Recommendation:** the list below.

## What stays out of M9 (M9-Q20)

Each item keeps its place in the feature list and the plan:

- **Encrypted backups** (SEC-20, V1) and backups to outside storage (S3).
- **The update notice inside Bugaboo** with its signed feed (OPS-11, M14), and updates of served instances (DIST-02, M14).
- **The record of processing** and the privacy notice template (PRIV-07, M14); the data-breach help (PRIV-11, M14); the incident runbook as manufacturer and operator in full (DIST-08, M14).
- **The technical documentation file** and the EU declaration of conformity (COMP-05, M14), and the CE marking, which the legal review decides.
- **Packages** (winget, MSI, deb, rpm, container image; OPS-05, M14).
- **Windows code signing** (Authenticode), until the maintainer decides on a certificate (M9-Q9).
- **ARM64 programs** (M9-Q11).
- **A sandbox for the child of a preview** (M4-F121), until a safe way exists.

## Risks

- **Removal touches almost every table.** WP-06 reads the walk instead of naming tables, so a table it misses is a missing link, which DATA-MARKS refuses; the privacy review reads the removal plan against the generated personal-data list.
- **A restore can lose saves.** The restore on screen signs everybody out first and makes a backup of the present state before it starts (ADR-0046, decision 4); the concurrent test of WP-02 saves during a restore and expects a clear refusal, never a half-saved change.
- **The security log's chain after a restore.** A restore that forgot the anchor would leave the status page red for ever (M2-F09). WP-02's tests restore an older backup and expect a verified log with a named gap.
- **No clean computer, no certificate, no systemd here.** WP-13 proves the service in Windows Sandbox and a throwaway WSL distribution (M9-Q17); what is left for the maintainer: switching Windows Sandbox on once, making and keeping the release key, signing and publishing, and an optional run on a real clean PC. Windows will warn about an unknown publisher until a certificate is bought (M9-Q9).
- **The law moves under the release.** The CRA's reporting duty applies now, the Product Liability Directive from 9 December 2026. A free preview that is not offered commercially keeps both light (ADR-0036, decision 5; ADR-0047, decision 5); the legal review is still open (compliance.md, open points 1 and 6).
- **The full run is not reliably green.** M8 ended with browser tests that pass alone and fail in a full run (M8-F10, F220). Each landing of M9 needs a green run; the lead keeps the rule of one full run at a time and records each flaky test with its follow-up instead of running again blindly.
- **The milestone is large.** Fourteen features, fifteen packages, five decision records. If wave 2 runs late, WP-12 (support access) and WP-13 (the service) can move into wave 4 beside WP-14 without changing anything else.
- **Disk space.** Backups, the old program of the upgrade test and the long run's datasets are large. Each lives under the folder's `target/` or `.work/`, and `cargo xtask folder clean` frees it after the landing.
