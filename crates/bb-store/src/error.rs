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
    /// A value that is stored as JSON could not be written or read.
    Json(serde_json::Error),
    /// The setting key is not in the registry, or the level or the type of the value does not fit.
    SettingRejected(SettingProblem),
    /// A secret could not be encrypted or decrypted, or the key file is unusable.
    Secret(SecretProblem),
    /// Another writer was faster too many times in a row; the caller may try again.
    Contended,
    /// The name is kept for the store itself (for example the head of the outbox).
    ReservedName,
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
    /// A project or person level value needs the project or person it is for; the organization
    /// level takes none.
    ScopeMismatch,
    /// The value has another type than the setting declares.
    WrongType,
    /// The setting cannot be locked.
    NotLockable,
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
            Self::Corrupt(what) => write!(formatter, "a stored value is unusable: {what}"),
            Self::Json(error) => write!(formatter, "a JSON value is unusable: {error}"),
            Self::SettingRejected(problem) => {
                write!(formatter, "the setting was refused: {problem:?}")
            }
            Self::Secret(problem) => write!(formatter, "the secret operation failed: {problem:?}"),
            Self::ReservedName => formatter.write_str("this name is reserved for the store"),
            Self::Contended => formatter.write_str("too many writers at once; try again"),
            Self::KeyFile(error) => write!(formatter, "the master key file failed: {error}"),
        }
    }
}

impl std::error::Error for StoreError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Database(error) => Some(error),
            Self::Json(error) => Some(error),
            Self::KeyFile(error) => Some(error),
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
        Self::Json(error)
    }
}

impl StoreError {
    /// Whether the database refused a row because the same key exists already.
    #[must_use]
    pub fn is_duplicate(&self) -> bool {
        matches!(self, Self::Database(sqlx::Error::Database(error)) if error.is_unique_violation())
    }
}
