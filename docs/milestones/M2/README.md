# M2 — People, permissions, projects and the app shell: work packages

Goal, features and exit demo: see [04-milestones.md](../../04-milestones.md) (M2). Rules: [`CLAUDE.md`](../../../CLAUDE.md) — **0 warnings, 0 errors, no disabled useful rules**, tests first, every new dependency registered, every new rule with a check.

M2 builds the permission engine first and everything that asks it afterwards, so it runs in **five waves**. A wave starts when the previous one is merged to `main`. At most four builders run at once.

| Wave | WP | Name | Who | Folder / branch |
|------|----|------|-----|-----------------|
| 1 | [WP-01](WP-01-policy-model-docs.md) | Permission model, words and M2 design docs | Lead | `bugaboo.win1` / `WIN1` |
| 1 | [WP-02](WP-02-lean-api.md) | Lean answers: `fields`, cursor paging, ETags, compression, bundle budget | Builder | `bugaboo.win2` / `WIN2` |
| 1 | [WP-03](WP-03-app-shell.md) | App shell: sidebar, top bar, phone bar, panels, keyboard, optimistic helpers, themes | Builder | `bugaboo.win3` / `WIN3` |
| 1 | [WP-04](WP-04-server-follow-ups.md) | Server follow-ups from M1 and the words check | Builder | `bugaboo.win4` / `WIN4` |
| 2 | [WP-05](WP-05-policy-engine.md) | Policy engine: catalogue, roles, groups, assignments, field masks, `Authorized<T>`, `PersonRef` | Builder | `bugaboo.win2` / `WIN2` |
| 2 | [WP-06](WP-06-sample-tool.md) | Sample organization tool and its people (`cargo xtask dataset sample`) | Builder | `bugaboo.win3` / `WIN3` |
| 3 | [WP-07](WP-07-settings-in-layers.md) | Settings in layers and *My settings* | Builder | `bugaboo.win2` / `WIN2` |
| 3 | [WP-08](WP-08-projects.md) | Projects, sub-projects, short names, moving with a preview | Builder | `bugaboo.win3` / `WIN3` |
| 3 | [WP-09](WP-09-access-keys.md) | Access keys and two-step sign-in by role | Builder | `bugaboo.win4` / `WIN4` |
| 3 | [WP-10](WP-10-permission-checks.md) | Permission checks everywhere: SEC-MATRIX, ARCH-PERMS, ARCH-API-OUT, ARCH-NO-COUNTERS | Builder | `bugaboo.win5` / `WIN5` |
| 4 | [WP-11](WP-11-people-groups-roles.md) | *Organization settings*: People, Groups, Roles & permissions | Builder | `bugaboo.win2` / `WIN2` |
| 4 | [WP-12](WP-12-field-permissions-and-check-access.md) | Field permissions, *Who sees people* and *Check access* | Builder | `bugaboo.win3` / `WIN3` |
| 5 | [WP-13](WP-13-sample-organization-and-exit-demo.md) | Sample organization, part 1, and the exit demo | Builder | `bugaboo.win4` / `WIN4` |

Progress (which package is building, merged, reviewed) is kept only in [`docs/STATUS.md`](../../STATUS.md).

**Every builder starts by resetting its folder to the current `main`:** `git fetch origin && git checkout -B WINn origin/main`, then `cargo xtask setup`. `win5` is used only after `origin/WIN5` was overwritten (STATUS, next action 2); until then WP-10 runs in `win2` after WP-07, or in a new `win6`.

## Before wave 1

The maintainer answers the questions at the end of this page (M2-Q1 … M2-Q7). WP-01 writes the answers into the docs. Wave 1 builders do not need the answers, except WP-02 for compression (M2-Q2): it builds everything else first.

## Why the waves are in this order

| Wave | Needs merged | Because |
|------|--------------|---------|
| 1 | M1 | WP-02, WP-03 and WP-04 touch no permission code; WP-01 is docs |
| 2 | WP-01, WP-02, WP-03 | WP-05 builds the model WP-01 writes down, plugs `Authorized<T>` into WP-02's `fields` helper and turns WP-03's menus over to permissions; WP-06 needs the `bb-sample` entry in the module map (WP-01) |
| 3 | WP-02, WP-03, WP-05 | Every new route asks `bb-policy` and declares its permission (WP-05), answers lean (WP-02), and every new screen sits in the shell (WP-03) |
| 4 | WP-07, WP-08, WP-10 | The settings screens assign roles on projects (WP-08), store through the settings layers (WP-07), and their routes must pass ARCH-PERMS and SEC-MATRIX (WP-10) |
| 5 | WP-06, WP-09, WP-11, WP-12 | The sample organization is made through the use cases of all packages, and the exit demo uses the screens of WP-11 and WP-12 |

Inside wave 3, WP-10 merges last: it takes in the routes of WP-07, WP-08 and WP-09 and adds their SEC-MATRIX examples, so those three only declare their permissions (WP-05's mechanism) and never wait for WP-10.

## File ownership

A package changes only what it owns; shared files get added lines only; the lead resolves conflicts at merge. The shared files of M2:

| File | Rule |
|------|------|
| `messages/en.json`, `docs/glossary.md` | Added lines only; new words go into the glossary first (WP-01 adds the M2 words) |
| `tools/checks/catalog.toml` | Each package changes only the entries of the checks it switches on |
| `docs/third-party/README.md` | One row per new dependency, in the same commit |
| `crates/bb-core/src/settings/catalog.rs` | Added settings only (WP-07 adds the *My settings* settings) |
| `crates/bb-store/src/migrations/` | One new numbered migration per package: WP-05 `m0003`, WP-07 `m0004`, WP-08 `m0005`, WP-09 `m0006`; later packages ask the lead for a number |
| `web/app/src/shared/paths.ts`, the menus of the shell | WP-03 moves the addresses to the ones in the UX design (§2.1); later packages add lines |
| `web/packages/api-client/openapi.json` | Generated (`cargo xtask gen api`); a merge conflict is solved by generating again |
| `docs/architecture/12-security.md` | WP-01 writes the M2 section; packages add rows with the tests that prove them |

## Follow-ups handled in M2

| Follow-up | Package |
|-----------|---------|
| M0-F04 (page title from the message catalog) | WP-03 |
| M0-F05 (one frame in the device theme; review again) | WP-03 |
| M1-F03 (certificate renewed only at start) | WP-04 |
| M1-F12 (server-only message keys ship to the browser) | WP-03 |
| M1-F18 (generated test that runs every command against the fakes) | WP-05 |
| M1-F32 (`x-permission` in the API description) | WP-05 (declared on every route), WP-10 (checked) |
| M1-F40 (security-log entry that cannot be written; status page warns) | WP-04 |
| M1-F45 (two-step sign-in required per role) | WP-05 (the rule on the role), WP-09 (sign-in asks for it) |
| M1-F52 (fallback when a security-log entry cannot be written has no test) | WP-05 |
| M1-F56 (test hook `rename_table_for_tests` in `bb-store`) | WP-05 |
| M1-F58 (`setup-code.txt` stays until the next start) | WP-04 |
| M1-F60 (status page verifies the whole log on every visit) | WP-04 |
| M1-F62 (`--database-url` shows the password in the process list) | WP-04 |
| M1-F65 (an existing data folder with too open rights is never reported) | WP-04 (report at start; the backup command stays with OPS-06 in M9) |
| M1-F66 (own certificate in `tls/` before the first start) | WP-04 |

**Re-planned:** M1-F49 (sign-in limit keyed by account and usual network) says "M2", but no M2 feature makes sessions remember networks. It moves to M4 with M1-F53 (warning on a sign-in from a new network); WP-01 changes its row. **Not in M2:** M0-F11 (cosmetic, next time those files change), M1-F10 (WebAssembly budget, M4: PERF-BUNDLE covers JavaScript now), M1-F44 (lists with a cap, when a list grows), and the job-runner follow-ups M1-F21, M1-F43, M1-F61 (M4).

## Checks switched on in M2

| Check | Package |
|-------|---------|
| PERF-BUNDLE | WP-02 |
| TEST-A11Y, TXT-LITERAL, STY-TOKENS, ARCH-WEB-FEATURES | WP-03 |
| TXT-TERMS | WP-04 |
| ARCH-AUTHZ-ONLY, ARCH-PERSON-REF | WP-05 |
| TEST-SAMPLE-DATA (people) | WP-06; projects, groups and roles in WP-13 |
| SEC-MATRIX, ARCH-PERMS, ARCH-API-OUT, ARCH-NO-COUNTERS | WP-10 |

ARCH-LEAN is already active; WP-02 extends it. TXT-LITERAL, STY-TOKENS, ARCH-WEB-FEATURES and TXT-TERMS are not in the M2 list of 04-milestones, but they are planned and scheduled nowhere, and M2 adds most of the browser code and words; the lead may move them back out if a wave runs long.

## What M2 covers of each feature

Some M2 features are about things that come later (issues in M3, search in M5). M2 builds the part that exists now and the machinery that later milestones plug into:

- **SEC-04, SEC-06:** the field catalogue, field masks and `Authorized<T>` work and are tested for every built-in field; the entry points of M2 (API answers, *Check access*) obey them. Each later entry point adds its row to SEC-MATRIX (issues M3, e-mail M4, search M5, assistant and AI apps M8, exports M9). Rules per issue type come with issue types (M3).
- **PRIV-12:** every kind of people information can be set; the kinds shown in M2 screens obey it through `PersonRef`. **PRIV-13:** random labels for removed people are made by `PersonRef`; the removal itself is PRIV-04 (M9).
- **UX-05:** the shortcut system, `?` and the shortcuts of M2 screens; each later screen adds its own. The quick-search frame opens with `Ctrl+K` and goes to pages and projects; finding issues and running actions is CFG-08 (M5).
- **PRJ-02:** settings and permissions are inherited down the tree; workflows, issue types, fields and layouts join when they exist.

## Later (not in M2)

*See as …* (SEC-07, M12), restricted issues (SEC-05, V1), bot accounts (AUTH-08, M12), nested groups (L), company sign-in groups (AUTH-04, M12), the theme editor and own themes (THM-03, THM-04, M7), project accent colors (THM-05), the content of quick search (CFG-08, M5), *Remove a person* (PRIV-04, M9), the issues and everything else in the sample organization (M3 on), passkeys in the sample organization.

## Questions for the maintainer

- **M2-Q1 — Taking a role away on a sub-project.** SEC-03 asks for an explicit *deny*. Proposal: on a project's *Who has which role*, a person or group can be set to **No access** for that project and its sub-projects; it wins over any role given higher up, and at the same project it wins over a role given to one of their groups. Is *No access* the right word (it becomes a glossary entry), and should it win over a group's role at the same project?
- **M2-Q2 — Compressing API answers.** PERF-03 asks for Brotli or zstd on API answers, but the security design says JSON is never compressed, because a compressed answer that holds a secret next to text an attacker chooses can give the secret away (BREACH). Proposal: compress JSON answers above 1 KiB, except the routes that hand out a secret (a new access key, backup codes, setting up a code app, an invitation link); a list in the code names those routes and a test keeps it complete. The other choice is to keep JSON uncompressed and change PERF-03.
- **M2-Q3 — Names in settings screens.** *Who sees people* hides names from some roles. Proposal: the people who may manage people or roles (Administrators, and Project leads for their projects) always see names in *Organization settings*, *Who has which role* and *Check access*, because they cannot do the job without them; everybody else sees member lists by a new kind of people information, *Project members* (default: Names for Team members and up, Role only for Contributors and Viewers, Hidden for Guests).
- **M2-Q4 — Short names.** Proposal: 2 to 10 capital letters or digits, starting with a letter (`WEB`, `APP2`), unique in the whole organization, and changeable later; old addresses (`/p/WEB`) keep working, as old issue IDs will (ADR-0021). Can short names change after the project was made?
- **M2-Q5 — What the starting roles may do.** WP-01 writes a table of the permissions of the six starting roles (Administrator, Project lead, Team member, Contributor, Viewer, Guest) and their default *Who sees people* (the table in the UX design §5.2 as the starting point). Please confirm or change it before wave 2. In short: Administrators everything; Project leads manage their projects, sub-projects and who has which role there; Team members change issues; Contributors create issues and change their own; Viewers only look; Guests see only what they reported and see people by role only.
- **M2-Q6 — Access keys.** Proposal: *What it may do* has three plain choices — **See only**, **See and change**, **Everything I can do** — with single permissions under *More options*, and optionally a list of projects; every key runs out, after 90 days unless the person picks another date, at most after one year (an organization setting).
- **M2-Q7 — The fields in the field-permission matrix.** Issues come in M3, but the matrix and the exit demo (hiding *Cost center*) need the built-in fields now. Proposal: Title, Description, Status, Priority, Assigned to, Reported by, Due date, Labels, Tags, Planned for release, Found in release, Estimated time, Time left, Time spent, Activity, Cost center. Own fields join the matrix in M3 (CF-06).
