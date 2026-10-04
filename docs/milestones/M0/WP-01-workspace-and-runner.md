# WP-01 — Workspace and check runner (lead) · done

**Features:** groundwork for COMP-06 and every check (ADR-0014, ADR-0020, ADR-0024).

**Delivered:**

- Cargo workspace (`resolver = "3"`, edition 2024, Rust 1.97.1 via `rust-toolchain.toml`) with all 16 crates from architecture §4 as documented shells; `#![forbid(unsafe_code)]` in every crate root.
- Strict lints for every crate in `[workspace.lints]`: rustc `missing_docs`, `unreachable_pub`, `unused_qualifications`, `unused_crate_dependencies`, `rust_2018_idioms`; Clippy `all`, `pedantic`, `cargo`, plus restriction lints (`unwrap_used`, `expect_used`, `panic`, `todo`, `unimplemented`, `dbg_macro`, `print_stdout`, `print_stderr`, `allow_attributes_without_reason`, `undocumented_unsafe_blocks`). Tests may unwrap (`clippy.toml`).
- `rustfmt.toml`, `clippy.toml` (product names in `doc-valid-idents`), `deny.toml` (license allow list = architecture §1), `.cargo/config.toml` (`cargo xtask` alias, optional ignored `config.local.toml`).
- `xtask`: catalogue loader, `Check` trait and registry, runner for `check edit|staged|push|all|only`, `list`, plain-language output, the guard against building in the master.
- Active checks: **META-CATALOG**, **BUILD-LOCATION** (with `bad_example`/`good_example` tests).
- `tools/dev/tools.toml`: pinned external tools for `cargo xtask setup` (implemented in WP-03).
- `CLAUDE.md`: rules for agents.

**How a check is built** (pattern for all other packages)

1. Add `xtask/src/checks/<snake_id>.rs` with a struct implementing `Check` (`id()` returns the catalogue ID; `run()` returns findings, empty = pass).
2. Register it in `checks::registry()` (one added line).
3. Set its catalogue entry to `status = "active"`.
4. Add `bad_example` and `good_example` tests (or `tools/checks/fixtures/<ID>/bad|good/`).
5. Findings say what is wrong and how to fix it, in plain words; the runner prints the rule.

**Verified:** `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings` (0 warnings), `cargo test --workspace` (11 tests), `cargo xtask check all` (2/2 passed).
