//! Saved searches (QRY-06, CFG-02, ADR-0034): a name, the canonical search text, how the results
//! are shown, the person who made it, and whom it is shared with.
//!
//! A saved search is private when it is made. The text is the one canonical text of the search
//! language (ADR-0005, amendment M5): the tree never crosses the wire and is never stored. How the
//! results are shown (the *presentation*) is not part of the language: the kind of view, the
//! column and the grouping name a field by its search word, so that a screen needs no second list.

use jiff::Timestamp;

use crate::id::{GroupId, ProjectId, SavedSearchId, UserId};
use crate::personal::{Entity, NotPersonal};

/// The most columns one saved search shows.
pub const COLUMNS_MAX: usize = 40;

/// The longest search word of a column or of the grouping, in characters: a name of an own field
/// is the longest word there is (`own_field_word` leaves a name that is not one word as it is).
pub const WORD_MAX: usize = crate::text::NAME_MAX;

/// The narrowest and the widest a column can be, in the units of the list (pixels at the starting
/// size of text). A column never disappears by its width, and no width is wider than a screen.
pub const COLUMN_WIDTH_RANGE: std::ops::RangeInclusive<u32> = 24..=2000;

/// The most groups and projects one saved search is shared with.
pub const SHARES_MAX: usize = 100;

words! {
    /// The kind of view a saved search is shown as. M5 shows the kind *list*; board, timeline and
    /// calendar read the same record when their milestones come (ADR-0034, decision 1).
    pub enum ViewKind {
        /// A list of issues, one row each.
        List => "list",
    }
}

words_in_api!(
    ViewKind,
    "The kind of view a saved search is shown as. Other kinds come with their views."
);

words! {
    /// How close together the rows are: *Comfortable* or *Compact*. The same two words as the
    /// setting *Spacing* in *My settings* (`appearance.spacing`), which reads them from here.
    pub enum Spacing {
        /// More room around each row.
        Comfortable => "comfortable",
        /// Closer rows.
        Compact => "compact",
    }
    messages name = "appearance.spacing.";
}

words_in_api!(Spacing, "How close together the rows of a list are.");

/// One column of a list: the field by its search word and, when the person dragged it, its width.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Column {
    /// The search word of the field (`assignedTo`).
    pub field: String,
    /// How wide the column is; `None` for the width the list chooses.
    pub width: Option<u32>,
}

/// How a saved search is shown (*presentation*, ADR-0034, decision 1). No columns at all means the
/// columns of the project's list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Presentation {
    /// The kind of view.
    pub view: ViewKind,
    /// The columns, in order.
    pub columns: Vec<Column>,
    /// How close together the rows are.
    pub spacing: Spacing,
    /// The field the rows are grouped by, by its search word.
    pub group_by: Option<String>,
}

impl Default for Presentation {
    /// A list with the columns of the project's list, *Comfortable*, not grouped.
    fn default() -> Self {
        Self {
            view: ViewKind::List,
            columns: Vec::new(),
            spacing: Spacing::Comfortable,
            group_by: None,
        }
    }
}

/// Whether `word` can be the search word of a field: a text as it is kept (trimmed, not empty, at
/// most [`WORD_MAX`] characters, no control character). Whether a field has the word is the check
/// of the use case.
fn is_word(word: &str) -> bool {
    crate::text::clean(word, WORD_MAX) == Some(word)
}

impl Presentation {
    /// Whether the presentation can be kept: at most [`COLUMNS_MAX`] columns, each a field once
    /// with a width in range, and a grouping that is a word. Whether the fields exist and may be
    /// read is the check of the use case (the readable fields of the person).
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.columns.len() <= COLUMNS_MAX
            && self.columns.iter().enumerate().all(|(at, column)| {
                is_word(&column.field)
                    && column
                        .width
                        .is_none_or(|width| COLUMN_WIDTH_RANGE.contains(&width))
                    && !self.columns[..at]
                        .iter()
                        .any(|earlier| earlier.field == column.field)
            })
            && self.group_by.as_deref().is_none_or(is_word)
    }

    /// The search words of every field the presentation names: the columns and the grouping.
    pub fn fields(&self) -> impl Iterator<Item = &str> {
        self.columns
            .iter()
            .map(|column| column.field.as_str())
            .chain(self.group_by.as_deref())
    }
}

/// A saved search (QRY-06).
#[derive(Clone, Debug, PartialEq, Eq, Entity)]
pub struct SavedSearch {
    /// The ID of the search, in its address `/s/<id>`.
    #[not_personal]
    pub id: SavedSearchId,
    /// What the search is called. Free text that could name a person.
    #[personal(category = "content", basis = "legitimate_interests")]
    pub name: String,
    /// The canonical search text. A person wrote it, so it can name a person, a group or a
    /// project.
    #[personal(category = "content", basis = "legitimate_interests")]
    pub text: String,
    /// The person who made it. After *Remove a person* the ID stays as the link to *Former
    /// user …*; a private search goes with the person.
    #[personal(category = "activity", basis = "legitimate_interests")]
    pub maker: UserId,
    /// The kind of view.
    #[not_personal]
    pub view: ViewKind,
    /// How close together the rows are.
    #[not_personal]
    pub spacing: Spacing,
    /// The field the rows are grouped by, by its search word.
    #[personal(category = "content", basis = "legitimate_interests")]
    pub group_by: Option<String>,
    /// When it was made.
    #[not_personal]
    pub created_at: Timestamp,
    /// When it was last changed.
    #[not_personal]
    pub updated_at: Timestamp,
    /// When it was deleted into the trash, while it is there.
    #[not_personal]
    pub deleted_at: Option<Timestamp>,
    /// Counts up with every change, for the check that nobody changed it meanwhile (ADR-0019).
    #[not_personal]
    pub version: i64,
}

/// One column of a saved search, in order.
#[derive(Clone, Debug, PartialEq, Eq, Entity)]
pub struct SearchColumn {
    /// The search.
    #[not_personal]
    pub search: SavedSearchId,
    /// Its place among the columns of the search, counted from 0.
    #[not_personal]
    pub position: i64,
    /// The search word of the field. It can be the name of an own field, which a person wrote.
    #[personal(category = "content", basis = "legitimate_interests")]
    pub field: String,
    /// How wide the column is, if the person dragged it.
    #[not_personal]
    pub width: Option<i64>,
}

/// Whom a saved search is shared with: a group or a project (ADR-0034, decision 2).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ShareTarget {
    /// The people in a group.
    Group(GroupId),
    /// The people who see a project.
    Project(ProjectId),
}

// An ID of a group or a project, nothing a person wrote.
impl NotPersonal for ShareTarget {}

/// A saved search that is shared with a group or a project.
#[derive(Clone, Debug, PartialEq, Eq, Entity)]
pub struct SearchShare {
    /// The search.
    #[not_personal]
    pub search: SavedSearchId,
    /// The group or the project.
    #[not_personal]
    pub target: ShareTarget,
    /// When it was shared.
    #[not_personal]
    pub shared_at: Timestamp,
}

/// The saved search a project shows as its list (ADR-0034, decision 6). A project has at most one.
#[derive(Clone, Debug, PartialEq, Eq, Entity)]
pub struct ProjectDefaultList {
    /// The project.
    #[not_personal]
    pub project: ProjectId,
    /// The search, shared with the project.
    #[not_personal]
    pub search: SavedSearchId,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn column(field: &str, width: Option<u32>) -> Column {
        Column {
            field: field.to_owned(),
            width,
        }
    }

    #[test]
    fn qry_06_the_starting_presentation_is_a_comfortable_list_with_the_columns_of_the_project() {
        let starting = Presentation::default();
        assert_eq!(starting.view, ViewKind::List);
        assert_eq!(starting.spacing, Spacing::Comfortable);
        assert!(starting.columns.is_empty() && starting.group_by.is_none());
        assert!(starting.is_valid());
    }

    #[test]
    fn qry_06_a_presentation_keeps_columns_once_in_range_and_words_only() {
        let presentation = |columns: Vec<Column>, group_by: Option<&str>| Presentation {
            columns,
            group_by: group_by.map(str::to_owned),
            ..Presentation::default()
        };
        let good = presentation(
            vec![column("status", None), column("Due-date (2)", Some(120))],
            Some("statusGroup"),
        );
        assert!(good.is_valid());
        assert_eq!(
            good.fields().collect::<Vec<_>>(),
            ["status", "Due-date (2)", "statusGroup"]
        );
        for bad in [
            presentation(
                vec![column("status", None), column("status", Some(90))],
                None,
            ),
            presentation(
                vec![column("status", Some(*COLUMN_WIDTH_RANGE.start() - 1))],
                None,
            ),
            presentation(
                vec![column("status", Some(*COLUMN_WIDTH_RANGE.end() + 1))],
                None,
            ),
            presentation(vec![column("", None)], None),
            presentation(vec![column(" padded ", None)], None),
            presentation(
                vec![column("status", None)],
                Some(
                    "two
lines",
                ),
            ),
            presentation(vec![column(&"w".repeat(WORD_MAX + 1), None)], None),
            presentation(
                (0..=COLUMNS_MAX)
                    .map(|at| column(&format!("field{at}"), None))
                    .collect(),
                None,
            ),
        ] {
            assert!(!bad.is_valid(), "{bad:?}");
        }
        let widest = presentation(
            (0..COLUMNS_MAX)
                .map(|at| column(&format!("field{at}"), None))
                .collect(),
            None,
        );
        assert!(widest.is_valid());
    }
}
