# WP-02 — The search language

**Wave 1 · Folder / branch:** `bugaboo.win2` / `WIN2`
**Features:** QRY-01, QRY-03, QRY-04 (the language; the server's compiler is WP-04, the search bar WP-07). ADR-0005 with its amendment M5; architecture §7 "Search language", §4 `bb-searchlang`.

## Goal

One grammar, in Rust, that the server and the browser both use: a person types a search, sees the same suggestions and the same mistakes the server would find, and Simple search and Advanced search never disagree, because both edit one tree that prints to one canonical text.

## Owns

`crates/bb-searchlang/**`; the WebAssembly module of the search language (a second crate such as `bb-wasm-search`, or a second build of `bb-wasm` with only the search part: one way, chosen and written into the module map of architecture §4 and §5); its package in `web/packages/` and its generated files; the search keys of the message catalog (`search.*`); the "Search language" section of architecture §7 (the final grammar); the WebAssembly part of `tools/checks/perf-bundle.toml` and of PERF-BUNDLE (a budget per module); TEST-SEARCH-ROUNDTRIP in `tools/checks/catalog.toml`.

It does not touch `bb-store`, `bb-app`, `bb-api` or a screen.

## What is decided (ADR-0005, amendment M5)

- A hand-written recursive descent parser; no parser generator.
- English words; a field as on screen in one word, lower camel case (`assignedTo`, `statusGroup`); the word of each built-in field defined once, next to the field; an own field's word made from its name by one rule, quoted when the name is not one word. The same search text in every language of the app.
- The language holds filters and `sort by`. Columns, widths, spacing and grouping are not part of it.
- The canonical text is what the address, the API and a saved search carry. The tree never crosses the wire.
- The checker gets the fields the person may read (the **readable fields**, passed in by the caller). A hidden field is an unknown field: the same message, the same error code.
- At most 50 filters; `within N steps` at most 5. Both numbers are named constants in one place.
- Its own WebAssembly module with its own budget (at most 100 KB after Brotli at the server's quality, `tools/dev/compression.json`).

## Tasks

1. **The grammar.** Write the final grammar into architecture §7, starting from the sketch there and the words of QRY-01 and QRY-04:
   - `and`, `or`, `not`, brackets; `is`, `is not`, `<`, `>`, `<=`, `>=`, `in (…)`, `contains "…"`, `is empty`, `is not empty`;
   - values: words, quoted text, numbers, dates, issue IDs (both kinds, ADR-0021), `me`;
   - time words: `today`, `yesterday`, `this week`, `last week`, `this month`, `in last N days|weeks|months`; they stay relative in the tree (the server turns them into moments in the person's region zone, WP-04);
   - helpers: `member of "…"`, `within KEY`, `under KEY`, `blocks KEY`, `blocked by KEY`, `connected to KEY [within N steps]`;
   - `sort by field [direction], …` with plain directions (`highest first`, `lowest first`, `newest first`, `oldest first`, `A to Z`, `Z to A`), as QRY-01's example `sort by priority, highest first`.
   Every word is in the glossary or added to it in this package (ask the lead first).
2. **Lexer, parser and tree.** Positions (byte ranges) on every node, so the browser can mark a word. A parse never panics, on any input (fuzz it with proptest).
3. **The printer.** One canonical text per tree: one spelling of each word, one way to quote, the order of `sort by` kept, brackets only where they are needed. `parse(print(tree)) == tree` for every tree, and `print(parse(text))` is the canonical text of any accepted text.
4. **The checker.** Input: a tree and the readable fields (each field: its word, its kind of value, the operators it allows, its choices if it has a fixed list). Output: the tree with each value checked against its field, or mistakes. A mistake has a range, a message key with its arguments, and suggestions ("There is no field called 'prio'. Did you mean 'priority'?"). The checker enforces the two limits. The field that does not exist and the field the catalogue leaves out give the same mistake, and a test proves it with the same text, key and arguments.
5. **Suggestions.** Given a text and the place of the cursor: what may come next (fields, operators, values from the catalogue's choices, helper words). Values that only the server knows (people, groups, projects, issues) come back as a request of their kind ("ask for people starting with 'an'"), which WP-07 sends to the server.
6. **The browser module.** The bindings for parse, print, check and suggest; loaded only when a search bar first opens; its own budget line in `perf-bundle.toml` from your measurement, and PERF-BUNDLE checks each module against its own budget. Vectors generated from Rust (texts, canonical texts, mistakes) are run in the browser's unit tests, so both sides prove the same results.
7. **Messages.** Every mistake is a message key in the catalog, in plain words, with the field's word as an argument; no literal text in Rust or TypeScript.

## Acceptance tests (write first)

- TEST-SEARCH-ROUNDTRIP: proptest over generated trees, `parse(print(tree)) == tree`; and over generated texts, a parse never panics.
- The QRY-01 example and every QRY-04 helper parse, check and print back to their canonical text.
- `qry_05_a_hidden_field_and_an_unknown_field_give_the_same_mistake`.
- The limits: 50 filters pass, 51 fail; 5 steps pass, 6 fail, each with its message.
- Suggestions after `assi`, after `assignedTo `, after `priority is `, inside quotes.
- The generated vectors pass in the browser (Vitest, `npm test`).
- PERF-BUNDLE: the search module within its own budget, the old module within 150 KB.

## Checks

TEST-SEARCH-ROUNDTRIP planned → active ("Adding a rule", architecture §10, with a bad and a good example). PERF-BUNDLE grows to one budget per module. ARCH-PURE and ARCH-WASM keep `bb-searchlang` without I/O and buildable for WebAssembly.

## Depends on

`main` at the M5 plan. Nothing else in M5; it runs beside WP-03.
