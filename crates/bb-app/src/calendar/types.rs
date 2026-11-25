//! What the calendar read takes and answers (PLAN-09, M10-Q5).

use bb_core::id::IssueId;
use bb_core::issue::CalendarDay;
use bb_core::saved_search::CalendarOptions;
use bb_policy::{Authorized, Masked};
use serde::Serialize;

use crate::issues::IssueView;
use crate::search::ViewSearch;

/// The most days one calendar answer covers: six weeks, a month with the days around it.
pub const CALENDAR_DAYS_MAX: i64 = 42;

/// The most issues one calendar answer holds; the answer says when the window has more.
pub const CALENDAR_ISSUES_MAX: usize = 200;

/// Ask for the issues of a window of days.
pub struct CalendarQuery {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The search the calendar shows.
    pub search: ViewSearch,
    /// The project whose calendar it is (it and the projects inside it), by ID or short name;
    /// none for everywhere the reader sees.
    pub project: Option<String>,
    /// *Only this project*: without its sub-projects (`project is KEY` in the text).
    pub only_this_project: bool,
    /// The options of the view: the date field (*Dates from*).
    pub options: CalendarOptions,
    /// The first day of the window.
    pub first: CalendarDay,
    /// The last day of the window, at most [`CALENDAR_DAYS_MAX`] days after the first.
    pub last: CalendarDay,
}

/// An issue on its day.
#[derive(Debug, Serialize)]
pub struct CalendarEntry {
    /// The day of its date field.
    pub day: CalendarDay,
    /// The issue, with what the reader may see of it.
    pub issue: Authorized<IssueView>,
    /// Whether the reader may change the date field on this issue: only then can it be dragged to
    /// another day. The change is the change of that field, judged again when it is saved.
    pub may_move: bool,
}

/// A mark on a day that is not dragged on the calendar: the due date of a goal (and, with the
/// releases, a release date).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CalendarMark {
    /// The goal.
    pub issue: IssueId,
    /// Its readable ID.
    pub readable_id: String,
    /// Its title; none when the reader may not read it.
    pub title: Option<String>,
    /// The day it stands on: its *Due date*.
    pub day: CalendarDay,
}

/// The issues of a window of days, each on its day, and the marks of the window.
#[derive(Debug, Serialize)]
pub struct CalendarPage {
    /// The search text that was run: a project's calendar has its project written in.
    pub search_text: String,
    /// The issues, by day, then in the order of the search.
    pub entries: Vec<CalendarEntry>,
    /// Whether the window holds more issues than [`CALENDAR_ISSUES_MAX`]: the screen asks for a
    /// shorter window or a narrower search.
    pub more: bool,
    /// The goals of the issues shown whose *Due date* the reader may read and that lies in the
    /// window.
    pub marks: Vec<CalendarMark>,
}

impl Masked for CalendarPage {}
