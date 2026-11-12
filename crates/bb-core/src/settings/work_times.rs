//! The settings of work times (PLAN-11, ADR-0030, amendment M8): which public holidays count
//! (*Holidays*, `work-times.holidays`) and whether a person is told when somebody else changes their
//! work times. They are part of the registry like every other setting ([`super::registry`] lists
//! them) and stand in a file of their own, like the settings of working hours.

mod catalog;

pub use catalog::*;
