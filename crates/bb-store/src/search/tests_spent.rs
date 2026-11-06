//! Time spent through the one expression of the compiler, on both databases (ADR-0035 decision 4,
//! ARCH-DB-PARITY): the minutes of each issue of a list, the sum over the issues a search finds with
//! the reader's sight in it, a person's minutes for each date, and one timer row for each person.

use bb_app::issues::IssueStore as _;
use bb_app::ports::ChangeStore;
use bb_app::time::TimeStore as _;
use bb_app::time::spent::TimeSpentStore as _;
use bb_core::id::{IssueId, TimeEntryId, UserId};
use bb_core::time::Minutes;
use bb_core::time::entry::{Running, TimeEntry, Timer};
use bb_core::version::{FIRST_VERSION, Version};
use bb_testing::{TestResult, each_database};
use jiff::civil::date;

use super::tests::{request, seed};
use crate::AuthStorage;
use crate::test_support::{at, event, fixture};

/// An entry of `person` on `issue`.
fn entry(
    source: &mut impl bb_core::id::IdSource,
    person: UserId,
    issue: IssueId,
    (day, minutes): (i8, u32),
) -> Result<TimeEntry, bb_core::id::IdError> {
    Ok(TimeEntry {
        id: TimeEntryId::generate(source)?,
        person,
        issue,
        date: date(2027, 1, day),
        minutes: Minutes::new(minutes),
        note: String::new(),
        billable: false,
        started_at: None,
        created_at: at(0),
        version: FIRST_VERSION,
    })
}

#[tokio::test]
async fn arch_db_parity_time_spent_is_the_same_sum_through_the_list_the_search_and_the_dates()
-> TestResult {
    each_database("time_spent", async |db| {
        let fixture = fixture(db).await?;
        let mut data = seed(&fixture.storage).await;
        let storage = &fixture.storage;
        let (first, second, third) = (data.issues[0].id, data.issues[1].id, data.issues[2].id);
        let stranger = UserId::generate(&mut data.source);
        let entries = vec![
            entry(&mut data.source, data.me, first, (14, 90))?,
            entry(&mut data.source, data.me, first, (14, 30))?,
            entry(&mut data.source, stranger, second, (15, 15))?,
            entry(&mut data.source, data.me, data.hidden.id, (16, 60))?,
        ];
        ChangeStore::record_change(storage, &event(&mut data.source, 7), move |tx| {
            Box::pin(async move {
                for each in &entries {
                    AuthStorage::insert_time_entry(tx, each).await?;
                }
                Ok(())
            })
        })
        .await?;
        // One row for each issue asked about, 0 for one without entries and one that is not there.
        let nowhere = IssueId::generate(&mut data.source)?;
        assert_eq!(
            storage
                .spent_on_issues(&[first, second, third, nowhere])
                .await?,
            vec![
                (first, Minutes::new(120)),
                (second, Minutes::new(15)),
                (third, Minutes::default()),
                (nowhere, Minutes::default()),
            ]
        );
        // The sum over a search counts only the issues the sight lets it find.
        let everything = request(&data, "", None, 1);
        assert_eq!(
            storage.spent_on_search(&everything).await.ok(),
            Some(Minutes::new(135))
        );
        let mut own = request(&data, "", None, 1);
        own.sight.all.clear();
        own.sight.own = vec![data.project];
        own.sight.person = stranger;
        assert_eq!(
            storage.spent_on_search(&own).await.ok(),
            Some(Minutes::default())
        );
        // A person's minutes for each date, the dates without entries left out.
        assert_eq!(
            storage
                .booked_per_date(data.me, date(2027, 1, 11), date(2027, 1, 17))
                .await?,
            vec![
                (date(2027, 1, 14), Minutes::new(120)),
                (date(2027, 1, 16), Minutes::new(60)),
            ]
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn arch_db_parity_the_sum_of_a_search_leaves_out_the_entries_of_an_issue_in_the_trash()
-> TestResult {
    each_database("time_spent_trash", async |db| {
        let fixture = fixture(db).await?;
        let mut data = seed(&fixture.storage).await;
        let storage = &fixture.storage;
        let (kept, trashed) = (data.issues[0].id, data.issues[1].id);
        let entries = vec![
            entry(&mut data.source, data.me, kept, (14, 60))?,
            entry(&mut data.source, data.me, trashed, (14, 45))?,
        ];
        ChangeStore::record_change(storage, &event(&mut data.source, 7), move |tx| {
            Box::pin(async move {
                for each in &entries {
                    AuthStorage::insert_time_entry(tx, each).await?;
                }
                Ok(())
            })
        })
        .await?;
        let everything = request(&data, "", None, 1);
        assert_eq!(
            storage.spent_on_search(&everything).await.ok(),
            Some(Minutes::new(105))
        );
        // Into the trash: its entries stay, but no search finds the issue, so its minutes are not summed.
        ChangeStore::record_change(storage, &event(&mut data.source, 8), move |tx| {
            Box::pin(async move {
                AuthStorage::set_issue_deleted(tx, trashed, Some(at(9)), at(9)).await?;
                Ok(())
            })
        })
        .await?;
        assert_eq!(
            storage.spent_on_search(&everything).await.ok(),
            Some(Minutes::new(60))
        );
        // The port still tells the minutes booked on the issue itself.
        assert_eq!(
            storage.spent_on_issues(&[trashed]).await?,
            vec![(trashed, Minutes::new(45))]
        );
        Ok(())
    })
    .await
}

/// Writes a timer in a change of its own: a new row when `version` is none.
async fn put(
    storage: &AuthStorage,
    source: &mut impl bb_core::id::IdSource,
    timer: Timer,
    version: Option<Version>,
) -> Result<bool, crate::error::StoreError> {
    ChangeStore::record_change(storage, &event(source, 8), move |tx| {
        Box::pin(async move { AuthStorage::put_timer(tx, &timer, version).await })
    })
    .await
}

#[tokio::test]
async fn time_01_a_person_has_one_timer_row() -> TestResult {
    each_database("time_one_timer_row", async |db| {
        let fixture = fixture(db).await?;
        let mut data = seed(&fixture.storage).await;
        let storage = &fixture.storage;
        let timer = Timer {
            person: data.me,
            running: Some(Running {
                issue: data.issues[0].id,
                started_at: at(10),
                warned_at: None,
            }),
            changed_at: at(10),
            version: FIRST_VERSION,
        };
        assert!(put(storage, &mut data.source, timer.clone(), None).await?);
        // A second new row for the person is refused; a change made on the version that is
        // stored is written, one made on an older version is not.
        assert!(!put(storage, &mut data.source, timer.clone(), None).await?);
        let stopped = Timer {
            running: None,
            version: FIRST_VERSION + 1,
            ..timer
        };
        let read = Some(Version::new(FIRST_VERSION));
        assert!(put(storage, &mut data.source, stopped.clone(), read).await?);
        assert!(!put(storage, &mut data.source, stopped, read).await?);
        let stored = storage.timer(data.me).await?;
        assert_eq!(
            stored.map(|found| (found.running, found.version)),
            Some((None, FIRST_VERSION + 1))
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn time_01_a_start_that_follows_the_first_add_time_still_starts_the_timer() -> TestResult {
    each_database("time_held_timer_row", async |db| {
        let fixture = fixture(db).await?;
        let mut data = seed(&fixture.storage).await;
        let storage = &fixture.storage;
        let me = data.me;
        // The first *Add time* holds the person's time: a row is made, and it is no timer.
        ChangeStore::record_change(storage, &event(&mut data.source, 8), move |tx| {
            Box::pin(async move { AuthStorage::hold_time_of(tx, me).await })
        })
        .await?;
        assert_eq!(storage.timer(me).await?, None);
        // A start that read no timer before the hold writes its new timer over the held row.
        let started = Timer {
            person: me,
            running: Some(Running {
                issue: data.issues[0].id,
                started_at: at(10),
                warned_at: None,
            }),
            changed_at: at(10),
            version: FIRST_VERSION,
        };
        assert!(put(storage, &mut data.source, started.clone(), None).await?);
        assert_eq!(storage.timer(me).await?, Some(started.clone()));
        // A second hold keeps the timer as it is, and a second new timer is still refused.
        ChangeStore::record_change(storage, &event(&mut data.source, 9), move |tx| {
            Box::pin(async move { AuthStorage::hold_time_of(tx, me).await })
        })
        .await?;
        assert!(!put(storage, &mut data.source, started.clone(), None).await?);
        assert_eq!(storage.timer(me).await?, Some(started));
        Ok(())
    })
    .await
}

#[tokio::test]
async fn time_13_field_usage_names_the_project_of_an_entry_that_holds_a_value() -> TestResult {
    use bb_app::fields::{Definition, FieldStore as _};
    use bb_core::event::{EntityKind, RecordKind};
    use bb_core::id::{EntityId, FieldId};
    use bb_core::own_fields::{FieldFor, FieldType, Look, OwnField, Rules, Scope};
    use bb_core::own_fields_value::Cell;

    each_database("time_field_usage", async |db| {
        let fixture = fixture(db).await?;
        let mut data = seed(&fixture.storage).await;
        let storage = &fixture.storage;
        let field = FieldId::generate(&mut data.source)?;
        let kind = FieldType::Text;
        let definition = Definition {
            field: OwnField {
                id: field,
                name: "Work kind".to_owned(),
                help: String::new(),
                kind,
                applies_to: FieldFor::TimeEntries,
                rules: Rules::default(),
                default_value: String::new(),
                look: Look::starting(kind),
                never_to_assistant: false,
                position: 0,
                retired_at: None,
                version: FIRST_VERSION,
            },
            scope: Scope::default(),
            choices: Vec::new(),
        };
        let booked = entry(&mut data.source, data.me, data.issues[0].id, (14, 30))?;
        let entry_id = booked.id;
        ChangeStore::record_change(storage, &event(&mut data.source, 7), move |tx| {
            Box::pin(async move {
                AuthStorage::insert_field(tx, &definition).await?;
                AuthStorage::insert_time_entry(tx, &booked).await?;
                let cells = [Cell {
                    text: Some("Review".to_owned()),
                    number: None,
                    date: None,
                    reference: None,
                }];
                AuthStorage::put_field_value(
                    tx,
                    EntityKind::from(RecordKind::TimeEntry),
                    EntityId::from(entry_id),
                    field,
                    &cells,
                )
                .await
            })
        })
        .await?;
        // Only an entry holds a value: its issue's project uses the field.
        assert_eq!(storage.projects_with_values(field).await?, [data.project]);
        Ok(())
    })
    .await
}
