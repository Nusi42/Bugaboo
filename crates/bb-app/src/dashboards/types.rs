//! What the dashboard use cases take and give back (CFG-03, ADR-0050).

use bb_core::dashboard::{BoxSpec, ReadyMade};
use bb_core::id::{DashboardBoxId, DashboardId};
use bb_core::saved_search::{SearchSet, ShareTarget};
use bb_policy::{Authorized, Masked, PersonRef};
use jiff::Timestamp;
use serde::Serialize;

use crate::history::HistoryItem;
use crate::plan::TimelinePage;
use crate::saved_searches::ShareView;
use crate::search::{SearchPage, SearchTotals};
use crate::time::{TimeEntryPage, TimerView};

/// Create a dashboard: private, and the person's own.
pub struct CreateDashboard {
    /// The secret of the session of the request.
    pub session_token: String,
    /// What it is called.
    pub name: String,
    /// Its boxes, in order.
    pub boxes: Vec<BoxSpec>,
}

/// Change a dashboard the person made. What is `None` stays as it is.
pub struct ChangeDashboard {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The dashboard.
    pub dashboard: DashboardId,
    /// The version the person looked at; a change made meanwhile stops this one.
    pub version: i64,
    /// A new name.
    pub name: Option<String>,
    /// The boxes, all of them, in order: a box that is not named any more goes, and a box keeps
    /// its ID when it is named by it.
    pub boxes: Option<Vec<BoxChange>>,
}

/// A box of a change: one that is there, by its ID, or a new one.
pub struct BoxChange {
    /// The box it was; none for a new one.
    pub id: Option<DashboardBoxId>,
    /// The box as it is to be.
    pub spec: BoxSpec,
}

/// Share a dashboard with a group or a project.
pub struct ShareDashboard {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The dashboard.
    pub dashboard: DashboardId,
    /// The group or the project.
    pub target: ShareTarget,
}

/// Stop sharing a dashboard with a group or a project.
pub struct StopSharingDashboard {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The dashboard.
    pub dashboard: DashboardId,
    /// The group or the project.
    pub target: ShareTarget,
}

/// Put a dashboard into the trash.
pub struct DeleteDashboard {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The dashboard.
    pub dashboard: DashboardId,
}

/// Take a dashboard out of the trash (*Undo*).
pub struct RestoreDashboard {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The dashboard.
    pub dashboard: DashboardId,
}

/// *Save a copy*: a private dashboard of the person with the boxes of one they see, as they see
/// them.
pub struct CopyDashboard {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The dashboard to copy.
    pub dashboard: DashboardId,
    /// What the copy is called; the name of the original when `None`.
    pub name: Option<String>,
}

/// A page of the dashboards of one scope.
pub struct DashboardsQuery {
    /// The secret of the session of the request.
    pub session_token: String,
    /// Which dashboards: the person's own, or the ones shared with them.
    pub scope: SearchSet,
    /// Where the page starts: the `next_cursor` of the page before.
    pub cursor: Option<String>,
    /// How many the page holds at most.
    pub limit: u32,
}

/// One dashboard, opened.
pub struct DashboardQuery {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The dashboard.
    pub dashboard: DashboardId,
}

/// Open the person's *Home*: it is made, with its event, at the first visit.
pub struct OpenHome {
    /// The secret of the session of the request.
    pub session_token: String,
}

/// What one box of a dashboard answers.
pub struct BoxQuery {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The dashboard.
    pub dashboard: DashboardId,
    /// The box.
    pub dashboard_box: DashboardBoxId,
}

/// What a box would answer, before it is added (*Add box*).
pub struct BoxPreviewQuery {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The box.
    pub spec: BoxSpec,
}

/// The ready-made boxes *Add box* offers.
pub struct ReadyMadeQuery {
    /// The secret of the session of the request.
    pub session_token: String,
}

/// A box as one viewer sees it: its place is packed for them.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct BoxView {
    /// The ID of the box.
    pub id: DashboardBoxId,
    /// The kind, the place for this viewer, the title, the source and the options.
    pub spec: BoxSpec,
}

/// A dashboard as one person sees it: the boxes that check for them, packed.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct DashboardView {
    /// The ID, in the address `/d/<id>`.
    pub id: DashboardId,
    /// What it is called.
    pub name: String,
    /// Who made it, as the viewer sees people who write.
    pub maker: PersonRef,
    /// Whether the viewer made it.
    pub mine: bool,
    /// Whether it is the viewer's *Home*.
    pub home: bool,
    /// Whom it is shared with; only its maker is told, for everybody else this is empty.
    pub shared_with: Vec<ShareView>,
    /// The boxes the viewer sees, in order, packed upwards.
    pub boxes: Vec<BoxView>,
    /// When it was last changed.
    pub updated_at: Timestamp,
    /// Counts up with every change, for the check that nobody changed it meanwhile.
    pub version: i64,
}

impl Masked for DashboardView {}

/// A dashboard in a list: without its boxes.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct DashboardItem {
    /// The ID.
    pub id: DashboardId,
    /// What it is called.
    pub name: String,
    /// Who made it, as the viewer sees people who write.
    pub maker: PersonRef,
    /// Whether the viewer made it.
    pub mine: bool,
    /// When it was last changed.
    pub updated_at: Timestamp,
    /// Counts up with every change.
    pub version: i64,
}

/// A page of dashboards.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct DashboardPage {
    /// The dashboards.
    pub items: Vec<DashboardItem>,
    /// The cursor of the next page, if there is one.
    pub next_cursor: Option<String>,
}

impl Masked for DashboardPage {}

/// What taking a dashboard out of the trash answers: the dashboard, when it is one the person may
/// open.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct RestoredDashboard {
    /// The dashboard as the person sees it; none when it is not theirs to open.
    pub dashboard: Option<DashboardView>,
}

impl Masked for RestoredDashboard {}

/// One of the newest changes of an issue a box of *Recent changes* reads: the issue and the row of
/// its history, as the history shows it to the viewer.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct RecentChange {
    /// The issue, by its readable ID.
    pub issue: String,
    /// The row of the history.
    pub item: HistoryItem,
}

/// The newest changes of the issues a search finds, newest first.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct RecentChanges {
    /// The changes.
    pub items: Vec<RecentChange>,
}

impl Masked for RecentChanges {}

/// What a box answers: the answer of the read that exists for its kind, as the viewer.
pub enum BoxAnswer {
    /// *Search results*: the first issues of the search.
    Issues(Authorized<SearchPage>),
    /// *Numbers and chart* and the *Time totals* of a search: the totals of the search.
    Totals(Authorized<SearchTotals>),
    /// The viewer's own time entries of this week.
    Week(Authorized<TimeEntryPage>),
    /// *Small timeline*: one page of the timeline of the search.
    Timeline(Authorized<TimelinePage>),
    /// *My timer*: the viewer's own timer.
    Timer(Authorized<TimerView>),
    /// *Recent changes*: the newest changes of the issues the search finds.
    Changes(Authorized<RecentChanges>),
}

/// A ready-made box *Add box* offers, with what it starts with.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ReadyMadeView {
    /// Which one; its title is the message of the word.
    pub which: ReadyMade,
    /// The box it adds, at the top left, with its title in the words of the catalog.
    pub spec: BoxSpec,
}

/// The ready-made boxes, in the order *Add box* offers them.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ReadyMadeList {
    /// The boxes.
    pub items: Vec<ReadyMadeView>,
}

impl Masked for ReadyMadeList {}
