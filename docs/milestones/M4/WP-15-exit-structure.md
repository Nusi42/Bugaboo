# WP-15 — Exit review: structure, checks and docs

**Exit wave A · Folder / branch:** `bugaboo.win2` / `WIN2`
**Features:** WF-01…WF-06, ATT-01…ATT-07, NTF-01, NTF-02 (the code and checks behind them).
**Source:** [the M4 exit review](exit-review.md), section *M4 exit review: architecture*, all items except the API list (WP-16), the notifications reader and the attachment event (WP-12) and the web items (WP-14).

## Goal

The code of M4 has one definition of each rule, the checks catch the mistakes M4 made, and the docs say what the code does.

## Owns

`crates/bb-core` (name keys), `crates/bb-store/src/{db.rs,files.rs,handles.rs,issues.rs,migrations}` (migration 20 `m0020_mention_read_at`, which drops `mention.read_at`, M4-F83), `crates/bb-app/src/{jobs.rs,issues/issue_types.rs,tags/labels.rs (split),files/upload.rs,files/attachments.rs}`, the read-once caches, `xtask/src/checks/**`, `tools/checks/**`, `docs/architecture/**` (not 12-security), the ADRs named below.

## Tasks

1. **Must 4**: one `bb_core` name key and one "taken where they meet" helper for issue types, label groups, workflow statuses and the M3 places (fields, choices, kinds of relation, tags).
2. **Must 5 and 6 (M4-F44)**: one required-tests implementation that sees modules that are never declared, `#[ignore]` and `cfg`-off tests; TEST-WORKFLOWS and TEST-UPLOADS require every test that the security chapter or an ADR cites as proof, and the publish-move tests run on both databases.
3. **Must 8** and the docs items of *Should fix*: 05-runtime "Sorting", ADR-0031 ceiling 47, 12-security "M4 planned" (coordinate the last with WP-12), 04-building-blocks (bb-files and bb-notify sections, nothing planned that is built), the keys-never-hold lists from `NOT_FOR_KEYS`, ADR-0032 trashed files (M9, plus a follow-up), MiB, the lock wording of ADR-0031 and ADR-0027.
4. **Should fix, server**: a lock-order guard (a transaction refuses a lower lock class than one it holds, in debug and test builds) and the order named in 07-cross-cutting; one `lock_key(id)`; PERF-N1 switched on; `tags/labels.rs` split; one "may add or remove a file" rule; `reaches` from `bb_core::offer`; one editor `Where`; one way to handle a poisoned read-once cache; job kinds as a `words!` enum with `is_daily()`; dead code removed (`notifications_by_ids`, `set_field_retired`, the Rust field `Mention.read_at`, the stale migration comment); m0018's DDL without `format!` if sea-query can, otherwise a written reason (m0018 is pinned: do not change its checksum; if the fix would change it, write the reason instead).
5. **Checks that should exist**: 1 lock order, 2 required tests, 3 "Proven by" cells, 4 a file-size ceiling with named exemptions, 6 is WP-12's, 7 no raw query keys and e2e constants from generated files. Check 5 (API) is WP-16's.
6. **Follow-ups section**: record each as M4-F130 and up; the testing-features item widens M4-F100.

## Depends on

`main` after WP-10. Works beside WP-12, WP-13, WP-14.
