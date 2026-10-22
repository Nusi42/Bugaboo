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

/// Reads the JSON text of the browser as `Input`, gives it to `work` and writes the answer as JSON;
/// `null` for a text that is not an `Input`.
fn answer<Input: for<'a> Deserialize<'a>, Output: Serialize>(
    json: &str,
    work: impl FnOnce(&Input) -> Output,
) -> String {
    serde_json::from_str(json)
        .ok()
        .and_then(|input| serde_json::to_string(&work(&input)).ok())
        .unwrap_or_else(|| "null".to_owned())
}

/// The sentence of a change of one field, as the server words it (`bb_core::wording`): the message
/// keys with the words for their placeholders, as JSON. The browser looks the keys up in its own
/// catalog.
#[must_use]
#[wasm_bindgen]
pub fn change_sentence(change: &str) -> String {
    answer(change, |change: &Change| wording::change_sentence(change))
}

/// The sentences of a notification, as a list in JSON (see [`change_sentence`]).
#[must_use]
#[wasm_bindgen]
pub fn notification_sentences(notice: &str) -> String {
    answer(notice, |notice: &Notice| {
        wording::notification_sentences(notice)
    })
}

/// One side of a change that is asked about on its own, with the code of its field.
#[derive(Deserialize)]
struct SideOf {
    field: String,
    side: Side,
}

/// The words of one side of a change, or `null` when it holds no value (see [`change_sentence`]).
#[must_use]
#[wasm_bindgen]
pub fn side_words(side: &str) -> String {
    answer(side, |asked: &SideOf| {
        wording::side_said(Field::from_code(&asked.field), &asked.side)
    })
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
        let said: serde_json::Value =
            serde_json::from_str(&change_sentence(change)).unwrap_or_default();
        assert_eq!(said["key"], "history.changed");
        assert_eq!(said["args"]["after"]["key"], "priority.high");
    }

    #[test]
    fn a_text_that_is_no_input_gets_null() {
        assert_eq!(change_sentence("not json"), "null");
        assert_eq!(side_words("{}"), "null");
        let empty = r#"{"field":"tags","side":{"shown":"names","names":[]}}"#;
        assert_eq!(side_words(empty), "null");
    }

    #[test]
    fn the_sentences_of_a_notification_come_as_a_list() {
        let notice = r#"{"kind":"mentioned","who":"Ana"}"#;
        let list: serde_json::Value =
            serde_json::from_str(&notification_sentences(notice)).unwrap_or_default();
        assert_eq!(list[0]["key"], "notifications.mentioned");
    }
}
