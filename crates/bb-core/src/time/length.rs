//! The length of time a person types, read once for the server and the browser (the browser runs
//! this code through `bb-wasm`): `1:30`, `1.5` hours with the decimal mark of the person's *Number
//! format* (`1,5` in Germany), `90m`, `1h 30m` and `2h` are all 90 or 120 minutes. A bare number is
//! hours, like `1.5`. One entry is at most a day of 24 hours; anything else is a plain mistake.
//!
//! The units are the short words of the catalog (`fields.unit-hour`, `fields.unit-minute`), the
//! ones a length of an own field is written with. The search language keeps its own words
//! (`1 hour 30 minutes`, ADR-0005, decision 10) and counts in the same minutes.

use serde::{Deserialize, Serialize};

use super::{HOURS_PER_DAY, MINUTES_PER_HOUR, SECONDS_PER_MINUTE};
use crate::message;
use crate::settings::{NUMBER_FORMAT_COMMA, NUMBER_FORMAT_SPACE};
use crate::wording::Said;

/// A length of time in whole minutes: the one type of it (a time entry, time spent, the reader of
/// typed lengths). In the API it is a plain number of minutes (`serde(transparent)`); `TryFrom<i64>`
/// takes such a number and refuses a negative one or one too large, `From` gives it back.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Minutes(u32);

impl Minutes {
    /// A length of `minutes` minutes.
    #[must_use]
    pub const fn new(minutes: u32) -> Self {
        Self(minutes)
    }

    /// The minutes.
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }

    /// The sum of two lengths; `None` when it is too long to count.
    #[must_use]
    pub const fn checked_add(self, other: Self) -> Option<Self> {
        match self.0.checked_add(other.0) {
            Some(sum) => Some(Self(sum)),
            None => None,
        }
    }

    /// What is left of this length after `other`; `None` when `other` is longer.
    #[must_use]
    pub const fn checked_sub(self, other: Self) -> Option<Self> {
        match self.0.checked_sub(other.0) {
            Some(rest) => Some(Self(rest)),
            None => None,
        }
    }

    /// The whole minutes in a number of seconds, the rest of a minute dropped (a length of an own
    /// field is seconds in the API); `None` for a negative number or one too long to count.
    #[must_use]
    pub fn from_seconds(seconds: i64) -> Option<Self> {
        let minutes = seconds.checked_div(i64::from(SECONDS_PER_MINUTE))?;
        (seconds >= 0).then_some(())?;
        u32::try_from(minutes).ok().map(Self)
    }

    /// The length in seconds.
    #[must_use]
    pub fn as_seconds(self) -> i64 {
        i64::from(self.0) * i64::from(SECONDS_PER_MINUTE)
    }

    /// The length on a clock, as a timesheet shows it: `1:30`, `0:05`, `24:00`.
    #[must_use]
    pub fn on_clock(self) -> String {
        format!(
            "{}:{:02}",
            self.0 / MINUTES_PER_HOUR,
            self.0 % MINUTES_PER_HOUR
        )
    }
}

impl TryFrom<i64> for Minutes {
    type Error = std::num::TryFromIntError;

    fn try_from(minutes: i64) -> Result<Self, Self::Error> {
        u32::try_from(minutes).map(Self)
    }
}

impl From<Minutes> for i64 {
    fn from(minutes: Minutes) -> Self {
        Self::from(minutes.0)
    }
}

/// The longest one time entry may be: a day of 24 hours (1,440 minutes). The reader of typed
/// lengths refuses more, and the server of time entries holds an entry to it.
pub const TIME_ENTRY_MINUTES_MAX: Minutes = Minutes::new(HOURS_PER_DAY * MINUTES_PER_HOUR);

/// The mark before the decimals of an hour, as the person's *Number format* writes it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DecimalMark {
    /// `1.5`.
    #[default]
    Point,
    /// `1,5`.
    Comma,
}

impl DecimalMark {
    /// The mark of a value of the setting *Number format*: a comma for `comma` and `space`, a point
    /// for any other (the usual way, also where a server does not know the device).
    #[must_use]
    pub fn of_number_format(value: Option<&str>) -> Self {
        match value {
            Some(NUMBER_FORMAT_COMMA | NUMBER_FORMAT_SPACE) => Self::Comma,
            _ => Self::Point,
        }
    }

    /// The mark itself.
    #[must_use]
    pub const fn character(self) -> char {
        match self {
            Self::Point => '.',
            Self::Comma => ',',
        }
    }

    /// An hour and a half, written with this mark: the example in a mistake.
    fn example(self) -> String {
        format!("1{}5", self.character())
    }
}

/// What is wrong with a typed length of time.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LengthMistake {
    /// The text is no length of time.
    NotALength,
    /// The length is longer than one entry may be.
    TooLong,
}

impl LengthMistake {
    /// The sentence that says what is wrong and how to write it.
    #[must_use]
    pub fn said(self, mark: DecimalMark) -> Said {
        match self {
            Self::NotALength => Said::with(
                &message::time_length_not_a_length(""),
                vec![Said::text(mark.example())],
            ),
            Self::TooLong => Said::with(
                &message::time_length_too_long(""),
                vec![Said::text(
                    (TIME_ENTRY_MINUTES_MAX.get() / MINUTES_PER_HOUR).to_string(),
                )],
            ),
        }
    }
}

/// The most digits of the whole hours or minutes, and of the decimals of an hour, that are read.
/// More could not be a length of one entry, and fewer keep the sums far from overflowing.
const DIGITS_MOST: usize = 9;
/// Ten to the power of the decimals of an hour, for rounding them to whole minutes.
const TEN: u64 = 10;

/// A number as it was typed: its whole part and its decimals (with how many digits they had).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Typed {
    whole: u64,
    decimals: u64,
    places: u32,
}

impl Typed {
    /// The minutes of this many hours, rounded half up to a whole minute.
    fn hours_in_minutes(self) -> Option<u64> {
        let per_hour = u64::from(MINUTES_PER_HOUR);
        let scale = TEN.checked_pow(self.places)?;
        let part = (self.decimals * per_hour * 2 + scale) / (scale * 2);
        self.whole.checked_mul(per_hour)?.checked_add(part)
    }
}

/// The digits at the start of `text`: their value and the rest of the text.
fn digits(text: &str) -> Option<(u64, usize, &str)> {
    let count = text.chars().take_while(char::is_ascii_digit).count();
    if count == 0 || count > DIGITS_MOST {
        return None;
    }
    let (number, rest) = text.split_at(count);
    Some((number.parse().ok()?, count, rest))
}

/// A number at the start of `text`, with decimals after `mark` (`1.5`, `,5`, `2`), and the rest.
fn number(text: &str, mark: char) -> Option<(Typed, &str)> {
    let (whole, rest) = match digits(text) {
        Some((whole, _, rest)) => (Some(whole), rest),
        None => (None, text),
    };
    let Some(after_mark) = rest.strip_prefix(mark) else {
        return whole.map(|whole| {
            let typed = Typed {
                whole,
                decimals: 0,
                places: 0,
            };
            (typed, rest)
        });
    };
    let (decimals, places, rest) = digits(after_mark)?;
    let typed = Typed {
        whole: whole.unwrap_or(0),
        decimals,
        places: u32::try_from(places).ok()?,
    };
    Some((typed, rest))
}

/// `1:30`: whole hours, a colon, and two digits of minutes under 60.
fn on_clock(text: &str) -> Option<u64> {
    let (hours, minutes) = text.split_once(':')?;
    let (hours, _, rest) = digits(hours)?;
    let (minutes, count, after) = digits(minutes)?;
    (rest.is_empty() && after.is_empty() && count == 2 && minutes < u64::from(MINUTES_PER_HOUR))
        .then(|| hours * u64::from(MINUTES_PER_HOUR) + minutes)
}

/// How a person writes a length: the decimal mark of their *Number format*, and the short words of
/// the units in their language (`fields.unit-hour`, `fields.unit-minute`). The caller hands the
/// words in, so the module of the browser carries no catalog: it takes them from its own.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LengthWords<'a> {
    /// The mark before the decimals of an hour.
    pub mark: DecimalMark,
    /// The short word of an hour: `h`.
    pub hour: &'a str,
    /// The short word of a minute: `m`.
    pub minute: &'a str,
}

impl LengthWords<'static> {
    /// The words of the catalog the server holds, with `mark`.
    #[must_use]
    pub fn of_catalog(mark: DecimalMark) -> Self {
        Self {
            mark,
            hour: message::FIELDS_UNIT_HOUR.template(),
            minute: message::FIELDS_UNIT_MINUTE.template(),
        }
    }
}

/// A unit at the start of `text` (after spaces): whether it is hours, and the rest.
fn unit<'t>(text: &'t str, words: &LengthWords<'_>) -> Option<(bool, &'t str)> {
    let text = text.trim_start();
    let lower = text.to_lowercase();
    let starts = |word: &str| {
        let word = word.to_lowercase();
        (!word.is_empty() && lower.starts_with(&word)).then(|| text.get(word.len()..))
    };
    if let Some(rest) = starts(words.hour) {
        return rest.map(|rest| (true, rest));
    }
    starts(words.minute).and_then(|rest| rest.map(|rest| (false, rest)))
}

/// `1.5`, `2`, `90m`, `1h 30m`, `1.5h`: a bare number of hours, or numbers each with a unit, hours
/// before minutes, each unit once. Only hours may have decimals.
fn in_units(text: &str, words: &LengthWords<'_>) -> Option<u64> {
    let mark = words.mark.character();
    let (first, rest) = number(text, mark)?;
    if rest.trim().is_empty() {
        return first.hours_in_minutes();
    }
    let (hours, rest) = unit(rest, words)?;
    if !hours {
        return (first.places == 0 && rest.trim().is_empty()).then_some(first.whole);
    }
    let from_hours = first.hours_in_minutes()?;
    let rest = rest.trim_start();
    if rest.is_empty() {
        return Some(from_hours);
    }
    let (minutes, rest) = number(rest, mark)?;
    let (also_hours, rest) = unit(rest, words)?;
    (minutes.places == 0 && !also_hours && rest.trim().is_empty())
        .then(|| from_hours.checked_add(minutes.whole))
        .flatten()
}

/// The minutes of a length of time a person typed for one entry, or what is wrong with it.
///
/// # Errors
/// [`LengthMistake::NotALength`] for text that is no length of time in any of the forms,
/// [`LengthMistake::TooLong`] for a length longer than [`TIME_ENTRY_MINUTES_MAX`].
pub fn read(text: &str, words: &LengthWords<'_>) -> Result<Minutes, LengthMistake> {
    let text = text.trim();
    let minutes = if text.contains(':') {
        on_clock(text)
    } else {
        in_units(text, words)
    }
    .ok_or(LengthMistake::NotALength)?;
    u32::try_from(minutes)
        .ok()
        .map(Minutes::new)
        .filter(|minutes| *minutes <= TIME_ENTRY_MINUTES_MAX)
        .ok_or(LengthMistake::TooLong)
}

#[cfg(test)]
mod tests;
