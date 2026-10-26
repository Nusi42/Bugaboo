//! The index of the full-text search (ISS-13, ADR-0016 amendment M5, ADR-0007): a read model that
//! an outbox consumer keeps, beside the history and the notifications.
//!
//! - `map.rs`: which events touch the texts of which issue (pure).
//! - `build.rs`: the consumer; idempotent, resumes from the place it recorded.
//! - `store.rs`: what it needs from storage.
//!
//! What a reader may find is decided by the search, never here.

mod build;
mod map;
mod store;

pub use build::{catch_up, catch_up_batch};
pub use map::{Touched, touched};
pub use store::TextIndexStore;
