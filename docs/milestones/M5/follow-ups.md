# M5 follow-ups

Points found while building M5 that were accepted for later. Each names where it will be done. The IDs (`M5-F01` …) are what the review refers to. WP-02 uses F10 and up, WP-03 F20 and up, WP-04 F40 and up, WP-05 F50 and up, WP-06 F60 and up, WP-07 F70 and up, WP-08 F80 and up, WP-09 F90 and up, WP-10 F100 and up, WP-11 F110 and up.

| ID | What | Why later | Planned in |
|----|------|-----------|------------|
| M5-F10 | The test of reading a short name in `bb-core` (`ShortName: FromStr`) names no feature ID | Found after the merge of WP-02; the test itself is right | The next package that changes `bb-core` |
| M5-F20 | The notification count of the speed project was over its budget on PostgreSQL (100.5 ms) before WP-03 read the rights of all projects of a page at once (34.8 ms) | Fixed in WP-03; recorded so the nightly benchmark watches it | The nightly benchmark (WP-03) |
| M5-F21 | `held.roles().iter().map(\|role\| role.id).collect()` is written three times (`layouts/effective.rs`, `layouts/read.rs`, `workflows/status.rs`); one `Held::role_ids()` next to `Held::roles()` in `bb-policy` | Older code, found in the fourth gate check of WP-03 | The next package that changes these files, or WP-04 |
| M5-F22 | The doc of `Ctx::leveled` explains why its later read is acceptable by its only use today; it should also say "use it only for that; what decides what is shown reads `shown_with_levels`" | Only a doc line | With F21 |
| M5-F60 | The places a saved search is shared with come in the answer of one search, bounded by 100, and not as a paged list | At most 100 rows of a search that only its maker reads; a paged list is an addition after the freeze | When a screen needs more than the *Share* window shows |
| M5-F40 | Text is sorted by its bytes (`COLLATE "C"` on PostgreSQL, the same order as SQLite), so *Zebra* stands before *apple*; people expect an order that ignores case and accents, the same on both databases | Parity of both databases first (ARCH-DB-PARITY); an order people expect needs a sort key made by one rule in Rust and stored beside the text | WP-08 (sorting by a column), or a package of its own |
| M5-F41 | A duration is written in the search language as a number of seconds (`estimate > 7200`) | The server's form came first; people must be able to write and see *2 hours* | WP-07 (the search bar), with the words of the language decided by the lead |
| M5-F42 | When WP-04 is merged: `bb-app/src/fields/values.rs` is under 800 lines and no longer exempt; drop its name from M4-F130 | Only true once WP-04 is in `main` | The lead, at the merge of WP-04 |
| M5-F61 | An organization-wide list of the shared saved searches, for whoever may change organization settings | Such a person deletes any search by its ID but has no list to find one by | M13, admin power tools |
