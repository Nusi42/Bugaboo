# ADR-0015: Personal-data marks drive GDPR features

Status: **Accepted** · 2026-10-04

## Context

GDPR features (export, removal, keep-and-delete rules, record of processing) break silently when a new feature adds personal data and forgets them.

## Decision

Every stored field is marked in its entity definition with a derive attribute, for example `#[personal(category = "employee_time", basis = "legal_obligation")]` or `#[not_personal]`. Unmarked fields fail the build. A generated **personal-data map** drives *Download my data*, the *Remove a person* plan, keep-and-delete jobs, the record of processing (Art. 30) and the data-flow map.

## Consequences

- GDPR compliance becomes part of the type system and CI, not a checklist.
- Events and logs follow the same marks.

## Amendment M6 WP-05: marks by the kind of record

A table that holds rows about many kinds of record (the domain events, the history) marks a field once for every row, and may also mark it for the rows of one kind: `#[personal_for(kind = "time_entry", category = "employee_time", basis = "contract")]`, once for each kind, beside the field's own mark. In those rows the data is about the person of that record (the person of a time entry, of a work day), not the actor of the row. `#[derive(Entity)]` writes the marks into `FieldMark::by_kind` (a kind is a word of `RecordKind`, checked when the program is built), and the personal-data page lists each one under its field. A field marked `#[not_personal]` for every row, such as the moment of an event, may still be marked for one kind: the moment of an event of a timer or a time entry is when a person started or stopped work. A kind is marked once on a field; the same kind given twice does not build. The working hours of WP-06 mark the `reason`, the `changes` and the moment of the events of a work day the same way. Enforced by: DATA-MARKS (the build of the marks), DOC-GENERATED (the page).

## Amendment M9: the links of a person

The marks say whether a field is personal; the links say whose data a row is (ADR-0044, decision 1). Every field whose type holds the ID of a person (the types listed by `names_a_person!` in `bb_core::personal`: `UserId` and `Actor`, and an `Option` or a `Vec` of them) carries one link beside its mark: `#[person(author)]`, `recipient`, `subject`, `actor` or `holder`, the words of `Link`. A reference to a record that is a person in the rows about some kinds of record (the record of an event about a person, a timer, work times or a share; the subject of a security-log entry) says so for each kind: `#[person_for(kind = "person", link = "subject")]`. `#[derive(Entity)]` writes the links into the entity's marks (`EntityMarks::person_links`, a `PersonLinks`), the list that the walk of a person's data reads, and the personal-data page names them under each entity. A link on a field whose type cannot name a person, an unknown word or two links on one field do not build; a field that names a person and has no link fails DATA-MARKS, which reads the list of person types from the same source. A linked field is stored in one column, or in a kind and an ID in that order. Enforced by: DATA-MARKS (the build and the check), DOC-GENERATED (the page), the test `priv_02_every_field_that_names_a_person_has_a_link_on_a_stored_column` of `bb-store`.

## Alternatives considered

- **A hand-maintained spreadsheet of personal data:** drifts. Rejected.

## Enforced by

DATA-MARKS, DATA-LOGS (see `tools/checks/catalog.toml`).

## Amendment (M1)

- A person's ID is itself personal data: a pseudonym, marked `#[personal(category = "identity", basis = "legitimate_interests")]`. After *Remove a person* it is kept only as the link to the *Former user …* tombstone.
- `#[not_personal]` is accepted only for field types that cannot hold personal data (they implement `NotPersonal`: numbers, moments, IDs of records, the words of the marks, and types that say so). Free text is never on that list. Remaining limits are written on the generated page `docs/privacy/personal-data.md`.
- Checked by DATA-MARKS and by the compile-fail tests in `crates/bb-core/tests/compile_fail.rs` (`not_personal_person`, `not_personal_text`).
