//! WebAssembly bindings for Markdown, the wording of changes and notifications and the rule of the
//! same name (ADR-0011, ADR-0033). The search language has a module of its own, `bb-wasm-search`
//! (ADR-0005, amendment M5).
//!
//! This crate only passes calls through to the pure crates, so the browser runs the very code the
//! server runs. It holds no rule of its own. The browser loads the built module (`cargo xtask
//! wasm`) only when a preview or an editor opens (coding-standards §6).

#![forbid(unsafe_code)]

pub mod names;

use bb_core::fields::Field;
use bb_core::time::Minutes;
use bb_core::wording::{self, Change, File, Notice, Side};
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

/// The sentence of a file that was added, deleted or brought back (see [`change_sentence`]).
///
/// # Errors
/// The text is not a file as `bb_core::wording::File` reads it.
#[wasm_bindgen]
pub fn file_sentence(file: &str) -> Result<String, JsError> {
    answer(file, |file: &File| wording::file_sentence(file)).map_err(|e| JsError::new(&e))
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

/// Reads a length of time a person typed, as the server reads it (`bb_core::time::length`): the
/// minutes, or the sentence of the mistake, as JSON.
///
/// # Errors
/// The text is not a typed length as `bb_core::wording::LengthTyped` reads it.
#[wasm_bindgen]
pub fn read_length(typed: &str) -> Result<String, JsError> {
    answer(typed, wording::length_read).map_err(|e| JsError::new(&e))
}

/// A length of whole minutes in a sentence, as the server words it (`bb_core::wording`): message
/// keys with their words, as JSON.
#[must_use]
#[wasm_bindgen]
pub fn length_sentence(minutes: u32) -> String {
    serde_json::to_string(&wording::length_sentence(Minutes::new(minutes))).unwrap_or_default()
}

/// A length of whole minutes on a clock, as a grid shows it: `1:30` (`Minutes::on_clock`).
#[must_use]
#[wasm_bindgen]
pub fn length_on_clock(minutes: u32) -> String {
    Minutes::new(minutes).on_clock()
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
    fn the_browser_reads_a_length_as_the_server_does() {
        let read = |json: &str| answer(json, wording::length_read).unwrap_or_default();
        assert_eq!(
            read(r#"{"text":"1,5","number_format":"comma","hour_unit":"h","minute_unit":"m"}"#),
            r#"{"minutes":90,"mistake":null}"#
        );
        let wrong: serde_json::Value = serde_json::from_str(&read(
            r#"{"text":"soon","number_format":"point","hour_unit":"h","minute_unit":"m"}"#,
        ))
        .unwrap_or_default();
        assert_eq!(wrong["mistake"]["key"], "time.length.not-a-length");
        assert_eq!(length_on_clock(90), "1:30");
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
