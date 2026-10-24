//! The security log in the database: chain, anchor, checkpoints and pruning (SEC-15).

use bb_core::id::UserId;
use bb_core::network::NetworkPrefix;
use bb_core::security_log::Digest;
use bb_core::technical::TechnicalName;
use bb_testing::{TestResult, each_database};
use sea_query::{Expr, ExprTrait as _, Query};

use super::*;
use crate::anchor::LogSeal;
use crate::chain::{BreakReason, START, Verification};
use crate::db::Database;
use crate::error::{SecretProblem, StoreError};
use crate::keys::SigningKey;
use crate::schema::{SecurityLog as Columns, SecurityLogCheckpoint as Checkpoint};
use crate::test_support::{at, open};

fn new_entry(seconds: i64, action: &str) -> NewSecurityLogEntry {
    NewSecurityLogEntry {
        time: at(seconds),
        person: Some(UserId::from_random([3; 16])),
        action: TechnicalName::new(action).unwrap(),
        subject: None,
        network: Some(NetworkPrefix::from_address("203.0.113.9".parse().unwrap())),
    }
}

async fn append_many(database: &Database, seal: &LogSeal, count: i64) -> Result<(), StoreError> {
    for number in 1..=count {
        database
            .append_security_log(seal, &new_entry(number, "sign_in.ok"))
            .await?;
    }
    Ok(())
}

/// A seal with a key of its own and an anchor file in a folder that lives as long as the
/// returned guard.
fn new_seal() -> (tempfile::TempDir, LogSeal) {
    let folder = tempfile::tempdir().unwrap();
    std::fs::create_dir(folder.path().join("logs")).unwrap();
    let seal = LogSeal::with_key(
        SigningKey::generate().unwrap(),
        folder.path().join("logs").join("security-head.log"),
    );
    (folder, seal)
}

async fn update_entry(database: &Database, seq: i64, column: Columns, value: sea_query::Value) {
    let mut update = Query::update();
    update
        .table(Columns::Table)
        .value(column, value)
        .and_where(Expr::col(Columns::Seq).eq(seq));
    database.execute(&update).await.unwrap();
}

fn broken(seq: i64, reason: BreakReason) -> Verification {
    Verification::Broken { seq, reason }
}

#[tokio::test]
async fn sec_15_entries_are_chained_and_verify() -> TestResult {
    each_database("sec_15_chain_verifies", async |db| {
        let database = open(db).await?;
        let (_folder, seal) = new_seal();
        assert_eq!(
            database.verify_security_log(&seal).await?,
            Verification::Valid {
                entries: 0,
                head: None,
                anchored: false
            }
        );
        append_many(&database, &seal, 5).await?;
        let entries = database.security_log_entries(0, 10).await?;
        assert_eq!(
            entries.iter().map(|entry| entry.seq).collect::<Vec<_>>(),
            [1, 2, 3, 4, 5]
        );
        assert_eq!(entries[0].previous_hash, START);
        assert_eq!(entries[1].previous_hash, entries[0].hash);
        assert_eq!(entries[0].network.as_ref().unwrap().as_str(), "203.0.113.0");
        assert_eq!(
            database.verify_security_log(&seal).await?,
            Verification::Valid {
                entries: 5,
                head: Some((5, entries[4].hash)),
                anchored: true
            }
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn sec_15_an_entry_with_a_former_code_verifies_and_is_shown_with_the_new_one() -> TestResult {
    each_database("sec_15_former_codes", async |db| {
        let database = open(db).await?;
        let (_folder, seal) = new_seal();
        // The log of M3: the entries hold the words of that time.
        let former = bb_core::sign_in::FORMER_CODES;
        for (number, (old, _)) in (1..).zip(former) {
            database
                .append_security_log(&seal, &new_entry(number, old))
                .await?;
        }
        database
            .append_security_log(&seal, &new_entry(10, "sign_in.succeeded"))
            .await?;
        // The chain still verifies: no entry was rewritten.
        assert!(matches!(
            database.verify_security_log(&seal).await?,
            Verification::Valid { entries: 8, .. }
        ));
        // It is read back with the new codes, and the other codes stay as they are.
        let shown: Vec<String> = database
            .security_log_entries(0, 20)
            .await?
            .iter()
            .map(|entry| entry.action.as_str().to_owned())
            .collect();
        let wanted: Vec<&str> = former
            .iter()
            .map(|(_, new)| *new)
            .chain(["sign_in.succeeded"])
            .collect();
        assert_eq!(shown, wanted);
        // The database still holds the old words, which the hashes were made from.
        let mut select = Query::select();
        select
            .column(Columns::Action)
            .from(Columns::Table)
            .order_by(Columns::Seq, sea_query::Order::Asc);
        let rows = database.fetch_all(&select).await?;
        assert_eq!(rows[0].get::<String>(Columns::Action)?, "role.made");
        // Running the check again changes nothing.
        assert!(matches!(
            database.verify_security_log(&seal).await?,
            Verification::Valid { entries: 8, .. }
        ));
        Ok(())
    })
    .await
}

#[tokio::test]
async fn bad_example_a_changed_row_is_found_in_every_column() -> TestResult {
    each_database("sec_15_tampering", async |db| {
        let database = open(db).await?;
        let (_folder, seal) = new_seal();
        let changes: [(Columns, sea_query::Value); 4] = [
            (Columns::Time, 1_i64.into()),
            (Columns::Action, "sign_in.failed".into()),
            (Columns::Network, "10.0.0.0".into()),
            (
                Columns::Person,
                Some(UserId::from_random([9; 16]).to_bytes().to_vec()).into(),
            ),
        ];
        for (column, value) in changes {
            let mut delete = Query::delete();
            delete.from_table(Columns::Table);
            database.execute(&delete).await?;
            append_many(&database, &seal, 4).await?;
            assert!(matches!(
                database.verify_security_log(&seal).await?,
                Verification::Valid { entries: 4, .. }
            ));
            update_entry(&database, 3, column, value).await;
            assert_eq!(
                database.verify_security_log(&seal).await?,
                broken(3, BreakReason::ChangedContent)
            );
        }
        Ok(())
    })
    .await
}

#[tokio::test]
async fn sec_15_somebody_with_only_the_database_cannot_hash_the_log_again() -> TestResult {
    each_database("sec_15_rehashed_tail", async |db| {
        let database = open(db).await?;
        let (_folder, seal) = new_seal();
        append_many(&database, &seal, 5).await?;
        // Change entry 2 and make entries 2..5 fit again with a key of their own.
        let attacker = SigningKey::generate()?;
        let mut entries = database.security_log_entries(0, 10).await?;
        entries[1].action = TechnicalName::new("sign_in.failed").unwrap();
        let mut previous = entries[0].hash;
        for entry in &mut entries[1..] {
            entry.previous_hash = previous;
            entry.hash = hash_of(&attacker, &previous, entry)?;
            previous = entry.hash;
            let mut update = Query::update();
            update
                .table(Columns::Table)
                .value(Columns::Action, entry.action.as_str())
                .value(
                    Columns::PreviousHash,
                    entry.previous_hash.as_bytes().to_vec(),
                )
                .value(Columns::Hash, entry.hash.as_bytes().to_vec())
                .and_where(Expr::col(Columns::Seq).eq(entry.seq));
            database.execute(&update).await?;
        }
        assert_eq!(
            database.verify_security_log(&seal).await?,
            broken(2, BreakReason::ChangedContent)
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn sec_15_cutting_off_the_end_or_replacing_it_is_found_through_the_anchor() -> TestResult {
    each_database("sec_15_anchor", async |db| {
        let database = open(db).await?;
        let (_folder, seal) = new_seal();
        append_many(&database, &seal, 5).await?;
        let mut delete = Query::delete();
        delete
            .from_table(Columns::Table)
            .and_where(Expr::col(Columns::Seq).gt(3_i64));
        database.execute(&delete).await?;
        assert_eq!(
            database.verify_security_log(&seal).await?,
            broken(5, BreakReason::Behind)
        );

        // The whole log replaced by another one that is right under the same key (somebody
        // with the key but not the anchor file): the anchor knows the old hash.
        let mut everything = Query::delete();
        everything.from_table(Columns::Table);
        database.execute(&everything).await?;
        let (_other_folder, other) = new_seal();
        let same_key = LogSeal::with_key(seal.key().clone(), other.file.clone());
        for number in 1..=5 {
            database
                .append_security_log(&same_key, &new_entry(number + 50, "sign_in.ok"))
                .await?;
        }
        assert_eq!(
            database.verify_security_log(&seal).await?,
            broken(5, BreakReason::Truncated)
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn the_anchor_file_can_be_forged_or_lost_but_not_unnoticed() -> TestResult {
    each_database("sec_15_anchor_file", async |db| {
        let database = open(db).await?;
        let (_folder, seal) = new_seal();
        append_many(&database, &seal, 3).await?;
        // A line signed with another key.
        let stranger = LogSeal::with_key(SigningKey::generate()?, seal.file.clone());
        stranger.record(9, &Digest::new([9; 32]))?;
        assert_eq!(
            database.verify_security_log(&seal).await?,
            broken(0, BreakReason::Anchor)
        );
        // The file deleted: the log verifies but says it is not anchored.
        std::fs::remove_file(&seal.file)?;
        assert!(matches!(
            database.verify_security_log(&seal).await?,
            Verification::Valid {
                entries: 3,
                anchored: false,
                ..
            }
        ));
        Ok(())
    })
    .await
}

#[tokio::test]
async fn bad_example_a_removed_entry_leaves_a_gap_and_a_missing_start_is_found() -> TestResult {
    each_database("sec_15_removed_entries", async |db| {
        let database = open(db).await?;
        let (_folder, seal) = new_seal();
        append_many(&database, &seal, 5).await?;
        let mut delete = Query::delete();
        delete
            .from_table(Columns::Table)
            .and_where(Expr::col(Columns::Seq).eq(3_i64));
        database.execute(&delete).await?;
        assert_eq!(
            database.verify_security_log(&seal).await?,
            broken(4, BreakReason::Gap)
        );
        // The first entries removed without a checkpoint.
        let mut delete = Query::delete();
        delete
            .from_table(Columns::Table)
            .and_where(Expr::col(Columns::Seq).lte(3_i64));
        database.execute(&delete).await?;
        assert_eq!(
            database.verify_security_log(&seal).await?,
            broken(4, BreakReason::Checkpoint)
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn sec_15_old_blocks_are_deleted_and_the_rest_still_verifies() -> TestResult {
    each_database("sec_15_prune_blocks", async |db| {
        let database = open(db).await?;
        let (_folder, seal) = new_seal();
        append_many(&database, &seal, 8).await?;

        // Nothing is older than the cut-off: nothing is deleted.
        assert_eq!(database.prune_in_blocks(&seal, at(0), at(100), 3).await?, 0);
        // Entries 1..=7 are older; blocks 1-3 and 4-6 are whole, block 7-9 is not.
        assert_eq!(database.prune_in_blocks(&seal, at(8), at(100), 3).await?, 6);
        let left = database.security_log_entries(0, 10).await?;
        assert_eq!(
            left.iter().map(|entry| entry.seq).collect::<Vec<_>>(),
            [7, 8]
        );
        assert!(matches!(
            database.verify_security_log(&seal).await?,
            Verification::Valid {
                entries: 2,
                head: Some((8, _)),
                anchored: true
            }
        ));

        // Appending goes on from the end; the log still verifies.
        append_many(&database, &seal, 2).await?;
        assert_eq!(
            database
                .security_log_entries(0, 10)
                .await?
                .last()
                .unwrap()
                .seq,
            10
        );
        assert!(matches!(
            database.verify_security_log(&seal).await?,
            Verification::Valid { entries: 4, .. }
        ));
        // The newest entry always stays, even when its block is whole and old.
        assert_eq!(
            database
                .prune_in_blocks(&seal, at(1000), at(100), 3)
                .await?,
            3
        );
        assert_eq!(database.security_log_entries(0, 10).await?.len(), 1);
        assert!(matches!(
            database.verify_security_log(&seal).await?,
            Verification::Valid { entries: 1, .. }
        ));
        Ok(())
    })
    .await
}

#[tokio::test]
async fn bad_example_prune_refuses_to_delete_blocks_that_do_not_verify() -> TestResult {
    each_database("sec_15_prune_verifies", async |db| {
        let database = open(db).await?;
        let (_folder, seal) = new_seal();
        append_many(&database, &seal, 8).await?;
        let tampered = |result: Result<u64, StoreError>| {
            assert!(
                matches!(result, Err(StoreError::Secret(SecretProblem::Tampered))),
                "{result:?}"
            );
        };

        // A changed entry inside a block that would be deleted.
        update_entry(&database, 2, Columns::Action, "sign_in.failed".into()).await;
        tampered(database.prune_in_blocks(&seal, at(8), at(100), 3).await);
        assert_eq!(database.security_log_entries(0, 10).await?.len(), 8);

        // A first kept entry that does not point to the last deleted one.
        update_entry(&database, 2, Columns::Action, "sign_in.ok".into()).await;
        update_entry(&database, 7, Columns::PreviousHash, vec![1_u8; 32].into()).await;
        tampered(database.prune_in_blocks(&seal, at(8), at(100), 3).await);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn bad_example_a_forged_or_removed_checkpoint_is_found() -> TestResult {
    each_database("sec_15_forged_checkpoint", async |db| {
        let database = open(db).await?;
        let (_folder, seal) = new_seal();
        append_many(&database, &seal, 8).await?;
        database.prune_in_blocks(&seal, at(8), at(100), 3).await?;

        // Another installation's key cannot vouch for this one.
        let (_other_folder, other) = new_seal();
        assert_eq!(
            database.verify_security_log(&other).await?,
            broken(6, BreakReason::Checkpoint)
        );
        assert!(
            database
                .prune_in_blocks(&other, at(1000), at(100), 3)
                .await
                .is_err()
        );

        // A moved checkpoint, or a changed time, no longer matches its signature.
        let mut update = Query::update();
        update
            .table(Checkpoint::Table)
            .value(Checkpoint::CreatedAt, 5_i64);
        database.execute(&update).await?;
        assert_eq!(
            database.verify_security_log(&seal).await?,
            broken(6, BreakReason::Checkpoint)
        );
        let mut update = Query::update();
        update
            .table(Checkpoint::Table)
            .value(Checkpoint::Seq, 5_i64);
        database.execute(&update).await?;
        assert_eq!(
            database.verify_security_log(&seal).await?,
            broken(5, BreakReason::Checkpoint)
        );

        // No checkpoint at all, but the first entries are gone.
        let mut delete = Query::delete();
        delete.from_table(Checkpoint::Table);
        database.execute(&delete).await?;
        assert_eq!(
            database.verify_security_log(&seal).await?,
            broken(7, BreakReason::Checkpoint)
        );
        Ok(())
    })
    .await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn writers_at_the_same_time_extend_one_chain_and_verify_needs_no_write_lock() -> TestResult {
    each_database("sec_15_concurrent_appends", async |db| {
        let database = open(db).await?;
        let (_folder, seal) = new_seal();
        let mut tasks = Vec::new();
        for number in 0..8 {
            let database = database.clone();
            let seal = seal.clone();
            tasks.push(tokio::spawn(async move {
                database
                    .append_security_log(&seal, &new_entry(number, "sign_in.ok"))
                    .await
            }));
        }
        for task in tasks {
            task.await??;
        }
        assert!(matches!(
            database.verify_security_log(&seal).await?,
            Verification::Valid {
                entries: 8,
                head: Some((8, _)),
                anchored: true
            }
        ));
        // A writer holds the write lock: verifying still works (one snapshot, no lock).
        let writer = database.begin().await?;
        assert!(matches!(
            database.verify_security_log(&seal).await?,
            Verification::Valid { entries: 8, .. }
        ));
        drop(writer);
        Ok(())
    })
    .await
}
/// The place and hash of the newest entry, as a check would keep them.
async fn head_of(database: &Database) -> Result<(i64, Digest), StoreError> {
    let entries = database.security_log_entries(0, 100).await?;
    let last = entries.last().ok_or(StoreError::Corrupt("no entry"))?;
    Ok((last.seq, last.hash))
}

#[tokio::test]
async fn ops_08_the_status_page_checks_only_new_entries_and_the_command_checks_all() -> TestResult {
    each_database("ops_08_incremental_check", async |db| {
        let database = open(db).await?;
        let (_folder, seal) = new_seal();
        append_many(&database, &seal, 4).await?;

        // A check from a head that an earlier check found right reads what came after it.
        let known = head_of(&database).await?;
        assert_eq!(
            database
                .verify_security_log_since(&seal, Some(known))
                .await?,
            Verification::Valid {
                entries: 4,
                head: Some(known),
                anchored: true
            }
        );
        append_many(&database, &seal, 2).await?;
        let newest = head_of(&database).await?;
        assert_eq!(
            database
                .verify_security_log_since(&seal, Some(known))
                .await?,
            Verification::Valid {
                entries: 6,
                head: Some(newest),
                anchored: true
            }
        );
        // Without a known head, everything is read.
        assert!(matches!(
            database.verify_security_log_since(&seal, None).await?,
            Verification::Valid { entries: 6, .. }
        ));

        // An old entry is changed. The check from the known head does not read it, so the
        // status page does not see it; the full check, which the command runs, does.
        update_entry(&database, 2, Columns::Action, "sign_in.failed".into()).await;
        assert!(matches!(
            database
                .verify_security_log_since(&seal, Some(known))
                .await?,
            Verification::Valid { entries: 6, .. }
        ));
        assert_eq!(
            database.verify_security_log(&seal).await?,
            broken(2, BreakReason::ChangedContent)
        );
        assert_eq!(
            database.verify_security_log_since(&seal, None).await?,
            broken(2, BreakReason::ChangedContent)
        );

        // A change in the entries after the known head is found by both.
        update_entry(&database, 5, Columns::Action, "sign_in.failed".into()).await;
        assert_eq!(
            database
                .verify_security_log_since(&seal, Some(known))
                .await?,
            broken(5, BreakReason::ChangedContent)
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn ops_08_a_known_head_that_the_log_no_longer_has_makes_a_full_check() -> TestResult {
    each_database("ops_08_incremental_check_lost_head", async |db| {
        let database = open(db).await?;
        let (_folder, seal) = new_seal();
        append_many(&database, &seal, 5).await?;
        let known = head_of(&database).await?;

        // The head was changed: the old head is not trusted, and the whole log is checked.
        update_entry(&database, 5, Columns::Action, "sign_in.failed".into()).await;
        assert_eq!(
            database
                .verify_security_log_since(&seal, Some(known))
                .await?,
            broken(5, BreakReason::ChangedContent)
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn ops_08_a_known_head_before_the_checkpoint_makes_a_full_check() -> TestResult {
    each_database("ops_08_incremental_check_pruned", async |db| {
        let database = open(db).await?;
        let (_folder, seal) = new_seal();
        append_many(&database, &seal, 8).await?;
        let head = head_of(&database).await?;
        database.prune_in_blocks(&seal, at(8), at(100), 3).await?;
        // A head from before the deleted entries cannot be followed: the answer is the one
        // of the full check.
        let early = (2, Digest::new([7; 32]));
        assert_eq!(
            database
                .verify_security_log_since(&seal, Some(early))
                .await?,
            database.verify_security_log(&seal).await?
        );
        // A head after the checkpoint still is.
        assert_eq!(
            database
                .verify_security_log_since(&seal, Some(head))
                .await?,
            Verification::Valid {
                entries: 2,
                head: Some(head),
                anchored: true
            }
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn sec_15_an_anchor_that_is_ahead_of_the_database_has_a_reason_of_its_own() -> TestResult {
    each_database("sec_15_anchor_ahead", async |db| {
        let database = open(db).await?;
        let (_folder, seal) = new_seal();
        append_many(&database, &seal, 5).await?;
        // The database goes back to an earlier state (a restore from an older backup): the
        // anchor file knows of entries that it does not have.
        let mut delete = Query::delete();
        delete
            .from_table(Columns::Table)
            .and_where(Expr::col(Columns::Seq).gt(3_i64));
        database.execute(&delete).await?;
        assert_eq!(
            database.verify_security_log(&seal).await?,
            broken(5, BreakReason::Behind)
        );
        // A check from the head that is left finds the same.
        let known = head_of(&database).await?;
        assert_eq!(
            database
                .verify_security_log_since(&seal, Some(known))
                .await?,
            broken(5, BreakReason::Behind)
        );
        Ok(())
    })
    .await
}
