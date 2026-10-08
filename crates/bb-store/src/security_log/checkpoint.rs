//! The checkpoint a deleted block leaves behind: signed, stored, and checked.

use bb_core::security_log::{Digest, SecurityLogCheckpoint};
use jiff::Timestamp;
use sea_query::{Expr, Order, Query};

use crate::chain::{CHECKPOINT_DOMAIN, checkpoint_message};
use crate::db::{Transaction, to_micros};
use crate::error::StoreError;
use crate::keys::SigningKey;
use crate::schema::SecurityLogCheckpoint as Checkpoint;

use super::rows::digest;

pub(super) fn sign_checkpoint(
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
pub(super) fn checkpoint_is_signed(key: &SigningKey, checkpoint: &SecurityLogCheckpoint) -> bool {
    key.verify(
        &[
            CHECKPOINT_DOMAIN,
            &checkpoint_message(checkpoint.seq, &checkpoint.hash, checkpoint.created_at),
        ],
        checkpoint.signature.as_bytes(),
    )
}

pub(super) async fn replace_checkpoint(
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

pub(super) async fn read_checkpoint(
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
