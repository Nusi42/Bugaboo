//! The databases Bugaboo runs on, and the variables that tell the test harness (`bb-testing`)
//! which of them to use. `cargo xtask db` sets the variables; the harness reads them.

use std::fmt;

use bb_core::words;

/// Names the databases to run on, comma separated.
pub const TEST_DATABASES_VARIABLE: &str = "BB_TEST_DATABASES";
/// The URL of the throwaway PostgreSQL (its maintenance database).
pub const TEST_POSTGRES_URL_VARIABLE: &str = "BB_TEST_POSTGRES_URL";
/// The file that records which test ran on which database (ARCH-DB-PARITY reads it).
pub const PARITY_RECORD_VARIABLE: &str = "BB_DB_PARITY_FILE";

words! {
    /// The databases Bugaboo supports, in the order the tools run them. The word is the name used
    /// on command lines, in `BB_TEST_DATABASES` and in the parity record.
    pub enum DatabaseKind {
        /// SQLite, the built-in database: a file.
        Sqlite => "sqlite",
        /// PostgreSQL.
        Postgres => "postgres",
    }
}

impl DatabaseKind {
    /// The names of every database, separated by `separator`.
    #[must_use]
    pub fn names(separator: &str) -> String {
        Self::ALL
            .iter()
            .map(|kind| kind.code())
            .collect::<Vec<_>>()
            .join(separator)
    }
}

/// A name that is no database.
#[derive(Debug, PartialEq, Eq)]
pub struct UnknownDatabase(pub String);

impl fmt::Display for UnknownDatabase {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "`{}` is no database; use {}",
            self.0,
            DatabaseKind::names(" or ")
        )
    }
}

impl std::error::Error for UnknownDatabase {}

/// The database with this name, or an error that says what is known (for command lines and
/// variables, where the plain error of a word would not name the text).
///
/// # Errors
/// Fails when `name` is no database.
pub fn parse(name: &str) -> Result<DatabaseKind, UnknownDatabase> {
    DatabaseKind::from_code(name).ok_or_else(|| UnknownDatabase(name.to_owned()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_database_is_found_by_its_name() {
        for kind in DatabaseKind::ALL {
            assert_eq!(parse(kind.code()), Ok(*kind));
        }
        assert_eq!(DatabaseKind::names(","), "sqlite,postgres");
    }

    #[test]
    fn an_unknown_name_is_refused_and_says_what_is_known() {
        let refused = parse("mysql").unwrap_err();
        assert_eq!(
            refused.to_string(),
            "`mysql` is no database; use sqlite or postgres"
        );
    }
}
