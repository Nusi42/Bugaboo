//! The inputs and answers of the use cases of work times.

use bb_core::holidays::Region as HolidayRegion;
use bb_core::id::{AwayTimeId, UserId};
use bb_core::work_times::UsualDay;
use bb_policy::{Masked, PersonRef};
use jiff::civil::Date;

use crate::access::ProjectLabel;

/// Read a person's work times, with the away times that touch the days from `first` to `last`.
pub struct WorkTimesQuery {
    /// The secret of the session of the request.
    pub session_token: String,
    /// Whose work times; `None` for the reader's own.
    pub person: Option<UserId>,
    /// The first day of the stretch the away times are read for.
    pub first: Date,
    /// The last day of the stretch.
    pub last: Date,
}

/// An away time as its reader sees it. Another person's shows only its days: no note, no reason
/// (there is none) and nobody who entered it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AwayView {
    /// The ID of the away time.
    pub id: AwayTimeId,
    /// The first day.
    pub first: Date,
    /// The last day.
    pub last: Date,
    /// Whether the person is away only in the afternoon of the first day.
    pub first_afternoon_only: bool,
    /// Whether the person is away only in the morning of the last day.
    pub last_morning_only: bool,
    /// The note; only its person reads it.
    pub note: Option<String>,
    /// Who entered it; only its person sees that.
    pub entered_by: Option<PersonRef>,
    /// Counts up with every change: a change is made on the version that was looked at.
    pub version: i64,
}

/// One share of a person, with the project as the reader may name it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShareItem {
    /// The project.
    pub project: ProjectLabel,
    /// The share in percent.
    pub percent: u8,
    /// Whether the share is in force: it counts as nothing while the person has no role in the
    /// project, and counts again when a role comes back.
    pub applies: bool,
}

/// The shares a person has set, and what they add up to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SharesOfPerson {
    /// The shares, by the name of the project; a project the reader may not see comes last.
    pub items: Vec<ShareItem>,
    /// What the shares that count add up to, in percent.
    pub sum: u32,
    /// Whether the sum is more than 100 percent.
    pub over_100: bool,
}

/// A person's work times.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkTimesView {
    /// Whose they are, as the reader sees people of the kind *Work times*.
    pub person: PersonRef,
    /// Whether they are the reader's own.
    pub own: bool,
    /// Whether the reader may change the usual work days and enter away times.
    pub may_change: bool,
    /// The seven usual work days, Monday first.
    pub usual: Vec<UsualDay>,
    /// Which public holidays count for the person (the setting *Holidays*).
    pub holidays: HolidayRegion,
    /// The away times that touch the stretch, by first day.
    pub away: Vec<AwayView>,
    /// Whether more away times touch the stretch than `AWAY_TIMES_MAX`: the oldest are left out.
    pub truncated: bool,
    /// The shares the person set; only the person reads them here (a project lead reads them on
    /// *Project members*).
    pub shares: Option<SharesOfPerson>,
}

impl Masked for WorkTimesView {}

/// New usual work days and work hours of a person.
pub struct ChangeUsualDays {
    /// The secret of the session of the request.
    pub session_token: String,
    /// Whose; `None` for the reader's own.
    pub person: Option<UserId>,
    /// The seven days, Monday first.
    pub days: [UsualDay; 7],
}

/// The usual work days after a change.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChangedUsualDays {
    /// The seven days, Monday first.
    pub usual: Vec<UsualDay>,
}

impl Masked for ChangedUsualDays {}

/// The days of an away time, as a request names them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AwayDays {
    /// The first day.
    pub first: Date,
    /// The last day.
    pub last: Date,
    /// Away only in the afternoon of the first day.
    pub first_afternoon_only: bool,
    /// Away only in the morning of the last day.
    pub last_morning_only: bool,
    /// The note, only for the person's own away time.
    pub note: Option<String>,
}

/// Enter an away time.
pub struct EnterAwayTime {
    /// The secret of the session of the request.
    pub session_token: String,
    /// Whose; `None` for the reader's own.
    pub person: Option<UserId>,
    /// Its days.
    pub days: AwayDays,
}

/// Change an away time.
pub struct ChangeAwayTime {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The away time.
    pub id: AwayTimeId,
    /// The version the reader looked at.
    pub version: i64,
    /// The new days. A note is kept as it is when somebody other than the person changes the days.
    pub days: AwayDays,
}

/// Remove an away time.
pub struct RemoveAwayTime {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The away time.
    pub id: AwayTimeId,
    /// The version the reader looked at.
    pub version: i64,
}

/// An away time after a change.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChangedAwayTime {
    /// The away time, as the reader sees it.
    pub away: AwayView,
}

impl Masked for ChangedAwayTime {}

/// Set a person's share in a project.
pub struct SetShare {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The project: its ID or its short name.
    pub project: String,
    /// The person.
    pub person: UserId,
    /// The share in percent, 0 to 100.
    pub percent: u8,
}

/// A share after a change.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChangedShare {
    /// The share in percent.
    pub percent: u8,
}

impl Masked for ChangedShare {}

/// The shares of the people of one project.
pub struct ProjectSharesQuery {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The project: its ID or its short name.
    pub project: String,
    /// Where the page starts: the `next_cursor` of the page before.
    pub cursor: Option<String>,
    /// How many people a page holds at most.
    pub limit: u32,
}

/// One person of a project with their share there.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MemberShare {
    /// The person, as the reader sees people of the kind *Work times*.
    pub person: PersonRef,
    /// The share in the project, in percent: 100 for a person with none set.
    pub percent: u8,
    /// Whether the shares of the person that apply add up to more than 100 percent; only for a
    /// reader who decides who has which role in the project, who is told the fact and never the
    /// sum, which would tell how much the projects they may not see hold.
    pub over_100: Option<bool>,
    /// The other projects the person has a share in, as the reader may name them: a project the
    /// reader may not see is not named, and only `others_hidden` says there is one.
    pub others: Vec<ShareItem>,
    /// Whether the person has a share in another project the reader may not see.
    pub others_hidden: bool,
}

/// A page of the people of a project with their shares.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProjectShares {
    /// The people by name.
    pub items: Vec<MemberShare>,
    /// The cursor of the next page, if there is one.
    pub next_cursor: Option<String>,
}

impl Masked for ProjectShares {}
