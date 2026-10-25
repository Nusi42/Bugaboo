//! WebAssembly bindings for the search language, a module of its own (ADR-0005, ADR-0011).
//!
//! This crate only passes calls through to `bb-searchlang`, so the browser runs the very code the
//! server runs. It holds no rule of its own. The search bar loads the built module (`cargo xtask
//! wasm`) when it first opens, and no page before it (coding-standards §6). The browser gives and
//! gets JSON; the positions in it are bytes of the UTF-8 text, which the package of the browser
//! turns into positions in the string.

#![forbid(unsafe_code)]

use serde::Serialize;
use wasm_bindgen::JsError;
use wasm_bindgen::prelude::wasm_bindgen;

/// Writes an answer as the JSON text the browser reads.
fn write_json(answer: &impl Serialize) -> Result<String, String> {
    serde_json::to_string(answer).map_err(|error| format!("unwritable answer: {error}"))
}

/// Reads the readable fields of the browser. JSON that is not what the Rust types hold is an error
/// that says what is wrong with it: the browser and the Rust types differ, which is a defect and
/// must not pass as an empty answer.
fn read_fields(json: &str) -> Result<bb_searchlang::ReadableFields, JsError> {
    bb_searchlang::readable_fields_from_json(json)
        .map_err(|problem| thrown(&format!("unreadable input: {problem}")))
}

/// Gives the error of the Rust side to the browser.
fn thrown(error: &str) -> JsError {
    JsError::new(error)
}

/// What a text comes to without the fields: its tree, its canonical text or its first mistake
/// (`bb_searchlang::Answer`, as JSON).
///
/// # Errors
/// The answer cannot be written as JSON.
#[wasm_bindgen]
pub fn parse(text: &str) -> Result<String, JsError> {
    write_json(&bb_searchlang::read(text, None)).map_err(|e| thrown(&e))
}

/// What a text comes to with the fields the person may read: the same answer, with the mistakes
/// of the checker.
///
/// # Errors
/// The readable fields are not a `bb_searchlang::ReadableFields` as JSON.
#[wasm_bindgen]
pub fn check(text: &str, fields: &str) -> Result<String, JsError> {
    let fields = read_fields(fields)?;
    write_json(&bb_searchlang::read(text, Some(&fields))).map_err(|e| thrown(&e))
}

/// The canonical text of a tree, which Simple search edits.
///
/// # Errors
/// The text is not a `bb_searchlang::Search` as JSON, the tree breaks a rule of a tree, or the
/// language does not read its text back as the same tree.
#[wasm_bindgen]
pub fn print(search: &str) -> Result<String, JsError> {
    bb_searchlang::print_from_json(search).map_err(|e| thrown(&format!("unreadable input: {e}")))
}

/// What may come next at byte `cursor` of the text (`bb_searchlang::Suggestions`, as JSON).
///
/// # Errors
/// The readable fields are not a `bb_searchlang::ReadableFields` as JSON.
#[wasm_bindgen]
pub fn suggest(text: &str, cursor: usize, fields: &str) -> Result<String, JsError> {
    let fields = read_fields(fields)?;
    write_json(&bb_searchlang::suggest(text, cursor, &fields)).map_err(|e| thrown(&e))
}

/// The word of an own field, made from its name by the one rule of the language.
#[must_use]
#[wasm_bindgen]
pub fn own_field_word(name: &str) -> String {
    bb_searchlang::own_field_word(name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use bb_searchlang::vectors::sample_fields;

    fn fields_json() -> String {
        write_json(&sample_fields()).unwrap_or_default()
    }

    #[test]
    fn the_browser_gets_the_answer_of_the_server() {
        let text = "status is Open sort by priority, highest first";
        let fields = bb_searchlang::readable_fields_from_json(&fields_json());
        let json = write_json(&bb_searchlang::read(text, fields.as_ref().ok())).unwrap_or_default();
        let expected = bb_searchlang::read(text, Some(&sample_fields()));
        assert_eq!(json, write_json(&expected).unwrap_or_default());
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap_or_default();
        assert_eq!(parsed["canonical"], text);
    }

    #[test]
    fn a_tree_comes_back_to_its_canonical_text() {
        let answer = bb_searchlang::read("status  is Open", None);
        let tree = serde_json::to_string(&answer.search).unwrap_or_default();
        assert_eq!(
            bb_searchlang::print_from_json(&tree),
            Ok("status is Open".to_owned())
        );
    }

    #[test]
    fn the_suggestions_are_those_of_the_server() {
        let fields = bb_searchlang::readable_fields_from_json(&fields_json()).unwrap_or_default();
        let json = write_json(&bb_searchlang::suggest("assi", 4, &fields)).unwrap_or_default();
        let found: serde_json::Value = serde_json::from_str(&json).unwrap_or_default();
        assert_eq!(found["items"][0]["text"], "assignedTo");
    }

    #[test]
    fn a_text_that_is_no_input_is_an_error_that_says_so() {
        assert!(bb_searchlang::readable_fields_from_json("not json").is_err());
        assert!(bb_searchlang::print_from_json("{}").is_err());
    }

    #[test]
    fn the_word_of_an_own_field_is_the_rule_of_the_language() {
        assert_eq!(own_field_word("Story points"), "storyPoints");
    }
}
