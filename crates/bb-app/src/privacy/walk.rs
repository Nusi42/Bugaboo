//! What the walk of a person's data hands over (ADR-0044, decision 1): for each kind of record
//! (a table) the rows that name the person, cell by cell, with what the person is to each row. The
//! store finds them from the links of the entities (`bb_core::personal::PersonLinks`) and nothing
//! else; the export and the removal read them through the port
//! [`PersonData`](crate::ports::PersonData) and search no row by person themselves.

use bb_core::id::UserId;
use bb_core::personal::{EntityMarks, Link};
use jiff::Timestamp;

/// How many rows the walk hands over at a time (coding-standards §6: no list without a page).
pub const ROWS_PAGE: u64 = 500;

/// One kind of record that names the person, and how many of its rows do.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KindOfRows {
    /// The kind: the name of the table.
    pub kind: String,
    /// The marks and links of the entity stored in it.
    pub marks: EntityMarks,
    /// How many of its rows name the person.
    pub rows: u64,
}

/// A page of the rows of one kind that name the person.
#[derive(Clone, Debug, PartialEq)]
pub struct RowsPage {
    /// The rows, in a fixed order.
    pub rows: Vec<FoundRow>,
    /// Where the next page starts; none after the last.
    pub next: Option<u64>,
}

/// One row that names the person.
#[derive(Clone, Debug, PartialEq)]
pub struct FoundRow {
    /// What the person is to the row, by each link that names them in it.
    pub links: Vec<Link>,
    /// The cells of the row, in the order of the columns.
    pub cells: Vec<Cell>,
}

/// One cell of a row.
#[derive(Clone, Debug, PartialEq)]
pub struct Cell {
    /// The column.
    pub column: String,
    /// The field of the entity the column holds.
    pub field: &'static str,
    /// The value.
    pub value: Value,
}

/// The value of a cell, as both databases give it.
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    /// No value.
    Empty,
    /// A whole number.
    Number(i64),
    /// A number with a decimal part.
    Decimal(f64),
    /// Yes or no.
    Truth(bool),
    /// A moment (a field of the type `Timestamp`).
    Moment(Timestamp),
    /// A text.
    Text(String),
    /// The ID of a person, in a column of a field that names a person.
    Person(UserId),
    /// The ID of a record.
    Id(String),
    /// Other bytes, as hexadecimal digits.
    Bytes(String),
}
