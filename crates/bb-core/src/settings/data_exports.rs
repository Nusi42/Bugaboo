//! The settings of the notifications of *Download my data* (ADR-0044, decision 2): the two switches
//! of *Your data is ready*. *An administrator downloaded your data* has none, because nobody can
//! switch it off (decision 4). They are part of the registry ([`super::registry`] lists them) and
//! stand in a file of their own, like the settings of the notifications of agents.

mod catalog;

pub use catalog::*;
