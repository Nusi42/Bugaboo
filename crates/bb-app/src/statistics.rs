//! The statistics the database chooses its indexes by (the daily job `statistics.refresh`,
//! ADR-0017): made again from every row for each table that is out of date. A technical upkeep of
//! the database, not a change of its data, so it writes no event; PostgreSQL keeps its own.

use crate::auth::{AppError, AuthStore, Ctx, internal};

impl<S: AuthStore> Ctx<'_, S> {
    /// Makes the statistics the database chooses its indexes by again where they are out of date.
    /// Returns whether a table was analysed.
    pub(crate) async fn refresh_statistics(&self) -> Result<bool, AppError> {
        self.store()
            .refresh_statistics()
            .await
            .map_err(|error| internal::<S>(&error))
    }
}
