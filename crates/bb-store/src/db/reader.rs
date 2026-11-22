//! The one way a read runs its statements, on the pool or in a transaction.

use std::future::Future;

use sea_query::SelectStatement;

use super::{Database, Row, Transaction};
use crate::error::StoreError;

/// Something that runs the statements of a read: the pool (one statement, no snapshot) or a
/// transaction (a snapshot, or a change that is open). A read is written once over this, so there
/// is no pool version and no transaction version of it.
pub(crate) trait Reader: Send {
    /// Reads all rows of a query.
    fn fetch_all(
        &mut self,
        select: &SelectStatement,
    ) -> impl Future<Output = Result<Vec<Row>, StoreError>> + Send;

    /// Reads the first row of a query, if there is one.
    fn fetch_optional(
        &mut self,
        select: &SelectStatement,
    ) -> impl Future<Output = Result<Option<Row>, StoreError>> + Send {
        async move { Ok(self.fetch_all(select).await?.into_iter().next()) }
    }
}

impl Reader for &Database {
    fn fetch_all(
        &mut self,
        select: &SelectStatement,
    ) -> impl Future<Output = Result<Vec<Row>, StoreError>> + Send {
        Database::fetch_all(self, select)
    }
}

impl Reader for Transaction {
    fn fetch_all(
        &mut self,
        select: &SelectStatement,
    ) -> impl Future<Output = Result<Vec<Row>, StoreError>> + Send {
        Self::fetch_all(self, select)
    }
}

/// The number a counting statement answers in its column `Computed::Total`: the one way a read
/// takes a count, on the pool or in a transaction.
pub(crate) async fn count_of(
    reader: &mut impl Reader,
    select: &SelectStatement,
) -> Result<u64, StoreError> {
    let count: i64 = match reader.fetch_optional(select).await? {
        Some(row) => row.get(crate::schema::Computed::Total)?,
        None => 0,
    };
    Ok(u64::try_from(count).unwrap_or(0))
}
