//! What the use cases of the timer and of time entries take and give back (TIME-01, TIME-02,
//! TIME-13). Inputs that change something are named after what they do; inputs that only read end
//! in `Query` (ARCH-EVENTS). Everything that is given back is wrapped in `Authorized<T>`.

use std::collections::BTreeMap;

use bb_core::id::{FieldId, IssueId, TimeEntryId};
use bb_core::own_fields_value::OwnValues;
use bb_core::time::Minutes;
use bb_policy::{Authorized, Masked, PersonRef};
use jiff::Timestamp;
use jiff::civil::Date;
use serde::Serialize;
use serde_json::Value;

/// What a person fills in for an entry: the note, *Billable* and the values of the own fields of
/// time entries (by the key of the field). A field left out takes its default from the issue, then
/// the project, then the projects above.
#[derive(Clone, Debug, Default)]
pub struct EntryDetails {
    /// The note, plain text; none or empty for no note.
    pub note: Option<String>,
    /// Whether the time can be charged to a customer; only taken while the organization offers it.
    pub billable: Option<bool>,
    /// The values of own fields of time entries, by the key of the field.
    pub fields: BTreeMap<String, Value>,
}

/// The timer of the person who asks.
pub struct TimerQuery {
    /// The secret of the session of the request.
    pub session_token: String,
}

/// What a new time entry on an issue starts with.
pub struct EntryDefaultsQuery {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The issue, by its readable ID or its internal ID.
    pub issue: String,
}

/// What a new time entry on an issue starts with: whether the organization offers *Billable*, and
/// the values of the own fields of time entries that its issue, its project or the projects above
/// give it. The server works them out once; the screens only show them.
#[derive(Clone, Debug, Serialize)]
pub struct EntryDefaults {
    /// Whether the organization offers *Billable*.
    pub billable_offered: bool,
    /// The values of own fields the person may see, by the key of the field.
    pub own_fields: OwnValues,
}

impl Masked for EntryDefaults {}

/// Start the timer on an issue.
pub struct StartTimer {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The issue, by its readable ID or its internal ID.
    pub issue: String,
}

/// Stop the timer and save the time as an entry.
pub struct StopTimer {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The version of the timer the person looked at.
    pub version: i64,
    /// What the person filled in for the entry.
    pub details: EntryDetails,
}

/// Save the time so far with the defaults of its issue and start the timer on another issue.
pub struct SwitchTimer {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The version of the timer the person looked at.
    pub version: i64,
    /// The issue to go on with, by its readable ID or its internal ID.
    pub issue: String,
    /// What the person filled in for the entry of the time so far (the window of a required field
    /// without a default); empty for the defaults.
    pub details: EntryDetails,
}

/// Move the start of the running timer.
pub struct ChangeStartTime {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The version of the timer the person looked at.
    pub version: i64,
    /// The new start: never in the future, never before the end of the last entry with a start.
    pub started_at: Timestamp,
}

/// Stop the timer and save nothing.
pub struct ThrowAwayTimer {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The version of the timer the person looked at.
    pub version: i64,
}

/// Book time by hand (*Add time*).
pub struct AddTime {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The issue, by its readable ID or its internal ID.
    pub issue: String,
    /// The date the time belongs to.
    pub date: Date,
    /// How long, in whole minutes.
    pub minutes: Minutes,
    /// What the person filled in.
    pub details: EntryDetails,
}

/// Change one of one's own time entries; what is left out stays.
pub struct ChangeTimeEntry {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The entry.
    pub entry: TimeEntryId,
    /// The version of the entry the person looked at.
    pub version: i64,
    /// A new date.
    pub date: Option<Date>,
    /// A new length.
    pub minutes: Option<Minutes>,
    /// A new note, *Billable* and values of own fields.
    pub details: EntryDetails,
}

/// Delete one of one's own time entries.
pub struct DeleteTimeEntry {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The entry.
    pub entry: TimeEntryId,
    /// The version of the entry the person looked at.
    pub version: i64,
}

/// One's own time entries of some dates (a week of the timesheet).
pub struct MyTimeEntriesQuery {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The first date; with `to` left out, the week of today by the person's *First day of the
    /// week*.
    pub from: Option<Date>,
    /// The last date (included).
    pub to: Option<Date>,
    /// Where the page starts: the `next_cursor` of the page before.
    pub cursor: Option<String>,
    /// How many entries a page holds at most.
    pub limit: u32,
}

/// The time entries on an issue.
pub struct IssueTimeEntriesQuery {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The issue, by its readable ID or its internal ID.
    pub issue: String,
    /// Where the page starts: the `next_cursor` of the page before.
    pub cursor: Option<String>,
    /// How many entries a page holds at most.
    pub limit: u32,
}

/// A timer that runs.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct RunningView {
    /// The issue it runs on; none for an access key that does not see it.
    pub issue: Option<IssueId>,
    /// The readable ID of the issue while the person sees it.
    pub readable_id: Option<String>,
    /// When it started.
    pub started_at: Timestamp,
    /// Whether it runs longer than the longest day of the working-time rules.
    pub runs_long: bool,
}

/// The timer of the person.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct TimerView {
    /// What it runs on; none while it stands still.
    pub running: Option<RunningView>,
    /// Counts up with every change; a change is made on the version that was looked at.
    pub version: i64,
    /// The own field each starter field of time entries is, by its mark (`activity`,
    /// `cost-center`): how the *Stop timer* window finds them whatever they are called now.
    pub starters: BTreeMap<String, FieldId>,
}

impl Masked for TimerView {}

/// One of the person's own time entries.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct TimeEntryView {
    /// The ID of the entry.
    pub id: TimeEntryId,
    /// The issue.
    pub issue: IssueId,
    /// The readable ID of the issue while the person sees it.
    pub readable_id: Option<String>,
    /// The date.
    pub date: Date,
    /// How long, in whole minutes.
    pub minutes: Minutes,
    /// The note, plain text.
    pub note: String,
    /// *Billable*; none while the organization does not offer it.
    pub billable: Option<bool>,
    /// The start of the timer it was saved from; none for an entry made by hand.
    pub started_at: Option<Timestamp>,
    /// The values of its own fields that the person may see, by the key of the field.
    pub own_fields: OwnValues,
    /// Counts up with every change.
    pub version: i64,
}

impl Masked for TimeEntryView {}

/// What stopping or switching the timer did.
#[derive(Clone, Debug, Serialize)]
pub struct TimerSaved {
    /// The timer as it is now.
    pub timer: TimerView,
    /// The entry that was saved, masked with the rights where its issue is; none when the timer
    /// ran for less than a minute.
    pub entry: Option<Authorized<TimeEntryView>>,
}

impl Masked for TimerSaved {}

/// A page of the person's own time entries.
#[derive(Clone, Debug, Serialize)]
pub struct TimeEntryPage {
    /// The entries, by date, then in the order they were made, each masked with the rights where
    /// its issue is.
    pub items: Vec<Authorized<TimeEntryView>>,
    /// The cursor of the next page, if there is one.
    pub next_cursor: Option<String>,
}

impl Masked for TimeEntryPage {}

/// A time entry on an issue as a reader sees it: the person as *Who sees people* allows for the
/// kind *Time entries*, the length and the note only for a reader who may read *Time spent*.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct IssueTimeEntryView {
    /// The ID of the entry.
    pub id: TimeEntryId,
    /// Whose time it is, as the reader may see people of the kind *Time entries*.
    pub person: PersonRef,
    /// The date.
    pub date: Date,
    /// How long; none for a reader who may not read *Time spent*.
    pub minutes: Option<Minutes>,
    /// The note; none for a reader who may not read *Time spent*.
    pub note: Option<String>,
    /// The values of its own fields that the reader may see.
    pub own_fields: OwnValues,
}

/// The time entries on an issue, newest first.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct IssueTimeEntries {
    /// The entries.
    pub items: Vec<IssueTimeEntryView>,
    /// The cursor of the next page, if there is one.
    pub next_cursor: Option<String>,
}

impl Masked for IssueTimeEntries {}
