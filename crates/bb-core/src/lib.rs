//! Domain types, IDs, error codes, domain events and field types. Does no I/O (architecture §4).

#![forbid(unsafe_code)]

// The derive macros write `::bb_core::…`, which inside this crate means the crate itself.
extern crate self as bb_core;

#[macro_use]
mod words;

pub mod access;
pub mod entities;
pub mod error_code;
pub mod event;
pub mod fields;
pub mod id;
pub mod job;
pub mod message;
pub mod migration;
pub mod network;
pub mod outbox;
pub mod paging;
pub mod people_kinds;
pub mod person;
pub mod personal;
pub mod project;
pub mod secret;
pub mod security_log;
pub mod settings;
pub mod sign_in;
pub mod technical;

pub use error_code::{ErrorCode, Problem};
pub use event::DomainEvent;
pub use words::UnknownWord;

// Used by the integration test in tests/compile_fail.rs only (`unused_crate_dependencies`).
#[cfg(test)]
use trybuild as _;
