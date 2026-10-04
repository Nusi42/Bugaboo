# M1 follow-ups

Points found in the M1 reviews that were accepted for later. Each names where it will be done. The IDs (`M1-F01` …) are what suppressions and TODOs refer to. WP-04 may add its own rows; the lead merges the lists.

| ID | What | Why later | Planned in |
|----|------|-----------|------------|
| M1-F01 | Server strings (M0-F01) are not yet catalog messages; WP-02 delivers the Rust message API in `bb_core::message` | Moving them touches code that WP-04 changes; the lead switches the server over at integration | integration of WP-02 and WP-04 |
| M1-F02 | ARCH-ERRORS finds error answers built from a literal text only in the form `(StatusCode::X, "text")` | The API skeleton does not exist yet | WP-06 |
| M1-F03 | ARCH-SETTINGS has no storage part (nothing may read or write setting values around the registry) | Storage comes with the store | WP-05 |
| M1-F04 | ARCH-WASM only builds `bb-core`, `bb-searchlang` and `bb-markdown` for WebAssembly; there is no size budget yet (PERF-04) | `bb-wasm` and `wasm-opt` come with the search language editor | M4 |
| M1-F05 | `#[not_personal]` cannot be used on text; a text field that is known to be harmless needs a newtype with an explicit `NotPersonal` | No such field exists yet | when the first one appears |
| M1-F06 | The generated TypeScript messages file holds the `error.*` and `setting.*` keys, so they ship to the browser | A few hundred bytes; filtering by area needs a second catalog file | M2 (app shell) |
| M1-F07 | `IdSource` has no real implementation; the server context (clock and secure random source) supplies it | Needs `bb-app`'s context | WP-05 |
| M1-F08 | `Person` in `bb-core` is a minimal entity (id, name, e-mail, created) | The store and sign-in decide the final fields | WP-05, WP-07 |
