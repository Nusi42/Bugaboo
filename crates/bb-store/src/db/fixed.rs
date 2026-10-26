//! Fixed statements that only one database knows (a virtual table, a generated column), which the
//! query builder cannot describe. A migration holds one list for each database; the text of a
//! statement is written in the code and no value goes into it. `Transaction::execute_fixed` runs
//! them.

use super::Dialect;

/// The statements of one step of a migration, for each database.
pub(crate) struct FixedSql {
    /// What SQLite runs.
    pub(crate) sqlite: Vec<&'static str>,
    /// What PostgreSQL runs.
    pub(crate) postgres: Vec<&'static str>,
}

impl FixedSql {
    /// The statements for one database.
    pub(super) fn of(&self, dialect: Dialect) -> &[&'static str] {
        match dialect {
            Dialect::Sqlite => &self.sqlite,
            Dialect::Postgres => &self.postgres,
        }
    }

    /// The text of both lists, one text for each database, for the checksum of the migration.
    pub(crate) fn texts(&self) -> [String; 2] {
        [self.sqlite.join(";"), self.postgres.join(";")]
    }
}
