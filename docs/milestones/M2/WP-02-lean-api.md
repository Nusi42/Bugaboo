# WP-02 — Lean answers: `fields`, cursor paging, ETags, compression, bundle budget

**Wave 1 · Folder / branch:** `bugaboo.win2` / `WIN2`
**Features:** PERF-02, PERF-03, PERF-04; ADR-0008, ADR-0009.

## Goal

Every list and read in the API follows one set of conventions, written once in `bb-api`, so later packages only declare what a route returns: the fields the screen asks for, pages with a cursor, ETags with `304`, compression where it is safe, and a first page of the browser app that stays under its budget.

## Owns

`crates/bb-api/src/{lean,etag,paging}*` (new), the compression predicate in `crates/bb-api/src/router.rs`, the invitations list as the first user of the helpers (`crates/bb-api/src/api/auth_people.rs`, list route only), `xtask/src/checks/{perf_bundle,arch_lean}.rs`, `tools/checks/fixtures/{PERF-BUNDLE,ARCH-LEAN}/`, `web/app/vite.config.*` (manifest output only), the section *Performance* and the line *Compression* of `docs/architecture/07-cross-cutting.md`.

## Tasks

1. **`?fields=`** (PERF-02): one helper that parses the list of fields, refuses unknown names with `request.unknown_field` (a field the person may not read must look the same as one that does not exist, SEC-06), and drops everything else from each item. The answer schema lists the fields that can be asked for; the default is a small set named by the route, never everything. It works on any serializable view, so `Authorized<T>` (WP-05) plugs in by giving its readable field names.
2. **Cursor paging:** an opaque cursor (sort key and ID, base64url, checked when read: a changed cursor answers `request.bad_cursor`), `limit` with a maximum of 100, `next_cursor` in every page. The invitations list moves to the helpers.
3. **References, not embedded objects:** a list item names related things by ID (people by `PersonRef` from WP-05); embedding needs an `include` parameter that the route declares. Written in architecture §7 with *review* (whether a property is "another resource" is a judgment).
4. **ETags and `304`** (PERF-03): a weak ETag made from a hash of the final answer body (after masking, so two people with different rights never share one), `If-None-Match` answers `304`; `Cache-Control: private, no-cache` on API answers, `no-store` on answers that hand out a secret. Static assets keep their content hash and `immutable`; test both.
5. **Compression** (PERF-03) — **done.** Decision M2-Q2 (maintainer decision): compress JSON answers bigger than 1 KiB, but never an answer that hands out a secret (a new access key, backup codes, setting up a code app, an invitation link) or one that sets a cookie. No separate list of routes: the compression skips every answer that carries the `SecretAnswer` mark of the cache rules (a field marked `x-secret` in the API description, or any answer that sets a cookie) or a `Set-Cookie` header, so the one list is the marks in the API description, and a test walks it. The threshold is `JSON_COMPRESS_ABOVE_BYTES` in `router.rs`. Files of the app keep their rules. The *Compression* line of the architecture and the BREACH row of the threat table are updated; `sec_11_json_answers_are_not_compressed` became `sec_11_json_answers_are_compressed_above_1_kib_only`. Brotli and gzip are negotiated; zstd is not built (it would need a C library, a new dependency).
6. **PERF-BUNDLE** (PERF-04): `cargo xtask check only PERF-BUNDLE` reads the Vite manifest, adds up the JavaScript the first route loads (entry and its static imports), compresses it with Brotli and fails above 250 KB; the budget is defined once (in the check's settings, not in the catalogue text); WebAssembly joins with M1-F10 (M4). Bad and good fixtures (a fake manifest over and under the budget).
7. **ARCH-LEAN extension:** every `GET` in the API description documents the `ETag` header and the `304` answer, and every list documents `include` only when it embeds; a fixture for each.

## Acceptance tests (write first)

- `perf_02_a_list_answers_only_the_fields_asked_for` and `…_refuses_an_unknown_field_like_a_hidden_one` (same code, same message).
- `perf_02_paging_walks_every_item_once_and_refuses_a_changed_cursor` (property test over random page sizes).
- `perf_03_every_get_answers_an_etag_and_304_on_a_match`, generated over all `GET` routes of the API description.
- `perf_03_answers_with_a_secret_are_not_stored_or_compressed`, `perf_03_no_route_that_hands_out_a_secret_is_compressed` (every route of the API description, with a big answer) and `sec_11_json_answers_are_compressed_above_1_kib_only`.
- PERF-BUNDLE fails on the bad fixture and passes on the good one and on the real app.

## Checks

PERF-BUNDLE switched on (catalogue `status = "active"`, implementation, fixtures; the rule is PERF-04 and ADR-0009). ARCH-LEAN extended (its fixtures grow).

## Depends on

M1. M2-Q2 for task 5 only (answered, see task 5).
