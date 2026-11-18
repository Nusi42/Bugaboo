//! The cells of a row the walk found, read the same from both databases: each column by the type
//! the database declares for it, a moment where the field holds one, and the ID of a person where
//! the field is linked to a person.

use bb_app::privacy::walk::{Cell, FoundRow, Value};
use bb_core::id::{EntityId, UserId};
use bb_core::personal::{LinkScope, PersonLink};
use sqlx::{Column as _, Row as _, TypeInfo as _, ValueRef as _};

use crate::db::{Row, StoredId as _, from_micros};
use crate::error::StoreError;
use crate::schema::StoredTable;

/// The name of the flag that says whether the link at `at` names the person in a row.
pub(super) fn flag(at: usize) -> String {
    format!("person_link_{at}")
}

/// One row: its cells in the order of the columns, and the links whose flag is set.
pub(super) fn read_row(
    row: &Row,
    table: &StoredTable,
    links: &[PersonLink],
) -> Result<FoundRow, StoreError> {
    let mut found = Vec::new();
    for (at, link) in links.iter().enumerate() {
        let set: i64 = match row {
            Row::Sqlite(row) => row.try_get(flag(at).as_str())?,
            Row::Postgres(row) => row.try_get(flag(at).as_str())?,
        };
        if set != 0 {
            found.extend(link.link());
        }
    }
    found.dedup();
    let cells = table
        .columns
        .iter()
        .map(|(column, field)| {
            let raw = raw(row, column)?;
            let value = match raw {
                Raw::Number(micros) if holds_moment(table, field) => {
                    Value::Moment(from_micros(micros)?)
                }
                Raw::Bytes(bytes) if holds_person(table, links, field, column) => {
                    Value::Person(UserId::read(bytes)?)
                }
                other => other.walked(),
            };
            Ok(Cell {
                column: column.clone(),
                field,
                value,
            })
        })
        .collect::<Result<Vec<Cell>, StoreError>>()?;
    Ok(FoundRow {
        links: found,
        cells,
    })
}

/// Whether the field of the entity holds a moment.
fn holds_moment(table: &StoredTable, field: &str) -> bool {
    table
        .marks
        .fields()
        .iter()
        .any(|mark| mark.name() == field && mark.holds_moment())
}

/// Whether `column` holds the ID of a person: the last column of a field that names a person in
/// every row, or the column of a value of an own field of a type that holds people.
fn holds_person(table: &StoredTable, links: &[PersonLink], field: &str, column: &str) -> bool {
    links
        .iter()
        .filter(|link| link.field() == field)
        .any(|link| {
            matches!(link.scope(), LinkScope::Every | LinkScope::FieldType { .. })
                && table
                    .columns
                    .iter()
                    .rev()
                    .find(|(_, of)| *of == field)
                    .is_some_and(|(last, _)| last == column)
        })
}

/// A value as the database gives it, before the field says what it is.
enum Raw {
    Empty,
    Number(i64),
    Decimal(f64),
    Truth(bool),
    Text(String),
    Bytes(Vec<u8>),
}

impl Raw {
    fn walked(self) -> Value {
        match self {
            Self::Empty => Value::Empty,
            Self::Number(number) => Value::Number(number),
            Self::Decimal(number) => Value::Decimal(number),
            Self::Truth(truth) => Value::Truth(truth),
            Self::Text(text) => Value::Text(text),
            Self::Bytes(bytes) => <[u8; 16]>::try_from(bytes.as_slice())
                .ok()
                .and_then(|id| EntityId::from_bytes(id).ok())
                .map_or_else(
                    || Value::Bytes(bb_core::hex::encode(&bytes)),
                    |id| Value::Id(id.to_string()),
                ),
        }
    }
}

/// The value of `column`, by the type the database declares for the column: whole numbers,
/// decimals, truth values, texts and bytes are the only types the schema uses.
fn raw(row: &Row, column: &str) -> Result<Raw, StoreError> {
    macro_rules! read {
        ($row:expr) => {{
            let row = $row;
            if row.try_get_raw(column)?.is_null() {
                return Ok(Raw::Empty);
            }
            let declared = row
                .try_column(column)?
                .type_info()
                .name()
                .to_ascii_uppercase();
            if declared.contains("BOOL") {
                Raw::Truth(row.try_get(column)?)
            } else if declared.contains("INT") {
                Raw::Number(row.try_get(column)?)
            } else if ["REAL", "DOUBLE", "FLOAT", "NUMERIC"]
                .iter()
                .any(|name| declared.contains(name))
            {
                Raw::Decimal(row.try_get(column)?)
            } else if declared.contains("BLOB") || declared.contains("BYTEA") {
                Raw::Bytes(row.try_get(column)?)
            } else {
                Raw::Text(row.try_get(column)?)
            }
        }};
    }
    Ok(match row {
        Row::Sqlite(row) => read!(row),
        Row::Postgres(row) => read!(row),
    })
}
