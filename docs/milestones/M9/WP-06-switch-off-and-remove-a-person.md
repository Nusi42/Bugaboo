# WP-06 — *Switch off* and *Remove a person* on the server

**Wave 2 · Folder / branch:** `bugaboo.win3` / `WIN3` · **Model:** Opus
**Features:** PRIV-04 (server; the screens are WP-10). [ADR-0044](../../adr/0044-download-my-data-and-remove-a-person.md) decisions 5 to 8; ADR-0022, ADR-0035 decision 7; follow-ups M2-F75, M3-F34, M3-F67, M4-F52, M4-F64, M5-F52, M5-F62, M5-F63, M7-F36, M8-F30, M8-F74.

## Goal

An administrator can switch a person off at once, and switch them on again; and can remove a person for good after seeing exactly what is deleted, what stays as *Former user K7Q2* and what is kept by law until when. The removal is one change plus one job that finishes it, survives a restore, and leaves every reference valid.

## Owns

The person's states in `crates/bb-core/src/person.rs` (*active*, *switched off*, *removed*) and the tombstone code (`former_code`, drawn from the alphabet of random issue IDs in `crates/bb-core/src/readable_id.rs`, not a second alphabet); migration 57 (`m0057_person_states.rs`: the state, the moment, who did it, the code; the unique code); the use cases in `crates/bb-app/src/people/switch_off.rs` and `crates/bb-app/src/privacy/removal/` (the plan, the change, the job kind `privacy.finish-removal`); the removal list file `privacy/removed.list` in the data folder (`crates/bb-server/src/data_folder/` names it; the store module of WP-02's restore reads it); the permission *Remove people* in `crates/bb-policy/src/catalogue.rs`; the setting `privacy.removed-texts` in `crates/bb-core/src/settings/privacy.rs` (new); the routes in `crates/bb-api/src/api/people_state.rs` (`POST /api/v1/people/{person}/switch-off`, `POST /api/v1/people/{person}/switch-on`, `GET /api/v1/people/{person}/removal-plan`, `POST /api/v1/people/{person}/remove`); `PersonRef` for removed and switched-off people in `crates/bb-app/src/access/` (wherever it resolves today, one place).

## What is decided

- ***Switch off*** (decision 5, M9-Q4 as answered): with *Manage people*; in one change: the person's sessions end, their access keys and OAuth grants are taken back, their agents stop (M7-F36: the event that the agents' package listens to), invitations they made are taken back. A switched-off person signs in to nothing, is offered in no pick of people, gets no notification or e-mail, and keeps every record; they still show by name where they did before. *Switch on* gives back the sign-in only (keys, grants and agents stay taken back). Both are in the security log.
- **The plan** (decision 6): `GET …/removal-plan` answers three lists, all from the walk of WP-03 (`PersonData` counts) and the port `KeepPeriods` of WP-04: *deleted now* (kinds and counts), *kept, shown as Former user* (kinds and counts, with the texts choice), *kept by law until* (work days with their last date to keep, the security log by its period). Nothing in the plan names a table.
- **The removal** (decision 6): with *Remove people* (organization, *Manage*, *Administrator* at the start; no key or agent; the count in architecture §7 grows by one); only a switched-off person can be removed (the window switches off first when needed); one change deletes the account and what only serves it, through each module's own function where one exists (`delete_work_times_in`, M8-F30; `delete_private_searches_of`, M5-F62; the purge of drafts, locks and notes, M3-F34; notifications and follows, M4-F64; settings and the theme choice, M8-F74), and in the walk's order for the rest; draws the code; scrubs the personal values in events (the person's settings, M2-F75; their private searches' names and texts, M5-F63; the values of deleted rows); writes the removal list line; writes a security-log entry naming the code, never the name. Files the person uploaded stay as records of the organization (M4-F52: their bytes stay with the issue; the export of WP-03 holds them for the person beforehand).
- **The texts** (decision 7, M9-Q3 as answered): `privacy.removed-texts` (*Keep, shown as Former user* at the start) and the choice in the request; *Delete their comments* deletes their comments with their versions and events.
- **The job** `privacy.finish-removal`: reindexes the full-text index of every issue whose text changed or named the person, and on SQLite runs `secure_delete` and merges the segments so the old text is not left in free pages (M5-F52); safe to run again; queued in the removal's change.
- **What stays:** work days with their breaks and reasons, under the person's name for holders of *See working hours of others* on *Working hours of others* only (ADR-0035, decision 7); the security log by ID. Everywhere else the person is *Former user K7Q2* through `PersonRef`. *Undo* of a change to a removed person is refused as for a deleted group (M3-F67, now testable over HTTP).
- **Removal list:** one line per removal (ID and moment) in `privacy/removed.list`, owner only, written in the same step as the change commits (written first to a new file and renamed, like the anchor); WP-02's restore runs every line newer than the backup again.

## Tasks

1. Write the acceptance tests below.
2. The states, the code and migration 57.
3. *Switch off* and *Switch on*, with the stop of agents, keys, grants and sessions.
4. The plan from the walk and the keep periods.
5. The removal change, the scrub of events, the texts choice, the removal list; the job.
6. `PersonRef` for removed and switched-off people; the routes; the permission and the setting.

## Acceptance tests (write first)

- `priv_04_switch_off_ends_sessions_keys_grants_and_agents_at_once` (M7-F36).
- `priv_04_a_switched_off_person_is_offered_nowhere_and_told_nothing`.
- `priv_04_switch_on_gives_back_the_sign_in_only`.
- `priv_04_the_plan_names_what_goes_what_stays_and_until_when` (read from the walk and the keep periods; a scene with every kind).
- `priv_04_after_removal_no_row_holds_the_name_or_the_e_mail` (both databases; the walk finds nothing personal left but the kept kinds).
- `priv_04_a_removed_person_is_former_user_everywhere_but_the_work_days_for_hr` (ADR-0035, decision 7).
- `priv_04_the_events_keep_no_value_of_the_persons_settings_or_private_searches` (M2-F75, M5-F63).
- `priv_04_a_shared_search_shows_former_user_as_its_maker` (M5-F62).
- `priv_04_work_times_go_in_the_change_that_removes_the_person` (M8-F30).
- `priv_04_the_full_text_index_forgets_the_removed_text` (M5-F52; SQLite free pages hold no old text).
- `priv_04_delete_their_comments_deletes_comments_and_nothing_else`.
- `priv_04_a_removal_is_on_the_removal_list_and_runs_again_after_a_restore` (with WP-02's restore).
- `priv_04_undo_of_a_change_to_a_removed_person_is_refused` (M3-F67, over HTTP).
- `priv_04_the_tombstone_code_is_random_and_unique` (PRIV-13).
- SEC-MATRIX: rows for the four routes for every starting role and an access key, both databases; a switched-off person's session, key and agent refused.

## Checks

ARCH-PERSON-REF, ARCH-PERMS, ARCH-EVENTS, ARCH-DB-PARITY, ARCH-SETTINGS, DATA-MARKS, DATA-MIGRATIONS, DATA-LOGS, SEC-MATRIX, PERF-N1, TXT-CATALOG, TXT-TERMS.

## Review

Code review with the SSOT/DRY/ODR gate: the walk is the only search by person; each module's own deletion function is called, never copied; one `PersonRef` decides how a removed or switched-off person reads. **Privacy review:** the plan against the generated personal-data list, what stays and why, the scrub of events, the full-text index, the removal list, the security-log entry. **Security review:** *Switch off* is immediate for every way in (session, key, grant, agent, MCP).

## Depends on

WP-01, WP-03 merged; WP-04's port `KeepPeriods` (its shape first; the real one when WP-04 is merged); WP-02's restore for the test of the removal list. Runs beside WP-07, WP-08 and WP-09.
