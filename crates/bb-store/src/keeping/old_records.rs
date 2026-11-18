//! The store's part of the records an administrator deletes after their keep period (ADR-0045,
//! decision 6): each kind of the group *by an administrator* has its own rows, date and what goes
//! with a record, in the module of its tables; this only says which.

use bb_app::keeping::{OldCount, OldRecordStore};
use bb_core::id::UserId;
use bb_core::keeping::KeepKind;
use jiff::civil::Date;

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

    async fn delete_old_records(
        transaction: &mut Transaction,
        kind: KeepKind,
        before: Date,
        person: Option<UserId>,
    ) -> Result<Vec<OldCount>, StoreError> {
        match kind {
            KeepKind::WorkDays => working_hours::old::delete(transaction, before, person).await,
            KeepKind::TimeEntries => time::old::delete(transaction, before, person).await,
            _ => Err(StoreError::Corrupt("a kind Bugaboo deletes by itself")),
        }
    }
}
