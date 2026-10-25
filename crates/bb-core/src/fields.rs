//! The field catalogue (ADR-0025 decision 10, M2-Q7): the built-in fields that field permissions
//! apply to, and [`FieldKey`], the one key that names a field of either kind (CF-06).
//!
//! The built-in list is declared here once. The policy engine masks by [`FieldKey`], the API
//! description and *Check access* name the fields from it, and the screens of the field-permission
//! matrix are made from it. Own fields are rows, so they are named by their ID and never listed
//! here (architecture §7, *The field catalogue*).

use std::borrow::Cow;
use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::id::FieldId;
use crate::people_kinds::PeopleKind;

words! {
    /// A built-in field. The word is the key of the field (`assigned-to`).
    pub enum Field {
        /// *Title*.
        Title => "title",
        /// *Description*.
        Description => "description",
        /// *Status*.
        Status => "status",
        /// *Priority*.
        Priority => "priority",
        /// *Assigned to*.
        AssignedTo => "assigned-to",
        /// *Reported by*.
        ReportedBy => "reported-by",
        /// *Due date*.
        DueDate => "due-date",
        /// *Labels*.
        Labels => "labels",
        /// *Tags*.
        Tags => "tags",
        /// *Planned for release*.
        PlannedForRelease => "planned-for-release",
        /// *Found in release*.
        FoundInRelease => "found-in-release",
        /// *Estimated time*.
        EstimatedTime => "estimated-time",
        /// *Time left*.
        TimeLeft => "time-left",
        /// *Time spent*.
        TimeSpent => "time-spent",
        /// *Activity*.
        Activity => "activity",
        /// *Cost center*.
        CostCenter => "cost-center",
    }
    messages name = "issue-field.";
}

words_in_api!(
    Field,
    "A built-in field of the field catalogue, by its key."
);

/// The key of a field, built-in or own: what levels, masks, the matrix of *Field permissions*,
/// history and `fields=` name a field by. A built-in field has its word (`assigned-to`), an own
/// field its ID.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FieldKey {
    /// A field of the catalogue.
    BuiltIn(Field),
    /// A field the organization made.
    Own(FieldId),
}

impl From<Field> for FieldKey {
    fn from(field: Field) -> Self {
        Self::BuiltIn(field)
    }
}

impl From<FieldId> for FieldKey {
    fn from(field: FieldId) -> Self {
        Self::Own(field)
    }
}

impl FieldKey {
    /// The key as text: the word of a built-in field, the ID of an own field. The name of a
    /// member of an event, of a level and of `fields=`.
    #[must_use]
    pub fn code(self) -> Cow<'static, str> {
        match self {
            Self::BuiltIn(field) => Cow::Borrowed(field.code()),
            Self::Own(field) => Cow::Owned(field.to_string()),
        }
    }

    /// The key with this text, if it names a built-in field or is the ID of an own field.
    #[must_use]
    pub fn from_code(code: &str) -> Option<Self> {
        Field::from_code(code)
            .map(Self::BuiltIn)
            .or_else(|| code.parse::<FieldId>().ok().map(Self::Own))
    }

    /// The own field, if the key names one.
    #[must_use]
    pub const fn own(self) -> Option<FieldId> {
        match self {
            Self::Own(field) => Some(field),
            Self::BuiltIn(_) => None,
        }
    }

    /// The built-in field, if the key names one.
    #[must_use]
    pub const fn built_in(self) -> Option<Field> {
        match self {
            Self::BuiltIn(field) => Some(field),
            Self::Own(_) => None,
        }
    }
}

impl fmt::Display for FieldKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.code())
    }
}

impl FromStr for FieldKey {
    type Err = crate::words::UnknownWord;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::from_code(text).ok_or(crate::words::UnknownWord)
    }
}

impl Serialize for FieldKey {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for FieldKey {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = <Cow<'de, str>>::deserialize(deserializer)?;
        text.parse().map_err(serde::de::Error::custom)
    }
}

// A key is a word or a random reference, never personal data.
impl crate::personal::NotPersonal for FieldKey {}
impl crate::plain::sealed::Sealed for FieldKey {}
impl crate::plain::Plain for FieldKey {}

#[cfg(feature = "openapi")]
impl utoipa::PartialSchema for FieldKey {
    fn schema() -> utoipa::openapi::RefOr<utoipa::openapi::schema::Schema> {
        use utoipa::openapi::schema::{ObjectBuilder, Type};
        ObjectBuilder::new()
            .schema_type(Type::String)
            .description(Some(
                "The key of a field: the word of a built-in field, or the ID of an own field.",
            ))
            .into()
    }
}

#[cfg(feature = "openapi")]
impl utoipa::ToSchema for FieldKey {}

/// The thing a field belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BelongsTo {
    /// An issue.
    Issue,
    /// An issue, as the total of its time entries.
    IssueTimeTotal,
    /// A time entry (it has a default on the issue).
    TimeEntry,
    /// A time entry (it has a default on the issue and on the project).
    TimeEntryWithProjectDefault,
}

impl Field {
    /// What the field belongs to.
    #[must_use]
    pub const fn belongs_to(self) -> BelongsTo {
        match self {
            Self::TimeSpent => BelongsTo::IssueTimeTotal,
            Self::Activity => BelongsTo::TimeEntry,
            Self::CostCenter => BelongsTo::TimeEntryWithProjectDefault,
            _ => BelongsTo::Issue,
        }
    }

    /// The word of the field in the search language (ADR-0005): the key in lower camel case
    /// (`assigned-to` is `assignedTo`), so the word is made from the key by one rule and no
    /// second list of words can disagree with the catalogue.
    #[must_use]
    pub fn search_word(self) -> String {
        let mut word = String::new();
        let mut upper = false;
        for letter in self.code().chars() {
            if letter == '-' {
                upper = true;
            } else if upper {
                word.extend(letter.to_uppercase());
                upper = false;
            } else {
                word.push(letter);
            }
        }
        word
    }

    /// The kind of people information the field holds, when it holds a person: a person in the
    /// field is shown only when the field is visible, and then as this kind says.
    #[must_use]
    pub const fn person_kind(self) -> Option<PeopleKind> {
        match self {
            Self::AssignedTo => Some(PeopleKind::AssignedTo),
            Self::ReportedBy => Some(PeopleKind::ReportedBy),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn m2_q7_the_catalogue_has_the_sixteen_fields_in_order() {
        let keys: Vec<&str> = Field::ALL.iter().map(|field| field.code()).collect();
        assert_eq!(
            keys,
            [
                "title",
                "description",
                "status",
                "priority",
                "assigned-to",
                "reported-by",
                "due-date",
                "labels",
                "tags",
                "planned-for-release",
                "found-in-release",
                "estimated-time",
                "time-left",
                "time-spent",
                "activity",
                "cost-center",
            ]
        );
    }

    #[test]
    fn qry_05_the_search_word_of_a_field_is_its_key_in_lower_camel_case() {
        assert_eq!(Field::AssignedTo.search_word(), "assignedTo");
        assert_eq!(Field::PlannedForRelease.search_word(), "plannedForRelease");
        assert_eq!(Field::Title.search_word(), "title");
        let words: std::collections::HashSet<String> =
            Field::ALL.iter().map(|field| field.search_word()).collect();
        assert_eq!(words.len(), Field::ALL.len());
    }

    #[test]
    fn cf_06_a_key_is_a_word_or_an_id_and_reads_back() {
        let own = FieldId::generate(&mut crate::id::SequentialIds::default()).unwrap();
        for key in [FieldKey::from(Field::DueDate), FieldKey::from(own)] {
            assert_eq!(FieldKey::from_code(&key.code()), Some(key));
            let json = serde_json::to_string(&key).unwrap();
            assert_eq!(serde_json::from_str::<FieldKey>(&json).unwrap(), key);
        }
        assert_eq!(FieldKey::from_code("nonsense"), None);
        assert!(FieldKey::from(Field::Title) < FieldKey::from(own));
    }

    #[test]
    fn every_field_has_its_own_name_and_only_two_hold_a_person() {
        let mut seen = std::collections::HashSet::new();
        for field in Field::ALL {
            let key = field.name();
            assert_eq!(key.as_str(), format!("issue-field.{}", field.code()));
            assert!(seen.insert(key));
        }
        let with_person: Vec<Field> = Field::ALL
            .iter()
            .copied()
            .filter(|field| field.person_kind().is_some())
            .collect();
        assert_eq!(with_person, [Field::AssignedTo, Field::ReportedBy]);
        assert_eq!(Field::Activity.belongs_to(), BelongsTo::TimeEntry);
        assert_eq!(Field::TimeSpent.belongs_to(), BelongsTo::IssueTimeTotal);
    }
}
