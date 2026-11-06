//! What the use cases of working hours take and give back (TIME-08, TIME-09, TIME-10, TIME-12).

use bb_core::id::UserId;
use bb_core::time::Minutes;
use bb_core::working_time::{Span, Times, Totals, Warning};
use bb_policy::{Masked, PersonRef};
use jiff::Timestamp;
use jiff::civil::Date;

/// A date as a request writes it (`2026-10-05`); `None` for a text that is none.
#[must_use]
pub fn read_date(text: &str) -> Option<Date> {
    text.parse().ok()
}

/// A stretch of a work day as a request writes it: its moments in RFC 3339, no end while it runs;
/// `None` for a text that is no moment.
#[must_use]
pub fn read_span(start: &str, end: Option<&str>) -> Option<Span> {
    Some(Span {
        start: start.parse().ok()?,
        end: match end {
            Some(end) => Some(end.parse().ok()?),
            None => None,
        },
    })
}

/// The work days of a stretch of dates: the person's own, or another person's on *Working hours
/// of others*.
pub struct WorkDaysQuery {
    /// The secret of the session of the request.
    pub session_token: String,
    /// Another person; `None` for the person who asks.
    pub person: Option<UserId>,
    /// The first date.
    pub first: Date,
    /// The last date, at most [`super::RANGE_DAYS_MAX`] days after the first.
    pub last: Date,
}

/// *Start work day*: the moment of the tap starts the person's day, or a new period of it.
pub struct StartWorkDay {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The time zone of the device, by its name; it counts only when the person left their time
    /// zone on *Match my device* and the organization chose none.
    pub device_zone: Option<String>,
}

/// A tap on the day that runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tap {
    /// *Start break*.
    StartBreak,
    /// *End break*.
    EndBreak,
    /// *End work day*: the break that runs ends too.
    End,
}

/// A tap on the person's day that runs: *Start break*, *End break* or *End work day*.
pub struct TapWorkDay {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The version of the day the person looked at.
    pub version: i64,
    /// Which tap.
    pub tap: Tap,
}

/// New times of a work day, typed afterwards: the person's own, or another person's.
pub struct ChangeWorkDay {
    /// The secret of the session of the request.
    pub session_token: String,
    /// Another person; `None` for the person who asks.
    pub person: Option<UserId>,
    /// The date of the day.
    pub date: Date,
    /// The version the person looked at; 0 for a date that has no work day yet.
    pub version: i64,
    /// The periods of work.
    pub periods: Vec<Span>,
    /// The breaks.
    pub breaks: Vec<Span>,
    /// Why, in the person's own words: needed for another person's day and for one's own past
    /// day (ADR-0035, decision 7).
    pub reason: Option<String>,
    /// The time zone of the device of the person who asks; it counts only for a date that has no
    /// work day yet, and only when the person and the organization chose no zone.
    pub device_zone: Option<String>,
}

/// The history of one work day: its changes with the old and the new times, who and why.
pub struct WorkDayHistoryQuery {
    /// The secret of the session of the request.
    pub session_token: String,
    /// Another person; `None` for the person who asks.
    pub person: Option<UserId>,
    /// The date of the day.
    pub date: Date,
}

/// The person's own week: the working hours, the time booked on issues and the difference of each
/// day (ADR-0035, decision 4).
pub struct BookedWeekQuery {
    /// The secret of the session of the request.
    pub session_token: String,
    /// A date of the week.
    pub date: Date,
}

/// The people whose working hours the reader may open, to choose one from.
pub struct PeopleOfHoursQuery {
    /// The secret of the session of the request.
    pub session_token: String,
    /// Words that every found person holds in their name.
    pub search: Option<String>,
    /// Where the page starts: the `next_cursor` of the page before.
    pub cursor: Option<String>,
    /// How many people the page holds at most.
    pub limit: u32,
}

/// How many work days are older than the keep period, for each person and in total.
pub struct OldWorkDaysQuery {
    /// The secret of the session of the request.
    pub session_token: String,
}

/// *Delete them*: the work days older than the keep period, of everybody or of one person.
pub struct DeleteOldWorkDays {
    /// The secret of the session of the request.
    pub session_token: String,
    /// One person; `None` for everybody.
    pub person: Option<UserId>,
}

/// A work day as its reader sees it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkDayView {
    /// Its date.
    pub date: Date,
    /// The time zone it started in.
    pub zone: String,
    /// Its periods of work and its breaks.
    pub times: Times,
    /// What it adds up to; what runs counts until now.
    pub totals: Totals,
    /// The warnings of the working-time rules, as stored with the day.
    pub warnings: Vec<Warning>,
    /// Whether a period or a break runs.
    pub runs: bool,
    /// Counts up with every change.
    pub version: i64,
    /// Whether a change of the day has a reason: it shows *Changed* and has a history to open.
    pub changed: bool,
}

/// The total of one week.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WeekTotal {
    /// The first date of the week.
    pub first: Date,
    /// The time worked in it.
    pub worked: Minutes,
}

/// The work days of a stretch of dates.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkDaysPage {
    /// Whose days they are.
    pub person: PersonRef,
    /// The days with a record, by date; none while *Totals only* hides them.
    pub days: Vec<WorkDayView>,
    /// What the days add up to; `None` while *Totals only* hides it.
    pub totals: Option<Totals>,
    /// The time worked in each week of the stretch, by the person's first day of the week.
    pub weeks: Vec<WeekTotal>,
    /// Whether *Totals only* hides the days.
    pub totals_only: bool,
    /// Whether the reader may change the days.
    pub may_change: bool,
}

impl Masked for WorkDaysPage {}

/// The answer to a change of a work day: the day as it is now.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChangedWorkDay {
    /// The day.
    pub day: WorkDayView,
}

impl Masked for ChangedWorkDay {}

/// One change of a work day in its history.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkDayChange {
    /// When.
    pub at: Timestamp,
    /// Who changed it.
    pub by: PersonRef,
    /// Why, when a reason was given.
    pub reason: Option<String>,
    /// The times before; none for the change that made the day.
    pub before: Option<Times>,
    /// The times after.
    pub after: Option<Times>,
}

/// The history of a work day, oldest change first.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkDayHistory {
    /// The changes, at most [`super::HISTORY_CHANGES_MAX`] of the newest.
    pub changes: Vec<WorkDayChange>,
    /// Whether older changes were left out.
    pub truncated: bool,
}

impl Masked for WorkDayHistory {}

/// One day of the person's own week, compared with the time booked on issues.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BookedDay {
    /// The date.
    pub date: Date,
    /// The working hours of the day.
    pub worked: Minutes,
    /// The time booked on issues on the day.
    pub booked: Minutes,
    /// *Working hours not booked on issues*: the working hours minus the time booked; below zero
    /// when more was booked than worked.
    pub not_booked: i64,
}

/// The person's own week, compared with the time booked on issues.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BookedWeek {
    /// The seven days, in order.
    pub days: Vec<BookedDay>,
    /// The week: what the days add up to, worked out here and nowhere else.
    pub week: BookedTotals,
}

/// What the days of a week add up to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BookedTotals {
    /// The working hours of the week.
    pub worked: i64,
    /// The time booked on issues in the week.
    pub booked: i64,
    /// *Working hours not booked on issues* in the week.
    pub not_booked: i64,
}

impl Masked for BookedWeek {}

/// The work days of one person that are older than the keep period.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OldOfPerson {
    /// The person, as the reader sees people in the settings of the organization.
    pub person: PersonRef,
    /// The ID of the person, for *Delete them* of one person; none when the reader does not see
    /// the person by name.
    pub id: Option<UserId>,
    /// How many of their days.
    pub count: u64,
}

/// The work days older than the keep period.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OldWorkDays {
    /// The keep period in force, in whole years.
    pub keep_years: i64,
    /// The days with a date before this one are older than it.
    pub before: Date,
    /// How many there are.
    pub total: u64,
    /// How many of each person, in the order of their names, at most [`super::OLD_PEOPLE_MAX`].
    pub people: Vec<OldOfPerson>,
    /// Whether more people have old days than the list names.
    pub truncated: bool,
}

impl Masked for OldWorkDays {}

/// How many work days were deleted.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DeletedWorkDays {
    /// The count.
    pub deleted: u64,
}

impl Masked for DeletedWorkDays {}
