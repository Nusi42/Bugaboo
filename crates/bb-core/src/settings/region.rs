//! The choices of *Region* (CFG-04): how a date, a time of day and a number are written, and the
//! day a week starts on. The catalog names these lists; the values a server acts on are constants,
//! so the wording and the person's day name them instead of writing them again.

use super::Choice;
use crate::message;

/// The value of *Date format* for day, month, year.
pub const DATE_FORMAT_DAY_MONTH_YEAR: &str = "day-month-year";
/// The value of *Date format* for month, day, year.
pub const DATE_FORMAT_MONTH_DAY_YEAR: &str = "month-day-year";

/// The value of *Number format* for a comma before the decimals.
pub const NUMBER_FORMAT_COMMA: &str = "comma";
/// The value of *Number format* for a space between groups and a comma before the decimals.
pub const NUMBER_FORMAT_SPACE: &str = "space";

pub(super) const DATE_FORMAT_CHOICES: &[Choice] = &[
    Choice::new("device", message::THEME_DEVICE),
    Choice::new(
        DATE_FORMAT_DAY_MONTH_YEAR,
        message::SETTING_REGION_DATE_FORMAT_DAY_MONTH_YEAR,
    ),
    Choice::new(
        DATE_FORMAT_MONTH_DAY_YEAR,
        message::SETTING_REGION_DATE_FORMAT_MONTH_DAY_YEAR,
    ),
    Choice::new(
        "year-month-day",
        message::SETTING_REGION_DATE_FORMAT_YEAR_MONTH_DAY,
    ),
];

pub(super) const NUMBER_FORMAT_CHOICES: &[Choice] = &[
    Choice::new("device", message::THEME_DEVICE),
    Choice::new("point", message::SETTING_REGION_NUMBER_FORMAT_POINT),
    Choice::new(
        NUMBER_FORMAT_COMMA,
        message::SETTING_REGION_NUMBER_FORMAT_COMMA,
    ),
    Choice::new(
        NUMBER_FORMAT_SPACE,
        message::SETTING_REGION_NUMBER_FORMAT_SPACE,
    ),
];

/// The value of *First day of the week* for a week from Sunday to Saturday.
pub const FIRST_DAY_SUNDAY: &str = "sunday";
/// The value of *First day of the week* for a week from Saturday to Friday.
pub const FIRST_DAY_SATURDAY: &str = "saturday";

pub(super) const FIRST_DAY_CHOICES: &[Choice] = &[
    Choice::new("device", message::THEME_DEVICE),
    Choice::new("monday", message::SETTING_REGION_FIRST_DAY_MONDAY),
    Choice::new(FIRST_DAY_SUNDAY, message::SETTING_REGION_FIRST_DAY_SUNDAY),
    Choice::new(
        FIRST_DAY_SATURDAY,
        message::SETTING_REGION_FIRST_DAY_SATURDAY,
    ),
];

/// The value of *Time of day* for a clock of 24 hours (14:30).
pub const TIME_OF_DAY_24_HOUR: &str = "24-hour";
/// The value of *Time of day* for a clock of 12 hours (2:30 PM).
pub const TIME_OF_DAY_12_HOUR: &str = "12-hour";

/// The choices of *Time of day*. The first follows the date format: the clock of the device when
/// the date format is the device's, 24 hours with any date format the person fixed.
pub(super) const TIME_OF_DAY_CHOICES: &[Choice] = &[
    Choice::new("device", message::SETTING_REGION_TIME_OF_DAY_DEVICE),
    Choice::new(
        TIME_OF_DAY_24_HOUR,
        message::SETTING_REGION_TIME_OF_DAY_TWENTY_FOUR_HOURS,
    ),
    Choice::new(
        TIME_OF_DAY_12_HOUR,
        message::SETTING_REGION_TIME_OF_DAY_TWELVE_HOURS,
    ),
];
