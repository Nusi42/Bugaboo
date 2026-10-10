//! The values of own fields (CF-01, CF-03, ADR-0006): reading a value from JSON for its type,
//! writing it back, the cells it is stored in (one typed column per kind, several rows for several
//! values), and the check of the rules of a field.
//!
//! One function checks a value against the rules ([`Rules::check`]), for the server and, through
//! `bb-wasm`, for the browser, so the instant feedback and the decision never differ. A pattern
//! is checked by an engine the caller passes: the server passes a linear-time one, the browser
//! passes none and leaves the pattern to the server.

use jiff::Timestamp;
use jiff::civil::Date;
use jiff::tz::TimeZone;
use serde_json::{Number, Value};

use crate::ErrorCode;
use crate::id::{ChoiceId, GroupId, IssueId, UserId};
use crate::own_fields::{DECIMAL_PARTS, FieldType, Rules, SHORT_TEXT_MAX, VALUES_MAX};
use crate::personal::Entity;
use crate::text::{LONG_TEXT_MAX, clean, clean_long};

const SECONDS_PER_DAY: i64 = 86_400;

words! {
    /// Why a value is refused (CF-03). Each is an error code with a sentence; the numbers the
    /// sentence may need are in the rules the person sees.
    pub enum Violation {
        /// The field has to be filled in.
        Required => "required",
        /// Shorter, or fewer values, than the rules allow.
        TooShort => "too-short",
        /// Longer, or more values, than the rules allow.
        TooLong => "too-long",
        /// Below the lowest value.
        TooSmall => "too-small",
        /// Above the highest value.
        TooBig => "too-big",
        /// The text does not match the pattern.
        NoMatch => "no-match",
        /// Not a value this kind of field can hold.
        WrongValue => "wrong-value",
        /// A choice the field does not offer (any more).
        UnknownChoice => "unknown-choice",
    }
}

impl Violation {
    /// The error code the API answers with.
    #[must_use]
    pub const fn error(self) -> ErrorCode {
        match self {
            Self::Required => ErrorCode::FieldRequired,
            Self::TooShort => ErrorCode::FieldTooShort,
            Self::TooLong => ErrorCode::FieldTooLong,
            Self::TooSmall => ErrorCode::FieldTooSmall,
            Self::TooBig => ErrorCode::FieldTooBig,
            Self::NoMatch => ErrorCode::FieldNoMatch,
            Self::WrongValue => ErrorCode::FieldWrongValue,
            Self::UnknownChoice => ErrorCode::FieldUnknownChoice,
        }
    }
}

/// A value, in the shape of its column. What the shape means depends on the [`FieldType`] of the
/// field: a number may be a whole number, a decimal in millionths, or seconds.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FieldValue {
    /// A text, a long text or a web address.
    Text(String),
    /// A whole number, a decimal (millionths) or a duration (seconds).
    Number(i64),
    /// Yes or no.
    Flag(bool),
    /// A day, as days since 1970-01-01.
    Day(i64),
    /// A moment, as microseconds since 1970-01-01T00:00:00Z.
    Moment(i64),
    /// The IDs of choices, people, groups or issues, as stored: one, or several.
    Ids(Vec<[u8; 16]>),
}

/// One stored row of a value: exactly one of the four is set (ADR-0006).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cell {
    /// `value_text`.
    pub text: Option<String>,
    /// `value_number`.
    pub number: Option<i64>,
    /// `value_date`.
    pub date: Option<i64>,
    /// `value_ref`.
    pub reference: Option<[u8; 16]>,
}

impl Cell {
    const fn empty() -> Self {
        Self {
            text: None,
            number: None,
            date: None,
            reference: None,
        }
    }
}

/// A value of an own field for one item (an issue or a project), as stored: one row per value.
/// The text of a value is always marked personal data (a field cannot say otherwise yet, M3-F45); a reference may be a
/// person.
#[derive(Clone, Debug, PartialEq, Eq, Entity)]
pub struct StoredFieldValue {
    /// The kind of item (`issue`, `project`).
    #[not_personal]
    pub entity_kind: crate::event::EntityKind,
    /// The item.
    #[not_personal]
    pub entity_id: crate::id::EntityId,
    /// The field.
    #[not_personal]
    pub field: crate::id::FieldId,
    /// A text. Free text that could name a person.
    #[personal(category = "content", basis = "legitimate_interests")]
    pub value_text: Option<String>,
    /// A number, a yes or no, or a length of time.
    #[not_personal]
    pub value_number: Option<i64>,
    /// A day or a moment.
    #[not_personal]
    pub value_date: Option<i64>,
    /// A choice, a person, a group or an issue.
    #[personal(category = "activity", basis = "legitimate_interests")]
    pub value_ref: Option<crate::id::EntityId>,
    /// The place among several values.
    #[not_personal]
    pub position: i64,
}

/// The values of the own fields of one item, by the key of the field (its ID), each as the JSON
/// of its type ([`FieldValue::to_json`]). A view holds it as a member marked
/// `#[masked(own_fields)]`, so a value of a field the person may not see never leaves.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize)]
#[serde(transparent)]
pub struct OwnValues(pub std::collections::BTreeMap<String, Value>);

impl OwnValues {
    /// Whether no value is set.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

#[cfg(feature = "openapi")]
impl utoipa::PartialSchema for OwnValues {
    fn schema() -> utoipa::openapi::RefOr<utoipa::openapi::schema::Schema> {
        use utoipa::openapi::schema::{AdditionalProperties, ObjectBuilder, Type};
        ObjectBuilder::new()
            .schema_type(Type::Object)
            .description(Some(
                "The values of the own fields, by the key (ID) of the field. A value has the form of its field's type.",
            ))
            .additional_properties(Some(AdditionalProperties::FreeForm(true)))
            .into()
    }
}

#[cfg(feature = "openapi")]
impl utoipa::ToSchema for OwnValues {}

/// Decides whether a text matches a pattern. The server passes an engine that cannot run
/// away; the browser has none.
pub trait PatternEngine {
    /// Whether the whole `text` matches `pattern`.
    fn matches(&self, pattern: &str, text: &str) -> bool;
}

/// No engine: every text matches. For the browser, which leaves the pattern to the server.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoPattern;

impl PatternEngine for NoPattern {
    fn matches(&self, _pattern: &str, _text: &str) -> bool {
        true
    }
}

impl Rules {
    /// Checks a value against the rules. `None` is "no value": refused only when the field is
    /// required.
    ///
    /// # Errors
    /// Says which rule the value breaks.
    pub fn check(
        &self,
        kind: FieldType,
        value: Option<&FieldValue>,
        engine: &impl PatternEngine,
    ) -> Result<(), Violation> {
        let Some(value) = value else {
            return if self.required {
                Err(Violation::Required)
            } else {
                Ok(())
            };
        };
        let length = match value {
            FieldValue::Text(text) => Some(text.chars().count()),
            FieldValue::Ids(ids) if kind.is_many() => Some(ids.len()),
            _ => None,
        };
        if let Some(length) = length {
            if self.min_length.is_some_and(|least| length < least as usize) {
                return Err(Violation::TooShort);
            }
            if self.max_length.is_some_and(|most| length > most as usize) {
                return Err(Violation::TooLong);
            }
        }
        if let (FieldValue::Text(text), Some(pattern)) = (value, &self.pattern)
            && !engine.matches(pattern, text)
        {
            return Err(Violation::NoMatch);
        }
        if let FieldValue::Number(at) | FieldValue::Day(at) | FieldValue::Moment(at) = value {
            if self.min.is_some_and(|least| *at < least) {
                return Err(Violation::TooSmall);
            }
            if self.max.is_some_and(|most| *at > most) {
                return Err(Violation::TooBig);
            }
        }
        Ok(())
    }
}

fn day_of(date: Date) -> Option<i64> {
    let moment = date.to_zoned(TimeZone::UTC).ok()?;
    Some(moment.timestamp().as_second().div_euclid(SECONDS_PER_DAY))
}

fn date_of(days: i64) -> Option<Date> {
    let moment = Timestamp::from_second(days.checked_mul(SECONDS_PER_DAY)?).ok()?;
    Some(moment.to_zoned(TimeZone::UTC).date())
}

/// A decimal in millionths, from `12.5`, `-0.25` or `7`: at most six digits after the point.
fn decimal_from(text: &str) -> Option<i64> {
    let (negative, digits) = match text.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, text.strip_prefix('+').unwrap_or(text)),
    };
    let (whole, parts) = digits.split_once('.').unwrap_or((digits, ""));
    let all_digits = |text: &str| text.bytes().all(|byte| byte.is_ascii_digit());
    if whole.is_empty() || !all_digits(whole) || !all_digits(parts) || parts.len() > 6 {
        return None;
    }
    let whole: i64 = whole.parse().ok()?;
    let mut fraction: i64 = if parts.is_empty() {
        0
    } else {
        parts.parse().ok()?
    };
    for _ in parts.len()..6 {
        fraction = fraction.checked_mul(10)?;
    }
    let amount = whole.checked_mul(DECIMAL_PARTS)?.checked_add(fraction)?;
    Some(if negative { -amount } else { amount })
}

/// A decimal in millionths as text, without trailing zeros: `12.5`.
fn decimal_text(millionths: i64) -> String {
    let sign = if millionths < 0 { "-" } else { "" };
    let amount = millionths.unsigned_abs();
    let (whole, fraction) = (
        amount / DECIMAL_PARTS.unsigned_abs(),
        amount % DECIMAL_PARTS.unsigned_abs(),
    );
    if fraction == 0 {
        format!("{sign}{whole}")
    } else {
        let fraction = format!("{fraction:06}");
        format!("{sign}{whole}.{}", fraction.trim_end_matches('0'))
    }
}

/// Whether the text is a web address: `http` or `https`, a host, no spaces or control characters.
fn is_web_address(text: &str) -> bool {
    let rest = text
        .strip_prefix("https://")
        .or_else(|| text.strip_prefix("http://"));
    rest.is_some_and(|rest| {
        let host = rest.split(['/', '?', '#']).next().unwrap_or_default();
        !host.is_empty() && !text.chars().any(|c| c.is_whitespace() || c.is_control())
    })
}

fn id_bytes(kind: FieldType, text: &str) -> Option<[u8; 16]> {
    match kind {
        FieldType::PickOne | FieldType::PickSeveral => {
            text.parse::<ChoiceId>().ok().map(ChoiceId::to_bytes)
        }
        FieldType::Person | FieldType::People => text.parse::<UserId>().ok().map(UserId::to_bytes),
        FieldType::Group => text.parse::<GroupId>().ok().map(GroupId::to_bytes),
        FieldType::Issue => text.parse::<IssueId>().ok().map(IssueId::to_bytes),
        _ => None,
    }
}

fn id_text(kind: FieldType, bytes: [u8; 16]) -> Option<String> {
    match kind {
        FieldType::PickOne | FieldType::PickSeveral => {
            ChoiceId::from_bytes(bytes).ok().map(|id| id.to_string())
        }
        FieldType::Person | FieldType::People => {
            UserId::from_bytes(bytes).ok().map(|id| id.to_string())
        }
        FieldType::Group => GroupId::from_bytes(bytes).ok().map(|id| id.to_string()),
        FieldType::Issue => IssueId::from_bytes(bytes).ok().map(|id| id.to_string()),
        _ => None,
    }
}

impl FieldValue {
    /// Reads a value of a field of this type from JSON: `null`, an empty text and an empty list
    /// are no value (`Ok(None)`).
    ///
    /// # Errors
    /// [`Violation::WrongValue`] when the JSON is not a value of the type, [`Violation::TooLong`]
    /// for a text above its largest size or too many values.
    pub fn from_json(kind: FieldType, json: &Value) -> Result<Option<Self>, Violation> {
        if json.is_null() {
            return Ok(None);
        }
        let wrong = Violation::WrongValue;
        let value = match kind {
            FieldType::Text => match json.as_str().ok_or(wrong)?.trim() {
                "" => return Ok(None),
                text => Self::Text(clean(text, SHORT_TEXT_MAX).ok_or(wrong)?.to_owned()),
            },
            FieldType::LongText => {
                let text = json.as_str().ok_or(wrong)?;
                if text.is_empty() {
                    return Ok(None);
                }
                Self::Text(clean_long(text, LONG_TEXT_MAX).ok_or(wrong)?.to_owned())
            }
            FieldType::WebAddress => match json.as_str().ok_or(wrong)?.trim() {
                "" => return Ok(None),
                text if text.chars().count() <= SHORT_TEXT_MAX && is_web_address(text) => {
                    Self::Text(text.to_owned())
                }
                _ => return Err(wrong),
            },
            FieldType::WholeNumber | FieldType::Duration => {
                let number = json.as_i64().ok_or(wrong)?;
                if kind == FieldType::Duration && number < 0 {
                    return Err(wrong);
                }
                Self::Number(number)
            }
            FieldType::Decimal => match json {
                Value::String(text) => Self::Number(decimal_from(text.trim()).ok_or(wrong)?),
                Value::Number(number) => {
                    Self::Number(decimal_from(&number.to_string()).ok_or(wrong)?)
                }
                _ => return Err(wrong),
            },
            FieldType::YesNo => Self::Flag(json.as_bool().ok_or(wrong)?),
            FieldType::Date => {
                let date: Date = json.as_str().ok_or(wrong)?.parse().map_err(|_| wrong)?;
                Self::Day(day_of(date).ok_or(wrong)?)
            }
            FieldType::DateTime => {
                let moment: Timestamp = json.as_str().ok_or(wrong)?.parse().map_err(|_| wrong)?;
                Self::Moment(moment.as_microsecond())
            }
            FieldType::PickOne | FieldType::Person | FieldType::Group | FieldType::Issue => {
                Self::Ids(vec![
                    id_bytes(kind, json.as_str().ok_or(wrong)?).ok_or(wrong)?,
                ])
            }
            FieldType::PickSeveral | FieldType::People => {
                let items = json.as_array().ok_or(wrong)?;
                // Too long a list is refused before it is read, so no list costs more than the cap.
                if items.len() > VALUES_MAX {
                    return Err(Violation::TooLong);
                }
                let mut ids: Vec<[u8; 16]> = Vec::new();
                for item in items {
                    let id = id_bytes(kind, item.as_str().ok_or(wrong)?).ok_or(wrong)?;
                    if !ids.contains(&id) {
                        ids.push(id);
                    }
                }
                if ids.is_empty() {
                    return Ok(None);
                }
                if ids.len() > VALUES_MAX {
                    return Err(Violation::TooLong);
                }
                Self::Ids(ids)
            }
        };
        Ok(Some(value))
    }

    /// The value as JSON, in the form [`FieldValue::from_json`] reads. `None` when the shape does
    /// not fit the type (a corrupt row).
    #[must_use]
    pub fn to_json(&self, kind: FieldType) -> Option<Value> {
        Some(match (kind, self) {
            (FieldType::Text | FieldType::LongText | FieldType::WebAddress, Self::Text(text)) => {
                Value::String(text.clone())
            }
            (FieldType::WholeNumber | FieldType::Duration, Self::Number(number)) => {
                Value::Number(Number::from(*number))
            }
            (FieldType::Decimal, Self::Number(millionths)) => {
                Value::String(decimal_text(*millionths))
            }
            (FieldType::YesNo, Self::Flag(flag)) => Value::Bool(*flag),
            (FieldType::Date, Self::Day(days)) => Value::String(date_of(*days)?.to_string()),
            (FieldType::DateTime, Self::Moment(micros)) => {
                Value::String(Timestamp::from_microsecond(*micros).ok()?.to_string())
            }
            (
                FieldType::PickOne | FieldType::Person | FieldType::Group | FieldType::Issue,
                Self::Ids(ids),
            ) => Value::String(id_text(kind, *ids.first()?)?),
            (FieldType::PickSeveral | FieldType::People, Self::Ids(ids)) => Value::Array(
                ids.iter()
                    .map(|id| id_text(kind, *id).map(Value::String))
                    .collect::<Option<Vec<_>>>()?,
            ),
            _ => return None,
        })
    }

    /// The rows the value is stored as: one cell for each value.
    #[must_use]
    pub fn to_cells(&self) -> Vec<Cell> {
        match self {
            Self::Text(text) => vec![Cell {
                text: Some(text.clone()),
                ..Cell::empty()
            }],
            Self::Number(number) => vec![Cell {
                number: Some(*number),
                ..Cell::empty()
            }],
            Self::Flag(flag) => vec![Cell {
                number: Some(i64::from(*flag)),
                ..Cell::empty()
            }],
            Self::Day(at) | Self::Moment(at) => vec![Cell {
                date: Some(*at),
                ..Cell::empty()
            }],
            Self::Ids(ids) => ids
                .iter()
                .map(|id| Cell {
                    reference: Some(*id),
                    ..Cell::empty()
                })
                .collect(),
        }
    }

    /// The value stored in these rows (in the order of their positions); `None` for no rows or
    /// rows that do not fit the type.
    #[must_use]
    pub fn from_cells(kind: FieldType, cells: &[Cell]) -> Option<Self> {
        let first = cells.first()?;
        Some(match kind {
            FieldType::Text | FieldType::LongText | FieldType::WebAddress => {
                Self::Text(first.text.clone()?)
            }
            FieldType::WholeNumber | FieldType::Decimal | FieldType::Duration => {
                Self::Number(first.number?)
            }
            FieldType::YesNo => Self::Flag(first.number? != 0),
            FieldType::Date => Self::Day(first.date?),
            FieldType::DateTime => Self::Moment(first.date?),
            FieldType::PickOne
            | FieldType::PickSeveral
            | FieldType::Person
            | FieldType::People
            | FieldType::Group
            | FieldType::Issue => Self::Ids(
                cells
                    .iter()
                    .map(|cell| cell.reference)
                    .collect::<Option<Vec<_>>>()?,
            ),
        })
    }

    /// The IDs a value holds, for the types that hold IDs.
    #[must_use]
    pub fn ids(&self) -> &[[u8; 16]] {
        match self {
            Self::Ids(ids) => ids,
            _ => &[],
        }
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;
    use serde_json::json;

    use super::*;
    use crate::id::SequentialIds;

    struct Digits;

    impl PatternEngine for Digits {
        fn matches(&self, _pattern: &str, text: &str) -> bool {
            text.chars().all(|c| c.is_ascii_digit())
        }
    }

    fn id_json(kind: FieldType, seed: u8) -> Value {
        let mut source = SequentialIds::default();
        let mut text = String::new();
        for _ in 0..=seed {
            text = match kind {
                FieldType::PickOne | FieldType::PickSeveral => {
                    ChoiceId::generate(&mut source).unwrap().to_string()
                }
                FieldType::Person | FieldType::People => UserId::generate(&mut source).to_string(),
                FieldType::Group => GroupId::generate(&mut source).unwrap().to_string(),
                _ => IssueId::generate(&mut source).unwrap().to_string(),
            };
        }
        Value::String(text)
    }

    /// A JSON value of every type, made from a few numbers.
    fn sample(kind: FieldType, number: i64, text: &str, seed: u8) -> Value {
        match kind {
            FieldType::Text | FieldType::LongText => json!(format!("t{text}")),
            FieldType::WebAddress => json!(format!("https://example.test/{text}")),
            FieldType::WholeNumber => json!(number),
            FieldType::Duration => json!(number.abs()),
            FieldType::Decimal => json!(decimal_text(number % 1_000_000_000_000)),
            FieldType::YesNo => json!(number % 2 == 0),
            FieldType::Date => json!(date_of(number % 100_000).unwrap().to_string()),
            FieldType::DateTime => {
                json!(
                    Timestamp::from_microsecond((number % 4_000_000_000_000_000).abs())
                        .unwrap()
                        .to_string()
                )
            }
            FieldType::PickOne | FieldType::Person | FieldType::Group | FieldType::Issue => {
                id_json(kind, seed)
            }
            FieldType::PickSeveral | FieldType::People => {
                json!([id_json(kind, seed), id_json(kind, seed + 1)])
            }
        }
    }

    proptest! {
        #[test]
        fn cf_01_every_field_type_reads_writes_and_stores_its_value(
            number in any::<i64>(),
            text in "[a-z0-9]{1,20}",
            seed in 0u8..20,
        ) {
            for kind in FieldType::ALL {
                let json = sample(*kind, number, &text, seed);
                let value = FieldValue::from_json(*kind, &json).unwrap().unwrap();
                // JSON back is the JSON that was read (the form is canonical).
                prop_assert_eq!(value.to_json(*kind), Some(json));
                // The cells give the value back.
                let cells = value.to_cells();
                prop_assert_eq!(cells.len(), value.ids().len().max(1));
                prop_assert_eq!(FieldValue::from_cells(*kind, &cells), Some(value));
            }
        }
    }

    #[test]
    fn cf_01_no_value_is_null_an_empty_text_or_an_empty_list() {
        assert_eq!(
            FieldValue::from_json(FieldType::Text, &Value::Null),
            Ok(None)
        );
        assert_eq!(
            FieldValue::from_json(FieldType::Text, &json!("  ")),
            Ok(None)
        );
        assert_eq!(
            FieldValue::from_json(FieldType::PickSeveral, &json!([])),
            Ok(None)
        );
    }

    #[test]
    fn cf_03_a_value_of_the_wrong_shape_is_refused() {
        let wrong = Err(Violation::WrongValue);
        assert_eq!(
            FieldValue::from_json(FieldType::WholeNumber, &json!("3")),
            wrong
        );
        assert_eq!(
            FieldValue::from_json(FieldType::WholeNumber, &json!(1.5)),
            wrong
        );
        assert_eq!(
            FieldValue::from_json(FieldType::Duration, &json!(-3)),
            wrong
        );
        assert_eq!(
            FieldValue::from_json(FieldType::YesNo, &json!("yes")),
            wrong
        );
        assert_eq!(
            FieldValue::from_json(FieldType::Date, &json!("10/10/2026")),
            wrong
        );
        assert_eq!(
            FieldValue::from_json(FieldType::WebAddress, &json!("ftp://x")),
            wrong
        );
        assert_eq!(
            FieldValue::from_json(FieldType::WebAddress, &json!("https://a b")),
            wrong
        );
        assert_eq!(
            FieldValue::from_json(FieldType::Decimal, &json!("1.1234567")),
            wrong
        );
        assert_eq!(
            FieldValue::from_json(FieldType::PickOne, &json!("nope")),
            wrong
        );
        assert_eq!(
            FieldValue::from_json(FieldType::Text, &json!("two\nlines")),
            wrong
        );
        let other_kind = id_json(FieldType::Person, 1);
        assert_eq!(
            FieldValue::from_json(FieldType::PickOne, &other_kind),
            wrong
        );
    }

    #[test]
    fn cf_01_decimals_are_exact_and_dates_are_days() {
        assert_eq!(
            FieldValue::from_json(FieldType::Decimal, &json!("12.5")),
            Ok(Some(FieldValue::Number(12_500_000)))
        );
        assert_eq!(decimal_text(-250_000), "-0.25");
        assert_eq!(
            FieldValue::from_json(FieldType::Date, &json!("1970-01-02")),
            Ok(Some(FieldValue::Day(1)))
        );
        assert_eq!(
            FieldValue::from_json(FieldType::Date, &json!("1969-12-31")),
            Ok(Some(FieldValue::Day(-1)))
        );
    }

    #[test]
    fn cf_03_a_required_field_and_a_rule_refuse_a_wrong_value_with_a_reason() {
        let rules = Rules {
            required: true,
            min: Some(2),
            max: Some(5),
            min_length: Some(2),
            max_length: Some(4),
            pattern: Some("[0-9]+".into()),
        };
        let check = |kind, value: Option<FieldValue>| rules.check(kind, value.as_ref(), &Digits);
        assert_eq!(check(FieldType::Text, None), Err(Violation::Required));
        let text = |text: &str| Some(FieldValue::Text(text.into()));
        assert_eq!(check(FieldType::Text, text("1")), Err(Violation::TooShort));
        assert_eq!(
            check(FieldType::Text, text("12345")),
            Err(Violation::TooLong)
        );
        assert_eq!(check(FieldType::Text, text("ab")), Err(Violation::NoMatch));
        assert_eq!(check(FieldType::Text, text("123")), Ok(()));
        let number = |at| Some(FieldValue::Number(at));
        assert_eq!(
            check(FieldType::WholeNumber, number(1)),
            Err(Violation::TooSmall)
        );
        assert_eq!(
            check(FieldType::WholeNumber, number(6)),
            Err(Violation::TooBig)
        );
        assert_eq!(check(FieldType::WholeNumber, number(3)), Ok(()));
        // The browser has no engine: the pattern is left to the server.
        assert_eq!(
            rules.check(FieldType::Text, text("ab").as_ref(), &NoPattern),
            Ok(())
        );
        // The count of values of a *Pick several* field.
        let many = Rules {
            max_length: Some(1),
            ..Rules::default()
        };
        let ids = FieldValue::Ids(vec![[1; 16], [2; 16]]);
        assert_eq!(
            many.check(FieldType::PickSeveral, Some(&ids), &NoPattern),
            Err(Violation::TooLong)
        );
        assert_eq!(
            Rules::default().check(FieldType::Text, None, &NoPattern),
            Ok(())
        );
    }

    #[test]
    fn every_violation_is_an_error_code_of_its_own() {
        let mut codes: Vec<ErrorCode> = Violation::ALL.iter().map(|each| each.error()).collect();
        codes.sort();
        codes.dedup();
        assert_eq!(codes.len(), Violation::ALL.len());
    }
}
