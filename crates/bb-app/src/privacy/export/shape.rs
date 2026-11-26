//! What of a row the walk found goes into the file (ADR-0044, decision 3): never a credential
//! (password hashes, keys of passkeys, secrets of access keys), other people only as the person may
//! see them now (`PersonRef`), and the texts of others not at all, only that they name the person.

use std::collections::BTreeMap;

use bb_core::id::UserId;
use bb_core::personal::{Category, EntityMarks, FieldMark, Link, Personal};
use bb_policy::PersonRef;
use jiff::Timestamp;

use crate::privacy::walk::{FoundRow, Value};

/// One kind of record as it goes into the file.
#[derive(Clone, Debug, PartialEq)]
pub struct ShapedKind {
    /// The kind: the name of the table.
    pub kind: String,
    /// The marks and links of the entity stored in it.
    pub marks: EntityMarks,
    /// The rows, in the order of the walk.
    pub rows: Vec<ShapedRow>,
}

/// One row as it goes into the file.
#[derive(Clone, Debug, PartialEq)]
pub struct ShapedRow {
    /// What the person is to the row.
    pub links: Vec<Link>,
    /// The cells that go into the file, in the order of the columns.
    pub cells: Vec<ShapedCell>,
}

/// One cell as it goes into the file.
#[derive(Clone, Debug, PartialEq)]
pub struct ShapedCell {
    /// The field of the entity.
    pub field: &'static str,
    /// The column that holds it: a field can be stored in several (the place of a box of a
    /// dashboard, the options of the timeline of a saved search).
    pub column: String,
    /// What is shown of it.
    pub value: Shown,
}

/// A value as it goes into the file.
#[derive(Clone, Debug, PartialEq)]
pub enum Shown {
    /// No value.
    Empty,
    /// A whole number.
    Number(i64),
    /// A number with a decimal part.
    Decimal(f64),
    /// Yes or no.
    Truth(bool),
    /// A moment.
    Moment(Timestamp),
    /// A text; `written` when a person wrote it, so the pages render it with `bb-markdown`.
    Text {
        /// The text.
        text: String,
        /// Whether a person wrote it (a field of the category *content*).
        written: bool,
    },
    /// A person, as the person whose data it is sees them now.
    Person(PersonRef),
    /// The ID of a record.
    Id(String),
    /// Other bytes, as hexadecimal digits.
    Bytes(String),
}

/// The categories of the mark of `field`: its own, and the ones it has in the rows of each kind
/// of record.
fn categories(field: &FieldMark) -> impl Iterator<Item = Category> + '_ {
    let own = match field.personal_data() {
        Personal::Yes { category, .. } => Some(category),
        Personal::No => None,
    };
    own.into_iter().chain(
        field
            .by_kind()
            .iter()
            .filter_map(|mark| match mark.personal_data() {
                Personal::Yes { category, .. } => Some(category),
                Personal::No => None,
            }),
    )
}

/// Whether `field` holds `category` anywhere.
fn holds(marks: &EntityMarks, field: &str, category: Category) -> bool {
    marks
        .fields()
        .iter()
        .find(|mark| mark.name() == field)
        .is_some_and(|mark| categories(mark).any(|found| found == category))
}

/// Whether somebody other than `person` wrote or did the row: a field linked as *author* or *actor*
/// holds another person.
fn by_somebody_else(row: &FoundRow, marks: &EntityMarks, person: UserId) -> bool {
    let links = marks.person_links();
    row.cells.iter().any(|cell| match cell.value {
        Value::Person(other) if other != person => links
            .of_field(cell.field)
            .any(|link| matches!(link.link(), Some(Link::Author | Link::Actor))),
        _ => false,
    })
}

/// The row as it goes into the file of `person`'s data. `seen` is how the person sees each other
/// person of the row now; somebody missing from it is *Someone*.
#[must_use]
pub fn shape_row(
    row: FoundRow,
    marks: &EntityMarks,
    person: UserId,
    seen: &BTreeMap<UserId, PersonRef>,
) -> ShapedRow {
    let others_wrote = by_somebody_else(&row, marks, person);
    let cells = row
        .cells
        .into_iter()
        .filter(|cell| !holds(marks, cell.field, Category::Credentials))
        .filter(|cell| !(others_wrote && holds(marks, cell.field, Category::Content)))
        .map(|cell| {
            let written = holds(marks, cell.field, Category::Content);
            let value = match cell.value {
                Value::Empty => Shown::Empty,
                Value::Number(number) => Shown::Number(number),
                Value::Decimal(number) => Shown::Decimal(number),
                Value::Truth(truth) => Shown::Truth(truth),
                Value::Moment(moment) => Shown::Moment(moment),
                Value::Text(text) => Shown::Text { text, written },
                Value::Person(id) => {
                    Shown::Person(seen.get(&id).cloned().unwrap_or(PersonRef::Someone))
                }
                Value::Id(id) => Shown::Id(id),
                Value::Bytes(bytes) => Shown::Bytes(bytes),
            };
            ShapedCell {
                field: cell.field,
                column: cell.column,
                value,
            }
        })
        .collect();
    ShapedRow {
        links: row.links,
        cells,
    }
}

/// The people a row names, so they can be looked up once for a whole page.
pub fn people_in(row: &FoundRow) -> impl Iterator<Item = UserId> + '_ {
    row.cells.iter().filter_map(|cell| match cell.value {
        Value::Person(id) => Some(id),
        _ => None,
    })
}

#[cfg(test)]
#[path = "tests/shape.rs"]
mod tests;
