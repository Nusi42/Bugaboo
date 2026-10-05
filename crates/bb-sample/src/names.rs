//! The names of the sample people: the list written for Bugaboo in `data/names.toml`. That file
//! is the only source; TEST-SAMPLE-DATA reads the same file to check that every name in a made
//! sample comes from it.

use std::collections::BTreeSet;

use serde::Deserialize;

use crate::error::SampleError;

/// The text of the list that is part of this program.
const BUNDLED: &str = include_str!("../data/names.toml");

/// Given names and family names to choose from.
#[derive(Clone, Debug, Deserialize)]
pub struct NameList {
    first: Vec<String>,
    last: Vec<String>,
}

impl NameList {
    /// The list that is part of this program.
    ///
    /// # Errors
    /// Fails when the bundled file is not a usable list (a test keeps it usable).
    pub fn bundled() -> Result<Self, SampleError> {
        Self::parse(BUNDLED)
    }

    /// A list from the text of a `names.toml` file.
    ///
    /// # Errors
    /// Fails when the text is no list of first and last names, or has an empty or repeated name,
    /// or a name with anything but letters (it would not fit in an e-mail address).
    pub fn parse(text: &str) -> Result<Self, SampleError> {
        let list: Self =
            toml::from_str(text).map_err(|error| SampleError::Names(error.to_string()))?;
        for names in [&list.first, &list.last] {
            let distinct: BTreeSet<&String> = names.iter().collect();
            if names.is_empty() || distinct.len() != names.len() {
                return Err(SampleError::NamesRepeated);
            }
            if let Some(odd) = names
                .iter()
                .find(|name| name.is_empty() || !name.chars().all(|c| c.is_ascii_alphabetic()))
            {
                return Err(SampleError::NameNotLetters(odd.clone()));
            }
        }
        Ok(list)
    }

    /// The given names.
    #[must_use]
    pub fn first(&self) -> &[String] {
        &self.first
    }

    /// The family names.
    #[must_use]
    pub fn last(&self) -> &[String] {
        &self.last
    }

    /// Whether `name` is one given name and one family name of the list, in this order.
    #[must_use]
    pub fn contains_full_name(&self, name: &str) -> bool {
        name.split_once(' ').is_some_and(|(first, last)| {
            self.first.iter().any(|known| known == first)
                && self.last.iter().any(|known| known == last)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_bundled_list_is_usable_and_big_enough_for_fifty_people() {
        let list = NameList::bundled().unwrap();
        assert!(list.first().len() >= 50 && list.last().len() >= 50);
        assert!(list.contains_full_name("Ana Moreno"));
        assert!(!list.contains_full_name("Ana"));
        assert!(!list.contains_full_name("Ana Unknown"));
        assert!(!list.contains_full_name("Moreno Ana"));
    }

    #[test]
    fn a_list_with_a_repeated_or_odd_name_is_refused() {
        let error = |text: &str| NameList::parse(text).unwrap_err();
        assert!(matches!(
            error("first = [\"Ana\", \"Ana\"]\nlast = [\"Moreno\"]"),
            SampleError::NamesRepeated
        ));
        assert!(matches!(
            error("first = [\"Ana\"]\nlast = []"),
            SampleError::NamesRepeated
        ));
        assert!(matches!(
            error("first = [\"Zo\u{eb}\"]\nlast = [\"Moreno\"]"),
            SampleError::NameNotLetters(name) if name == "Zo\u{eb}"
        ));
        assert!(matches!(error("not toml at all"), SampleError::Names(_)));
    }
}
