//! Time as a person counts it, written once: the units of time that more than one crate counts in,
//! the day of a moment for a person and the week it is in ([`day`]), the time zones the server knows
//! ([`zone`]), and the length of time a person types ([`length`]). Pure: no clock and no I/O, the
//! caller gives the moment.

pub mod day;
pub mod length;
pub mod zone;

pub use length::{Minutes, TIME_ENTRY_MINUTES_MAX};

use jiff::SignedDuration;

/// The seconds in a minute.
pub const SECONDS_PER_MINUTE: u32 = 60;

/// The minutes in an hour.
pub const MINUTES_PER_HOUR: u32 = 60;

/// The hours in a day on the clock (a day of a time zone may have 23 or 25, see [`day`]).
pub const HOURS_PER_DAY: u32 = 24;

/// The seconds in a day. A day here is 24 hours: the keep periods and lifetimes counted in days do
/// not follow the clocks of a time zone.
pub const SECONDS_PER_DAY: i64 = 86_400;

/// A number of days of 24 hours as a length of time: how a keep period or a lifetime counted in
/// days becomes the time between two moments. A count too large to hold is the longest length.
#[must_use]
pub const fn days(count: i64) -> SignedDuration {
    SignedDuration::from_secs(count.saturating_mul(SECONDS_PER_DAY))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn days_are_days_of_twenty_four_hours_and_never_overflow() {
        assert_eq!(days(2), SignedDuration::from_hours(48));
        assert_eq!(days(-1), SignedDuration::from_hours(-24));
        assert_eq!(days(i64::MAX), SignedDuration::from_secs(i64::MAX));
    }
}
