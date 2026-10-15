# WP-14 — Exit review: structure fixes (one source for each fact)

**Wave 5 (exit fixes) · Folder / branch:** `bugaboo.win4` / `WIN4`
**Features:** ISS-17 (conflict answers), ISS-09 (background runs), ISS-02 (issue IDs); ADR-0017, ADR-0019, ADR-0021, ADR-0023.
**Source:** the M3 exit architecture and SSOT/DRY/ODR review, copied below.

## Goal

Every route answers a stale save the same structured way; a background run that fails ends; each fact (default priority, edit parts, history tabs, bind limit, "count the version up", first start, path IDs) is written once.

## Owns

`crates/bb-app/src/editing/{conflict.rs,records.rs}` (except the mask line WP-12 fixes), `crates/bb-app/src/{bulk/run.rs (job outcome only),jobs.rs,first start}`, `crates/bb-store/src/{db.rs,events.rs,relations.rs,issues.rs,fields.rs,jobs.rs,bulk.rs}`, `crates/bb-api/src/api.rs` and the path/null helpers in route files, `crates/bb-server/src/wiring.rs`, `crates/bb-sample/src/{generate.rs,issues_setup.rs,dump_issues.rs,relations.rs}`, `xtask/src/gen/api/web_constants.rs`, `xtask/src/checks/test_issue_ids.rs`, the Rust and e2e test helpers, `docs/architecture/**` and `docs/adr/0017*` (docs vs code). WP-12 owns `tags/**`'s security paths: the `.about()` calls in `tags/labels.rs` and `tags/manage.rs` are yours, keep the commits small.

## Tasks

1. All **must-fix** items 1–8 below, tests first (the table-driven conflict test over every versioned route; a dead bulk job ends the run).
2. **Should-fix, do:** the whole "Errors and checks" group; the whole "Docs vs code" group; "Tests/e2e support" (one helper each, `t("save.saved")`, query keys only in `query-keys.ts`, `FIRST_VERSION`, page-size constants, the long-text route table).
3. **Should-fix, record as follow-ups** (target M4, before the API freeze) instead of doing: "Glossary drift" (maintainer decision on code names), "Size and structure" (file splits, bool flags to enums, store ports taking domain types) — unless a file you already change can be split cheaply.
4. The **nits**.
5. New follow-ups from M3-F150 up.

## Checks

ARCH-* checks stay green; TEST-ISSUE-IDS requires the moved-issue test.

## Depends on

M3 WP-11.

## Findings (exit architecture review)

## Must-fix

1. Conflict answer differs between routes (ADR-0019 amendment "the conflict answer is structured"): tags/labels return bare `edit.version_changed` without `.about()` (`crates/bb-app/src/tags/labels.rs:256,336,486,553,620,669`; `tags/manage.rs:284,360,389,431`). For Field, RelationKind, Layout `Ctx::version_now` (`bb-app/src/editing/conflict.rs:170-230`) doesn't know the kind → `conflict.version` null, `fields` [] (callers `fields/manage.rs:538`, `fields/choices.rs`, `relations/kinds.rs:259`, `layouts/write.rs:435`; `shown_by_us` `conflict.rs:88-127` lists none). `openapi.rs:290-318` promises ConflictProblem for every versioned route. Fix: `.about(...)` on tag/label paths; one table kind → version lookup + shown members (Field, RelationKind, Layout, LabelGroup, Label, Tag); one table-driven HTTP test over every versioned route asserting `conflict.version` = current.
2. Failed background run stays "running" forever: `JobStore::fail_job` drops the `FailureOutcome` (`bb-store/src/bulk.rs:86-98`); `run_bulk_step` (`bb-app/src/bulk/run.rs:72-180`) marks Failed only on refusal/item error; store error → 5 retries → dead-letter while `bulk_run` stays Running and RunProgress never ends; same for `issues.renew-ids`. Refusals retried 5 times. `07-cross-cutting.md:358` says admins can requeue; `dead_jobs`/`requeue_dead_job` (`bb-store/src/jobs.rs:226,247`) have no caller. Fix: return outcome; on dead `bulk.step` end the run Failed + log; end state for ID renewal; refused job → dead-letter at once; add requeue (CLI or route) or correct the doc + follow-up.
3. New-issue default priority copied into the browser as logic: `bb-core/src/issue.rs:106` `Priority::NEW` vs `web/app/src/features/issues/new-issue-form.ts:25` `priority: "normal"` and line 143 omits it when equal. Fix: generate `newIssuePriority` (and `newIssueStatus`) in `xtask/src/gen/api/web_constants.rs`, or always send it.
4. Hand-mirrored enums: `web/app/src/features/history/history.ts:15` copies `HistoryTab` (`bb-core/src/history.rs:38`) — give it a named schema, add to `WORD_LISTS` (`web_constants.rs:153`). Edit parts: `"description"` typed 7× (`features/issues/long-text-words.ts:12`, `e2e/support/issues.ts:75,138`, `e2e/concurrent/locks.spec.ts:46`, `web/tooling/load/person.ts:42,144`), `"text"` in `features/history/CommentEditor.tsx:39` (copies `bb-core/src/comment.rs:22`) — generate `descriptionEditPart`, `commentEditPart` like `layoutEditPart`.
5. bb-api helpers twice: `null_or_value` in `api/fields.rs:251` and `tags.rs:399`; "path text not an ID → NotFound" 5× (`fields.rs:351` id_in_path = `people.rs:174` parse, `comments.rs:218` comment_id, `tags.rs:424` id_of, `projects.rs:242`, `field_permissions.rs:211`). Fix: one `pub(crate)` each in `api.rs`.
6. Bind limit stated four ways: `bb-store/src/db.rs:38` STATEMENT_ITEMS=500 (comment: SQLite allows 32,766); `events.rs:382` EVENTS_PER_STATEMENT=100 (comment: "older limit of 999"); `bb-app/src/tags/labels.rs:34` and `tags/rewrite.rs:26` CHUNK=200; `bb-sample/src/dump_issues.rs:32` CHUNK=400. Store splits `field_values` (`fields.rs:195`) but not `tags_of_issues_in`/`labels_of_issues_in` (`tags.rs:318,339`). Fix: one bind limit in db.rs, derive statement sizes; store splits tag/label reads; one app-side page constant; drop sample chunking.
7. "Count an issue's version up" ~8× with different rules: `bb-store/src/relations.rs:343` `count_issue_version_up` vs `:360` `count_up_issue_version` (one checks version, one not; tags call relations port `bb-app/src/tags/issue.rs:219`); `fields.rs:382` `touch_items`; `issues.rs` update_issue/move_issue/set_issue_deleted/set_issue_code (~445-566), some set UpdatedAt some not. Fix: one store helper (version+1 and UpdatedAt, optional version guard) and one clearly named port.
8. Startup wiring copied by every caller: `CommentRecords` plugged in from outside in `bb-server/src/wiring.rs:130`, `bb-sample/src/generate.rs:293`, `bb-server/src/tests/sign_in/rig.rs:215` (against ADR-0019 WP-06 amendment: bb-app-owned kinds are built in). First-start sequence (starting roles, issue types, relation kinds) in `wiring.rs:162-178`, `bb-sample/src/generate.rs:376` + `issues_setup.rs:86-90`, and the test rig. Fix: comment is a built-in kind in `editing/records.rs` and `conflict.rs`; one bb-app use case `first_start` used by all three.

## Should-fix

Errors and checks:

- Bulk matrix rows prove nothing: `bb-server/src/tests/matrix/examples.rs:383-384` expect 200 for every actor; add quirk like `SkipsWhatItMayNotChange` asserting the item's outcome against change-issues on target.
- Full store answers two codes: fields/choices `request.invalid` (`bb-app/src/fields/manage.rs:269`, `fields/choices.rs:154`) vs `limit.reached` elsewhere → LimitReached everywhere; widen doc `bb-core/src/error_code.rs:99-102`.
- Swallowed errors: `editing/conflict.rs:233-238`; `bb-api/src/headers.rs:101-108` (failed image-hosts read cached as empty); `bulk/judge.rs:103` `_ => SkipReason::NotValid` (make exhaustive); TS `features/bulk/undo.ts:45` unhandled rejection; `features/issues/use-delete-issue.ts:25-37` refused delete never shown; `shared/editing/version-watch.ts:86` `.catch(() => null)`.
- ID renewal has no idempotency test (ADR-0017), `issues/ids.rs:351`.
- TEST-ISSUE-IDS `xtask/src/checks/test_issue_ids.rs:22` REQUIRED lacks "old ID keeps working after a move" (promised `catalog.toml:464`, ADR-0021 amendment); add test on both databases.
- Job kind names: `editing.purge_drafts` (`bb-core/src/edit.rs:47`) vs `history.catch-up`, `issues.renew-ids` (`bb-app/src/jobs.rs:30,36`); move constant to `bb_app::jobs`, hyphens, migration rename waiting rows, naming rule in 07.
Docs vs code:
- `docs/adr/0017-background-jobs.md:11` schedules are settings vs fixed `DAILY` (`jobs.rs:42`).
- `docs/architecture/04-building-blocks.md`: :21 bb-jobs holds only the loop (needs own section); :165 bb-dev-words depends on bb-core; :173 xtask depends on bb-core; :176 three wiring crates; :129 M3 bb-app modules missing.
- `05-runtime.md:115` project is the one kind bb-app knows itself — outdated.
- `12-security.md:7,11,110` still "M3 planned".
- Field catalogue table `07-cross-cutting.md:229-246` hand-written next to generated tables → generate with `gen roles`.
Glossary drift (decide before API freeze; MAINTAINER/LEAD DECISION): `/issues/{issue}/sorting`, `IssueSorting*` (invented; screen says "Labels and tags"); "bulk" code name of Change many issues, "run" no entry (`BulkRun` Rust vs `RunProgress` TS); `IssueKey` uses banned "issue key"; "assignee" banned yet `issue_assignee`, `clean_assignees`, `AssigneePicker`. Either rename or add a "code name" column + TXT-TERMS reads identifiers. Missing entries: "Group of labels" (bare "Group" in `labels.field-label-group`, `labels.no-group`, `labels.make-group` clashes with people Group); Combine says "remove the first one" but combined choice is retired; Layout says "issue layout" vs "issue page".
Size and structure: split `bb-api/src/api/tags.rs` (1,141; planned labels.rs), `api/editing.rs` (858), `api/fields.rs` (853), relation kinds out of `api/relations.rs`, `bb-app/src/fields/values.rs` (786; reading vs planning), `history/read.rs`, `bb-store/src/tags.rs`. Bool flags → enums: `fields/values.rs:591` plan_values(creating), `:539` reference_may_be_named(access), `bb-store/src/policy/levels.rs:39` table(kind,in_project,typed). Store ports taking values per column: `bb-store/src/tags.rs:586,640`, `relations.rs:239`, `issues.rs:375,464`.
Tests/e2e support: `fields_http.rs:1191,1224` copies `issues_editing.rs:164,198` and lost conflict/draft assertions → one table over LONG_TEXT_ROUTES. Rust test helpers `key_of` 4×, `id_of` 4×, `version_of` matrix+HTTP → next to `issues_site.rs`/`examples_issues.rs`. e2e: find a field (`support/fields.ts:10`, `perf-reads.ts:213`, `demo-m3-prepare.ts:34`); `makeOwnField` returns nothing so `demo-m3-prepare.ts:22`, `layouts.ts:25` POST themselves; open new-issue dialog copied `layouts.spec.ts:29-41`, `screenshots-layouts.spec.ts:50-56`; read an issue `crowd-checks.ts:76`, `issues.ts:145,157`, `demo-m3-prepare.ts:62-71`; literal "counting" `crowd.ts:101`, `issues.ts:103`, `tooling/load/organization.ts:37`; `/^Saved/u` 9× instead of `t("save.saved")`. Query keys outside `query-keys.ts`: `features/fields/ProjectsToUse.tsx:39`, `features/labels/labels.ts:101-111`. `version ?? 1` (FIRST_VERSION) 5×: `CommentEntry.tsx:27`, `use-description.ts:62`, `use-issue-changes.ts:95`, `use-issue-page.ts:34`, `e2e/support/issues.ts:161`. Page sizes typed: `CommentVersions.tsx:11` (100), `perf-reads.ts:16` (50), `:145` (100), `bb-sample/src/issues_setup.rs:34` (200 > PAGE_SIZE_MAX).

## Nits

Time units 4× in TS; `SECONDS_PER_DAY` twice in bb-core (`edit.rs:49`, `own_fields_value.rs:21`); test clock `1_800_000_000` in 5 places. bb-api cursor block 22× → helper in `paging.rs`; `refreshIssues` twice (`bulk/bulk.ts:100`, `use-delete-issue.ts:11`). `version-exemptions.toml` two groups for "makes a new record". `JobStore for AuthStorage` in `bb-store/src/bulk.rs` not `jobs.rs`; jobs ignores `Ok(false)` from complete/fail (`jobs.rs:217-229`); bb-jobs loop untested. Wrong comments on `testing` feature (`bb-store/Cargo.toml:14`, `bb-app/src/editing/testing.rs:4`); `cargo build --workspace` switches test hooks on (say so in 04 rule 6). Names: `relations/links.rs` (banned "link"), `bb-sample/src/roster.rs` (banned "roster"), "edit notice" vs "notes"/LockNotes. `LoadDatabase` repeats `DatabaseKind`; crowd size 3× in `crowd.ts`; `sample.spec.ts:15` 50 people by hand; `bb-sample/src/relations.rs:10-14` BLOCKS/DUPLICATE positions.

Sound: module map, ports, ADR-0019/0027/0006/0021/0011/0028 in code, one ErrorCode, hidden = not_found, matrix rows, both databases, server limits not re-typed, generated files intact, AppLongPreview shared.
