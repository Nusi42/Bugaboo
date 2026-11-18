//! The purge of the trash beside a person who takes an issue out of it, on both databases (PRIV-05,
//! ADR-0045): the purge chooses and holds its issues in one statement that checks again, on the row
//! as it is when the purge gets it, that the issue is still in the trash.

use std::time::Duration;

use bb_app::issues::IssueStore as _;
use bb_app::ports::ChangeStore;
use bb_testing::{TestResult, each_database};
use sea_query::{Alias, Expr, ExprTrait as _, Query};

use super::*;
use crate::AuthStorage;
use crate::db::Dialect;
use crate::schema::Computed;
use crate::test_support::{at, event, fixture, ids, world};

/// How often the test looks whether the purge waits for the lock of the restore.
const LOOK_EVERY: Duration = Duration::from_millis(10);

/// How many sessions of PostgreSQL on the database of the test wait for a lock on a row: the
/// databases of other tests that run at the same moment do not count.
async fn waiting_for_a_lock(database: &Database) -> Result<i64, StoreError> {
    let mut select = Query::select();
    select
        .expr_as(Expr::cust("COUNT(*)"), Computed::Total)
        .from(Alias::new("pg_stat_activity"))
        .and_where(Expr::col(Alias::new("wait_event_type")).eq("Lock"))
        .and_where(Expr::col(Alias::new("datname")).eq(Expr::cust("current_database()")));
    match database.fetch_optional(&select).await? {
        Some(row) => row.get(Computed::Total),
        None => Ok(0),
    }
}

#[tokio::test]
async fn priv_05_an_issue_taken_out_of_the_trash_while_the_purge_waits_is_kept() -> TestResult {
    each_database("priv_05_trash_restore_race", async |db| {
        let fixture = fixture(db).await?;
        let storage = fixture.storage.clone();
        let mut source = ids();
        let world = world(&storage, &mut source, 1).await;
        let issue = world.issues[0].id;
        ChangeStore::record_change(&storage, &event(&mut source, 5), move |tx| {
            Box::pin(async move {
                AuthStorage::set_issue_deleted(tx, issue, Some(at(40)), at(40), None).await
            })
        })
        .await?;
        // A person takes the issue out of the trash; the change is not committed yet.
        let mut restoring = storage.database().begin().await?;
        assert!(AuthStorage::set_issue_deleted(&mut restoring, issue, None, at(50), None).await?);
        let purging = {
            let storage = storage.clone();
            tokio::spawn(async move { storage.purge_trashed_issues(at(100), 10).await })
        };
        // On PostgreSQL the purge has chosen the issue and waits for the lock of the restore. SQLite
        // has one writer at a time: the purge cannot start before the restore is committed.
        if storage.database().dialect() == Dialect::Postgres {
            while waiting_for_a_lock(storage.database()).await? == 0 {
                assert!(
                    !purging.is_finished(),
                    "the purge finished without waiting for the lock of the restore"
                );
                tokio::time::sleep(LOOK_EVERY).await;
            }
        }
        restoring.commit().await?;
        assert_eq!(purging.await??, 0, "the restored issue was purged");
        let kept = storage.issue(issue).await?.ok_or("the issue went")?;
        assert!(kept.deleted_at.is_none());
        Ok(())
    })
    .await
}
