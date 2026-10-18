# WP-12 — Exit review: security, privacy and the notifications reader

**Exit wave A · Folder / branch:** `bugaboo.win3` / `WIN3`
**Features:** WF-01…WF-06, ATT-01…ATT-07, NTF-01, NTF-02, ISS-02, TAG-02, SEC-03 (security and privacy).
**Source:** [the M4 exit review](exit-review.md), section *M4 exit review: security*, and from the architecture section the items named below.

## Goal

Every right the owner can set holds on every path of M4, nothing tells a reader what they may not know, and the threat model says what is really protected.

## Owns

The server side of the findings below: `crates/bb-app/src/{issues/change.rs,notifications,history,relations/lines.rs,workflows/{publish,try_it,editor,records,read}.rs,access/changers.rs,many_changes/preview.rs,tags/judge.rs,files}`, `crates/bb-auth/src/email.rs`, `crates/bb-notify/src/smtp.rs`, `crates/bb-store/src/auth/sign_up.rs`, `crates/bb-api/src/{api/auth.rs,permissions.rs}`, `web/app/src/shared/editing/use-version-watch.ts`, the SEC-MATRIX record, `docs/architecture/12-security.md`, `docs/privacy/personal-data.md`.

## Tasks (tests first, both databases where the store is touched)

1. **Security M1** (type and status in one change skip the *Status* level of the new type): judge the named fields against both types' masks and pass `StatusCheck::may_change_status` for the new type. Tests over HTTP and through *Change many issues*, with *Status* levels that differ by type.
2. **Security M2** (handle probing): judge the 7-day rule first, add a per-person limit on refused tries, and record the remaining one probe per change as an accepted risk in 12-security.
3. **Security M3** (addresses): accept only dot-atom characters before the `@`; build the recipient with `Address::new`, never `Mailbox` parsing. Tests for `x<a@b.c>` and `a+1@b.c`.
4. **Security M4** (preview child): an accepted-risk row in 12-security and ADR-0032 §4 saying what the child holds; the sandbox hardening (Linux landlock/seccomp or a separate user, Windows restricted token) becomes a follow-up.
5. **LM1**: *History*, notifications and e-mails name issue types and label groups only through the one may-know rule (`offers/known.rs`); otherwise a neutral word. **LM2**: a workflow sits at the organization only if used there or nowhere; test with two sibling projects.
6. **L1–L8**: the version check is a background read (one rule in the route table, not a second copy of the method test); `sign_up_mail` rows are purged after `RESEND_AFTER_MINUTES` (keyed digest if cheap); a "Personal data in M4" section in 12-security (what leaves the server by e-mail; notifications 90 days, handles, sign-up rows, uploads, file bytes, mail job payloads; the M9 delete and export paths M4-F52, F64, F93) and "Still nothing leaves the server" removed; e-mail change lines cleaned once where e-mail text is put together; *Publish* uses `known_field_names`; `getWhoMayChange` checks the issue type; *Try it* needs *Check access* and the people gate of `who_may_change`; the docs of L8.
7. **Info items, decided by the lead:** a cap per hour for sign-up e-mails for the whole server (named constant); the file routes declare their real permission (`by-record`) or get matrix examples for the comment branch; the two label lookups and `project_statuses` use the one may-know rule. Recorded as follow-ups: asking for the password again before `mail.*` changes, and role mentions that e-mail every holder.
8. **From the architecture review:** adding a file notifies the followers (the reader handles `RecordKind::Attachment`; an exhaustive match so a new event kind is either handled or listed as ignored; ntf_01 test). The notifications reader decides what a person wants before it judges visibility, caches visibility per person and issue, and gets a PERF-05 budget.
9. New follow-ups go into `follow-ups.md` from M4-F120 up.

## Checks

SEC-MATRIX grows. The "event kinds the reader ignores" match is the check of task 8.

## Depends on

`main` after WP-10 (992771e4). Works beside WP-13 (texts of the same e-mails: WP-12 owns the code of `compose.rs` cleaning and the attachment event; WP-13 owns the sentences), WP-14 and WP-15.
