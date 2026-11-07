//! The history of an issue (ISS-06, SEC-06, ADR-0007): a read model built from the domain events by
//! an outbox consumer, and the page that reads it.
//!
//! - `map.rs`: which events belong in the history of which issue (pure).
//! - `build.rs`: the consumer; idempotent, resumes from the place it recorded.
//! - `read.rs`: the rows shaped for one reader, with the tabs *Everything*, *Comments* and
//!   *Changes*; a change to a field the reader may not see is left out.
//! - `page.rs`: the page of the history of an issue for one reader.
//! - `dated.rs`: the same page for a reader who may not see the moment of a time entry: placed by
//!   the date of those rows.
//! - `walk.rs`: the rows walked past what the reader may not see, for the cursor.

mod build;
mod dated;
mod map;
mod names;
mod opaque;
mod page;
mod read;
mod run_mark;
mod store;
mod time;
mod types;
mod walk;

pub use build::{catch_up, catch_up_batch};
pub(crate) use map::issue_in;
pub use opaque::opaque_id;
pub use page::history;
pub(crate) use read::{ChangeReader, visible_changes};
pub use store::{HistoryStore, ShownKey};
pub use time::shown_moment;
pub use types::*;
