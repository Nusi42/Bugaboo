//! Projects and their short names (PRJ-01, ADR-0025 decision 13, M2-Q4).

use std::fmt;

use jiff::Timestamp;
use serde::{Deserialize, Serialize};

use crate::id::ProjectId;
use crate::personal::Entity;

/// The fewest characters of a short name.
pub const SHORT_NAME_MIN: usize = 2;
/// The most characters of a short name.
pub const SHORT_NAME_MAX: usize = 10;

/// The text is not a short name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InvalidShortName;

impl fmt::Display for InvalidShortName {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "a short name has {SHORT_NAME_MIN} to {SHORT_NAME_MAX} capital letters or digits and starts with a letter"
        )
    }
}

impl std::error::Error for InvalidShortName {}

/// The letters that start a project's issue IDs and its address (`WEB`, `APP2`): 2 to 10 capital
/// letters or digits, starting with a letter. It holds nothing but those characters, so it can
/// never carry personal data. Whether it is free is the store's business (unique in the
/// organization, old short names included).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct ShortName(String);

impl ShortName {
    /// The short name `text`.
    ///
    /// # Errors
    /// Fails unless `text` is 2 to 10 capital ASCII letters or digits and starts with a letter.
    pub fn new(text: &str) -> Result<Self, InvalidShortName> {
        let valid = (SHORT_NAME_MIN..=SHORT_NAME_MAX).contains(&text.len())
            && text.starts_with(|c: char| c.is_ascii_uppercase())
            && text
                .chars()
                .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit());
        if valid {
            Ok(Self(text.to_owned()))
        } else {
            Err(InvalidShortName)
        }
    }

    /// The short name as text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for ShortName {
    type Error = InvalidShortName;

    fn try_from(text: String) -> Result<Self, InvalidShortName> {
        Self::new(&text)
    }
}

impl From<ShortName> for String {
    fn from(name: ShortName) -> Self {
        name.0
    }
}

impl fmt::Display for ShortName {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

// Only capital letters and digits: nothing a person could write about themselves.
impl crate::personal::NotPersonal for ShortName {}

/// A project (PRJ-01): a place in the tree where roles are given and work happens. Projects hold
/// sub-projects; a project at the top has no parent. Its name and short name can change and it
/// can move (WP-08); every change counts `version` up.
#[derive(Clone, Debug, PartialEq, Eq, Entity)]
pub struct Project {
    /// The ID of the project.
    #[not_personal]
    pub id: ProjectId,
    /// The project it is inside, or `None` at the top level.
    #[not_personal]
    pub parent: Option<ProjectId>,
    /// The letters of its issue IDs and address.
    #[not_personal]
    pub short_name: ShortName,
    /// What people call it. Free text that could name a person.
    #[personal(category = "content", basis = "legitimate_interests")]
    pub name: String,
    /// Counts up with every change, for the check that nobody changed it meanwhile (ADR-0019).
    #[not_personal]
    pub version: i64,
}

/// An old short name of a project (M2-Q4): it stays with the project, so addresses that used it
/// keep working (`/p/OLD` opens the project), and nobody else can take it. Only that project can
/// take it back.
#[derive(Clone, Debug, PartialEq, Eq, Entity)]
pub struct ProjectAlias {
    /// The old short name.
    #[not_personal]
    pub short_name: ShortName,
    /// The project it stays with.
    #[not_personal]
    pub project: ProjectId,
    /// When the project stopped using it.
    #[not_personal]
    pub retired_at: Timestamp,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prj_01_a_short_name_breaks_no_rule_of_m2_q4() {
        for good in ["WEB", "APP2", "AB", "ABCDEFGHIJ", "A1", "X9Z"] {
            assert!(ShortName::new(good).is_ok(), "{good}");
        }
        for bad in [
            "",
            "A",
            "ABCDEFGHIJK",
            "web",
            "Web",
            "2WEB",
            "WE B",
            "WEB-1",
            "WEB_",
            "ÄB",
            "W\u{0}B",
            " WEB",
            "WEB\n",
        ] {
            assert_eq!(ShortName::new(bad), Err(InvalidShortName), "{bad:?}");
        }
    }

    #[test]
    fn a_short_name_survives_json_and_is_checked_when_read() {
        let name = ShortName::new("WEB").unwrap();
        assert_eq!(serde_json::to_string(&name).unwrap(), "\"WEB\"");
        assert_eq!(serde_json::from_str::<ShortName>("\"WEB\"").unwrap(), name);
        assert!(serde_json::from_str::<ShortName>("\"web\"").is_err());
    }
}
