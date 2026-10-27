//! The fields a person may search by: the **readable fields** that the caller passes to the checker
//! and to the suggestions. The crate knows nothing about permissions: a field the person may not
//! read is simply not among them, and is then as unknown as a field that does not exist (QRY-05,
//! ADR-0005 amendment M5, decision 5). Not to be mixed up with the field catalogue of `bb-core`
//! (the built-in fields that field permissions apply to).

use serde::Serialize;

use crate::syntax::Comparison;
use crate::words::Keyword;

/// The fields a person may read, each with its word.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct ReadableFields {
    /// The fields, in the order they are offered.
    pub fields: Vec<Field>,
}

/// One field that may stand in a search.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Field {
    /// The word of the field: one word in lower camel case (`assignedTo`), or the name of an own
    /// field made by [`own_field_word`].
    pub word: String,
    /// What kind of values it holds.
    pub kind: FieldKind,
    /// What may be asked of it.
    pub operators: Vec<Operator>,
    /// The values of a fixed list (a choice, a status, a priority); empty when anything goes.
    pub choices: Vec<String>,
}

/// What kind of values a field holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FieldKind {
    /// Words: a title, a text.
    Text,
    /// A number.
    Number,
    /// A length of time, written in hours and minutes.
    Duration,
    /// A day or a moment.
    Date,
    /// One of a list, or several (a status, a priority, a tag).
    Choice,
    /// A person.
    Person,
    /// A project.
    Project,
    /// An issue.
    Issue,
}

/// What may be asked of a field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Operator {
    /// `is`.
    Is,
    /// `is not`.
    IsNot,
    /// `<`.
    Less,
    /// `>`.
    Greater,
    /// `<=`.
    LessOrEqual,
    /// `>=`.
    GreaterOrEqual,
    /// `in (…)`.
    In,
    /// `contains`.
    Contains,
    /// `is empty`.
    IsEmpty,
    /// `is not empty`.
    IsNotEmpty,
    /// `in last N days`.
    InLast,
    /// `member of`.
    MemberOf,
    /// `within` a project.
    Within,
}

every!(
    Operator: Is,
    IsNot,
    Less,
    Greater,
    LessOrEqual,
    GreaterOrEqual,
    In,
    Contains,
    IsEmpty,
    IsNotEmpty,
    InLast,
    MemberOf,
    Within
);
every!(FieldKind: Text, Number, Duration, Date, Choice, Person, Project, Issue);

impl Operator {
    /// How the operator is written, the way a person is offered it.
    #[must_use]
    pub fn text(self) -> String {
        use Keyword::{Contains, Empty, In, Is, Last, Member, Not, Of, Within};
        if let Some(comparison) = self.comparison() {
            return comparison.sign().to_owned();
        }
        let words: &[Keyword] = match self {
            Self::Is => &[Is],
            Self::IsNot => &[Is, Not],
            Self::In => &[In],
            Self::Contains => &[Contains],
            Self::IsEmpty => &[Is, Empty],
            Self::IsNotEmpty => &[Is, Not, Empty],
            Self::InLast => &[In, Last],
            Self::MemberOf => &[Member, Of],
            Self::Within => &[Within],
            Self::Less | Self::Greater | Self::LessOrEqual | Self::GreaterOrEqual => &[],
        };
        words
            .iter()
            .map(|word| word.text())
            .collect::<Vec<_>>()
            .join(" ")
    }
}

impl Operator {
    /// The comparison this operator is, if it is one: found among the comparisons by the one
    /// conversion `From<Comparison>`, so no second mapping can disagree with it.
    #[must_use]
    pub fn comparison(self) -> Option<Comparison> {
        Comparison::ALL
            .into_iter()
            .find(|comparison| Self::from(*comparison) == self)
    }
}

impl From<Comparison> for Operator {
    fn from(comparison: Comparison) -> Self {
        match comparison {
            Comparison::Less => Self::Less,
            Comparison::Greater => Self::Greater,
            Comparison::LessOrEqual => Self::LessOrEqual,
            Comparison::GreaterOrEqual => Self::GreaterOrEqual,
        }
    }
}

impl FieldKind {
    /// What may be asked of a field of this kind, unless the field says otherwise.
    #[must_use]
    pub const fn operators(self) -> &'static [Operator] {
        use Operator::{
            Contains, Greater, GreaterOrEqual, In, InLast, Is, IsEmpty, IsNot, IsNotEmpty, Less,
            LessOrEqual, MemberOf, Within,
        };
        match self {
            Self::Text => &[Is, IsNot, Contains, IsEmpty, IsNotEmpty],
            Self::Number | Self::Duration => &[
                Is,
                IsNot,
                Less,
                Greater,
                LessOrEqual,
                GreaterOrEqual,
                IsEmpty,
                IsNotEmpty,
            ],
            Self::Date => &[
                Is,
                IsNot,
                Less,
                Greater,
                LessOrEqual,
                GreaterOrEqual,
                InLast,
                IsEmpty,
                IsNotEmpty,
            ],
            Self::Choice => &[
                Is,
                IsNot,
                In,
                Less,
                Greater,
                LessOrEqual,
                GreaterOrEqual,
                IsEmpty,
                IsNotEmpty,
            ],
            Self::Person => &[Is, IsNot, In, MemberOf, IsEmpty, IsNotEmpty],
            Self::Project => &[Is, IsNot, In, Within, IsEmpty, IsNotEmpty],
            Self::Issue => &[Is, IsNot, In, IsEmpty, IsNotEmpty],
        }
    }
}

impl Field {
    /// A field with the operators its kind allows and no fixed list of values.
    #[must_use]
    pub fn new(word: &str, kind: FieldKind) -> Self {
        Self {
            word: word.to_owned(),
            kind,
            operators: kind.operators().to_vec(),
            choices: Vec::new(),
        }
    }

    /// The field, with the values of its fixed list.
    #[must_use]
    pub fn with_choices(mut self, choices: &[&str]) -> Self {
        self.choices = choices.iter().map(|choice| (*choice).to_owned()).collect();
        self
    }
}

impl ReadableFields {
    /// The field that has this word. The word is compared as written.
    #[must_use]
    pub fn field(&self, word: &str) -> Option<&Field> {
        self.fields.iter().find(|field| field.word == word)
    }

    /// Whether a `contains` on its own is a search for this person: it looks in the title and in
    /// the description among other texts, so both have to be readable. The one rule, read by the
    /// checker and by the suggestions.
    #[must_use]
    pub fn allows_bare_contains(&self) -> bool {
        [
            bb_core::fields::Field::Title,
            bb_core::fields::Field::Description,
        ]
        .into_iter()
        .all(|field| self.field(&field.search_word()).is_some())
    }
}

/// The word of an own field, made from its name by one rule: a name of words and numbers becomes
/// one word in lower camel case (`Story points` is `storyPoints`); any other name stays as it is
/// and is written in quotes in a search.
#[must_use]
pub fn own_field_word(name: &str) -> String {
    let parts: Vec<&str> = name.split_whitespace().collect();
    let one_word = !parts.is_empty()
        && parts
            .iter()
            .all(|part| part.chars().all(char::is_alphanumeric));
    let camel = bb_core::fields::lower_camel(parts);
    if one_word && camel.chars().next().is_some_and(char::is_alphabetic) {
        camel
    } else {
        name.trim().to_owned()
    }
}

/// The word of *yes* in a field that holds yes or no.
pub const YES: &str = "yes";

/// The word of *no* in a field that holds yes or no.
pub const NO: &str = "no";

/// A field every issue has, which no role can hide and which is not in the field catalogue of
/// `bb-core` (seeing the issue is what lets a person read it). Its word is defined here, once, and
/// reserved for it for everyone: an own field never takes it (ADR-0005 amendment M5, decision 9).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IssueField {
    /// The project the issue is in.
    Project,
    /// The issue type, by its name.
    Type,
    /// The group of the status.
    StatusGroup,
    /// The issue it sits under.
    Parent,
    /// When it was made.
    Created,
    /// When it last changed.
    Changed,
    /// When it was closed.
    Closed,
}

every!(
    IssueField: Project,
    Type,
    StatusGroup,
    Parent,
    Created,
    Changed,
    Closed
);

impl IssueField {
    /// The word of the field.
    #[must_use]
    pub const fn word(self) -> &'static str {
        match self {
            Self::Project => "project",
            Self::Type => "type",
            Self::StatusGroup => "statusGroup",
            Self::Parent => "parent",
            Self::Created => "created",
            Self::Changed => "changed",
            Self::Closed => "closed",
        }
    }

    /// What kind of values it holds.
    #[must_use]
    pub const fn kind(self) -> FieldKind {
        match self {
            Self::Project => FieldKind::Project,
            Self::Type | Self::StatusGroup => FieldKind::Choice,
            Self::Parent => FieldKind::Issue,
            Self::Created | Self::Changed | Self::Closed => FieldKind::Date,
        }
    }

    /// The field of the search language, with the values of its fixed list.
    #[must_use]
    pub fn field(self, choices: Vec<String>) -> Field {
        let field = match self {
            // The types are not ordered: *less than Bug* means nothing.
            Self::Type => Field::unordered(self.word()),
            _ => Field::new(self.word(), self.kind()),
        };
        Field { choices, ..field }
    }
}

impl Field {
    /// A field with a list of values that have no order (a status, an issue type, a tag, a group):
    /// it can be asked whether it is, is not or is one of them, and whether it is empty.
    #[must_use]
    pub fn unordered(word: &str) -> Self {
        Self {
            operators: vec![
                Operator::Is,
                Operator::IsNot,
                Operator::In,
                Operator::IsEmpty,
                Operator::IsNotEmpty,
            ],
            ..Self::new(word, FieldKind::Choice)
        }
    }

    /// A field that holds yes or no ([`YES`], [`NO`]).
    #[must_use]
    pub fn yes_no(word: &str) -> Self {
        Self {
            operators: vec![
                Operator::Is,
                Operator::IsNot,
                Operator::IsEmpty,
                Operator::IsNotEmpty,
            ],
            choices: vec![YES.to_owned(), NO.to_owned()],
            ..Self::new(word, FieldKind::Choice)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_own_field_gets_a_word_from_its_name_by_one_rule() {
        assert_eq!(own_field_word("Story points"), "storyPoints");
        assert_eq!(own_field_word("  customer   ID "), "customerID");
        assert_eq!(own_field_word("Größe"), "größe");
        assert_eq!(own_field_word("Due-date (2)"), "Due-date (2)");
        assert_eq!(own_field_word("3D model"), "3D model");
        assert_eq!(own_field_word(""), "");
    }

    #[test]
    fn the_words_of_the_built_in_fields_are_all_different() {
        let mut words: Vec<String> = bb_core::fields::Field::ALL
            .iter()
            .map(|field| field.search_word())
            .chain(IssueField::ALL.iter().map(|each| each.word().to_owned()))
            .collect();
        let all = words.len();
        words.sort();
        words.dedup();
        assert_eq!(words.len(), all);
    }

    #[test]
    fn a_list_without_order_allows_no_comparison() {
        let field = Field::unordered("type");
        assert!(!field.operators.contains(&Operator::Less));
        assert!(field.operators.contains(&Operator::In));
        let yes_no = Field::yes_no("urgent");
        assert_eq!(yes_no.choices, [YES, NO]);
        assert!(!yes_no.operators.contains(&Operator::In));
    }

    #[test]
    fn operators_are_written_in_plain_words() {
        assert_eq!(Operator::IsNotEmpty.text(), "is not empty");
        assert_eq!(Operator::MemberOf.text(), "member of");
        assert_eq!(Operator::LessOrEqual.text(), "<=");
    }

    #[test]
    fn a_comparison_and_its_operator_are_each_other() {
        for comparison in Comparison::ALL {
            assert_eq!(Operator::from(comparison).comparison(), Some(comparison));
            assert_eq!(Operator::from(comparison).text(), comparison.sign());
        }
        let others = Operator::ALL
            .iter()
            .filter(|operator| operator.comparison().is_none())
            .count();
        assert_eq!(others, Operator::ALL.len() - Comparison::ALL.len());
    }

    #[test]
    fn a_new_field_has_the_operators_of_its_kind() {
        let field = Field::new("assignedTo", FieldKind::Person).with_choices(&[]);
        assert!(field.operators.contains(&Operator::MemberOf));
        assert!(!field.operators.contains(&Operator::Contains));
    }
}
