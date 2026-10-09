# M2C — Clean-up: one source, one definition, one theme file

A clean-up step between M2 and M3, asked for by the maintainer:

- **SSOT, ODR and DRY are mandatory for all existing and new code** (coding-standards §1, review gate).
- **All UI theming stays in one theming file**, the way the sister project does it.

It adds no feature. Every package removes duplicated knowledge that the [audit](audit.md) found, and makes a check catch it from now on where a tool can. Rules: [`CLAUDE.md`](../../../CLAUDE.md). Progress lives only in [`docs/STATUS.md`](../../STATUS.md).

## Waves

| Wave | Package | Name | Folder / branch | Audit findings |
|------|---------|------|-----------------|----------------|
| 1 | C-01 | Theme engine and decision record (lead decides the format first) | `win1` lead, then `win4` / `WIN4` | D |
| 1 | C-02 | Rust core: one definition of levels, walks, places, paging, words | `win2` / `WIN2` | A1–A17 |
| 1 | C-03 | API marks, routes and server test rigs | `win3` / `WIN3` | B1, B2, B6 (bug), B7, B8, B11 (server part) |
| 2 | C-04 | xtask check infrastructure and LINT-DRY | `win3` / `WIN3` | B3–B5, B6 (xtask part), B9–B12 |
| 2 | C-05 | Web app and e2e: generated choices, one paged list, one picker | `win2` / `WIN2` | C1–C3, C5–C12 |
| 3 | C-06 | Move every component onto the theme file | `win4` / `WIN4` | D, C4 |
| 4 | C-07 | No private information in Git (SEC-PRIVATE) | `win2` / `WIN2` | maintainer |

- Wave 2 waits for wave 1: C-04 builds on C-03's route table and shared marks, and C-05 builds on C-02's generated words and paging.
- Wave 3 waits for C-05, because both touch every web component.
- Packages in a wave own separate files. A shared list (`messages/en.json`, `catalog.toml`, follow-ups) gets added lines only.

## The packages

### C-01 Theme engine and decision record

1. A new ADR replaces the styling part of ADR-0010. It covers:
   - one theming file holds every value (tokens with their variants per mode, density and contrast) and every component rule;
   - components carry no style values and no class names of their own; they emit `data-part`, `data-state` and `data-variant`;
   - the file is compiled **at build time** into one static CSS file, because the content security policy is `style-src 'self'` and the sister project's runtime-injected styles are not allowed here;
   - custom themes still store token values only.
2. The theming file at `design/theme.<format>` replaces `design/tokens.json`. It gains the missing tokens: font weight, line height, border width, layers (z-index), opacity, motion and keyframes, focus ring, and forced-colors as a mode.
3. `cargo xtask gen theme` writes the static CSS, the TypeScript token and part names, and the docs copy. `docs/design/*.css` is generated too.
4. New check THEME-ONLY. It forbids `style=`, literal or template `className`, CSS modules and any CSS file other than the generated one. It allows React Aria's own state attributes. It has the four parts. STY-TOKENS, STY-CONTRAST and LINT-CSS move onto the theming file.
5. Prove the engine on the `packages/ui` primitives (Button, TextField, Alert, Overlay, Switch, Checkbox, Link). The app features move in C-06.

### C-02 Rust core

Fix audit section A:

- one level-subject trait for fields and people across bb-policy, bb-app and bb-store;
- `nearest_rule` as the only level walk;
- `Policy::inputs_for` as the only "rows at a chain", used by the store and the fake;
- one evaluate-and-narrow;
- one move-rules function;
- tree walks and the depth rule on `Tree` only;
- `From<Option<ProjectId>> for Place`;
- paging helpers in bb-core and the store;
- one change-error mapping, with catch-all handlers replaced by typed errors;
- a word enum for entity kinds;
- one text-field rule;
- hex, comma lists and the domain check once each;
- derived message keys for error codes, permissions and starting roles;
- `words!` exported and used by every word enum;
- one store read per question with a pool/transaction executor instead of `X` / `X_in` pairs;
- the bad patterns in A17, including splitting the files that are far over the size guideline.

### C-03 API marks, routes and server test rigs

- `permissions::ROUTES` becomes the one table of a route's permission, public/CSRF status, and stored/never-stored/secret answer. `openapi.rs` adds the marks from it. No handler writes `x-never-stored` or `x-secret` by hand.
- The marks and words move into a small crate that xtask may import.
- `CSRF_EXEMPT` and `READS_THAT_MAY_BE_KEPT` are derived from the table.
- A public `openapi::operations()` serves every test that walks the routes. Fix the PATCH bug.
- Remove the double checks in B7.
- bb-server test rigs:
  - `Browser::signed_in` and `invite`, one setup helper and `Options::for_tests()`;
  - `bb_testing::Failure`, named rig sizes, typed CSRF header and cookie names;
  - table-driven header tests.

### C-04 xtask check infrastructure and LINT-DRY

- `checks/exemptions.rs` (one loader, reason rules, stale-entry tracking) used by all exemption files.
- One Rust and web code reader with `contains_word`.
- One source selector driven by the catalog's `files` globs; `normalize` everywhere.
- `api_spec.rs` for every check that reads the API description.
- One `ports.json` reader; one `sample.json` schema shared with bb-sample; one `DatabaseKind` and environment-name source.
- Two generator lists merged.
- **LINT-DRY switched on:** jscpd pinned in `tools/dev/tools.toml`, registered in `docs/third-party/README.md`, with a threshold over Rust and TypeScript, bad and good examples, and the existing code passing.

### C-05 Web app and e2e

- The generator writes setting choices and defaults, page sizes and maxima, permission keys and field-name lists.
- One `shared/paged.ts`, one `SearchPicker` in `packages/ui`, one project-page table, one `QueryGate`, one person-name module, one storage module with a key registry.
- Query keys only in the cache module.
- Variant props instead of boolean flags.
- `shared/my-settings.ts` split by job; dead code removed.
- e2e:
  - addresses from `paths.ts` and a typed API session;
  - one `check`, one `shot`;
  - setup, sign-in and code-app setup through `flows.ts`;
  - `giveRole` everywhere;
  - viewports and breakpoints from the generated tokens.

### C-06 Move every component onto the theme file

- Every app feature and the shell move from CSS modules to `data-part` rules in the theming file.
- Shared classes (`.stack`, `.text`, `.muted`, `.row`, `.list`) become parts once. The focus ring, skip link and forced-colors rules exist once.
- THEME-ONLY passes with no exemption. Screenshots in four themes on phone and computer show no visual change, except the agreed token fixes.

### C-07 No private information in Git

Requirement: nothing private about the maintainer may end up in Git. The lead rewrote the history once; this package keeps it so. The commit author stays as the maintainer set it.

1. **SEC-PRIVATE** (all four parts: rule in the docs, catalog entry, `xtask/src/checks/sec_private.rs`, failing and passing fixtures) runs on commit, on push and in ci. It fails on:
   - a user-profile path with an account name in a tracked text file or a commit message (`X:\Users\<name>\`, `/c/Users/<name>/`, `/home/<name>/`, `/Users/<name>/`); `%USERPROFILE%` and `~` are the allowed forms;
   - a Claude session ID or link (`originSessionId:`, `claude.ai/code/session_…`, `session_` followed by a long ID), and a `Claude-Session:` line in a commit message;
   - the account name and computer name of the machine the check runs on (`USERNAME`, `COMPUTERNAME`, `USER`, `HOSTNAME`), as whole words, and the words in an optional private list outside the repository (`a private-words file`, one per line, never committed). The check never prints a private word; it names the file, the line and the rule that matched.
2. Commit messages are checked by the commit-msg hook and, for the pushed range, by the pre-push hook.
3. Claude Code adds `originSessionId:` to every memory note it saves. `cargo xtask hook post-edit` strips that line from `.claude/memory/*.md` right after the edit, so a commit does not fail on it.
4. Made-up names in test data (`EXAMPLE-PC\ana`) stay allowed, and so does the breached-password list (whole-word matching; a data file the project does not write is exempt with a reason).

## Done means

- Every audit finding is fixed, or recorded as a follow-up with a reason (`M2C-Fnn` in a follow-ups file next to this one).
- LINT-DRY and THEME-ONLY are active and green.
- `cargo xtask ci` is green.
- Every package is reviewed with the SSOT/DRY/ODR review gate.

## Question for the maintainer

- **M2C-Q1, the format of the theming file** (answered: **KDL**, `design/theme.kdl`). Proposal: **KDL, like the sister project** (`design/theme.kdl`). It allows comments, keeps each value's variants on one line, and is read only by `cargo xtask gen theme` at build time, so the browser never parses it. The alternative is to keep JSON (`design/theme.json`): it needs no new dependency, but it gets long and has no comments.
