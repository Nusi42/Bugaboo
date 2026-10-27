//! The test vectors of the search language: texts, their canonical texts, their mistakes and their
//! suggestions, all made by the Rust code the server runs. `cargo xtask gen search-vectors` writes
//! them to a file, and the unit tests of the browser run the WebAssembly module on the same texts,
//! so both sides prove the same results (QRY-03; the files are held to the code by DOC-GENERATED).
//! The sample fields are also what the tests of this crate use.

use serde::Serialize;

use crate::answer::{Answer, read};
use crate::fields::{Field, FieldKind, ReadableFields};
use crate::suggest::{Suggestions, suggest};

/// The fields of the vectors, as a person who may read all of them would get them.
#[must_use]
pub fn sample_fields() -> ReadableFields {
    let priority = ["lowest", "low", "normal", "high", "highest"];
    ReadableFields {
        fields: vec![
            Field::new("project", FieldKind::Project),
            Field::new("title", FieldKind::Text),
            Field::new("description", FieldKind::Text),
            Field::new("status", FieldKind::Choice).with_choices(&["Open", "In progress", "Done"]),
            Field::new("statusGroup", FieldKind::Choice).with_choices(&[
                "To do",
                "In progress",
                "Done",
            ]),
            Field::new("priority", FieldKind::Choice).with_choices(&priority),
            Field::new("tag", FieldKind::Choice),
            Field::new("assignedTo", FieldKind::Person),
            Field::new("reportedBy", FieldKind::Person),
            Field::new("parent", FieldKind::Issue),
            Field::new("created", FieldKind::Date),
            Field::new("changed", FieldKind::Date),
            Field::new("due", FieldKind::Date),
            Field::new("estimate", FieldKind::Number),
            Field::new("storyPoints", FieldKind::Number),
            Field::new("timeSpent", FieldKind::Duration),
        ],
    }
}

/// Texts that are searches: each parses, checks against the sample fields and prints to its
/// canonical text (the first example is the one of QRY-01).
pub const ACCEPTED: &[&str] = &[
    "project within WEB and statusGroup is not Done and assignedTo is me and changed in last 7 days sort by priority, highest first",
    "",
    "sort by due, oldest first, title, A to Z",
    "status is Open",
    "status IS NOT \"In progress\"",
    "status in (Open, \"In progress\")",
    "priority >= high",
    "estimate <= 2.50 or estimate > 10",
    "timeSpent > 2 hours",
    "timeSpent <= 1 hour 30 minutes",
    "timeSpent is 45 minutes",
    "timeSpent < 90 minutes and timeSpent >= 1 hour",
    "timeSpent is 2 hours 90 minutes",
    "title contains \"crash on save\"",
    "title contains crash",
    "assignedTo member of \"Backend\"",
    "assignedTo is empty",
    "reportedBy is not empty",
    "created is today or created is yesterday",
    "changed is this week and due is last week and due is this month",
    "due < 2026-12-31",
    "changed in last 1 day",
    "changed in last 2 weeks",
    "changed in last 6 months",
    "project within WEB",
    "parent is WEB-142",
    "parent is WEB-K7Q2X",
    "under WEB-7",
    "contains \"crash on save\"",
    "contains crash and not contains \"login error\" and status is Open",
    "(contains crash or contains save) and priority is high sort by created",
    "blocks WEB-1",
    "blocked by WEB-1",
    "connected to WEB-7",
    "connected to WEB-7 within 3 steps",
    "connected to WEB-7 within 1 step",
    "not (status is Done or status is Open)",
    "(status is Open and priority is high) or assignedTo is me",
    "(status is Open and (priority is high))",
    "not not status is Open",
    "status is Open and (priority is high and assignedTo is me)",
    "storyPoints > 5 sort by storyPoints",
    "tag is re-test",
    "tag in (re-test, WEB-1)",
    "parent is web-142",
    "project within web",
    "under web-7",
    "title contains \"Größe\"",
    "title is \"ünï 😀\" and description contains \"naïve café\"",
];

/// Texts that are not accepted, each for a different reason (some with letters of more than one
/// byte, so the places of the mistakes are not the same in bytes and in units of the string).
pub const REFUSED: &[&str] = &[
    "prio is high",
    "priority < 3",
    "status is",
    "status is Finished",
    "status is Donee",
    "status",
    "status is Open and",
    "status is Open or or",
    "(status is Open",
    "status is Open)",
    "title contains",
    "contains",
    "contains and status is Open",
    "contains \"unfinished",
    "title contains \"unfinished",
    "title < 5",
    "estimate is high",
    "timeSpent > 5",
    "timeSpent > 1.5 hours",
    "estimate > 2 hours",
    "estimate is \"5\"",
    "due is soon",
    "due is 2026-13-40",
    "assignedTo is 5",
    "parent is WEB",
    "status is week",
    "changed in last 0 days",
    "changed in last 3 hours",
    "changed in",
    "under 12",
    "project within and",
    "blocked WEB-1",
    "connected to WEB-7 within 6 steps",
    "connected to WEB-7 within three steps",
    "sort priority",
    "sort by",
    "sort by priority, sideways first",
    "sort by nothing",
    "status = Open",
    "status is Open, priority is high",
    "ünï is Open",
    "title contains \"Größe\" and ünï < 3",
    "title is 😀",
    "status is Öpen",
    "title contains \"Größe and",
];

/// Where the cursor is in a text, marked with a bar that is not part of the text. Some texts hold
/// letters of more than one byte, so the browser has places to turn from bytes into units of the
/// string.
pub const SUGGESTED: &[&str] = &[
    "|",
    "assi|",
    "assignedTo |",
    "priority is |",
    "priority is not |",
    "priority is hi|",
    "priority is \"hi|",
    "status is \"In pr|\" and",
    "assignedTo is |",
    "assignedTo member of |",
    "assignedTo member of \"Bac|",
    "created is |",
    "created |",
    "changed in |",
    "changed in last 3 |",
    "status is Open |",
    "(status is Open |",
    "status in (Open |",
    "status in (Open, |",
    "not |",
    "blocked |",
    "connected to |",
    "connected to WEB-7 within 3 |",
    "project within |",
    "parent is |",
    "status is Open sort |",
    "status is Open sort by |",
    "status is Open sort by priority |",
    "status is Open sort by priority, high|",
    "status is Open sort by priority, highest |",
    "sort by priority, |",
    "nothing |",
    "title contains \"Grö|",
    "ünï|",
    "title contains \"Größe\" and assi|",
    "description contains \"😀\" and sta|",
    "title is \"Größe\" and |",
];

/// One text and what the code makes of it.
#[derive(Serialize)]
struct VectorCase {
    text: String,
    answer: Answer,
}

/// One cursor place and what is offered there.
#[derive(Serialize)]
struct VectorPlace {
    text: String,
    cursor: usize,
    suggestions: Suggestions,
}

/// The whole file.
#[derive(Serialize)]
struct VectorFile {
    readable_fields: ReadableFields,
    cases: Vec<VectorCase>,
    places: Vec<VectorPlace>,
}

/// A text with the cursor bar taken out, and where the bar stood.
fn cursor_in(marked: &str) -> (String, usize) {
    let at = marked.find('|').unwrap_or(marked.len());
    (marked.replacen('|', "", 1), at)
}

/// The text of the vectors file.
///
/// # Errors
/// The vectors cannot be written as JSON.
pub fn file() -> Result<String, serde_json::Error> {
    let readable = sample_fields();
    let cases = ACCEPTED
        .iter()
        .chain(REFUSED)
        .map(|text| VectorCase {
            text: (*text).to_owned(),
            answer: read(text, Some(&readable)),
        })
        .collect();
    let places = SUGGESTED
        .iter()
        .map(|marked| {
            let (text, cursor) = cursor_in(marked);
            VectorPlace {
                suggestions: suggest(&text, cursor, &readable),
                text,
                cursor,
            }
        })
        .collect();
    let mut text = serde_json::to_string_pretty(&VectorFile {
        readable_fields: readable,
        cases,
        places,
    })?;
    text.push('\n');
    Ok(text)
}
