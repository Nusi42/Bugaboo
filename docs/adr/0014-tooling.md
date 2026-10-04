# ADR-0014: One check runner for hooks, agents and CI

Status: **Accepted** · 2026-10-04

## Context
Checks (formatting, linting, hard-coded text, tokens only, terminology, licenses, register, architecture docs, personal-data marks, size budgets) must behave the same on Windows, Linux and WSL, for people, AI agents and CI.

## Decision
- **`cargo xtask`** is the single entry point: `cargo xtask check staged`, `check all`, `gen …`, `release …`. It's Rust, so it runs wherever the code builds.
- **lefthook** runs `check staged` on commit and `check all` on push.
- **Claude Code hooks** (`.claude/settings.json`) run the relevant checks after each file edit, so agents get immediate feedback.
- **CI** runs `check all` plus slow checks (benchmarks, end-to-end, SBOM).
- **Rust:** `rustfmt` formats, `clippy` lints (warnings are errors, selected pedantic lints on).
- **TypeScript 7:** **oxfmt** formats; **oxlint** lints, with **oxlint-tsgolint** for type-aware rules. tsgolint is built on typescript-go, the TypeScript 7 compiler itself, so it understands TypeScript 7 exactly. `tsc` (TypeScript 7) does the type check.
- **CSS:** stylelint enforces "tokens only"; formatting by oxfmt (or Biome, if oxfmt doesn't cover CSS at setup time).
- **Other files:** taplo (TOML), rumdl (Markdown), typos (spelling), lychee (links), actionlint (CI files).
- **Pattern rules** across Rust and TypeScript (no literal UI text, no raw colors, no `unwrap` in production code, no SQL built with `format!`, `Authorized<T>` only in bb-policy) are **ast-grep** rules in `tools/rules/`.
- Every check is listed once in **`tools/checks/catalog.toml`** (ADR-0020).

## Consequences
- One list of checks (SSOT), visible in `xtask/src/checks/`.

## Alternatives considered
- **Shell scripts:** differ between Windows and Linux. Rejected.
- **ESLint + typescript-eslint:** typescript-eslint supports TypeScript only up to 6.0. Rejected.
- **Biome:** a good all-in-one tool, but its type-aware rules use its own inference instead of the TypeScript 7 compiler. Kept as fallback for formatting.

## Enforced by
META-WIRING, META-CATALOG, META-SPEED, FMT-*, LINT-* (see `tools/checks/catalog.toml`).
