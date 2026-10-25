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
every!(FieldKind: Text, Number, Date, Choice, Person, Project, Issue);

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
            Self::Number => &[
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
    let camel: String = parts
        .iter()
        .enumerate()
        .flat_map(|(at, part)| {
            let mut letters = part.chars();
            let first = letters.next().into_iter().flat_map(move |letter| {
                if at == 0 {
                    letter.to_lowercase().collect::<Vec<_>>()
                } else {
                    letter.to_uppercase().collect()
                }
            });
            first.chain(letters).collect::<Vec<_>>()
        })
        .collect();
    if one_word && camel.chars().next().is_some_and(char::is_alphabetic) {
        camel
    } else {
        name.trim().to_owned()
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
