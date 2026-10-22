//! The units of time that more than one crate counts in, written once.

/// The seconds in a minute.
pub const SECONDS_PER_MINUTE: u32 = 60;

/// The minutes in an hour.
pub const MINUTES_PER_HOUR: u32 = 60;

/// The seconds in a day. A day here is 24 hours: the keep periods and lifetimes counted in days do
/// not follow the clocks of a time zone.
pub const SECONDS_PER_DAY: i64 = 86_400;
