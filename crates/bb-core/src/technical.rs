//! Technical names: the short, fixed words that name a kind of job, a consumer of events, a secret
//! or an action in the security log. They are never typed by a person as free text, so they cannot
//! hold personal data and a field of this type may be `#[not_personal]` (ADR-0015).

use std::fmt;

use serde::{Deserialize, Serialize};

/// The longest technical name, in characters.
const MAX_LENGTH: usize = 64;

/// The text is not a technical name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InvalidTechnicalName;

impl fmt::Display for InvalidTechnicalName {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "a technical name starts with a lower-case letter, has at most {MAX_LENGTH} characters \
             and holds only lower-case letters, digits, dots, hyphens and underscores"
        )
    }
}

impl std::error::Error for InvalidTechnicalName {}

/// A short fixed name like `mail.send`, `search-index` or `sign_in.failed`.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct TechnicalName(String);

impl TechnicalName {
    /// The technical name `name`.
    ///
    /// # Errors
    /// Fails unless `name` starts with a lower-case letter, is at most 64 characters long and
    /// holds only lower-case letters, digits, dots, hyphens and underscores.
    pub fn new(name: &str) -> Result<Self, InvalidTechnicalName> {
        let valid = name.starts_with(|c: char| c.is_ascii_lowercase())
            && name.len() <= MAX_LENGTH
            && name.chars().all(|c| {
                c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '.' | '-' | '_')
            });
        if valid {
            Ok(Self(name.to_owned()))
        } else {
            Err(InvalidTechnicalName)
        }
    }

    /// The name as text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for TechnicalName {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl TryFrom<String> for TechnicalName {
    type Error = InvalidTechnicalName;

    fn try_from(name: String) -> Result<Self, InvalidTechnicalName> {
        Self::new(&name)
    }
}

impl From<TechnicalName> for String {
    fn from(name: TechnicalName) -> Self {
        name.0
    }
}

// It cannot hold anything a person typed: the characters and the length are fixed.
impl crate::personal::NotPersonal for TechnicalName {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn good_example_dotted_lower_case_words() {
        for name in ["mail.send", "search-index", "sign_in.failed", "k1"] {
            assert_eq!(TechnicalName::new(name).unwrap().as_str(), name);
        }
    }

    #[test]
    fn bad_example_anything_else_is_refused() {
        let long = "a".repeat(65);
        for name in ["", "Mail", "1st", "a b", "a@b", "ä", long.as_str()] {
            assert_eq!(
                TechnicalName::new(name),
                Err(InvalidTechnicalName),
                "{name}"
            );
        }
        assert!(serde_json::from_str::<TechnicalName>("\"No Good\"").is_err());
    }
}
