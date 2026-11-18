//! What a backup is (OPS-07, ADR-0046): one file, `bugaboo-<moment>-<version>.backup`, a tar
//! archive with a manifest, the copy of the database, the files the database points to and a copy
//! of the replay list. It holds no keys.
//!
//! This module is the one description of a backup, and it is pure: the names of its members and
//! the check that refuses a name leaving the restore's own folder ([`member`]), the manifest with
//! its checks ([`manifest`]), the name of the file ([`name`]) and which backups the schedule keeps
//! ([`rotation`]). The copy of the database and the archive itself are written by `bb-store`; the
//! commands, the route and the restore read this description and nothing else.

pub mod manifest;
pub mod member;
pub mod name;
pub mod replay;
pub mod rotation;
pub mod run;

use serde::{Deserialize, Serialize};

/// The kind of database a backup was made from. A restore goes into the same kind (decision 2).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DatabaseKind {
    /// SQLite: the copy is one database file made with `VACUUM INTO`.
    Sqlite,
    /// PostgreSQL: the copy is the rows of every table, written table by table.
    Postgres,
}

impl DatabaseKind {
    /// The member of the archive that holds the copy of a database of this kind.
    #[must_use]
    pub const fn member(self) -> &'static str {
        match self {
            Self::Sqlite => member::DATABASE_SQLITE,
            Self::Postgres => member::DATABASE_ROWS,
        }
    }
}
