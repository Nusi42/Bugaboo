//! A length of time a person typed in the browser, read by the one reader of `bb-core`
//! ([`length::read`]), with the sentence of a mistake: what the module of the browser calls. The
//! types it takes and answers stand in the module above, where their TypeScript is made from.

use super::{LengthRead, LengthTyped};
use crate::time::Minutes;
use crate::time::length::{self, DecimalMark, LengthWords};

/// Reads a typed length of time without a bound (an own field), for the browser.
#[must_use]
pub fn length_read(typed: &LengthTyped) -> LengthRead {
    read_typed(typed, length::read)
}

/// Reads a typed length of one time entry, with its bound, for the browser.
#[must_use]
pub fn time_entry_read(typed: &LengthTyped) -> LengthRead {
    read_typed(typed, length::read_time_entry)
}

/// What a reader of [`length`] makes of a typed length, with the sentence of a mistake.
fn read_typed(
    typed: &LengthTyped,
    read: fn(&str, &LengthWords<'_>) -> Result<Minutes, length::LengthMistake>,
) -> LengthRead {
    let mark = DecimalMark::of_number_format(Some(&typed.number_format));
    let words = LengthWords {
        mark,
        hour: &typed.hour_unit,
        minute: &typed.minute_unit,
        day: &typed.day_units,
        day_minutes: typed.day_minutes,
    };
    match read(&typed.text, &words) {
        Ok(minutes) => LengthRead {
            minutes: Some(minutes.get()),
            mistake: None,
        },
        Err(mistake) => LengthRead {
            minutes: None,
            mistake: Some(mistake.said(mark)),
        },
    }
}
