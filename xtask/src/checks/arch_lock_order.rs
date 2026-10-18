//! ARCH-LOCK-ORDER: advisory locks are taken in one order, and a transaction refuses a wrong one
//! (architecture §07 "Locks", ADR-0031 amendment 5).
//!
//! Two changes that take two locks in different orders can wait for each other. On SQLite a lock
//! does nothing, so a wrong order would show only as a PostgreSQL deadlock under load. The
//! transaction therefore records the classes it took, on both databases, and refuses a new lower one
//! or a shared lock taken alone (an error in debug builds and in every test build, release ones
//! too; the program only logs it, `crates/bb-store/src/locks.rs`). This check keeps the guard from
//! being deleted or switched off (its tests are required, like any list of required tests), and
//! keeps the SQL of a lock in the one file that may write SQL: the word `advisory` appears in no
//! other file of any crate (the two methods of the guard apart), so no lock can be taken without
//! passing the guard.

use super::required_tests::{Required, crate_sources, evaluate};
use super::{Check, Context, Finding};
use crate::rust_source::strip_comments;
use crate::source::Source;

const GUARD: &str = "crates/bb-store/src/locks.rs";
/// The files that speak of advisory locks: `db.rs` writes the SQL, `locks.rs` keeps the order.
const HOMES: [&str; 2] = ["crates/bb-store/src/db.rs", GUARD];
/// What a lock is called in SQL (`pg_advisory_xact_lock`, `pg_try_advisory_lock`, …).
const SQL_NAME: &str = "advisory";
/// The methods of `Transaction` that take a lock through the guard; their name is not SQL.
const GUARDED: [&str; 2] = [".advisory_lock_shared(", ".advisory_lock("];

/// The tests of the guard.
pub(super) const REQUIRED: [Required; 7] = [
    Required::anywhere(
        "arch_lock_order_the_classes_in_the_written_order_and_the_same_class_again_are_allowed",
    )
    .within(GUARD),
    Required::anywhere("arch_lock_order_a_class_taken_after_a_higher_one_is_refused").within(GUARD),
    Required::anywhere("arch_lock_order_a_class_held_shared_cannot_be_taken_alone").within(GUARD),
    Required::anywhere(
        "arch_lock_order_a_wrong_order_fails_in_every_test_build_release_ones_included",
    )
    .within(GUARD),
    Required::anywhere("arch_lock_order_the_order_is_the_written_one_and_covers_every_class")
        .within(GUARD),
    Required::anywhere("arch_lock_order_the_key_is_the_last_four_bytes_of_the_id").within(GUARD),
    Required::both("arch_lock_order_a_transaction_refuses_a_lower_class_than_one_it_holds")
        .within(GUARD),
];

pub(crate) struct ArchLockOrder;

impl Check for ArchLockOrder {
    fn id(&self) -> &'static str {
        "ARCH-LOCK-ORDER"
    }

    fn run(&self, context: &Context<'_>) -> anyhow::Result<Vec<Finding>> {
        Ok(findings(&crate_sources(context.repo)?))
    }
}

/// Whether the code of a file names an advisory lock other than by a guarded method.
fn speaks_of_a_lock(text: &str) -> bool {
    let mut code = strip_comments(text);
    for method in GUARDED {
        code = code.replace(method, "");
    }
    code.contains(SQL_NAME)
}

fn findings(sources: &[Source]) -> Vec<Finding> {
    let mut found = evaluate(
        sources,
        &REQUIRED,
        "The order of the advisory locks (architecture §07) is kept by it;",
    );
    found.extend(
        sources
            .iter()
            .filter(|(path, _)| !HOMES.contains(&path.as_str()))
            .filter(|(_, text)| speaks_of_a_lock(text))
            .map(|(path, _)| {
                Finding::at(
                    path,
                    format!(
                        "A lock is taken here without passing the guard of the order. Take it with `Transaction::advisory_lock` or `advisory_lock_shared`; only db.rs and locks.rs speak of `{SQL_NAME}` locks."
                    ),
                )
            }),
    );
    found
}

#[cfg(test)]
mod tests {
    use super::super::required_tests::examples;
    use super::*;

    #[test]
    fn good_example_this_repository_has_the_guard_and_its_tests() {
        examples::this_repository_has_every_test(&REQUIRED);
    }

    #[test]
    fn bad_example_without_the_guard_every_test_is_missing() {
        examples::without_the_sources_every_test_is_missing(&REQUIRED);
    }

    #[test]
    fn bad_example_a_lock_taken_outside_the_one_file() {
        let sources = vec![
            (
                "crates/bb-store/src/db.rs".to_owned(),
                "select pg_advisory_xact_lock(1, 2)".to_owned(),
            ),
            (
                "crates/bb-api/src/other.rs".to_owned(),
                "// pg_advisory in a comment is fine
let sql = \"select pg_try_advisory_lock(1)\";"
                    .to_owned(),
            ),
            (
                "crates/bb-store/src/files.rs".to_owned(),
                "transaction.advisory_lock(LockClass::Uploads, 1).await?;
// pg_advisory in a comment
"
                .to_owned(),
            ),
        ];
        let found: Vec<_> = findings(&sources)
            .into_iter()
            .filter(|finding| finding.location.is_some())
            .collect();
        assert_eq!(found.len(), 1);
        assert_eq!(
            found[0].location.as_deref(),
            Some("crates/bb-api/src/other.rs")
        );
    }
}
