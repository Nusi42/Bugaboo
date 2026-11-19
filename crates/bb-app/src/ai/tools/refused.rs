//! Which member of the input of a tool was refused, read from the schema the tool shows (AI-21):
//! the refusal names the member and, where the schema lists them, the values it takes. It never
//! repeats the value that was given, which may be text a person wrote, and names a member the
//! schema does not know only when its name is a plain name.

use bb_core::ErrorCode;
use bb_core::message::{self, Message};
use bb_core::words::Word;
use serde_json::Value;

use crate::auth::AppError;

/// The refusal of an input that does not fit the schema of its tool: the first member that is
/// unknown, missing or of the wrong kind, else the plain `request.invalid`.
pub(super) fn refusal(schema: &Value, input: &Value) -> AppError {
    let Some(member) = refused_member(schema, input) else {
        return AppError::Refused(ErrorCode::InvalidRequest);
    };
    let choices = choices(schema, &member);
    AppError::RefusedInput {
        field: Some(member).filter(|name| plain_name(name)),
        choices,
    }
}

/// The longest name of a member that a refusal repeats.
const PLAIN_NAME_MOST: usize = 64;

/// Whether a name of a member reads as a name of a schema (`^[a-z_]{1,64}$`) and not as text a
/// person may have written: only such a name of an unknown member is repeated.
fn plain_name(name: &str) -> bool {
    (1..=PLAIN_NAME_MOST).contains(&name.len())
        && name
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte == b'_')
}

/// The refusal of a member whose text does not read as what it names (an ID, a date): the member,
/// with no values to choose from.
pub(super) fn refused_at(field: &str) -> AppError {
    AppError::RefusedInput {
        field: Some(field.to_owned()),
        choices: Vec::new(),
    }
}

/// A word of the program read from a member of the input; a text that is none of them is refused
/// at the member, with every word it takes.
pub(super) fn word_at<W: Word>(field: &str, text: &str) -> Result<W, AppError> {
    W::from_code(text).ok_or_else(|| AppError::RefusedInput {
        field: Some(field.to_owned()),
        choices: W::ALL.iter().map(|word| word.code().to_owned()).collect(),
    })
}

/// The member the input is refused at, by the schema: an unknown member, then a required one that
/// is missing, then one whose value is not of a kind or a value the schema allows.
fn refused_member(schema: &Value, input: &Value) -> Option<String> {
    let given = input.as_object()?;
    let properties = schema.get("properties").and_then(Value::as_object);
    let known = |name: &str| properties.is_some_and(|all| all.contains_key(name));
    if let Some(unknown) = given.keys().find(|name| !known(name)) {
        return Some(unknown.clone());
    }
    let required = schema.get("required").and_then(Value::as_array);
    let missing = required
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .find(|name| !given.contains_key(*name));
    if let Some(missing) = missing {
        return Some(missing.to_owned());
    }
    let properties = properties?;
    given
        .iter()
        .find(|(name, value)| {
            properties
                .get(name.as_str())
                .is_some_and(|member| !fits(member, value))
        })
        .map(|(name, _)| name.clone())
}

/// Whether a value fits the schema of its member: its kind (`type`, one or a list) and, when the
/// schema lists them, one of the values (`enum`). A member without a kind takes any value.
fn fits(member: &Value, value: &Value) -> bool {
    let kinds: Vec<&str> = match member.get("type") {
        Some(Value::String(kind)) => vec![kind.as_str()],
        Some(Value::Array(kinds)) => kinds.iter().filter_map(Value::as_str).collect(),
        _ => Vec::new(),
    };
    let kind_fits = kinds.is_empty() || kinds.iter().any(|kind| is_kind(kind, value));
    let listed = member.get("enum").and_then(Value::as_array);
    kind_fits && listed.is_none_or(|values| value.is_null() || values.contains(value))
}

/// Whether a value is of a kind of JSON schema.
fn is_kind(kind: &str, value: &Value) -> bool {
    match kind {
        "string" => value.is_string(),
        "integer" => value.is_i64() || value.is_u64(),
        "number" => value.is_number(),
        "boolean" => value.is_boolean(),
        "array" => value.is_array(),
        "object" => value.is_object(),
        "null" => value.is_null(),
        _ => true,
    }
}

/// The values a member takes, when its schema lists them.
fn choices(schema: &Value, member: &str) -> Vec<String> {
    schema
        .get("properties")
        .and_then(|all| all.get(member))
        .and_then(|member| member.get("enum"))
        .and_then(Value::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

/// The sentence an AI reads about a value refused at a field: why, and the field. The one writer of
/// it, for MCP and for the assistant.
#[must_use]
pub fn field_refusal(code: ErrorCode, field: &str) -> String {
    format!(
        "{} {}",
        code.text().render(),
        message::ai_refused_field(field).render()
    )
}

/// The sentence an AI reads about an input refused at a member: why, the member (or that it is
/// unknown, when its name is not repeated), and the values it takes when they are known. The one
/// writer of it, for MCP and for the assistant.
#[must_use]
pub fn input_refusal(field: Option<&str>, choices: &[String]) -> String {
    let code = ErrorCode::InvalidRequest;
    let mut sentence = match field {
        Some(field) => field_refusal(code, field),
        None => format!(
            "{} {}",
            code.text().render(),
            Message::from(message::AI_REFUSED_UNKNOWN_FIELD).render()
        ),
    };
    if !choices.is_empty() {
        sentence.push(' ');
        sentence.push_str(&message::ai_refused_choices(choices.join(", ")).render());
    }
    sentence
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn schema() -> Value {
        json!({
            "type": "object",
            "required": ["issue", "version"],
            "properties": {
                "issue": {"type": "string"},
                "version": {"type": "integer"},
                "priority": {"type": ["string", "null"], "enum": ["low", "high"]},
                "limit": {"type": "integer"},
            }
        })
    }

    fn member(input: &Value) -> Option<String> {
        refused_member(&schema(), input)
    }

    #[test]
    fn ai_21_the_refused_member_is_named_by_the_schema() {
        assert_eq!(
            member(&json!({"issue": "A", "version": 1, "colour": "x"})).as_deref(),
            Some("colour")
        );
        assert_eq!(member(&json!({"issue": "A"})).as_deref(), Some("version"));
        assert_eq!(
            member(&json!({"issue": "A", "version": "one"})).as_deref(),
            Some("version")
        );
        assert_eq!(
            member(&json!({"issue": "A", "version": 1, "priority": "urgent"})).as_deref(),
            Some("priority")
        );
        assert_eq!(
            member(&json!({"issue": "A", "version": 1, "priority": null})),
            None
        );
        assert_eq!(member(&json!("not an object")), None);
    }

    #[test]
    fn ai_21_a_refusal_lists_the_values_the_schema_knows_and_never_the_value_given() {
        let refused = refusal(
            &schema(),
            &json!({"issue": "A", "version": 1, "priority": "secret words"}),
        );
        assert_eq!(
            refused,
            AppError::RefusedInput {
                field: Some("priority".to_owned()),
                choices: vec!["low".to_owned(), "high".to_owned()],
            }
        );
        let AppError::RefusedInput { field, choices } = refused else {
            unreachable!("matched above");
        };
        let sentence = input_refusal(field.as_deref(), &choices);
        assert!(sentence.contains("priority"), "{sentence}");
        assert!(sentence.contains("low, high"), "{sentence}");
        assert!(!sentence.contains("secret"), "{sentence}");
        // A member the schema lists no values for is named alone.
        let plain = input_refusal(Some("version"), &[]);
        assert!(plain.contains("version") && !plain.contains(','), "{plain}");
    }

    #[test]
    fn ai_21_an_unknown_member_is_named_only_when_its_name_is_plain() {
        let named = refusal(&schema(), &json!({"issue": "A", "version": 1, "colour": 1}));
        assert_eq!(
            named,
            AppError::RefusedInput {
                field: Some("colour".to_owned()),
                choices: Vec::new(),
            }
        );
        let long = "a".repeat(PLAIN_NAME_MOST + 1);
        for odd in ["My secret note", "Colour", "colour-2", "", long.as_str()] {
            let refused = refusal(&schema(), &json!({"issue": "A", "version": 1, odd: 1}));
            let AppError::RefusedInput { field, choices } = refused else {
                unreachable!("an unknown member is refused at a member");
            };
            assert_eq!(field, None, "{odd}");
            let sentence = input_refusal(field.as_deref(), &choices);
            assert!(sentence.contains("an unknown field"), "{sentence}");
            assert!(odd.is_empty() || !sentence.contains(odd), "{sentence}");
        }
        assert!(plain_name(&"a".repeat(PLAIN_NAME_MOST)));
    }
}
