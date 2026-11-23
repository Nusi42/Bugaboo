//! The index of the walk on both databases (ADR-0044, decision 1, amendment *the index of the
//! walk*): each writer of a table whose changes or JSON can name a person writes its index in the
//! same change, the walk finds a person named only inside such a text through it, a row that goes
//! takes its index along, and migration 64 fills the index of the rows written before it.

use bb_app::history::HistoryStore as _;
use bb_app::jobs::JobStore as _;
use bb_app::notifications::NotificationStore as _;
use bb_app::ports::{ChangeStore, PersonData as _};
use bb_core::event::{Actor, EntityKind, EntityRef, FieldChange, RecordKind};
use bb_core::history::{HistoryEntry, HistoryKind};
use bb_core::id::{EntityId, EventId, IssueId, JobId, UserId};
use bb_core::technical::TechnicalName;
use bb_testing::{TestResult, each_database};
use serde_json::json;

use super::*;
use crate::AuthStorage;
use crate::db::StoredId as _;
use crate::notifications::tests::{row, stored_person};
use crate::schema::{
    Event, EventNamesPerson, HistoryNamesPerson, JobNamesPerson, NotificationNamesPerson,
};
use crate::test_support::{at, event, fixture, ids};

/// What the walk counts for `person` in `table`.
async fn walked(storage: &AuthStorage, person: UserId, table: &str) -> Result<u64, StoreError> {
    Ok(storage
        .kinds_of(person)
        .await?
        .into_iter()
        .find(|kind| kind.kind == table)
        .map_or(0, |kind| kind.rows))
}

/// How many rows of the index `table` name `person`.
async fn indexed(
    storage: &AuthStorage,
    table: impl Iden + 'static,
    person_column: impl Iden + 'static,
    person: UserId,
) -> Result<i64, StoreError> {
    let mut select = Query::select();
    select
        .expr_as(Expr::cust("COUNT(*)"), crate::schema::Computed::Total)
        .from(table)
        .and_where(Expr::col(person_column).eq(person.stored()));
    match storage.database().fetch_optional(&select).await? {
        Some(row) => row.get(crate::schema::Computed::Total),
        None => Ok(0),
    }
}

/// A change of an own field of the issue whose value holds `person` somewhere inside a text: no
/// member declared to name a person, so only the text of the change names them.
fn named_inside(person: UserId) -> Vec<FieldChange> {
    vec![FieldChange {
        field: "note".to_owned(),
        before: None,
        after: Some(json!({ "text": format!("ask {person} first") })),
    }]
}

#[tokio::test]
async fn priv_03_a_person_named_only_inside_a_change_is_found_through_the_index() -> TestResult {
    each_database("priv_03_named_only_inside_a_change", async |db| {
        let fixture = fixture(db).await?;
        let storage = &fixture.storage;
        let mut source = ids();
        let lea = stored_person(storage, &mut source, "lea@example.test").await?;
        let ada = stored_person(storage, &mut source, "ada@example.test").await?;
        let issue = IssueId::generate(&mut source)?;
        let change = bb_core::event::DomainEvent {
            actor: Actor::Person(ada),
            entity: EntityRef {
                kind: EntityKind::from(RecordKind::Issue),
                id: EntityId::from(issue),
            },
            changes: named_inside(lea),
            reason: None,
            ..event(&mut source, 20)
        };
        ChangeStore::record_change(storage, &change, |_| Box::pin(async { Ok(()) })).await?;
        assert_eq!(
            indexed(
                storage,
                EventNamesPerson::Table,
                EventNamesPerson::Person,
                lea
            )
            .await?,
            1
        );
        assert_eq!(
            walked(storage, lea, "event").await?,
            1,
            "the change names Lea"
        );
        // Ada made it and is its actor; the index does not name her: her ID is not in the text.
        assert_eq!(
            indexed(
                storage,
                EventNamesPerson::Table,
                EventNamesPerson::Person,
                ada
            )
            .await?,
            0
        );
        // An ID of nobody is in no index.
        let nobody = UserId::generate(&mut source);
        let issue_of_nobody = bb_core::event::DomainEvent {
            changes: named_inside(nobody),
            ..event(&mut source, 25)
        };
        ChangeStore::record_change(storage, &issue_of_nobody, |_| Box::pin(async { Ok(()) }))
            .await?;
        assert_eq!(
            indexed(
                storage,
                EventNamesPerson::Table,
                EventNamesPerson::Person,
                nobody
            )
            .await?,
            0
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn priv_03_the_history_and_a_notification_index_whom_their_changes_name() -> TestResult {
    each_database("priv_03_history_and_notification_index", async |db| {
        let fixture = fixture(db).await?;
        let storage = &fixture.storage;
        let mut source = ids();
        let lea = stored_person(storage, &mut source, "lea@example.test").await?;
        let ada = stored_person(storage, &mut source, "ada@example.test").await?;
        let issue = IssueId::generate(&mut source)?;
        let entry = HistoryEntry {
            event: EventId::generate(&mut source)?,
            issue,
            position: 1,
            at: at(20),
            by: Some(ada),
            agent: None,
            kind: HistoryKind::Change,
            record: EntityId::from(issue),
            record_kind: TechnicalName::new("issue")?,
            run: None,
            undid_run: None,
            changes: named_inside(lea),
        };
        storage
            .store_history(std::slice::from_ref(&entry), 1, at(21))
            .await?;
        // Read again, the row is there already: nothing is written twice.
        storage.store_history(&[entry], 1, at(22)).await?;
        assert_eq!(
            indexed(
                storage,
                HistoryNamesPerson::Table,
                HistoryNamesPerson::Person,
                lea
            )
            .await?,
            1
        );
        assert_eq!(walked(storage, lea, "history_entry").await?, 1);
        let mut told = row(&mut source, ada, Some(issue), 23)?;
        told.changes = named_inside(lea);
        storage
            .store_notifications(&[told.clone()], &[], 1, at(23))
            .await?;
        // The same notification once more is left out, and so is its index.
        storage.store_notifications(&[told], &[], 1, at(24)).await?;
        assert_eq!(
            indexed(
                storage,
                NotificationNamesPerson::Table,
                EventNamesPerson::Person,
                lea
            )
            .await?,
            1
        );
        assert_eq!(walked(storage, lea, "notification").await?, 1);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn priv_03_a_row_that_goes_takes_its_index_along() -> TestResult {
    each_database("priv_03_index_goes_with_the_row", async |db| {
        let fixture = fixture(db).await?;
        let storage = &fixture.storage;
        let mut source = ids();
        let lea = stored_person(storage, &mut source, "lea@example.test").await?;
        let kind = TechnicalName::new("mail.send")?;
        let id = JobId::generate(&mut source)?;
        let payload = json!({ "person": lea.to_string() });
        storage.queue_job(id, &kind, &payload, at(1), at(1)).await?;
        assert_eq!(
            indexed(
                storage,
                JobNamesPerson::Table,
                EventNamesPerson::Person,
                lea
            )
            .await?,
            1
        );
        assert_eq!(walked(storage, lea, "job").await?, 1);
        let worker = TechnicalName::new("worker-1")?;
        let leased = storage
            .database()
            .lease_jobs(
                &worker,
                at(2),
                jiff::SignedDuration::from_secs(60),
                5,
                &bb_app::jobs::LaneKinds::AllBut(Vec::new()),
            )
            .await?;
        assert_eq!(leased.len(), 1);
        assert!(storage.database().complete_job(id, &worker).await?);
        assert_eq!(
            indexed(
                storage,
                JobNamesPerson::Table,
                EventNamesPerson::Person,
                lea
            )
            .await?,
            0,
            "the job is done and its index went with it"
        );
        assert_eq!(walked(storage, lea, "job").await?, 0);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn priv_03_migration_64_fills_the_index_of_the_rows_written_before_it() -> TestResult {
    each_database("priv_03_migration_64_fills_the_index", async |db| {
        let database = crate::Database::connect(db.url()).await?;
        database.migrate_up_to(63, at(0)).await?;
        let mut source = ids();
        let lea = UserId::generate(&mut source);
        crate::test_support::insert_old_person(&database, lea.stored()).await?;
        // An event written the way the program before the index wrote it: no row of the index.
        let old = bb_core::event::DomainEvent {
            changes: named_inside(lea),
            ..event(&mut source, 10)
        };
        let mut insert = Query::insert();
        insert
            .into_table(Event::Table)
            .columns(crate::events::event_columns().into_iter().take(9))
            .values([
                Expr::val(old.id.stored()),
                Expr::val(crate::db::to_micros(old.time)),
                Expr::val("system"),
                Expr::val(Option::<Vec<u8>>::None),
                Expr::val(old.channel.code()),
                Expr::val(old.entity.kind.as_str()),
                Expr::val(old.entity.id.stored()),
                Expr::val(serde_json::to_string(&old.changes)?),
                Expr::val(Option::<String>::None),
            ])?;
        database.execute(&insert).await?;
        database.migrate(at(1)).await?;
        let mut select = Query::select();
        select
            .column(EventNamesPerson::Row)
            .from(EventNamesPerson::Table)
            .and_where(Expr::col(EventNamesPerson::Person).eq(lea.stored()));
        let rows = database.fetch_all(&select).await?;
        assert_eq!(rows.len(), 1);
        let row: Vec<u8> = rows[0].get(EventNamesPerson::Row)?;
        assert_eq!(row, old.id.stored());
        Ok(())
    })
    .await
}
