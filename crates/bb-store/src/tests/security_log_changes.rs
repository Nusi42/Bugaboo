//! Entries of the security log written as part of a change (SEC-15): committed with the change or
//! not at all, and in one chain with the entries written alone, also when both are written at once.

use bb_core::id::UserId;
use bb_testing::{TestResult, each_database};

use super::tests::{append_many, new_entry, new_seal};
use super::*;
use crate::chain::Verification;
use crate::test_support::{at, open};

#[tokio::test]
async fn sec_15_an_entry_of_a_change_is_committed_with_it_or_not_at_all() -> TestResult {
    use std::sync::Arc;

    use bb_app::auth::{AuthStore as _, SecurityEvent};
    use bb_app::ports::ChangeStore;
    use bb_core::sign_in::SecurityAction;

    use crate::AuthStorage;

    each_database("sec_15_entry_of_a_change", async |db| {
        let database = open(db).await?;
        let (folder, seal) = new_seal();
        let keys = Arc::new(database.open_keys(&folder.path().join("keys")).await?);
        let storage = AuthStorage::new(database.clone(), keys, seal.clone());
        let line = SecurityEvent {
            time: at(2),
            person: Some(UserId::from_random([3; 16])),
            action: SecurityAction::TimeEntriesDeleted,
            subject: None,
            network: None,
        };
        let change = |fails: bool| {
            let line = line.clone();
            ChangeStore::record_changes(&storage, &[], move |tx| {
                Box::pin(async move {
                    AuthStorage::append_security_log_in_change(tx, &line).await?;
                    if fails {
                        return Err(StoreError::Aborted);
                    }
                    Ok(())
                })
            })
        };
        // Between entries written alone, the entry of a change takes the next place.
        append_many(&database, &seal, 1).await?;
        change(false).await?;
        database
            .append_security_log(&seal, &new_entry(3, "sign_in.ok"))
            .await?;
        // A change that fails leaves no entry.
        assert!(change(true).await.is_err());
        let entries = database.security_log_entries(0, 10).await?;
        assert_eq!(
            entries.iter().map(|entry| entry.seq).collect::<Vec<_>>(),
            [1, 2, 3]
        );
        assert_eq!(
            entries[1].action.as_str(),
            SecurityAction::TimeEntriesDeleted.code()
        );
        // The chain holds, and the anchor file knows the end.
        assert_eq!(
            database.verify_security_log(&seal).await?,
            Verification::Valid {
                entries: 3,
                head: Some((3, entries[2].hash)),
                anchored: true
            }
        );
        // The entry of the last change is anchored too.
        change(false).await?;
        let entries = database.security_log_entries(0, 10).await?;
        assert_eq!(
            database.verify_security_log(&seal).await?,
            Verification::Valid {
                entries: 4,
                head: Some((4, entries[3].hash)),
                anchored: true
            }
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn sec_15_entries_of_changes_and_entries_alone_written_at_once_keep_the_chain_whole()
-> TestResult {
    use std::sync::Arc;

    use bb_app::auth::{AuthStore as _, SecurityEvent};
    use bb_app::ports::ChangeStore;
    use bb_core::sign_in::SecurityAction;

    use crate::AuthStorage;

    /// How many writers of each kind run at once, and how many entries each writes.
    const WRITERS: i64 = 4;
    const EACH: i64 = 10;

    each_database("sec_15_entries_at_once", async |db| {
        let database = open(db).await?;
        let (folder, seal) = new_seal();
        let keys = Arc::new(database.open_keys(&folder.path().join("keys")).await?);
        let storage = AuthStorage::new(database.clone(), keys, seal.clone());
        let mut writers = tokio::task::JoinSet::new();
        for writer in 0..WRITERS {
            let storage = storage.clone();
            writers.spawn(async move {
                for number in 0..EACH {
                    let line = SecurityEvent {
                        time: at(writer * EACH + number),
                        person: None,
                        action: SecurityAction::TimeEntriesDeleted,
                        subject: None,
                        network: None,
                    };
                    ChangeStore::record_changes(&storage, &[], move |tx| {
                        Box::pin(async move {
                            AuthStorage::append_security_log_in_change(tx, &line).await
                        })
                    })
                    .await?;
                }
                Ok::<_, StoreError>(())
            });
            let (database, seal) = (database.clone(), seal.clone());
            writers.spawn(async move {
                for number in 0..EACH {
                    database
                        .append_security_log(
                            &seal,
                            &new_entry(writer * EACH + number, "sign_in.ok"),
                        )
                        .await?;
                }
                Ok(())
            });
        }
        while let Some(written) = writers.join_next().await {
            written??;
        }
        let all = 2 * WRITERS * EACH;
        let entries = database
            .security_log_entries(0, u64::try_from(all)? + 1)
            .await?;
        assert_eq!(
            entries.iter().map(|entry| entry.seq).collect::<Vec<_>>(),
            (1..=all).collect::<Vec<_>>()
        );
        assert_eq!(
            database.verify_security_log(&seal).await?,
            Verification::Valid {
                entries: u64::try_from(all)?,
                head: Some((all, entries[usize::try_from(all - 1)?].hash)),
                anchored: true
            }
        );
        Ok(())
    })
    .await
}
