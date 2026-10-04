//! The security log (SEC-15) in the database: appending, verifying and deleting old blocks.
//!
//! Entries are only ever added. After the keep period an administrator-set job deletes the oldest
//! *whole blocks* of [`BLOCK_SIZE`] entries and leaves one signed checkpoint behind: the place
//! and hash of the last deleted entry, signed with a key derived from the master key. The
//! entries that remain still verify, because the first of them points to the checkpoint's hash.
//! A checkpoint that is missing or forged, or entries missing without one, show as a break.

use bb_core::event::{EntityKind, EntityRef};
use bb_core::id::{EntityId, UserId};
use bb_core::network::NetworkPrefix;
use bb_core::security_log::{Digest, SecurityLogCheckpoint, SecurityLogEntry};
use bb_core::technical::TechnicalName;
use jiff::Timestamp;
use sea_query::{Expr, ExprTrait as _, Order, Query};

use crate::chain::{
    BLOCK_SIZE, BreakReason, CHECKPOINT_DOMAIN, START, Verification, Verifier, checkpoint_message,
    hash_of,
};
use crate::db::{Database, Row, Transaction, to_micros};
use crate::error::{SecretProblem, StoreError};
use crate::keys::KeyRing;
use crate::schema::{Computed, SecurityLog as Columns, SecurityLogCheckpoint as Checkpoint};

/// How often an append starts over when another writer was faster.
const APPEND_ATTEMPTS: u32 = 8;
/// How many entries are read at a time when verifying.
const PAGE: u64 = 500;

/// Something security-relevant that happened.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NewSecurityLogEntry {
    /// When it happened.
    pub time: Timestamp,
    /// Who did it, if somebody was signed in.
    pub person: Option<UserId>,
    /// What happened, like `sign_in.failed`.
    pub action: TechnicalName,
    /// The record it was about, if one.
    pub subject: Option<EntityRef>,
    /// Where it came from, shortened.
    pub network: Option<NetworkPrefix>,
}

impl Database {
    /// Adds an entry to the end of the chain and returns it with its place and hash. When two
    /// writers add at the same moment, one of them starts over.
    ///
    /// # Errors
    /// Fails with [`StoreError::Contended`] after repeated collisions, and when the database
    /// fails.
    pub async fn append_security_log(
        &self,
        new: &NewSecurityLogEntry,
    ) -> Result<SecurityLogEntry, StoreError> {
        for _ in 0..APPEND_ATTEMPTS {
            match self.try_append(new).await {
                Err(error) if error.is_duplicate() => {}
                other => return other,
            }
        }
        Err(StoreError::Contended)
    }

    async fn try_append(&self, new: &NewSecurityLogEntry) -> Result<SecurityLogEntry, StoreError> {
        let mut transaction = self.begin().await?;
        let (seq, previous) = match last_entry(&mut transaction).await? {
            Some((seq, hash)) => (seq + 1, hash),
            None => match read_checkpoint(&mut transaction).await? {
                Some(checkpoint) => (checkpoint.seq + 1, checkpoint.hash),
                None => (1, START),
            },
        };
        let mut entry = SecurityLogEntry {
            seq,
            time: new.time,
            person: new.person,
            action: new.action.clone(),
            subject: new.subject.clone(),
            network: new.network.clone(),
            previous_hash: previous,
            hash: START,
        };
        entry.hash = hash_of(&previous, &entry);

        let mut insert = Query::insert();
        insert
            .into_table(Columns::Table)
            .columns(entry_columns())
            .values([
                Expr::val(entry.seq),
                Expr::val(to_micros(entry.time)),
                Expr::val(entry.person.map(|person| person.to_bytes().to_vec())),
                Expr::val(entry.action.as_str()),
                Expr::val(entry.subject.as_ref().map(|subject| subject.kind.as_str())),
                Expr::val(
                    entry
                        .subject
                        .as_ref()
                        .map(|subject| subject.id.to_bytes().to_vec()),
                ),
                Expr::val(entry.network.as_ref().map(NetworkPrefix::as_str)),
                Expr::val(entry.previous_hash.as_bytes().to_vec()),
                Expr::val(entry.hash.as_bytes().to_vec()),
            ])
            .map_err(|_| StoreError::Corrupt("a security log entry"))?;
        transaction.execute(&insert).await?;
        transaction.commit().await?;
        Ok(entry)
    }

    /// Up to `limit` entries after place `after_seq`, oldest first (0 for the start).
    ///
    /// # Errors
    /// Fails when the database fails or an entry is unreadable.
    pub async fn security_log_entries(
        &self,
        after_seq: i64,
        limit: u64,
    ) -> Result<Vec<SecurityLogEntry>, StoreError> {
        let mut select = Query::select();
        select
            .columns(entry_columns())
            .from(Columns::Table)
            .and_where(Expr::col(Columns::Seq).gt(after_seq))
            .order_by(Columns::Seq, Order::Asc)
            .limit(limit);
        self.fetch_all(&select)
            .await?
            .iter()
            .map(read_entry)
            .collect()
    }

    /// Checks the whole chain, including the checkpoint, if there is one. This is what
    /// `bugaboo security-log verify` and the status page run.
    ///
    /// # Errors
    /// Fails when the database fails or an entry is unreadable. A broken chain is not an error;
    /// it is the answer ([`Verification::Broken`]).
    pub async fn verify_security_log(&self, keys: &KeyRing) -> Result<Verification, StoreError> {
        let mut transaction = self.begin().await?;
        let checkpoint = read_checkpoint(&mut transaction).await?;
        transaction.commit().await?;
        let mut verifier = if let Some(checkpoint) = &checkpoint {
            if !checkpoint_is_signed(keys, checkpoint) {
                return Ok(Verification::Broken {
                    seq: checkpoint.seq,
                    reason: BreakReason::Checkpoint,
                });
            }
            Verifier::new(checkpoint.seq + 1, checkpoint.hash)
        } else {
            // Without a checkpoint the log must start at 1; a later start means entries were
            // deleted without leaving one.
            if let Some(first) = self.security_log_entries(0, 1).await?.first()
                && first.seq != 1
            {
                return Ok(Verification::Broken {
                    seq: first.seq,
                    reason: BreakReason::Checkpoint,
                });
            }
            Verifier::new(1, START)
        };
        let mut after = checkpoint.as_ref().map_or(0, |checkpoint| checkpoint.seq);
        loop {
            let page = self.security_log_entries(after, PAGE).await?;
            let Some(last) = page.last() else {
                break;
            };
            after = last.seq;
            for entry in &page {
                if let Err(reason) = verifier.check(entry) {
                    return Ok(Verification::Broken {
                        seq: entry.seq,
                        reason,
                    });
                }
            }
        }
        Ok(verifier.finish())
    }

    /// Deletes the oldest whole blocks whose entries are all older than `before`, and signs a
    /// checkpoint for what was deleted. At least one newer entry always stays. Returns how many
    /// entries were deleted.
    ///
    /// # Errors
    /// Fails when the database fails or the existing checkpoint does not verify (nothing is
    /// deleted then).
    pub async fn prune_security_log(
        &self,
        keys: &KeyRing,
        before: Timestamp,
        now: Timestamp,
    ) -> Result<u64, StoreError> {
        self.prune_in_blocks(keys, before, now, BLOCK_SIZE).await
    }

    /// [`Self::prune_security_log`] with a block size; the tests use a small one. The size must
    /// stay the same for the life of a log, because blocks are counted from its first entry.
    pub(crate) async fn prune_in_blocks(
        &self,
        keys: &KeyRing,
        before: Timestamp,
        now: Timestamp,
        block_size: i64,
    ) -> Result<u64, StoreError> {
        let mut transaction = self.begin().await?;
        let checkpoint = read_checkpoint(&mut transaction).await?;
        if let Some(checkpoint) = &checkpoint
            && !checkpoint_is_signed(keys, checkpoint)
        {
            return Err(StoreError::Secret(SecretProblem::Tampered));
        }
        let Some((head, _)) = last_entry(&mut transaction).await? else {
            return Ok(0);
        };
        let mut next_first = checkpoint
            .as_ref()
            .map_or(1, |checkpoint| checkpoint.seq + 1);

        let block = Expr::col(Columns::Seq).sub(1_i64).div(block_size);
        let mut blocks = Query::select();
        blocks
            .expr_as(block, Computed::Block)
            .expr_as(Expr::col(Columns::Seq).min(), Computed::First)
            .expr_as(Expr::col(Columns::Seq).max(), Computed::Last)
            .expr_as(Expr::col(Columns::Time).max(), Computed::Newest)
            .from(Columns::Table)
            .add_group_by([Expr::col(Computed::Block)])
            .order_by(Computed::Block, Order::Asc);
        let mut delete_up_to = None;
        for row in transaction.fetch_all(&blocks).await? {
            let first: i64 = row.get(Computed::First)?;
            let last: i64 = row.get(Computed::Last)?;
            let newest: i64 = row.get(Computed::Newest)?;
            let whole = first == next_first && last - first + 1 == block_size;
            if !(whole && last < head && newest < to_micros(before)) {
                break;
            }
            delete_up_to = Some(last);
            next_first = last + 1;
        }
        let Some(last) = delete_up_to else {
            return Ok(0);
        };

        let mut select = Query::select();
        select
            .column(Columns::Hash)
            .from(Columns::Table)
            .and_where(Expr::col(Columns::Seq).eq(last));
        let row = transaction
            .fetch_optional(&select)
            .await?
            .ok_or(StoreError::Corrupt("a security log entry"))?;
        let hash = digest(&row, Columns::Hash)?;
        let signature = sign_checkpoint(keys, last, &hash)?;

        let mut delete = Query::delete();
        delete
            .from_table(Columns::Table)
            .and_where(Expr::col(Columns::Seq).lte(last));
        let deleted = transaction.execute(&delete).await?;
        replace_checkpoint(
            &mut transaction,
            &SecurityLogCheckpoint {
                seq: last,
                hash,
                signature,
                created_at: now,
            },
        )
        .await?;
        transaction.commit().await?;
        Ok(deleted)
    }
}

/// Signs the existing checkpoint again with the newest key, after a rotation added one.
pub(crate) async fn resign_checkpoint(
    transaction: &mut Transaction,
    keys: &KeyRing,
) -> Result<(), StoreError> {
    let Some(checkpoint) = read_checkpoint(transaction).await? else {
        return Ok(());
    };
    if !checkpoint_is_signed(keys, &checkpoint) {
        return Err(StoreError::Secret(SecretProblem::Tampered));
    }
    let signature = sign_checkpoint(keys, checkpoint.seq, &checkpoint.hash)?;
    replace_checkpoint(
        transaction,
        &SecurityLogCheckpoint {
            signature,
            ..checkpoint
        },
    )
    .await
}

fn sign_checkpoint(keys: &KeyRing, seq: i64, hash: &Digest) -> Result<Digest, StoreError> {
    let key = keys.newest()?.signing_key()?;
    let signature = key.sign(&[CHECKPOINT_DOMAIN, &checkpoint_message(seq, hash)])?;
    Ok(Digest::new(signature))
}

/// Whether any key of the ring signed this checkpoint.
fn checkpoint_is_signed(keys: &KeyRing, checkpoint: &SecurityLogCheckpoint) -> bool {
    let message = checkpoint_message(checkpoint.seq, &checkpoint.hash);
    keys.all().iter().any(|key| {
        key.signing_key().is_ok_and(|signing| {
            signing.verify(
                &[CHECKPOINT_DOMAIN, &message],
                checkpoint.signature.as_bytes(),
            )
        })
    })
}

async fn replace_checkpoint(
    transaction: &mut Transaction,
    checkpoint: &SecurityLogCheckpoint,
) -> Result<(), StoreError> {
    let mut delete = Query::delete();
    delete.from_table(Checkpoint::Table);
    transaction.execute(&delete).await?;
    let mut insert = Query::insert();
    insert
        .into_table(Checkpoint::Table)
        .columns([
            Checkpoint::Seq,
            Checkpoint::Hash,
            Checkpoint::Signature,
            Checkpoint::CreatedAt,
        ])
        .values([
            Expr::val(checkpoint.seq),
            Expr::val(checkpoint.hash.as_bytes().to_vec()),
            Expr::val(checkpoint.signature.as_bytes().to_vec()),
            Expr::val(to_micros(checkpoint.created_at)),
        ])
        .map_err(|_| StoreError::Corrupt("a checkpoint"))?;
    transaction.execute(&insert).await?;
    Ok(())
}

async fn read_checkpoint(
    transaction: &mut Transaction,
) -> Result<Option<SecurityLogCheckpoint>, StoreError> {
    let mut select = Query::select();
    select
        .columns([
            Checkpoint::Seq,
            Checkpoint::Hash,
            Checkpoint::Signature,
            Checkpoint::CreatedAt,
        ])
        .from(Checkpoint::Table)
        .order_by(Checkpoint::Seq, Order::Desc)
        .limit(1);
    transaction
        .fetch_optional(&select)
        .await?
        .map(|row| {
            Ok(SecurityLogCheckpoint {
                seq: row.get(Checkpoint::Seq)?,
                hash: digest(&row, Checkpoint::Hash)?,
                signature: digest(&row, Checkpoint::Signature)?,
                created_at: row.time(Checkpoint::CreatedAt)?,
            })
        })
        .transpose()
}

/// The place and hash of the newest entry.
async fn last_entry(transaction: &mut Transaction) -> Result<Option<(i64, Digest)>, StoreError> {
    let mut select = Query::select();
    select
        .columns([Columns::Seq, Columns::Hash])
        .from(Columns::Table)
        .order_by(Columns::Seq, Order::Desc)
        .limit(1);
    transaction
        .fetch_optional(&select)
        .await?
        .map(|row| Ok((row.get(Columns::Seq)?, digest(&row, Columns::Hash)?)))
        .transpose()
}

fn digest(row: &Row, column: impl sea_query::Iden + Copy) -> Result<Digest, StoreError> {
    let bytes: Vec<u8> = row.get(column)?;
    Digest::from_slice(&bytes).ok_or(StoreError::Corrupt("a hash"))
}

fn entry_columns() -> [Columns; 9] {
    [
        Columns::Seq,
        Columns::Time,
        Columns::Person,
        Columns::Action,
        Columns::SubjectKind,
        Columns::SubjectId,
        Columns::Network,
        Columns::PreviousHash,
        Columns::Hash,
    ]
}

fn read_entry(row: &Row) -> Result<SecurityLogEntry, StoreError> {
    let subject = match (
        row.get::<Option<String>>(Columns::SubjectKind)?,
        row.optional_id_bytes(Columns::SubjectId)?,
    ) {
        (Some(kind), Some(id)) => Some(EntityRef {
            kind: EntityKind::new(&kind).map_err(|_| StoreError::Corrupt("a kind of record"))?,
            id: EntityId::from_bytes(id).map_err(|_| StoreError::Corrupt("a record ID"))?,
        }),
        (None, None) => None,
        _ => return Err(StoreError::Corrupt("a subject")),
    };
    Ok(SecurityLogEntry {
        seq: row.get(Columns::Seq)?,
        time: row.time(Columns::Time)?,
        person: row
            .optional_id_bytes(Columns::Person)?
            .map(|bytes| UserId::from_bytes(bytes).map_err(|_| StoreError::Corrupt("a person ID")))
            .transpose()?,
        action: TechnicalName::new(&row.get::<String>(Columns::Action)?)
            .map_err(|_| StoreError::Corrupt("an action"))?,
        subject,
        network: row
            .get::<Option<String>>(Columns::Network)?
            .map(|text| NetworkPrefix::from_stored(&text).ok_or(StoreError::Corrupt("a network")))
            .transpose()?,
        previous_hash: digest(row, Columns::PreviousHash)?,
        hash: digest(row, Columns::Hash)?,
    })
}

#[cfg(test)]
mod tests {
    use bb_testing::{TestResult, each_database};
    use sea_query::Query;

    use super::*;
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

    async fn append_many(database: &Database, count: i64) -> Result<(), StoreError> {
        for number in 1..=count {
            database
                .append_security_log(&new_entry(number, "sign_in.ok"))
                .await?;
        }
        Ok(())
    }

    fn keys() -> (tempfile::TempDir, KeyRing) {
        let folder = tempfile::tempdir().unwrap();
        let ring = KeyRing::open_or_create(folder.path()).unwrap();
        (folder, ring)
    }

    #[tokio::test]
    async fn sec_15_entries_are_chained_and_verify() -> TestResult {
        each_database("sec_15_chain_verifies", async |db| {
            let database = open(db).await?;
            let (_folder, ring) = keys();
            assert_eq!(
                database.verify_security_log(&ring).await?,
                Verification::Valid {
                    entries: 0,
                    head: None
                }
            );
            append_many(&database, 5).await?;
            let entries = database.security_log_entries(0, 10).await?;
            assert_eq!(
                entries.iter().map(|entry| entry.seq).collect::<Vec<_>>(),
                [1, 2, 3, 4, 5]
            );
            assert_eq!(entries[0].previous_hash, START);
            assert_eq!(entries[1].previous_hash, entries[0].hash);
            assert_eq!(entries[0].network.as_ref().unwrap().as_str(), "203.0.113.0");
            assert_eq!(
                database.verify_security_log(&ring).await?,
                Verification::Valid {
                    entries: 5,
                    head: Some((5, entries[4].hash))
                }
            );
            Ok(())
        })
        .await
    }

    #[tokio::test]
    async fn bad_example_a_changed_row_is_found_in_every_column() -> TestResult {
        each_database("sec_15_tampering", async |db| {
            let database = open(db).await?;
            let (_folder, ring) = keys();
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
                append_many(&database, 4).await?;
                assert!(matches!(
                    database.verify_security_log(&ring).await?,
                    Verification::Valid { entries: 4, .. }
                ));
                let mut update = Query::update();
                update
                    .table(Columns::Table)
                    .value(column, value)
                    .and_where(Expr::col(Columns::Seq).eq(3_i64));
                database.execute(&update).await?;
                assert_eq!(
                    database.verify_security_log(&ring).await?,
                    Verification::Broken {
                        seq: 3,
                        reason: BreakReason::ChangedContent
                    }
                );
            }
            Ok(())
        })
        .await
    }

    #[tokio::test]
    async fn bad_example_a_removed_entry_leaves_a_gap_and_a_missing_start_is_found() -> TestResult {
        each_database("sec_15_removed_entries", async |db| {
            let database = open(db).await?;
            let (_folder, ring) = keys();
            append_many(&database, 5).await?;
            let mut delete = Query::delete();
            delete
                .from_table(Columns::Table)
                .and_where(Expr::col(Columns::Seq).eq(3_i64));
            database.execute(&delete).await?;
            assert_eq!(
                database.verify_security_log(&ring).await?,
                Verification::Broken {
                    seq: 4,
                    reason: BreakReason::Gap
                }
            );
            // The first entries removed without a checkpoint.
            let mut delete = Query::delete();
            delete
                .from_table(Columns::Table)
                .and_where(Expr::col(Columns::Seq).lte(3_i64));
            database.execute(&delete).await?;
            assert_eq!(
                database.verify_security_log(&ring).await?,
                Verification::Broken {
                    seq: 4,
                    reason: BreakReason::Checkpoint
                }
            );
            Ok(())
        })
        .await
    }

    #[tokio::test]
    async fn sec_15_old_blocks_are_deleted_and_the_rest_still_verifies() -> TestResult {
        each_database("sec_15_prune_blocks", async |db| {
            let database = open(db).await?;
            let (_folder, ring) = keys();
            append_many(&database, 8).await?;

            // Nothing is older than the cut-off: nothing is deleted.
            assert_eq!(database.prune_in_blocks(&ring, at(0), at(100), 3).await?, 0);
            // Entries 1..=7 are older; blocks 1-3 and 4-6 are whole, block 7-9 is not.
            assert_eq!(database.prune_in_blocks(&ring, at(8), at(100), 3).await?, 6);
            let left = database.security_log_entries(0, 10).await?;
            assert_eq!(
                left.iter().map(|entry| entry.seq).collect::<Vec<_>>(),
                [7, 8]
            );
            assert!(matches!(
                database.verify_security_log(&ring).await?,
                Verification::Valid {
                    entries: 2,
                    head: Some((8, _))
                }
            ));

            // Appending goes on from the end; the log still verifies.
            append_many(&database, 2).await?;
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
                database.verify_security_log(&ring).await?,
                Verification::Valid { entries: 4, .. }
            ));
            // The newest entry always stays, even when its block is whole and old.
            assert_eq!(
                database
                    .prune_in_blocks(&ring, at(1000), at(100), 3)
                    .await?,
                3
            );
            assert_eq!(database.security_log_entries(0, 10).await?.len(), 1);
            assert!(matches!(
                database.verify_security_log(&ring).await?,
                Verification::Valid { entries: 1, .. }
            ));
            Ok(())
        })
        .await
    }

    #[tokio::test]
    async fn bad_example_a_forged_or_removed_checkpoint_is_found() -> TestResult {
        each_database("sec_15_forged_checkpoint", async |db| {
            let database = open(db).await?;
            let (_folder, ring) = keys();
            append_many(&database, 8).await?;
            database.prune_in_blocks(&ring, at(8), at(100), 3).await?;

            // Another installation's key cannot vouch for this one.
            let (_other_folder, other) = keys();
            assert_eq!(
                database.verify_security_log(&other).await?,
                Verification::Broken {
                    seq: 6,
                    reason: BreakReason::Checkpoint
                }
            );
            assert!(
                database
                    .prune_in_blocks(&other, at(1000), at(100), 3)
                    .await
                    .is_err()
            );

            // A moved checkpoint no longer matches its signature.
            let mut update = Query::update();
            update
                .table(Checkpoint::Table)
                .value(Checkpoint::Seq, 5_i64);
            database.execute(&update).await?;
            assert_eq!(
                database.verify_security_log(&ring).await?,
                Verification::Broken {
                    seq: 5,
                    reason: BreakReason::Checkpoint
                }
            );

            // No checkpoint at all, but the first entries are gone.
            let mut delete = Query::delete();
            delete.from_table(Checkpoint::Table);
            database.execute(&delete).await?;
            assert_eq!(
                database.verify_security_log(&ring).await?,
                Verification::Broken {
                    seq: 7,
                    reason: BreakReason::Checkpoint
                }
            );
            Ok(())
        })
        .await
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn writers_at_the_same_time_extend_one_chain() -> TestResult {
        each_database("sec_15_concurrent_appends", async |db| {
            let database = open(db).await?;
            let (_folder, ring) = keys();
            let mut tasks = Vec::new();
            for number in 0..8 {
                let database = database.clone();
                tasks.push(tokio::spawn(async move {
                    database
                        .append_security_log(&new_entry(number, "sign_in.ok"))
                        .await
                }));
            }
            for task in tasks {
                task.await??;
            }
            assert!(matches!(
                database.verify_security_log(&ring).await?,
                Verification::Valid {
                    entries: 8,
                    head: Some((8, _))
                }
            ));
            Ok(())
        })
        .await
    }
}
