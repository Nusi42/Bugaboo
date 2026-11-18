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

## Amendment M9 WP-03: the links of a person

The marks say whether a field is personal; the links say whose data a row is (ADR-0044, decision 1). Each field that can name a person says, beside its mark, in which rows it does and what the person is to the row, with a word of `Link` (`author`, `recipient`, `subject`, `actor`, `holder`):

- a field whose type holds the ID of a person (the types of `names_a_person!` in `bb_core::personal`: `UserId` and `Actor`, inside `Option`, `Vec`, `Box`, `BTreeSet` or `BTreeMap`) carries `#[person(author)]`;
- a reference to a record (the types of `refers_to_a_record!`: `EntityRef`, `EntityId`) carries `#[person_for(record, link = "subject")]` when it names a person through the record it refers to, as `bb_core::whose::whose` says for each kind of record (the record is the person, belongs to a person whose ID is in a field of the record's entity, or is nobody's; a `match` without a catch-all, so a new kind has to say); `kind = "…"` names the field that holds the kind when the reference holds only the ID;
- a reference that holds a person in the rows of an own field of type *Person* or *People* carries `#[person_for(field_type = "person", key = "field", link = "subject")]`, once for each type;
- the text of an e-mail address carries `#[person_email(recipient)]` (the invitations);
- the changes of a record (the types of `carries_changes!`: `FieldChange`) carry `#[person_in_changes]`; the walk finds a person in them by the text of their ID wherever it is, and what the person is to the event comes from one place, `bb_core::whose::IN_CHANGES`, which declares the members that hold a person for each kind of record beside their names (`bb_core::whose::members`); the in-memory store of the use-case tests stops a test whose event or notification holds a person in a member that is not declared;
- JSON (the types of `holds_json!`: `serde_json::Value`) carries `#[person_in_json(subject)]`, and the walk finds a person in it by the text of their ID (the value of a proposal of an agent, the action of a change to many issues, the payload of a job);
- a reference or JSON that never names a person carries `#[names_no_person]`.

`#[derive(Entity)]` writes the links into the entity's marks (`EntityMarks::person_links`, a `PersonLinks`), the list that the walk of a person's data reads, and the personal-data page names them under each entity. A link that does not fit the type of its field, an unknown word or field type, a link given twice, two ways on one field, and `#[person_for]` without `link` do not build. A field of a type that can name a person (a person, a reference to a record, changes or JSON, also through an alias or a newtype in `bb-core`) and has no link, and a person inside a type no link sees through, fail DATA-MARKS, which reads the lists of types from the same source. A linked field is stored in one column, or in a kind and an ID in that order. The fields the walk reads from another table (the person a record belongs to, the type of an own field, the address of a person) are named once, in `bb_core::whose`, and a link the walk cannot turn into a condition is an error, not a miss. Enforced by: DATA-MARKS (the build and the check), DOC-GENERATED (the page), the tests `priv_02_every_field_that_names_a_person_has_a_link_on_a_stored_column` and `priv_02_every_kind_of_record_says_whose_it_is_through_a_stored_owner` of `bb-store`, and the store of the `bb-app` tests (the members of the changes).

## Alternatives considered

- **A hand-maintained spreadsheet of personal data:** drifts. Rejected.

## Enforced by

DATA-MARKS, DATA-LOGS (see `tools/checks/catalog.toml`).

## Amendment (M1)

- A person's ID is itself personal data: a pseudonym, marked `#[personal(category = "identity", basis = "legitimate_interests")]`. After *Remove a person* it is kept only as the link to the *Former user …* tombstone.
- `#[not_personal]` is accepted only for field types that cannot hold personal data (they implement `NotPersonal`: numbers, moments, IDs of records, the words of the marks, and types that say so). Free text is never on that list. Remaining limits are written on the generated page `docs/privacy/personal-data.md`.
- Checked by DATA-MARKS and by the compile-fail tests in `crates/bb-core/tests/compile_fail.rs` (`not_personal_person`, `not_personal_text`).
