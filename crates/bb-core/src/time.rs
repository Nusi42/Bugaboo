//! The units of time that more than one crate counts in, written once.

/// The seconds in a day. A day here is 24 hours: the keep periods and lifetimes counted in days do
/// not follow the clocks of a time zone.
pub const SECONDS_PER_DAY: i64 = 86_400;
