//! The signed reference of a proposal (AI-05, SEC): what makes *Apply* run exactly what was shown.
//!
//! The reference says "this server showed this person this change until this moment". The change
//! itself (the tool and its input) is too long for a token, so it travels beside the reference and
//! is bound into the signature: change one byte of the input or the tool, or use the reference as
//! another person, and it does not open. The key is the one of the signed links of the data folder,
//! under a signature domain of its own. The reference grants nothing: *Apply* still runs the use
//! case as the person, who may have lost the right or the issue in the meantime.

use bb_auth::LinkKey;
use bb_core::ai::PROPOSAL_LIFETIME_SECONDS;
use bb_core::id::UserId;
use jiff::{SignedDuration, Timestamp};
use serde_json::Value;

/// Keeps a reference of a proposal apart from every other claim the key signs.
const PURPOSE: &str = "assistant.apply.v1";

/// The input written with its members in the order of their names, whatever order they came in, so
/// the same change is the same bytes.
fn canonical(value: &Value, out: &mut String) {
    match value {
        Value::Object(members) => {
            let mut names: Vec<&String> = members.keys().collect();
            names.sort();
            out.push('{');
            for name in names {
                out.push_str(&Value::String(name.clone()).to_string());
                out.push(':');
                if let Some(member) = members.get(name) {
                    canonical(member, out);
                }
                out.push(',');
            }
            out.push('}');
        }
        Value::Array(items) => {
            out.push('[');
            for item in items {
                canonical(item, out);
                out.push(',');
            }
            out.push(']');
        }
        other => out.push_str(&other.to_string()),
    }
}

/// What travels beside the reference and is signed with it.
fn bound(tool: &str, input: &Value) -> Vec<u8> {
    let mut text = format!("{tool}\n");
    canonical(input, &mut text);
    text.into_bytes()
}

fn claim(person: UserId, expires_at: i64, word: &str) -> Vec<u8> {
    format!("{PURPOSE}\n{person}\n{expires_at}\n{word}").into_bytes()
}

/// A word that is new for every reference, so a reference can be told from another made for the
/// same change and works once.
fn fresh_word() -> Option<String> {
    let mut bytes = [0_u8; 12];
    getrandom::fill(&mut bytes).ok()?;
    Some(bb_core::hex::encode(&bytes))
}

/// A reference that checked out: its random word and the moment it runs out.
pub(super) struct Opened {
    pub(super) word: String,
    pub(super) expires_at: Timestamp,
}

/// The reference for a change shown to `person` now.
pub(super) fn create(
    key: &LinkKey,
    person: UserId,
    now: Timestamp,
    tool: &str,
    input: &Value,
) -> Option<String> {
    let expires = now.checked_add(SignedDuration::from_secs(PROPOSAL_LIFETIME_SECONDS));
    key.sign_bound(
        &claim(person, expires.ok()?.as_second(), &fresh_word()?),
        &bound(tool, input),
    )
}

/// The word and the end of `reference`, when it was made for exactly this person, tool and input
/// and has not run out. Whether it was used before is for the store to say.
pub(super) fn open(
    key: &LinkKey,
    reference: &str,
    person: UserId,
    now: Timestamp,
    tool: &str,
    input: &Value,
) -> Option<Opened> {
    let opened = key.open_bound(reference, &bound(tool, input))?;
    let text = String::from_utf8(opened).ok()?;
    let mut parts = text.split('\n');
    let (Some(purpose), Some(who), Some(until), Some(word), None) = (
        parts.next(),
        parts.next(),
        parts.next(),
        parts.next(),
        parts.next(),
    ) else {
        return None;
    };
    let until: i64 = until.parse().ok()?;
    (purpose == PURPOSE && who == person.to_string() && until > now.as_second()).then(|| Opened {
        word: word.to_owned(),
        expires_at: Timestamp::from_second(until).unwrap_or(now),
    })
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn the_order_of_members_does_not_change_the_bytes() {
        let (mut one, mut two) = (String::new(), String::new());
        canonical(&json!({"b": [1, {"y": 2, "x": 1}], "a": null}), &mut one);
        canonical(
            &serde_json::from_str(r#"{"a": null, "b": [1, {"x": 1, "y": 2}]}"#).unwrap_or_default(),
            &mut two,
        );
        assert_eq!(one, two);
        assert_ne!(
            bound("a", &json!({"x": "1"})),
            bound("a", &json!({"x": 1})),
            "a text is not a number"
        );
    }
}
