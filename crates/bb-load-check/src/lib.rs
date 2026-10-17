//! The invariants after a run of many people at once, written once (architecture §11,
//! TEST-CONCURRENT). The test of the server (`test_concurrent_invariants_hold_after_a_run`, in
//! memory, on both databases) and `cargo xtask load` (the driver writes its book to a file) both
//! call [`check`]. The rules are in [`judge`]; this crate reads the database only through
//! `bb-store`, so no query is written twice.

#![forbid(unsafe_code)]

mod book;
mod facts;
mod judge;

pub use book::{Book, DraftEnd, Drafted, Kept, Seen, Work};
pub use facts::{Facts, Failure, LockRow, NotificationFacts, gather, lock_rows};
pub use judge::{Afterwards, judge, locks_beyond_lease};

use bb_store::Database;

// Only the program uses these; Cargo gives every dependency to the library too
// (`unused_crate_dependencies`).
use serde_json as _;
use tokio as _;

/// Reads the database and judges the run against the book; what does not hold is listed in words.
///
/// # Errors
/// Fails when the database cannot be read or the book names something that is not there.
pub async fn check(
    database: &Database,
    book: &Book,
    afterwards: &Afterwards,
) -> Result<Vec<String>, Failure> {
    Ok(judge(book, &gather(database, book).await?, afterwards))
}
