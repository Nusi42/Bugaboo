//! The walk of a person's data on both databases (ADR-0044, decision 1): a scene with one row of
//! each way a row can name a person, and the counts the walk finds for the person and for nobody
//! else.

use bb_app::auth::{AccountStore as _, InvitationStore as _};
use bb_app::fields::FieldStore as _;
use bb_app::jobs::JobStore as _;
use bb_app::ports::{ChangeStore, PersonData as _};
use bb_app::time::TimeStore as _;
use bb_app::working_hours::WorkingHoursStore as _;
use bb_core::event::{Actor, EntityKind, EntityRef, FieldChange, RecordKind};
use bb_core::fields::{Field, FieldKey};
use bb_core::id::{EntityId, IdSource, InvitationId, IssueId, TimeEntryId, UserId, WorkDayId};
use bb_core::own_fields::{FieldFor, FieldType};
use bb_core::own_fields_value::FieldValue;
use bb_core::security_log::Digest;
use bb_core::sign_in::Invitation;
use bb_core::time::Minutes;
use bb_core::time::entry::TimeEntry;
use bb_core::version::FIRST_VERSION;
use bb_core::working_time::Times;
use bb_core::working_time::record::{StoredDay, WorkDay};
use bb_testing::{TestResult, each_database};
use serde_json::json;

use super::*;
use crate::db::FixedSql;
use crate::test_support::{at, definition, event, fixture, ids, person, world};

/// What the walk counts for `person`, by table.
async fn counted(storage: &AuthStorage, person: UserId) -> Result<Vec<(String, u64)>, StoreError> {
    Ok(storage
        .kinds_of(person)
        .await?
        .into_iter()
        .map(|count| (count.kind, count.rows))
        .collect())
}

/// An event by `actor` about the record `entity` of `kind`, with `changes` and `reason`.
fn about(
    source: &mut impl IdSource,
    actor: UserId,
    kind: RecordKind,
    entity: EntityId,
    changes: Vec<FieldChange>,
    reason: Option<&str>,
) -> bb_core::event::DomainEvent {
    bb_core::event::DomainEvent {
        actor: Actor::Person(actor),
        entity: EntityRef {
            kind: EntityKind::from(kind),
            id: entity,
        },
        changes,
        reason: reason.map(str::to_owned),
        ..event(source, 30)
    }
}

/// A work day of `person`.
fn work_day_of(source: &mut impl IdSource, person: UserId) -> StoredDay {
    StoredDay {
        day: WorkDay {
            id: WorkDayId::generate(source).unwrap(),
            person,
            date: jiff::civil::date(2026, 1, 5),
            zone: "Europe/Berlin".to_owned(),
            warnings: Vec::new(),
            updated_at: at(10),
            version: 1,
        },
        times: Times::default(),
    }
}

/// An invitation to `email`, made by `by`.
fn invitation_to(source: &mut impl IdSource, email: &str, by: UserId) -> Invitation {
    Invitation {
        id: InvitationId::generate(source),
        token_hash: Digest::new([7; 32]),
        email: email.to_owned(),
        invited_by: Some(by),
        created_at: at(3),
        expires_at: at(9_000),
        used_at: None,
    }
}

/// Queues a job that waits with `person` in its payload.
async fn job_naming(
    storage: &AuthStorage,
    source: &mut impl IdSource,
    person: UserId,
) -> Result<(), StoreError> {
    let kind = bb_core::technical::TechnicalName::new("mail.send")
        .map_err(|_| StoreError::Corrupt("a kind of job"))?;
    let id = bb_core::id::JobId::generate(source).map_err(|_| StoreError::Corrupt("a job"))?;
    let payload = json!({ "person": person.to_string() });
    storage.queue_job(id, &kind, &payload, at(40), at(40)).await
}

#[tokio::test]
async fn priv_03_the_walk_finds_every_row_of_a_person() -> TestResult {
    each_database("priv_03_the_walk_finds_rows", async |db| {
        let fixture = fixture(db).await?;
        let storage = &fixture.storage;
        let mut source = ids();
        let lea = person(&mut source, "lea@example.test");
        let ada_person = person(&mut source, "ada@example.test");
        let ada = ada_person.id;
        let scene = world(storage, &mut source, 1).await;
        let reviewer = definition(&mut source, FieldType::Person, 0);
        let field = reviewer.field.id;
        let issue: IssueId = scene.issues[0].id;
        let day = work_day_of(&mut source, lea.id);
        let invitation = invitation_to(&mut source, &lea.email, ada);
        let (stored_lea, stored_day) = (lea.clone(), day.clone());
        ChangeStore::record_change(storage, &event(&mut source, 6), move |tx| {
            Box::pin(async move {
                AuthStorage::insert_person(tx, &stored_lea, "hash", at(1)).await?;
                AuthStorage::insert_person(tx, &ada_person, "hash", at(1)).await?;
                AuthStorage::insert_field(tx, &reviewer).await?;
                AuthStorage::put_field_value(
                    tx,
                    EntityKind::from(RecordKind::Issue),
                    EntityId::from(issue),
                    field,
                    &FieldValue::Ids(vec![stored_lea.id.to_bytes()]).to_cells(),
                )
                .await?;
                AuthStorage::insert_work_day(tx, &stored_day).await?;
                AuthStorage::insert_invitation(tx, &invitation).await
            })
        })
        .await?;
        // Ada changes Lea's past work day, with a reason; assigns her; Lea comments as herself.
        let events = [
            about(
                &mut source,
                ada,
                RecordKind::WorkDay,
                EntityId::from(day.day.id),
                Vec::new(),
                Some("forgot to stop"),
            ),
            about(
                &mut source,
                ada,
                RecordKind::Issue,
                EntityId::from(issue),
                vec![FieldChange {
                    field: FieldKey::BuiltIn(Field::AssignedTo).code().into_owned(),
                    before: Some(json!([])),
                    after: Some(json!([lea.id.to_string()])),
                }],
                None,
            ),
            about(
                &mut source,
                lea.id,
                RecordKind::Issue,
                EntityId::from(issue),
                Vec::new(),
                None,
            ),
            about(
                &mut source,
                ada,
                RecordKind::Person,
                EntityId::from(lea.id),
                Vec::new(),
                None,
            ),
        ];
        for one in &events {
            ChangeStore::record_change(storage, one, |_| Box::pin(async { Ok(()) })).await?;
        }
        job_naming(storage, &mut source, lea.id).await?;
        let found = counted(storage, lea.id).await?;
        let rows = |table: &str| {
            found
                .iter()
                .find(|(name, _)| name == table)
                .map_or(0, |(_, rows)| *rows)
        };
        assert_eq!(rows("person"), 1, "{found:?}");
        assert_eq!(rows("work_day"), 1, "{found:?}");
        assert_eq!(rows("job"), 1, "the job that names her: {found:?}");
        assert_eq!(
            rows("invitation"),
            1,
            "the invitation to her address: {found:?}"
        );
        assert_eq!(rows("field_value"), 1, "Reviewer = Lea: {found:?}");
        // The work day by its owner, the assignment by its changes, her own event as actor, the
        // change of her record as its subject.
        assert_eq!(rows("event"), 4, "{found:?}");
        rows_say_what_she_is(storage, lea.id).await?;
        // Nobody else is found in her rows.
        let stranger = UserId::generate(&mut source);
        assert_eq!(counted(storage, stranger).await?, Vec::new());
        Ok(())
    })
    .await
}

/// A link whose field is not stored would make the walk miss rows without a word: it is an error.
#[test]
fn priv_03_a_link_to_a_field_that_is_not_stored_is_an_error_not_a_miss() {
    let all = tables();
    let comments = all
        .iter()
        .find(|table| table.marks.name() == "Comment")
        .unwrap();
    let without_columns = StoredTable {
        name: comments.name.clone(),
        marks: comments.marks,
        columns: Vec::new(),
    };
    let person = UserId::generate(&mut ids());
    assert!(
        names_the_person(comments, &all, person, Dialect::Sqlite)
            .unwrap()
            .is_some()
    );
    assert!(names_the_person(&without_columns, &all, person, Dialect::Sqlite).is_err());
}

/// Whether the plan `lines` of a query reads `table` whole: on SQLite a step `SCAN <table>` (also
/// through a whole index), on PostgreSQL a step `Seq Scan on <table>` although sequential scans
/// were switched off, that is: no index of the table can answer the arm.
fn reads_whole(lines: &[String], table: &str) -> bool {
    lines.iter().any(|line| {
        line == &format!("SCAN {table}")
            || line.starts_with(&format!("SCAN {table} "))
            || line.contains(&format!("Seq Scan on {table} "))
    })
}

/// Every arm of the walk is answered by an index (M9-F144): the walk never reads a table whole,
/// whichever way the table names the person. SQLite, which plans an empty table as a large one,
/// takes any index that can answer; PostgreSQL plans by the statistics of real rows, so on its
/// empty tables the test only finds an arm that no index can answer at all, and the plans on real
/// rows are measured with the datasets (architecture §11).
#[tokio::test]
async fn priv_03_every_arm_of_the_walk_is_answered_by_an_index() -> TestResult {
    each_database("priv_03_every_arm_has_an_index", async |db| {
        let fixture = fixture(db).await?;
        let database = fixture.storage.database();
        let person = UserId::generate(&mut ids());
        let all = tables();
        let mut whole = Vec::new();
        for table in &all {
            let Some(arms) = names_the_person(table, &all, person, database.dialect())? else {
                continue;
            };
            for (at, arm) in arms.into_iter().enumerate() {
                let plan = database
                    .plan_for_tests(&arms::counted(table, vec![arm]), false)
                    .await?;
                if reads_whole(&plan, &table.name) {
                    whole.push(format!("{} (arm {at}): {plan:?}", table.name));
                }
            }
        }
        assert!(whole.is_empty(), "read whole: {whole:#?}");
        Ok(())
    })
    .await
}

/// On a table with many rows and fresh statistics, PostgreSQL picks the index of the walk with its
/// own settings, not only when sequential scans are switched off (the test above, whose tables are
/// empty): 20,000 jobs, each naming another person, and every arm of the walk of the jobs asked for
/// one person. SQLite plans by the indexes it has, which the test above shows.
#[tokio::test]
async fn priv_03_postgresql_picks_the_index_of_the_walk_on_a_seeded_table() -> TestResult {
    each_database("priv_03_postgresql_picks_the_index", async |db| {
        let fixture = fixture(db).await?;
        let database = fixture.storage.database();
        if database.dialect() != Dialect::Postgres {
            return Ok(());
        }
        let mut transaction = database.begin().await?;
        transaction
            .execute_fixed(&FixedSql {
                sqlite: Vec::new(),
                postgres: vec![
                "INSERT INTO job (id, kind, payload, run_at, attempts, max_attempts, state,                  created_at) SELECT decode(lpad(to_hex(i), 32, '0'), 'hex'), 'mail.send',                  '{}', 0, 0, 5, 'queued', 0 FROM generate_series(1, 20000) AS i",
                "INSERT INTO job_names_person (\"row\", person) SELECT                  decode(lpad(to_hex(i), 32, '0'), 'hex'),                  decode(lpad(to_hex(i + 1000000), 32, '0'), 'hex')                  FROM generate_series(1, 20000) AS i",
                "ANALYZE job",
                    "ANALYZE job_names_person",
                ],
            })
            .await?;
        transaction.commit().await?;
        let person = UserId::generate(&mut ids());
        let all = tables();
        let jobs = all.iter().find(|table| table.name == "job").ok_or(StoreError::Corrupt("job"))?;
        let arms = names_the_person(jobs, &all, person, database.dialect())?.unwrap_or_default();
        assert!(!arms.is_empty());
        for (at, arm) in arms.into_iter().enumerate() {
            let plan = database
                .plan_for_tests(&arms::counted(jobs, vec![arm]), true)
                .await?;
            assert!(!reads_whole(&plan, "job"), "arm {at}: {plan:#?}");
        }
        Ok(())
    })
    .await
}

/// The rows of the walk: each with what the person is to it, the person as a person and the
/// moments as moments, the same on both databases.
async fn rows_say_what_she_is(storage: &AuthStorage, lea: UserId) -> Result<(), StoreError> {
    use bb_app::privacy::walk::Value;
    use bb_core::personal::Link;
    let days = storage.person_rows(lea, "work_day", 0).await?;
    assert_eq!(days.next, None);
    let day = &days.rows[0];
    assert_eq!(day.links, [Link::Subject]);
    let cell = |field: &str| {
        day.cells
            .iter()
            .find(|cell| cell.field == field)
            .map(|cell| cell.value.clone())
    };
    assert_eq!(cell("person"), Some(Value::Person(lea)));
    assert_eq!(cell("updated_at"), Some(Value::Moment(at(10))));
    assert_eq!(cell("zone"), Some(Value::Text("Europe/Berlin".to_owned())));
    let events = storage.person_rows(lea, "event", 0).await?;
    let links: Vec<Vec<Link>> = events.rows.iter().map(|row| row.links.clone()).collect();
    assert_eq!(events.rows.len(), 4, "{links:?}");
    assert!(
        links.iter().any(|found| found.contains(&Link::Actor)),
        "{links:?}"
    );
    assert!(
        links.iter().any(|found| found.contains(&Link::Subject)),
        "{links:?}"
    );
    // The changes name her, and what she is to them comes from the members, not from the link.
    assert!(links.iter().any(Vec::is_empty), "{links:?}");
    assert_eq!(
        storage.person_rows(lea, "no_such_table", 0).await?.rows,
        Vec::new()
    );
    Ok(())
}

/// The choice of an own field of time entries (a cost center) on a person's time entry is read as
/// the record it is, not as a person: only the rows of an own field that holds people hold one.
#[tokio::test]
async fn priv_03_a_choice_on_a_persons_time_entry_is_read_and_is_no_person() -> TestResult {
    each_database("priv_03_choice_on_time_entry", async |db| {
        let fixture = fixture(db).await?;
        let storage = &fixture.storage;
        let mut source = ids();
        let lea = person(&mut source, "lea@example.test");
        let scene = world(storage, &mut source, 1).await;
        let mut center = definition(&mut source, FieldType::PickOne, 0);
        center.field.applies_to = FieldFor::TimeEntries;
        let (field, choice) = (center.field.id, center.choices[0].id);
        let entry = TimeEntry {
            id: TimeEntryId::generate(&mut source)?,
            person: lea.id,
            issue: scene.issues[0].id,
            date: at(1).to_zoned(jiff::tz::TimeZone::UTC).date(),
            minutes: Minutes::new(30),
            note: String::new(),
            billable: false,
            started_at: None,
            created_at: at(1),
            version: FIRST_VERSION,
        };
        let (stored_lea, stored_entry) = (lea.clone(), entry.clone());
        ChangeStore::record_change(storage, &event(&mut source, 2), move |tx| {
            Box::pin(async move {
                AuthStorage::insert_person(tx, &stored_lea, "hash", at(1)).await?;
                AuthStorage::insert_field(tx, &center).await?;
                AuthStorage::insert_time_entry(tx, &stored_entry).await?;
                AuthStorage::put_field_value(
                    tx,
                    EntityKind::from(RecordKind::TimeEntry),
                    EntityId::from(stored_entry.id),
                    field,
                    &FieldValue::Ids(vec![choice.to_bytes()]).to_cells(),
                )
                .await
            })
        })
        .await?;
        // Every page of every kind of her rows reads; the value of the choice is no person.
        for kind in storage.kinds_of(lea.id).await? {
            let page = storage.person_rows(lea.id, &kind.kind, 0).await?;
            for row in &page.rows {
                for cell in &row.cells {
                    assert!(
                        !matches!(cell.value, bb_app::privacy::walk::Value::Person(person) if person != lea.id),
                        "{}: {:?}",
                        kind.kind,
                        cell
                    );
                }
            }
        }
        Ok(())
    })
    .await
}
