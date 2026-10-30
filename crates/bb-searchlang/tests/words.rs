//! The words of the search language (D3, ADR-0005 amendment M5, decision 11), tested through its
//! public functions.

#![cfg(test)]

use bb_core as _;
use proptest as _;
use serde as _;
use serde_json as _;

use bb_searchlang::messages;
use bb_searchlang::vectors::sample_fields;
use bb_searchlang::{
    Field, FieldKind, ReadableFields, Suggestion, check, parse_with, print, suggest,
};

/// A text parsed and checked against the sample fields, printed.
fn checked(text: &str) -> Option<String> {
    let search = parse_with(text, &sample_fields()).ok()?;
    check(search, &sample_fields())
        .ok()
        .map(|checked| print(checked.search()))
}

#[test]
fn qry_05_the_words_of_the_old_language_read_and_are_shown_in_the_new_words() {
    // D3 (decision): fields and values in the glossary's words; every old form still reads.
    for (old, shown) in [
        ("statusGroup is not done", "status group is not Done"),
        (
            "statusGroup is inProgress",
            "status group is \"In progress\"",
        ),
        ("assignedTo is me", "assigned to is me"),
        ("PRIORITY >= high", "priority >= High"),
        ("priority in (low, HIGH)", "priority in (Low, High)"),
        (
            "title is x sort by assignedTo",
            "title is x sort by assigned to",
        ),
        ("timeSpent > 2 hours", "time spent > 2 hours"),
        ("status group is not Done", "status group is not Done"),
        ("\"status group\" is not Done", "status group is not Done"),
    ] {
        assert_eq!(checked(old).as_deref(), Some(shown), "{old}");
        // What is shown reads back as itself.
        assert_eq!(checked(shown).as_deref(), Some(shown), "{shown}");
    }
}

#[test]
fn qry_05_an_own_field_with_a_name_of_several_words_is_written_bare_or_in_quotes() {
    let readable = ReadableFields {
        fields: vec![
            Field::new("story points", FieldKind::Number),
            Field::new("a is b", FieldKind::Number),
        ],
    };
    let shown = |text: &str| {
        let tree = parse_with(text, &readable).unwrap();
        check(tree, &readable).map(|checked| print(checked.search()))
    };
    assert_eq!(shown("storyPoints > 3").as_deref(), Ok("story points > 3"));
    assert_eq!(shown("\"a is b\" < 2").as_deref(), Ok("\"a is b\" < 2"));
}

#[test]
fn qry_01_an_unknown_field_before_the_end_of_an_unfinished_text_is_told_first() {
    // S7: `prio is high and ` stops at the end; the wrong word is what the person is told first.
    let answer = bb_searchlang::read("prio is high and ", Some(&sample_fields()));
    assert!(answer.canonical.is_none());
    assert_eq!(answer.mistakes[0].key, messages::UNKNOWN_FIELD_DID_YOU_MEAN);
    assert_eq!(answer.mistakes[0].args["field"], "prio");
    assert_eq!(answer.mistakes[0].args["suggestion"], "priority");
    assert_eq!(
        (answer.mistakes[0].span.start, answer.mistakes[0].span.end),
        (0, 4)
    );
    // The mistake of the end follows, after it.
    assert_eq!(answer.mistakes.len(), 2);
    assert!(answer.mistakes[1].span.start >= answer.mistakes[0].span.end);
    // An unfinished text with nothing wrong before its end has the one mistake of its end.
    let answer = bb_searchlang::read("priority is ", Some(&sample_fields()));
    assert_eq!(answer.mistakes.len(), 1);
}

#[test]
fn qry_05_two_choices_of_an_own_field_that_differ_in_spaces_or_case_are_never_merged() {
    let readable = ReadableFields {
        fields: vec![
            Field::unordered("kind").with_choices(&["Bug fix", "bugfix", "Task"]),
            Field::unordered("priority").with_choices(&["Low", "High"]),
            Field::unordered("status group").with_choices(&["Not started", "In progress"]),
        ],
    };
    let shown = |text: &str| {
        let tree = parse_with(text, &readable).ok()?;
        check(tree, &readable)
            .ok()
            .map(|checked| print(checked.search()))
    };
    // The same name, in any case and with any spaces between words, is that choice.
    assert_eq!(shown("kind is BUGFIX").as_deref(), Some("kind is bugfix"));
    assert_eq!(
        shown("kind is \"bug   FIX\"").as_deref(),
        Some("kind is \"Bug fix\"")
    );
    // Case alone never tells the two apart, spaces do: this is `bugfix`, not `Bug fix`.
    assert_eq!(shown("kind is bugFix").as_deref(), Some("kind is bugfix"));
    // A spelling that is neither of the two names is not a choice, and picks neither.
    assert_eq!(shown("kind is bugf\"ix\""), None);
    assert_eq!(shown("kind is \"bugf ix\""), None);
    // The built-in lists read the old run-together form.
    assert_eq!(
        shown("statusGroup is inProgress").as_deref(),
        Some("status group is \"In progress\"")
    );
    assert_eq!(shown("kind is taskk"), None);
}

/// A name that begins with the words of a direction is quoted as a key of `sort by`, where the
/// words would be read as the direction of the key before it, and offered so (ADR-0005 amendment
/// M5, decision 11); in a filter it stands bare.
#[test]
fn qry_03_a_field_that_begins_like_a_direction_is_a_quoted_key_of_sort_by() {
    let fields = ReadableFields {
        fields: vec![
            Field::new("status", FieldKind::Choice).with_choices(&["Open"]),
            Field::new("newest release", FieldKind::Date),
        ],
    };
    let canonical = |text: &str| {
        let search = parse_with(text, &fields).map_err(|mistake| vec![mistake])?;
        check(search, &fields).map(|search| print(search.search()))
    };
    let text = r#"newest release is today sort by status, "newest release""#;
    assert_eq!(canonical(text).as_deref(), Ok(text));
    assert_eq!(
        canonical("sort by newest release, status").as_deref(),
        Ok(r#"sort by "newest release", status"#)
    );
    // Bare after a key, the words are the direction of the key.
    assert!(canonical("sort by status, newest release").is_err());
    let offered = |text: &str| -> Vec<String> {
        suggest(text, text.len(), &fields)
            .items
            .into_iter()
            .filter_map(|item| match item {
                Suggestion::Word { text, .. } => Some(text),
                Suggestion::Ask { .. } => None,
            })
            .collect()
    };
    assert!(offered("sort by status, ").contains(&r#""newest release""#.to_owned()));
    assert!(offered("").contains(&"newest release".to_owned()));
}
