//! The words of the values in an e-mail: a status, a priority, yes or no, a length of time, a
//! number, a day, a moment and a list of names, worded the way the screen words them (the history
//! of an issue, `web/app/src/shared/own-value-text.ts` and `format.ts`), so a person reads one
//! thing in both places. The test vectors in `tools/testdata/value-words.json` hold the two
//! implementations together: a test here and a test in the browser app both read them.
//!
//! The screen writes days, moments and numbers the way the person chose under *Region*, and lets
//! the browser fill in what the person left on *device*. A server does not know the device, so an
//! e-mail takes what the organization chose for that setting, and the usual way (year, month, day;
//! a point before decimals; UTC) when the organization chose nothing either. A moment is followed
//! by the name of its time zone, because an e-mail is read away from the screen.

use bb_core::fields::Field;
use bb_core::issue::Priority;
use bb_core::message::{self, Message};
use bb_core::own_fields::FieldType;
use bb_core::settings::{
    DEVICE, Layers, REGION_DATE_FORMAT, REGION_NUMBER_FORMAT, REGION_TIME_ZONE, Setting, Text,
};
use bb_core::workflow::starter::STARTERS;
use jiff::Timestamp;
use jiff::civil::Date;
use jiff::tz::TimeZone;
use serde_json::Value;

const SECONDS_PER_MINUTE: f64 = 60.0;
const MINUTES_PER_HOUR: f64 = 60.0;
const HOURS_ON_A_CLOCK: i8 = 12;
/// The digits in a group of a big number.
const GROUP: usize = 3;
/// How many decimals a number is written with at the most (as the browser writes it).
const DECIMALS_MOST: usize = 3;
/// The zone of a person and an organization who chose none.
const UTC: &str = "UTC";

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
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Region {
    dates: DateOrder,
    numbers: NumberMarks,
    zone_name: String,
    zone: TimeZone,
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
    /// What the layers of a person say.
    pub(crate) fn of(layers: &Layers) -> Self {
        Self::from_choices(
            chosen(layers, &REGION_DATE_FORMAT).as_deref(),
            chosen(layers, &REGION_NUMBER_FORMAT).as_deref(),
            chosen(layers, &REGION_TIME_ZONE).as_deref(),
        )
    }

    /// The region for the values of the three settings; `None` is *device* with nothing to take
    /// its place. A zone that the time-zone data does not know counts as none.
    pub(crate) fn from_choices(
        date_format: Option<&str>,
        number_format: Option<&str>,
        time_zone: Option<&str>,
    ) -> Self {
        let dates = match date_format {
            Some("day-month-year") => DateOrder::DayMonthYear,
            Some("month-day-year") => DateOrder::MonthDayYear,
            _ => DateOrder::YearMonthDay,
        };
        let numbers = match number_format {
            Some("comma") => NumberMarks {
                group: ".",
                decimal: ",",
            },
            Some("space") => NumberMarks {
                group: "\u{202F}",
                decimal: ",",
            },
            _ => NumberMarks {
                group: ",",
                decimal: ".",
            },
        };
        let (zone_name, zone) = time_zone
            .and_then(|name| TimeZone::get(name).ok().map(|zone| (name.to_owned(), zone)))
            .unwrap_or_else(|| (UTC.to_owned(), TimeZone::UTC));
        Self {
            dates,
            numbers,
            zone_name,
            zone,
        }
    }

    fn day(&self, date: Date) -> String {
        let (year, month, day) = (date.year(), date.month(), date.day());
        match self.dates {
            DateOrder::DayMonthYear => format!("{day:02}.{month:02}.{year:04}"),
            DateOrder::MonthDayYear => format!("{month:02}/{day:02}/{year:04}"),
            DateOrder::YearMonthDay => format!("{year:04}-{month:02}-{day:02}"),
        }
    }

    /// A day and a time of day in the zone of the person, as the screen writes them: the day in
    /// the person's order, the time on a clock of twelve hours.
    fn moment(&self, moment: Timestamp) -> String {
        let zoned = moment.to_zoned(self.zone.clone());
        let hour = zoned.hour();
        let on_clock = match hour % HOURS_ON_A_CLOCK {
            0 => HOURS_ON_A_CLOCK,
            hour => hour,
        };
        let minute = format!("{:02}", zoned.minute());
        let time = if hour < HOURS_ON_A_CLOCK {
            message::mail_time_am(on_clock, minute)
        } else {
            message::mail_time_pm(on_clock, minute)
        };
        format!("{}, {}", self.day(zoned.date()), time.render())
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
            format!("{sign}{grouped}{}{decimals}", self.numbers.decimal)
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

/// A value of an own field that the screen words itself, as an e-mail words it: a moment is
/// followed by the name of its time zone. `None` for a value that does not fit the kind (the
/// screen shows nothing for it either).
pub(crate) fn typed_text(kind: FieldType, value: &Value, region: &Region) -> Option<String> {
    let words = typed_words(kind, value, region)?;
    Some(if kind == FieldType::DateTime {
        message::mail_date_time(words, &region.zone_name).render()
    } else {
        words
    })
}

/// The words of a value of an own field, as the screen words them for the same region.
fn typed_words(kind: FieldType, value: &Value, region: &Region) -> Option<String> {
    match (kind, value) {
        (FieldType::YesNo, Value::Bool(yes)) => Some(
            Message::from(if *yes {
                message::FIELDS_YES
            } else {
                message::FIELDS_NO
            })
            .render(),
        ),
        (FieldType::Duration, Value::Number(seconds)) => seconds
            .as_f64()
            .filter(|seconds| *seconds >= 0.0)
            .map(duration_text),
        (FieldType::WholeNumber | FieldType::Decimal, Value::Number(number)) => {
            Some(region.number(number))
        }
        (FieldType::Date, Value::String(day)) => {
            day.parse::<Date>().ok().map(|day| region.day(day))
        }
        (FieldType::DateTime, Value::String(moment)) => {
            moment.parse::<Timestamp>().ok().map(|at| region.moment(at))
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

/// Seconds as "2h 5m", "45m" or "0m": whole minutes; a part of a second is dropped like the rest of
/// a minute.
fn duration_text(seconds: f64) -> String {
    let minutes_all = (seconds / SECONDS_PER_MINUTE).floor();
    let hours = (minutes_all / MINUTES_PER_HOUR).floor();
    let minutes = minutes_all - hours * MINUTES_PER_HOUR;
    let hour = Message::from(message::FIELDS_UNIT_HOUR).render();
    let minute = Message::from(message::FIELDS_UNIT_MINUTE).render();
    if hours == 0.0 {
        format!("{minutes}{minute}")
    } else if minutes == 0.0 {
        format!("{hours}{hour}")
    } else {
        format!("{hours}{hour} {minutes}{minute}")
    }
}

#[cfg(test)]
mod tests {
    use bb_core::settings::{OrganizationValue, SettingValue};
    use serde::Deserialize;
    use serde_json::json;

    use super::*;

    /// The vectors that the browser app reads too (`web/app/src/shared/value-words.test.ts`).
    const VECTORS: &str = include_str!("../../../../tools/testdata/value-words.json");

    #[derive(Deserialize)]
    struct Choices {
        #[serde(rename = "dateFormat")]
        date_format: String,
        #[serde(rename = "numberFormat")]
        number_format: String,
        #[serde(rename = "timeZone")]
        time_zone: String,
    }

    #[derive(Deserialize)]
    struct Case {
        name: String,
        kind: FieldType,
        value: Value,
        region: String,
        expected: Option<String>,
    }

    #[derive(Deserialize)]
    struct Word {
        name: String,
        field: String,
        text: String,
        expected: String,
    }

    #[derive(Deserialize)]
    struct List {
        names: Vec<String>,
        expected: String,
    }

    #[derive(Deserialize)]
    struct Vectors {
        regions: std::collections::BTreeMap<String, Choices>,
        cases: Vec<Case>,
        words: Vec<Word>,
        lists: Vec<List>,
    }

    #[test]
    fn m4_f141_the_words_are_the_ones_the_browser_test_expects() {
        let vectors: Vectors = serde_json::from_str(VECTORS).expect("the vectors are JSON");
        assert!(vectors.cases.len() > 20, "the vectors are not empty");
        for case in vectors.cases {
            let choices = vectors.regions.get(&case.region).expect("a known region");
            let region = Region::from_choices(
                Some(&choices.date_format),
                Some(&choices.number_format),
                Some(&choices.time_zone),
            );
            let words = typed_words(case.kind, &case.value, &region);
            assert_eq!(words, case.expected, "case {}", case.name);
        }
        for word in vectors.words {
            let said = word_of(Field::from_code(&word.field), &word.text);
            assert_eq!(said, word.expected, "word {}", word.name);
        }
        for list in vectors.lists {
            assert_eq!(
                list_text(&list.names),
                list.expected,
                "list {:?}",
                list.names
            );
        }
    }

    fn layers(person: &[(&str, &str)], organization: &[(&str, &str)]) -> Layers {
        let mut layers = Layers::default();
        for (key, value) in person {
            layers
                .person
                .insert((*key).to_owned(), SettingValue::Text((*value).to_owned()));
        }
        for (key, value) in organization {
            layers.organization.insert(
                (*key).to_owned(),
                OrganizationValue {
                    value: SettingValue::Text((*value).to_owned()),
                    locked: false,
                },
            );
        }
        layers
    }

    const MOMENT: &str = "2026-10-05T07:30:00Z";

    fn moment_in(layers: &Layers) -> Option<String> {
        typed_text(FieldType::DateTime, &json!(MOMENT), &Region::of(layers))
    }

    #[test]
    fn m4_f140_a_moment_is_in_the_zone_of_the_person_and_names_it() {
        let own = layers(
            &[
                ("region.time-zone", "Europe/Berlin"),
                ("region.date-format", "day-month-year"),
            ],
            &[],
        );
        assert_eq!(
            moment_in(&own).as_deref(),
            Some("05.10.2026, 9:30 AM (Europe/Berlin)")
        );
    }

    #[test]
    fn m4_f140_a_person_on_device_gets_the_zone_of_the_organization() {
        let on_device = layers(
            &[("region.time-zone", "device")],
            &[("region.time-zone", "Europe/Berlin")],
        );
        assert_eq!(
            moment_in(&on_device).as_deref(),
            Some("2026-10-05, 9:30 AM (Europe/Berlin)")
        );
        // Nothing chosen by the person gives the same.
        let nothing = layers(&[], &[("region.time-zone", "Europe/Berlin")]);
        assert_eq!(moment_in(&nothing), moment_in(&on_device));
    }

    #[test]
    fn m4_f140_where_nobody_chose_a_zone_it_is_utc() {
        assert_eq!(
            moment_in(&layers(&[], &[])).as_deref(),
            Some("2026-10-05, 7:30 AM (UTC)")
        );
        // A name the time-zone data does not know counts as none.
        assert_eq!(
            moment_in(&layers(&[("region.time-zone", "Mars/Olympus")], &[])).as_deref(),
            Some("2026-10-05, 7:30 AM (UTC)")
        );
    }

    #[test]
    fn m4_f140_the_zone_data_follows_summer_time() {
        let berlin = Region::from_choices(None, None, Some("Europe/Berlin"));
        let winter = typed_words(FieldType::DateTime, &json!("2026-12-05T07:30:00Z"), &berlin);
        assert_eq!(winter.as_deref(), Some("2026-12-05, 8:30 AM"));
    }
}
