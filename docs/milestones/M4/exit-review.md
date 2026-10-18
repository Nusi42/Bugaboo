# M4 exit review

The findings of the four exit reviews of M4 (security, architecture, words, UX), read at `main` 992771e4, and what the lead saw in the screenshots. The fix packages WP-12 to WP-16 name which items they own. Screenshots: `<work root>/screenshots/M4` (outside the repository), taken from `main` with the `screenshots-*` specs and `m4-exit-demo.spec.ts`.

## The lead's notes on the screenshots

From `screenshots/M4/demo/05-done-phone-midnight.png` and `workflows/10-editor-list-computer-daylight.png`:

1. The issue ID reads `OPS-RBNFB` (letters, not a number like WEB-142). Find why the sample or the demo issue gets such a key (a renewal of issue IDs, a random code, or the readable-ID scheme) and make the demo issue read like the others. Same as UX A6.
2. History on the Done issue: the last change (to *Done*) is not in the list yet when the picture is taken (the newest row is Open → In progress). Either the history does not refresh after a status change, or the picture is taken too early. Find which; if it is the product, fix it.
3. History row "Nora Moreno set Reported by: Nora Moreno" uses a colon, while other rows say "set Customer to Alder Street Bakery". One sentence form for every field (see the words review A3 too).
4. The details line on the phone reads "Thea Almeida · — Normal": a stray dash between the person and the priority (likely the priority icon rendered as a dash, or an empty value). Fix it.
5. Workflow editor at the level of the whole organization offers *Use the inherited workflow again* (nothing to inherit there). Hide it at the organization level (UX B, words B1: the action is *Reset to default*).
6. The workflow editor is long and dense on the computer; the UX review has the details (B: 10-editor-list).

## M4 exit review: security (read-only, at 992771e4)

I edited, built and ran nothing. I covered the files, preview, mail, sign-up, handle, notification and GDPR paths myself. Two helper agents covered workflows and offers, and the route matrix, CSRF, access keys and rate limits. I read the code myself to confirm the findings marked "verified"; the others come from their reports.

**Result:** no critical or high findings. Four medium findings should be fixed, or explicitly accepted, before `m4-done`.

### Medium

**M1. Changing the type in the same request skips the *Status* level of the new type (authorization bypass). Fix before closing M4. Verified.**

- **Where:**
  - `crates/bb-app/src/issues/change.rs:215-218`: the named fields are judged with `held.field_mask_for(Some(current.issue_type))`.
  - `crates/bb-app/src/issues/change.rs:136-147`: the status change is judged in the new type's workflow with `may_change_status: true` hard-coded.
- **Scenario:** a role has *Status* = Can change on Task and Can see on Incident. A member sends one PATCH `{issue_type: Incident, status: escalated}`. It passes. The same change made in two steps is refused.
  - This breaks ADR-0027 and "a rule never gives a right" (12-security, M4 row 1).
  - `wf_03_changing_the_type_does_not_skip_the_rules` only uses an administrator, so it misses this.
- **Fix:** when the type changes, judge the named fields against the masks of both types. Pass `StatusCheck::may_change_status(held, next.issue_type, …)` instead of `true`. Add a test with *Status* levels that differ by type, over HTTP and through *Change many issues*.

**M2. Changing one's handle tells anyone which people exist. This defeats *Who sees people*. Fix or accept before closing. Verified.**

- **Where:** `crates/bb-app/src/notifications/handles.rs:151-159`. The `name.taken` check runs before the once-in-7-days rule, which is only judged inside the change at `:182-187`.
- **Scenario:** a Guest whose *Who sees people* is *Hidden* calls the handle-change route with `firstname.lastname` guesses (handles are made from names).
  - A taken handle answers `name.taken`; a free one answers `handle.changed_lately` or succeeds.
  - A refusal never starts the 7-day clock, so the probe can be repeated without limit. Only the general limit of 600 requests a minute applies.
  - The same check also reveals group and role names through `reserved_handles`.
  - ADR-0033 §6 says "a handle is people information and follows *Who sees people*". Nothing records this as accepted.
- **Fix:**
  1. Judge the 7-day rule first.
  2. Add a small per-person limit on refused handle tries.
  3. Record the remaining one probe per change as an accepted risk in 12-security.

#### M3. `EmailAddress::parse` accepts mailbox syntax

It can disagree with lettre. Fix before closing (small). Parser verified; lettre's reading still needs a test.

- **Where:**
  - `crates/bb-auth/src/email.rs:21-36`: the part before the `@` may hold `< > " , ; ( )`.
  - `crates/bb-notify/src/smtp.rs:57`: the recipient is built with `message.to.parse::<Mailbox>()`.
- **Scenario:** with an empty domain list, `x<victim@d.com>` is a valid address. lettre very likely reads it as display name `x` plus `victim@d.com`. That would:
  - bypass the 10-minute rule per address;
  - bypass the "this address already has an account" e-mail;
  - make a second account whose stored address is not the real mailbox.

  Plus-tags (`victim+1@…`) also get past the 10-minute rule, leaving only 5 tries a minute per network. The same parser is used wherever addresses come in.
- **Fix:** accept only the RFC 5321 dot-atom characters before the `@`. Build the recipient with `Address::new(user, domain)`, never `Mailbox::parse`. Add tests for `x<a@b.c>` and for `a+1@b.c`.

#### M4. The preview child runs with all the rights of the server account

The threat model overstates its isolation. Document before closing; harden as a follow-up.

- **Where:**
  - `crates/bb-files/src/preview/spawn.rs:140-152`: `env_clear`, standard input and output, a working folder, and nothing more.
  - 12-security row "Denial of service … preview" and ADR-0032 §4 say "gets nothing of the server's".
- **Scenario:** a memory-safety bug in `image`, `hayro` or `vello_cpu` lets a crafted file run code in the child. These crates use `unsafe` internally; the no-`unsafe` rule only covers our own code. The child can then:
  - read `keys/master.key`, `keys/log.key`, `keys/csrf.key` and the database;
  - reach the network.

  The limits stop crashes and bombs, not a compromise.
- **Fix:**
  - Now: add an accepted-risk row ("the child holds the rights of the server account; protection is process separation and memory, time and pixel limits only").
  - Follow-up: on Linux, landlock or seccomp, or a separate user. On Windows, a restricted token or AppContainer once a safe crate exists.
- **Also:** 5 previews at once × 1 GiB heap means 5 GiB, with no address-space limit on Windows. Keep that in mind for small hosts.

### Low-medium

**LM1. *History* and notifications name issue types and labels that the reader may not know. Should be fixed before closing, because the WP-11 row claims "one rule".**

- **Where:**
  - `crates/bb-app/src/history/read.rs:326-332`: `Member::IssueType` goes through `LineMaker::type_name`, which holds every type (`relations/lines.rs:49-61`).
  - `history/read.rs:308-319`: label names from the event are shown without a check.
  - Notifications read changes through the same path (`notifications/read.rs:228`).
- **Scenario:** an issue of type "Question" (created in ACME) moves to WEB and is later changed to Task. A reader who only works in WEB sees "Question → Task" in *History* and in the notification or e-mail.
- **Fix:** resolve the name through `offers/known.rs` and show a neutral word otherwise, as `project_seen` already does for projects. Filter labels by `known_groups`.
- **SSOT gate (§1):** this is a second, weaker copy of the may-know rule.

#### LM2. A workflow whose uses share no common project counts as "at the organization". Follow-up

- **Where:** `crates/bb-app/src/workflows/records.rs:49` (`Place::Organization => true`) together with `place_from_uses` (`:86`).
- **Scenario:** a workflow used in HR and LEGAL only (after an org-wide use is removed, or after a project move). Any lead with *Manage workflows* can list it by name, use it, and then read its statuses, rules and group names through the editor. Its own leads also lose the right to manage it, which contradicts amendment 10.
- **Fix:** treat a workflow as sitting at the organization only if it is used there or used nowhere; otherwise require the permission for the whole organization. Add a test with two sibling projects.

### Low

- **L1. A visible issue tab still keeps a session alive.**
  - **Where:** `web/app/src/shared/editing/use-version-watch.ts:23` posts `POST /api/v1/versions/check` every 30 s. `crates/bb-api/src/api/auth.rs:120` treats a request as background only when it is a GET (verified).
  - **Effect:** the 12-security row claims this is solved; only the unread-count poll was fixed.
  - **Fix:** mark this read-only POST as background through the route table (one place), or make it a GET. Add a test like `sec_12_a_background_count_…`.
- **L2. GDPR: `sign_up_mail` rows are never deleted. Verified.**
  - **Where:** `crates/bb-store/src/auth/sign_up.rs:81`. `sessions.purge` deletes only the invitations.
  - **Effect:** the address digest and the send time are kept forever, though they are needed for 10 minutes.
  - **Also:** `address_digest` is an unkeyed hash (`bb-auth/src/token.rs:149`), so anyone with a copy of the database can confirm a guessed address.
  - **Fix:** delete rows older than `RESEND_AFTER_MINUTES` in `sessions.purge`. Optionally use a keyed hash. Small; before closing or M9.
- **L3. 12-security has no personal-data section for M3 or M4, and still says "Still nothing leaves the server" (`12-security.md:302`).**
  - **Effect:** M4 is the first time data leaves the server (e-mail: address, issue title, comment text, names to the organization's mail server).
  - **Fix:** add a "Personal data in M4" flow and table: notifications (90 days), handles (with former handles kept for good), sign-up links and mail rows, uploads and attachments, file bytes (M4-F52), mail job payloads (30 days when dead). Name the M9 delete and export paths: F52, F64, F93. Doc fix before closing (COMP-07 says each milestone review checks this).
- **L4. E-mail change lines are not cleaned.**
  - **Where:** `crates/bb-app/src/notifications/compose.rs:56-70,85-90`. `value_text` is not passed through `clean()`/`plain_line`.
  - **Effect:** a title change containing U+202E (titles go through `bb_core::text::clean`, which keeps format characters) reaches the e-mail body. ADR-0033 §3 says such characters are removed.
  - **Fix:** clean in `value_text`.
  - **SSOT gate:** the cleaning is applied call site by call site; it should happen once, where e-mail text is put together.
- **L5. *Publish* uses a weaker field rule.** `workflows/publish.rs:236-249` accepts any own field at level ≥ Can see instead of `known_field_names`. This tells whether an own-field ID exists. Use `known_field_names` (SSOT).
- **L6. `getWhoMayChange` does not check the issue type.** `access/changers.rs:63-78` does not call `ensure_type_known`. No name leaks; add the call.
- **L7. *Try it* tells about as much as *Check access* without requiring it.**
  - **Where:** `workflows/try_it.rs:53-66` and `workflows/editor.rs:150-161`.
  - **Effect:** it needs only *Manage workflows*, but answers for any person ID which roles, groups and levels they hold. It ignores the *Who sees people* gate that `who_may_change` applies. The editor lists every role name, while `listRoles` needs *Manage roles* or *Decide roles*.
  - **Fix:** require *Check access* and the same people gate, or record it as accepted. It matters only for custom roles.
- **L8. Docs no longer match the code.**
  - 12-security row ~100 lists 7 permissions no key keeps. `bb-policy/src/catalogue.rs:23-36` has more, including Manage workflows, issue types, fields and labels, and Edit anyway.
  - ADR-0031 decision 12 and 12-security say the preview needs *Manage workflows* "wherever the workflow is used". The code judges it at the workflow's place. That is sound, but write it down.

### Info (follow-up or accept)

- **The mail host can be used to probe the internal network.** Someone with *Change organization settings*, or a stolen admin session, sets `mail.host`/`mail.port` and uses the test e-mail. `mail.unavailable` and `mail.rejected` tell open ports apart. It is limited to one try every 10 s, and changing the host deletes the password. Consider asking for the password again before changing `mail.*`.
- **Sign-up can be used to send mail to strangers.** It allows 5 tries a minute per network to any address when the domain list is empty, with no overall cap. Consider a cap per hour for the whole server.
- **A Guest can mention a role and so e-mail everyone who holds it.** E-mails for mentions are on by default. It is bounded by 50 mentions per text and the 5-minute batch per issue.
- **The declared permission of the file routes is narrower than the code** (`bb-api/src/permissions.rs:480-523`). They declare `change-issues` / `see-issues`; the code also lets in a person on their own issue, and on files of their own comment. Declare `by-record` or add matrix examples for the comment branch.
- **Two label lookups skip `known_group`:** `many_changes/preview.rs:52` and `tags/judge.rs:100-119`. They give different answers for a label of a group the person may not know. The ID cannot be guessed and no name leaks.
- **The word match of a chosen status change is optional.** `status_button` is optional (`api/many_changes.rs:53`), so amendment 13's match applies only when the client sends it. The one judge still decides.
- **`project_statuses`** (`workflows/read.rs:67`) has its own `reaches` filter, a second copy of the list rule.

### SSOT/DRY/ODR (coding-standards §1) for security rules

- **The may-know rule** should come only from `offers/known.rs`. Copies or bypasses: LM1 (`LineMaker::type_name`, label names in history), L5 (`publishable`), L6, `project_statuses`, `label_exists`/`judge_label`.
- **Cleaning text for e-mail** is applied per call site (L4).
- **Background requests** are decided by method plus header in `auth.rs:120`; L1 needs a route-table rule, not a second copy.

### Checked and sound

- **Download headers** (`bb-api/src/api/files.rs:541-593`): always `application/octet-stream`, PNG only for previews. `Content-Disposition: attachment` with a safe plain name and an RFC 8187 form (no CR/LF). `nosniff`. The policy `default-src 'none'; frame-ancestors 'none'; sandbox`, which the page policy does not override (`if_not_present`) and image hosts are only added to `text/html`. The page headers also send CORP `same-origin`. Streamed bodies get no ETag and are `no-store`, so a file is never buffered.
- **Signed links:** HMAC-SHA256 with domain separation (`bugaboo.link.v1` vs `bugaboo.csrf.v1`), constant-time check, an exact 41-byte claim (attachment, target, person, expiry), at most 256 characters. Every download needs the session or key and checks the person, the expiry, the issue and the comment again.
- **Uploads:** belong to the person who started them; access is judged again at every part and at the finish. The part end is checked against the declared size. Writes are ordered under a lock with the offset checked, and synced. Type blocking at the first part and again at the finish, on the whole file. Names cleaned (path cut, control and format characters refused, trailing dots and spaces removed). Content stored by hash, so paths never come from input.
- **Preview child:** pixels read from the header before decoding, decoder limit of 512 MiB, input cap, heap cap set before reading, `RLIMIT_AS` on Linux, a 20 s kill, output read up to 4 MiB + 1 and checked to be a PNG, panics caught. Only the gif, jpeg, png and webp decoders are compiled in. The shipped program never takes the in-process path (`wiring.rs:152`; M4-F100 known).
- **Mail:**
  - The password is stored encrypted and only `credential_set` is shown. Changing host, port or user name deletes it in the same transaction (`settings/mod.rs:479-502`).
  - Secrets in the clear are refused unless the server is on this machine; the loopback check is strict.
  - With TLS, lettre's relay or starttls_relay is used, which require TLS.
  - The subject and one-line texts go through `plain_line` (CR/LF, U+2028/9, format characters). Comments are quoted line by line and the real link comes first.
  - *Short e-mails* drop the title and the text. Links use `public_url` from the start-up config, never the Host header. The sign-up secret travels in the `#fragment`.
  - Errors are logged without addresses.
- **Notifications:** checked with the recipient's mask when built and again when shown or sent (`read.rs`, `mail.rs` `mail_items`). Followers who lost access are dropped. People go through `PersonRef`. The routes are for a browser session only (`inbox_owner` refuses keys). Cursors are sealed at the API layer. The unread count is capped and counts only rows that still show.
- **Mentions** follow the writer's people level and need the recipient to see the issue (`comments/resolve.rs:165-205`).
- **Sign-up:** the same `202` for every address. The request does no account lookup (same work for every address). No person and no password exist before the link is opened. The policy is checked again when the job runs and when the link is used. Links are 24 h and single use, and are purged.
- **Matrix:** every new or renamed route has a row in the route table and in `tools/checks/sec-matrix-record.txt`, and its quirks are justified.
- **CSRF:** middleware on every change. The exemptions are exact method and path (sign-up still needs the origin check). The pagehide release is a `keepalive` fetch with the CSRF header and the lock token, no `sendBeacon`. `x-background-read` only stops a GET from renewing the session; it skips no check.
- **Access keys:** `NOT_FOR_KEYS` gained Manage workflows. Mail is covered by *Change organization settings*. Handle, notification and edit-lock routes refuse keys.
- **Workflows:** one judge on every path. *Publish* moves are judged with the publisher's rights, inside the change, under the workflows lock. The preview counts unseen issues and never names them. The strand check names nothing. Every query is built with sea-query.
- **Offers:** the may-know rule sits in one module and is used by the lists, the offer rows, *Check access* and layouts.
- **Personal-data map** (`docs/privacy/personal-data.md`): lists Notification, PersonHandle, SignUpInvitation, SignUpMail, Upload, Attachment, StoredFile, Workflow and the Job payload, all with their marks.
- **Git:** no secrets in the M4 diff (only made-up test passwords) and no profile paths or session trailers in the 398 commit messages. The commit author address is the maintainer's normal Git identity, not counted as a finding.

### Before closing M4

- **Fix:** M1, M3.
- **Fix, or accept in writing:** M2.
- **Docs:** M4 (accepted-risk row), L3 (personal-data section), L8.
- **Preferably also:** LM1, L1, L2.

Everything else can be a follow-up.

## M4 exit review: architecture (m3-done..992771e4, read-only)

How this was done: read-only (reading, grep, git, and node reading openapi.json). Nothing was built, run or edited. I checked locks, migrations, statement budgets, sizes and dependency rules myself. Four sub-reviews covered the API, Rust DRY, web DRY and docs against code. I spot-checked their key claims (F1 names, file events, the API casing and descriptions, the web query keys) and they hold.

### MUST FIX before m4-done

1. **Adding a file notifies nobody, although the docs and the UI say it does.**
   - Docs and text: ADR-0033:20, `messages/en.json` ~1819/1831-1834 ("…or adds a file to it"), and the doc comment at `bb-core/src/notification.rs:85`.
   - Code: `bb-app/src/files/upload.rs:417-421` writes only a `RecordKind::Attachment` event. `bb-app/src/notifications/build.rs:224-236` dispatches only Issue, Comment, ChangeRun and Project, and drops everything else silently (`Ok(())`). No test covers it.
   - Fix: handle Attachment events, with an ntf_01 test and a delivery line. Or reword the ADR, the texts and the comment, and record a follow-up.

2. **ARCH-API-FREEZE is a gate of m4-done but has no implementation.** It is `planned` in `tools/checks/catalog.toml` and xtask has no code for it, while the ADR-0008 M4 amendment says the frozen copy is stored at m4-done. Do the API changes listed below first, then store the compact document and switch the check on.

3. **API shape: the A-list below** has to happen before the freeze.

4. **The issue type and label group name checks are written twice, with different rules (SSOT/DRY).**
   - `bb-app/src/issues/issue_types.rs:66-80` `name_taken` compares `to_lowercase()`.
   - `bb-app/src/tags/labels.rs:101-116` `group_name_taken` compares `name_key()` (folds whitespace).
   - Both came in commit 67ae41a1. "Bug report" and "bug report" are refused as label groups but allowed as issue types.
   - "Same name" is spelled further ways in `bb-core/src/workflow/content.rs:163-166,350-357` (`trim().to_lowercase()`), and in M3 code: fields/manage.rs:45, fields/choices.rs:37, relations/kinds.rs:32, tags/manage.rs:54.
   - Fix: one `bb_core` `name_key`/`same_name`, plus one generic "taken where they meet" helper.

5. **Two required-tests implementations in xtask; this is the cause of M4-F44, so it should not be deferred.**
   - `xtask/src/checks/required_tests.rs` (TEST-WORKFLOWS, TEST-ISSUE-IDS) only looks for names. It does not see `#[ignore]`, `cfg` or a test module that is never declared.
   - `test_uploads.rs:101-135` and `test_edit_locks.rs:86-163` do see all three.
   - Fix: one implementation that has the module, ignore and cfg detection.

6. **The two check lists miss tests that the security chapter cites as proof.**
   - TEST-WORKFLOWS: the publish-move tests cited in `docs/architecture/12-security.md:142-143` (`wf_02_a_publish_with_an_issue_the_publisher_does_not_see…`, `…a_move_ignores_the_rules…`, `…the_preview_names_what_the_publisher_sees…`) are not required. They also run only on the fakes in bb-app, while ADR-0031 says "both databases".
   - TEST-UPLOADS: `xtask/src/checks/test_uploads.rs` does not require `att_04_a_pdf_bomb_costs_the_child_its_memory_limit…` (the test that backs the accepted Windows risk in ADR-0032:72 and 12-security:146), the open-upload and trash-count limit tests, or the no-disk-room and unfinished-upload tests.
   - Fix: add them, or stop citing the check ID in those rows.

7. **Web copies in M4 code that are cheap to remove (merge gate).**
   - The workflow editor copies the layout editor:
     - `usedNow`: `features/workflows/CustomizeWorkflow.tsx:8-22` = `features/layouts/CustomizeForm.tsx:64-77`.
     - The kept-draft note three times: `workflows/KeptDraftNote.tsx`, `layouts/KeptDraft.tsx`, `shared/editing/DraftOffered.tsx`.
     - Also LockedWorkflow/LockedEditor, UseInherited/RemoveLayout, and the screens and routes.
   - The offers code is copied: `issue-types/IssueTypeOffers.tsx:15-49` = `labels/GroupOffers.tsx:63-97`, and `ProjectIssueTypesScreen.tsx` ≈ `ProjectLabelsScreen.tsx`. The `offered_here ?? true` mapping is written in two ways.
   - The "status by key, a new one by name" rule is written twice: `PublishDialog.tsx:28` and `draft-model.ts:57-60`.
   - A second `settingOf` in `mail-server/connection-draft.ts:27` (one exists in `shared/my-settings.ts:61`), plus five inline copies.
   - A raw query key `["get","/api/v1/issues/{issue}/files"]` in `features/files/files.ts:84-86`. The architecture doc says keys live only in `shared/query-keys.ts`.
   - Dead `mentions.all`/`mentions.choices` keys and their reread in `shared/query-keys.ts:110-113,283`.

8. **Docs that are wrong about M4.**
   - `docs/architecture/05-runtime.md:46` says "LabelsAndTags, grouping and totals…", damage from a find-and-replace in d9da48da. It should say "Sorting". The generated `docs/03-architecture.html` carries the same error.
   - ADR-0031:82 gives the PERF-05 ceiling as 46; the code has 47 (`perf_queries.rs:21`, the WP-05 files step).
   - `12-security.md:7` still says "M4 planned".

### API changes to make before the freeze

Totals: openapi.json is 1,123,198 bytes, with 204 operations and 362 schemas.

What is consistent already: paging (`fields`/`cursor`/`limit`, answering `{items, next_cursor}`), one problem+json error shape, snake_case property names, string IDs. The M4 renames left no `bulk`, `sorting` or `me/mentions` leftovers.

#### Must

- **A1. One casing for enum values and error codes.** Today there are 118 kebab values and 87 snake values. There are 23 `rename_all = "snake_case"` in bb-api, for example `NotificationKind`, `NoticeKind`, `ReasonBody`, `ManyChangeActionBody`, `LockStatusBody`, `TriedVerdictBody`, `LevelOriginBody`, `FormatBody` (`time_zone` next to the setting key `region.time-zone`) and `RunningDatabaseBody`. The ErrorCode texts are `auth.wrong_credentials` while their message keys are kebab. Recommend kebab everywhere (the `words!` convention), or write the error-code exception into ADR-0008.
- **A2 (M4-F65). One shape for a change.**
  - Replace the 15 flat members of `HistoryEntryItem` (`api/history.rs:119-162`) and `NotificationEntry` (`api/notifications.rs:114-161`) with `change?: FieldChangeBody` (`api/many_changes.rs:232`, nested before/after `ChangeValueBody`).
  - `field_kind` goes away. `html` stays at the top, so the safe-HTML mark test is untouched. The browser already rebuilds that shape (`shared/change-sentence.ts:38`).
  - Remove the dry-exemption. Doing it "as an addition after" the freeze would keep both shapes for ever.
- **A3. Path parameters.**
  - Drop the `_id` suffix (19 operations: `{role_id}`, `{group_id}`, `{person_id}`, `{session_id}`, `{passkey_id}`, `{invitation_id}`, `{assignment_id}`, `{access_key_id}`). Today they sit next to `{role}`, `{person}` and so on.
  - `/label-groups/{group}` → `{label_group}` (the glossary says a bare *group* means people).
  - `{kind}` → `{relation_kind}` and `{people_kind}`.
- **A4. Bare `group` means three things.** Rename it to `label_group` (api/tags/bodies.rs:41,252,286,453,468) and `status_group` (`StatusRef`, workflows.rs:35; `StatusDraftBody`, workflows/bodies.rs:165).
- **A5. `IssueItem.key` / `IssueRef.key` → `readable_id`.** The glossary bans *issue key* (api/issues.rs:104, api/relations.rs:76).
- **A6. One way to name the thing being edited.** Today it is `what`, `record`+`part`, a flat `target`, and query params `kind`/`id`/`part`, with `kind` as a free string (editing/bodies.rs:7,34,271,289; drafts.rs:50; locks.rs:53). Use one `EditTargetBody { record: RecordRefBody, part }` with a `RecordKind` enum.
- **A7. Duplicate schemas (ODR).**
  - `AppliesTo` = `AppliesToBody`.
  - `{id, name}` under six names: `RoleRef`, `RoleRefBody`, `EditorRoleBody`, `GroupRef`, `GroupRefBody`, `NamedBody`.
  - `ProjectRef` = `ProjectRefBody` (and `ProjectChoiceItem` has the same members).
  - `SaveCommentBody` = `SaveDescriptionBody` = `SaveFieldTextBody`.
  - `LayoutSlotBody` = `WorkflowSlotBody`.
  - `GroupBody` = `WorkflowChoiceBody`.
- **A8. Page two lists whose answer shape would otherwise break later.**
  - `listFields` returns `{fields}` with up to 200 fields × 200 choices inline (fields/bodies.rs:116).
  - `listWorkflows` returns `{workflows, truncated}` (workflows/bodies.rs:452).
- **A9. `default_value` type.** It is declared `Option<Object>` (fields/bodies.rs:73,212,237); it should be any JSON value. The generated TS type is wrong today.
- **A10. `$ref` instead of copies.**
  - `Page<T>` (lean.rs:221) copies the item schema inline 25 times.
  - 17 schemas are orphans (no path uses them), for example `HistoryEntryItem`, `NotificationEntry`, `ProjectItem` and `Field`.
  - The `tab`/`filter` parameters inline their enums.
  - Do this before the frozen copy is stored, or the freeze diff is noisy.

#### Should (cheap now)

- **B1. Size (M4-F53).** 42% of the file is whitespace, and about 243 KB is error answers repeated per route. Moving errors, ETag, Retry-After and `fields`/`cursor`/`limit` into `components` (openapi.rs:305/405/429), together with A10, gives about 681 KB pretty, 41% smaller.
- **B1 bug.** The 429/500/503 descriptions are message keys, not English (`"error.rate-limit.exceeded (rate_limit.exceeded)"`, openapi.rs:322 and :436 use `code.message()`).
- **B2. Error answers.**
  - 400 is missing on 30 of the 31 paged lists.
  - The shared 400 and 409 texts list codes that do not belong to the route.
  - The mail routes lack 403, and `sendTestEmail` lacks its 409/502 codes.
  - 413 is missing on bodies.
  - Better: a per-route error-code list.
- **B3.** 409 has two shapes: `ConflictProblem` (27 routes) and plain `Problem` (36). `deleteLayout` and `deleteWorkflow` answer a stale version with plain `Problem`.
- **B4.** Capped lists that cannot be paged: the change/move/publish previews, sessions, field used-by, level tables, and `ManyChangePreviewBody.rows` (up to 5,000).
- **B5.** No `format: date-time` on any of the `*_at` fields.
- **B6.** Optional and nullable are mixed: 46 members are always sent, nullable, and not required. `changeIssue` answers `IssueItem` with even `id` optional.
- **B7.** Decide 200 vs 201 (and `Location`) for creates now.
- **B8. Methods and paths.**
  - Long-text saves mix PATCH and PUT.
  - `renameGroup` is the only PATCH not named `change*`.
  - Previews follow five path patterns.
  - `/auth/invitation/*` is singular next to `/auth/invitations`.
  - `/access-keys/projects` should be `/access-keys/project-choices`.
  - "Me" routes are split between `/auth/*` and `/me/*`.
- **B9. operationIds.**
  - `readDraft`, `readEditLock` → `get*`.
  - `readNotifications` → `markNotificationsRead`.
  - `getUnreadNotifications` → `getUnreadCount`.
  - `revokeInvitation`: the glossary rules out *revoke*.
  - `assignRole` → `createAssignment`.
  - `heartbeatEditLock` is a technical word.
- **B10. Clashes with glossary words.**
  - *release* (edit-locks/release, `released`) → give back.
  - `filter` vs `tab` for the same idea.
  - `/status`, `ServerStatus` vs issue status.
  - The `status` discriminator → `kind`.
  - `PersonRef.label` → `code`.
  - `display_name` vs `name`.
- **B11. Leaky or loose names.**
  - Generic names leak into schemas: `LevelTableBody_FieldKey_FieldLevel` and nine more.
  - Ten enums carry a `Body` suffix.
  - Generic schema names `Test` and `Why`.
  - `KindBody` says `bool`/`integer` where `FieldType` says `yes-no`/`whole-number`.
  - `relation` is a string where an enum fits.
  - `topic` is a string while a `Topic` enum exists.
  - `fields` maps take anything while `OwnValues` exists.
  - `PartBytes` is also used for downloads → `FileBytes`.
- **B12.** 27 rustdoc links and 7 ticket IDs show up in the descriptions.

**Fine after the freeze (additions):** security requirements per operation, validation keywords (maxLength and so on), an optional `?version` on more DELETE routes, cursors for the capped lists.

### SHOULD FIX

- **Lock order is written only in prose and tested on Postgres only by chance.**
  - The order (Policy, TagsAndLabels, Workflows, IssueCodes, then by number) is only the doc comment of `LockClass` (`bb-store/src/db.rs:398-442`).
  - Every path that takes several locks follows it today. I checked these:
    - issue_types.rs:248-250
    - tags/issue.rs:441-449 (through `group_offers_in` → `lock_chain_in`)
    - issues/create.rs:267,280
    - issues/moving.rs:332-333
    - projects/moving.rs:407,437
  - On SQLite `advisory_lock` does nothing, so a wrong order only shows up as a Postgres deadlock under load.
  - Suggest: the `Transaction` records the highest class it took and refuses (in debug builds or tests) a lower one. Name the order in `07-cross-cutting.md`.
  - The "Proven by" cell at `12-security.md:157` cites a statement-count test, which proves nothing about the order.
- **The lock key is computed from an ID four times** (`tail.copy_from_slice(&id.to_bytes()[12..])`): `bb-store/src/files.rs:268-272,425-429`, `handles.rs:78-82`, `issues.rs:640-644`. Fix: one `lock_key(id)` or `advisory_lock_for(class, id)`.
- **N+1 in the notifications reader.**
  - `build.rs:471-516` runs `seen_in` (policy statements) for every hearer of every event.
  - Visibility is not cached in `Round`, although people and settings layers are.
  - It runs before `push_row` looks at whether the person wants that kind. By default followers do not get others' field changes, yet every follower still pays a visibility check.
  - The reader has no statement budget.
  - Fix: decide wants first, cache per (person, issue), add a PERF-05 test.
- **The fixed cost per request.** The notifications page ceiling is `AT_MOST = 60` (`perf_notifications.rs:20`), "most of them the session and the rights". Measure and cut the fixed per-request cost.
- **PERF-N1 status.** It is `planned` in the catalog while its tests exist; switch it on.
- **File sizes.**
  - `bb-app/src/tags/labels.rs` is 1,056 lines (772 at M3, +284 in WP-11). That breaks the 800-line split WP-02 did.
  - About 90 non-test Rust files are over the ~400 lines of §2, with no reason given.
  - Functions are fine: clippy pedantic `too_many_lines` is on, with no suppressions.
- **Rust copies.**
  - The rule "may add or remove a file" is written three times: `files/upload.rs:55-66`, `files/attachments.rs:38-43`, `:196`, and the comment part again in `comments/write.rs`.
  - "Reaches" is written again in `workflows/records.rs:47-50`; use `bb_core::offer::reaches`.
  - The editor `Where`/`place()` pair is copied between `workflows/editor.rs:25-66` and `layouts/read.rs:27-71`, and a third `Where` (Live/Trash) means something else.
  - Five hand-written read-once caches handle a poisoned lock in two ways: `offers/known.rs` with `.ok()`, `workflows/resolve.rs` and `comments/resolve.rs` with `into_inner`.
- **Job kinds are strings, and the daily list is kept by hand.** `bb-app/src/jobs.rs:24-67`: `DAILY_JOBS` and the string `match` (195-335) are held together only by counts in tests. Use a `words!` enum with `is_daily()`.
- **Dead code.**
  - `NotificationStore::notifications_by_ids` (only a store test calls it).
  - `FieldStore::set_field_retired` (no caller).
  - `Mention.read_at`: drop the Rust field now and the column with the next migration (M4-F83). It is marked `not_personal`, while `Notification.read_at` is personal.
  - Web: dead path helpers in `shared/paths.ts:94-108`.
  - The stale comment "18 belongs to…" at migrations/mod.rs:233.
- **Migration 18 builds DDL with `format!`** (`m0018_offers.rs:21-27`, `project_reference()`), against §3 "never build SQL by string formatting". It is identifiers only; if sea-query cannot do it, give it a written reason. The same file has a `use` after a fn, and m0019 imports `sea_query` twice.
- **Web, should fix.**
  - Starting an upload is written four times (`FilesBlock`, `CommentFiles`, `chosen-files` ×2).
  - The paste handler is copied (`ui/MarkdownEditor.tsx` vs `use-pasted-files.ts`).
  - The "too big" message can say 0 bytes, because the limit is read at add time (`transfers.ts:40`).
  - `NO_ROOM = 507` should compare the error code `file.no_room` (`transfer.ts:38-45`).
  - Draft rules are copied from the server (`draft-model.ts:118-131`). An *Add* with a taken name does nothing and says nothing; rename has no check.
  - A workflow draft left by a refused publish is kept only in `useState` (`LockedWorkflow.tsx:24`). It is lost on reload, against ADR-0019 "nothing typed is dropped", and it is not a follow-up.
  - *Try it* is a `useMutation` over a GET (`workflows.ts:109-118`).
  - The file rows have their own Undo instead of `undoStore` (`FileRows.tsx:54-121`), and copy server items into state.
  - The status menu shows only `change.button`, so two changes with the same words look alike. Use `choiceLabelOf`.
  - Notification list code is copied (`NotificationsBlock` vs `NotificationEntries`).
  - Second names: `StatusGroupWord`, two `changeSentence`, two `LockedEditor`.
  - The e2e tests copy `PART_BYTES` and `MEGABYTE` by hand.
- **Docs, should fix.**
  - `04-building-blocks.md:131` lists workflows and attachments as planned.
  - bb-files and bb-notify have no sections, and bb-files is described with "scanning", which is not built.
  - The keys-never-hold list at `07-cross-cutting.md:16` and ADR-0025:66 has 7 entries; `NOT_FOR_KEYS` in `bb-policy/src/catalogue.rs:23-36` has 12.
  - ADR-0032:48 promises that files of a trashed issue go after 30 days. Nothing purges trashed issues, so they are kept for ever (a GDPR claim): reword to M9 and add a follow-up.
  - ADR-0032 says MB where `MEGABYTE` is 1024² (MiB).
  - ADR-0031 amendment 5 and the ADR-0027 WP-11 amendment describe the shared workflows lock slightly wrong.

### FOLLOW-UP

- **Widen M4-F100 (testing features).**
  - ARCH-DEPS only looks at direct dependencies; the tool list `TOOLS_WITH_TESTING` (arch_deps.rs:27) is kept by hand next to the module map.
  - `clippy --workspace` unifies features, so bb-app, bb-files and bb-store are always linted with `testing` on. The shipped configuration is never linted, and dead code kept alive only by `cfg(feature="testing")` callers (for example `PublishWorkflow::from_editor`) is never caught.
  - Suggest: a `clippy -p bb-server` step; ARCH-DEPS walks normal dependencies from bb-server transitively and makes the tools leaves.
- `StatusKey::from_name` drops non-ASCII ("Prüfung" → `pr-fung`), while handles already use `ascii_of`; reuse it. Days are turned into durations four different ways.
- **Module-map edges no crate has** (`sample --> policy`, `server --> ai` and others). The dotted "tests only" edges are not enforced.
- **ADR-0023 says nothing about test listeners on port 0** (the mailbox and the bb-notify tests).
- **The `offer` table has no unique key and no reference for `subject`**, and `label_group_by_key` was dropped (m0018). Uniqueness now rests on program checks under locks. That is fine, but it has to be in the lock-order check above.
- **Web:** pixel constants in `drawing-layout.ts`; `sendsInTheClear` knowledge kept in the browser; `sentencesOf` is not exhaustive. M4-F81 has reached its trigger (the third tab list); put a Tabs component in `ui`.

### Checks that should exist (mistakes M4 made, or could make)

1. **Lock order:** a run-time guard on the order in debug and test builds (above).
2. **Required tests:** one implementation that sees uncompiled, ignored and cfg-off tests (the cause of F44).
3. **"Proven by" cells:** every check ID and test name in such a cell of 12-security or an ADR must be in that check's required list. Finding 6 is this mistake.
4. **File size ceiling** (for example 800 lines, with named exemptions), so that WP-02's split cannot quietly come back (labels.rs).
5. **ARCH-API-FREEZE**, plus an API lint:
   - one casing for enum values
   - no duplicate schemas
   - no orphan schemas
   - no `_id` path parameters
   - glossary words in schema and property names (extend TXT-TERMS to openapi.json)
6. **Event kinds the notifications reader ignores:** every `RecordKind` is either handled or listed as ignored, with an exhaustive match (finding 1).
7. **Web:** no raw query keys outside `query-keys.ts`; the e2e tests take server constants from the generated files.

### In good shape

- **Place lookups:** `bb_core::lookup::nearest_row` is the only chain walk. Workflows, layouts, field levels and offers all use it.
- **Visibility:** one `seen_in`/`sees_issue`, and one field mask through `visible_changes`. Downloads re-check the issue.
- **Statuses:** `bb_core::issue::Status` is gone. One resolver (`StatusView::of_issue` → `WorkflowDirectory::named`). The browser has no fixed status list.
- **Constants:** file, notification and mail values live once in `bb_core` and the settings registry. The browser takes `part_bytes` and the generated constants.
- **Edit locks:** one implementation (`editing/lock.rs`, plugged in through `RecordFacts`).
- **Migrations:**
  - The checksums of migrations 1–19 are pinned in `migrations/mod.rs:449`. The pin (3c5fe3de) came before the WP-02 renames, and the values for 1–13 have not changed since.
  - m0007 and m0011 were made independent of the schema names.
  - Migrations 14–19 are additive and run on both databases.
- **Function size and suppressions:** no lint suppressions were added in M4; clippy pedantic keeps functions short.
- **One DRY exemption added** in M4 (F65, above).
- **Statement budgets:**
  - the issue page (ceiling 47, busy vs quiet)
  - lists (5 vs 40)
  - tag suggestions
  - types and label groups
  - notifications and the unread count
- **API paging and errors:** one paging protocol, one problem+json error shape. Every operationId in bb-api matches the spec, and no removed route is left behind.
- **Shipped program:** built per package (`-p bb-server`), so it has no testing hooks today.
- **Docs:** the constants, job kinds, outbox reader, `bugaboo preview`, routes and migrations named in the M4 docs all match the code.

## Words review: M4 exit (m3-done..992771e4)

I read everything without changing anything. I reviewed `messages/en.json` (556 lines added, 187 removed), the e-mail code (`crates/bb-app/src/notifications/compose.rs`), the user docs (`userDocs/` — workflows, files, notifications, labels, issue types, sign-in, people, comments), the glossary changes, UX §1, §1.1, §4.11, §5.1, §5.4 and §9, and `docs/milestones/M4/README.md`. I compared the texts against the code wherever a text makes a claim about what happens.

### A. Wrong or misleading (the text lies about behaviour, or describes something removed)

1. **Files on followed issues send no notification.** The notifications reader handles only Issue, Comment, ChangeRun and Project events (`crates/bb-app/src/notifications/build.rs:216-236`), not Attachment events. Uploading a file emits only an Attachment event (`files/upload.rs:417`), and no test covers it. These texts all promise "a file":
   - `notification.kind.followed-activity`: "A status change, a comment or a file on an issue you follow"
   - `setting.notifications.followed-activity-*.name` and `.help`: "…or adds a file to it"
   - `userDocs/notifications/notifications.md:19`, UX §4.11, ADR-0033:20
   - Fix the code, or drop "file": "A status change or a comment on an issue you follow" and "…changes the status of an issue you follow, or comments on it."
2. **The e-mail says a changed description was cleared.** The description carries no value (`history/types.rs:74`), so `compose.rs:86-92` falls through to `mail.line-cleared`: "Ana cleared Description." The screen correctly uses `history.changed-long-text`. The e-mail needs that same sentence, "{who} changed {field}."
3. **The e-mail line `mail.line-changed` reads badly or leaks code values.** The text is "{who} changed {field} to {value}."
   - *Assigned to* gives "changed Assigned to to Ben." The screen avoids this with `history.set-people` ("{who} set {field}: {after}").
   - Yes/no, time and day values are written as raw JSON (`compose.rs:65`: `value.to_string()`). A reader gets "true", "false", minutes as a bare number, or an ISO date: English or code words that are not in the catalog and not in the reader's region.
   - Proposal: reuse `history.changed`, `history.set`, `history.set-people` and `history.cleared`, and the same value wording as `change-value.ts`. Then the screen and the e-mail say one thing.
4. **A released edit lock is described wrongly.** `NoticeKind::Released` means an administrator released a stuck lock (`bb-core/src/edit.rs:149`).
   - The e-mail (`compose.rs:113`) sends `mail.line-taken-over`: "{who} took over the editing…". That is untrue.
   - The screen sends `notifications.released`: "{who} let go of the editing". That reads as if the editor themselves stopped.
   - Proposal: a key `mail.line-released` / `notifications.released` saying "{who} ended your editing because it was stuck, so others can edit again."
5. **A failed renewal of issue IDs is reported as a change to many issues.** `renewal_event` (`build.rs:239-262`) raises a `RunEnded` notification when renewing a project's issue IDs gave up. The person then reads "Your change to many issues stopped before it was done", gets the subject "Bugaboo: your change to many issues has ended", and sees *Show the result* aimed at a project ID.
   - The help text "…or when work you asked for gave up" is the only hint, and it is vague.
   - Proposal: its own sentence, "Bugaboo could not give every issue of {project} its new ID. Open the project settings to try again." In the help text, replace "work you asked for gave up" with "new issue IDs for a project could not be given out."
6. **Leftovers from before M4 that are now false:**
   - `userDocs/issues/comments.md:17` says "Comments that name you are listed under **Mentions** on the start page…". The *Mentions* block was removed. Proposal: "When a comment names you, you get a notification (see [Notifications] (the user page *Notifications*))."
   - `userDocs/issues/comments.md:19` says "Nothing is sent by e-mail yet, and there is no bell: that comes with the next version." Delete it.
   - `userDocs/issues/README.md:24` says "At the start every issue type uses the same three statuses…". Bug and Feature now use other starter workflows. Proposal: "Each issue type has a workflow. At the start, Task and Goal use *Simple* (Open, In progress, Done), Bug uses *Bug* and Feature uses *Development*: see [Workflows] (the user page on workflows)."
   - `userDocs/sign-in/sign-in.md:37` and `userDocs/people/people.md:15` say "Bugaboo cannot send e-mail yet". It can now, it just does not send invitations. Proposal: "Bugaboo does not send invitations by e-mail, so you send them the link yourself."
   - `userDocs/sign-in/sign-in.md:39` (signing up from chosen e-mail domains) does not say that signing up now needs e-mail. Add: "This works only while e-mail is on."
7. **The UX doc contradicts the code and still says "make":**
   - UX §5.4 shows the buttons "Stop working on it" and "Send back to work". The catalog and the starter workflows say "Stop working" and "Back to work". Align the UX page to the catalog.
   - UX §5.1 still has "(who can make their own account)". Proposal: "who can create their own account".
   - UX §5.1 "A later topic (mail.*, …) names its menu item here" was never filled in for mail. Add "mail.* → Connections › E-mail & notifications".
8. **The glossary itself still says "make".** Row *Invitation* (line 192): "A link an administrator makes so that one person can make their account". Proposal: "A link an administrator creates so that one person can create their account".
9. **`userDocs/workflows/changing-a-workflow.md` "Try it" section:** "…is answered with *Cannot be filled in*". No such text exists. The screen shows `error.workflow.cannot-fill`, "This status change needs a field that you cannot fill in." Quote that instead.
10. **`userDocs/notifications/mail-server.md:21`:** "*Sent too soon*: …". No such message exists. The screen says `mail-server.test-wait`. Proposal: "*You can send another test e-mail in … seconds*: a person can send one test e-mail every 10 seconds."
    - Related: when the server refuses with `rate_limit.exceeded`, `TestEmail.tsx` always shows the full 10 seconds, not the time actually left. That is a small lie in the text.
11. **`userDocs/notifications/mail-server.md:13` leaks a code name:** "…it is refused (`mail.insecure`)". Quote the text instead: "…it is refused: *The mail server is set to send its user name or password without encryption…*"

### B. Inconsistent (the same thing gets different names)

1. **`workflows.use-inherited` / `-title`:** "Use the inherited workflow again". The glossary and layouts say **Reset to default** for this action. Proposals:
   - `workflows.use-inherited` and `-title`: "Reset to default" and "Reset to default?"
   - `workflows.use-inherited-text`: replace "follows its parent again" with "follows the project above again", as Offered in does.
2. **`mail.footer`:** "You can change them under Customize." The page is called *Notification settings* (in the menu and as the page heading). Proposal: "You get this e-mail because of your notification settings. You can change them in Bugaboo under My settings › Notification settings."
   - Also, the run e-mail has no link at all (`compose.rs:154`), which is a dead end (UX principle 9). Add a line such as "Open Notifications in Bugaboo to see the result."
3. **E-mails say "address" where the screen says "link"**, which is confusing in a text about e-mail addresses:
   - `mail.sign-up-text` ("Open this address…") and `mail.open` ("Open it in Bugaboo: {address}")
   - `setting.auth.sign-up-enabled.help` ("…when they open the address in it")
   - `userDocs/people/people.md:23` and `userDocs/notifications/mail-server.md:29` ("web address")
   - `sign-up.check-text` says "link". Use "link" everywhere, e.g. "Open this link to choose your name and your password…" and "Open it in Bugaboo: {address}" → "Link to the issue: {address}".
4. **"Label group" leftovers.**
   - Catalog keys still say "groups of labels" or a bare "group":
     - `labels.project-intro`: "Switch off the label groups this project does not use…"
     - `labels.project-list`: "Label groups of the project"
     - `labels.project-create-title`: "Create a label group for this project"
     - `labels.project-create-intro`: "A label group created here is offered in this project and below."
     - `sample.step.labels`: "creating a label group or a label"
   - `userDocs/labels/README.md` §"Label groups per project" says bare "group(s)" about ten times. The glossary says a bare *Group* always means people, so each needs "label group".
5. **"Offered" and "does not use" for the same thing.** The glossary word is *Offered in* / *Not offered*.
   - `error.issue.type-not-offered` → "This project does not offer that issue type. Choose another type, or ask the project lead."
   - `error.label.not-offered` → "This project does not offer the label group of that label. Choose another label, or ask who looks after labels."
   - `many-changes-skip.label-not-here` → "The project of the issue does not offer the label group of that label"
   - `many-changes.note-type-not-used` → "{project} does not offer {type}. The issue keeps it." Also update the quote in `userDocs/issues/issue-types.md`.
6. **`mail-server.test-title` "Try it"** clashes with the glossary's *Try it (a workflow)*. Proposal: "Test e-mail", and the same for the heading in `userDocs/notifications/mail-server.md`.
7. **The mail server field is called *Mail server*, but texts call it "host name":**
   - `error.mail.unavailable` → "Bugaboo could not reach the mail server. Check the mail server, its port and the network, then try again."
   - `userDocs/notifications/mail-server.md:19` says "host name" too.
   - Also, the docs call the password box **Password**, but the label is "New password" (`mail-server.password-new`).
8. **Three names for the same notification kind:** "Your run of many changes has ended" (`notification.kind.run-ended`, `setting.notifications.run-ended-*.name`), "Your change to many issues has ended" (docs table, mail subject), and "Your change to many issues is done" (screen).
   - Proposal for the kind and setting names: "Your change to many issues has ended".
   - Likewise, `notification.kind.followed-fields` says "…to fields of an issue you follow" but the setting name says "Other changes to issues you follow". Pick one.
9. **Comment files are added with "Add files", issue files with "Upload".** `files.add-to-comment` says "Add files" while the issue block says **Upload**. The glossary verb is *Upload*. Proposal: `files.add-to-comment` → "Upload files".
10. **Rule names differ between screen and docs.**
    - `userDocs/workflows/rules.md` says "*the person assigned*", but the screen says "The person it is assigned to" (`workflows.who-assigned`).
    - Same file: "group to pick one from" vs the glossary's "Pick one group". `error.label.pick-one-conflict` also says "a label group to pick one from".
    - Choose one; I propose updating the glossary to *Label group to pick one from*, because that is what the screens say.
11. **`workflows.uses-starting`:** "These issues use the starting workflow {name}." Since M4 the glossary calls it a starter workflow. Proposal: "These issues use the starter workflow {name}."
12. **"machine" and "computer" for the same thing.** `error.mail.insecure`, `mail-server.no-encryption-note` and `setting.mail.security.help` say "machine". `tls.key-readable` and the install docs say "computer". Use "computer".
13. **Lock words in UI text, which the glossary avoids (*locked by*, *steal lock*):**
    - `workflows.ask-for-help`: "…it is long text with a lock of its own." Proposal: "The description cannot be asked for: it is written on the issue page."
    - `userDocs/workflows/changing-a-workflow.md` and `README.md` say "edit lock". Proposal: "while you edit, others see *… is editing*" (or add a glossary row, see below).
14. **Same text twice, so two places can drift apart (the SSOT/DRY merge gate, coding-standards §1):**
    - `person.bugaboo` and `history.system` (both "Bugaboo")
    - `workflows.uses-type` and `layouts.inherited-type` (both "It is only for {type}.")
    - `notifications.*` and `mail.line-*`: the same sentences, differing only in the full stop

### C. Could be plainer

1. **`workflows.publish-where`:** "{count, plural, one {# issue is} other {# issues are}} in {status}. They go to". The singular case still says "They". Proposal: `{count, plural, one {# issue is in {status}. It goes to} other {# issues are in {status}. They go to}}`.
2. **`error.workflow.invalid`:** "The workflow breaks a rule…". "Rule" is a glossary word for status-change rules. Proposal: "The workflow cannot be published like this. It needs at least one status, every status and button needs its own name, and no status change may go back to the same status."
3. **`error.workflow.no-such-change`:** add what to do. Proposal: "This status change is not in the workflow of this issue any more. Choose one from the status button."
4. **`error.workflow.cannot-fill`:** add what to do. Proposal: "This status change needs a field that you cannot fill in. Ask the person it is assigned to, or the project lead."
5. **`workflows.who-help`:** "Name nobody and everybody who may change the issue can do it." Proposal: "If you choose nobody, everybody who may change the issue can do it. A rule never gives a right that the person does not have."
6. **`workflows.rules-done`:** "Done" sits next to a status called *Done*. Proposal: "Close".
7. **The *Won't fix* button in the Bug workflow** (`workflow-change.wont-fix`) has the same words as the status. Buttons should say what they do (R6). Proposal: "Close as won't fix".
8. **`userDocs/workflows/changing-a-workflow.md` and `rules.md`** are written for developers: "counts the version up", "by what was sent", "A new status gets its key from its name", "the answer of the judge", "One judge decides". Proposals:
   - "Publish makes your changes count for everybody."
   - Drop the key sentence.
   - "Bugaboo checks every status change the same way, wherever it is made."
9. **`userDocs/labels/README.md` and `issues/issue-types.md`:**
   - "A type/group … is told to the people who see…" → "Only people who see that project … see it."
   - "the same line of projects" collides with *Offered in* lines → "the project, the projects above it and those inside it".
   - "the reason is not explained further" → "Choose another name."
10. **`handle.invalid`:** "That is not a handle." reads as blame. Proposal: "A handle can have only the letters a to z, digits, dots, dashes and underscores."
11. **`setting.mail.security.help`:** "would travel in the clear" → "could be read on the way".
12. **`setting.security.log-keep-days.help`:** "…in whole blocks once a day; a checkpoint that Bugaboo signs stays, so the log still checks out." Proposal: "Older entries are deleted once a day. A signed note of what was deleted stays, so you can still check that nobody changed the log."
13. **`userDocs/notifications/mail-server.md:16`:** "Bugaboo trusts the certificate authorities that are built into it; a mail server with a certificate of your own company cannot use encryption yet." Proposal: "Encryption works with mail servers whose certificate comes from a well-known provider. A certificate your company made itself does not work yet."
14. **`sign-up.check-text` and `sign-up.resent`** switch to "we sent". The rest of the catalog says "Bugaboo sends". Proposal: "If {email} can sign up here, Bugaboo sent a link to it…"
15. **`sign-up.resend` vs `invitation.ask-again`:** "Send a new link" and "Ask for a new link" for the same action. Pick one; I propose "Send me a new link" for both.
16. **`notifications.news`:** "There is news on this issue" is the fallback when nothing more specific can be said. It is fine as a fallback, but once file notifications exist (A1), a file needs its own sentence: "{who} added a file".
17. **`userDocs/notifications/notifications.md:35`:** "in place of the old *Mentions* block". A new reader never saw that block. Proposal: "The start page shows your newest unread notifications."
18. **`many-changes.progress`:** "{done} of {total} issues done" does not use a plural rule. Proposal: `{total, plural, one {{done} of # issue done} other {{done} of # issues done}}`.

### Glossary rows to add or change

- **Change** *Invitation*: "A link an administrator creates so that one person can create their account…" (removes the leftover "make").
- **Add** *Notification settings*: the page under *My settings* where you choose, for each kind of event, in the app or by e-mail. *Customize* on Notifications opens it. Don't say: subscriptions, alerts settings.
- **Change** *Customize*: widen it from "(notifications)" to also cover workflows and layouts ("Create a copy for this place that you can change"), or name the two meanings.
- **Change** *Reset to default*: say it also covers taking a place's own workflow or layout away. Add "use the inherited … again" to Don't say.
- **Add** *Test e-mail* / *Send me a test e-mail*. Don't say: Try it (for e-mail).
- **Add** *Label group to pick one from*, replacing *Pick one group*, or change the screens to *Pick one group*. One of the two.
- **Add** *Link* (in e-mails and on screen) for a web address you open. Don't say: address (for links), URL (in UI).
- **Add** *E-mail & notifications* (an organization settings page) next to *Mail server*, or rename the page *E-mail*. Today the admin page's name echoes the personal *Notifications*.
- **Add** to *… is editing* / *Edit anyway*: "edit lock, lock (in UI)" as Don't say, or define a UI word for it. User docs and `workflows.ask-for-help` use "lock" now.
- **Change** *Starting workflow*: mark it "(docs only, before M4)". Since M4 the UI should say *starter workflow* (see `workflows.uses-starting`).

The most urgent are A1 to A5. Each is a text that tells people something the server does not do. A1, A3 and A5 most likely need a code change as well as new words.

## UX review, M4 exit (main 992771e4, screenshots in `<work root>/screenshots/M4`)

**What I checked against:** docs/02-ux-design.html, sections §1.1 R1–R10, §3.3, §4.4, §4.5, §4.11, §5.4, §6, §6.3 and §7.2–7.5, plus docs/glossary.md.

**What I looked at:** every screen in daylight and midnight, on computer and phone. For the main screens (workflow drawing, notifications, issue page, demo) I also looked at paper and dusk. To read the tall full-page PNG files, I cut them into crops in my scratchpad. The repo and the screenshot folder were not touched.

Short paths below are relative to `screenshots/M4/`. "Computer" covers both `-computer-` and `-desktop-` file names.

---

### A. Problems on many screens (fix these once, everywhere)

#### Must fix

1. **Windows and questions are not shown at screen size.**
   - On the computer, windows sit in the middle of a 4000 px page. In `demo/02-…-computer-*` the "Start working" window is at y≈1860, and `22-publish-question-computer` has the same problem.
   - On the phone, the full-screen window footer (Done, Save, Cancel) is at the very bottom of a 2100–5500 px empty white page: `11-rules-phone-*`, `21-ask-for-phone-*`, `demo/02-…-phone-*`, `new-issue-phone-*`.
   - `22-publish-question-phone-*` shows the question in the middle of about 4000 px of blank page.
   - Either the window is not pinned to the viewport, or the screenshot tool captures the whole page. Either way the maintainer sees blank pages. Fix the window if needed, and take window and dialog shots at viewport size.
2. **Screens that ask a question have no filled main button (R6).**
   - The main button is outlined in `30-many-issues-*` ("Change 3 issues"), `move-note-*` ("Change 1 issue") and `22-publish-question-*` ("Publish and move 1 issue").
   - It should be the filled main button at the right end of the footer.
3. **Quiet buttons and links that start a line are indented about 13 px (R1).** Their inner padding pushes the text off the left edge. Examples:
   - "Show every field" and "Show every kind" in `check-access-type-*`
   - "Change" under Assigned to in `demo/03`–`05` and in `demo/02` phone
   - The Bold/Italic toolbar on every comment box
   - "The whole organization" in `offered-in-*`
   - "Follow the project above" in `project-issue-types-phone-*`

   Text-style buttons need to sit on the same left edge as the label above them.
4. **Dates and times break in the middle.**
   - File rows show "Oct 18,⏎2026", "Oct⏎18, 2026", "2026-⏎10-18": `files-*`, `issue-page/issue-*`, `demo/04`, `05`, `07`.
   - "Saved · 6:16⏎AM" in `issue-tag-saved-desktop-*`.
   - "Issue ID, for example WEB-⏎142" in `13-try-it-phone-*` and `10-editor-list-phone-*`.

   IDs, dates and times must not wrap.
5. **The notification badge in the narrow menu is cut off at the menu edge.** It shows "0C", "00" or "1…" at x≈58–72 in `demo/01`–`07` computer, all themes. It needs to sit inside the 72 px menu, or over the bell icon.
6. **The demo data does not hold together.**
   - Issue IDs look random: OPS-WKV8T, OPS-RBNFB. The glossary and concept use WEB-142.
   - The same demo step shows a different issue in different files: computer/daylight shows OPS-WKV8T, while phone and dusk show OPS-RBNFB.
   - Files history shows holiday-video-daylight.bin, -dusk.bin and -paper.bin being added and deleted, one per theme run (`files-undo-*`, `files-progress-phone-*`).

   Each theme needs fresh, identical data, and the IDs should read like WEB-142.
7. **People the viewer may not see are not shown as "Someone".** `demo/06` and `07` say "A person with the role Team member changed …" and "Added by a person with the role Team member". The glossary says *Someone*, for example "Someone (Team member)". "changed Assigned to from nobody to someone" also needs rewording.

#### Should fix

8. **Two labels stacked on one field.** "People / Search for a person", "Where? / Search for a project", "Move to / Search projects", "Add a line / Pick a project / Search projects". Seen in `10`/`13` try-it, `check-access-type`, `move-note` and `offered-in`. Use one label.
9. **Three boxes inside each other** (page card, group box, item box). This leaves a text column of about 210 px on the phone in `10-editor-list-phone`, `check-access-type-phone`, notifications and `offered-in-phone`. Drop one level.
10. **"Back" is in different places.**
    - Phone windows (rules, ask-for, new issue) put a small "< Back" at the top right, after the title.
    - Pages put it at the top left.
    - Notifications, a bottom-bar destination, has a Back it does not need.
    - Put Back top left everywhere.
11. **Secondary buttons have three looks.** Cancel is a link ("Cancel" in ask-for and new issue), an outlined button (many-issues) or a grey filled button ("Hide details" on the phone). Delete, "Mark as read", "Delete your comment" and "Take away" are accent links, but R8 keeps accent links for things that lead somewhere. Pick one quiet-button style for actions.
12. **Priority is a plain dropdown in Details and New issue, without its icon (R5).** The status in the menu, in try-it results and in move previews is plain text without its pill (`20`, `31`, `13`, `30`).
13. **Fields with nothing in them show bare empty boxes.** Tags, Customer and "Found in version" show an empty input, then an "Add tag" button below it. Concept §4.4 says a quiet "Add …".
14. **Disabled filled buttons have low contrast** (white on light violet): Publish in `13-try-it`, "Send me a test e-mail" in `11-mail-server-refused`.

---

### B. Per screen

#### workflows/10-editor-list (computer and phone)

- Must:
  - The Rules link and ✕ jump sideways from row to row, because the inputs have different widths (Rules at x≈1014, 1041 or 1072 in the computer crop).
  - On "In progress → Testing", "Has rules" pushes the ✕ onto its own line.
  - Give the inputs a fixed width, or right-align Rules and ✕.
- Must: statuses in the editor show no group color or icon (R5). Show the status pill next to each status name.
- Should:
  - The up, down and ✕ buttons are icons without words (principle 2).
  - On the phone the ✕ wraps under the select. These controls need labels and 44 px touch targets.
- Should:
  - Publish sits at the left in the middle of the page (R6 says right end).
  - "Not saved yet" is red, but red means "did not happen". Use the muted "Not saved yet" save state.
  - "Saved" next to a disabled Publish reads as "published". Say "Draft saved".
- Should: "Use the inherited workflow again" is a full-width centered button below Try it. On *Workflows of the whole organization* it means nothing, because there is nothing to inherit. Hide it there, and put it in the header elsewhere.
- Should: the "Button text" label moves left or right with the width of the Go to select (x 700 vs 726).

#### workflows/11-rules

- Should: section names (Who can do it, Only if …, Must be filled in, Ask for) are in normal weight, the same as checkbox labels. Make them 16 bold.
- Should: the hint text sits 2 px left of its heading.
- Should: "Name nobody and everybody … can do it" is hard for below-average users. Suggest "Tick nobody and everybody who may change the issue can do it."
- Must (phone): see A1 and A10.

#### workflows/12-drawing

- Should: the arrows cross and zig-zag between Open and In progress. The labels sit inside the source box, not on the arrows, so you cannot tell which button text belongs to which arrow. The arrows are thin and grey and low contrast in midnight.
- Statuses do have color and icon here, which is good.

#### workflows/13-try-it

- Must: every result says only "You may not change the status this way." without a reason. The concept promises "and why". The target status is muted text, not a pill. "WEB-1 is in Open." has no pill either.
- Should: there is no yes/no icon per row.

#### workflows/20-status-list and 31-bug-status-list

- Must: the status menu opens at the top center of the screen (y≈33). It is not attached to the status button (y≈246) and covers the header. The items are centered (R1) and have no target icons.
- Must (phone): the bottom sheet has no title and no Cancel.

#### workflows/21-ask-for and demo/02

- Should: the main button says "Save". It should say what it does: "Send to testing" or "Start working" (R6).
- Should: a form with one field opens in a 760 px window. Consider the small 440 px dialog.

#### workflows/22-publish-question

- Must: A2 (no filled main button).
- Should: grammar. "1 issue is in In progress. They go to" should read "It goes to:".
- The target select has no status icon.

#### workflows/30-many-issues

- Must: the button says "Change 3 issues" while the preview says "2 issues will change, 1 will be skipped". The button must say "Change 2 issues".
- Should:
  - The skipped row is told apart only by a slightly darker grey. Add a "Skipped" word and icon.
  - Rows show "WEB-4" without the title or status (R9).
  - Single-item bullet lists inside each box look odd.
- Should (phone): the question is centered, with about 190 px of empty space above it.

#### issue-page/issue and files (computer and phone)

- Must: Labels shows "No labels. Every label is on this issue already." This contradicts itself. If the project has no labels, say "This project has no labels yet."
- Must (computer):
  - The "Assigned to" name is not lined up with its value (`demo/03`–`05`: the label is centered between the value and the Change line).
  - On the phone, field names sit 5 px above the middle of their values.

  Align names to the first line of the value.
- Should: "Edit the description" floats in the middle of the column (x≈650). Put it at the right end of the Description header.
- Should: Followers stacks four lines ("1 follower", "Stop following", "Following", "You reported it"). Use one line with the state and one button.
- Should:
  - The tag "urgent" is pink/red text, but tags are neutral round words (§7.5).
  - "Saved · Undo" appears below the Add tag button, not where the change happened.
- Should:
  - The comment toolbar wraps on the phone, leaving "Quote" alone on a second line.
  - The phone issue bar (Status · Comment · More) does not fill the width.
- Should: Back to list and Hide list are links, while Previous and Next are buttons, all in the same row.
- Should (Paper): the selected row (blue-grey) is hard to tell from in-progress rows (teal) (`demo/03-…-paper`).
- Must (`files-phone-*`): the "Close" button in the "Too big" note sits about 8 px from the red edge, and the note text starts at 20 px.
- Should: the Undo note "crash.log is deleted" uses the accent violet. Notes are blue for telling (§6.1).
- Should: comment actions on the phone ("Add files", "Edit your comment", "Delete your comment") are stacked with big gaps. The Download and Delete in a comment's file row do not line up with those in the issue's own file rows.

#### issue-page/new-issue and new-issue-types

- Must (phone): the footer buttons wrap. Cancel and "Create and start another" share one row, and Create drops alone to the next row. Stack them full width, with the main button first.
- Should: the Write/Preview tabs sit above the "Description" label. The label belongs above the tabs.
- Should: "Details" and "Priority" are crammed together with the same style.
- Should: Create looks enabled with an empty required title, while the hint says "Write a title first".
- Should: "Customize this form" is 12 px left of Create's right edge.
- Note: `new-issue-types` is identical to `new-issue`. The type dropdown is closed, so the screenshot does not show which types are offered.

#### notifications/01-start-page

- Should: "Welcome, Ana Analyst" sits alone in a big card with uneven padding.
- Should: unread items get a violet fill plus a 4 px violet left edge. That copies the priority-edge pattern, and color is kept for state (R4). Use a quieter "New" marker.
- Should: the issue group header has no status pill (R9).

#### notifications/02–04 (notifications page)

- Must: "Mark all as read" (small, plain) and "Customize" (larger, underlined) have different styles and baselines. They sit on their own row instead of at the right of the title row.
- Should:
  - The first group box touches the tab underline, with no gap.
  - In `04`, the active tab "Mentions" has a grey fill that "All" did not have in `02`. Focus or hover is leaking into the screenshot.
- Should (phone): there is a Back button it does not need, the heading "Notifications" appears twice, and the action links are right-aligned.

#### notifications/05-customize

- Should (midnight phone and computer): the off switch track almost disappears on black. Its border needs more contrast.

#### notifications/06-handle-refused

- Should: the error is a separate box. The input itself is not marked. Put the message under the field and mark the field.
- Should: the text "…or had it, or a group or a role has it" is clumsy.

#### notifications/10–12 (mail server)

- Must: two filled buttons in one part, "Save the mail server" and "Send me a test e-mail" (R6). Make the test button outlined.
- Must: in the two-column grid the inputs do not line up. The hints have different lengths, so the user name input is at y≈654 and Send from at y≈615. Put hints below the fields, or align the rows.
- Should: section headings (Sending, Connection) look the same as field labels.
- Should: "No password is set." is bold and reads like a heading.
- Should (phone): the three connection choices wrap into ragged buttons. Use a full-width radio list.

#### notifications/20-sign-up and 21-check-your-e-mail

- Must: three different left edges. The wordmark is at x=336, the card at 421, the Theme row at 336–944. Line them up or center them all.
- Should (phone, 21): the reason "You can ask for a new link in 10 minutes" sits under "Use another address", not under the disabled "Send a new link".

#### offered-in (organization Issue types)

- Must: the type list rows (Task, Feature, Goal) are underlined accent links, and the selected Bug row has no highlight (R8). The icon is on the right.
- Must: "The whole organization · Not offered [Take away]" does not make sense, because "take away" what? Say "Remove this line".
  - Also, Offered / Not offered have no icon.
  - "The whole organization" option is styled differently from the radio rows below it.
- Should:
  - "Save" sits in the middle of the panel, but the Retire switch below it saves at once.
  - The panel is a card inside a card.
- Should (phone): the type list is not visible next to the open Bug editor. Check that people can get to the other types.

#### project-issue-types and project-labels

- Must: the same thing is shown two ways. Issue types show "Follow the project above" as an accent action on every row. Labels show "Follows the project above" as muted state text (R9).
- Must (phone): nine underlined links for project settings wrap into four rows, and they also wrap on the computer. Use the same side menu or list as Organization settings.

#### move-note

- Must: A2.
- Should: "Bug is not used in Lab. The issue keeps it." is a bullet like a change. Show it as an info note: "The type Bug is not offered in Lab; the issue keeps it."
- Should: "Chosen project: Lab" sits above the double label.

#### check-access-type

- Should: A3, A8 and A9 apply.
- Nice: Yes/No have words, but no check or cross icon. Sixteen repeated "No. No role … gives this." lines could be grouped.

#### demo/01–07

- Must: A5, A6 and A7 apply.
- Should (01–05):
  - Four identical folder icons in the narrow menu cannot be told apart.
  - The Customer value "Alder Street Bakery" touches the input's right edge.
- Should (07, guest):
  - The Done button keeps its dropdown arrow although the guest may not change it.
  - The phone issue bar still offers "Status".
  - Each read-only reason wraps to four lines with the lock icon centered on it. That is crowded. Use a lock beside the value and a one-line reason.
- Nice: the time shows "1:16 AM" for one viewer and "3:16 PM" for another on the same issue. Check the time zone.

---

### C. Screens that look good

These are good in all themes, apart from the shared items in A:

- `notifications/05-customize` on the computer: clean rows, aligned switches, plain words.
- `notifications/12-mail-server-tried-and-on`: success, warning and error notes are correct.
- The status button and the list row colors (tint plus priority edge, icon plus word) on the issue page, in all four themes. Paper's teal for In progress works.
- `workflows/12-drawing` status boxes: color plus icon plus word.
- The handle page layout on the computer, apart from the error placement.

The sign-up and check-your-e-mail cards are good inside the card, but the page edges are not.

In my view, the **must** items in A1–A7 and the must items in B have to be fixed before the maintainer sees this set. A1 and A6 alone would make the set look broken on a phone.
