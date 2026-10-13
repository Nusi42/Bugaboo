//! Own fields (CF-01, CF-03, CF-04, CF-06, CF-07, CF-08, ADR-0006): the fields an organization
//! adds to issues and to projects. This file holds what a field *is*: its type, where it is used,
//! its rules, its choices and its look. The values people fill in are in `own_fields_value.rs`.
//!
//! A field is a row, so it is named by its ID ([`crate::fields::FieldKey::Own`]); the built-in
//! catalogue (`fields.rs`) is not touched by it. Everything people can choose here is a word
//! declared once: the types, the looks, the colors of a choice.

use jiff::Timestamp;

use crate::fields::FieldKey;
use crate::id::{ChoiceId, FieldId, IssueTypeId, ProjectId};
use crate::issue::IssueIcon;
use crate::personal::Entity;
use crate::technical::{InvalidTechnicalName, TechnicalName};

/// The most own fields an organization can make. A field costs a column in every list of issues,
/// so the number is bounded.
pub const FIELDS_MAX: usize = 200;

/// The most choices one field offers.
pub const CHOICES_MAX: usize = 200;

/// The most values a *Pick several*, *People* field holds on one item.
pub const VALUES_MAX: usize = 50;

/// The most projects the page of a field names under *Where it is used*; the answer holds the
/// number of all of them and says when the list was cut.
pub const USAGE_PROJECTS_MAX: usize = 100;

/// What a change does with a value that can also be taken away: leave it, take it away or set it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum Edit<T> {
    /// The request says nothing about it: it stays as it is.
    #[default]
    Keep,
    /// The request says `null`: the value is taken away.
    Clear,
    /// The request gives a new value.
    Set(T),
}

impl<T> Edit<T> {
    /// Whether the request says nothing about the value.
    #[must_use]
    pub fn is_keep(&self) -> bool {
        matches!(self, Self::Keep)
    }

    /// What a request that does say something means: `null` takes the value away.
    #[must_use]
    pub fn said(value: Option<T>) -> Self {
        value.map_or(Self::Clear, Self::Set)
    }
}

/// The longest pattern a rule can have, in characters. Longer patterns are refused when the rule
/// is made (a pattern is checked with a linear-time engine, and this cap keeps it small).
pub const PATTERN_MAX: usize = 200;

/// The longest help text of a field, in characters.
pub const HELP_MAX: usize = 300;

/// The longest value of a *Text* or *Web address* field, in characters.
pub const SHORT_TEXT_MAX: usize = 1_000;

/// How many parts a decimal number has after the point: a decimal is stored as a whole number of
/// millionths, so it sorts and adds up exactly on both databases.
pub const DECIMAL_PARTS: i64 = 1_000_000;

/// The name of the part of an issue that a long text field is: what an edit lock and a draft of
/// the text are about (`field-<ID>`).
///
/// # Errors
/// Never in practice: the ID is 36 characters of hex digits and hyphens, so the name is a valid
/// technical name.
pub fn edit_part(field: FieldId) -> Result<TechnicalName, InvalidTechnicalName> {
    TechnicalName::new(&format!("{EDIT_PART_PREFIX}{field}"))
}

/// What the name of the edit part of a long-text field starts with, before the ID of the field.
pub const EDIT_PART_PREFIX: &str = "field-";

words! {
    /// What kind of information a field holds (CF-01). The word is stored. *Release* and *Pick
    /// one, then one more* come with their features (ISS-08, CF-02).
    pub enum FieldType {
        /// A short text.
        Text => "text",
        /// A long text in Markdown.
        LongText => "long-text",
        /// A whole number.
        WholeNumber => "whole-number",
        /// A number with a decimal part.
        Decimal => "decimal",
        /// Yes or no.
        YesNo => "yes-no",
        /// A web address.
        WebAddress => "web-address",
        /// A day.
        Date => "date",
        /// A day and a time.
        DateTime => "date-time",
        /// A length of time.
        Duration => "duration",
        /// One choice of a list.
        PickOne => "pick-one",
        /// Several choices of a list.
        PickSeveral => "pick-several",
        /// One person.
        Person => "person",
        /// Several people.
        People => "people",
        /// One group.
        Group => "group",
        /// One issue.
        Issue => "issue",
    }
    messages name = "field-type.";
    messages example = "field-example."
}

words_in_api!(FieldType, "What kind of information an own field holds.");

/// How a value is stored: the column of `field_value` it goes into (ADR-0006).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Column {
    /// `value_text`.
    Text,
    /// `value_number`.
    Number,
    /// `value_date`.
    Date,
    /// `value_ref`.
    Reference,
}

impl FieldType {
    /// The column the values of the type are stored in.
    #[must_use]
    pub const fn column(self) -> Column {
        match self {
            Self::Text | Self::LongText | Self::WebAddress => Column::Text,
            Self::WholeNumber | Self::Decimal | Self::YesNo | Self::Duration => Column::Number,
            Self::Date | Self::DateTime => Column::Date,
            Self::PickOne
            | Self::PickSeveral
            | Self::Person
            | Self::People
            | Self::Group
            | Self::Issue => Column::Reference,
        }
    }

    /// Whether an item holds several values of the field (several rows in `field_value`).
    #[must_use]
    pub const fn is_many(self) -> bool {
        matches!(self, Self::PickSeveral | Self::People)
    }

    /// Whether the field offers choices that are made and ordered on the field page.
    #[must_use]
    pub const fn has_choices(self) -> bool {
        matches!(self, Self::PickOne | Self::PickSeveral)
    }

    /// Whether the value is a text that length and pattern rules apply to.
    #[must_use]
    pub const fn is_text(self) -> bool {
        matches!(self, Self::Text | Self::WebAddress | Self::LongText)
    }

    /// Whether the value is a number, a day, a moment or a length of time that a range applies
    /// to.
    #[must_use]
    pub const fn has_range(self) -> bool {
        matches!(
            self,
            Self::WholeNumber | Self::Decimal | Self::Date | Self::DateTime | Self::Duration
        )
    }

    /// The looks that can show a value of this type.
    #[must_use]
    pub fn looks(self) -> Vec<Look> {
        Look::ALL
            .iter()
            .copied()
            .filter(|look| look.fits(self))
            .collect()
    }

    /// Whether the field holds a long text, which is edited under an edit lock (ADR-0019).
    #[must_use]
    pub const fn is_long_text(self) -> bool {
        matches!(self, Self::LongText)
    }

    /// Whether the value points to a person, so it is shown as *Who sees people* says.
    #[must_use]
    pub const fn holds_people(self) -> bool {
        matches!(self, Self::Person | Self::People)
    }

    /// Whether the value is a day or a moment.
    #[must_use]
    pub const fn is_dated(self) -> bool {
        matches!(self, Self::Date | Self::DateTime)
    }

    /// Whether a field of this kind can start with a value: not one that points to people, groups
    /// or issues (what a new item points to is for the person to choose, and checked for them),
    /// and not a long text (saved only under an edit lock).
    #[must_use]
    pub const fn takes_default(self) -> bool {
        !matches!(
            self,
            Self::Person | Self::People | Self::Group | Self::Issue | Self::LongText
        )
    }
}

words! {
    /// What a screen needs to know about a kind of information (CF-01), so it never writes the
    /// kinds down a second time.
    pub enum KindTrait {
        /// It offers choices that are made on the field page.
        HasChoices => "has-choices",
        /// An item holds several values of it.
        Many => "many",
        /// It is a text that length and pattern rules apply to.
        Text => "text",
        /// A range applies to it.
        Range => "range",
        /// It is a long text, edited under an edit lock.
        LongText => "long-text",
        /// It points to people, who are shown as *Who sees people* says.
        HoldsPeople => "holds-people",
        /// A field of this kind can start with a value.
        TakesDefault => "takes-default",
        /// The value is a day or a moment.
        Dated => "dated",
    }
}

words_in_api!(KindTrait, "What is true of a kind of information.");

impl FieldType {
    /// What is true of the kind.
    #[must_use]
    pub fn traits(self) -> Vec<KindTrait> {
        [
            (self.has_choices(), KindTrait::HasChoices),
            (self.is_many(), KindTrait::Many),
            (self.is_text(), KindTrait::Text),
            (self.has_range(), KindTrait::Range),
            (self.is_long_text(), KindTrait::LongText),
            (self.holds_people(), KindTrait::HoldsPeople),
            (self.takes_default(), KindTrait::TakesDefault),
            (self.is_dated(), KindTrait::Dated),
        ]
        .into_iter()
        .filter_map(|(true_of_it, word)| true_of_it.then_some(word))
        .collect()
    }
}

words! {
    /// What a field is added to (CF-08). Time entries, working days, documents and releases use
    /// the same machinery later.
    pub enum FieldFor {
        /// Issues.
        Issues => "issues",
        /// Projects.
        Projects => "projects",
    }
    messages name = "field-for."
}

words_in_api!(FieldFor, "What an own field is added to.");

words! {
    /// How a value is shown (CF-04): the *Look* of a field.
    pub enum Look {
        /// A small colored label with the choice's icon.
        Badge => "badge",
        /// A rounded colored label.
        Pill => "pill",
        /// Plain text.
        PlainText => "plain-text",
        /// A bar that fills between the lowest and the highest value.
        ProgressBar => "progress-bar",
        /// The picture or initials of a person.
        Avatar => "avatar",
    }
    messages name = "field-look."
}

words_in_api!(Look, "How the value of an own field is shown.");

impl Look {
    /// Whether the look can show values of the type. Every type can be shown as plain text.
    #[must_use]
    pub const fn fits(self, kind: FieldType) -> bool {
        match self {
            Self::PlainText => true,
            Self::Badge | Self::Pill => matches!(
                kind,
                FieldType::PickOne | FieldType::PickSeveral | FieldType::YesNo | FieldType::Group
            ),
            Self::ProgressBar => matches!(kind, FieldType::WholeNumber | FieldType::Decimal),
            Self::Avatar => kind.holds_people(),
        }
    }

    /// The look a new field of the type starts with.
    #[must_use]
    pub const fn starting(kind: FieldType) -> Self {
        match kind {
            FieldType::PickOne | FieldType::PickSeveral | FieldType::YesNo => Self::Badge,
            FieldType::Person | FieldType::People => Self::Avatar,
            _ => Self::PlainText,
        }
    }
}

words! {
    /// The color of a choice: one of the twelve category colors of the theme (UX §7.1). The word
    /// is the name of the pair of tokens in `design/theme.kdl` (`cat3-bg`, `cat3-fg`), so a color
    /// that can be stored is a color every theme has.
    pub enum ChoiceColor {
        /// Category color 1.
        Cat1 => "cat1",
        /// Category color 2.
        Cat2 => "cat2",
        /// Category color 3.
        Cat3 => "cat3",
        /// Category color 4.
        Cat4 => "cat4",
        /// Category color 5.
        Cat5 => "cat5",
        /// Category color 6.
        Cat6 => "cat6",
        /// Category color 7.
        Cat7 => "cat7",
        /// Category color 8.
        Cat8 => "cat8",
        /// Category color 9.
        Cat9 => "cat9",
        /// Category color 10.
        Cat10 => "cat10",
        /// Category color 11.
        Cat11 => "cat11",
        /// Category color 12.
        Cat12 => "cat12",
    }
    messages name = "choice-color."
}

words_in_api!(ChoiceColor, "The color of a choice, as a category token.");

impl ChoiceColor {
    /// The color a new choice gets: the colors in turn, so neighbours differ.
    #[must_use]
    pub fn nth(index: usize) -> Self {
        Self::ALL[index % Self::ALL.len()]
    }
}

/// The rules a value has to keep (CF-03). Which rules a type can have is decided by
/// [`FieldType`]; [`Rules::check_definition`] refuses a rule the type cannot have.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct Rules {
    /// The value has to be there.
    #[serde(default)]
    pub required: bool,
    /// The lowest number, day, moment or length of time, in the unit the value is stored in
    /// (millionths for a decimal, days for a date, microseconds for a moment, seconds for a
    /// duration).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min: Option<i64>,
    /// The highest one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max: Option<i64>,
    /// The fewest characters of a text, or values of a *Pick several* and *People* field.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_length: Option<u32>,
    /// The most.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_length: Option<u32>,
    /// A pattern the whole text has to match (a regular expression without look-around).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pattern: Option<String>,
}

/// Why the rules of a field cannot be.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RulesRefused {
    /// A rule the type cannot have (a pattern on a number).
    NotForThisType,
    /// The lowest is above the highest.
    Backwards,
    /// The pattern is longer than [`PATTERN_MAX`].
    PatternTooLong,
}

impl Rules {
    /// Whether the rules say nothing but "not required".
    #[must_use]
    pub const fn is_plain(&self) -> bool {
        self.min.is_none()
            && self.max.is_none()
            && self.min_length.is_none()
            && self.max_length.is_none()
            && self.pattern.is_none()
    }

    /// Checks that the rules make sense for a field of this type.
    ///
    /// # Errors
    /// Says what is wrong.
    pub fn check_definition(&self, kind: FieldType) -> Result<(), RulesRefused> {
        let range = self.min.is_some() || self.max.is_some();
        let length = self.min_length.is_some() || self.max_length.is_some();
        let counts = kind.is_text() || kind.is_many();
        if (range && !kind.has_range())
            || (length && !counts)
            || (self.pattern.is_some() && !kind.is_text())
        {
            return Err(RulesRefused::NotForThisType);
        }
        if matches!((self.min, self.max), (Some(low), Some(high)) if low > high)
            || matches!((self.min_length, self.max_length), (Some(low), Some(high)) if low > high)
        {
            return Err(RulesRefused::Backwards);
        }
        if self
            .pattern
            .as_ref()
            .is_some_and(|pattern| pattern.chars().count() > PATTERN_MAX)
        {
            return Err(RulesRefused::PatternTooLong);
        }
        Ok(())
    }
}

// The rules hold numbers and a pattern the administrator chose, never personal data.
impl crate::personal::NotPersonal for Rules {}

/// Where a field is used (CF-03): the whole organization, some projects (each with its
/// sub-projects), and all issue types or some. An empty list means "all".
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Scope {
    /// The projects the field is used in, with their sub-projects; empty for the whole
    /// organization.
    pub projects: Vec<ProjectId>,
    /// The issue types the field is used for; empty for all. Only fields for issues have any.
    pub issue_types: Vec<IssueTypeId>,
}

impl Scope {
    /// Whether the field is offered on an item in a project whose projects from the top down are
    /// `chain`, and (for an issue) of the issue type `issue_type`.
    #[must_use]
    pub fn reaches(&self, chain: &[ProjectId], issue_type: Option<IssueTypeId>) -> bool {
        let project_ok =
            self.projects.is_empty() || self.projects.iter().any(|project| chain.contains(project));
        project_ok && self.reaches_type(issue_type)
    }

    /// Whether the field is offered for issues of the issue type `issue_type` in some project: the
    /// half of [`Self::reaches`] that is about the type, for the whole organization, which has no
    /// project.
    #[must_use]
    pub fn reaches_type(&self, issue_type: Option<IssueTypeId>) -> bool {
        self.issue_types.is_empty()
            || issue_type.is_some_and(|wanted| self.issue_types.contains(&wanted))
    }
}

/// An own field (CF-01): what the organization added, with its rules and look.
#[derive(Clone, Debug, PartialEq, Eq, Entity)]
pub struct OwnField {
    /// The ID of the field.
    #[not_personal]
    pub id: FieldId,
    /// What it is called. Free text that could name a person.
    #[personal(category = "content", basis = "legitimate_interests")]
    pub name: String,
    /// Words that help people fill it in. Free text that could name a person.
    #[personal(category = "content", basis = "legitimate_interests")]
    pub help: String,
    /// What kind of information it holds.
    #[not_personal]
    pub kind: FieldType,
    /// What it is added to.
    #[not_personal]
    pub applies_to: FieldFor,
    /// The rules.
    #[not_personal]
    pub rules: Rules,
    /// The value a new item starts with, as JSON; empty for none. Free text that could name a
    /// person.
    #[personal(category = "content", basis = "legitimate_interests")]
    pub default_value: String,
    /// How the value is shown.
    #[not_personal]
    pub look: Look,
    /// Whether the assistant must never be sent the value (UX §5.3, *Assistant*).
    #[not_personal]
    pub never_to_assistant: bool,
    /// The place in the order the fields are offered in.
    #[not_personal]
    pub position: i64,
    /// When it was retired: no new values, while the old ones stay.
    #[not_personal]
    pub retired_at: Option<Timestamp>,
    /// Counts up with every change, for the check that nobody changed it meanwhile (ADR-0019).
    #[not_personal]
    pub version: i64,
}

/// A project a field is limited to.
#[derive(Clone, Debug, PartialEq, Eq, Entity)]
pub struct FieldProject {
    /// The field.
    #[not_personal]
    pub field: FieldId,
    /// The project, with its sub-projects.
    #[not_personal]
    pub project: ProjectId,
}

/// An issue type a field is limited to.
#[derive(Clone, Debug, PartialEq, Eq, Entity)]
pub struct FieldIssueType {
    /// The field.
    #[not_personal]
    pub field: FieldId,
    /// The issue type.
    #[not_personal]
    pub issue_type: IssueTypeId,
}

/// One choice of a *Pick one* or *Pick several* field (CF-04).
#[derive(Clone, Debug, PartialEq, Eq, Entity)]
pub struct FieldChoice {
    /// The ID of the choice.
    #[not_personal]
    pub id: ChoiceId,
    /// The field it belongs to.
    #[not_personal]
    pub field: FieldId,
    /// What it is called. Free text that could name a person.
    #[personal(category = "content", basis = "legitimate_interests")]
    pub name: String,
    /// Its color.
    #[not_personal]
    pub color: ChoiceColor,
    /// Its icon, if it has one.
    #[not_personal]
    pub icon: Option<IssueIcon>,
    /// The place in the order of the choices.
    #[not_personal]
    pub position: i64,
    /// When it was retired: not offered for new values, kept on old ones.
    #[not_personal]
    pub retired_at: Option<Timestamp>,
}

impl OwnField {
    /// The key that names the field in levels, masks and history.
    #[must_use]
    pub const fn key(&self) -> FieldKey {
        FieldKey::Own(self.id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cf_01_every_type_has_a_column_and_the_looks_fit_somewhere() {
        for kind in FieldType::ALL {
            // The starting look always fits its type.
            assert!(Look::starting(*kind).fits(*kind), "{kind:?}");
            assert!(Look::PlainText.fits(*kind));
        }
        assert_eq!(FieldType::ALL.len(), 15);
        assert_eq!(FieldType::People.column(), Column::Reference);
        assert!(FieldType::PickSeveral.is_many() && FieldType::People.is_many());
        assert!(!FieldType::PickOne.is_many());
    }

    #[test]
    fn cf_04_there_are_twelve_category_colors_and_they_repeat_in_turn() {
        assert_eq!(ChoiceColor::ALL.len(), 12);
        assert_eq!(ChoiceColor::nth(0), ChoiceColor::Cat1);
        assert_eq!(ChoiceColor::nth(12), ChoiceColor::Cat1);
    }

    #[test]
    fn cf_03_rules_that_the_type_cannot_have_are_refused() {
        let pattern = Rules {
            pattern: Some("a+".into()),
            ..Rules::default()
        };
        assert_eq!(pattern.check_definition(FieldType::Text), Ok(()));
        assert_eq!(
            pattern.check_definition(FieldType::WholeNumber),
            Err(RulesRefused::NotForThisType)
        );
        let backwards = Rules {
            min: Some(5),
            max: Some(1),
            ..Rules::default()
        };
        assert_eq!(
            backwards.check_definition(FieldType::WholeNumber),
            Err(RulesRefused::Backwards)
        );
        let long = Rules {
            pattern: Some("a".repeat(PATTERN_MAX + 1)),
            ..Rules::default()
        };
        assert_eq!(
            long.check_definition(FieldType::Text),
            Err(RulesRefused::PatternTooLong)
        );
        let count = Rules {
            max_length: Some(3),
            ..Rules::default()
        };
        assert_eq!(count.check_definition(FieldType::PickSeveral), Ok(()));
    }

    #[test]
    fn cf_03_a_scope_reaches_a_project_and_its_sub_projects_and_some_types() {
        use crate::id::SequentialIds;
        let mut source = SequentialIds::default();
        let parent = ProjectId::generate(&mut source).unwrap();
        let child = ProjectId::generate(&mut source).unwrap();
        let other = ProjectId::generate(&mut source).unwrap();
        let bug = IssueTypeId::generate(&mut source).unwrap();
        let task = IssueTypeId::generate(&mut source).unwrap();
        let everywhere = Scope::default();
        assert!(everywhere.reaches(&[other], Some(bug)));
        let scope = Scope {
            projects: vec![parent],
            issue_types: vec![bug],
        };
        assert!(scope.reaches(&[parent, child], Some(bug)));
        assert!(!scope.reaches(&[other], Some(bug)));
        assert!(!scope.reaches(&[parent], Some(task)));
        assert!(!scope.reaches(&[parent], None));
        // The whole organization has no project: only the type counts.
        assert!(scope.reaches_type(Some(bug)));
        assert!(!scope.reaches_type(Some(task)));
        assert!(!scope.reaches_type(None));
        assert!(everywhere.reaches_type(None));
    }
}
