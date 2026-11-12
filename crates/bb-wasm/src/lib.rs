//! WebAssembly bindings for Markdown, the wording of changes and notifications, the rule of the
//! same name and the readability of an own theme (ADR-0011, ADR-0033, ADR-0026 update M8). The search language has a module of its own, `bb-wasm-search`
//! (ADR-0005, amendment M5).
//!
//! This crate only passes calls through to the pure crates, so the browser runs the very code the
//! server runs. It holds no rule of its own. The browser loads the built module (`cargo xtask
//! wasm`) only when a preview or an editor opens (coding-standards §6).

#![forbid(unsafe_code)]

pub mod names;

use bb_core::fields::Field;
use bb_core::history::TimeEvent;
use bb_core::plan::TimeFactor;
use bb_core::theme::{OwnValues, ThemeRead, theme_read};
use bb_core::time::Minutes;
use bb_core::wording::{self, Change, File, Notice, Side, Time, TimeChange};
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
    written(&work(&input))
}

/// Writes an answer as JSON for the browser.
fn written<Output: Serialize>(output: &Output) -> Result<String, String> {
    serde_json::to_string(output).map_err(|error| format!("unwritable answer: {error}"))
}

/// The sentences of the changes of one event, as the server words them (`bb_core::wording`): one
/// for each change, and one for a *Start date* and a *Due date* that both move. Each is the message
/// keys with the words for their placeholders; the answer is a list of them, as JSON. The browser
/// looks the keys up in its own catalog. A single change is a list of one: one entry, so the module
/// carries one reader of changes (PERF-04).
///
/// # Errors
/// The text is not a list of changes as `bb_core::wording::Change` reads them.
#[wasm_bindgen]
pub fn change_sentences(changes: &str) -> Result<String, JsError> {
    sentences_of(changes).map_err(|e| JsError::new(&e))
}

/// The sentences of the changes the browser asks about, as JSON (see [`change_sentences`]).
fn sentences_of(changes: &str) -> Result<String, String> {
    answer(changes, |changes: &Vec<Change>| {
        wording::change_sentences(changes)
    })
}

/// The sentence of a file that was added, deleted or brought back (see [`change_sentences`]).
///
/// # Errors
/// The text is not a file as `bb_core::wording::File` reads it.
#[wasm_bindgen]
pub fn file_sentence(file: &str) -> Result<String, JsError> {
    answer(file, |file: &File| wording::file_sentence(file)).map_err(|e| JsError::new(&e))
}

/// The sentence of time that was booked on an issue, changed or deleted (see [`change_sentences`]):
/// who did it, the word of what happened (`booked`, `changed`, `deleted`; another word, or none,
/// is an event this version does not know) and the length in whole minutes, if the reader may read
/// it. The three come as they are, not as JSON: reading them as JSON would put a reader of its own
/// for `bb_core::wording::Time` into the module, which the browser fetches on every history.
///
/// # Errors
/// The answer cannot be written, a defect.
#[wasm_bindgen]
pub fn time_sentence(who: &str, event: &str, minutes: Option<u32>) -> Result<String, JsError> {
    let time = Time {
        who: who.to_owned(),
        what: TimeChange {
            event: TimeEvent::from_code(event),
            minutes,
        },
    };
    written(&wording::time_sentence(&time)).map_err(|e| JsError::new(&e))
}

/// The sentences of a notification, as a list in JSON (see [`change_sentences`]).
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

/// The words of one side of a change, or `null` when it holds no value (see [`change_sentences`]).
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

/// Reads a length of time a person typed, as the server reads it (`bb_core::time::length`), without
/// a bound (an own field): the minutes, or the sentence of the mistake, as JSON.
///
/// # Errors
/// The text is not a typed length as `bb_core::wording::LengthTyped` reads it.
#[wasm_bindgen]
pub fn read_length(typed: &str) -> Result<String, JsError> {
    answer(typed, wording::length_read).map_err(|e| JsError::new(&e))
}

/// Reads a length of one time entry a person typed, with the bound of an entry (see
/// [`read_length`]).
///
/// # Errors
/// The text is not a typed length as `bb_core::wording::LengthTyped` reads it.
#[wasm_bindgen]
pub fn read_time_entry(typed: &str) -> Result<String, JsError> {
    answer(typed, wording::time_entry_read).map_err(|e| JsError::new(&e))
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

/// The effort of an issue with an estimate of `estimate` minutes and a time factor of `percent`
/// percent, as the plan counts it (`TimeFactor::effort`: rounded up to the whole minute); `None`
/// for a factor outside the range.
#[must_use]
#[wasm_bindgen]
pub fn factor_effort(estimate: u32, percent: u16) -> Option<u32> {
    TimeFactor::new(percent).map(|factor| factor.effort(Minutes::new(estimate)).get())
}

/// The time factor in percent of *Takes me …* (`takes` minutes) against an estimate of `estimate`
/// minutes, as the server works it out (`TimeFactor::of_length`); `None` without an estimate or
/// outside the range.
#[must_use]
#[wasm_bindgen]
pub fn factor_of_length(estimate: u32, takes: u32) -> Option<u16> {
    TimeFactor::of_length(Minutes::new(estimate), Minutes::new(takes)).map(TimeFactor::percent)
}

/// An own theme as the editor asks about it: the built-in theme it extends and its values, in the
/// shape of a theme file.
#[derive(Deserialize)]
struct ThemeAsked {
    extends: String,
    values: serde_json::Value,
}

/// What the theme editor is told about an own theme (`bb_core::theme::theme_read`): its sums, the
/// pairs and colors that are hard to read, the worst rating of each color and the colors it shows,
/// rated as the server rates a save. JSON, or `null` for values a save would refuse or a built-in
/// theme that does not exist.
///
/// # Errors
/// The text is not a theme with `extends` and `values`.
#[wasm_bindgen]
pub fn theme_readability(asked: &str) -> Result<String, JsError> {
    answer(asked, rate_asked).map_err(|e| JsError::new(&e))
}

/// The readability of the theme the editor asks about, or `None` (see [`theme_readability`]).
fn rate_asked(asked: &ThemeAsked) -> Option<ThemeRead> {
    theme_read(&asked.extends, &OwnValues::from_json(&asked.values).ok()?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thm_03_the_editor_gets_the_readability_of_the_server() {
        let asked = r##"{"extends":"daylight","values":{"colors":{"fg":{"value":"#ffffff"}}}}"##;
        let json = theme_readability_text(asked);
        let said: serde_json::Value = serde_json::from_str(&json).unwrap_or_default();
        let hard = said["hard"].as_array().cloned().unwrap_or_default();
        assert!(hard.iter().any(|pair| pair["first"] == "fg"));
        assert_eq!(
            Some(hard.len() as u64),
            said["summary"]["hard_to_read"].as_u64()
        );
        assert!(said["summary"]["hard_to_read"].as_u64() > Some(0));
        let colors = said["colors"].as_array().cloned().unwrap_or_default();
        assert!(
            colors
                .iter()
                .any(|color| color["name"] == "fg" && color["value"] == "#ffffff")
        );
        let unknown =
            r##"{"extends":"daylight","values":{"colors":{"nothing":{"value":"#ffffff"}}}}"##;
        assert_eq!(theme_readability_text(unknown), "null");
    }

    fn theme_readability_text(asked: &str) -> String {
        answer(asked, rate_asked).unwrap_or_default()
    }

    #[test]
    fn plan_14_the_browser_counts_a_time_factor_as_the_plan_does() {
        // 7h times 1.07 is 449.4 minutes: the plan rounds up, so a factor never loses work.
        assert_eq!(factor_effort(420, 107), Some(450));
        assert_eq!(factor_effort(480, 5), None);
        // Takes 10h against 8h: ×1.25.
        assert_eq!(factor_of_length(480, 600), Some(125));
        assert_eq!(factor_of_length(0, 600), None);
    }

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
        let change = r#"[{"who":"Ben","field":"priority","field_name":"Priority",
            "before":{"shown":"text","text":"normal"},"after":{"shown":"text","text":"high"}}]"#;
        let json = sentences_of(change).unwrap_or_default();
        let said: serde_json::Value = serde_json::from_str(&json).unwrap_or_default();
        assert_eq!(said[0]["key"], "history.changed");
        assert_eq!(said[0]["args"]["after"]["key"], "priority.high");
    }

    #[test]
    fn a_text_that_is_no_input_is_an_error_that_says_so() {
        let error = sentences_of("not json");
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
