//! How a day, a moment and a number are written for a reader of an e-mail: the part of the wording
//! that depends on what a person chose under *Region*. The screen writes the same with `Intl`
//! (`web/app/src/shared/format.ts`), because its choice *device* uses what the browser does, which
//! the server cannot run; the two are held together by the vectors in
//! `tools/testdata/value-words.json`, which `cargo xtask gen word-vectors` writes from this code
//! (ADR-0033, the amendment about value wording).
//!
//! The screen writes days, moments and numbers the way the person chose under *Region*, and lets
//! the browser fill in what the person left on *device*. A server does not know the device, so an
//! e-mail takes what the organization chose for that setting, and the usual way (year, month, day;
//! a point before decimals; UTC) when the organization chose nothing either. A moment is followed
//! by the name of its time zone, because an e-mail is read away from the screen.

use jiff::Timestamp;
use jiff::civil::Date;
use jiff::tz::TimeZone;
use serde_json::Value;

use crate::message;
use crate::own_fields::FieldType;
use crate::settings::{
    DATE_FORMAT_DAY_MONTH_YEAR, DATE_FORMAT_MONTH_DAY_YEAR, DEVICE, Layers, NUMBER_FORMAT_COMMA,
    NUMBER_FORMAT_SPACE, REGION_DATE_FORMAT, REGION_NUMBER_FORMAT, REGION_TIME_ZONE, Setting, Text,
};

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
pub struct Region {
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
    /// The time zone of the region: where a day begins for the person.
    #[must_use]
    pub const fn zone(&self) -> &TimeZone {
        &self.zone
    }

    /// What the layers of a person say.
    #[must_use]
    pub fn of(layers: &Layers) -> Self {
        Self::from_choices(
            chosen(layers, &REGION_DATE_FORMAT).as_deref(),
            chosen(layers, &REGION_NUMBER_FORMAT).as_deref(),
            chosen(layers, &REGION_TIME_ZONE).as_deref(),
        )
    }

    /// The region for the values of the three settings; `None` is *device* with nothing to take
    /// its place. A zone that the time-zone data does not know counts as none.
    #[must_use]
    pub fn from_choices(
        date_format: Option<&str>,
        number_format: Option<&str>,
        time_zone: Option<&str>,
    ) -> Self {
        let dates = match date_format {
            Some(DATE_FORMAT_DAY_MONTH_YEAR) => DateOrder::DayMonthYear,
            Some(DATE_FORMAT_MONTH_DAY_YEAR) => DateOrder::MonthDayYear,
            _ => DateOrder::YearMonthDay,
        };
        let numbers = match number_format {
            Some(NUMBER_FORMAT_COMMA) => NumberMarks {
                group: ".",
                decimal: ",",
            },
            Some(NUMBER_FORMAT_SPACE) => NumberMarks {
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
                day.parse::<Date>().ok().map(|day| self.day(day))
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

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::settings::{OrganizationValue, SettingValue};

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
        Region::of(layers).worded_for_mail(FieldType::DateTime, &json!(MOMENT))
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
        let winter = berlin.worded(FieldType::DateTime, &json!("2026-12-05T07:30:00Z"));
        assert_eq!(winter.as_deref(), Some("2026-12-05, 8:30 AM"));
    }

    #[test]
    fn a_number_is_rounded_half_up_on_its_digits() {
        let usual = Region::from_choices(None, None, None);
        let said = |number: f64| usual.worded(FieldType::Decimal, &json!(number));
        assert_eq!(said(0.0625).as_deref(), Some("0.063"));
        assert_eq!(said(0.9996).as_deref(), Some("1"));
        assert_eq!(said(-0.0625).as_deref(), Some("-0.063"));
    }

    #[test]
    fn the_hours_of_a_moment_are_on_a_clock_of_twelve() {
        let usual = Region::from_choices(None, None, None);
        let at = |moment: &str| usual.worded(FieldType::DateTime, &json!(moment));
        assert_eq!(
            at("2026-10-05T00:00:00Z").as_deref(),
            Some("2026-10-05, 12:00 AM")
        );
        assert_eq!(
            at("2026-10-05T12:05:00Z").as_deref(),
            Some("2026-10-05, 12:05 PM")
        );
        assert_eq!(
            at("2026-10-05T13:05:00Z").as_deref(),
            Some("2026-10-05, 1:05 PM")
        );
    }

    #[test]
    fn the_two_fixed_orders_of_a_day_are_written_with_their_marks() {
        let day = |date: &str| {
            Region::from_choices(Some(date), None, None)
                .worded(FieldType::Date, &json!("2026-10-05"))
        };
        assert_eq!(
            day(DATE_FORMAT_DAY_MONTH_YEAR).as_deref(),
            Some("05.10.2026")
        );
        assert_eq!(
            day(DATE_FORMAT_MONTH_DAY_YEAR).as_deref(),
            Some("10/05/2026")
        );
    }
}
