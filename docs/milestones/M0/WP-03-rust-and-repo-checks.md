# WP-03 — Rust and repository checks, tool installer

**Folder / branch:** `<work root>/bugaboo.win3` / `WIN3`
**Features:** COMP-02 (register check), groundwork for SEC-18; ADR-0014, ADR-0020.

## Goal
`cargo xtask setup` installs every pinned tool, and the Rust, repository-wide and dependency checks run with 0 warnings on the whole repository, including all existing docs.

## Owns
`xtask/src/setup.rs`, `xtask/src/tools.rs`, configs `typos.toml`, `taplo.toml`, `.rumdl.toml`, `.gitleaks.toml`, `lychee.toml`, `.editorconfig`, and these checks in `xtask/src/checks/`: `fmt_rust.rs`, `fmt_toml.rs`, `fmt_md.rs`, `lint_rust.rs`, `lint_spell.rs`, `lint_links.rs`, `lint_ci.rs`, `sec_secrets.rs`, `git_commit_msg.rs`, `git_files.rs`, `arch_deps.rs`, `arch_no_unsafe.rs`, `dep_register.rs`, `dep_licenses.rs`. You may fix wording, typos and formatting in any file under `docs/` and `userDocs/` so the new checks pass (content changes only where a rule demands it).

## Tasks
1. **`cargo xtask setup`**: reads `tools/dev/tools.toml`; installs `kind = "cargo"` tools with `cargo install --locked --root .tools <crate> --version <v>` (skips tools already at the right version); for `kind = "download"`, downloads the archive for this platform, **verifies the sha256** before unpacking into `.tools/bin`; also runs `npm ci` if `package.json` exists. The checks find tools only in `.tools/bin` (and `node_modules/.bin` for npm tools). A missing tool is a failing finding that says "run `cargo xtask setup`" — never a silent skip.
2. **Checks** (catalogue entries set to active, each with bad and good examples):
   - FMT-RUST (`cargo fmt --check`), LINT-RUST (`cargo clippy --workspace --all-targets -- -D warnings`), FMT-TOML (taplo), FMT-MD (rumdl), LINT-SPELL (typos), LINT-LINKS (lychee, offline, repo links only), LINT-CI (actionlint; passes when there are no workflow files), SEC-SECRETS (gitleaks on staged files / whole history for `all`).
   - GIT-COMMIT-MSG (Conventional Commits: `type(scope)?: subject`, types feat, fix, docs, refactor, test, build, ci, chore, perf, style; subject ≤ 72 characters). Add a `commit-msg` subcommand to the runner if needed (`cargo xtask check commit-msg <file>`).
   - GIT-FILES (no file over 1 MB unless it matches an allow list in the check; no CRLF in text files; respects `.gitattributes`).
   - ARCH-DEPS: `cargo metadata` → compare workspace crate dependencies with the allowed edges of architecture §4 (encode the allowed graph once, in the check, with a comment pointing to the doc).
   - ARCH-NO-UNSAFE: every crate root has `#![forbid(unsafe_code)]` and no crate overrides `unsafe_code` in `[lints]`.
   - DEP-REGISTER: direct dependencies of every `Cargo.toml` and `package.json` ↔ rows in `docs/third-party/README.md`, both ways.
   - DEP-LICENSES: `cargo deny check licenses bans sources` with `deny.toml`; for npm, read licenses from `node_modules/*/package.json` (lockfile-listed packages) and apply the same allow list (parse it from `deny.toml` — one source).
3. **Configure, don't disable:** set the tools up so they run cleanly on the repository. For rumdl, keep the useful rules; long table rows and long URLs are fine, so use the rule options (not a global off switch) where possible, and write a one-line reason next to every changed default. Fix real problems in the docs instead.
4. **Register** every new crate in `docs/third-party/README.md`.

## Acceptance
- On a fresh clone: `cargo xtask setup` succeeds; running it again does nothing; a wrong checksum stops it with a clear message.
- `cargo xtask check all` passes with all checks of this package active; each check's bad example fails.
- `cargo fmt --check`, Clippy with `-D warnings`, `cargo test --workspace`: 0 warnings, 0 failures.
