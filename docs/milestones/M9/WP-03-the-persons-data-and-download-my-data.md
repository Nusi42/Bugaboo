# WP-03 — The walk of a person's data, and *Download my data* on the server

**Wave 1 · Folder / branch:** `bugaboo.win3` / `WIN3` · **Model:** Opus
**Features:** PRIV-03 (server; the screens are WP-10). [ADR-0044](../../adr/0044-download-my-data-and-remove-a-person.md) decisions 1 to 4; ADR-0015; follow-ups M2-F65, M3-F45, M4-F52, M6-F58, M7-F65.

## Goal

Bugaboo can find every row that names one person, by what the person is to the row, through one walk that is generated beside the personal-data marks. Every person can ask for a file of everything Bugaboo stores about them, as readable pages and as JSON, and an administrator with the new permission can make the same file for somebody else, recorded and told.

## Owns

The links in `crates/bb-core/src/personal.rs` and the derive in `crates/bb-macros/` (`#[person(author)]` and its kinds beside `#[personal(…)]`), the generated list `personal::PersonLinks`; the port `PersonData` in `crates/bb-app/src/ports.rs` and its store in `crates/bb-store/src/person_data/` (one walk over the links, both databases, paged per entity); the export in `crates/bb-app/src/privacy/export/` (the job kind `privacy.export`, the ZIP writer, the readable pages, the JSON, the `README` of field names); migration 55 (`m0055_data_exports.rs`: one row per export with who asked, for whom, when, its state and the file's name); the permission *Download someone's data* in `crates/bb-policy/src/catalogue.rs`; the notification kinds *Your data is ready* and *An administrator downloaded your data* in `crates/bb-app/src/notifications/`; the routes in `crates/bb-api/src/api/data_exports.rs` (`POST` and `GET /api/v1/me/data-export`, `POST /api/v1/people/{person}/data-export`, `GET /api/v1/admin/data-exports`, the signed download link); the grown DATA-MARKS in `xtask/src/checks/data_marks.rs` with its fixtures; the regenerated `docs/privacy/personal-data.md`.

## What is decided

- **Two landings.** The links (the derive, the generated list, the grown DATA-MARKS, and the links on every existing entity) land first, as their own small landing within days of the wave's start. WP-02 and WP-04 rebase onto it and add the links of their own new columns; until it lands, they add none and write no person column the check would refuse later without one. The walk and the export land second.
- **Links** (ADR-0044, decision 1): every field that holds a `UserId` (or a list of them) carries one link: `author`, `recipient`, `subject`, `actor` or `holder`. A field of an event's payload that names a person is linked through its record kind's description. DATA-MARKS fails an entity with a person field and no link (the check reads the derive's output, the same source as the marks).
- **The walk:** `PersonData::person_rows(person, kind)` (named so, because `rows_of` is a method of the policy engine that ARCH-AUTHZ-ONLY keeps to the access module) answers, entity by entity, the rows that name the person and how; it is paged (no list without a page, coding-standards §6) and counted per entity for the removal plan (WP-06). It is the only code that searches rows by person; the export and the removal call it.
- **The file** (decision 2): one ZIP in `exports/` of the data folder (owner only, like `backups/`): `index.html` and one readable page per kind of record (plain HTML, one style sheet with the colours of *Daylight* written into the file, no script, text through `bb-markdown`), `data/<kind>.json` with the same rows, `files/` with the bytes of the files the person uploaded (M4-F52), `README.txt` naming each field in the glossary's words. Text written with the assistant keeps its mark in both (M7-F65). The `README` gives, for each kind of record, the information of Art. 15(1): its purpose, its legal basis, how long it is kept and who receives it, generated from the marks (each category has its purpose sentence in `bb-core`), `KEEP_KINDS` (WP-04; until it lands, a port whose shape WP-04 writes first) and the outside services that are switched on (mail server, AI provider and its country). Nothing of it is written by hand. The full-text index is left out. Dates in the person's *Region* on the pages, as moments in UTC in the JSON.
- **What is in it** (decision 3, M9-Q7): the person's own rows of every link kind; other people as the person sees them now (`PersonRef`); others' texts not copied. The notifications of work days are included by recipient (M6-F58). The text of an own field keeps its personal mark (M3-F45: a field cannot say "no personal data" yet; the export takes the conservative side).
- **The administrator's export** (decision 4, M9-Q8): always with a reason; *Download someone's data* (organization, *Manage*, *Administrator* at the start; no key or agent; ADR-0025 amendment M9, and the count of what no key holds in architecture §7 grows by one); the file holds what the person's own holds; the person gets *An administrator downloaded your data* in the app and by e-mail at the address on record (also when they are switched off), with the reason, never the administrator's name when *Who sees people* hides it from that person; the security log records who for whom and why.
- **Limits:** one export waiting or running per person; an export older than 7 days is deleted by the keep rule *Data exports* (WP-04 adds the row; this package deletes the file with its row when the row is deleted, through WP-04's helper once it lands, its own call until then, which WP-04 replaces).
- **Personal data:** migration 55's rows are marked; the export file is listed in *Personal data in M9* (WP-01).
- **The record of the matrix** (M2-F65): the export is an entry point of its own (`export`) in the record of SEC-MATRIX, with a line for every kind of record and every starting role, so a field hidden from a role on screen is proved hidden from it in an export the role makes of itself.

## Tasks

1. Write the acceptance tests below.
2. The links in the derive and on every entity; the generated list; the grown DATA-MARKS with a bad and a good fixture.
3. The walk in `bb-store` on both databases, with its counts.
4. Migration 55, the use cases, the job, the ZIP, the pages and the JSON.
5. The permission, the notifications, the routes and the signed link.
6. Regenerate the personal-data list; the user docs of *Download my data* (`userDocs/my-settings/my-data.md`).

## Acceptance tests (write first)

- `priv_02_every_field_that_names_a_person_has_a_link` (the generated list against the entities; DATA-MARKS's fixtures).
- `priv_03_the_walk_finds_every_row_of_a_person` (a scene with one row of every linked entity; both databases).
- `priv_03_the_file_holds_readable_pages_and_the_same_rows_as_json`.
- `priv_03_others_appear_only_as_the_person_may_see_them` (*Names*, *Role only*, *Hidden*).
- `priv_03_the_files_the_person_uploaded_are_in_the_file_with_their_bytes`.
- `priv_03_text_written_with_the_assistant_keeps_its_mark` (COMP-09).
- `priv_03_the_pages_hold_no_script` (a test of the HTML written).
- `priv_03_an_administrator_needs_download_someones_data_and_the_person_is_told`.
- `priv_03_the_administrators_export_is_in_the_security_log_with_its_reason`.
- `priv_03_a_switched_off_person_is_told_by_e_mail_of_an_administrators_export`.
- `priv_03_the_readme_gives_purpose_basis_keep_period_and_recipients_for_every_kind` (generated, compared with the marks and the keep list).
- `priv_03_one_export_at_a_time_per_person`.
- `priv_03_the_download_link_is_for_the_person_and_runs_out` (5 minutes, the clock of the app moved).
- SEC-MATRIX: rows for the four routes and the download for every starting role and an access key, both databases.

## Checks

DATA-MARKS (grown), DATA-LOGS, DATA-MIGRATIONS, ARCH-PERMS, ARCH-PERSON-REF, ARCH-EVENTS, ARCH-DB-PARITY, ARCH-AI-TOOLS (no tool reaches exports), SEC-MATRIX, SEC-RICHTEXT, PERF-N1, TXT-CATALOG, TXT-TERMS, DOC-GENERATED.

## Review

Code review with the SSOT/DRY/ODR gate: one walk (no second search by person anywhere), links beside the marks in one derive, the export's field names from the glossary. **Privacy review:** what is in the file against ADR-0044 and the generated list, what an administrator learns, what the person is told, the file's rights and keep period, nothing in logs.

## Depends on

WP-01 merged. Runs beside WP-02, WP-04 and WP-05. WP-06 builds on its walk.
