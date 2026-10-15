# WP-12 — Exit review: security and privacy fixes

**Wave 5 (exit fixes) · Folder / branch:** `bugaboo.win2` / `WIN2`
**Features:** SEC-04 (own fields, per issue type), SEC-06 (history, comments, autocomplete), PRIV-12, PRIV-13, TAG-01, TAG-04 (permissions), ISS-09, ISS-17 (locks and drafts); ADR-0019, ADR-0027.
**Source:** the M3 exit security review, copied below. Follow-ups M3-F113 (raised to should-fix here).

## Goal

Every role setting the owner can make is respected on every path: a field hidden per issue type, *Tags* or *Labels* hidden, a Guest who sees only their own issues. No path tells a reader that a hidden issue, tag or field exists. One comment cannot make page loads expensive for everyone.

## Owns

`crates/bb-policy/**` (the field mask), `crates/bb-app/src/{issues,relations,tags,editing/lock.rs,editing/drafts.rs,comments/resolve.rs,history,bulk/run.rs,follow}/**`, `crates/bb-store/src/tags.rs`, `crates/bb-api/src/api/editing.rs` (part checks only), `crates/bb-markdown/src/lib.rs` (limits), `crates/bb-core/src/text.rs`, the security tests and SEC-MATRIX quirks, `docs/architecture/12-security.md`. `crates/bb-app/src/editing/conflict.rs` only at line ~319 (the mask); WP-14 owns the rest of that file — coordinate through small commits.

## Tasks

1. All **must-fix** items M1–M3 below, each with a test written first (HTTP or SEC-MATRIX, both databases).
2. All **should-fix** items S1–S9. For S6, the cancel route and ending a person's runs on *Sign out everywhere* and on a password change. For S5's oracle, answer as for an unknown name.
3. The **nits**, except "preview cost" (measure only, record a follow-up if it misses the budget) and `is_kept` and the port range (record as accepted in `follow-ups.md` with the reason).
4. SSOT gate: after M1, bb-app can only get the per-type field mask for an issue; one helper for "seen in", one bb-policy function for "may change this issue".
5. New follow-ups go into `docs/milestones/M3/follow-ups.md` from M3-F130 up.

## Checks

SEC-MATRIX gains rows or quirks for every fixed path. TEST-ISSUE-IDS, ARCH-PERMS unchanged.

## Depends on

M3 WP-11 (main at the WP-11 merge).

## Findings (exit security review)

## Must-fix

M1. Some paths use the general field mask `field_mask()` instead of `field_mask_for(issue_type)`:

- `crates/bb-app/src/issues/description.rs:40` — PUT description can replace text hidden/read-only on this type (and the reverse: refused where allowed).
- `crates/bb-app/src/relations/lines.rs:82` `line_of` (IssueLine has empty Masked impl): title/status of a related issue hidden on its type leak via relation list, sub-issues, parent line, history parent rows, bulk preview (`bulk/judge.rs:137`, `bulk/preview.rs:95`).
- `crates/bb-app/src/editing/conflict.rs:319` — 409 `current` map shows built-in fields hidden for this type.
- `crates/bb-app/src/issues/assignees.rs:154` `may_set_assigned_to` (gate only).
Fix: use `field_mask_for(Some(issue.issue_type))`; SSOT: make the per-type mask the only one bb-app can get for an issue (make `field_mask()` private to bb-policy or rename to say "no type"); SEC-MATRIX/HTTP tests with per-type Hidden for Description, Title, Priority.

M2. Tag suggestions leak tags from issues a Guest cannot see: `crates/bb-app/src/tags/suggest.rs:36-48`, `crates/bb-store/src/tags.rs:360-395` (`used_tags`) check only `sees_project()`, never `sees_issue(reported_by)`. Fix: two project lists (all issues seen / only own reported); same in `known_tags` (`tags/manage.rs`); nothing when Tags hidden; test a Guest is not offered a tag only on someone else's issue; widen the row in `12-security.md`.

M3. `crates/bb-app/src/tags/issue.rs:67-98` `sorting_of` (GET /issues/{issue}/sorting and every add/remove answer) always returns tags and labels even when the role hides those fields (history masks them, `history/read.rs:298`). Fix: empty/omit when `!mask.can_see(Field::Tags|Labels)` for the type.

## Should-fix

S1. `crates/bb-app/src/relations/parents.rs:77-88` reads the current parent raw: `{parent:null}` on an issue under hidden H bumps the version (existence leak) and detaches from H. Read through `seen_issue_by_id`, treat hidden as none; setting a new parent while a hidden one exists: refuse as missing (or record as a decision). Test.
S2. Edit-lock/draft `part` is unchecked (`crates/bb-api/src/api/editing.rs:67-80`, `bb-app/src/editing/drafts.rs:31-49`, `editing/lock.rs:136-330`): unlimited drafts (100k chars, 30 days) fill the disk; a hidden long-text own field can be locked ("Ana is editing" leak, blocks editors). Fix: each record kind lists its valid parts (`description`, `field-<id>` of a long-text field at "can change" for the type; "can see" to read the lock); others refused like missing; cap drafts per author (~200).
S3. `crates/bb-app/src/editing/lock.rs:405-407` releasing a stuck lock needs only EditAnyway; add `ctx.require(&actor, facts.edit, facts.place)` like `edit_anyway` (M3-F35); SEC-MATRIX quirk `AlsoNeeds`.
S4. `crates/bb-app/src/comments/resolve.rs:91-131` with `MAX_TOKENS = 2_000` (`bb-markdown/src/lib.rs:44`): one comment with 2,000 distinct handles/IDs costs ≥4,000 queries per page load for every reader. Fix: resolve at most ~50 mentions and ~100 IDs per text (rest plain via `plain_tokens`); read groups and roles once per request; raise M3-F113 to should-fix.
S5. `crates/bb-app/src/tags/manage.rs:125` shortcut `limited.is_empty()` lists every unlimited tag, so a tag manager with No access to SECRET sees/renames/combines its tags: list an unlimited tag only if used where the person sees, or nowhere. `tags/judge.rs:63-65`: `tag.not_here` vs creating a new tag is an existence oracle, and an unlimited tag from a hidden project returns its stored spelling/colour: answer as for an unknown name (or accept next to F77/F78).
S6. `crates/bb-app/src/bulk/run.rs:93-100`: a background bulk run cannot be stopped; Sign out everywhere / password change don't stop it. Add a cancel route (owner, administrators) and end the person's runs on sign-out-everywhere and password change.
S7. `crates/bb-app/src/history/read.rs:351-356`: own field of kind Issue pointing at a hidden issue keeps a row "(empty) → (empty)"; skip when both sides None (as parent/project). Adjust `iss_06_history_words_values_as_the_issue_page_does…` (M3-F79 says "as for the parent").
S8. `crates/bb-app/src/issues/read.rs:106-113` and `fields/values.rs:449-456` render with `Options::default()` instead of `Ctx::html_of`, so every @name / WEB-123 becomes a chip incl. hidden ones. Render all three kinds through `html_of`.
S9. `LONG_TEXT_MAX = 100_000` chars (`bb-core/src/text.rs:15`) vs `MAX_TEXT_BYTES = 200_000` bytes (`bb-markdown/src/lib.rs:31`): CJK/emoji text is stored then shown cut off. Count in bytes or set MAX_TEXT_BYTES ≥ 4×LONG_TEXT_MAX; pin with a test.

## Nits

- Restore from trash: `issues/delete.rs:90-93`, `comments/write.rs:279-283` answer forbidden for trashed vs not_found for missing; answer not_found.
- No equal-work step for a missing comment/relation (`comments/write.rs:85-92`, `relations/links.rs:236-243`).
- SSOT: `issues/editing.rs:42-44` re-derives own-vs-all (use bb-policy `may_change_issue`); `comments/records.rs:45-58` re-implements `seen_in`.
- Followers list (`follow/use_cases.rs:147-165`) names followers who lost access via the Followers level, not `may_be_named` — decide.
- Tag search text has no length limit (`api/tags.rs:612-616`, `tags/types.rs:146`): `clean_text(…, TAG_NAME_MAX)`.
- Zero-width / RTL (Unicode Cf) characters pass in tag and label names (`bb-core/src/text.rs:86`): refuse or strip.
- `history/read.rs:246` logs a serde_json error that can contain a person value (DATA-LOGS): fixed message.
- `a@b.com` in `bb-markdown/src/tokens.rs:175` and `tests/corpus/tokens.{md,html}`: use `b.example`.
- Bulk preview of 5,000 issues in one request: consider lower limit or paging (not measured).
- Field pattern compiled per value (`fields/pattern.rs:34`): cache per definition.
- `layout_facts` (`layouts/records.rs:13`) checks no permission (only version by random ID).
- `is_kept` (`fields/values.rs:654-657`) accepts a hidden person ID already held (negligible oracle).
- Port range in `is_host` (`settings/format.rs:160`) unchecked.

Sound: hidden = missing via one rule; counts; history masking; names via one rule; locks/drafts ownership; own fields; layouts; bulk preview = apply; markdown/CSP/Trusted Types; SQL via sea-query; input limits; GDPR register; Git clean; no MCP tools yet.
