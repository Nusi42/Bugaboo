//! The file of an own theme: what *Download* gives and *Upload* takes (THM-04, ADR-0026 update M8).
//! A JSON object with the name, the built-in theme it extends and the values; the one definition of
//! its shape, for writing and for reading.

use serde::Serialize;

use super::values::{OwnValues, ValueProblem, ValueProblemKind};

/// The parts of the file.
const NAME: &str = "name";
const EXTENDS: &str = "extends";
const VALUES: &str = "values";

/// An own theme as a file.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ThemeFile {
    /// What the theme is called.
    pub name: String,
    /// The ID of the built-in theme it extends.
    pub extends: String,
    /// The values it changes.
    pub values: OwnValues,
}

/// Why a file is no file of a theme.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FileProblem {
    /// It is no JSON object with a name and a built-in theme as text.
    NotATheme,
    /// It has parts or values an own theme may not have: each named.
    Values(Vec<ValueProblem>),
}

impl ThemeFile {
    /// Reads a file from its JSON. A part that is not one of the three, a token that is not on the
    /// list and a value of the wrong shape are named; nothing of the file is kept as text.
    ///
    /// # Errors
    /// What is wrong with the file.
    pub fn read(file: &serde_json::Value) -> Result<Self, FileProblem> {
        let object = file.as_object().ok_or(FileProblem::NotATheme)?;
        let text = |key: &str| {
            object
                .get(key)
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned)
                .ok_or(FileProblem::NotATheme)
        };
        let (name, extends) = (text(NAME)?, text(EXTENDS)?);
        let mut problems: Vec<ValueProblem> = object
            .keys()
            .filter(|key| ![NAME, EXTENDS, VALUES].contains(&key.as_str()))
            .map(|key| ValueProblem::new(key, ValueProblemKind::UnknownToken))
            .collect();
        let values = match object.get(VALUES) {
            Some(values) => OwnValues::from_json(values).unwrap_or_else(|found| {
                problems.extend(found);
                OwnValues::default()
            }),
            None => OwnValues::default(),
        };
        if !problems.is_empty() {
            return Err(FileProblem::Values(problems));
        }
        Ok(Self {
            name,
            extends,
            values,
        })
    }
}
