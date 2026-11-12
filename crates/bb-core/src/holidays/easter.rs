//! The date of Easter Sunday in the Gregorian calendar (the anonymous algorithm of Meeus, Jones and
//! Butcher): every day that follows Easter counts from it.

use jiff::civil::Date;

/// Easter Sunday of a year; `None` only for a year the calendar cannot hold.
#[must_use]
pub(super) fn sunday_of(year: i16) -> Option<Date> {
    let year = i32::from(year);
    let golden = year % 19;
    let century = year / 100;
    let rest = year % 100;
    let leap_centuries = century / 4;
    let century_in_four = century % 4;
    let moon_correction = (century + 8) / 25;
    let moon_shift = (century - moon_correction + 1) / 3;
    let epact = (19 * golden + century - leap_centuries - moon_shift + 15) % 30;
    let leap_years = rest / 4;
    let leap_rest = rest % 4;
    let weekday_offset = (32 + 2 * century_in_four + 2 * leap_years - epact - leap_rest) % 7;
    let correction = (golden + 11 * epact + 22 * weekday_offset) / 451;
    let month = (epact + weekday_offset - 7 * correction + 114) / 31;
    let day = (epact + weekday_offset - 7 * correction + 114) % 31 + 1;
    Date::new(
        i16::try_from(year).ok()?,
        i8::try_from(month).ok()?,
        i8::try_from(day).ok()?,
    )
    .ok()
}
