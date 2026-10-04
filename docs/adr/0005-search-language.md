# ADR-0005: Search language: one Rust crate, also in the browser

Status: **Accepted** · 2026-10-04

## Context

People need simple filter buttons and an advanced typed search, converting exactly into each other, with the same behavior in the browser and on the server, and without copying another vendor's language.

## Decision

Write **`bb-searchlang`**: lexer, parser, syntax tree, canonical printer, checker and suggestions, with no I/O. Simple search edits the syntax tree; Advanced search edits text that parses into the same tree. Compile the crate to **WebAssembly** (`bb-wasm`) for instant highlighting, suggestions and error messages in the browser. The server compiles the tree to SQL in `bb-store`.

## Consequences

- One grammar (SSOT). Property test: `parse(print(tree)) == tree`.
- WebAssembly adds download size; it's loaded only when search is first used, with a 150 KB budget shared with the Markdown renderer.

## Alternatives considered

- **A grammar written twice (Rust and TypeScript):** drifts. Rejected.
- **Parser generators (pest, lalrpop):** fine, but hand-written recursive descent gives better plain-language error messages. Decision left to the milestone, as long as the crate stays I/O-free.

## Enforced by

ARCH-PURE, ARCH-WASM, TEST-SEARCH-ROUNDTRIP (see `tools/checks/catalog.toml`).
