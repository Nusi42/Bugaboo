//! Deleting old blocks of the log, and signing the checkpoint they leave.

use bb_core::security_log::SecurityLogCheckpoint;
use jiff::Timestamp;
use sea_query::{Expr, ExprTrait as _, Order, Query};

use crate::anchor::LogSeal;
use crate::chain::{BLOCK_SIZE, START, Verifier};
use crate::db::{Database, to_micros};
use crate::error::{SecretProblem, StoreError};
use crate::schema::{Computed, SecurityLog as Columns};

use super::checkpoint::{
    checkpoint_is_signed, read_checkpoint, replace_checkpoint, sign_checkpoint,
};
use super::rows::{PAGE, last_entry, read_page};

impl Database {
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
