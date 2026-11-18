# M9 — Privacy, operations and the first release: work packages

Goal, features and exit demo: see [04-milestones.md](../../04-milestones.md) (M9). Rules: "Working in a folder", "Life of a work package", "Definition of done" and "The lead's routine" in [04-milestones.md](../../04-milestones.md), the [coding standards](../../coding-standards.md) (the SSOT/DRY/ODR review gate first), the [glossary](../../glossary.md).

**Goal:** everything needed to run the MVP for real, and its first release. Every person can download what Bugaboo stores about them and correct it; an administrator can switch a person off and, when the time comes, remove them, with the records the law asks for kept and explained. One page lists how long each kind of data is kept and who deletes it. An administrator makes backups while people work and restores them from the command line, an upgrade backs up before it migrates and finds its way back, and Bugaboo runs as a service on Windows and Linux. The release is signed, comes with its parts list (SBOM), licenses, support period and a way to report vulnerabilities, and nobody outside the organization gets in by default. Every screen meets WCAG 2.2 AA, and an accessibility statement says so.

M9 starts from M8 (`m8-done`, version 0.5.1; API v1 frozen at `m4-done`: new routes and optional members are additions, nothing is renamed, removed or tightened). What exists:

- **The marks, not their use.** Every stored field carries a personal-data mark (ADR-0015, `bb-core::personal`, the generated [personal-data list](../../privacy/personal-data.md)), but nothing reads them to export or remove a person's data. The marks say whether a field is personal, not whose data a row is.
- **Half a tombstone, no way to remove or switch off a person.** A person has an ID, a display name and an e-mail address (`crates/bb-core/src/person.rs`). `crates/bb-policy/src/people.rs` already has `PersonRef::Former`, `Target.removed` with `FormerUser.label`, and `former_user_label`, which draws the random code (`priv_13_former_user_labels_are_random_and_unique`); `resolve` already keeps a removed person's name for the work days (ADR-0035, decision 7). Nothing fills `Target.removed` yet. Pieces wait: `delete_work_times_in` (M8-F30), `delete_private_searches_of` (M5-F62), the stop of agents (M7-F36). *Manage people* says "correct their names", but no route changes a name or an e-mail address.
- **Keeping and deleting in pieces.** Daily jobs delete dead sessions, drafts, notifications after 90 days, OAuth grants, away times after a year, files of the trash after 30 days, and prune the security log after `security.log-keep-days` (`crates/bb-app/src/keeping.rs`, `notifications/keep.rs`, ADR-0017). Work days are deleted only by an administrator after their keep period (ADR-0035, decision 7, the page *Working hours*). Issues in the trash, the events that carry old values and the anchor file of the security log are kept for ever.
- **No backups.** The data folder has an empty `backups/`; migrations (53 so far) run at start with no copy made (`crates/bb-store/src/migrations/`). The status page warns when the anchor of the security log is ahead of the database, and nothing can start a new anchor (M2-F09).
- **A service that never ran for real.** `bugaboo service install` and `uninstall` exist for Windows and systemd (`crates/bb-server/src/cli/service/`), tested as plans and texts only (M1-F57); the systemd unit is only partly hardened (M1-F63).
- **Release pieces, no release.** Lockfiles, the register, the license allow list and the advisory scan run in every full run (DEP-REGISTER, DEP-LICENSES, SEC-ADVISORIES); `cargo xtask build` builds the Windows program with the browser app inside; the static Linux program is built by `cargo xtask ci` in WSL. DEP-SBOM is planned, not built. Nothing is signed. `SECURITY.md` points to GitHub's private reporting; Bugaboo serves no `security.txt` and has no *About* page.
- **Accessibility, checked by machine.** TEST-A11Y runs axe on every address and the keyboard alternatives of the timeline; forced colours and touch targets have their tests. No statement and no EN 301 549 report exist.

Decided already: ADR-0015 (personal-data marks), ADR-0022 (the tombstone *Former user*), ADR-0035 decision 7 (work days kept by law, deleted only by an administrator, with the person's name after a removal), ADR-0030 amendment M8 (away times deleted after a year; work times go when a person leaves), ADR-0017 (jobs), ADR-0007 (events and the security log), ADR-0012 (sold and served), ADR-0036 (license; nothing commercial before the legal review), ADR-0037 (no hosted CI), ADR-0040 (version numbers: M9 done makes 0.6.0, 157 of 225 features).

**Accepted on 2026-11-18** (the maintainer took every recommendation): [ADR-0044](../../adr/0044-download-my-data-and-remove-a-person.md) (*Download my data* and *Remove a person*, the legal-hold name, the replay list), [ADR-0045](../../adr/0045-keep-and-delete-rules.md) (keep and delete rules), [ADR-0046](../../adr/0046-backups-restore-and-upgrades.md) (backups, restore from the command line, upgrades), [ADR-0047](../../adr/0047-releases-signatures-parts-list-and-support.md) (releases: signatures, the parts list, the support period, vulnerability handling), [ADR-0048](../../adr/0048-support-access-with-the-customers-approval.md) (support access: decision 1, nothing by default, holds now; the rest is planned for M14).

**Decided by the rules, not asked** (the lead, 2026-11-18): the first release is for Windows x64 and Linux x64 only, ARM64 later (the build tools are missing, M1-F01); xtask writes the parts list in CycloneDX and SPDX from the lockfiles, with no new tool (SSOT, coding-standards §1 and §5); the long run of ten people lasts two hours on each database, by hand at the exit on a quiet computer (no nightly run; benchmarks by hand at the exit).

## Waves

M9 runs in **five waves**: wave 0 alone, then four waves of at most four builders in `bugaboo.win2` to `bugaboo.win5`. The three cuts of M9-Q20 are taken: restore from the command line only, support access (DIST-03) moved to M14, and three small operations follow-ups moved to later.

- **Wave 0 (the lead, docs only):** WP-01 wrote the answers where they belong and accepted the decision records. **Done (2026-11-18).**
- **Wave 1 (four side by side):** the foundations, in different crates and tables: backups (WP-02), the walk of a person's data with *Download my data* (WP-03), the keep and delete rules (WP-04), and the parts list with the notices and VEX, which is xtask (WP-05). **WP-03's person links land first**, as their own small landing in the first days of the wave (the derive, the generated list, the grown DATA-MARKS and the links of every existing entity); WP-02 and WP-04 rebase onto it and add the links of their own new columns.
- **Wave 2 (four side by side):** what stands on them: *Remove a person* on the walk and the keep rules (WP-06), upgrades on backups (WP-07), signed releases with vulnerability handling on the parts list (WP-08), and corrections of one's own data (WP-09), which needs nothing of wave 1 but the links.
- **Wave 3 (three side by side):** the screens of privacy (WP-10) and operations (WP-11), and the service proven on real systems (WP-13), which needs the signed program and the upgrade path.
- **Wave 4 (two side by side, then alone):** the accessibility pass over every screen (WP-14), then the exit (WP-15): the sample, the long run of ten people, speed, the release candidate and the exit demo. WP-15's measurements run alone on a quiet computer.

| Wave | WP | Name | Model | Folder / branch | Features (owner in **bold**) |
|------|----|------|-------|-----------------|------------------------------|
| 0 | [WP-01](WP-01-decision-records-words-and-answers.md) | Decision records, words, the UX gaps, the maintainer's answers (done) | Opus (lead) | `bugaboo.win1` / `WIN1` | all of M9, as docs |
| 1 | [WP-02](WP-02-backup-and-restore.md) | Backup, and restore from the command line | Opus | `bugaboo.win2` / `WIN2` | **OPS-07** |
| 1 | [WP-03](WP-03-the-persons-data-and-download-my-data.md) | The walk of a person's data, and *Download my data* on the server | Opus | `bugaboo.win3` / `WIN3` | **PRIV-03** |
| 1 | [WP-04](WP-04-keep-and-delete-rules.md) | Keep and delete rules on the server | Opus | `bugaboo.win4` / `WIN4` | **PRIV-05** |
| 1 | [WP-05](WP-05-parts-list-notices-and-vex.md) | The parts list, the notices and the VEX statements | Sonnet | `bugaboo.win5` / `WIN5` | **COMP-01** |
| 2 | [WP-06](WP-06-switch-off-and-remove-a-person.md) | *Switch off* and *Remove a person* on the server | Opus | `bugaboo.win3` / `WIN3` | **PRIV-04** |
| 2 | [WP-07](WP-07-upgrades-with-a-backup-and-a-way-back.md) | Upgrades: a backup before migrating, the way back, the upgrade test | Opus | `bugaboo.win2` / `WIN2` | **OPS-06** |
| 2 | [WP-08](WP-08-signed-releases-and-the-support-period.md) | Signed releases, updates, the support period, vulnerability handling, secure defaults | Opus | `bugaboo.win5` / `WIN5` | **SEC-18**, **COMP-03**, **COMP-04**, **DIST-07** |
| 2 | [WP-09](WP-09-correct-your-own-data.md) | Correct your own data, and corrections by administrators | Opus | `bugaboo.win4` / `WIN4` | **PRIV-06** |
| 3 | [WP-10](WP-10-the-privacy-screens.md) | The privacy screens: *My data*, *Keep and delete rules*, *Data exports*, *Remove a person* | Opus | `bugaboo.win3` / `WIN3` | PRIV-03, PRIV-04, PRIV-05 on screen |
| 3 | [WP-11](WP-11-the-operations-screens.md) | The operations screens: *Backups* and *About* | Sonnet | `bugaboo.win2` / `WIN2` | OPS-07, COMP-01, COMP-04 on screen |
| 3 | [WP-13](WP-13-run-as-a-service.md) | Run as a service, proven on a clean Windows and a Linux with systemd | Opus | `bugaboo.win4` / `WIN4` | **OPS-04** |
| 4 | [WP-14](WP-14-accessibility-pass-and-statement.md) | The accessibility pass, the EN 301 549 report and the statement | Opus | `bugaboo.win4` / `WIN4` | **COMP-15** |
| 4 | [WP-15](WP-15-sample-long-run-release-candidate-and-exit-demo.md) | Sample part 8, the long run, speed, the release candidate, the exit demo | Opus | `bugaboo.win2` / `WIN2` | all of M9, proved end to end |
| — | [WP-12](WP-12-support-access.md) | *Support access* — **moved to M14** with DIST-03 (M9-Q20); kept here as the draft M14 starts from | — | — | — |

Progress is kept only in [`docs/STATUS.md`](../../STATUS.md). The model column follows "The lead's routine", point 12, and the maintainer's trial result of M8: Opus for privacy, security, the plan and open UX work; Sonnet only where the package is an exact brief (WP-05: generators with fixed formats; WP-11: screens on routes that exist, whose words and frames are fixed); Haiku for the chores around them (re-runs, follow-up rows, notes commits, folder cleaning). The lead counts the fix rounds of each package and model, as in M8.

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
| COMP-03 Vulnerability handling | WP-08 | WP-05 (VEX) |
| COMP-04 Secure by default and security updates | WP-08 | WP-11 |
| COMP-15 Accessibility | WP-14 | — |
| DIST-07 Updates | WP-08 | WP-13 |

DIST-03 (operator access) is a V1 feature of M14 since 2026-11-18; its "nothing by default" is a line of WP-08's test of secure defaults.

## The packages in short

- **WP-01 (lead, docs only; done 2026-11-18):** the answers below, each written where it belongs; ADR-0044 to ADR-0048 accepted (ADR-0048 in part); DIST-03 moved to M14 as a V1 feature in the roadmap and the feature list; PRIV-05, OPS-07 and DIST-07 reworded; the keep table of UX design §5.7 in its two groups; the glossary; the follow-ups that moved. The architecture chapters and the amendments of older records (ADR-0015, ADR-0017, ADR-0022, ADR-0025, ADR-0035, ADR-0012, ADR-0037) are written by the package whose code changes them, in the same change ("Definition of done"), as each package file names.
- **WP-02 (OPS-07; migration 54):** one backup file with a manifest and a copy of the replay list (ADR-0046), made while people work on both databases, by `bugaboo backup`, by the route behind *Make a backup now* and by the schedule; `bugaboo restore` with the server stopped, which signs everybody out, merges and replays the replay list, keeps the backup it made of the present for the security log's keep period, verifies the security log and starts a new anchor with the lost range named (M2-F09); `bugaboo keys import`; the master key's own access list on Windows (M1-F22).
- **WP-03 (PRIV-03; migration 55):** first landing: the links of each entity to a person, generated beside the marks, and the grown DATA-MARKS. Second landing: the one walk `PersonData`; *Download my data* as a job that writes one ZIP of readable pages and JSON, with the information of Art. 15(1) generated per kind; the administrator's export with *Download someone's data*, a reason, and an e-mail to the person; the export as an entry point of SEC-MATRIX (M2-F65).
- **WP-04 (PRIV-05; migration 56):** `KEEP_KINDS`, the one list of what is kept how long and who deletes it; every existing purge moved under one shape that deletes in steps; the trash purged after its period with its files, but never an issue whose time entries are still kept (M3-F25, M4-F131); records take their events along (M4-F93, M6-F45); dead jobs' payloads (M1-F21); the anchor file trimmed (M1-F26); time entries deleted only by an administrator through the general use case of old records, which also clears a removed person's legal-hold name with their last work days.
- **WP-05 (COMP-01):** xtask writes the CycloneDX and SPDX files from the lockfiles (no font files; the Lucide icons listed); DEP-SBOM goes from planned to active in every full run, with a test-only schema validator; the third-party notices file; the program carries both, and `GET /api/v1/about` answers them; VEX statements from the accepted advisories (M6-F126).
- **WP-06 (PRIV-04; migration 57):** *Switch off* (agents stop, standing rules end, assignments and memberships stay) and *Switch on*; the removal plan; *Remove a person* in one change plus the job `privacy.finish-removal`, through `bb-policy`'s existing `former_user_label`, `FormerUser` and `PersonRef::Former`; one legal-hold name with its reason and end; time entries as *Former user*; the replay list; every waiting follow-up (M2-F75, M3-F34, M3-F67, M4-F52, M4-F64, M5-F52, M5-F62, M5-F63, M7-F36, M8-F30, M8-F74).
- **WP-07 (OPS-06):** a backup named `before-<version>` before the first migration of an upgrade, chosen by comparing the set of applied migrations; on a failed migration, the restore of that backup and a plain explanation; the exit code of an upgrade stop; the upgrade test from one release program of `m8-done`, which becomes the check TEST-UPGRADE.
- **WP-08 (SEC-18, COMP-03, COMP-04, DIST-07):** `cargo xtask release` builds the release folder; `cargo xtask release sign` signs it with an offline key (`minisign`), and the program checks with `minisign-verify` against a list of two public keys; the installed `bugaboo update verify` warns on a downgrade; `SECURITY.md`, the vulnerability runbook and `security.txt`; the support period in the program and on the release page; the test of secure defaults (with "nothing by default" of DIST-03) and the CRA mapping.
- **WP-09 (PRIV-06; migration 58):** *My settings › Profile*: the name, and the e-mail address changed through a link to the new address with a note to the old one; an administrator with *Manage people* corrects name and e-mail of others with a reason; the keyed digest of addresses (M4-F122).
- **WP-10 (screens of PRIV-03, PRIV-04, PRIV-05):** *My settings › My data*, *Organization settings › Privacy › Keep and delete rules*, *Data exports*, *Switch off* and the window *Remove a person* on a person's page.
- **WP-11 (screens of OPS-07, COMP-01, COMP-04):** *Backups* (list, schedule, *Make a backup now*, and the command that restores) and *About*.
- **WP-13 (OPS-04):** the service installed, started, upgraded and removed for real in Windows Sandbox (through `wsb`) and in a fresh WSL distribution with systemd (M1-F57); the hardened unit (M1-F63); the services' failure settings, reading WP-07's exit code; `bugaboo service status`.
- **WP-14 (COMP-15):** every screen measured against WCAG 2.2 AA and EN 301 549; fixes; the EN 301 549 report, the statement template and the screen-reader checklist.
- **WP-15 (exit):** sample part 8; the long run; the benchmarks; the release candidate 0.6.0; an Opus security pass; the exit demo.

## Why the waves are in this order

Wave 0 first, because the answers decide what is kept, what is deleted and how a release is signed. Wave 1 lays the four foundations, which touch different places: backups `bb-server` and `bb-store`'s database code, the walk `bb-core::personal` and a new store module, the keep rules the jobs, and the parts list xtask. WP-03's links land first in it, because DATA-MARKS then asks every new person column for a link, and the other packages of the wave add person columns. Wave 2 needs wave 1: *Remove a person* reads the walk and the keep periods, upgrades back up with WP-02's code, signing signs WP-05's parts list. Wave 3 draws the screens once their routes answer and proves the service once there is a signed program that upgrades itself. Wave 4 measures accessibility once every screen of M9 exists, and the exit proves everything together. WP-15's measurements start only when WP-14 has pushed its last change, because both run browser tests (one full run at a time, "The lead's routine", point 6).

**Shared files, one entry each; rebase conflicts expected:** the packages of a wave each add their entries to `crates/bb-core/src/settings/catalog.rs`, `crates/bb-store/src/migrations/mod.rs`, `crates/bb-api/src/router.rs`, `crates/bb-policy/src/catalogue.rs`, `bb_app::jobs::JobKind`, `messages/en.json`, `crates/bb-server/src/cli/mod.rs`, `web/app/src/shared/paths.ts` and `web/app/src/features/settings/SettingsScreens.tsx`, and nothing else in those files; `web/app/src/features/people/PersonPage.tsx` is shared by WP-09 and WP-10 (WP-10 lands later and rebases). The one who lands later rebases and keeps both. **Migrations given out:** 54 (WP-02), 55 (WP-03), 56 (WP-04), 57 (WP-06), 58 (WP-09); none depends on another (architecture §7, *Storage types*). Migration 59 is not used in M9.

**New dependencies, each with its register row in its package:** `tar` joins the shipped program (WP-02; already in the lockfile for xtask); a JSON Schema validator used only by xtask's checks and never shipped (WP-05); `minisign-verify` in the program and `minisign` in xtask (WP-08).

## Follow-ups handled in M9

| Follow-up | Package |
|-----------|---------|
| M1-F22 (the master key's own access list on Windows), M2-F09 (a new anchor after a restore) | WP-02 |
| M2-F65 (the export as an entry point of the matrix), M3-F45 (the mark of the text of a field), M4-F52 (file bytes in the export), M6-F58 (marks of the notifications of work days), M7-F65 (the mark of text written with the assistant in exports) | WP-03 |
| M1-F21 (payloads of dead jobs), M1-F26 (the anchor file grows), M3-F25 (the trash's keep period), M4-F93 (file names in events), M4-F104 (purges in steps), M4-F131 (files of the trash), M6-F45 (time entries in events) | WP-04 |
| M6-F126 (VEX statements and their folder) | WP-05 |
| M2-F75, M3-F34, M3-F67, M4-F64, M5-F52, M5-F62, M5-F63, M7-F36, M8-F30, M8-F74 (what *Remove a person* deletes or scrubs) | WP-06 |
| M4-F122 (a keyed digest for the memory of sign-up e-mails) | WP-09 |
| M1-F57 (a real service on Windows and systemd), M1-F63 (the systemd unit hardened) | WP-13 |

**Moved out of M9 (M9-Q20):** M4-F61 (the mail server's own certificate authority), M4-F63 (try a failed job again) and M4-F128 (the note when sign-up e-mails were capped) wait for an operations package after the first release; their rows in the M4 follow-ups say so. **Not taken into M9:** M4-F121 (a sandbox for the child of a preview: still no safe crate; the accepted risk stays, and WP-15's security pass looks at it again), M4-F166 (binding many projects in one statement: needs thousands of projects), M0-F02 and M1-F02 (a Linux CI runner: WSL proves Linux locally, ADR-0037), and the speed follow-ups of the search (M5-F89 and its family), which keep their known gaps until the exit benchmarks say otherwise.

## Checks added or grown in M9

Each follows "Adding a rule" of [architecture §10](../../architecture/10-checks.md): the rule written where it belongs with `Enforced by: <ID>`, the entry in `tools/checks/catalog.toml`, the code in `xtask/src/checks/`, and its bad and good examples in `tools/checks/fixtures/<ID>/`. The META checks fail until all four are there.

- **DEP-SBOM** (WP-05, its `release` part WP-08; planned until now). Rule: COMP-01 and ADR-0047, decision 3, written into coding-standards §5 and architecture §7, *Compliance artifacts*. Catalogue: `when` grows from `release` to `ci` and `release`, `status` becomes active, `tool` names the generator of xtask instead of the outside tools. Code: `xtask/src/checks/dep_sbom.rs` makes the parts list into `target/` and fails when a crate that a program compiles in, or an npm package the browser app bundles, is missing or has no license or checksum, when an entry has no package, or when either file fails its schema (the schemas of CycloneDX 1.6 and SPDX 2.3 kept as test data); at `release` it also fails when the release folder lacks a file or its signature. Examples: a lockfile with a crate the parts list leaves out (bad), a parts list that matches (good).
- **TEST-UPGRADE** (WP-07; new). Rule: ADR-0046, decision 5, in architecture §11, *The upgrade test*. Catalogue: `when = ["ci", "release"]`, files `crates/bb-store/src/migrations/**`, `xtask/src/upgrade*`. Code: `xtask/src/checks/test_upgrade.rs` checks that the upgrade test exists, that it finds the tag of the previous milestone from Git (never written by hand), and that `cargo xtask ci` runs it; the test itself (`xtask/src/upgrade.rs`) builds one release program of the old tag, keeps only that program in `target/upgrade/<tag>/` and deletes the rest of that build, makes the sample with it on both databases, starts the new program on that data, and checks that the backup `before-<version>` was made, that every migration ran, and that the sample's counts and a list of known records read the same. Examples: a ci without the step (bad), the wired step (good).
- **TEST-CONCURRENT, the long run** (WP-15; grown). Rule: architecture §11, *Ten people at the same time*, grows a paragraph *The long run*: two hours on each database, by hand at the exit on a quiet computer, with an online backup made in the middle and restored into a second database after the run, whose invariants must hold too. Catalogue: the tool text names `cargo xtask load --people 10 --long 2h` instead of `--nightly` (there is no nightly run), and `when` gains `release`. Code: `xtask/src/load/` takes `--long <length>` and the backup step; `xtask/src/checks/test_concurrent.rs` checks that the long scenario and its backup step exist and are not switched off. Examples: a load plan without the backup step (bad), the full plan (good).
- **DATA-MARKS, the links of a person** (WP-03; grown). Rule: ADR-0044, decision 1, in ADR-0015 (amendment) and architecture §7, *Privacy engineering*. Code: the derive of the marks and `xtask/src/checks/data_marks.rs` fail an entity that has a column of a person's ID and no link that says what the person is to the row. Examples: an entity with `author: UserId` and no link (bad), with its link (good).
- **SEC-MATRIX** grows rows for every new route (exports, removal, switching off, corrections, backups, keep rules) for every starting role, on both databases, and the entry point `export` (M2-F65).
- **TEST-A11Y** grows the measures of WP-14 (reflow, zoom, text spacing) for every address of `paths.ts`.

## Reviews

Every package gets a code review with the SSOT/DRY/ODR gate, with a fresh reviewer for each fix round. **Privacy review** (Opus, fresh): WP-03, WP-04, WP-06, WP-09, WP-10, and WP-02 for what a backup and a restore keep. **Security review** (Opus, fresh): WP-02 (restore), WP-07, WP-08 (signing, the keys never in a folder, `security.txt`), WP-09 (change of e-mail), WP-13 (the service account and the unit). Screens (WP-09, WP-10, WP-11, WP-14) get a UX review from pictures in the four themes on computer and phone, and a look at frame samples for anything that changes in place (UX §6.4, nothing moves under the cursor).

## The exit demo

As written in 04-milestones (M9), broken into steps. Steps 1 to 3 are tests of xtask and runs on the clean systems, recorded on the review page; steps 4 to 8 are tests in `web/app/e2e/m9-exit-demo.spec.ts` (with the commands of steps 4 and 8 in Rust tests of `bb-server`), titled by number and a plain sentence, with pictures in the four themes on computer and phone.

1. **"The release 0.6.0 is built, signed and checked"**: `cargo xtask release` writes the folder; the installed `bugaboo update verify` accepts every file, refuses one with a changed byte (`sec_18_a_signed_file_verifies_and_a_changed_byte_fails`, a key made for the test), and warns on an older version.
2. **"Installed on a clean Windows computer as a service"**: in Windows Sandbox, the signed program is checked, `bugaboo setup --non-interactive` and `bugaboo service install` run, the service starts with the computer's service manager, and the browser setup opens with the setup code (WP-13's run, its log kept in `.work/`).
3. **"Installed in a fresh WSL as a service"**: the same in a WSL distribution imported for the test, with systemd on, then removed again.
4. **"A backup made while people work is restored"**: Ada makes a backup from *Backups* while two people save issues; an issue is changed and Ben is switched off; the service is stopped and `bugaboo restore` brings the backup back; the change is gone, Ben is still switched off (the replay list), everybody was signed out, and the security log says what was restored.
5. **"Lea downloads her data, and an administrator downloads Ben's"**: Lea gets her file from *My data*, with the purposes, legal bases, keep periods and recipients in its README; Ada makes Ben's in *Data exports* with a reason; Ben is told by e-mail, and the security log names both.
6. **"Ben is removed"**: the window *Remove a person* says what goes, what stays and that his work days and his name for them are kept until their date; after it Ben is *Former user K7Q2* on his issues, comments and time entries, and Hana (*HR*) still sees his work days under his name.
7. **"The About page shows the release"**: version 0.6.0, the support period, the licenses and the parts list to download; the release folder holds the same parts list, its signature and the notices.
8. **"An installation of M8 is upgraded without losing anything"**: TEST-UPGRADE on both databases, and once by hand on the service of step 2.

## Questions for the maintainer

**Decided (2026-11-18): the maintainer took every recommendation** ("do as your recommendation", on the question page). Each answer is written where it belongs; the record or file is named after it.

1. **M9-Q1, which data is deleted by itself:** two groups in one table. By itself after its period: IP addresses, sessions and sign-in history, notifications, the trash, data exports, payloads of dead jobs, away times, the security log, backups. By an administrator after the period, with a count and *Delete them*: work days and time entries. An issue in the trash whose time entries are still inside their keep period stays in the trash, hidden, until they may go. ADR-0045, decision 1; PRIV-05; UX design §5.7; the glossary (*Keep and delete rules*).
2. **M9-Q2, time entries:** at least 10 years at the start (an organization setting, no floor of law; receipts are kept 8 years since 2025 and the books 10), then an administrator deletes them. ADR-0045, decision 1; UX design §5.7.
3. **M9-Q3, texts of a removed person:** *Keep their comments, shown as Former user* (the start, `privacy.removed-texts`) or *Delete their comments*; everything else stays. ADR-0044, decision 7.
4. **M9-Q4, switch off:** *Switch off* and *Switch on* with *Manage people*; a removal needs a switched-off person. ADR-0044, decision 5; the glossary.
5. **M9-Q5, agents, rules, assignments, memberships:** at *Switch off* agents stop and standing rules end, assignments and memberships stay; at *Remove a person* agents and rules are deleted, memberships end, assignments stay as *Former user*. ADR-0044, decisions 5 and 6; WP-06.
6. **M9-Q6, names on time entries:** no; time entries show *Former user*. ADR-0044, decision 6.
7. **M9-Q7, the export's content:** as recommended, with the Art. 15(1) information per kind, generated. ADR-0044, decisions 2 and 3; WP-03.
8. **M9-Q8, an administrator's export:** *Download someone's data*, always with a reason, the person told in the app and by e-mail. ADR-0044, decision 4; the glossary.
9. **M9-Q9, a backup's content:** one file with the data, the attached files and a copy of the replay list, no keys. ADR-0046, decision 1.
10. **M9-Q10, restore:** from the command line only in M9; *Backups* shows the command. ADR-0046, decision 4; OPS-07.
11. **M9-Q11, a restore and what it must not undo:** every restore signs everybody out and replays the replay list; its report names what else is gone. ADR-0044, decision 8; ADR-0046, decision 4.
12. **M9-Q12, signing:** our own Ed25519 keys in the minisign format, a release key and a backup key kept offline and apart; Windows code signing later. ADR-0047, decision 2.
13. **M9-Q13, support period:** 0.6.0 is a free public preview, only the newest release gets fixes until 1.0; from 1.0, 5 years per minor release. ADR-0047, decision 5.
14. **M9-Q14, updates:** signed releases, `bugaboo update verify`, the automatic backup and migration at start, release notes; the notice inside Bugaboo stays OPS-11 (M14). ADR-0047, decision 8; DIST-07.
15. **M9-Q15, support access:** as recommended, for M14 (see Q20). ADR-0048, decisions 2 to 6, planned for M14.
16. **M9-Q16, `security.txt`:** the organization's contact first, then ours; it expires one year after the release. ADR-0047, decision 4; WP-08.
17. **M9-Q17, clean computers:** Windows Sandbox (you switch it on once) and a throwaway WSL distribution in `.work/`. WP-13.
18. **M9-Q18, screen reader:** machine checks in WP-14, and about an hour of yours with NVDA or Narrator on its checklist. WP-14.
19. **M9-Q19, security review:** an Opus pass over each boundary before 0.6.0; a paid outside test before 1.0. WP-15.
20. **M9-Q20, the cut:** all three cuts: restore from the command line only; support access (DIST-03) to M14 as a V1 feature; M4-F61, M4-F63 and M4-F128 to a later operations package. 04-milestones (M9, M14); the feature list (DIST-03); ADR-0048; the M4 follow-ups.
21. **M9-Q21, what stays out:** the list below.

## What stays out of M9 (M9-Q21)

Each item keeps its place in the feature list and the plan:

- **Encrypted backups** (SEC-20, V1) and backups to outside storage (S3).
- **The restore from the screen** (M9-Q10, M9-Q20): not part of the MVP; a later milestone may add it on ADR-0046.
- **Support access** (DIST-03, ADR-0048 decisions 2 to 6): M14, with the served model.
- **The update notice inside Bugaboo** with its signed feed (OPS-11, M14), and updates of served instances (DIST-02, M14).
- **The record of processing** and the privacy notice template (PRIV-07, M14); the data-breach help (PRIV-11, M14); the incident runbook as manufacturer and operator in full (DIST-08, M14).
- **The technical documentation file** and the EU declaration of conformity (COMP-05, M14), and the CE marking, which the legal review decides.
- **Packages** (winget, MSI, deb, rpm, container image; OPS-05, M14).
- **Windows code signing** (Authenticode), until the maintainer decides on a certificate (M9-Q12).
- **ARM64 programs** (decided by the rules).
- **A sandbox for the child of a preview** (M4-F121), until a safe way exists.
- **M4-F61, M4-F63, M4-F128** (an operations package after the first release).

## What the maintainer still does

- **Before wave 3 (WP-13):** switch on the Windows feature *Windows Sandbox* (administrator rights and a restart). Until then WP-13 can prove only the WSL half.
- **Before the release candidate (WP-15):** make the release key and the backup key with `cargo xtask release keygen`, keep both offline and apart with their passphrases, and hand over the two public keys; until then the candidate is signed with a test key and marked so.
- **At the exit:** about an hour with NVDA or Narrator on WP-14's checklist; sign the release and publish it on GitHub; optionally repeat the install on a real clean PC.

## Risks

- **Removal touches almost every table.** WP-06 reads the walk instead of naming tables, so a table it misses is a missing link, which DATA-MARKS refuses; the privacy review reads the removal plan against the generated personal-data list.
- **A restore can undo security decisions.** Every restore signs everybody out and replays the replay list (removals, switch-offs, keys, grants and passkeys taken back); the report names what else is gone. The list lives in three places a restore merges (the data folder, every backup, the copy of `keys export`), so a lost computer does not lose it.
- **The security log's chain after a restore.** A restore that forgot the anchor would leave the status page red for ever (M2-F09). WP-02's tests restore an older backup and expect a verified log with a named gap.
- **No clean computer, no certificate, no systemd here.** WP-13 proves the service in Windows Sandbox and a throwaway WSL distribution; Windows will warn about an unknown publisher until a certificate is bought (M9-Q12).
- **The law moves under the release.** The CRA's reporting duty applies now, the Product Liability Directive from 9 December 2026. A free preview that is not offered commercially keeps both light (ADR-0036, decision 5; ADR-0047, decision 5); the legal review is still open (compliance.md, open points 1 and 6).
- **The full run is not reliably green.** M8 ended with browser tests that pass alone and fail in a full run (M8-F10, F220). Each landing of M9 needs a green run; the lead keeps the rule of one full run at a time and records each flaky test with its follow-up instead of running again blindly.
- **Disk space.** Backups, the old program of the upgrade test (one file kept, its build deleted) and the long run's datasets are large. Each lives under the folder's `target/` or `.work/`, and `cargo xtask folder clean` frees it after the landing.
