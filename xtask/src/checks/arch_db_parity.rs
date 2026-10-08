//! ARCH-DB-PARITY: the repository tests ran on both SQLite and PostgreSQL (ADR-0003).
//!
//! The test harness (`bb-testing`) appends one line per finished run to `target/db-parity.txt`:
//! `<database> <test name>`. `cargo xtask db run` (used by `cargo xtask ci`) deletes the record
//! before the tests and checks it right afterwards; only that run decides. A record left on disk
//! can be from an older state of the code, so `check all` never reads it: the check here makes
//! sure `cargo xtask ci` runs the Rust tests through `db run`, where the record is checked.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;

use bb_dev_words::database::DatabaseKind;

use super::{Check, Context, Finding};

/// Where the harness writes what ran, relative to the repository root.
pub(crate) const RECORD_PATH: &str = "target/db-parity.txt";

pub(crate) struct ArchDbParity;

impl Check for ArchDbParity {
    fn id(&self) -> &'static str {
        "ARCH-DB-PARITY"
    }

    fn run(&self, context: &Context<'_>) -> anyhow::Result<Vec<Finding>> {
        let path = context.repo.path(CI_SOURCE);
        let text = fs::read_to_string(&path).unwrap_or_default();
        Ok(wiring_problem(&text)
            .map(|message| Finding::at(CI_SOURCE, message))
            .into_iter()
            .collect())
    }
}

/// The file that lists the steps of `cargo xtask ci`.
const CI_SOURCE: &str = "xtask/src/ci.rs";

/// The step that runs the repository tests on both databases and then checks the record.
const TEST_STEP: &str = r#"&["db", "run", "--", "cargo", "test", "--workspace"]"#;

/// Why the CI steps would not check the record, if they would not.
fn wiring_problem(ci_source: &str) -> Option<String> {
    (!ci_source.contains(TEST_STEP)).then(|| {
        format!(
            "`cargo xtask ci` has to run the Rust tests as `{TEST_STEP}`, so the tests run on both databases and the record is checked right after them."
        )
    })
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
    for database in DatabaseKind::ALL.iter().map(|kind| kind.code()) {
        if !ran.contains_key(database) {
            problems.push(format!(
                "No test ran on `{database}`. Repository tests must run on {}.",
                DatabaseKind::names(" and ")
            ));
        }
    }
    let all: BTreeSet<&str> = ran.values().flatten().copied().collect();
    for database in DatabaseKind::ALL.iter().map(|kind| kind.code()) {
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

    #[test]
    fn bad_example_ci_without_db_run_is_reported() {
        let ci = r#"steps(&["cargo", "test", "--workspace"])"#;
        assert!(wiring_problem(ci).is_some());
    }

    #[test]
    fn good_example_ci_runs_the_tests_through_db_run() {
        let ci = r#"step("Rust tests", &["db", "run", "--", "cargo", "test", "--workspace"])"#;
        assert!(wiring_problem(ci).is_none());
    }
}
