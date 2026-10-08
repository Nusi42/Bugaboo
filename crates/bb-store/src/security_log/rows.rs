//! The rows of the log: reading entries and their place and hash.

use bb_core::event::{EntityKind, EntityRef};
use bb_core::id::EntityId;
use bb_core::network::NetworkPrefix;
use bb_core::security_log::{Digest, SecurityLogEntry};
use bb_core::technical::TechnicalName;
use sea_query::{Expr, ExprTrait as _, Order, Query};

use crate::db::{Row, Transaction};
use crate::error::StoreError;
use crate::schema::SecurityLog as Columns;

/// How many entries are read at a time when verifying.
pub(super) const PAGE: u64 = 500;

/// The place and hash of the newest entry.
pub(super) async fn last_entry(
    transaction: &mut Transaction,
) -> Result<Option<(i64, Digest)>, StoreError> {
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

pub(super) fn entries_after(after_seq: i64, limit: u64) -> sea_query::SelectStatement {
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
pub(super) async fn read_page(
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

pub(super) fn digest(row: &Row, column: impl sea_query::Iden + Copy) -> Result<Digest, StoreError> {
    let bytes: Vec<u8> = row.get(column)?;
    Digest::from_slice(&bytes).ok_or(StoreError::Corrupt("a hash"))
}

pub(super) fn entry_columns() -> [Columns; 9] {
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

pub(super) fn read_entry(row: &Row) -> Result<SecurityLogEntry, StoreError> {
    let subject = match (
        row.get::<Option<String>>(Columns::SubjectKind)?,
        row.optional_id::<EntityId>(Columns::SubjectId)?,
    ) {
        (Some(kind), Some(id)) => Some(EntityRef {
            kind: EntityKind::new(&kind).map_err(|_| StoreError::Corrupt("a kind of record"))?,
            id,
        }),
        (None, None) => None,
        _ => return Err(StoreError::Corrupt("a subject")),
    };
    Ok(SecurityLogEntry {
        seq: row.get(Columns::Seq)?,
        time: row.time(Columns::Time)?,
        person: row.optional_id(Columns::Person)?,
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
