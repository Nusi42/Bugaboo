# ADR-0005: Search language: one Rust crate, also in the browser

Status: **Accepted** · 2026-10-04

## Context

People need simple filter buttons and an advanced typed search, converting exactly into each other, with the same behavior in the browser and on the server, and without copying another vendor's language.

## Decision

Write **`bb-searchlang`**: lexer, parser, syntax tree, canonical printer, checker and suggestions, with no I/O. Simple search edits the syntax tree; Advanced search edits text that parses into the same tree. Compile the crate to **WebAssembly** (`bb-wasm`) for instant highlighting, suggestions and error messages in the browser. The server compiles the tree to SQL in `bb-store`.

## Consequences

- One grammar (SSOT). Property test: `parse(print(tree)) == tree`.
- WebAssembly adds download size; it's loaded only when search is first used, within its own WebAssembly budget in `tools/checks/perf-bundle.toml` (amendment M5, decision 7).

## Alternatives considered

- **A grammar written twice (Rust and TypeScript):** drifts. Rejected.
- **Parser generators (pest, lalrpop):** fine, but hand-written recursive descent gives better plain-language error messages. Decision left to the milestone, as long as the crate stays I/O-free.

## Amendment M5: the words, the limits and what crosses the wire

The M5 proposals (M5-Q7, Q10) were accepted. Decided:

1. **The parser is written by hand** (recursive descent), for plain-language messages and a small WebAssembly module. No parser generator.
2. **English words** for the language (`and`, `or`, `not`, `is`, `is not`, `contains`, `in last`, `sort by` …). A field is written as on screen, as one word in lower camel case (`assignedTo`, `statusGroup`). The word of each built-in field is defined once, next to the field; an own field gets its word from its name by one rule, and a name that cannot be written as one word is written in quotes. The search text is the same in every language of the app, so a shared search works for everyone.
3. **The language holds filters and the order** (`sort by`). How a search is shown (columns, widths, spacing, grouping) is not part of it; a saved search stores that beside the text.
4. **The canonical text is what travels:** the address, the API and a saved search carry the text the printer writes; the tree never crosses the wire. The server parses and checks every search again and never relies on the browser's check.
5. **The checker is given the fields the person may read** (the *readable fields*). A field outside that set is unknown: the same message, the same error code and no difference in time to a field that does not exist (QRY-05).
6. **Limits of the language** (the parser and the checker enforce them): at most 50 filters (a list `in (a, b)` counts as many as it has values), `within N steps` at most 5 (M5-Q10), at most 5 keys in `sort by`, and a text of at most 4 096 bytes (a fix of the review of M5 WP-02: no work is done for a longer text). The time, page and rate limits are the server's (SEC-17, M5 WP-04).
7. **A module of its own** (the lead): the search language is a WebAssembly module of its own, fetched when a search bar first opens, with its own budget in `tools/checks/perf-bundle.toml` (set by M5 WP-02 from its measurement, at most 100 KB with Brotli). The shared budget of 150 KB was planned for Markdown and the search language; the wording of changes and notifications (M4 WP-13) took it, the module is at 145.6 KB, and a list should not fetch Markdown, nor an editor the search language. Each module keeps a budget of its own.
8. **How the module is made** (M5 WP-02): the bindings are a crate of their own, `bb-wasm-search`, loaded by the package `@bugaboo/wasm-search`; `bb-wasm` keeps Markdown and the wording. The browser gives the tree and the readable fields as JSON, which the module reads with a reader that is **generated from the Rust types** (`cargo xtask gen search-reader`, held to the types by DOC-GENERATED): deriving `Deserialize` made the module bigger than its budget (116 KB with Brotli against 100 KB at most), and a reader written by hand would be a second definition of the tree. A tree that is read is held to the rules of a tree (a day of the calendar, an Issue ID by the rules of `bb-core`, a number in its shortest form, `and` and `or` of at least two items with none of their own kind inside; how deep a tree may be is the parser's rule (`MAX_DEPTH`), and a tree that is too deep has no text that reads back), and `print` of the module only gives a text that the language reads back as the same tree. The reader is behind the feature `browser` of `bb-searchlang`, which only `bb-wasm-search` turns on: the server never reads a tree from outside. `bb-searchlang` depends on `bb-core` for the rules of an Issue ID and of a short name, so the browser and the server read an ID the same way (architecture §4, rule 1). Places in the answers are bytes of the UTF-8 text; the package turns them into places in the string. Measured at 88.9 KB with Brotli at the server's quality; the line in `tools/checks/perf-bundle.toml` is 90 KB.

Enforced by: TEST-SEARCH-ROUNDTRIP (decisions 1 to 4, the printer and the parser), the tests of M5 WP-02 (decisions 5 and 6), SEC-MATRIX with the `search` entry point (decision 5 on the server), PERF-BUNDLE (decision 7).

9. **Values are written as the person sees them** (the lead, from the review of M5 WP-04): a status, an issue type and a choice of an own field are written by their name as shown on screen (`status is "In progress"`, `type is Bug`), never by a stored key or ID; two names are the same by the one rule that keeps names apart everywhere in the app (`bb_core::names::same_name`: case and repeated spaces do not count, accents do), so the checker, the choice lists and the server compare alike. The choices of built-in fields (*Priority*, *Status group*) are words of the language like the field words: one English word each, defined once next to the choice and the same in every language of the app, never the text of the message catalog. The server turns a name into the things of that name that the person may see, at the moment the search runs; a name the person cannot see is an unknown value, with the same message as a name that does not exist. A rename makes the old name unknown in a saved search, and the checker says so; nothing silently matches something else. Each word of the language (built-in fields, the words of *yes* and *no*, the operators each kind of field allows) is defined once, next to the field, and a word belongs to one field for everyone: an own field never takes the word of a built-in field, also not of one the person may not read or that cannot be searched yet.

## Enforced by

ARCH-PURE, ARCH-WASM, TEST-SEARCH-ROUNDTRIP (see `tools/checks/catalog.toml`).
