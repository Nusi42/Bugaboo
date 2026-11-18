//! The values of the rows a PostgreSQL copy holds, and how each is written in the copy: a JSON
//! array per row, where a whole number and a text are plain, a byte string is `{"b": "<hex>"}`, a
//! fraction `{"f": 1.5}` and a missing value `null`. The kind of each column is written once, in
//! the head of its table, so a missing value goes back with the right type.

use bb_core::hex;
use serde_json::{Value as Json, json};
use sqlx::Row as _;

use crate::error::StoreError;
use crate::rows::Row;

/// The kinds of column Bugaboo's schema has (all whole numbers are `BIGINT`, architecture §7).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ColumnKind {
    Whole,
    Fraction,
    YesNo,
    Text,
    Bytes,
}

impl ColumnKind {
    /// The kind of a column of this type as `information_schema` names it.
    pub(crate) fn from_type(name: &str) -> Option<Self> {
        Some(match name {
            "bigint" | "integer" | "smallint" => Self::Whole,
            "double precision" | "real" => Self::Fraction,
            "boolean" => Self::YesNo,
            "text" | "character varying" | "character" => Self::Text,
            "bytea" => Self::Bytes,
            _ => return None,
        })
    }

    /// The word the head of a table in the copy uses.
    pub(crate) const fn word(self) -> &'static str {
        match self {
            Self::Whole => "whole",
            Self::Fraction => "fraction",
            Self::YesNo => "yes-no",
            Self::Text => "text",
            Self::Bytes => "bytes",
        }
    }

    /// The kind with this word.
    pub(crate) fn from_word(word: &str) -> Option<Self> {
        [
            Self::Whole,
            Self::Fraction,
            Self::YesNo,
            Self::Text,
            Self::Bytes,
        ]
        .into_iter()
        .find(|kind| kind.word() == word)
    }
}

fn unreadable() -> StoreError {
    StoreError::Corrupt("a row of the copy of a database")
}

/// The values of a row of PostgreSQL as the copy writes them, column by column of `kinds`.
pub(crate) fn row_to_json(row: &Row, kinds: &[ColumnKind]) -> Result<Json, StoreError> {
    let Row::Postgres(row) = row else {
        return Err(StoreError::UnsupportedDatabase);
    };
    let mut values = Vec::with_capacity(kinds.len());
    for (index, kind) in kinds.iter().enumerate() {
        let value = match kind {
            ColumnKind::Whole => row.try_get::<Option<i64>, _>(index)?.map(Json::from),
            ColumnKind::Fraction => row
                .try_get::<Option<f64>, _>(index)?
                .map(|number| json!({ "f": number })),
            ColumnKind::YesNo => row.try_get::<Option<bool>, _>(index)?.map(Json::from),
            ColumnKind::Text => row.try_get::<Option<String>, _>(index)?.map(Json::from),
            ColumnKind::Bytes => row
                .try_get::<Option<Vec<u8>>, _>(index)?
                .map(|bytes| json!({ "b": hex::encode(&bytes) })),
        };
        values.push(value.unwrap_or(Json::Null));
    }
    Ok(Json::Array(values))
}

/// A row of the copy as values to bind, column by column of `kinds`.
pub(crate) fn json_to_values(
    row: &Json,
    kinds: &[ColumnKind],
) -> Result<Vec<sea_query::Value>, StoreError> {
    let values = row.as_array().ok_or_else(unreadable)?;
    if values.len() != kinds.len() {
        return Err(unreadable());
    }
    values
        .iter()
        .zip(kinds)
        .map(|(value, kind)| value_of(value, *kind))
        .collect()
}

fn value_of(value: &Json, kind: ColumnKind) -> Result<sea_query::Value, StoreError> {
    use sea_query::Value;
    let null = value.is_null();
    Ok(match kind {
        ColumnKind::Whole if null => Value::BigInt(None),
        ColumnKind::Whole => Value::BigInt(Some(value.as_i64().ok_or_else(unreadable)?)),
        ColumnKind::Fraction if null => Value::Double(None),
        ColumnKind::Fraction => Value::Double(Some(
            value
                .get("f")
                .and_then(Json::as_f64)
                .ok_or_else(unreadable)?,
        )),
        ColumnKind::YesNo if null => Value::Bool(None),
        ColumnKind::YesNo => Value::Bool(Some(value.as_bool().ok_or_else(unreadable)?)),
        ColumnKind::Text if null => Value::String(None),
        ColumnKind::Text => Value::String(Some(value.as_str().ok_or_else(unreadable)?.to_owned())),
        ColumnKind::Bytes if null => Value::Bytes(None),
        ColumnKind::Bytes => {
            let text = value
                .get("b")
                .and_then(Json::as_str)
                .ok_or_else(unreadable)?;
            let mut bytes = vec![0_u8; text.len() / 2];
            if text.len() % 2 != 0 || !hex::decode_into(text, &mut bytes) {
                return Err(unreadable());
            }
            Value::Bytes(Some(bytes))
        }
    })
}

/// The bytes of a column of `row` of the copy, for the check of the stored secrets before a
/// restore; `None` when the value is missing.
pub(crate) fn bytes_of(value: &Json) -> Result<Option<Vec<u8>>, StoreError> {
    match value_of(value, ColumnKind::Bytes)? {
        sea_query::Value::Bytes(bytes) => Ok(bytes),
        _ => Err(unreadable()),
    }
}

/// The text of a column of `row` of the copy.
pub(crate) fn text_of(value: &Json) -> Result<Option<String>, StoreError> {
    match value_of(value, ColumnKind::Text)? {
        sea_query::Value::String(text) => Ok(text),
        _ => Err(unreadable()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_row_of_the_copy_reads_back_with_its_kinds() {
        let kinds = [
            ColumnKind::Whole,
            ColumnKind::Fraction,
            ColumnKind::YesNo,
            ColumnKind::Text,
            ColumnKind::Bytes,
            ColumnKind::Bytes,
        ];
        let row = json!([7, { "f": 1.5 }, true, "Printer jams", { "b": "00ff" }, null]);
        let values = json_to_values(&row, &kinds).unwrap();
        assert_eq!(
            values,
            [
                sea_query::Value::BigInt(Some(7)),
                sea_query::Value::Double(Some(1.5)),
                sea_query::Value::Bool(Some(true)),
                sea_query::Value::String(Some("Printer jams".to_owned())),
                sea_query::Value::Bytes(Some(vec![0, 255])),
                sea_query::Value::Bytes(None),
            ]
        );
        for kind in kinds {
            assert_eq!(ColumnKind::from_word(kind.word()), Some(kind));
        }
        assert!(json_to_values(&json!([1]), &kinds).is_err());
        assert!(json_to_values(&json!(["x"]), &[ColumnKind::Whole]).is_err());
        assert!(json_to_values(&json!([{ "b": "abc" }]), &[ColumnKind::Bytes]).is_err());
    }
}
