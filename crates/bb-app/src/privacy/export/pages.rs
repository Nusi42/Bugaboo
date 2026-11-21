//! The readable half of the file (ADR-0044, decision 2): plain HTML pages with one style sheet of
//! the colours of *Daylight* written into each page, no script, every text a person wrote rendered
//! by `bb-markdown` and every other word escaped by it, dates in the person's *Region*.

use std::fmt::Write as _;

use bb_core::message::{self, Message, MessageKey};
use bb_core::personal::{EntityMarks, FieldMark};
use bb_core::theme::{OwnValues, shown_colors};
use bb_core::wording::Region;
use bb_markdown::{Options, escape_text, render};
use bb_policy::PersonRef;

use super::shape::{ShapedKind, Shown};

/// The built-in theme whose colours the pages wear: the light one every person can read.
const PAGE_THEME: &str = "daylight";

/// The text of a message of the catalogue.
pub(super) fn words(key: MessageKey) -> String {
    Message::from(key).render()
}

/// How a kind of record is named, in plain words: its message `personal-kind.…`. The walk finds
/// only kinds that name a person, and each of them has one (`#[derive(Entity)]`), so the name of
/// the struct is only the answer for a kind no list shows.
pub(crate) fn title_of(marks: &EntityMarks) -> String {
    marks.title().map_or_else(|| marks.name().to_owned(), words)
}

/// How a field is named, in plain words: its message `personal-field.<entity>.<field>`, the header of its column.
/// The walk finds only fields of kinds that name a person, and each of them has one
/// (`#[derive(Entity)]`), so the name in the code is only the answer for a field no list shows.
pub(super) fn label_of(field: &FieldMark) -> String {
    field.title().map_or_else(|| field.name().to_owned(), words)
}

/// The style sheet of every page: the colour tokens of [`PAGE_THEME`] as variables, and the few
/// rules of a table that reads on paper and on a screen.
fn style() -> String {
    let mut sheet = String::from(":root{");
    for color in shown_colors(PAGE_THEME, &OwnValues::default()).unwrap_or_default() {
        let _ = write!(sheet, "--{}:{};", color.name, color.value);
    }
    sheet.push_str(
        "}body{background:var(--bg-canvas);color:var(--fg);font-family:system-ui,sans-serif;\
         margin:2em;line-height:1.5}a{color:var(--accent-fg)}table{border-collapse:collapse}\
         th,td{border:1px solid var(--border);padding:.3em .6em;text-align:left;vertical-align:top}\
         th{background:var(--bg-surface)}",
    );
    sheet
}

/// A whole page with `title` and `body`.
fn page(title: &str, body: &str) -> String {
    format!(
        "<!doctype html><html><head><meta charset=\"utf-8\"><title>{}</title><style>{}</style>\
         </head><body>{body}</body></html>",
        escape_text(title),
        style()
    )
}

/// How a person is written on a page.
fn person_text(person: &PersonRef) -> String {
    match person {
        PersonRef::Name { name, .. } => name.clone(),
        PersonRef::Role { role } => message::export_page_role(role).render(),
        PersonRef::Someone => words(message::EXPORT_PAGE_SOMEONE),
        PersonRef::Former { code } => message::export_page_former(code).render(),
    }
}

/// A text a person wrote, rendered as the screens render it, with no picture from outside and no
/// link but to the web.
fn written(text: &str) -> String {
    render(
        text,
        &Options {
            image_hosts: Vec::new(),
            plain_tokens: Vec::new(),
            ai_answer: false,
            web_links_only: true,
        },
    )
    .html
}

/// A cell on a page.
fn cell(value: &Shown, region: &Region) -> String {
    match value {
        Shown::Empty => String::new(),
        Shown::Number(number) => number.to_string(),
        Shown::Decimal(number) => number.to_string(),
        Shown::Truth(true) => words(message::EXPORT_PAGE_YES),
        Shown::Truth(false) => words(message::EXPORT_PAGE_NO),
        Shown::Moment(moment) => escape_text(&region.moment(*moment)),
        Shown::Text {
            text,
            written: true,
        } => written(text),
        Shown::Text { text, .. } | Shown::Id(text) | Shown::Bytes(text) => escape_text(text),
        Shown::Person(person) => escape_text(&person_text(person)),
    }
}

/// The page of one kind of record: a table of its rows, a column for each field.
#[must_use]
pub(super) fn kind_page(kind: &ShapedKind, region: &Region) -> String {
    let title = &title_of(&kind.marks);
    let mut body = format!(
        "<p><a href=\"index.html\">{}</a></p><h1>{}</h1><table><tr><th>{}</th>",
        escape_text(&words(message::EXPORT_PAGE_ALL_KINDS)),
        escape_text(title),
        escape_text(&words(message::EXPORT_PAGE_WHAT_YOU_ARE)),
    );
    let fields: Vec<&str> = kind
        .rows
        .first()
        .map(|row| row.cells.iter().map(|cell| cell.field).collect())
        .unwrap_or_default();
    for field in &fields {
        let label = kind
            .marks
            .fields()
            .iter()
            .find(|mark| mark.name() == *field)
            .map_or_else(|| (*field).to_owned(), label_of);
        let _ = write!(body, "<th>{}</th>", escape_text(&label));
    }
    body.push_str("</tr>");
    for row in &kind.rows {
        let links: Vec<String> = row.links.iter().map(|link| words(link.name())).collect();
        let _ = write!(body, "<tr><td>{}</td>", escape_text(&links.join(", ")));
        for shaped in &row.cells {
            let _ = write!(body, "<td>{}</td>", cell(&shaped.value, region));
        }
        body.push_str("</tr>");
    }
    body.push_str("</table>");
    page(title, &body)
}

/// One line of the first page: a kind of record and how many of its rows are in the file.
#[derive(Clone, Debug)]
pub(super) struct IndexLine {
    /// The kind: the name of the table, and of its page.
    pub kind: String,
    /// How it is named.
    pub title: String,
    /// How many rows.
    pub rows: usize,
}

/// The first page: what the file holds, and a link to the page of each kind of record.
#[must_use]
pub(super) fn index_page(lines: &[IndexLine]) -> String {
    let title = words(message::EXPORT_PAGE_TITLE);
    let mut body = format!(
        "<h1>{}</h1><p>{}</p><h2>{}</h2><table><tr><th>{}</th><th>{}</th></tr>",
        escape_text(&title),
        escape_text(&words(message::EXPORT_PAGE_INTRO)),
        escape_text(&words(message::EXPORT_PAGE_KINDS)),
        escape_text(&words(message::EXPORT_PAGE_KINDS)),
        escape_text(&words(message::EXPORT_PAGE_ROWS)),
    );
    for line in lines {
        let _ = write!(
            body,
            "<tr><td><a href=\"{}.html\">{}</a></td><td>{}</td></tr>",
            escape_text(&line.kind),
            escape_text(&line.title),
            line.rows
        );
    }
    body.push_str("</table>");
    page(&title, &body)
}
