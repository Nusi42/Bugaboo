//! Committing a change (ADR-0007): what the change writes, its events and their outbox entries,
//! and the entries of the security log it adds, in one transaction.

use bb_core::event::DomainEvent;
use bb_core::security_log::Digest;

use crate::db::{Database, Transaction};
use crate::error::StoreError;
use crate::events::append_events;
use crate::keys::SigningKey;
use crate::security_log::append_in;

impl Database {
    /// Runs `change` and appends `event` and its outbox entry, all in one transaction. If
    /// `change` fails, nothing is written.
    ///
    /// # Errors
    /// Returns the error of `change`, or a database error.
    pub async fn record_change<T>(
        &self,
        event: &DomainEvent,
        change: impl AsyncFnOnce(&mut Transaction) -> Result<T, StoreError>,
    ) -> Result<T, StoreError> {
        self.record_changes(std::slice::from_ref(event), change)
            .await
    }

    /// The same for any number of events of its own, also none.
    ///
    /// # Errors
    /// Returns the error of `change`, or a database error.
    pub async fn record_changes<T>(
        &self,
        events: &[DomainEvent],
        change: impl AsyncFnOnce(&mut Transaction) -> Result<T, StoreError>,
    ) -> Result<T, StoreError> {
        self.record_signed_changes(events, None, change)
            .await
            .map(|(result, _)| result)
    }

    /// The same, and the entries of the security log the change added, signed with `key` and
    /// appended after its events, so the lock of the end of the log is the last lock of the
    /// transaction and an entry written alone never waits behind the outbox. Answers the place and
    /// hash of the last entry it appended, for the anchor file after the commit. Without a key a
    /// change that added entries is refused: they would be lost.
    ///
    /// # Errors
    /// Returns the error of `change`, or a database error.
    pub(crate) async fn record_signed_changes<T>(
        &self,
        events: &[DomainEvent],
        key: Option<&SigningKey>,
        change: impl AsyncFnOnce(&mut Transaction) -> Result<T, StoreError>,
    ) -> Result<(T, Option<(i64, Digest)>), StoreError> {
        let mut transaction = self.begin().await?;
        let result = change(&mut transaction).await?;
        append_events(&mut transaction, events).await?;
        // The events the change added besides its own come after them, so no writer waits on the
        // lock of the outbox while this one holds a row.
        let follow_ups = std::mem::take(&mut transaction.follow_ups);
        append_events(&mut transaction, &follow_ups).await?;
        let entries = std::mem::take(&mut transaction.security_log);
        let mut head = None;
        if !entries.is_empty() {
            let key = key.ok_or(StoreError::InvalidInput(
                "a change added entries of the security log that nothing signs",
            ))?;
            for new in &entries {
                let entry = append_in(&mut transaction, key, new).await?;
                head = Some((entry.seq, entry.hash));
            }
        }
        transaction.commit().await?;
        Ok((result, head))
    }
}
