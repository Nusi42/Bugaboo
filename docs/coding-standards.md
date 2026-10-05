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

## 2. General

- **Comments explain *why*, not *what*.** Names should make the *what* obvious.
- **Small units:** one responsibility per function and module. A file over ~400 lines
  needs a reason.
- **Errors are values:** domain errors are typed and handled. Never silently swallow one.
- **No dead code:** no commented-out code, unused exports or unused dependencies.
- **No orphan TODOs:** every `TODO` must reference an issue (`TODO(BUG-123): …`). Enforced by: LINT-HYGIENE.
- **Feature traceability:** tests name the feature ID they cover (e.g. `sec_06_…`).
- **Commits:** follow Conventional Commits (`feat(qry): …`, `fix(sec): …`), and make each commit a single logical change.

## 3. Rust

- Latest stable toolchain, edition 2024; `rustfmt` defaults.
- `#![forbid(unsafe_code)]` in every crate except where an ADR approves an exception.
- Clippy with warnings treated as errors, plus selected `pedantic` lints.
- No `unwrap()`, `expect()`, `panic!`, `dbg!` or `println!` outside tests. Use structured
  `tracing` for logs.
- Use newtypes for IDs and units (`IssueId`, `Minutes`), never bare `i64` or `String`.
- **Authorization is in the type system:** data can only leave the domain layer wrapped in
  a type that proves the policy engine checked it. Code that bypasses the check must not
  compile. (Details in the architecture doc.) Enforced by: ARCH-AUTHZ-ONLY.
- SQL only through the query builder or compile-time checked queries. Never build SQL by
  string formatting.

## 4. TypeScript / React

- `strict` plus `noUncheckedIndexedAccess` and `exactOptionalPropertyTypes`.
- No `any`, no non-null `!`, no `@ts-ignore`. Use `@ts-expect-error` only with a reason.
- A secret scan hit on a fake test credential is marked with `gitleaks:allow` and a reason that names a section or an issue, on that line (`// gitleaks:allow -- fake test credential, SEC-16`). META-SUPPRESS counts it like any suppression; never word-join or encode a credential to dodge the scan.
- API types come **only** from the generated client. Never hand-write a response type.
- Server data is held in one place: the server-state cache. Components never keep their own
  copy of server data.
- No business rules in the frontend beyond the generated validation schema. The server
  decides.
- Function components and hooks only. Named exports. Feature-folder structure.
- Formatted by **oxfmt**; linted by **oxlint** with type-aware **tsgolint** rules; type-checked by `tsc` (TypeScript 7).
- **No hard-coded user-facing text:** JSX text, `aria-*`, `title`, `alt` and `placeholder`
  values must come from the message catalog. Enforced by: TXT-LITERAL.
- **No hard-coded styles:** no hex/rgb colors, pixel magic numbers or inline `style=` for
  theming. Only design tokens.
- No `dangerouslySetInnerHTML`, `eval`, `new Function`, or inline event-handler strings
  (these would break the CSP).

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
- Performance budgets (PERF-01…06) are CI gates, not aspirations.

---

## 7. Automated gates (hooks)

Every check is listed **once**, in [`tools/checks/catalog.toml`](../tools/checks/catalog.toml), with its ID, the rule it enforces, the tool, and when it runs. One cross-platform runner (`cargo xtask check …`, the same on Windows, Linux and WSL) implements them all. It is called from three places, which never call tools directly:

1. **Claude Code hooks** (`.claude/settings.json`): after an agent edits a file, the checks for that file run and errors go straight back to the agent.
2. **Git hooks** (lefthook): fast checks on staged files at commit, the wider set on push, the commit-message check.
3. **CI**: everything, plus nightly benchmarks and release-only checks (SBOM, notices). While the repository is local, CI is `cargo xtask ci` on the development PC (Windows and WSL); GitHub Actions runs the same command once the repository moves to GitHub.

**Tools:** rustfmt and clippy for Rust; oxfmt and oxlint with type-aware tsgolint (built on TypeScript 7) for TypeScript; stylelint for CSS; taplo, rumdl, typos, lychee and actionlint for the rest; ast-grep for pattern rules across languages.

**Checks for the checks** (META) make sure each check is implemented and tested, every file type is covered (a new kind of file needs a new check), new modules are declared, every rule names its check, and suppressions don't grow unnoticed. See [architecture §10](architecture/10-checks.md).

Hooks can be skipped locally with `--no-verify`, but CI runs the same checks, so nothing skipped locally reaches `main`.
