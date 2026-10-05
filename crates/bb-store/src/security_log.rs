//! The security log (SEC-15) in the database: appending, verifying and deleting old blocks.
//!
//! Entries are only ever added. Each entry stores the keyed hash (HMAC-SHA256 under the log key,
//! see [`crate::chain`]) of the entry before it, and after every append the newest place and hash
//! are written to the anchor file ([`crate::anchor`]), so somebody who can only change the
//! database can neither rewrite entries nor cut the end off.
//!
//! After the keep period a job deletes the oldest *whole blocks* of [`BLOCK_SIZE`] entries and
//! leaves one signed checkpoint behind: the place and hash of the last deleted entry and the time,
//! signed with the log key. The entries that remain still verify, because the first of them points
//! to the checkpoint's hash. Before it deletes anything, the job verifies the blocks it is about
//! to delete and the link to the first entry it keeps, in the same transaction, and refuses when
//! something does not fit. A checkpoint that is missing or forged, or entries missing without one,
//! show as a break.

use bb_core::event::{EntityKind, EntityRef};
use bb_core::id::{EntityId, UserId};
use bb_core::network::NetworkPrefix;
use bb_core::security_log::{Digest, SecurityLogCheckpoint, SecurityLogEntry};
use bb_core::technical::TechnicalName;
use jiff::Timestamp;
use sea_query::{Expr, ExprTrait as _, Order, Query};

use crate::anchor::{Anchored, LogSeal};
use crate::chain::{
    BLOCK_SIZE, BreakReason, CHECKPOINT_DOMAIN, START, Verification, Verifier, checkpoint_message,
    hash_of,
};
use crate::db::{Database, Row, Transaction, to_micros};
use crate::error::{SecretProblem, StoreError};
use crate::keys::SigningKey;
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
    /// Adds an entry to the end of the chain, writes the new end to the anchor file, and returns
    /// the entry with its place and hash. When two writers add at the same moment, one of them
    /// starts over.
    ///
    /// Do not call this again after [`StoreError::AnchorFile`]: the entry is stored already, and
    /// a second call would add it twice. Report the problem with the file instead.
    ///
    /// # Errors
    /// Fails with [`StoreError::Contended`] after repeated collisions, with
    /// [`StoreError::AnchorFile`] when the anchor file cannot be written (the entry is stored
    /// then), and when the database fails.
    pub async fn append_security_log(
        &self,
        seal: &LogSeal,
        new: &NewSecurityLogEntry,
    ) -> Result<SecurityLogEntry, StoreError> {
        for _ in 0..APPEND_ATTEMPTS {
            match self.try_append(seal.key(), new).await {
                Err(error) if error.is_duplicate() => {}
                Ok(entry) => {
                    seal.record(entry.seq, &entry.hash)?;
                    return Ok(entry);
                }
                Err(error) => return Err(error),
            }
        }
        Err(StoreError::Contended)
    }

    async fn try_append(
        &self,
        key: &SigningKey,
        new: &NewSecurityLogEntry,
    ) -> Result<SecurityLogEntry, StoreError> {
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
        entry.hash = hash_of(key, &previous, &entry)?;

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
        self.fetch_all(&entries_after(after_seq, limit))
            .await?
            .iter()
            .map(read_entry)
            .collect()
    }

    /// Checks the whole chain with the log key, the checkpoint and the anchor file. Read
    /// `Valid { anchored: false }` together with `entries` and `head`: on a log that has entries
    /// it means nothing vouches for the end (the anchor file is missing or empty), which is a
    /// warning; on an empty log it is normal. This is what `bugaboo security-log verify` runs. It
    /// reads one consistent snapshot of the database and takes no write lock, so appends go on
    /// while it runs.
    ///
    /// # Errors
    /// Fails when the database or the anchor file cannot be read, or an entry is unreadable. A
    /// broken chain is not an error; it is the answer ([`Verification::Broken`]).
    pub async fn verify_security_log(&self, seal: &LogSeal) -> Result<Verification, StoreError> {
        self.verify_from(seal, None).await
    }

    /// The same check, but starting after `known`: the place and hash of an entry that an
    /// earlier check found right. Only the entries after it are read, so a long log costs little
    /// on every visit of the status page; `entries` of the answer is still the number of entries
    /// the log has. Entries before `known` are not read again: an old entry that was changed
    /// since is found by
    /// [`verify_security_log`](Self::verify_security_log), which the command runs, and not by
    /// this. When the log no longer has the entry `known` names (it was changed, or deleted with
    /// the blocks before a checkpoint), everything is checked.
    ///
    /// # Errors
    /// The same as [`verify_security_log`](Self::verify_security_log).
    pub async fn verify_security_log_since(
        &self,
        seal: &LogSeal,
        known: Option<(i64, Digest)>,
    ) -> Result<Verification, StoreError> {
        self.verify_from(seal, known).await
    }

    async fn verify_from(
        &self,
        seal: &LogSeal,
        known: Option<(i64, Digest)>,
    ) -> Result<Verification, StoreError> {
        // The anchor is read first: it is never ahead of the database, so whatever it says
        // is in the snapshot taken afterwards.
        let anchor = seal.latest()?;
        if anchor == Anchored::Forged {
            return Ok(Verification::Broken {
                seq: 0,
                reason: BreakReason::Anchor,
            });
        }
        let mut transaction = self.begin_read().await?;
        let checkpoint = read_checkpoint(&mut transaction).await?;
        if let Some(checkpoint) = &checkpoint
            && !checkpoint_is_signed(seal.key(), checkpoint)
        {
            return Ok(Verification::Broken {
                seq: checkpoint.seq,
                reason: BreakReason::Checkpoint,
            });
        }
        // Start after the entry an earlier check found right, if the log still has it.
        let start = match known {
            Some((seq, hash))
                if still_there(
                    &mut transaction,
                    seal.key(),
                    checkpoint.as_ref(),
                    seq,
                    &hash,
                )
                .await? =>
            {
                Some((seq, hash))
            }
            _ => None,
        };
        let mut verifier =
            match starting_verifier(&mut transaction, seal.key(), checkpoint.as_ref(), start)
                .await?
            {
                Ok(verifier) => verifier,
                Err(broken) => return Ok(broken),
            };
        let anchored_seq = match anchor {
            Anchored::At(seq, _) => Some(seq),
            _ => None,
        };
        let mut hash_at_anchor = None;
        let mut after = start
            .map(|(seq, _)| seq)
            .or(checkpoint.as_ref().map(|checkpoint| checkpoint.seq))
            .unwrap_or(0);
        loop {
            let page = read_page(&mut transaction, after, PAGE).await?;
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
                if Some(entry.seq) == anchored_seq {
                    hash_at_anchor = Some(entry.hash);
                }
            }
        }
        let Verification::Valid {
            entries, mut head, ..
        } = verifier.finish()
        else {
            return Ok(Verification::Broken {
                seq: 0,
                reason: BreakReason::Gap,
            });
        };
        // Nothing new after the entry an earlier check vouched for: that entry is still the end.
        if head.is_none() {
            head = start;
        }
        // The entries are one run from the checkpoint (or the start) to the head, so the head's
        // place says how many there are, also when only the newest were read.
        let entries = match (start, head) {
            (Some(_), Some((seq, _))) => {
                let base = checkpoint.as_ref().map_or(0, |checkpoint| checkpoint.seq);
                u64::try_from(seq - base).unwrap_or(entries)
            }
            _ => entries,
        };
        Ok(judge_anchor(
            anchor,
            &Found {
                checkpoint: checkpoint.as_ref(),
                start,
                hash_at_anchor,
                head,
                entries,
            },
        ))
    }

    /// Deletes the oldest whole blocks whose entries are all older than `before`, and signs a
    /// checkpoint for what was deleted. At least one newer entry always stays. The blocks to be
    /// deleted and the link to the first entry kept are verified first, in the same transaction.
    /// Returns how many entries were deleted.
    ///
    /// # Errors
    /// Fails with [`SecretProblem::Tampered`] when the existing checkpoint, the blocks or the
    /// link do not verify (nothing is deleted then), and when the database fails.
    pub async fn prune_security_log(
        &self,
        seal: &LogSeal,
        before: Timestamp,
        now: Timestamp,
    ) -> Result<u64, StoreError> {
        self.prune_in_blocks(seal, before, now, BLOCK_SIZE).await
    }

    /// [`Self::prune_security_log`] with a block size; the tests use a small one. The size must
    /// stay the same for the life of a log, because blocks are counted from its first entry.
    pub(crate) async fn prune_in_blocks(
        &self,
        seal: &LogSeal,
        before: Timestamp,
        now: Timestamp,
        block_size: i64,
    ) -> Result<u64, StoreError> {
        let tampered = || StoreError::Secret(SecretProblem::Tampered);
        let mut transaction = self.begin().await?;
        let checkpoint = read_checkpoint(&mut transaction).await?;
        if let Some(checkpoint) = &checkpoint
            && !checkpoint_is_signed(seal.key(), checkpoint)
        {
            return Err(tampered());
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

        // Verify what is about to be deleted, from the checkpoint (or the start) up to `last`.
        let mut verifier = match &checkpoint {
            Some(checkpoint) => Verifier::new(seal.key(), checkpoint.seq + 1, checkpoint.hash),
            None => Verifier::new(seal.key(), 1, START),
        };
        let mut after = checkpoint.as_ref().map_or(0, |checkpoint| checkpoint.seq);
        while after < last {
            let limit = u64::try_from(std::cmp::min(
                last - after,
                i64::try_from(PAGE).unwrap_or(i64::MAX),
            ))
            .unwrap_or(PAGE);
            let page = read_page(&mut transaction, after, limit).await?;
            if page.is_empty() {
                return Err(tampered());
            }
            for entry in &page {
                verifier.check(entry).map_err(|_| tampered())?;
                after = entry.seq;
            }
        }
        let deleted_head = verifier.head();
        // The first entry that stays must point to the hash of the last one that goes.
        let first_kept = read_page(&mut transaction, last, 1)
            .await?
            .into_iter()
            .next()
            .ok_or_else(tampered)?;
        if first_kept.seq != last + 1 || first_kept.previous_hash != deleted_head {
            return Err(tampered());
        }

        let signature = sign_checkpoint(seal.key(), last, &deleted_head, now)?;
        let mut delete = Query::delete();
        delete
            .from_table(Columns::Table)
            .and_where(Expr::col(Columns::Seq).lte(last));
        let deleted = transaction.execute(&delete).await?;
        replace_checkpoint(
            &mut transaction,
            &SecurityLogCheckpoint {
                seq: last,
                hash: deleted_head,
                signature,
                created_at: now,
            },
        )
        .await?;
        transaction.commit().await?;
        Ok(deleted)
    }
}

/// Where checking starts: after the entry an earlier check found right, else after the
/// checkpoint, else at the beginning. A log that has no checkpoint must start at 1; a later start
/// means entries were deleted without leaving one (the answer is then the break).
async fn starting_verifier<'k>(
    transaction: &mut Transaction,
    key: &'k SigningKey,
    checkpoint: Option<&SecurityLogCheckpoint>,
    start: Option<(i64, Digest)>,
) -> Result<Result<Verifier<'k>, Verification>, StoreError> {
    if let Some((seq, hash)) = start {
        return Ok(Ok(Verifier::new(key, seq + 1, hash)));
    }
    if let Some(checkpoint) = checkpoint {
        return Ok(Ok(Verifier::new(key, checkpoint.seq + 1, checkpoint.hash)));
    }
    if let Some(first) = read_page(transaction, 0, 1).await?.first()
        && first.seq != 1
    {
        return Ok(Err(Verification::Broken {
            seq: first.seq,
            reason: BreakReason::Checkpoint,
        }));
    }
    Ok(Ok(Verifier::new(key, 1, START)))
}

/// What a verification found, for comparing it with the anchor file.
struct Found<'a> {
    checkpoint: Option<&'a SecurityLogCheckpoint>,
    /// The entry an earlier check started from, when this check continued from one.
    start: Option<(i64, Digest)>,
    /// The hash of the entry at the place the anchor file names, when it was read.
    hash_at_anchor: Option<Digest>,
    head: Option<(i64, Digest)>,
    entries: u64,
}

/// Compares the end of the log with the anchor file: the database must reach the anchor and have
/// the same entry there. An anchor older than the checkpoint cannot be compared any more (its
/// entry was deleted on purpose), and one older than the entry an earlier check started from was
/// compared by that check.
fn judge_anchor(anchor: Anchored, found: &Found<'_>) -> Verification {
    let Found {
        checkpoint,
        start,
        hash_at_anchor,
        head,
        entries,
    } = *found;
    let Anchored::At(anchor_seq, anchor_hash) = anchor else {
        return Verification::Valid {
            entries,
            head,
            anchored: false,
        };
    };
    let reached = head
        .map(|(seq, _)| seq)
        .or(checkpoint.map(|checkpoint| checkpoint.seq))
        .unwrap_or(0);
    let same = match (checkpoint, start) {
        (Some(checkpoint), _) if anchor_seq == checkpoint.seq => Some(checkpoint.hash),
        (Some(checkpoint), _) if anchor_seq < checkpoint.seq => Some(anchor_hash),
        (_, Some((seq, hash))) if anchor_seq == seq => Some(hash),
        (_, Some((seq, _))) if anchor_seq < seq => Some(anchor_hash),
        _ => hash_at_anchor,
    };
    if reached < anchor_seq {
        return Verification::Broken {
            seq: anchor_seq,
            reason: BreakReason::Behind,
        };
    }
    if same != Some(anchor_hash) {
        return Verification::Broken {
            seq: anchor_seq,
            reason: BreakReason::Truncated,
        };
    }
    Verification::Valid {
        entries,
        head,
        anchored: true,
    }
}

/// Whether the log still holds the entry at `seq` with `hash`, unchanged: as an entry, or as the
/// end of the part that a checkpoint stands for.
async fn still_there(
    transaction: &mut Transaction,
    key: &SigningKey,
    checkpoint: Option<&SecurityLogCheckpoint>,
    seq: i64,
    hash: &Digest,
) -> Result<bool, StoreError> {
    if let Some(checkpoint) = checkpoint {
        if seq < checkpoint.seq {
            return Ok(false);
        }
        if seq == checkpoint.seq {
            return Ok(checkpoint.hash == *hash);
        }
    }
    let entry = read_page(transaction, seq - 1, 1).await?.into_iter().next();
    Ok(entry.is_some_and(|entry| {
        entry.seq == seq
            && entry.hash == *hash
            && hash_of(key, &entry.previous_hash, &entry).is_ok_and(|made| made == entry.hash)
    }))
}

fn sign_checkpoint(
    key: &SigningKey,
    seq: i64,
    hash: &Digest,
    created_at: Timestamp,
) -> Result<Digest, StoreError> {
    key.sign(&[
        CHECKPOINT_DOMAIN,
        &checkpoint_message(seq, hash, created_at),
    ])
    .map(Digest::new)
}

/// Whether the log key signed this checkpoint.
fn checkpoint_is_signed(key: &SigningKey, checkpoint: &SecurityLogCheckpoint) -> bool {
    key.verify(
        &[
            CHECKPOINT_DOMAIN,
            &checkpoint_message(checkpoint.seq, &checkpoint.hash, checkpoint.created_at),
        ],
        checkpoint.signature.as_bytes(),
    )
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

fn entries_after(after_seq: i64, limit: u64) -> sea_query::SelectStatement {
    let mut select = Query::select();
    select
        .columns(entry_columns())
        .from(Columns::Table)
        .and_where(Expr::col(Columns::Seq).gt(after_seq))
        .order_by(Columns::Seq, Order::Asc)
        .limit(limit);
    select
}

/// A page of entries inside a transaction.
async fn read_page(
    transaction: &mut Transaction,
    after_seq: i64,
    limit: u64,
) -> Result<Vec<SecurityLogEntry>, StoreError> {
    transaction
        .fetch_all(&entries_after(after_seq, limit))
        .await?
        .iter()
        .map(read_entry)
        .collect()
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

    async fn append_many(
        database: &Database,
        seal: &LogSeal,
        count: i64,
    ) -> Result<(), StoreError> {
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
    async fn sec_15_cutting_off_the_end_or_replacing_it_is_found_through_the_anchor() -> TestResult
    {
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
    async fn writers_at_the_same_time_extend_one_chain_and_verify_needs_no_write_lock() -> TestResult
    {
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
    async fn ops_08_the_status_page_checks_only_new_entries_and_the_command_checks_all()
    -> TestResult {
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
    async fn sec_15_an_anchor_that_is_ahead_of_the_database_has_a_reason_of_its_own() -> TestResult
    {
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
}
