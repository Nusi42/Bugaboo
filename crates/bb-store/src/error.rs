//! What can go wrong in the store. Errors are typed; the texts here are for developers and logs.
//! A person sees a message from the catalog instead (the caller maps the error).

use std::fmt;

/// An error of the store.
#[derive(Debug)]
pub enum StoreError {
    /// The database refused or failed. The reason is from the database library and holds no
    /// stored values.
    Database(sqlx::Error),
    /// The database URL names a database Bugaboo does not support.
    UnsupportedDatabase,
    /// The database was changed by a newer Bugaboo than this one.
    SchemaTooNew {
        /// The newest migration the database has.
        found: i64,
        /// The newest migration this program knows.
        known: i64,
    },
    /// The recorded migrations are not the ones this program has (changed, missing or renamed).
    MigrationMismatch {
        /// The number of the migration.
        version: i64,
    },
    /// A stored value cannot be read back (a bad ID, time, name or hash).
    Corrupt(&'static str),
    /// A value that is stored as JSON could not be written or read. Only the kind of problem is
    /// kept: the text of a JSON error can quote the value, and stored values must not reach logs.
    Json(serde_json::error::Category),
    /// The input is not acceptable (for example a job that may run zero times).
    InvalidInput(&'static str),
    /// The database has secrets or a security log, but a key file is missing. New keys would
    /// never read what the old ones protected, so the program stops (SEC-16).
    KeysMissing,
    /// The setting key is not in the registry, or the level or the type of the value does not fit.
    SettingRejected(SettingProblem),
    /// A secret could not be encrypted or decrypted, or the key file is unusable.
    Secret(SecretProblem),
    /// Another writer was faster too many times in a row; the caller may try again.
    Contended,
    /// SQLite stayed busy: the upkeep of the database held the write lock past the last try of
    /// the write (`db/upkeep.rs`), or another writer held it past the busy timeout. Nothing was
    /// written; the person may try again a little later.
    Busy,
    /// A read took longer than the time it was given and was stopped by the database (SEC-17).
    TimedOut,
    /// A change decided not to go ahead (a check inside its transaction failed) and was undone.
    Aborted,
    /// A rule of the policy engine stopped a change (the last administrator, no escalation) and
    /// it was undone.
    Refused(bb_policy::guards::Refusal),
    /// The name is kept for the store itself (for example the head of the outbox).
    ReservedName,
    /// The anchor file of the security log could not be read or written.
    AnchorFile(std::io::Error),
    /// The master key file could not be read or written.
    KeyFile(std::io::Error),
}

/// Why a setting value was refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SettingProblem {
    /// The key is not declared in the settings registry.
    UnknownKey,
    /// The setting cannot be set on this level.
    LevelNotAllowed,
    /// The value has another type than the setting declares.
    WrongType,
    /// The setting cannot be locked.
    NotLockable,
    /// A whole number outside the bounds the setting declares, a list with more items, or longer
    /// items, than it allows, or a text that is not one of the choices or not in the format the
    /// setting declares.
    OutOfBounds,
}

/// Why a secret operation failed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SecretProblem {
    /// There is no secret with this name.
    Missing,
    /// The stored secret was changed, or belongs to another name or key.
    Tampered,
    /// The master key that wrapped the secret is not in the key file any more.
    UnknownKey,
    /// The key file has a line that is not a key.
    BadKeyFile,
    /// The operating system gave no secure random bytes.
    NoRandomness,
}

impl fmt::Display for StoreError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Database(error) => write!(formatter, "the database failed: {error}"),
            Self::UnsupportedDatabase => {
                formatter.write_str("the database URL must start with sqlite: or postgres:")
            }
            Self::SchemaTooNew { found, known } => write!(
                formatter,
                "the database has migration {found}, but this program only knows up to {known}; \
                 start the newer Bugaboo that changed it"
            ),
            Self::MigrationMismatch { version } => write!(
                formatter,
                "the recorded migration {version} is not the one this program has"
            ),
            Self::TimedOut => formatter.write_str("the read took longer than it was given"),
            Self::Corrupt(what) => write!(formatter, "a stored value is unusable: {what}"),
            Self::Json(kind) => write!(formatter, "a JSON value is unusable ({kind:?})"),
            Self::InvalidInput(what) => write!(formatter, "not acceptable: {what}"),
            Self::KeysMissing => formatter.write_str(
                &bb_core::message::Message::from(bb_core::message::SERVER_KEYS_MISSING).render(),
            ),
            Self::SettingRejected(problem) => {
                write!(formatter, "the setting was refused: {problem:?}")
            }
            Self::Secret(problem) => write!(formatter, "the secret operation failed: {problem:?}"),
            Self::Aborted => formatter.write_str("the change stopped itself and was undone"),
            Self::Refused(refusal) => write!(
                formatter,
                "a rule of the policy engine stopped the change and it was undone: {refusal:?}"
            ),
            Self::ReservedName => formatter.write_str("this name is reserved for the store"),
            Self::Contended => formatter.write_str("too many writers at once; try again"),
            Self::Busy => formatter.write_str("the database stayed busy; nothing was written"),
            Self::KeyFile(error) => write!(formatter, "a key file failed: {error}"),
            Self::AnchorFile(error) => {
                write!(formatter, "the security log anchor file failed: {error}")
            }
        }
    }
}

impl std::error::Error for StoreError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Database(error) => Some(error),
            Self::KeyFile(error) | Self::AnchorFile(error) => Some(error),
            _ => None,
        }
    }
}

impl From<sqlx::Error> for StoreError {
    fn from(error: sqlx::Error) -> Self {
        Self::Database(error)
    }
}

impl From<serde_json::Error> for StoreError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error.classify())
    }
}

impl StoreError {
    /// Whether the database refused a row because the same key exists already.
    #[must_use]
    pub fn is_duplicate(&self) -> bool {
        matches!(self, Self::Database(sqlx::Error::Database(error)) if error.is_unique_violation())
    }

    /// Whether SQLite refused because another connection holds the file (`SQLITE_BUSY` is 5,
    /// `SQLITE_LOCKED` is 6). PostgreSQL's codes have five characters, so none of them is one.
    #[must_use]
    pub(crate) fn is_locked(&self) -> bool {
        matches!(
            self,
            Self::Database(sqlx::Error::Database(database))
                if matches!(database.code().as_deref(), Some("5" | "6"))
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_json_error_never_carries_the_stored_value() {
        let error = serde_json::from_str::<u32>("\"hunter2-secret-value\"").unwrap_err();
        assert!(
            error.to_string().contains("hunter2"),
            "the library quotes the value"
        );
        let store: StoreError = error.into();
        assert!(!format!("{store} {store:?}").contains("hunter2"));
    }
}
