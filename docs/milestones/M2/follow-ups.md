# M2 follow-ups

Points found while building M2 that were accepted for later. Each names where it will be done. Until Bugaboo tracks its own issues, this list is the issue list for M2; the IDs (`M2-F01` …) are what suppressions and TODOs refer to.

| ID | What | Why later | Planned in |
|----|------|-----------|------------|
| M2-F01 | `Fields::project` (the `?fields=` helper) cuts only the top-level fields of an item; a view that holds an object inside (a mask inside a field) is not cut further | Every view of M2 so far is flat; the policy engine masks inside views | M2 WP-05 (field masks and `Authorized<T>`) |
| M2-F02 | ARCH-LEAN's `include` rule looks only at the `items` of a paged list: it does not cover a capped list, a free-form object (an object without properties) or a schema whose name ends in `Ref` but holds more than IDs and names | No route has one of these yet, and a check for them would guess | When the first capped list embeds something, or a `Ref` schema grows beyond IDs and names |
| M2-F03 | The WebAssembly part of the first-page budget is not checked: PERF-BUNDLE measures JavaScript only | `bb-wasm` and `wasm-opt` do not exist before the search language editor (see M1-F10) | M4, with `bb-wasm` |
| M2-F04 | PERF-BUNDLE measures whatever `web/app/dist` exists on the computer: a stale build passes or fails for the code it was made from, not for the code in the working tree | `cargo xtask ci` builds the app first, so CI is exact; a local run after a code change without a build is not | When `check all` can say how old the build is, or builds it |
| M2-F05 | The start page and the other files of the app that are `no-cache` have no `ETag`, so a revalidation sends the whole file again | The start page is small and the files that matter are named by their hash | When the start page grows or a second kind of file needs revalidation |
| M2-F06 | A `401` for a session that has run out does not clear the session cookie and the browser cache (`Clear-Site-Data`); only signing out and signing out everywhere do | The person is not signed in anyway; clearing needs the answer to know that the cookie belonged to a session that ended | M2 WP-09 (access keys and sign-in by role), which touches the session answers |
