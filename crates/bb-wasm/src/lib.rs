//! WebAssembly bindings for the search language, Markdown and validation (ADR-0005, ADR-0011).
//!
//! This crate only passes calls through to the pure crates, so the browser runs the very code the
//! server runs. It holds no rule of its own. The browser loads the built module (`cargo xtask
//! wasm`) only when a preview or an editor opens (coding-standards §6).

#![forbid(unsafe_code)]

use bb_core::fields::Field;
use bb_core::wording::{self, Change, Notice, Side};
use bb_markdown::Options;
use serde::Deserialize;
use serde::Serialize;
use wasm_bindgen::JsError;
use wasm_bindgen::prelude::wasm_bindgen;

/// Renders Markdown to safe HTML, exactly as the server does (`bb_markdown::render`).
/// `image_hosts` are the hosts of the organization setting `content.image-hosts`.
#[must_use]
#[wasm_bindgen]
pub fn render(text: &str, image_hosts: Vec<String>) -> String {
    bb_markdown::render(
        text,
        &Options {
            image_hosts,
            ..Options::default()
        },
    )
    .html
}

/// Reads the JSON text of the browser as `Input`, gives it to `work` and writes the answer as JSON.
/// A text that is not an `Input` is an error that says what is wrong with it: the browser and the
/// Rust types differ, which is a defect and must not pass as an empty answer.
fn answer<Input: for<'a> Deserialize<'a>, Output: Serialize>(
    json: &str,
    work: impl FnOnce(&Input) -> Output,
) -> Result<String, String> {
    let input = serde_json::from_str(json).map_err(|error| format!("unreadable input: {error}"))?;
    serde_json::to_string(&work(&input)).map_err(|error| format!("unwritable answer: {error}"))
}

/// The sentence of a change of one field, as the server words it (`bb_core::wording`): the message
/// keys with the words for their placeholders, as JSON. The browser looks the keys up in its own
/// catalog.
///
/// # Errors
/// The text is not a change as `bb_core::wording::Change` reads it.
#[wasm_bindgen]
pub fn change_sentence(change: &str) -> Result<String, JsError> {
    answer(change, |change: &Change| wording::change_sentence(change)).map_err(|e| JsError::new(&e))
}

/// The sentences of a notification, as a list in JSON (see [`change_sentence`]).
///
/// # Errors
/// The text is not a notice as `bb_core::wording::Notice` reads it.
#[wasm_bindgen]
pub fn notification_sentences(notice: &str) -> Result<String, JsError> {
    answer(notice, |notice: &Notice| {
        wording::notification_sentences(notice)
    })
    .map_err(|e| JsError::new(&e))
}

/// One side of a change that is asked about on its own, with the code of its field.
#[derive(Deserialize)]
struct SideOf {
    field: String,
    side: Side,
}

/// The words of one side of a change, or `null` when it holds no value (see [`change_sentence`]).
///
/// # Errors
/// The text is not a field with a side as `bb_core::wording::Side` reads it.
#[wasm_bindgen]
pub fn side_words(side: &str) -> Result<String, JsError> {
    answer(side, |asked: &SideOf| {
        wording::side_said(Field::from_code(&asked.field), &asked.side)
    })
    .map_err(|e| JsError::new(&e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_browser_gets_the_html_of_the_server() {
        let text = "**hi** <script>alert(1)</script>";
        assert_eq!(
            render(text, Vec::new()),
            bb_markdown::render(text, &Options::default()).html
        );
    }

    #[test]
    fn the_browser_gets_the_sentence_of_the_server() {
        let change = r#"{"who":"Ben","field":"priority","field_name":"Priority",
            "before":{"shown":"text","text":"normal"},"after":{"shown":"text","text":"high"}}"#;
        let json =
            answer(change, |change: &Change| wording::change_sentence(change)).unwrap_or_default();
        let said: serde_json::Value = serde_json::from_str(&json).unwrap_or_default();
        assert_eq!(said["key"], "history.changed");
        assert_eq!(said["args"]["after"]["key"], "priority.high");
    }

    #[test]
    fn a_text_that_is_no_input_is_an_error_that_says_so() {
        let error = answer("not json", |change: &Change| {
            wording::change_sentence(change)
        });
        assert!(error.is_err_and(|text| text.contains("unreadable input")));
        // A side of a kind that Rust does not know is an error too, not an empty answer.
        let odd = r#"{"field":"tags","side":{"shown":"colors"}}"#;
        let asked = answer(odd, |asked: &SideOf| {
            wording::side_said(Field::from_code(&asked.field), &asked.side)
        });
        assert!(asked.is_err());
    }

    #[test]
    fn a_side_without_a_value_is_null() {
        let empty = r#"{"field":"tags","side":{"shown":"names","names":[]}}"#;
        let words = answer(empty, |asked: &SideOf| {
            wording::side_said(Field::from_code(&asked.field), &asked.side)
        });
        assert_eq!(words.as_deref(), Ok("null"));
    }

    #[test]
    fn the_sentences_of_a_notification_come_as_a_list() {
        let notice = r#"{"kind":"mentioned","who":"Ana"}"#;
        let json = answer(notice, |notice: &Notice| {
            wording::notification_sentences(notice)
        })
        .unwrap_or_default();
        let list: serde_json::Value = serde_json::from_str(&json).unwrap_or_default();
        assert_eq!(list[0]["key"], "notifications.mentioned");
    }
}
