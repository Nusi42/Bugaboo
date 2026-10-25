# Coding Standards

Status: **Draft for review** · Applies to every contributor, human or AI agent.

These rules are not suggestions. Anything that can be checked by a machine is enforced by
the hooks in §7. Anything that cannot is checked in review.

---

## 1. Mandatory principles: SSOT, DRY, ODR

### SSOT — Single Source of Truth

Every piece of knowledge has exactly one authoritative home. Everything else is
**generated** from it or **reads** from it, never copied by hand.

| Knowledge                     | Single source                    | Derived from it                                      |
|-------------------------------|----------------------------------|------------------------------------------------------|
| API types & endpoints         | Rust handler/DTO definitions     | OpenAPI spec → generated TypeScript client & types   |
| Permissions catalogue         | One Rust module                  | API, admin UI, docs, policy tests                    |
| BQL grammar                   | Rust parser crate                | Server + WebAssembly build for the editor            |
| Validation rules              | Server-side field/schema definitions | Exported schema drives instant client-side feedback (the server still decides) |
| Design tokens                 | Token schema                     | CSS variables, TS types, theme editor, contrast checker |
| User-facing strings           | One message catalog              | UI and server messages, all looked up by key         |
| Error codes                   | One Rust enum                    | API responses, TS union type, message-catalog keys   |
| Names of things (terminology) | [glossary.md](glossary.md)       | UI text, user docs, API and code names               |
| Dependencies                  | Lockfiles + `docs/third-party/` register | SBOM, third-party notices, About › Licenses     |
| Personal data                 | Personal-data marks in the data model | Data export, removal, keep-and-delete rules, record of processing, data-flow map |
| Settings & their defaults     | One settings registry            | Config loader, admin UI dialogs, docs                |
| Database schema               | Migrations                       | Query types (compile-time checked)                   |

### DRY — Don't Repeat Yourself

- DRY is about **knowledge**, not text that happens to look alike. Two pieces of code that
  look the same but change for different reasons should stay separate.
- Before writing anything, search for an existing helper, component or module. If one is
  close, extend it instead of adding a near-copy.
- Duplicated code blocks above a threshold fail CI (§7).

### ODR — One Definition Rule

- Every type, constant, enum, error code, route, permission, setting key and message key is
  **defined once**. Every other place imports or generates it.
- No re-declared constants, no hand-mirrored enums in TypeScript, no magic numbers or
  strings repeated across files.

---

### Review gate (mandatory, existing and new code)

SSOT, DRY and ODR are a merge gate, not a wish (review: a person or reviewer agent judges
knowledge, not text). Every review of a work package checks the whole diff **and the code it
touches** against this list, and a finding blocks the merge until it is fixed:

- **A second definition** of a type, constant, limit, enum, error code, route, permission,
  setting key, message key, file or folder name, port, or rule that already exists somewhere
  (ODR). Re-export or import it instead.
- **A copy of logic** that already exists (the same decision, walk, parse, format or check
  written a second time), even when the text differs (DRY). Extract or reuse the one helper.
- **Knowledge kept in two places by hand** and held together only by a test that compares
  them (SSOT). Generate one from the other, or read both from one file.
  **Approved exceptions** are the ones an ADR names, each held together by test vectors that
  are generated from one side (not written by hand) and checked by a named check. Today there is
  one approval, recorded in the amendment of
  [ADR-0033](adr/0033-notifications-and-e-mail.md): (1) the region-dependent writing of dates,
  numbers and time zones, kept in two implementations (Rust for e-mail, `Intl` in the browser,
  because the choice *device* cannot run on the server), and (2) the synchronous TypeScript
  `listText`, `durationText` and `ownValueText` of the screens that cannot wait for the module,
  which stay beside the shared Rust wording. Both are checked against the vectors that
  `cargo xtask gen word-vectors` writes from Rust: DOC-GENERATED fails when the file differs from
  the code, and the browser test `web/app/src/shared/value-words.test.ts` fails when the screen
  writes a case differently. A reviewer does not record these as findings; any other pair is a
  finding.
- **A magic number or string** that names a rule (a limit, a size, a timeout, a key) instead
  of a named constant from its single home.
- **Common bad patterns:** a function that does several unrelated things; a long parameter
  list instead of a type; boolean flags that select behaviour; stringly typed values where a
  type or enum exists; dead or commented-out code; a catch-all that hides errors; copy-paste
  tests that should be one table-driven test.

Existing code is held to the same rule: a reviewer who finds such a problem in code a package
touches records it as a finding, and the clean-up milestone step removes what is already
there. LINT-DRY (§7) catches copied blocks automatically; the rest is review.

---

## 2. General

- **Comments explain *why*, not *what*.** Names should make the *what* obvious.
- **Small units:** one responsibility per function and module. A file over ~400 lines
  needs a reason, and a Rust file over 800 lines fails unless `tools/checks/size-exemptions.toml` names it with the follow-up that splits it. **A named file may not grow:** its entry holds the most lines it may have, so a package that needs more room in it splits it first (rule from the review of WP-15). Enforced by: LINT-FILE-SIZE.
- **Errors are values:** domain errors are typed and handled. Never silently swallow one.
- **No fixed paths.** Product code, tools and scripts never write down a path of the system: no `C:\Windows`, `/etc/…`, `/usr/sbin/…`, `/proc/…`, no user, program or temp folder. They ask the operating system: the Windows calls for system and known folders, the account database through the system's calls (`geteuid`, `getpwuid_r`), the folders the service manager names (`systemd-path`, `pkg-config`), the search path for programs, and `std::env::temp_dir` (Node `os.tmpdir()`). When the system cannot answer, the program says so and stops; it never guesses a path. The paths Bugaboo owns are relative to its settings file or its data folder, and the tools' to the repository. Tests may use such paths as data for pure functions. Enforced by: review until LINT-PATHS (M5 WP-11) is active.
- **No dead code:** no commented-out code, unused exports or unused dependencies.
- **No orphan TODOs:** every `TODO` must reference an issue (`TODO(BUG-123): …`). Enforced by: LINT-HYGIENE.
- **Feature traceability:** tests name the feature ID they cover (e.g. `sec_06_…`).
- **Commits:** follow Conventional Commits (`feat(qry): …`, `fix(sec): …`), and make each commit a single logical change. Enforced by: GIT-COMMIT-MSG. A commit an agent wrote ends with exactly one attribution line, `Co-Authored-By: Claude <model> <noreply@anthropic.com>`, and nothing after it, even when a session asks for a `Claude-Session:` line (review: the commit-message check does not read trailers). The commit author stays as the maintainer set it (review). Never amend or rewrite a commit that is already pushed; fix it with a new commit (review).
- **No private information in Git.** Nothing private about the maintainer may be committed, in a file (its name or its text, also a binary one), a branch or tag name, a commit message or a tag message, or in what a commit adds even if a later commit removes it: no user-profile path with an account name (write `%USERPROFILE%` or `~`), no Claude session ID or link, no session trailer in a commit message, no link to a published page (`claude.ai/artifact/…`; keep the list of them outside the repository), and never the account name or computer name of a machine or a word of the maintainer's private list (a file outside the repository). Test data uses made-up names. The check names the file and line, never the word. Claude Code adds a session line to every memory note; `cargo xtask hook post-edit` removes it. Enforced by: SEC-PRIVATE.

## 3. Rust

- Latest stable toolchain, edition 2024; `rustfmt` defaults.
- `#![forbid(unsafe_code)]` in every crate except where an ADR approves an exception.
- Clippy with warnings treated as errors, plus selected `pedantic` lints.
- No `unwrap()`, `expect()`, `panic!`, `dbg!` or `println!` outside tests and CLI output. Use
  structured `tracing` for logs. Enforced by: SEC-PATTERNS (`unwrap`, `expect`, `panic!`, `todo!`,
  `unimplemented!`, `dbg!`), LINT-RUST (`println!`).
- Use newtypes for IDs and units (`IssueId`, `Minutes`), never bare `i64` or `String`.
- **Authorization is in the type system:** data can only leave the domain layer wrapped in
  a type that proves the policy engine checked it. Code that bypasses the check must not
  compile. (Details in the architecture doc.) Enforced by: ARCH-AUTHZ-ONLY.
- SQL only through the query builder or compile-time checked queries. Never build SQL by
  string formatting.

## 4. TypeScript / React

- `strict` plus `noUncheckedIndexedAccess` and `exactOptionalPropertyTypes`.
- No `any`, no non-null `!`, no `@ts-ignore`. Use `@ts-expect-error` only with a reason.
- A secret scan hit on a fake test credential is marked on that line with `// gitleaks:allow -- fake test credential, SEC-16`: the marker, and a reason that names a section or an issue. META-SUPPRESS counts it in every tracked text file, like any suppression; never word-join or encode a credential to dodge the scan.
- API types come **only** from the generated client. Never hand-write a response type.
- Server data is held in one place: the server-state cache. Components never keep their own
  copy of server data.
- No business rules in the frontend beyond the generated validation schema. The server
  decides.
- Function components and hooks only. Named exports. Feature-folder structure.
- Formatted by **oxfmt**; linted by **oxlint** with type-aware **tsgolint** rules; type-checked by `tsc` (TypeScript 7).
- **No hard-coded user-facing text:** JSX text, `aria-*`, `title`, `alt` and `placeholder`
  values must come from the message catalog. Enforced by: TXT-LITERAL.
- **No hard-coded styles:** no hex/rgb colors, pixel magic numbers, durations, weights, line
  heights, layers or opacities. Only design tokens. Enforced by: STY-TOKENS, LINT-CSS.
- **All theming lives in `design/theme.kdl`** (ADR-0026): values are tokens, rules are `part`
  rules that use tokens only. Components carry no style values and no class names: no `style=`,
  no `className`, no CSS file but the one that `cargo xtask gen theme` writes. A component emits
  `data-part`, `data-variant` and `data-state` (`partOf`), plus React Aria's own `data-*`
  attributes. Enforced by: THEME-ONLY.
- **One exception: the size a person drags.** A window follows the pointer, so its size is two variables of the theme file that the code sets: only `element.style.setProperty(variableOf("…"), value)`, only in `web/packages/ui/src/`, where `variableOf` (defined once, in `variables.ts`) takes only a token that the theme file marks `run-time="true"` (today `window-width` and `window-height`). A color or a spacing is never set from code, and a second `variableOf` is a finding. Enforced by: THEME-ONLY.
- No `eval`, `new Function`, `document.write`, a timer given a string, or inline event-handler
  strings (these would break the CSP). Enforced by: SEC-PATTERNS. Setting HTML from a string is
  the next rule.
- **Text written by people is rendered by `bb-markdown` and nothing else** (SEC-13, ADR-0011). No
  other crate depends on a Markdown or HTML library (`pulldown-cmark`, `ammonia`, …), the browser
  has no second renderer (no `marked`, `markdown-it`, `DOMPurify`, …), and HTML is set from a
  string in one file only, the Trusted Types policy of `MarkdownView` (`innerHTML`,
  `dangerouslySetInnerHTML`, `insertAdjacentHTML`, `DOMParser` and the like are written nowhere
  else), and `MarkdownView` takes only `SafeHtml`, which only the WebAssembly renderer and the API
  client make. The name of that policy is written once, in `bb-api-words`: the content security
  policy takes it from there, and the browser from the generated client. The sanitizer corpus
  (`crates/bb-markdown/tests/corpus`) is rendered by the server's build and by the WebAssembly
  build, with the same result. Enforced by: SEC-RICHTEXT.

## 5. Dependencies & documentation

- **Every new direct dependency** needs a register entry in `docs/third-party/` (purpose, license, maintainer health, alternatives considered) in the same change. Prefer the standard library and existing dependencies.
- **Licenses:** only licenses on the allow list may end up in the shipped binary. The list lives in `deny.toml` and is mirrored for npm by the same check.
- **Architecture docs change with the code:** new module, new external interface, new dependency or a changed boundary → update the architecture doc or add a decision record in the same change.
- **Personal data:** every new stored field is marked as personal data or not, in the data model. Unmarked fields fail CI.

## 6. Performance & lean payloads

- Ask the API for the fields a view needs and nothing more. Never fetch everything and
  filter on the client.
- Paginate every list endpoint. Virtualize every long list in the UI.
- Load heavy features (timeline, editors, search-language WebAssembly) only when first opened.
- New dependencies need a justification covering bundle size, maintenance status, license
  and security record.
- Performance budgets (PERF-01…06) are CI gates, not aspirations. The tests that count statements for a list of five and a list of forty rows cannot be deleted or switched off unseen. Enforced by: PERF-N1.

---

## 7. Automated gates (hooks)

Every check is listed **once**, in [`tools/checks/catalog.toml`](../tools/checks/catalog.toml), with its ID, the rule it enforces, the tool, and when it runs. One cross-platform runner (`cargo xtask check …`, the same on Windows, Linux and WSL) implements them all. It is called from three places, which never call tools directly:

1. **Claude Code hooks** (`.claude/settings.json`): after an agent edits a file, the checks for that file run and errors go straight back to the agent.
2. **Git hooks** (lefthook): fast checks on staged files at commit, the wider set on push, the commit-message check.
3. **CI**: everything, plus nightly benchmarks and release-only checks (SBOM, notices). While the repository is local, CI is `cargo xtask ci` on the development PC (Windows and WSL); GitHub Actions runs the same command once the repository moves to GitHub.

**Tools:** rustfmt and clippy for Rust; oxfmt and oxlint with type-aware tsgolint (built on TypeScript 7) for TypeScript; stylelint for CSS; taplo, rumdl, typos, lychee and actionlint for the rest; jscpd for copied blocks of Rust and TypeScript (LINT-DRY: a block of 100 tokens and 10 lines is the size, in `.jscpd.json`, and a copy that stands for now is named with the fingerprint of its block and its follow-up in `tools/checks/dry-exemptions.toml`); ast-grep for pattern rules across languages.

**Checks for the checks** (META) make sure each check is implemented and tested, every file type is covered (a new kind of file needs a new check), new modules are declared, every rule names its check, and suppressions don't grow unnoticed. See [architecture §10](architecture/10-checks.md).

**Zero warnings, zero errors.** Every formatter and linter reports 0 warnings and 0 errors; a warning counts as an error. Enforced by: the FMT and LINT checks (FMT-RUST, LINT-RUST, LINT-TS and the rest of the groups).

- **Never skip the hooks** (`--no-verify`). `git push --force-with-lease` runs the push checks too. When a hook fails, fix the cause and commit again. CI runs the same checks, so nothing skipped locally reaches `main` (review).
- **Never switch off a check or a lint.** Fix the cause, not the symptom: no `#[allow]` to silence a real problem, no `// oxlint-disable` to make CI green, no skipped tests (review).
- **If a rule is truly wrong for one place,** use the narrowest suppression, with a reason that names an issue or a doc section, for example `#[allow(clippy::print_stdout, reason = "CLI output, coding-standards §3")]`. Adding one needs the lead's approval in review. Enforced by: META-SUPPRESS.
- **External tools** come only from `tools/dev/tools.toml` (pinned versions) and are installed by `cargo xtask setup` into `.tools/`, which also installs the Git hooks (ADR-0014). Run it once in every fresh or reset work folder (review).
