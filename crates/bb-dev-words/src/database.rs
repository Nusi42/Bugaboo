//! The databases Bugaboo runs on, and the variables that tell the test harness (`bb-testing`)
//! which of them to use. `cargo xtask db` sets the variables; the harness reads them.

use std::fmt;
use std::str::FromStr;

/// Names the databases to run on, comma separated.
pub const TEST_DATABASES_VARIABLE: &str = "BB_TEST_DATABASES";
/// The URL of the throwaway PostgreSQL (its maintenance database).
pub const TEST_POSTGRES_URL_VARIABLE: &str = "BB_TEST_POSTGRES_URL";
/// The file that records which test ran on which database (ARCH-DB-PARITY reads it).
pub const PARITY_RECORD_VARIABLE: &str = "BB_DB_PARITY_FILE";

/// The databases Bugaboo supports.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DatabaseKind {
    /// SQLite, the built-in database: a file.
    Sqlite,
    /// PostgreSQL.
    Postgres,
}

impl DatabaseKind {
    /// Every database, in the order the tools run them.
    pub const ALL: [Self; 2] = [Self::Sqlite, Self::Postgres];

    /// The name used on command lines, in `BB_TEST_DATABASES` and in the parity record.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Sqlite => "sqlite",
            Self::Postgres => "postgres",
        }
    }

    /// The names of every database, separated by `separator`.
    #[must_use]
    pub fn names(separator: &str) -> String {
        Self::ALL.map(Self::name).join(separator)
    }
}

impl fmt::Display for DatabaseKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.name())
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

impl FromStr for DatabaseKind {
    type Err = UnknownDatabase;

    fn from_str(name: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|kind| kind.name() == name)
            .ok_or_else(|| UnknownDatabase(name.to_owned()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_database_is_found_by_its_name() {
        for kind in DatabaseKind::ALL {
            assert_eq!(kind.name().parse(), Ok(kind));
        }
        assert_eq!(DatabaseKind::names(","), "sqlite,postgres");
    }

    #[test]
    fn an_unknown_name_is_refused_and_says_what_is_known() {
        let refused = "mysql".parse::<DatabaseKind>().unwrap_err();
        assert_eq!(
            refused.to_string(),
            "`mysql` is no database; use sqlite or postgres"
        );
    }
}
