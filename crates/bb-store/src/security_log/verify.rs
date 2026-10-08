//! Verifying the chain: with the log key, the checkpoint and the anchor file.

use bb_core::security_log::{Digest, SecurityLogCheckpoint};

use crate::anchor::{Anchored, LogSeal};
use crate::chain::{BreakReason, START, Verification, Verifier, hash_of};
use crate::db::{Database, Transaction};
use crate::error::StoreError;
use crate::keys::SigningKey;

use super::checkpoint::{checkpoint_is_signed, read_checkpoint};
use super::rows::{PAGE, read_page};

impl Database {
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
}

/// Where checking starts: after the entry an earlier check found right, else after the
/// checkpoint, else at the beginning. A log that has no checkpoint must start at 1; a later start
/// means entries were deleted without leaving one (the answer is then the break).
pub(super) async fn starting_verifier<'k>(
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
pub(super) async fn still_there(
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
