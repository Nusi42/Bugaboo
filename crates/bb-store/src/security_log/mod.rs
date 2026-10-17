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

mod checkpoint;
mod prune;
mod rows;
mod verify;

use bb_core::event::EntityRef;
use bb_core::id::UserId;
use bb_core::network::NetworkPrefix;
use bb_core::security_log::SecurityLogEntry;
use bb_core::technical::TechnicalName;
use jiff::Timestamp;
use sea_query::{Expr, Query};

use crate::anchor::LogSeal;
use crate::chain::{START, hash_of};
use crate::db::{Database, StoredId, to_micros};
use crate::error::StoreError;
use crate::keys::SigningKey;
use crate::schema::SecurityLog as Columns;

use self::checkpoint::read_checkpoint;
use self::rows::{entries_after, entry_columns, last_entry, read_entry};

/// How often an append starts over when another writer was faster.
const APPEND_ATTEMPTS: u32 = 8;

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
                Expr::val(entry.person.map(UserId::stored)),
                Expr::val(entry.action.as_str()),
                Expr::val(entry.subject.as_ref().map(|subject| subject.kind.as_str())),
                Expr::val(entry.subject.as_ref().map(|subject| subject.id.stored())),
                Expr::val(entry.network.as_ref().map(NetworkPrefix::as_str)),
                Expr::val(entry.previous_hash.as_bytes().to_vec()),
                Expr::val(entry.hash.as_bytes().to_vec()),
            ])
            .map_err(|_| StoreError::Corrupt("a security log entry"))?;
        transaction.execute(&insert).await?;
        transaction.commit().await?;
        Ok(entry)
    }

    /// Up to `limit` entries after place `after_seq`, oldest first (0 for the start). This is the
    /// reader for showing the log: an entry written with a former code has the code that stands
    /// for it now ([`bb_core::sign_in::shown_code`]). The entry is not verifiable like this (its
    /// hash was made from the stored code); verifying reads the stored entries.
    ///
    /// # Errors
    /// Fails when the database fails or an entry is unreadable.
    pub async fn security_log_entries(
        &self,
        after_seq: i64,
        limit: u64,
    ) -> Result<Vec<SecurityLogEntry>, StoreError> {
        let mut entries: Vec<SecurityLogEntry> = self
            .fetch_all(&entries_after(after_seq, limit))
            .await?
            .iter()
            .map(read_entry)
            .collect::<Result<_, _>>()?;
        for entry in &mut entries {
            if let Some(shown) = shown_action(&entry.action) {
                entry.action = shown;
            }
        }
        Ok(entries)
    }
}

/// The action to show for a stored one, when it is a former code.
fn shown_action(stored: &TechnicalName) -> Option<TechnicalName> {
    let shown = bb_core::sign_in::shown_code(stored.as_str());
    (shown != stored.as_str())
        .then(|| TechnicalName::new(shown).ok())
        .flatten()
}

#[cfg(test)]
#[path = "../tests/security_log.rs"]
mod tests;
