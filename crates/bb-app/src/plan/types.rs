//! What the timeline is asked and what it answers (PLAN-01, PLAN-08, PLAN-10, PLAN-13).

use bb_core::holidays::Holiday;
use bb_core::id::{IssueId, SavedSearchId, UserId};
use bb_core::plan::Days;
use bb_core::plan::bar::WorkedOut;
use bb_core::plan::warning::{Warning, WarningKind, WayOut};
use bb_core::saved_search::TimelineOptions;
use bb_core::work_times::Halves;
use bb_policy::{Authorized, Masked, PersonRef};
use jiff::Timestamp;
use jiff::civil::Date;
use serde::Serialize;

use crate::issues::IssueView;
use crate::search::ShownKey;

/// The search a timeline shows: a saved search the reader may open, or a text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TimelineSearch {
    /// A saved search, by its ID.
    Saved(SavedSearchId),
    /// A search text, which need not be saved.
    Text(String),
}

/// Which part of the timeline a page is of.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TimelinePart {
    /// The rows with their bars, the dependencies between them and their warnings.
    Rows,
    /// The issues of *Not scheduled yet*: no date the reader may read.
    NotScheduled,
}

/// Ask for a page of the timeline.
pub struct TimelineQuery {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The search.
    pub search: TimelineSearch,
    /// The project whose timeline it is (the project and the projects inside it), by ID or short
    /// name; none for everywhere the reader sees.
    pub project: Option<String>,
    /// The options of the view: the date fields and *Plan by effort*.
    pub options: TimelineOptions,
    /// The field the rows are grouped by, by its search word.
    pub group_by: Option<String>,
    /// The first day of the span the screen shows.
    pub first: Date,
    /// The last day of the span.
    pub last: Date,
    /// Which part.
    pub part: TimelinePart,
    /// Where the page starts: the `next_cursor` of the page before.
    pub cursor: Option<String>,
    /// How many issues a page holds at most.
    pub limit: u32,
}

/// What a day of a bar holds that keeps the person from working on it, as the reader may know it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum Hatched {
    /// The person is away (only ever *Away*, never why).
    Away {
        /// In the morning.
        morning: bool,
        /// In the afternoon.
        afternoon: bool,
    },
    /// A public holiday that counts for the person.
    Holiday {
        /// Which one.
        holiday: Holiday,
    },
    /// Not one of the person's usual work days, or no share in the project.
    DayOff,
}

impl Hatched {
    /// The away halves as they are shown.
    #[must_use]
    pub const fn away(halves: Halves) -> Self {
        Self::Away {
            morning: halves.morning,
            afternoon: halves.afternoon,
        }
    }
}

/// One hatched day of a bar.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct HatchedDay {
    /// The day.
    pub date: Date,
    /// What it holds.
    pub what: Hatched,
}

/// The time factor of a bar, as its card shows it (*×1.25 · Lea's own estimate*).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct FactorShown {
    /// In percent of the estimate.
    pub percent: u16,
    /// Whether the person on the bar set it themselves.
    pub own_estimate: bool,
}

/// The bar of an issue.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct BarView {
    /// The days it covers.
    pub days: Days,
    /// Which date the plan worked out (shown, never stored).
    pub worked_out: WorkedOut,
    /// How much of it is done, in percent; none without an estimate the reader may read.
    pub done: Option<u8>,
    /// The person on the bar, as the reader may see them; none for nobody, or when the reader may
    /// not see who it is.
    pub person: Option<PersonRef>,
    /// The time factor, when it is not ×1 and the reader may read it.
    pub factor: Option<FactorShown>,
    /// The days of the bar on which its person does not work, as *Who sees people* allows.
    pub hatched: Vec<HatchedDay>,
}

/// One row of the timeline: an issue in a group, with its bar.
#[derive(Debug, Serialize)]
pub struct TimelineRow {
    /// The group the row is in; none when the rows are not grouped, or when the reader may not
    /// know the group.
    pub group: Option<ShownKey>,
    /// The issue, with what the reader may see of it.
    pub issue: Authorized<IssueView>,
    /// Its bar; none in *Not scheduled yet*.
    pub bar: Option<BarView>,
    /// Whether the row is that of an assigned person who is not the person on the bar: the bar is
    /// planned with the first person's work times (*Group by person*).
    pub copy: bool,
}

/// A plan warning someone accepted, as the reader may see who.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct AcceptedView {
    /// Who accepted it.
    pub by: PersonRef,
    /// When.
    pub at: Timestamp,
}

/// A plan warning of an issue, with what it names as the reader may know it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct WarningView {
    /// The issue the warning is on.
    pub issue: IssueId,
    /// The kind.
    pub kind: WarningKind,
    /// The sentence of the warning, filled in with what it holds by the screen.
    pub sentence: bb_core::message::MessageKey,
    /// The days the warning is about.
    pub days: Days,
    /// What the warning holds.
    pub what: Warning,
    /// The other issue it names (the one it waits for, or the sub-issue), with its readable ID.
    pub other: Option<(IssueId, String)>,
    /// The person on the bar it is about, for *Away* and *Too much at once*; none when the reader
    /// may not see who it is (*"the assigned person is away"*).
    pub person: Option<PersonRef>,
    /// The ways out that fit it.
    pub ways_out: Vec<WayOut>,
    /// The day *Move it after* starts the issue after, when it is a way out.
    pub move_after: Option<Date>,
    /// The due date *Make it longer* answers, when it is a way out; it changes only when the person
    /// confirms it.
    pub due_needed: Option<Date>,
    /// Whether and by whom it was accepted (*Keep and accept*); it then shows faded.
    pub accepted: Option<AcceptedView>,
}

/// A page of the timeline.
#[derive(Debug, Serialize)]
pub struct TimelinePage {
    /// The rows, in the order of the search (grouped rows keep it inside each group).
    pub rows: Vec<TimelineRow>,
    /// The cursor of the next page, if there is one.
    pub next_cursor: Option<String>,
    /// The dependencies between issues of the page: the issue that must finish first, then the
    /// one that waits.
    pub dependencies: Vec<(IssueId, IssueId)>,
    /// The warnings of the issues of the page.
    pub warnings: Vec<WarningView>,
    /// How many issues the search finds that wait in *Not scheduled yet*.
    pub not_scheduled: u64,
}

impl Masked for TimelinePage {}

/// Ask for the preview of *Move it after*.
pub struct MoveAfterQuery {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The issue the warning is on, by its readable ID or internal ID.
    pub issue: String,
    /// The day the issue is to start after: the `move_after` of its warning.
    pub after: Date,
    /// The options of the view the warning was shown in: the date fields and *Plan by effort*.
    pub options: TimelineOptions,
}

/// One issue of the preview of *Move it after*, with its new dates.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct MovePreview {
    /// The issue.
    pub issue: IssueId,
    /// Its readable ID.
    pub key: String,
    /// The version it has: the move takes it, so a change in between is caught.
    pub version: i64,
    /// The days of its bar now.
    pub from: Days,
    /// The days of its bar after the move.
    pub to: Days,
    /// The *Start date* it gets; none when it has none.
    pub start_date: Option<Date>,
    /// The *Due date* it gets; none when it has none.
    pub due_date: Option<Date>,
}

/// The preview of *Move it after*: every issue that moves and its new dates, before anything is
/// saved. The move itself is one run of `POST /api/v1/issues/move-dates`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct MovePreviewPage {
    /// The issues, in the order they move along the dependencies.
    pub issues: Vec<MovePreview>,
}

impl Masked for MovePreviewPage {}

/// Keep and accept a plan warning.
pub struct AcceptWarning {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The issue, by its readable ID or internal ID.
    pub issue: String,
    /// The version of the issue the person looked at.
    pub version: i64,
    /// The kind of the warning.
    pub kind: WarningKind,
}

/// A warning that was accepted.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct AcceptedWarningView {
    /// The issue.
    pub issue: IssueId,
    /// The kind.
    pub kind: WarningKind,
    /// The dates it was accepted for: it comes back when they change.
    pub start_date: Option<Date>,
    /// The due date it was accepted for.
    pub due_date: Option<Date>,
    /// Who accepted it.
    pub by: UserId,
}

impl Masked for AcceptedWarningView {}
