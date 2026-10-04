//! DATA-MIGRATIONS: the schema is defined once, in Rust, and runs on both databases (ADR-0003).
//!
//! Two rules: no `.sql` migration files anywhere (two folders of hand-written SQL would be two
//! sources of truth), and, once `bb-store` has a migrations module, its `migrations` test has run
//! on both databases according to the parity record (see ARCH-DB-PARITY).

use std::fs;

use super::arch_db_parity::{self, DATABASES, RECORD_PATH};
use super::{Check, Context, Finding};
use crate::repo::Repo;

/// Where the migrations will live (WP-05); the second rule applies once this exists.
const MIGRATIONS_SOURCES: [&str; 2] = [
    "crates/bb-store/src/migrations.rs",
    "crates/bb-store/src/migrations",
];

/// The name of the test that runs all migrations from an empty database.
const MIGRATION_TEST: &str = "migrations";

/// Examples of rejected files live here and are not migrations.
const FIXTURES: &str = "tools/checks/fixtures/";

pub(crate) struct DataMigrations;

impl Check for DataMigrations {
    fn id(&self) -> &'static str {
        "DATA-MIGRATIONS"
    }

    fn run(&self, context: &Context<'_>) -> anyhow::Result<Vec<Finding>> {
        let record = fs::read_to_string(context.repo.path(RECORD_PATH)).ok();
        evaluate_repo(context.repo, record.as_deref())
    }
}

/// Both rules on the repository, with the parity record of the test run if there is one. Used by
/// the check and, after the tests, by `cargo xtask db run`.
pub(crate) fn evaluate_repo(repo: &Repo, record: Option<&str>) -> anyhow::Result<Vec<Finding>> {
    let files = repo.all_files()?;
    let has_migrations = MIGRATIONS_SOURCES
        .iter()
        .any(|path| repo.path(path).exists());
    Ok(evaluate(&files, has_migrations, record))
}

fn evaluate(files: &[String], has_migrations: bool, record: Option<&str>) -> Vec<Finding> {
    let mut findings: Vec<Finding> = files
        .iter()
        .filter(|file| file.to_ascii_lowercase().ends_with(".sql") && !file.starts_with(FIXTURES))
        .map(|file| {
            Finding::at(
                file,
                "SQL migration files are not allowed. Define the schema once in Rust with the \
                 query builder (ADR-0003).",
            )
        })
        .collect();
    if let (true, Some(record)) = (has_migrations, record) {
        let ran = arch_db_parity::parse(record);
        for database in DATABASES {
            let ok = ran
                .get(database)
                .is_some_and(|tests| tests.contains(MIGRATION_TEST));
            if !ok {
                findings.push(Finding::at(
                    RECORD_PATH,
                    format!(
                        "The `{MIGRATION_TEST}` test did not run on `{database}`. Migrations must \
                         run on both databases."
                    ),
                ));
            }
        }
    }
    findings
}

#[cfg(test)]
mod tests {
    use super::*;

    fn files(list: &[&str]) -> Vec<String> {
        list.iter().map(|f| (*f).to_owned()).collect()
    }

    #[test]
    fn good_example_rust_only_and_migrations_ran_on_both() {
        let record = "sqlite migrations\npostgres migrations\n";
        let ok = files(&["crates/bb-store/src/migrations.rs", "Cargo.toml"]);
        assert!(evaluate(&ok, true, Some(record)).is_empty());
        assert!(evaluate(&ok, true, None).is_empty(), "no record yet");
    }

    #[test]
    fn bad_example_a_sql_migration_file() {
        let bad = files(&["crates/bb-store/migrations/0001_init.SQL"]);
        let findings = evaluate(&bad, false, None);
        assert_eq!(findings.len(), 1);
        assert!(findings[0].message.contains("ADR-0003"));
    }

    #[test]
    fn bad_example_migrations_only_ran_on_sqlite() {
        let findings = evaluate(&[], true, Some("sqlite migrations\npostgres other\n"));
        assert_eq!(findings.len(), 1);
        assert!(findings[0].message.contains("`postgres`"));
    }

    #[test]
    fn example_sql_in_a_check_fixture_is_not_a_migration() {
        assert!(evaluate(&files(&["tools/checks/fixtures/X/bad/a.sql"]), false, None).is_empty());
    }
}
