//! The records an administrator deletes after their keep period, on both databases (PRIV-05,
//! ADR-0045, decision 6): a step deletes at most its limit, counts what it deleted, and takes the
//! notifications about the records along.

use bb_app::keeping::{OldCount, OldRecordStore as _};
use bb_app::notifications::NotificationStore as _;
use bb_app::ports::ChangeStore;
use bb_app::time::TimeStore as _;
use bb_core::id::{IdSource, IssueId, TimeEntryId, UserId};
use bb_core::keeping::KeepKind;
use bb_core::time::Minutes;
use bb_core::time::entry::TimeEntry;
use bb_core::version::FIRST_VERSION;
use bb_testing::{TestResult, each_database};
use jiff::civil::Date;

use super::*;
use crate::AuthStorage;
use crate::notifications::tests::{row, stored_person};
use crate::test_support::{at, event, fixture, ids, world};

/// A time entry of `person` on `issue` on `date`.
fn entry(source: &mut impl IdSource, person: UserId, issue: IssueId, date: Date) -> TimeEntry {
    TimeEntry {
        id: TimeEntryId::generate(source).unwrap(),
        person,
        issue,
        date,
        minutes: Minutes::new(30),
        note: String::new(),
        billable: false,
        started_at: None,
        created_at: at(1),
        version: FIRST_VERSION,
    }
}

/// One step of *Delete them* of time entries before `before`, at most `limit`.
async fn step(
    storage: &AuthStorage,
    before: Date,
    limit: u64,
) -> Result<Vec<OldCount>, StoreError> {
    ChangeStore::record_changes(storage, &[], move |tx| {
        Box::pin(async move {
            AuthStorage::delete_old_records(tx, KeepKind::TimeEntries, before, None, limit).await
        })
    })
    .await
}

#[tokio::test]
async fn priv_05_old_time_entries_go_in_steps_counted_with_their_notifications() -> TestResult {
    each_database("priv_05_old_entries_in_steps", async |db| {
        let fixture = fixture(db).await?;
        let storage = fixture.storage.clone();
        let mut source = ids();
        let world = world(&storage, &mut source, 1).await;
        let person = stored_person(&storage, &mut source, "ana@example.test").await?;
        let issue = world.issues[0].id;
        let old = Date::new(2000, 1, 3)?;
        let entries: Vec<TimeEntry> = (0..3)
            .map(|_| entry(&mut source, person, issue, old))
            .collect();
        let kept = entry(&mut source, person, issue, Date::new(2030, 1, 3)?);
        let written: Vec<TimeEntry> = entries.iter().chain([&kept]).cloned().collect();
        ChangeStore::record_change(&storage, &event(&mut source, 5), move |tx| {
            Box::pin(async move {
                for entry in &written {
                    AuthStorage::insert_time_entry(tx, entry).await?;
                }
                Ok(())
            })
        })
        .await?;
        // A notification about one of the old entries.
        let mut about = row(&mut source, person, Some(issue), 10)?;
        about.record = Some(entries[0].id.into());
        storage
            .store_notifications(&[about], &[], 0, at(11))
            .await?;

        let before = Date::new(2001, 1, 1)?;
        let counts = |found: &[OldCount]| found.iter().map(|old| old.count).sum::<u64>();
        assert_eq!(counts(&step(&storage, before, 2).await?), 2);
        assert_eq!(counts(&step(&storage, before, 2).await?), 1);
        assert_eq!(counts(&step(&storage, before, 2).await?), 0, "safe again");
        for gone in &entries {
            assert!(storage.time_entry(gone.id).await?.is_none());
        }
        assert!(
            storage.time_entry(kept.id).await?.is_some(),
            "a young entry stays"
        );
        assert!(
            storage
                .database()
                .stored_notifications(person)
                .await?
                .is_empty(),
            "the notification about the entry went with it"
        );
        Ok(())
    })
    .await
}
