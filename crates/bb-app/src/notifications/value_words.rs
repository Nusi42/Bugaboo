//! The words of the values in an e-mail: a status, a priority, yes or no, a length of time, a
//! number, a day, a moment and a list of names, worded the way the screen words them (the history
//! of an issue, `web/app/src/shared/change-value.ts`), so a person reads one thing in both places.
//!
//! The screen writes days, moments and numbers the way the person chose under *Region*, and lets
//! the browser fill in what the person left on *device*. A server does not know the device and has
//! no list of time zones, so an e-mail writes a day in the order the person chose (the usual
//! year, month, day while they chose none), a number with the marks they chose, and a moment in
//! UTC, and says so.

use bb_core::fields::Field;
use bb_core::issue::Priority;
use bb_core::message::{self, Message};
use bb_core::own_fields::FieldType;
use bb_core::settings::{Layers, REGION_DATE_FORMAT, REGION_NUMBER_FORMAT};
use bb_core::workflow::starter::STARTERS;
use jiff::Timestamp;
use jiff::civil::Date;
use serde_json::Value;

const SECONDS_PER_MINUTE: i64 = 60;
const MINUTES_PER_HOUR: i64 = 60;
/// The digits in a group of a big number.
const GROUP: usize = 3;

/// In which order a day is written; the values of the setting *Date format*.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DateOrder {
    DayMonthYear,
    MonthDayYear,
    YearMonthDay,
}

/// The marks of a number: between groups of digits, and before the decimals.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct NumberMarks {
    group: &'static str,
    decimal: &'static str,
}

/// What the person chose under *Region* that an e-mail can follow.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Region {
    dates: DateOrder,
    numbers: NumberMarks,
}

impl Region {
    /// What the layers of a person say; what is left on *device* is written in the usual way.
    pub(crate) fn of(layers: &Layers) -> Self {
        let dates = match layers.text(&REGION_DATE_FORMAT).as_str() {
            "day-month-year" => DateOrder::DayMonthYear,
            "month-day-year" => DateOrder::MonthDayYear,
            _ => DateOrder::YearMonthDay,
        };
        let numbers = match layers.text(&REGION_NUMBER_FORMAT).as_str() {
            "comma" => NumberMarks {
                group: ".",
                decimal: ",",
            },
            "space" => NumberMarks {
                group: "\u{202F}",
                decimal: ",",
            },
            _ => NumberMarks {
                group: ",",
                decimal: ".",
            },
        };
        Self { dates, numbers }
    }

    fn day(self, date: Date) -> String {
        let (year, month, day) = (date.year(), date.month(), date.day());
        match self.dates {
            DateOrder::DayMonthYear => format!("{day:02}.{month:02}.{year:04}"),
            DateOrder::MonthDayYear => format!("{month:02}/{day:02}/{year:04}"),
            DateOrder::YearMonthDay => format!("{year:04}-{month:02}-{day:02}"),
        }
    }

    fn number(self, number: &serde_json::Number) -> String {
        let written = number.to_string();
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
            format!("{sign}{grouped}{}{decimals}", self.numbers.decimal)
        }
    }
}

/// Names in a sentence as the catalog lists them: `Ana, Ben, and Cleo`.
pub(crate) fn list_text(items: &[String]) -> String {
    match items {
        [] => String::new(),
        [one] => one.clone(),
        [first, second] => message::list_two(first, second).render(),
        [start @ .., last] => message::list_many(last, start.join(", ")).render(),
    }
}

/// The word of a status or a priority; any other text as it is.
pub(crate) fn word_of(field: Option<Field>, text: &str) -> String {
    let known = match field {
        Some(Field::Status) => STARTERS
            .iter()
            .flat_map(|starter| starter.statuses)
            .find(|status| status.key == text)
            .map(|status| status.name),
        Some(Field::Priority) => Priority::from_code(text).map(Priority::name),
        _ => None,
    };
    known.map_or_else(|| text.to_owned(), |key| Message::from(key).render())
}

/// A value of an own field that the screen words itself; `None` for a value that does not fit the
/// kind (the screen shows nothing for it either).
pub(crate) fn typed_text(kind: FieldType, value: &Value, region: Region) -> Option<String> {
    match (kind, value) {
        (FieldType::YesNo, Value::Bool(yes)) => Some(
            Message::from(if *yes {
                message::FIELDS_YES
            } else {
                message::FIELDS_NO
            })
            .render(),
        ),
        (FieldType::Duration, Value::Number(seconds)) => seconds.as_i64().map(duration_text),
        (FieldType::WholeNumber | FieldType::Decimal, Value::Number(number)) => {
            Some(region.number(number))
        }
        (FieldType::Date, Value::String(day)) => {
            day.parse::<Date>().ok().map(|day| region.day(day))
        }
        (FieldType::DateTime, Value::String(moment)) => {
            let moment = moment
                .parse::<Timestamp>()
                .ok()?
                .to_zoned(jiff::tz::TimeZone::UTC);
            Some(
                message::mail_date_time(
                    region.day(moment.date()),
                    format!("{:02}:{:02}", moment.hour(), moment.minute()),
                )
                .render(),
            )
        }
        (FieldType::WholeNumber | FieldType::Decimal, Value::String(text)) => Some(text.clone()),
        (
            FieldType::YesNo
            | FieldType::Duration
            | FieldType::WholeNumber
            | FieldType::Decimal
            | FieldType::Date
            | FieldType::DateTime,
            _,
        ) => None,
        (_, Value::String(text)) => Some(text.clone()),
        _ => None,
    }
}

/// Seconds as "2h 5m", "45m" or "0m": whole minutes.
fn duration_text(seconds: i64) -> String {
    let minutes_all = seconds / SECONDS_PER_MINUTE;
    let (hours, minutes) = (
        minutes_all / MINUTES_PER_HOUR,
        minutes_all % MINUTES_PER_HOUR,
    );
    let hour = Message::from(message::FIELDS_UNIT_HOUR).render();
    let minute = Message::from(message::FIELDS_UNIT_MINUTE).render();
    match (hours, minutes) {
        (0, minutes) => format!("{minutes}{minute}"),
        (hours, 0) => format!("{hours}{hour}"),
        (hours, minutes) => format!("{hours}{hour} {minutes}{minute}"),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    const fn region(dates: DateOrder, group: &'static str, decimal: &'static str) -> Region {
        Region {
            dates,
            numbers: NumberMarks { group, decimal },
        }
    }

    const USUAL: Region = region(DateOrder::YearMonthDay, ",", ".");

    fn text(kind: FieldType, value: &Value, region: Region) -> Option<String> {
        typed_text(kind, value, region)
    }

    #[test]
    fn yes_and_no_are_the_words_of_the_screen() {
        assert_eq!(
            text(FieldType::YesNo, &json!(true), USUAL).as_deref(),
            Some("Yes")
        );
        assert_eq!(
            text(FieldType::YesNo, &json!(false), USUAL).as_deref(),
            Some("No")
        );
    }

    #[test]
    fn a_length_of_time_is_hours_and_whole_minutes() {
        let seconds = |seconds: i64| text(FieldType::Duration, &json!(seconds), USUAL);
        assert_eq!(seconds(7_500).as_deref(), Some("2h 5m"));
        assert_eq!(seconds(2_700).as_deref(), Some("45m"));
        assert_eq!(seconds(7_200).as_deref(), Some("2h"));
        assert_eq!(seconds(30).as_deref(), Some("0m"));
    }

    #[test]
    fn numbers_follow_the_marks_the_person_chose() {
        let comma = region(DateOrder::YearMonthDay, ".", ",");
        assert_eq!(
            text(FieldType::Decimal, &json!(1234.5), USUAL).as_deref(),
            Some("1,234.5")
        );
        assert_eq!(
            text(FieldType::Decimal, &json!(1234.5), comma).as_deref(),
            Some("1.234,5")
        );
        assert_eq!(
            text(FieldType::WholeNumber, &json!(-1_234_567), USUAL).as_deref(),
            Some("-1,234,567")
        );
        assert_eq!(
            text(FieldType::WholeNumber, &json!(12), USUAL).as_deref(),
            Some("12")
        );
    }

    #[test]
    fn a_day_follows_the_order_the_person_chose() {
        let day = json!("2026-10-05");
        let order =
            |dates| text(FieldType::Date, &day, region(dates, ",", ".")).unwrap_or_default();
        assert_eq!(order(DateOrder::YearMonthDay), "2026-10-05");
        assert_eq!(order(DateOrder::DayMonthYear), "05.10.2026");
        assert_eq!(order(DateOrder::MonthDayYear), "10/05/2026");
    }

    #[test]
    fn a_moment_is_written_in_utc_and_says_so() {
        let moment = json!("2026-10-05T09:30:00Z");
        assert_eq!(
            text(FieldType::DateTime, &moment, USUAL).as_deref(),
            Some("2026-10-05, 09:30 UTC")
        );
    }

    #[test]
    fn a_value_that_does_not_fit_its_kind_has_no_words() {
        assert_eq!(text(FieldType::YesNo, &json!("maybe"), USUAL), None);
        assert_eq!(
            text(FieldType::Text, &json!("hello"), USUAL).as_deref(),
            Some("hello")
        );
        assert_eq!(text(FieldType::Duration, &json!(true), USUAL), None);
        assert_eq!(text(FieldType::Date, &json!("not a day"), USUAL), None);
    }

    #[test]
    fn names_are_listed_like_on_the_screen() {
        let names = |count: usize| {
            list_text(
                &["Ana", "Ben", "Cem", "Dee"][..count]
                    .iter()
                    .map(|name| (*name).to_owned())
                    .collect::<Vec<_>>(),
            )
        };
        assert_eq!(names(1), "Ana");
        assert_eq!(names(2), "Ana and Ben");
        assert_eq!(names(3), "Ana, Ben, and Cem");
        assert_eq!(names(4), "Ana, Ben, Cem, and Dee");
    }

    #[test]
    fn a_status_and_a_priority_are_words_and_other_text_stays() {
        assert_eq!(word_of(Some(Field::Status), "in-progress"), "In progress");
        assert_eq!(word_of(Some(Field::Priority), "high"), "High");
        assert_eq!(word_of(Some(Field::Priority), "urgent!"), "urgent!");
        assert_eq!(word_of(None, "open"), "open");
    }
}
