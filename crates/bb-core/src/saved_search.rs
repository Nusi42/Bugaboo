//! Saved searches (QRY-06, CFG-02, ADR-0034): a name, the canonical search text, how the results
//! are shown, the person who made it, and whom it is shared with. How close together the rows are
//! is not kept: every list follows the person's setting *Spacing* (ADR-0034, amendment).
//!
//! A saved search is private when it is made. The text is the one canonical text of the search
//! language (ADR-0005, amendment M5): the tree never crosses the wire and is never stored. How the
//! results are shown (the *presentation*) is not part of the language: the kind of view, the
//! column and the grouping name a field by its search word, so that a screen needs no second list.

use jiff::Timestamp;
use serde::Serialize;

use crate::fields::ID_WORD;
use crate::id::{GroupId, ProjectId, SavedSearchId, UserId};
use crate::names::name_key;
use crate::personal::{Entity, NotPersonal};

/// The most columns one saved search shows.
pub const COLUMNS_MAX: usize = 40;

/// The longest search word of a column or of the grouping, in characters: a name of an own field
/// is the longest word there is (`own_field_word` leaves a name that is not one word as it is).
pub const WORD_MAX: usize = crate::text::NAME_MAX;

/// The narrowest and the widest a column can be, in the units of the list (pixels at the starting
/// size of text). A column never disappears by its width, and no width is wider than a screen.
pub const COLUMN_WIDTH_RANGE: std::ops::RangeInclusive<u32> = 24..=2000;

/// The most saved searches one person has (those in the trash included): the 501st is refused.
pub const SAVED_SEARCHES_MAX: usize = 500;

/// The most groups and projects one saved search is shared with.
pub const SHARES_MAX: usize = 100;

/// The most card limits one board keeps: one for each column, and a board has no more columns
/// than the most statuses of a workflow or choices of a field.
pub const CARD_LIMITS_MAX: usize = 100;

/// The highest card limit of a column: a soft limit says something only when it is small.
pub const CARD_LIMIT_MAX: u32 = 999;

words! {
    /// The kind of view a saved search is shown as (ADR-0034, decision 1 and the amendment of
    /// 2026-11-24): a list, the timeline, a board or a calendar.
    pub enum ViewKind {
        /// A list of issues, one row each.
        List => "list",
        /// The timeline: bars over days (PLAN-01, ADR-0043, decision 2).
        Timeline => "timeline",
        /// The relation map of the results: at most 200 of them and the relations between them,
        /// with no walk beyond them (ISS-16, ADR-0034, amendment 2026-11-24).
        Map => "map",
        /// A board: a column for each status, status group or choice, with cards (PLAN-06).
        Board => "board",
        /// A calendar: each issue on the day of one date field (PLAN-09).
        Calendar => "calendar",
    }
}

words_in_api!(
    ViewKind,
    "The kind of view a saved search is shown as: a list, the timeline, the relation map, a board or a calendar."
);

words! {
    /// What a calendar shows first: a month or a week (PLAN-09).
    pub enum CalendarSpan {
        /// A month, a row for each week.
        Month => "month",
        /// One week, a column for each day.
        Week => "week",
    }
}

words_in_api!(
    CalendarSpan,
    "What a calendar shows first: a month or a week."
);

/// The soft limit of one column of a board (M10-Q2): the column's title says "4 of 3" when more
/// cards than this are in it, and nothing is refused.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize)]
pub struct CardLimit {
    /// The column, by the value it stands for as the search language writes it: the name of a
    /// status, the word of a status group or the name of a choice; `None` for the column of the
    /// issues without a value.
    pub column: Option<String>,
    /// How many cards the column should hold at most.
    pub limit: u32,
}

/// The options of a board view, kept with its saved search (ADR-0034, amendment of 2026-11-24).
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize)]
pub struct BoardOptions {
    /// The field the columns come from, by its search word: *Status group* or an own field that
    /// holds one choice; `None` for *Status*, whose statuses of the same name are one column.
    pub columns: Option<String>,
    /// The field that splits the board into rows, by its search word; `None` for no rows.
    pub rows: Option<String>,
    /// The soft limits of the columns that have one, each column once.
    pub limits: Vec<CardLimit>,
}

impl BoardOptions {
    /// Whether the options can be kept: words where they name fields, two different fields for the
    /// columns and the rows, at most [`CARD_LIMITS_MAX`] limits, each column once (names that are
    /// the same name are one column, [`name_key`]), each limit
    /// between 1 and [`CARD_LIMIT_MAX`]. Whether the fields and the values exist for a person is
    /// the check of the use case.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.columns.as_deref().is_none_or(is_word)
            && self.rows.as_deref().is_none_or(is_word)
            && (self.rows.is_none() || self.rows != self.columns)
            && self.limits.len() <= CARD_LIMITS_MAX
            && self.limits.iter().enumerate().all(|(at, each)| {
                (1..=CARD_LIMIT_MAX).contains(&each.limit)
                    && each.column.as_deref().is_none_or(is_word)
                    && !self.limits[..at].iter().any(|earlier| {
                        earlier.column.as_deref().map(name_key)
                            == each.column.as_deref().map(name_key)
                    })
            })
    }

    /// The rows of the limits of the saved search `search`, counted from 0.
    #[must_use]
    pub fn limit_rows(&self, search: SavedSearchId) -> Vec<SearchCardLimit> {
        self.limits
            .iter()
            .zip(0_i64..)
            .map(|(each, position)| SearchCardLimit {
                search,
                position,
                column: each.column.clone(),
                limit: i64::from(each.limit),
            })
            .collect()
    }

    /// The limits of `rows` (in any order), as the board keeps them.
    #[must_use]
    pub fn limits_of(rows: &[SearchCardLimit]) -> Vec<CardLimit> {
        let mut ordered: Vec<&SearchCardLimit> = rows.iter().collect();
        ordered.sort_by_key(|row| row.position);
        ordered
            .into_iter()
            .filter_map(|row| {
                Some(CardLimit {
                    column: row.column.clone(),
                    limit: u32::try_from(row.limit).ok()?,
                })
            })
            .collect()
    }
}

/// The options of a calendar view, kept with its saved search (M10-Q5).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CalendarOptions {
    /// The date field the issues stand on (*Dates from*), by its search word; `None` for *Due
    /// date*.
    pub dates: Option<String>,
    /// What the calendar shows first.
    pub span: CalendarSpan,
}

impl Default for CalendarOptions {
    /// *Due date*, a month.
    fn default() -> Self {
        Self {
            dates: None,
            span: CalendarSpan::Month,
        }
    }
}

words! {
    /// How many days the timeline shows across the screen (PLAN-01): from a day to a quarter.
    pub enum Zoom {
        /// One column for each day.
        Day => "day",
        /// One column for each week.
        Week => "week",
        /// One column for each month.
        Month => "month",
        /// One column for each quarter.
        Quarter => "quarter",
    }
}

words_in_api!(
    Zoom,
    "How many days the timeline shows across the screen: day, week, month or quarter."
);

/// The options of a timeline view, kept with its saved search (ADR-0034; ADR-0043, decision 2).
/// The grouping is the one of every view ([`Presentation::group_by`]).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct TimelineOptions {
    /// The two date fields the bars take their days from, by their search words (the start, then
    /// the end); none for *Start date* and *Due date*.
    pub dates: Option<(String, String)>,
    /// Whether *Plan by effort* is on (ADR-0030, decision 3).
    pub plan_by_effort: bool,
    /// The zoom.
    pub zoom: Zoom,
    /// Whether *Deadline drivers* is on: the chains that set the ends of the view are outlined
    /// (PLAN-05, ADR-0043, amendment of 2026-11-24).
    pub deadline_drivers: bool,
}

impl Default for TimelineOptions {
    /// *Start date* and *Due date*, *Plan by effort* and *Deadline drivers* off, a column for each
    /// week.
    fn default() -> Self {
        Self {
            dates: None,
            plan_by_effort: false,
            zoom: Zoom::Week,
            deadline_drivers: false,
        }
    }
}

/// One column of a list: the field by its search word and, when the person dragged it, its width.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Column {
    /// The search word of the field (`assignedTo`).
    pub field: String,
    /// How wide the column is; `None` for the width the list chooses.
    pub width: Option<u32>,
}

/// How a saved search is shown (*presentation*, ADR-0034, decision 1). No columns at all means the
/// columns of the project's list.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Presentation {
    /// The kind of view.
    pub view: ViewKind,
    /// The columns, in order.
    pub columns: Vec<Column>,
    /// The field the rows are grouped by, by its search word.
    pub group_by: Option<String>,
    /// The options of the timeline, kept for every view so that switching back keeps them; `None`
    /// keeps the ones a search has (the starting ones for a new search), so a client that knows
    /// only lists never takes them away.
    pub timeline: Option<TimelineOptions>,
    /// The options of the board, kept like those of the timeline.
    pub board: Option<BoardOptions>,
    /// The options of the calendar, kept like those of the timeline.
    pub calendar: Option<CalendarOptions>,
}

impl Default for Presentation {
    /// A list with the columns of the project's list, not grouped.
    fn default() -> Self {
        Self {
            view: ViewKind::List,
            columns: Vec::new(),
            group_by: None,
            timeline: None,
            board: None,
            calendar: None,
        }
    }
}

/// Whether `word` can be the search word of a field: a text as it is kept (trimmed, not empty, at
/// most [`WORD_MAX`] characters, no control character). Whether a field has the word is the check
/// of the use case.
pub(crate) fn is_word(word: &str) -> bool {
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
            && self
                .timeline
                .as_ref()
                .and_then(|timeline| timeline.dates.as_ref())
                .is_none_or(|(start, end)| is_word(start) && is_word(end) && start != end)
            && self.board.as_ref().is_none_or(BoardOptions::is_valid)
            && self
                .calendar
                .as_ref()
                .and_then(|calendar| calendar.dates.as_deref())
                .is_none_or(is_word)
    }

    /// The presentation of `search` with its `columns` (in any order); the limits of its board are
    /// in `search` already.
    #[must_use]
    pub fn of(search: &SavedSearch, columns: &[SearchColumn]) -> Self {
        let mut ordered: Vec<&SearchColumn> = columns.iter().collect();
        ordered.sort_by_key(|column| column.position);
        Self {
            view: search.view,
            columns: ordered
                .into_iter()
                .map(|column| Column {
                    field: column.field.clone(),
                    width: column.width.and_then(|width| u32::try_from(width).ok()),
                })
                .collect(),
            group_by: search.group_by.clone(),
            timeline: Some(search.timeline.clone()),
            board: Some(search.board.clone()),
            calendar: Some(search.calendar.clone()),
        }
    }

    /// The columns as the rows of the saved search `search`, counted from 0.
    #[must_use]
    pub fn rows(&self, search: SavedSearchId) -> Vec<SearchColumn> {
        self.columns
            .iter()
            .zip(0_i64..)
            .map(|(column, position)| SearchColumn {
                search,
                position,
                field: column.field.clone(),
                width: column.width.map(i64::from),
            })
            .collect()
    }

    /// The search words of every field the presentation names: the columns (besides the ID column),
    /// the grouping, the date fields of the timeline and of the calendar, and the columns and the
    /// rows of the board.
    pub fn fields(&self) -> impl Iterator<Item = &str> {
        self.columns
            .iter()
            .map(|column| column.field.as_str())
            .filter(|word| *word != ID_WORD)
            .chain(self.group_by.as_deref())
            .chain(
                self.timeline
                    .iter()
                    .filter_map(|timeline| timeline.dates.as_ref())
                    .flat_map(|(start, end)| [start.as_str(), end.as_str()]),
            )
            .chain(self.board.iter().flat_map(|board| {
                board
                    .columns
                    .as_deref()
                    .into_iter()
                    .chain(board.rows.as_deref())
            }))
            .chain(
                self.calendar
                    .iter()
                    .filter_map(|calendar| calendar.dates.as_deref()),
            )
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
    #[person(author)]
    pub maker: UserId,
    /// The kind of view.
    #[not_personal]
    pub view: ViewKind,
    /// The field the rows are grouped by, by its search word.
    #[personal(category = "content", basis = "legitimate_interests")]
    pub group_by: Option<String>,
    /// The options of the timeline. Its date fields are named by their search words, which can be
    /// the names of own fields, which a person wrote.
    #[personal(category = "content", basis = "legitimate_interests")]
    pub timeline: TimelineOptions,
    /// The options of the board. Its fields are named by their search words and its limits by the
    /// names of statuses and choices, which a person wrote. The limits are rows of their own
    /// ([`SearchCardLimit`]).
    #[personal(category = "content", basis = "legitimate_interests")]
    pub board: BoardOptions,
    /// The options of the calendar. Its date field is named by its search word, which can be the
    /// name of an own field.
    #[personal(category = "content", basis = "legitimate_interests")]
    pub calendar: CalendarOptions,
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

impl SavedSearch {
    /// Takes how `presentation` shows the search (the kind of view, the grouping and, when it
    /// names them, the options of the timeline, the board and the calendar); the columns are rows
    /// of their own ([`Presentation::rows`]), and so are the limits of the board
    /// ([`BoardOptions::limit_rows`]).
    pub fn take_presentation(&mut self, presentation: &Presentation) {
        self.view = presentation.view;
        self.group_by.clone_from(&presentation.group_by);
        if let Some(timeline) = &presentation.timeline {
            self.timeline.clone_from(timeline);
        }
        if let Some(board) = &presentation.board {
            self.board.clone_from(board);
        }
        if let Some(calendar) = &presentation.calendar {
            self.calendar.clone_from(calendar);
        }
    }
}

/// One card limit of the board of a saved search, in order.
#[derive(Clone, Debug, PartialEq, Eq, Entity)]
pub struct SearchCardLimit {
    /// The search. A limit is the search's maker's as much as the search is.
    #[not_personal]
    #[person_for(part_of, link = "author")]
    pub search: SavedSearchId,
    /// Its place among the limits of the search, counted from 0.
    #[not_personal]
    pub position: i64,
    /// The column, by the name of its status or choice or the word of its status group; `None`
    /// for the column of the issues without a value. A person wrote the names.
    #[personal(category = "content", basis = "legitimate_interests")]
    pub column: Option<String>,
    /// The limit.
    #[not_personal]
    pub limit: i64,
}

/// One column of a saved search, in order.
#[derive(Clone, Debug, PartialEq, Eq, Entity)]
pub struct SearchColumn {
    /// The search. A column is the search's maker's as much as the search is.
    #[not_personal]
    #[person_for(part_of, link = "author")]
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

words! {
    /// Which set of saved searches a list shows.
    pub enum SearchSet {
        /// The ones the person made.
        Mine => "mine",
        /// The ones other people shared with a group of the person or a project they see.
        Shared => "shared",
    }
}

words_in_api!(
    SearchSet,
    "Which set of saved searches a list shows: the person's own, or the ones shared with them."
);

words! {
    /// What a saved search is shared with: a group or a project.
    pub enum ShareKind {
        /// The people in a group.
        Group => "group",
        /// The people who see a project.
        Project => "project",
    }
}

words_in_api!(
    ShareKind,
    "What a saved search is shared with: a group or a project."
);

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

impl ShareTarget {
    /// The word that tells a group from a project.
    #[must_use]
    pub const fn kind(self) -> ShareKind {
        match self {
            Self::Group(_) => ShareKind::Group,
            Self::Project(_) => ShareKind::Project,
        }
    }

    /// The ID of the group or the project, as text.
    #[must_use]
    pub fn id_text(self) -> String {
        match self {
            Self::Group(group) => group.to_string(),
            Self::Project(project) => project.to_string(),
        }
    }

    /// The group or project of `kind` that `id` names; `None` for text that is no such ID.
    #[must_use]
    pub fn parse(kind: ShareKind, id: &str) -> Option<Self> {
        match kind {
            ShareKind::Group => id.parse().ok().map(Self::Group),
            ShareKind::Project => id.parse().ok().map(Self::Project),
        }
    }
}

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
    fn qry_06_the_starting_presentation_is_a_list_with_the_columns_of_the_project() {
        let starting = Presentation::default();
        assert_eq!(starting.view, ViewKind::List);
        assert!(starting.columns.is_empty() && starting.group_by.is_none());
        assert!(starting.is_valid());
    }

    #[test]
    fn qry_06_a_share_tells_a_group_from_a_project_by_a_word_and_an_id() {
        let mut ids = crate::id::SequentialIds::default();
        let group = GroupId::generate(&mut ids).unwrap();
        let project = ProjectId::generate(&mut ids).unwrap();
        assert_eq!(ShareTarget::Group(group).kind().code(), "group");
        assert_eq!(ShareTarget::Project(project).kind().code(), "project");
        let named = |kind: ShareKind, id: String| ShareTarget::parse(kind, &id);
        assert_eq!(
            named(ShareKind::Group, group.to_string()),
            Some(ShareTarget::Group(group))
        );
        assert_eq!(ShareTarget::parse(ShareKind::Group, "not an id"), None);
        assert_eq!(ShareTarget::Group(group).id_text(), group.to_string());
        assert_eq!(ShareTarget::Project(project).id_text(), project.to_string());
    }

    #[test]
    fn qry_06_a_presentation_is_kept_as_rows_and_read_back_in_order() {
        let presentation = Presentation {
            columns: vec![column("title", Some(300)), column("status", None)],
            group_by: Some("statusGroup".to_owned()),
            // Read back, a search always names the options of its timeline, board and calendar.
            timeline: Some(TimelineOptions::default()),
            board: Some(BoardOptions::default()),
            calendar: Some(CalendarOptions::default()),

            ..Presentation::default()
        };
        let mut ids = crate::id::SequentialIds::default();
        let id = SavedSearchId::generate(&mut ids).unwrap();
        let mut rows = presentation.rows(id);
        assert_eq!(
            rows.iter().map(|row| row.position).collect::<Vec<_>>(),
            [0, 1]
        );
        rows.reverse();
        let search = SavedSearch {
            id,
            name: "Mine".to_owned(),
            text: String::new(),
            maker: UserId::generate(&mut ids),
            view: presentation.view,
            group_by: presentation.group_by.clone(),
            timeline: TimelineOptions::default(),
            board: BoardOptions::default(),
            calendar: CalendarOptions::default(),
            created_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
            deleted_at: None,
            version: 1,
        };
        assert_eq!(Presentation::of(&search, &rows), presentation);
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

    #[test]
    fn plan_01_a_timeline_keeps_two_different_date_fields_by_their_words() {
        let timeline = |dates: Option<(&str, &str)>| Presentation {
            view: ViewKind::Timeline,
            timeline: Some(TimelineOptions {
                dates: dates.map(|(start, end)| (start.to_owned(), end.to_owned())),
                plan_by_effort: true,
                zoom: Zoom::Month,
                deadline_drivers: false,
            }),
            ..Presentation::default()
        };
        assert!(timeline(None).is_valid());
        let own = timeline(Some(("Kick-off", "Launch")));
        assert!(own.is_valid());
        assert_eq!(own.fields().collect::<Vec<_>>(), ["Kick-off", "Launch"]);
        for bad in [Some(("Launch", "Launch")), Some(("", "Launch"))] {
            assert!(!timeline(bad).is_valid(), "{bad:?}");
        }
        assert_eq!(Zoom::from_code("quarter"), Some(Zoom::Quarter));
    }
}
