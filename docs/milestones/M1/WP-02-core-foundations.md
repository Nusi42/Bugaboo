# WP-02 — Core foundations

**Wave 1 · Folder / branch:** `bugaboo.win2` / `WIN2`
**Features:** PRIV-02, UX-07 (Rust side), groundwork for SEC-15; ADR-0001, ADR-0007, ADR-0015, ADR-0018, ADR-0021, ADR-0022; follow-up M0-F01.

## Goal

The shared vocabulary of the server, defined once: IDs, error codes, personal-data marks, domain events, the settings registry and message keys — in `bb-core` and `bb-macros`, with no I/O.

## Owns

`crates/bb-core/**`, `crates/bb-macros/**`, the Rust output of `cargo xtask gen messages` (extend WP-02/M0's generator in `xtask/src/gen/`), checks `xtask/src/checks/data_marks.rs`, `arch_errors.rs`, `arch_settings.rs` (pattern part).

## Tasks

1. **IDs:** newtype IDs over UUIDv7 (`uuid` with `v7`), `Display`/`FromStr`, serde; readable issue IDs come in M3. No raw `Uuid` in public signatures.
2. **Error codes (SSOT):** one `ErrorCode` enum with a stable string code (`auth.wrong_credentials`, …), the HTTP status it maps to, and its message key. Plain-language rule: safety errors never reveal hidden things (glossary, UX §9). A problem+json body type lives here (no HTTP crate: plain data).
3. **Personal-data marks (ADR-0015):** a derive macro (`#[derive(Entity)]`) that requires every field to carry `#[personal(category = "…", basis = "…")]` or `#[not_personal]`; unmarked fields are a compile error. The macro registers each entity's fields in a generated personal-data map (a `const` or an inventory-free registry built by an `entities!` list) that `cargo xtask gen privacy` turns into `docs/privacy/personal-data.md` (generated, covered by DOC-GENERATED).
4. **Domain events (ADR-0007):** event envelope (id, time, actor, channel `ui|api|mcp|assistant|system`, entity, changes before → after, reason). Actor and changes carry personal-data marks.
5. **Settings registry (ADR-0018):** a `settings!` declaration (key, type, default, levels organization/project/person, who may change it, help message key, lockable) and a registry listing all settings; storage comes in WP-05.
6. **Messages in Rust (UX-07, M0-F01):** `cargo xtask gen messages` also writes Rust constants/functions for message keys (with typed placeholders); move the server and xtask user-facing strings from M0 into `messages/en.json`. TXT-CATALOG covers Rust uses too.
7. **Checks** (catalogue entries active, bad/good examples): DATA-MARKS (macro compile-fail test plus a report of marked/unmarked counts), ARCH-ERRORS (every `ErrorCode` has a message key; no error strings in API responses — the API part completes in WP-06), ARCH-SETTINGS (pattern part: settings only through the registry; storage part in WP-05).
8. **Register** every new crate.

## Acceptance

- `cargo test -p bb-core -p bb-macros` incl. compile-fail tests (trybuild or equivalent) for unmarked fields.
- `cargo xtask gen messages` produces TS and Rust outputs; DOC-GENERATED green.
- All checks green, Clippy for Windows and Linux 0 warnings.
