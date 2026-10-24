# WP-04 — Versions on every change, edit locks and drafts (server)

**Wave 1 · Folder / branch:** `bugaboo.win4` / `WIN4`
**Features:** ISS-17, ISS-11 (server part); UX design §4.4 (*Editing long text*), §6; ADR-0019; architecture §5 (*Changing something*, *Editing long text*).

## Goal

No change ever overwrites another one unseen: every change route asks for the version it read, long text is edited under a lease that a closed tab frees within a minute, and a person's unsaved text is kept on the server until it is saved or thrown away.

## Owns

`crates/bb-core/src/edit*.rs` (new: `EditLock`, `LockToken`, `Draft`, lease and heartbeat times as named constants), `crates/bb-app/src/editing/**` (new), `crates/bb-store/src/editing.rs` (new) and migration `m0008` (`edit_lock`, `edit_draft`), `crates/bb-api/src/api/editing*` (new), the structured conflict answer in `bb-api`, ARCH-VERSIONED in `xtask`, the `version` field of every M2 change route that lacks one, `web/packages/api-client` helpers for heartbeats (no screens: WP-06 builds them).

## Tasks

1. **Versions everywhere:** every change route takes `version` in the body (ADR-0019 as amended in WP-01) and answers `edit.version_changed` (409) with a structured body: who changed it (a `PersonRef` for the reader), when, which fields, and the current values the reader may see. Bring every M2 change route that lacks a version up to this; where a version makes no sense (sign-in, a new record), the route is listed with its reason in an exemption file.
2. **Edit locks:** `take(what, session) → token` on the first keystroke; a lease of 60 s renewed by a heartbeat every 15 s; `release` on save or close; expired rows are deleted (no keep period); a second person gets "… is editing" with the holder as a `PersonRef` (kind *Editing*), or *Someone*. *Edit anyway* (permission from WP-01) takes the lock over and keeps the first person's text as their draft; the first person learns it at their next heartbeat. *Ask … to finish* records a request the holder sees at the next heartbeat (M3-Q3). An administrator's release of a stuck lock writes a security-log entry.
3. **Saving long text:** text, lock token and base version together; refused with a plain reason when the token is gone or taken over, or the version changed; the text is never dropped.
4. **Drafts (ISS-11):** saved privately with the heartbeat, one per person and thing being edited (or per person, project and form for a new issue); readable only by their author; deleted on save, on *Throw away*, and after the keep period of M3-Q9 (a job, ADR-0017); personal-data marks.
5. **Watching:** one small request answers the current versions of up to N things on screen (N a named constant), for the 30 s check and the return to a tab.

## Acceptance tests (write first)

- `iss_17_a_second_person_sees_who_is_editing_or_someone`, `iss_17_a_lock_frees_itself_a_minute_after_the_last_heartbeat` (fake clock), `iss_17_edit_anyway_keeps_the_first_persons_text_as_their_draft`, `iss_17_a_save_with_a_lost_lock_is_refused_and_the_text_kept`, `iss_17_a_stale_version_answers_who_changed_what` (both databases).
- `iss_11_a_draft_is_readable_only_by_its_author`, `iss_11_a_draft_goes_after_save_and_after_its_keep_period`.
- `sec_17_letting_go_of_a_stuck_lock_is_in_the_security_log`.
- `arch_versioned_every_change_route_takes_a_version` (the check's own tests).

## Checks

- **ARCH-VERSIONED** (planned → active): reads the generated OpenAPI; every change operation has `version` in its body and every long-text operation a lock token, or an exemption with a reason. The catalogue entry changes from `If-Match` to the body.
- **TEST-EDIT-LOCKS** (planned → active, server part): the tests above; WP-10 adds the browser scenarios.

## Depends on

WP-01 (ADR-0019 amendment, *Edit anyway* permission). It locks any record by `EntityRef`, so it does not wait for issues: its tests lock a record kind that exists only in the tests (in the fakes and on both databases), and WP-06 puts it on issues.
