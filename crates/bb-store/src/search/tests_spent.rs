//! Time spent through the one expression of the compiler, on both databases (ADR-0035 decision 4,
//! ARCH-DB-PARITY): the minutes of each issue of a list, the sum over the issues a search finds with
//! the reader's sight in it, a person's minutes for each date, and one timer row for each person.

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
