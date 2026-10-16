//! Domain types, IDs, error codes, domain events and field types. Does no I/O (architecture §4).

#![forbid(unsafe_code)]

// The derive macros write `::bb_core::…`, which inside this crate means the crate itself.
extern crate self as bb_core;

#[macro_use]
pub mod words;

pub mod access;
pub mod access_key;
pub mod comment;
pub mod edit;
pub mod entities;
pub mod error_code;
pub mod event;
pub mod fields;
pub mod follow;
pub mod hex;
pub mod history;
pub mod id;
pub mod issue;
pub mod issue_key;
pub mod job;
pub mod layout;
pub mod lookup;
pub mod many_changes;
pub mod message;
pub mod migration;
pub mod network;
pub mod outbox;
pub mod own_fields;
pub mod own_fields_value;
pub mod paging;
pub mod people_kinds;
pub mod person;
pub mod personal;
pub mod plain;
pub mod project;
pub mod relation;
pub mod secret;
pub mod security_log;
pub mod settings;
pub mod sign_in;
pub mod tags;
pub mod technical;
pub mod text;
pub mod time;
pub mod version;

pub use error_code::{ErrorCode, Problem};
pub use event::DomainEvent;
pub use words::UnknownWord;

// Used by the integration test in tests/compile_fail.rs only (`unused_crate_dependencies`).
#[cfg(test)]
use trybuild as _;
