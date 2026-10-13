//! The history of an issue (ISS-06, SEC-06, ADR-0007): a read model built from the domain events by
//! an outbox consumer, and the page that reads it.
//!
//! - `map.rs`: which events belong in the history of which issue (pure).
//! - `build.rs`: the consumer; idempotent, resumes from the place it recorded.
//! - `read.rs`: the rows shaped for one reader, with the tabs *Everything*, *Comments* and
//!   *Changes*; a change to a field the reader may not see is left out.

mod build;
mod map;
mod read;
mod store;
mod types;

pub use build::{catch_up, catch_up_batch};
pub use read::history;
pub use store::HistoryStore;
pub use types::*;
