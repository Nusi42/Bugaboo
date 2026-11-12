//! The routes of the timeline (PLAN-01, PLAN-04, PLAN-08, PLAN-10, PLAN-12, PLAN-13, ADR-0030
//! amendment M8, ADR-0043): one route answers a page of the plan (`POST /api/v1/timeline`), worked
//! out once on the server for the reader; the preview of *Move it after*; and *Keep and accept*.
//! The time factor of an issue is in `timeline_factor.rs`. The plan's rules are in `bb-core`, its
//! use cases in `bb-app`; no rule of the plan is in the browser.

use std::sync::Arc;

use axum::Extension;
use axum::extract::{FromRef, State};
use axum::response::Response;
use base64ct::{Base64UrlUnpadded, Encoding as _};
use bb_app::plan::{
    AcceptWarning, AcceptedWarningView, BarView, Hatched, MoveAfterQuery, MovePreview,
    MovePreviewPage, TimelinePage, TimelinePart, TimelineQuery, TimelineSearch, WarningView,
};
use bb_core::holidays::Holiday;
use bb_core::issue::CalendarDay;
use bb_core::plan::Days;
use bb_core::plan::bar::WorkedOut;
use bb_core::plan::warning::{Delay, Warning, WarningKind, WayOut};
use bb_core::saved_search::TimelineOptions;
use bb_policy::{Masked, PersonRef};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest as _, Sha256};
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use super::auth::{
    AuthHandle, ConflictAnswer, ForbiddenAnswer, Info, InvalidAnswer, NotFoundAnswer,
    NotSignedInAnswer, Session, answer_authorized,
};
use super::issues::IssueItem;
use super::saved_searches::TimelineOptionsBody;
use super::search::{SearchGroupKeyBody, counted, group_key};
use crate::errors::{ApiError, ApiJson};
use crate::limits::Limits;
use crate::paging;

/// Which part of the timeline a page is of.
#[derive(Clone, Copy, Default, Deserialize, ToSchema)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum TimelinePartBody {
    /// The rows with their bars, the dependencies between them and their warnings.
    #[default]
    Rows,
    /// The issues of *Not scheduled yet*.
    NotScheduled,
}

/// Ask for a page of the timeline: a saved search or a search text, the options of the view and the
/// span of days the screen shows.
#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct TimelineBody {
    /// The saved search the timeline is a view of, by its ID; or else `search_text`.
    #[serde(default)]
    saved_search: Option<String>,
    /// The search text, which need not be saved; used when `saved_search` is left out.
    #[serde(default)]
    search_text: Option<String>,
    /// The project whose timeline it is (it and the projects inside it), by ID or short name;
    /// left out, everywhere the person sees.
    #[serde(default)]
    project: Option<String>,
    /// The options of the view: the date fields, *Plan by effort* and the zoom.
    timeline: TimelineOptionsBody,
    /// The field the rows are grouped by, by its search word: `project`, `parent` (the goal),
    /// `assigned to` (*Group by person*), `status` or `priority`.
    #[serde(default)]
    group_by: Option<String>,
    /// The first day the screen shows (`2026-11-02`).
    #[schema(value_type = String, format = Date)]
    first: CalendarDay,
    /// The last day the screen shows; a span is a year at most.
    #[schema(value_type = String, format = Date)]
    last: CalendarDay,
    /// Which part: the rows, or the issues of *Not scheduled yet*.
    #[serde(default)]
    part: TimelinePartBody,
    /// Where the page starts: the `next_cursor` of the page before.
    #[serde(default)]
    cursor: Option<String>,
    /// How many issues a page holds at most.
    #[serde(default)]
    #[schema(schema_with = crate::lean::page_limit_schema)]
    limit: Option<u32>,
}

/// A stretch of days, both ends included.
#[derive(Clone, Copy, Serialize, ToSchema)]
pub(crate) struct DaysBody {
    /// The first day.
    #[schema(value_type = String, format = Date)]
    first: CalendarDay,
    /// The last day.
    #[schema(value_type = String, format = Date)]
    last: CalendarDay,
}

// The days of a bar are worked out only from the date fields the reader may read.
impl Masked for DaysBody {}

impl From<Days> for DaysBody {
    fn from(days: Days) -> Self {
        Self {
            first: days.first,
            last: days.last,
        }
    }
}

/// What a hatched day holds.
#[derive(Serialize, ToSchema)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum HatchedKindBody {
    /// The person is away (only ever *Away*).
    Away,
    /// A public holiday.
    Holiday,
    /// Not a usual work day of the person, or no share in the project.
    DayOff,
}

/// One day of a bar on which its person does not work, as *Who sees people* allows.
#[derive(Serialize, ToSchema)]
pub(crate) struct HatchedDayBody {
    /// The day.
    #[schema(value_type = String, format = Date)]
    date: CalendarDay,
    /// What it holds.
    kind: HatchedKindBody,
    /// For an away day: whether the morning is away.
    morning: bool,
    /// For an away day: whether the afternoon is away.
    afternoon: bool,
    /// For a public holiday: which one.
    #[schema(required = true)]
    holiday: Option<Holiday>,
}

/// The time factor a bar is planned with.
#[derive(Serialize, ToSchema)]
pub(crate) struct BarTimeFactorBody {
    /// In percent of the estimate (125 is ×1.25).
    percent: u16,
    /// Whether the person on the bar set it themselves.
    own_estimate: bool,
}

/// The bar of a row.
#[derive(Serialize, ToSchema)]
pub(crate) struct BarBody {
    /// The days it covers.
    days: DaysBody,
    /// Which date *Plan by effort* worked out (shown, never stored): `neither`, `start` or `due`.
    #[schema(value_type = String)]
    worked_out: WorkedOut,
    /// How much of it is done, in percent; `null` without an estimate the person may read.
    #[schema(required = true)]
    done: Option<u8>,
    /// The person on the bar (the first assigned person) as the reader may see them; `null` for
    /// nobody or when the reader may not see who it is.
    #[schema(required = true)]
    person: Option<PersonRef>,
    /// The time factor, when it is not ×1 and the reader may read it.
    #[schema(required = true)]
    time_factor: Option<BarTimeFactorBody>,
    /// The days inside the span on which the person does not work.
    hatched: Vec<HatchedDayBody>,
}

impl From<&BarView> for BarBody {
    fn from(bar: &BarView) -> Self {
        Self {
            days: bar.days.into(),
            worked_out: bar.worked_out,
            done: bar.done,
            person: bar.person.clone(),
            time_factor: bar.factor.map(|factor| BarTimeFactorBody {
                percent: factor.percent,
                own_estimate: factor.own_estimate,
            }),
            hatched: bar
                .hatched
                .iter()
                .map(|day| {
                    let (kind, morning, afternoon, holiday) = match day.what {
                        Hatched::Away { morning, afternoon } => {
                            (HatchedKindBody::Away, morning, afternoon, None)
                        }
                        Hatched::Holiday { holiday } => {
                            (HatchedKindBody::Holiday, false, false, Some(holiday))
                        }
                        Hatched::DayOff => (HatchedKindBody::DayOff, false, false, None),
                    };
                    HatchedDayBody {
                        date: day.date,
                        kind,
                        morning,
                        afternoon,
                        holiday,
                    }
                })
                .collect(),
        }
    }
}

/// One row of the timeline.
#[derive(Serialize, ToSchema)]
pub(crate) struct TimelineRowBody {
    /// The group of the row; `null` when not grouped or when the reader may not know the group.
    #[schema(required = true)]
    group: Option<SearchGroupKeyBody>,
    /// The issue, with what the reader may see of it in its project.
    #[schema(value_type = IssueItem)]
    issue: Value,
    /// Its bar; `null` in *Not scheduled yet*.
    #[schema(required = true)]
    bar: Option<BarBody>,
    /// Whether this is the row of an assigned person who is not the person on the bar: the bar is
    /// planned with the first person's work times.
    copy: bool,
}

/// One dependency between two issues of the page.
#[derive(Serialize, ToSchema)]
pub(crate) struct TimelineDependencyBody {
    /// The issue that must finish first.
    first: String,
    /// The issue that waits for it.
    then: String,
}

/// An issue a warning names besides its own.
#[derive(Serialize, ToSchema)]
pub(crate) struct PlanWarningIssueBody {
    /// The internal ID.
    id: String,
    /// The readable ID.
    readable_id: String,
}

/// Who accepted a warning and when.
#[derive(Serialize, ToSchema)]
pub(crate) struct PlanAcceptedBody {
    /// Who accepted it.
    by: PersonRef,
    /// When.
    at: String,
}

/// A plan warning of an issue of the page.
#[derive(Serialize, ToSchema)]
pub(crate) struct PlanWarningBody {
    /// The issue the warning is on.
    issue: String,
    /// The kind.
    kind: WarningKind,
    /// The key of its sentence in the messages; the screen fills it in with the members below.
    sentence: String,
    /// The days it is about.
    days: DaysBody,
    /// The person on the bar it is about, for *Away* and *Too much at once*; `null` when the
    /// reader may not see who it is (*"the assigned person is away"*).
    #[schema(required = true)]
    person: Option<PersonRef>,
    /// The other issue it names: the one it waits for, or the sub-issue of a goal.
    #[schema(required = true)]
    other: Option<PlanWarningIssueBody>,
    /// *Delay*: how many days after the issue starts the one it waits for ends, or how many days
    /// after the due date the sub-issue ends.
    #[schema(required = true)]
    late_days: Option<u32>,
    /// *Not enough days*: the work days the effort needs; `null` when the known days do not hold it.
    #[schema(required = true)]
    needs_days: Option<u32>,
    /// *Not enough days*: the work days there are before the due date.
    #[schema(required = true)]
    has_days: Option<u32>,
    /// *Too much at once*: the day with the most too much.
    #[schema(required = true, value_type = Option<String>, format = Date)]
    worst_day: Option<CalendarDay>,
    /// *Too much at once*: the minutes planned for the person on that day in the project.
    #[schema(required = true)]
    planned_minutes: Option<u32>,
    /// *Too much at once*: the minutes the person can work on that day in the project.
    #[schema(required = true)]
    can_minutes: Option<u32>,
    /// The ways out that fit it, in the order they are offered.
    ways_out: Vec<WayOut>,
    /// The day *Move it after* starts the issue after (the preview takes it).
    #[schema(required = true, value_type = Option<String>, format = Date)]
    move_after: Option<CalendarDay>,
    /// The due date *Make it longer* answers; it changes only when the person confirms it.
    #[schema(required = true, value_type = Option<String>, format = Date)]
    due_needed: Option<CalendarDay>,
    /// Who accepted it and when (*Keep and accept*): it shows faded until the dates of its issue
    /// change.
    #[schema(required = true)]
    accepted: Option<PlanAcceptedBody>,
}

impl From<&WarningView> for PlanWarningBody {
    fn from(view: &WarningView) -> Self {
        let (late_days, needs_days, has_days) = match view.what {
            Warning::Delay(Delay::Dependency { days, .. } | Delay::SubIssue { days, .. }) => {
                (Some(days), None, None)
            }
            Warning::NotEnoughDays(short) => (None, short.needs, Some(short.has)),
            Warning::Away { .. } | Warning::TooMuchAtOnce { .. } => (None, None, None),
        };
        let (worst_day, planned_minutes, can_minutes) = match view.what {
            Warning::TooMuchAtOnce {
                worst,
                planned,
                can,
                ..
            } => (Some(worst), Some(planned), Some(can)),
            _ => (None, None, None),
        };
        Self {
            issue: view.issue.to_string(),
            kind: view.kind,
            sentence: view.sentence.as_str().to_owned(),
            days: view.days.into(),
            person: view.person.clone(),
            other: view.other.as_ref().map(|(id, key)| PlanWarningIssueBody {
                id: id.to_string(),
                readable_id: key.clone(),
            }),
            late_days,
            needs_days,
            has_days,
            worst_day,
            planned_minutes,
            can_minutes,
            ways_out: view.ways_out.clone(),
            move_after: view.move_after,
            due_needed: view.due_needed,
            accepted: view.accepted.as_ref().map(|accepted| PlanAcceptedBody {
                by: accepted.by.clone(),
                at: accepted.at.to_string(),
            }),
        }
    }
}

/// A page of the timeline.
#[derive(Default, Serialize, ToSchema)]
pub(crate) struct TimelinePageBody {
    /// The rows, in the order of the search; an issue has a row for each of its groups.
    rows: Vec<TimelineRowBody>,
    /// The cursor of the next page; `null` on the last.
    #[schema(required = true)]
    next_cursor: Option<String>,
    /// The dependencies between issues of the page.
    dependencies: Vec<TimelineDependencyBody>,
    /// The warnings of the issues of the page.
    warnings: Vec<PlanWarningBody>,
    /// How many issues the search finds that wait in *Not scheduled yet*.
    not_scheduled: u64,
    /// Whether the bars were planned by effort: the view asked for it and the reader is no access
    /// key. A key reads nobody's work times, so its bars keep the dates people set.
    planned_by_effort: bool,
}

impl Masked for TimelinePageBody {}

/// The name of the list a cursor of the timeline is sealed for: the person and a hash of what the
/// page is of, so a cursor opens for nobody else and for no other timeline.
fn list_of(person: bb_core::id::UserId, body: &TimelineBody) -> String {
    let mut hash = Sha256::new();
    for part in [
        body.saved_search.as_deref(),
        body.search_text.as_deref(),
        body.project.as_deref(),
        body.timeline.start_field(),
        body.timeline.end_field(),
    ] {
        hash.update(part.unwrap_or_default().as_bytes());
        hash.update([0]);
    }
    hash.update([u8::from(matches!(
        body.part,
        TimelinePartBody::NotScheduled
    ))]);
    format!(
        "timeline:{person}:{}",
        Base64UrlUnpadded::encode_string(&hash.finalize())
    )
}

/// The body of a page, each row's issue with what the reader may see of it.
fn page_body(page: TimelinePage, list: &str) -> Result<TimelinePageBody, ApiError> {
    let rows = page
        .rows
        .into_iter()
        .map(|row| {
            let issue = row
                .issue
                .map(|view| IssueItem::from(&view))
                .masked_value()
                .map_err(|error| ApiError::internal(&error))?;
            Ok(TimelineRowBody {
                group: row.group.map(group_key),
                issue,
                bar: row.bar.as_ref().map(BarBody::from),
                copy: row.copy,
            })
        })
        .collect::<Result<_, ApiError>>()?;
    Ok(TimelinePageBody {
        rows,
        next_cursor: page
            .next_cursor
            .map(|position| paging::seal(list, &position))
            .transpose()?,
        dependencies: page
            .dependencies
            .iter()
            .map(|(first, then)| TimelineDependencyBody {
                first: first.to_string(),
                then: then.to_string(),
            })
            .collect(),
        warnings: page.warnings.iter().map(PlanWarningBody::from).collect(),
        not_scheduled: page.not_scheduled,
        planned_by_effort: page.planned_by_effort,
    })
}

/// A page of the timeline of a search: the issues with a date the person may read, each with its
/// bar (its days, the done part, the person on the bar, the hatched days), the dependencies between
/// them, their plan warnings and the count of *Not scheduled yet* (whose issues the same route
/// answers with `part: not-scheduled`). Restricted issues are left out entirely. It reads and
/// changes nothing.
#[utoipa::path(
    post,
    path = "/api/v1/timeline",
    operation_id = "getTimeline",
    tag = "timeline",
    request_body = TimelineBody,
    responses(
        (status = 200, description = "A page of the timeline.", body = TimelinePageBody),
        InvalidAnswer,
        NotSignedInAnswer,
        NotFoundAnswer
    )
)]
async fn get_timeline(
    State(auth): State<AuthHandle>,
    Extension(limits): Extension<Arc<Limits>>,
    Info(info): Info,
    Session(session_token): Session,
    ApiJson(body): ApiJson<TimelineBody>,
) -> Result<Response, ApiError> {
    let person = counted(&auth, &limits, &info, &session_token).await?;
    let list = list_of(person, &body);
    let cursor = body
        .cursor
        .as_deref()
        .map(|cursor| paging::open(&list, cursor))
        .transpose()?;
    let search = match (&body.saved_search, &body.search_text) {
        (Some(id), _) => TimelineSearch::Saved(
            id.parse()
                .map_err(|_| ApiError::new(bb_core::ErrorCode::NotFound))?,
        ),
        (None, text) => TimelineSearch::Text(text.clone().unwrap_or_default()),
    };
    let page = auth
        .api()?
        .timeline(
            info,
            TimelineQuery {
                session_token,
                search,
                project: body.project.clone(),
                options: TimelineOptions::try_from(body.timeline.clone())?,
                group_by: body.group_by.clone(),
                first: body.first,
                last: body.last,
                part: match body.part {
                    TimelinePartBody::Rows => TimelinePart::Rows,
                    TimelinePartBody::NotScheduled => TimelinePart::NotScheduled,
                },
                cursor,
                limit: bb_core::paging::page_size(body.limit),
            },
        )
        .await?;
    // Each row's issue is masked with what the reader holds in its project; a failure to do so
    // stops the answer.
    let mut failed = None;
    let body = page.map(|page| {
        page_body(page, &list).unwrap_or_else(|error| {
            failed = Some(error);
            TimelinePageBody::default()
        })
    });
    if let Some(error) = failed {
        return Err(error);
    }
    answer_authorized(&body)
}

/// Ask for the preview of *Move it after*.
#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct MoveAfterBody {
    /// The issue the warning is on, by its readable ID or internal ID.
    issue: String,
    /// The day the issue is to start after: the `move_after` of its warning.
    #[schema(value_type = String, format = Date)]
    after: CalendarDay,
    /// The options of the view the warning was shown in. Only a view of *Start date* and *Due
    /// date* can move issues.
    timeline: TimelineOptionsBody,
}

/// One issue of the preview, with its new dates: a date field the reader may not read is left out,
/// for each issue in its own project and again by the field levels of the project of the issue the
/// preview is for.
#[derive(Serialize, ToSchema, Masked)]
pub(crate) struct MoveAfterIssueBody {
    /// The internal ID.
    #[masked(open)]
    id: String,
    /// The readable ID.
    #[masked(open)]
    readable_id: String,
    /// The version to move it from (`POST /api/v1/issues/move-dates` takes it).
    #[masked(open)]
    version: i64,
    /// The days of its bar now, worked out from the dates the reader may read.
    #[masked(nested)]
    from: DaysBody,
    /// The days of its bar after the move.
    #[masked(nested)]
    to: DaysBody,
    /// The *Start date* it gets; `null` when it has none or the reader may not read it.
    #[masked(field = "start-date")]
    #[schema(value_type = Option<String>, format = Date)]
    start_date: Option<CalendarDay>,
    /// The *Due date* it gets; `null` when it has none or the reader may not read it.
    #[masked(field = "due-date")]
    #[schema(value_type = Option<String>, format = Date)]
    due_date: Option<CalendarDay>,
}

impl From<&MovePreview> for MoveAfterIssueBody {
    fn from(each: &MovePreview) -> Self {
        Self {
            id: each.issue.to_string(),
            readable_id: each.key.clone(),
            version: each.version,
            from: each.from.into(),
            to: each.to.into(),
            start_date: each.start_date,
            due_date: each.due_date,
        }
    }
}

/// The preview of *Move it after*.
#[derive(Serialize, ToSchema, Masked)]
pub(crate) struct MoveAfterPreviewBody {
    /// Every issue that moves, in the order along the dependencies, with its new dates.
    #[masked(nested)]
    issues: Vec<MoveAfterIssueBody>,
}

impl From<MovePreviewPage> for MoveAfterPreviewBody {
    fn from(page: MovePreviewPage) -> Self {
        Self {
            issues: page.issues.iter().map(MoveAfterIssueBody::from).collect(),
        }
    }
}

/// The preview of *Move it after*: the issue starts on the day after `after`, and the issues that
/// wait for it in turn each start on the day after the one before ends. It names every issue that
/// moves with its new dates and saves nothing; the move is one run of
/// `POST /api/v1/issues/move-dates`, which *Undo the run* takes back in one step.
#[utoipa::path(
    post,
    path = "/api/v1/timeline/move-after",
    operation_id = "previewMoveAfter",
    tag = "timeline",
    request_body = MoveAfterBody,
    responses(
        (status = 200, description = "The issues that move, with their new dates.", body = MoveAfterPreviewBody),
        InvalidAnswer,
        NotSignedInAnswer,
        NotFoundAnswer
    )
)]
async fn preview_move_after(
    State(auth): State<AuthHandle>,
    Extension(limits): Extension<Arc<Limits>>,
    Info(info): Info,
    Session(session_token): Session,
    ApiJson(body): ApiJson<MoveAfterBody>,
) -> Result<Response, ApiError> {
    // The walk reads every dependency: it counts against the searches of the person.
    counted(&auth, &limits, &info, &session_token).await?;
    let preview = auth
        .api()?
        .move_after_preview(
            info,
            MoveAfterQuery {
                session_token,
                issue: body.issue,
                after: body.after,
                options: TimelineOptions::try_from(body.timeline)?,
            },
        )
        .await?;
    answer_authorized(&preview.map(MoveAfterPreviewBody::from))
}

/// *Keep and accept* a plan warning.
#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct AcceptWarningBody {
    /// The issue the warning is on, by its readable ID or internal ID.
    issue: String,
    /// The version of the issue the person looked at.
    version: i64,
    /// The kind of the warning.
    kind: WarningKind,
    /// The options of the view the warning was shown in. Only a view of *Start date* and *Due
    /// date* keeps an accepted warning, and only one the issue has in it now.
    timeline: TimelineOptionsBody,
}

/// A warning that was accepted: it shows faded until the dates of its issue change.
#[derive(Serialize, ToSchema)]
pub(crate) struct AcceptedWarningBody {
    /// The issue.
    issue: String,
    /// The kind.
    kind: WarningKind,
}

impl Masked for AcceptedWarningBody {}

impl From<AcceptedWarningView> for AcceptedWarningBody {
    fn from(view: AcceptedWarningView) -> Self {
        Self {
            issue: view.issue.to_string(),
            kind: view.kind,
        }
    }
}

/// *Keep and accept*: keeps the plan as it is and the warning of this kind on the issue, faded for
/// everybody who sees the issue, until the dates of the issue change. The person on the bar and
/// whoever may change the issue may accept it.
#[utoipa::path(
    post,
    path = "/api/v1/timeline/accepted-warnings",
    operation_id = "acceptWarning",
    tag = "timeline",
    request_body = AcceptWarningBody,
    responses(
        (status = 200, description = "The accepted warning.", body = AcceptedWarningBody),
        InvalidAnswer,
        NotSignedInAnswer,
        ForbiddenAnswer,
        NotFoundAnswer,
        ConflictAnswer
    )
)]
async fn accept_warning(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiJson(body): ApiJson<AcceptWarningBody>,
) -> Result<Response, ApiError> {
    let accepted = auth
        .api()?
        .accept_warning(
            info,
            AcceptWarning {
                session_token,
                issue: body.issue,
                version: body.version,
                kind: body.kind,
                options: TimelineOptions::try_from(body.timeline)?,
            },
        )
        .await?;
    answer_authorized(&accepted.map(AcceptedWarningBody::from))
}

/// The routes of the timeline.
pub(crate) fn routes<S>(limits: &Arc<Limits>) -> OpenApiRouter<S>
where
    S: Clone + Send + Sync + 'static,
    AuthHandle: FromRef<S>,
{
    OpenApiRouter::new()
        .routes(routes!(get_timeline))
        .routes(routes!(preview_move_after))
        .routes(routes!(accept_warning))
        .layer(Extension(Arc::clone(limits)))
}
