//! ARCH-DB-PARITY: the repository tests ran on both SQLite and PostgreSQL (ADR-0003).
//!
//! The test harness (`bb-testing`) appends one line per finished run to `target/db-parity.txt`:
//! `<database> <test name>`. `cargo xtask db run` (used by `cargo xtask ci`) deletes the record
//! before the tests and checks it afterwards. Without a record the tests have not run yet, so a
//! plain `check all` passes; a record that exists has to show every test on both databases.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;

use super::{Check, Context, Finding};

/// Where the harness writes what ran, relative to the repository root.
pub(crate) const RECORD_PATH: &str = "target/db-parity.txt";

/// The databases every repository test has to run on, as the harness names them.
pub(crate) const DATABASES: [&str; 2] = ["sqlite", "postgres"];

pub(crate) struct ArchDbParity;

impl Check for ArchDbParity {
    fn id(&self) -> &'static str {
        "ARCH-DB-PARITY"
    }

    fn run(&self, context: &Context<'_>) -> anyhow::Result<Vec<Finding>> {
        let path = context.repo.path(RECORD_PATH);
        let Ok(record) = fs::read_to_string(&path) else {
            return Ok(Vec::new());
        };
        Ok(problems(&record)
            .into_iter()
            .map(|message| Finding::at(RECORD_PATH, message))
            .collect())
    }
}

/// The tests that ran, per database.
pub(crate) fn parse(record: &str) -> BTreeMap<&str, BTreeSet<&str>> {
    let mut ran: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    for line in record.lines() {
        if let Some((database, test)) = line.trim().split_once(' ') {
            ran.entry(database).or_default().insert(test.trim());
        }
    }
    ran
}

/// What is wrong with the record, in plain words; empty when both databases ran every test.
pub(crate) fn problems(record: &str) -> Vec<String> {
    let ran = parse(record);
    if ran.is_empty() {
        return vec![
            "No repository test is recorded. Run `cargo xtask ci` (or `cargo xtask db run -- \
             cargo test --workspace`)."
                .to_owned(),
        ];
    }
    let mut problems = Vec::new();
    for database in DATABASES {
        if !ran.contains_key(database) {
            problems.push(format!(
                "No test ran on `{database}`. Repository tests must run on {}.",
                DATABASES.join(" and ")
            ));
        }
    }
    let all: BTreeSet<&str> = ran.values().flatten().copied().collect();
    for database in DATABASES {
        let Some(tests) = ran.get(database) else {
            continue;
        };
        for test in all.difference(tests) {
            problems.push(format!("The test `{test}` did not run on `{database}`."));
        }
    }
    problems
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn good_example_every_test_ran_on_both_databases() {
        let record = "sqlite a\npostgres a\nsqlite b\npostgres b\n";
        assert!(problems(record).is_empty());
    }

    #[test]
    fn bad_example_postgres_never_ran() {
        let problems = problems("sqlite a\nsqlite b\n");
        assert!(
            problems.iter().any(|p| p.contains("`postgres`")),
            "{problems:?}"
        );
    }

    #[test]
    fn bad_example_one_test_skipped_a_database() {
        let problems = problems("sqlite a\npostgres a\nsqlite b\n");
        assert_eq!(problems, ["The test `b` did not run on `postgres`."]);
    }

    #[test]
    fn bad_example_an_empty_record_means_nothing_ran() {
        assert_eq!(problems("").len(), 1);
    }
}
