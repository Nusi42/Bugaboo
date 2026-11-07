//! The settings of the notifications of agent work (ADR-0033, amendment M7). They stand in a file
//! of their own like the settings of working hours, and are part of the registry
//! ([`super::registry`] lists them).

mod catalog;

pub use catalog::*;
