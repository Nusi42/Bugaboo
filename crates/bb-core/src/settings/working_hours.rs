//! The settings of working hours (ADR-0035, decisions 6 to 8): the preset of the working-time
//! rules and each of its numbers, the keep period of work days and *Totals only*. They are part of
//! the registry like every other setting ([`super::registry`] lists them); they stand in a file of
//! their own so the main catalog does not grow past its size. Every change of one of them is also
//! written to the security log (PRIV-09, [`super::SettingDefinition::touches_security`]).

mod catalog;

pub use catalog::*;
