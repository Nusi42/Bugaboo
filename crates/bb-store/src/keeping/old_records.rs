//! The store's part of the records an administrator deletes after their keep period (ADR-0045,
//! decision 6): each kind of the group *by an administrator* has its own rows, date and what goes
//! with a record, in the module of its tables; this says which, and [`choose`] is the one way a
//! step chooses its records and counts them by person.

use bb_app::keeping::{OldCount, OldRecordStore};
use std::collections::BTreeMap;

use bb_core::id::UserId;
use bb_core::keeping::KeepKind;
use jiff::Timestamp;
use jiff::civil::Date;
use sea_query::{Iden, Order, SelectStatement};

use crate::auth::AuthStorage;
use crate::db::Transaction;
use crate::error::StoreError;
use crate::{time, working_hours};

impl OldRecordStore for AuthStorage {
    async fn old_records(&self, kind: KeepKind, before: Date) -> Result<Vec<OldCount>, StoreError> {
        let mut reader = self.database();
        match kind {
            KeepKind::WorkDays => working_hours::old::count(&mut reader, before, None).await,
            KeepKind::TimeEntries => time::old::count(&mut reader, before, None).await,
            _ => Err(StoreError::Corrupt("a kind Bugaboo deletes by itself")),
        }
    }

    async fn count_past(&self, kind: KeepKind, before: Timestamp) -> Result<u64, StoreError> {
        super::past::count_past(self.database(), kind, before).await
    }

    async fn delete_old_records(
        transaction: &mut Transaction,
        kind: KeepKind,
        before: Date,
        person: Option<UserId>,
        limit: u64,
    ) -> Result<Vec<OldCount>, StoreError> {
        match kind {
            KeepKind::WorkDays => {
                working_hours::old::delete(transaction, before, person, limit).await
            }
            KeepKind::TimeEntries => time::old::delete(transaction, before, person, limit).await,
            _ => Err(StoreError::Corrupt("a kind Bugaboo deletes by itself")),
        }
    }
}

/// The records one step deletes: at most `limit` of what `records` finds (rows with their key `id`
/// and their person `person`), in the order of their keys; their keys, and how many of whose. The
/// count is made from the chosen rows, so it says what the step deletes.
pub(crate) async fn choose<C: Iden + Copy + 'static>(
    transaction: &mut Transaction,
    mut records: SelectStatement,
    (id, person): (C, C),
    limit: u64,
) -> Result<(Vec<Vec<u8>>, Vec<OldCount>), StoreError> {
    records
        .column(id)
        .column(person)
        .order_by(id, Order::Asc)
        .limit(limit);
    let mut keys = Vec::new();
    let mut by_person: BTreeMap<UserId, u64> = BTreeMap::new();
    for row in transaction.fetch_all(&records).await? {
        keys.push(row.get(id)?);
        *by_person.entry(row.id(person)?).or_default() += 1;
    }
    let counts = by_person
        .into_iter()
        .map(|(person, count)| OldCount { person, count })
        .collect();
    Ok((keys, counts))
}

#[cfg(test)]
#[path = "../tests/old_records.rs"]
mod tests;
