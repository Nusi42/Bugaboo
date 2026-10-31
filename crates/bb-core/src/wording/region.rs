//! How a date, a moment, a time of day and a number are written for a reader of an e-mail: the part
//! of the wording that depends on what a person chose under *Region*. The screen writes the same
//! with `Intl` (`web/app/src/shared/format.ts`), because its choice *device* uses what the browser
//! does, which the server cannot run; the two are held together by the vectors in
//! `tools/testdata/value-words.json`, which `cargo xtask gen word-vectors` writes from this code
//! (ADR-0033, the amendment about value wording).
//!
//! The screen writes days, moments and numbers the way the person chose under *Region*, and lets
//! the browser fill in what the person left on *device*. A server does not know the device, so an
//! e-mail takes what the organization chose for that setting, and the usual way (year, month, day;
//! a clock of 24 hours; a point before decimals; UTC; weeks from Monday) when the organization chose
//! nothing either. A moment is followed by the name of its time zone, because an e-mail is read away
//! from the screen.

use jiff::Timestamp;
use jiff::civil::Date;
use jiff::tz::TimeZone;
use serde_json::Value;

use crate::message;
use crate::own_fields::FieldType;
use crate::settings::{
    DATE_FORMAT_DAY_MONTH_YEAR, DATE_FORMAT_MONTH_DAY_YEAR, DEVICE, Layers, NUMBER_FORMAT_COMMA,
    NUMBER_FORMAT_SPACE, REGION_DATE_FORMAT, REGION_FIRST_DAY, REGION_NUMBER_FORMAT,
    REGION_TIME_OF_DAY, REGION_TIME_ZONE, Setting, TIME_OF_DAY_12_HOUR, Text,
};
use crate::time::day::{FirstDay, Week};
use crate::time::length::DecimalMark;
use crate::time::zone::{self, UTC};

const HOURS_ON_A_CLOCK: i8 = 12;
/// The digits in a group of a big number.
const GROUP: usize = 3;
/// How many decimals a number is written with at the most (as the browser writes it).
const DECIMALS_MOST: usize = 3;

/// In which order a day is written; the values of the setting *Date format*.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DateOrder {
    DayMonthYear,
    MonthDayYear,
    YearMonthDay,
}

/// The clock a time of day is written on; the values of the setting *Time of day*.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Clock {
    /// `14:30`: what every fixed date format starts with, so what a server writes for *device*.
    TwentyFour,
    /// `2:30 PM`.
    Twelve,
}

/// The marks of a number: between groups of digits, and before the decimals.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct NumberMarks {
    group: &'static str,
    decimal: DecimalMark,
}

/// The values of the settings of *Region*, as a server knows them: `None` is *device* with nothing
/// to take its place.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Choices<'a> {
    /// *Date format*.
    pub date_format: Option<&'a str>,
    /// *Time of day*.
    pub time_of_day: Option<&'a str>,
    /// *Number format*.
    pub number_format: Option<&'a str>,
    /// *Time zone*.
    pub time_zone: Option<&'a str>,
    /// *First day of the week*.
    pub first_day: Option<&'a str>,
}

/// What the person chose under *Region* that an e-mail can follow.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Region {
    dates: DateOrder,
    clock: Clock,
    numbers: NumberMarks,
    zone_name: String,
    zone: TimeZone,
    first_day: FirstDay,
}

/// The value of a setting of *Region* for an e-mail: the person's, or the organization's where the
/// person left it on *device*, or `None` where both did.
fn chosen(layers: &Layers, setting: &Setting<Text>) -> Option<String> {
    let mut value = layers.text(setting);
    if value == DEVICE {
        let mut organization = layers.clone();
        organization.person.clear();
        value = organization.text(setting);
    }
    (value != DEVICE).then_some(value)
}

impl Region {
    /// The time zone of the region: where a day begins for the person.
    #[must_use]
    pub const fn zone(&self) -> &TimeZone {
        &self.zone
    }

    /// The day a week of the person starts on.
    #[must_use]
    pub const fn first_day(&self) -> FirstDay {
        self.first_day
    }

    /// The week of the person that `date` is in, by their *First day of the week* (see
    /// [`Week::of`]).
    #[must_use]
    pub fn week_of(&self, date: Date) -> Option<Week> {
        Week::of(date, self.first_day)
    }

    /// The mark before the decimals of a number, also of a length of time typed in hours.
    #[must_use]
    pub const fn decimal_mark(&self) -> DecimalMark {
        self.numbers.decimal
    }

    /// What the layers of a person say.
    #[must_use]
    pub fn of(layers: &Layers) -> Self {
        let [
            date_format,
            time_of_day,
            number_format,
            time_zone,
            first_day,
        ] = [
            &REGION_DATE_FORMAT,
            &REGION_TIME_OF_DAY,
            &REGION_NUMBER_FORMAT,
            &REGION_TIME_ZONE,
            &REGION_FIRST_DAY,
        ]
        .map(|setting| chosen(layers, setting));
        Self::from_choices(&Choices {
            date_format: date_format.as_deref(),
            time_of_day: time_of_day.as_deref(),
            number_format: number_format.as_deref(),
            time_zone: time_zone.as_deref(),
            first_day: first_day.as_deref(),
        })
    }

    /// The region for the values of the settings. A zone that the server does not know counts as
    /// none.
    #[must_use]
    pub fn from_choices(choices: &Choices<'_>) -> Self {
        let dates = match choices.date_format {
            Some(DATE_FORMAT_DAY_MONTH_YEAR) => DateOrder::DayMonthYear,
            Some(DATE_FORMAT_MONTH_DAY_YEAR) => DateOrder::MonthDayYear,
            _ => DateOrder::YearMonthDay,
        };
        // *device* follows the date format, and a server writes every date in a fixed format.
        let clock = match choices.time_of_day {
            Some(TIME_OF_DAY_12_HOUR) => Clock::Twelve,
            _ => Clock::TwentyFour,
        };
        let group = match choices.number_format {
            Some(NUMBER_FORMAT_COMMA) => ".",
            Some(NUMBER_FORMAT_SPACE) => "\u{202F}",
            _ => ",",
        };
        let numbers = NumberMarks {
            group,
            decimal: DecimalMark::of_number_format(choices.number_format),
        };
        let (zone_name, zone) = choices
            .time_zone
            .and_then(|name| zone::named(name).map(|zone| (name.to_owned(), zone)))
            .unwrap_or_else(|| (UTC.to_owned(), TimeZone::UTC));
        Self {
            dates,
            clock,
            numbers,
            zone_name,
            zone,
            first_day: FirstDay::of_choice(choices.first_day),
        }
    }

    /// The words of a value of a kind that depends on the region: a number, a day or a moment, as
    /// the screen writes them for the same choices. `None` for any other kind, and for a value that
    /// does not fit its kind (the screen shows nothing for it either).
    #[must_use]
    pub fn worded(&self, kind: FieldType, value: &Value) -> Option<String> {
        match (kind, value) {
            (FieldType::WholeNumber | FieldType::Decimal, Value::Number(number)) => {
                Some(self.number(number))
            }
            (FieldType::Date, Value::String(day)) => {
                day.parse::<Date>().ok().map(|day| self.date(day))
            }
            (FieldType::DateTime, Value::String(moment)) => {
                moment.parse::<Timestamp>().ok().map(|at| self.moment(at))
            }
            _ => None,
        }
    }

    /// Like [`Region::worded`], and a moment is followed by the name of its time zone, because an
    /// e-mail is read away from the screen.
    #[must_use]
    pub fn worded_for_mail(&self, kind: FieldType, value: &Value) -> Option<String> {
        let words = self.worded(kind, value)?;
        Some(if kind == FieldType::DateTime {
            message::mail_date_time(words, &self.zone_name).render()
        } else {
            words
        })
    }

    /// A date in the person's order: `05.10.2026`, `10/05/2026` or `2026-10-05`.
    #[must_use]
    pub fn date(&self, date: Date) -> String {
        let (year, month, day) = (date.year(), date.month(), date.day());
        match self.dates {
            DateOrder::DayMonthYear => format!("{day:02}.{month:02}.{year:04}"),
            DateOrder::MonthDayYear => format!("{month:02}/{day:02}/{year:04}"),
            DateOrder::YearMonthDay => format!("{year:04}-{month:02}-{day:02}"),
        }
    }

    /// The time of day of a moment in the person's zone, on the person's clock: `09:30` or
    /// `9:30 AM`.
    #[must_use]
    pub fn time_of_day(&self, moment: Timestamp) -> String {
        let zoned = moment.to_zoned(self.zone.clone());
        let (hour, minute) = (zoned.hour(), format!("{:02}", zoned.minute()));
        match self.clock {
            Clock::TwentyFour => message::mail_time_24(format!("{hour:02}"), minute).render(),
            Clock::Twelve => {
                let on_clock = match hour % HOURS_ON_A_CLOCK {
                    0 => HOURS_ON_A_CLOCK,
                    hour => hour,
                };
                if hour < HOURS_ON_A_CLOCK {
                    message::mail_time_am(on_clock, minute).render()
                } else {
                    message::mail_time_pm(on_clock, minute).render()
                }
            }
        }
    }

    /// A date and a time of day in the person's zone, as the screen writes them: the date in the
    /// person's order, the time on the person's clock.
    #[must_use]
    pub fn moment(&self, moment: Timestamp) -> String {
        let date = moment.to_zoned(self.zone.clone()).date();
        format!("{}, {}", self.date(date), self.time_of_day(moment))
    }

    fn number(&self, number: &serde_json::Number) -> String {
        let written = rounded_half_up(&number.to_string());
        let (sign, digits) = written
            .strip_prefix('-')
            .map_or(("", written.as_str()), |rest| ("-", rest));
        let (whole, decimals) = digits.split_once('.').unwrap_or((digits, ""));
        let mut grouped = String::new();
        for (at, digit) in whole.chars().enumerate() {
            if at > 0 && (whole.len() - at).is_multiple_of(GROUP) {
                grouped.push_str(self.numbers.group);
            }
            grouped.push(digit);
        }
        if decimals.is_empty() {
            format!("{sign}{grouped}")
        } else {
            format!(
                "{sign}{grouped}{}{decimals}",
                self.numbers.decimal.character()
            )
        }
    }
}

/// A number as plain digits with at most [`DECIMALS_MOST`] decimals, rounded half up on the digits
/// as they are written (the browser's rule: 0.0625 is 0.063, not 0.062).
fn rounded_half_up(written: &str) -> String {
    let (sign, digits) = written
        .strip_prefix('-')
        .map_or(("", written), |rest| ("-", rest));
    let Some((whole, decimals)) = digits.split_once('.') else {
        return written.to_owned();
    };
    if decimals.len() <= DECIMALS_MOST {
        return written.to_owned();
    }
    let (kept, rest) = decimals.split_at(DECIMALS_MOST);
    let mut digits: Vec<u8> = format!("{whole}{kept}").bytes().collect();
    if rest.starts_with(['5', '6', '7', '8', '9']) {
        // Carry: 9 becomes 0 and the digit before it goes up; a new leading 1 if all were nines.
        let mut at = digits.len();
        loop {
            if at == 0 {
                digits.insert(0, b'1');
                break;
            }
            at -= 1;
            if digits[at] == b'9' {
                digits[at] = b'0';
            } else {
                digits[at] += 1;
                break;
            }
        }
    }
    let text = String::from_utf8_lossy(&digits).into_owned();
    let (whole, kept) = text.split_at(text.len() - DECIMALS_MOST);
    let kept = kept.trim_end_matches('0');
    if kept.is_empty() {
        format!("{sign}{whole}")
    } else {
        format!("{sign}{whole}.{kept}")
    }
}

#[cfg(test)]
mod tests;
